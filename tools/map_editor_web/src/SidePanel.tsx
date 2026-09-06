// Right-hand inspector: details for the selected map or portal.

import type { MapNodeInfo, PortalEdgeInfo } from "./api";
import { thumbnailUrl } from "./api";

export type Selection =
  | { kind: "node"; id: string }
  | { kind: "edge"; id: string }
  | null;

interface Props {
  selection: Selection;
  nodesById: Map<string, MapNodeInfo>;
  edgesById: Map<string, PortalEdgeInfo>;
  onSelectMap: (mapId: string) => void;
  onRetargetEdge: (edge: PortalEdgeInfo) => void;
  onDeleteEdge: (edge: PortalEdgeInfo) => void;
  width: number;
}

export function SidePanel({
  selection,
  nodesById,
  edgesById,
  onSelectMap,
  onRetargetEdge,
  onDeleteEdge,
  width,
}: Props) {
  return (
    <aside className="side-panel" style={{ width, minWidth: width }}>
      {selection === null && (
        <div className="panel-hint">
          <h2>Map Editor</h2>
          <p>Click a map or a portal edge to inspect it.</p>
          <p>
            Drag the dot on a map&apos;s right edge onto another map to create
            a portal.
          </p>
        </div>
      )}
      {selection?.kind === "node" && (
        <NodeDetails info={nodesById.get(selection.id)} />
      )}
      {selection?.kind === "edge" && (
        <EdgeDetails
          edge={edgesById.get(selection.id)}
          nodesById={nodesById}
          onSelectMap={onSelectMap}
          onRetargetEdge={onRetargetEdge}
          onDeleteEdge={onDeleteEdge}
        />
      )}
    </aside>
  );
}

function NodeDetails({ info }: { info: MapNodeInfo | undefined }) {
  if (!info) return <p>Map not found.</p>;
  const facilities = [
    info.has_inn && "inn",
    info.has_shop && "shop",
    info.has_apothecary && "apothecary",
    info.has_magic_core_shop && "magic-core shop",
  ].filter(Boolean) as string[];
  return (
    <div>
      <h2>{info.display_name}</h2>
      <img
        className="panel-thumb"
        src={thumbnailUrl(info.map_id)}
        alt={info.map_id}
      />
      <dl>
        <dt>id</dt>
        <dd>
          <code>{info.map_id}</code>
        </dd>
        <dt>files</dt>
        <dd>
          <code>{info.tmx_file}</code>
          {info.yaml_file && (
            <>
              {" · "}
              <code>{info.yaml_file}</code>
            </>
          )}
        </dd>
        <dt>kind</dt>
        <dd>{info.is_world ? "world map" : "interior"}</dd>
        <dt>size</dt>
        <dd>
          {info.map_size_px[0]}×{info.map_size_px[1]} px (tiles{" "}
          {info.tile_size_px[0]}×{info.tile_size_px[1]})
        </dd>
        {info.bgm && (
          <>
            <dt>bgm</dt>
            <dd>
              <code>{info.bgm}</code>
            </dd>
          </>
        )}
        {facilities.length > 0 && (
          <>
            <dt>facilities</dt>
            <dd>{facilities.join(", ")}</dd>
          </>
        )}
        {info.encounter && (
          <>
            <dt>encounters</dt>
            <dd>yes</dd>
          </>
        )}
      </dl>
      {info.npcs.length > 0 && (
        <>
          <h3>NPCs ({info.npcs.length})</h3>
          <ul className="npc-list">
            {info.npcs.map((npc) => (
              <li key={npc.npc_id}>
                <code>{npc.npc_id}</code>
                {npc.name ? ` — ${npc.name}` : ""}
                {npc.dialogue ? (
                  <span className="dim"> ({npc.dialogue})</span>
                ) : null}
              </li>
            ))}
          </ul>
        </>
      )}
      {info.item_boxes.length > 0 && (
        <h3>Item boxes: {info.item_boxes.length}</h3>
      )}
    </div>
  );
}

function EdgeDetails({
  edge,
  nodesById,
  onSelectMap,
  onRetargetEdge,
  onDeleteEdge,
}: {
  edge: PortalEdgeInfo | undefined;
  nodesById: Map<string, MapNodeInfo>;
  onSelectMap: (mapId: string) => void;
  onRetargetEdge: (edge: PortalEdgeInfo) => void;
  onDeleteEdge: (edge: PortalEdgeInfo) => void;
}) {
  if (!edge) return <p>Portal not found.</p>;
  const sourceName = nodesById.get(edge.source)?.display_name ?? edge.source;
  const targetName = nodesById.get(edge.target)?.display_name ?? edge.target;
  return (
    <div>
      <h2>Portal</h2>
      <dl>
        <dt>from</dt>
        <dd>
          <button className="link" onClick={() => onSelectMap(edge.source)}>
            {sourceName}
          </button>{" "}
          @ tile {edge.source_tile[0]},{edge.source_tile[1]}
        </dd>
        <dt>to</dt>
        <dd>
          <button className="link" onClick={() => onSelectMap(edge.target)}>
            {targetName}
          </button>{" "}
          @ tile {edge.target_tile[0]},{edge.target_tile[1]}
        </dd>
        <dt>object id</dt>
        <dd>
          <code>{edge.portal_obj_id}</code> in <code>{edge.source}.tmx</code>
        </dd>
        <dt>area</dt>
        <dd>
          {edge.source_rect_px[2]}×{edge.source_rect_px[3]} px at (
          {edge.source_rect_px[0]}, {edge.source_rect_px[1]})
        </dd>
      </dl>
      <div className="panel-actions">
        <button onClick={() => onRetargetEdge(edge)}>Move arrival tile…</button>
        <button className="danger" onClick={() => onDeleteEdge(edge)}>
          Delete portal
        </button>
      </div>
    </div>
  );
}
