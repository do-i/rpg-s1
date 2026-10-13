// Types mirroring the JSON emitted by tools/map_editor/service/editor_service.py,
// plus thin fetch wrappers for the REST API.

export interface NpcInfo {
  npc_id: string;
  name: string | null;
  npc_type: string | null;
  dialogue: string | null;
  position: [number, number] | null;
  sprite: string | null;
}

export interface ItemBoxInfo {
  box_id: string;
  position: [number, number] | null;
}

export interface MapNodeInfo {
  map_id: string;
  revision: string;
  render_revision: string | null;
  display_name: string;
  tmx_file: string;
  yaml_file: string | null;
  is_world: boolean;
  bgm: string | null;
  has_inn: boolean;
  has_shop: boolean;
  has_apothecary: boolean;
  has_magic_core_shop: boolean;
  npcs: NpcInfo[];
  item_boxes: ItemBoxInfo[];
  encounter: Record<string, unknown> | null;
  transport: unknown;
  map_size_px: [number, number];
  tile_size_px: [number, number];
}

export interface PortalEdgeInfo {
  id: string;
  source: string;
  target: string;
  source_tile: [number, number];
  target_tile: [number, number];
  portal_obj_id: number;
  revision?: string;
  source_rect_px: [number, number, number, number];
}

export interface GraphPayload {
  nodes: MapNodeInfo[];
  edges: PortalEdgeInfo[];
  revisionsChanged?: boolean;
}

export class ApiError extends Error {
  constructor(public readonly status: number, message: string) {
    super(message);
  }
}

const revisions = new Map<string, string>();

function editHeaders(sourceMap: string): Record<string, string> {
  const revision = revisions.get(sourceMap);
  if (!revision) throw new Error("Refresh the graph before editing this map.");
  return { "Content-Type": "application/json", "If-Match": revision };
}

async function editedEdge(res: Response): Promise<PortalEdgeInfo> {
  const edge: PortalEdgeInfo = await res.json();
  if (edge.revision) revisions.set(edge.source, edge.revision);
  return edge;
}

async function checkOk(res: Response): Promise<Response> {
  if (!res.ok) {
    let detail = `${res.status} ${res.statusText}`;
    try {
      const body = await res.json();
      if (body.detail) detail = String(body.detail);
    } catch {
      // non-JSON error body; keep the status line
    }
    throw new ApiError(res.status, detail);
  }
  return res;
}

export async function fetchGraph(): Promise<GraphPayload> {
  const res = await checkOk(await fetch("/api/graph", { cache: "no-store" }));
  const graph: GraphPayload = await res.json();
  graph.revisionsChanged = revisions.size > 0 && (
    graph.nodes.length !== revisions.size || graph.nodes.some(
      (node) => revisions.get(node.map_id) !== node.revision,
    )
  );
  revisions.clear();
  for (const node of graph.nodes) revisions.set(node.map_id, node.revision);
  return graph;
}

export function thumbnailUrl(mapId: string, renderRevision: string | null): string {
  return `/api/maps/${encodeURIComponent(mapId)}/thumbnail.png?revision=${encodeURIComponent(renderRevision ?? "unavailable")}`;
}

export function fullRenderUrl(mapId: string, renderRevision: string | null): string {
  return `/api/maps/${encodeURIComponent(mapId)}/full.png?revision=${encodeURIComponent(renderRevision ?? "unavailable")}`;
}

export interface CreatePortalRequest {
  source_map: string;
  source_rect_px: [number, number, number, number];
  target_map: string;
  target_tile: [number, number];
}

export async function createPortal(
  req: CreatePortalRequest,
): Promise<PortalEdgeInfo> {
  const res = await checkOk(
    await fetch("/api/portals", {
      method: "POST",
      headers: editHeaders(req.source_map),
      body: JSON.stringify(req),
    }),
  );
  return editedEdge(res);
}

export interface RetargetPortalRequest {
  target_map: string;
  target_tile: [number, number];
  source_rect_px: [number, number, number, number] | null;
}

export async function retargetPortal(
  sourceMap: string,
  portalObjId: number,
  req: RetargetPortalRequest,
): Promise<PortalEdgeInfo> {
  const res = await checkOk(
    await fetch(
      `/api/portals/${encodeURIComponent(sourceMap)}/${portalObjId}`,
      {
        method: "PATCH",
        headers: editHeaders(sourceMap),
        body: JSON.stringify(req),
      },
    ),
  );
  return editedEdge(res);
}

export async function deletePortal(
  sourceMap: string,
  portalObjId: number,
): Promise<string> {
  const res = await checkOk(
    await fetch(
      `/api/portals/${encodeURIComponent(sourceMap)}/${portalObjId}`,
      { method: "DELETE", headers: editHeaders(sourceMap) },
    ),
  );
  const revision = res.headers.get("X-Map-Revision");
  if (revision) revisions.set(sourceMap, revision);
  return res.headers.get("X-Restore-Token") ?? "";
}

export async function restorePortal(sourceMap: string, token: string): Promise<PortalEdgeInfo> {
  return editedEdge(await checkOk(await fetch("/api/portals/restore", {
    method: "POST", headers: editHeaders(sourceMap), body: JSON.stringify({ token }),
  })));
}
