import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  Background,
  Controls,
  MarkerType,
  MiniMap,
  ReactFlow,
  useEdgesState,
  useNodesState,
} from "@xyflow/react";
import type {
  Connection,
  EdgeMouseHandler,
  NodeMouseHandler,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";

import { fetchGraph } from "./api";
import type {
  CreatePortalRequest,
  GraphPayload,
  MapNodeInfo,
  PortalEdgeInfo,
} from "./api";
import { computeEdgeRoutes } from "./edgeRouting";
import type { EdgeRouteInput } from "./edgeRouting";
import { layoutGraph, nodeBox, LABEL_STRIP_H } from "./layout";
import { MapNode, MARKER_HANDLE_PREFIX } from "./MapNode";
import type { ArrivalMarker, MapFlowNode, PortalMarker } from "./MapNode";
import { ManhattanEdge } from "./ManhattanEdge";
import type { ManhattanFlowEdge } from "./ManhattanEdge";
import { applyOp, OpStack } from "./ops";
import { PortalDialog, RetargetDialog } from "./PortalDialog";
import { SidePanel } from "./SidePanel";
import type { Selection } from "./SidePanel";

const nodeTypes = { map: MapNode };
const edgeTypes = { manhattan: ManhattanEdge };
const TOAST_MS = 3200;
const PANEL_MIN = 220;
const PANEL_DEFAULT = 340;

function nodeBadges(info: MapNodeInfo): string[] {
  const badges: string[] = [];
  if (info.has_inn) badges.push("🛏");
  if (info.has_shop) badges.push("🛒");
  if (info.has_apothecary) badges.push("⚗");
  if (info.has_magic_core_shop) badges.push("💠");
  if (info.encounter) badges.push("⚔");
  return badges;
}

function tileFraction(
  tile: [number, number],
  mapSizePx: [number, number],
  tileSizePx: [number, number],
): [number, number] | null {
  const [mapW, mapH] = mapSizePx;
  if (mapW <= 0 || mapH <= 0) return null;
  return [
    ((tile[0] + 0.5) * tileSizePx[0]) / mapW,
    ((tile[1] + 0.5) * tileSizePx[1]) / mapH,
  ];
}

function buildFlowNodes(
  graph: GraphPayload,
  visible: (info: MapNodeInfo) => boolean,
  positions: Map<string, { x: number; y: number }>,
  onSelectEdge: (edgeId: string) => void,
): MapFlowNode[] {
  const nodesById = new Map(graph.nodes.map((n) => [n.map_id, n]));
  const markersByMap = new Map<string, PortalMarker[]>();
  const arrivalsByMap = new Map<string, ArrivalMarker[]>();
  for (const edge of graph.edges) {
    const source = nodesById.get(edge.source);
    if (source) {
      const [mapW, mapH] = source.map_size_px;
      if (mapW > 0 && mapH > 0) {
        const [x, y, w, h] = edge.source_rect_px;
        const list = markersByMap.get(edge.source) ?? [];
        list.push({
          edgeId: edge.id,
          targetMap: edge.target,
          fx: (x + w / 2) / mapW,
          fy: (y + h / 2) / mapH,
        });
        markersByMap.set(edge.source, list);
      }
    }
    const target = nodesById.get(edge.target);
    if (target) {
      const frac = tileFraction(
        edge.target_tile,
        target.map_size_px,
        target.tile_size_px,
      );
      if (frac) {
        const list = arrivalsByMap.get(edge.target) ?? [];
        list.push({
          edgeId: edge.id,
          fromMap: edge.source,
          fx: frac[0],
          fy: frac[1],
        });
        arrivalsByMap.set(edge.target, list);
      }
    }
  }

  return graph.nodes.filter(visible).map((info) => {
    const box = nodeBox(info);
    const pos = positions.get(info.map_id) ?? { x: 0, y: 0 };
    return {
      id: info.map_id,
      type: "map" as const,
      position: pos,
      data: {
        mapId: info.map_id,
        displayName: info.display_name,
        isWorld: info.is_world,
        thumbWidth: box.thumbWidth,
        thumbHeight: box.thumbHeight,
        markers: markersByMap.get(info.map_id) ?? [],
        arrivals: arrivalsByMap.get(info.map_id) ?? [],
        badges: nodeBadges(info),
        onSelectEdge,
      },
    };
  });
}

function buildFlowEdges(
  graph: GraphPayload,
  visibleIds: Set<string>,
): ManhattanFlowEdge[] {
  return graph.edges
    .filter((e) => visibleIds.has(e.source) && visibleIds.has(e.target))
    .map((edge) => ({
      id: edge.id,
      source: edge.source,
      target: edge.target,
      type: "manhattan" as const,
      markerEnd: { type: MarkerType.ArrowClosed, width: 14, height: 14 },
      data: {
        path: "",
        labelX: 0,
        labelY: 0,
        label: `${edge.source_tile[0]},${edge.source_tile[1]} → ${edge.target_tile[0]},${edge.target_tile[1]}`,
      },
    }));
}

export default function App() {
  const [graph, setGraph] = useState<GraphPayload | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [worldOnly, setWorldOnly] = useState(false);
  const [selection, setSelection] = useState<Selection>(null);
  const [nodes, setNodes, onNodesChange] = useNodesState<MapFlowNode>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<ManhattanFlowEdge>([]);
  const [connectDraft, setConnectDraft] = useState<{
    source: string;
    target: string;
    sourceTile: [number, number] | null;
  } | null>(null);
  const [retargetEdge, setRetargetEdge] = useState<PortalEdgeInfo | null>(null);
  const [toast, setToast] = useState<string | null>(null);
  const [historyVersion, setHistoryVersion] = useState(0);
  const [panelWidth, setPanelWidth] = useState(PANEL_DEFAULT);
  const opStack = useRef(new OpStack());
  const toastTimer = useRef<number | undefined>(undefined);
  const resizingPanel = useRef(false);
  // Positions survive refetches so the layout doesn't jump after each edit;
  // user drags are folded in via onNodesChange before every re-layout.
  const positionsRef = useRef(new Map<string, { x: number; y: number }>());

  const showToast = useCallback((text: string) => {
    setToast(text);
    window.clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToast(null), TOAST_MS);
  }, []);

  const refresh = useCallback(async () => {
    try {
      setGraph(await fetchGraph());
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const selectEdgeById = useCallback((edgeId: string) => {
    setSelection({ kind: "edge", id: edgeId });
    setEdges((prev) =>
      prev.map((e) => ({ ...e, selected: e.id === edgeId })),
    );
  }, [setEdges]);

  useEffect(() => {
    if (!graph) return;
    const visible = (info: MapNodeInfo) => !worldOnly || info.is_world;
    const visibleNodes = graph.nodes.filter(visible);
    const visibleIds = new Set(visibleNodes.map((n) => n.map_id));
    // Layout only maps that don't have a position yet (first load, or maps
    // revealed by toggling the world filter).
    const missing = visibleNodes.filter(
      (n) => !positionsRef.current.has(n.map_id),
    );
    if (missing.length > 0) {
      const fresh = layoutGraph(
        visibleNodes,
        graph.edges.filter(
          (e) => visibleIds.has(e.source) && visibleIds.has(e.target),
        ),
      );
      for (const [id, pos] of fresh) {
        if (!positionsRef.current.has(id)) positionsRef.current.set(id, pos);
      }
    }
    setNodes(
      buildFlowNodes(graph, visible, positionsRef.current, selectEdgeById),
    );
    setEdges(buildFlowEdges(graph, visibleIds));
  }, [graph, worldOnly, setNodes, setEdges, selectEdgeById]);

  // Keep the position cache in sync with user drags.
  const handleNodesChange: typeof onNodesChange = useCallback(
    (changes) => {
      for (const change of changes) {
        if (change.type === "position" && change.position) {
          positionsRef.current.set(change.id, change.position);
        }
      }
      onNodesChange(changes);
    },
    [onNodesChange],
  );

  const nodesById = useMemo(
    () => new Map((graph?.nodes ?? []).map((n) => [n.map_id, n])),
    [graph],
  );
  const edgesById = useMemo(
    () =>
      new Map<string, PortalEdgeInfo>(
        (graph?.edges ?? []).map((e) => [e.id, e]),
      ),
    [graph],
  );

  // ── Manhattan routing: recomputed whenever nodes move or edges change ──
  const routedEdges = useMemo(() => {
    if (!graph) return edges;
    const rects = new Map(
      nodes.map((n) => [
        n.id,
        {
          x: n.position.x,
          y: n.position.y,
          w: n.data.thumbWidth,
          h: n.data.thumbHeight + LABEL_STRIP_H,
        },
      ]),
    );
    const inputs: EdgeRouteInput[] = [];
    for (const edge of edges) {
      const info = edgesById.get(edge.id);
      const srcRect = rects.get(edge.source);
      const dstRect = rects.get(edge.target);
      const srcInfo = nodesById.get(edge.source);
      const dstInfo = nodesById.get(edge.target);
      if (!info || !srcRect || !dstRect || !srcInfo || !dstInfo) continue;
      const [srcMapW, srcMapH] = srcInfo.map_size_px;
      const dstFrac = tileFraction(
        info.target_tile,
        dstInfo.map_size_px,
        dstInfo.tile_size_px,
      );
      if (srcMapW <= 0 || srcMapH <= 0 || !dstFrac) continue;
      const [rx, ry, rw, rh] = info.source_rect_px;
      inputs.push({
        id: edge.id,
        srcInner: [
          srcRect.x + ((rx + rw / 2) / srcMapW) * srcRect.w,
          srcRect.y + ((ry + rh / 2) / srcMapH) * srcRect.h,
        ],
        dstInner: [
          dstRect.x + dstFrac[0] * dstRect.w,
          dstRect.y + dstFrac[1] * dstRect.h,
        ],
        srcRect,
        dstRect,
        pairKey:
          edge.source < edge.target
            ? `${edge.source}|${edge.target}`
            : `${edge.target}|${edge.source}`,
        orderKey: `${edge.source}|${edge.target}|${info.portal_obj_id}`,
      });
    }
    const routes = computeEdgeRoutes(inputs);
    return edges.map((edge) => {
      const route = routes.get(edge.id);
      if (!route) return edge;
      return {
        ...edge,
        data: {
          ...edge.data!,
          path: route.d,
          labelX: route.labelX,
          labelY: route.labelY,
        },
      };
    });
  }, [graph, nodes, edges, edgesById, nodesById]);

  const runMutation = useCallback(
    async (action: () => Promise<string>) => {
      try {
        const message = await action();
        setHistoryVersion((v) => v + 1);
        await refresh();
        showToast(message);
      } catch (e) {
        showToast(`Error: ${e instanceof Error ? e.message : String(e)}`);
      }
    },
    [refresh, showToast],
  );

  const onConnect = useCallback(
    (conn: Connection) => {
      if (!conn.source || !conn.target || conn.source === conn.target) return;
      // Dragging from a portal marker prefills that portal's door tile.
      let sourceTile: [number, number] | null = null;
      if (conn.sourceHandle?.startsWith(MARKER_HANDLE_PREFIX)) {
        const fromEdge = edgesById.get(
          conn.sourceHandle.slice(MARKER_HANDLE_PREFIX.length),
        );
        if (fromEdge) sourceTile = fromEdge.source_tile;
      }
      setConnectDraft({ source: conn.source, target: conn.target, sourceTile });
    },
    [edgesById],
  );

  const createFromDialog = useCallback(
    (request: CreatePortalRequest, reciprocal: CreatePortalRequest | null) => {
      setConnectDraft(null);
      runMutation(async () => {
        const op = {
          kind: "create" as const,
          requests: reciprocal ? [request, reciprocal] : [request],
          createdIds: [],
          label: `create ${request.source_map} → ${request.target_map}`,
        };
        await applyOp(op);
        opStack.current.push(op);
        return `Created portal ${request.source_map} → ${request.target_map}${
          reciprocal ? " (+ return portal)" : ""
        }`;
      });
    },
    [runMutation],
  );

  const retargetFromDialog = useCallback(
    (tile: [number, number]) => {
      const edge = retargetEdge;
      setRetargetEdge(null);
      if (!edge) return;
      runMutation(async () => {
        const op = {
          kind: "retarget" as const,
          sourceMap: edge.source,
          objId: edge.portal_obj_id,
          before: {
            target_map: edge.target,
            target_tile: edge.target_tile,
            source_rect_px: null,
          },
          after: {
            target_map: edge.target,
            target_tile: tile,
            source_rect_px: null,
          },
          label: `move arrival of ${edge.id}`,
        };
        await applyOp(op);
        opStack.current.push(op);
        return `Moved arrival to ${tile[0]},${tile[1]}`;
      });
    },
    [retargetEdge, runMutation],
  );

  const deleteEdge = useCallback(
    (edge: PortalEdgeInfo) => {
      if (
        !window.confirm(
          `Delete portal ${edge.source} → ${edge.target} (object #${edge.portal_obj_id})?`,
        )
      ) {
        return;
      }
      setSelection(null);
      runMutation(async () => {
        const op = {
          kind: "delete" as const,
          edge,
          label: `delete ${edge.id}`,
        };
        await applyOp(op);
        opStack.current.push(op);
        return `Deleted portal ${edge.source} → ${edge.target}`;
      });
    },
    [runMutation],
  );

  const undo = useCallback(() => {
    setSelection(null);
    runMutation(async () => {
      const label = await opStack.current.undo();
      return label ? `Undid: ${label}` : "Nothing to undo";
    });
  }, [runMutation]);

  const redo = useCallback(() => {
    setSelection(null);
    runMutation(async () => {
      const label = await opStack.current.redo();
      return label ? `Redid: ${label}` : "Nothing to redo";
    });
  }, [runMutation]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey)) return;
      if (e.key.toLowerCase() === "z") {
        e.preventDefault();
        if (e.shiftKey) redo();
        else undo();
      } else if (e.key.toLowerCase() === "y") {
        e.preventDefault();
        redo();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [undo, redo]);

  const onNodeClick: NodeMouseHandler = useCallback(
    (_evt, node) => setSelection({ kind: "node", id: node.id }),
    [],
  );
  const onEdgeClick: EdgeMouseHandler = useCallback(
    (_evt, edge) => setSelection({ kind: "edge", id: edge.id }),
    [],
  );
  const onPaneClick = useCallback(() => setSelection(null), []);
  const selectMap = useCallback(
    (mapId: string) => setSelection({ kind: "node", id: mapId }),
    [],
  );

  const startPanelResize = useCallback(
    (e: React.PointerEvent<HTMLDivElement>) => {
      e.preventDefault();
      resizingPanel.current = true;
      const onMove = (ev: PointerEvent) => {
        if (!resizingPanel.current) return;
        const max = Math.max(PANEL_MIN, window.innerWidth - 320);
        setPanelWidth(
          Math.min(max, Math.max(PANEL_MIN, window.innerWidth - ev.clientX)),
        );
      };
      const onUp = () => {
        resizingPanel.current = false;
        window.removeEventListener("pointermove", onMove);
        window.removeEventListener("pointerup", onUp);
      };
      window.addEventListener("pointermove", onMove);
      window.addEventListener("pointerup", onUp);
    },
    [],
  );

  if (error) {
    return (
      <div className="fatal-error">
        <h2>Failed to load graph</h2>
        <p>{error}</p>
        <p>
          Is the backend running? <code>python -m tools.map_editor --web
          --scenario ./rusted_kingdoms</code>
        </p>
      </div>
    );
  }

  const draftSource = connectDraft ? nodesById.get(connectDraft.source) : null;
  const draftTarget = connectDraft ? nodesById.get(connectDraft.target) : null;
  const retargetTargetMap = retargetEdge
    ? nodesById.get(retargetEdge.target)
    : null;
  void historyVersion; // re-render trigger for canUndo/canRedo

  return (
    <div className="app-shell">
      <header className="topbar">
        <span className="title">Map Graph</span>
        <span className="counts">
          {nodes.length}/{graph?.nodes.length ?? 0} maps · {edges.length}/
          {graph?.edges.length ?? 0} portals
        </span>
        <button
          onClick={undo}
          disabled={!opStack.current.canUndo}
          title="Ctrl+Z"
        >
          ⌫ undo
        </button>
        <button
          onClick={redo}
          disabled={!opStack.current.canRedo}
          title="Ctrl+Shift+Z"
        >
          redo ⌦
        </button>
        <span className="hint dim">
          drag a green dot onto another map to link · edits write TMX
          immediately (.bak kept)
        </span>
        <label className="toggle">
          <input
            type="checkbox"
            checked={worldOnly}
            onChange={(e) => setWorldOnly(e.target.checked)}
          />
          world maps only
        </label>
      </header>
      <div className="main-row">
        <div className="flow-wrap">
          <ReactFlow
            nodes={nodes}
            edges={routedEdges}
            nodeTypes={nodeTypes}
            edgeTypes={edgeTypes}
            onNodesChange={handleNodesChange}
            onEdgesChange={onEdgesChange}
            onConnect={onConnect}
            onNodeClick={onNodeClick}
            onEdgeClick={onEdgeClick}
            onPaneClick={onPaneClick}
            fitView
            minZoom={0.05}
            maxZoom={8}
            proOptions={{ hideAttribution: true }}
          >
            <Background gap={24} />
            <Controls />
            <MiniMap pannable zoomable nodeStrokeWidth={3} />
          </ReactFlow>
        </div>
        <div
          className="panel-divider"
          title="Drag to resize the panel"
          onPointerDown={startPanelResize}
        />
        <SidePanel
          selection={selection}
          nodesById={nodesById}
          edgesById={edgesById}
          onSelectMap={selectMap}
          onRetargetEdge={setRetargetEdge}
          onDeleteEdge={deleteEdge}
          width={panelWidth}
        />
      </div>
      {connectDraft && draftSource && draftTarget && (
        <PortalDialog
          source={draftSource}
          target={draftTarget}
          allEdges={graph?.edges ?? []}
          initialSourceTile={connectDraft.sourceTile}
          onCancel={() => setConnectDraft(null)}
          onCreate={createFromDialog}
        />
      )}
      {retargetEdge && retargetTargetMap && (
        <RetargetDialog
          edge={retargetEdge}
          target={retargetTargetMap}
          allEdges={graph?.edges ?? []}
          onCancel={() => setRetargetEdge(null)}
          onRetarget={retargetFromDialog}
        />
      )}
      {toast && <div className="toast">{toast}</div>}
    </div>
  );
}
