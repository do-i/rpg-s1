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
  source_rect_px: [number, number, number, number];
}

export interface GraphPayload {
  nodes: MapNodeInfo[];
  edges: PortalEdgeInfo[];
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
    throw new Error(detail);
  }
  return res;
}

export async function fetchGraph(): Promise<GraphPayload> {
  const res = await checkOk(await fetch("/api/graph"));
  return res.json();
}

export function thumbnailUrl(mapId: string): string {
  return `/api/maps/${encodeURIComponent(mapId)}/thumbnail.png`;
}

export function fullRenderUrl(mapId: string): string {
  return `/api/maps/${encodeURIComponent(mapId)}/full.png`;
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
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(req),
    }),
  );
  return res.json();
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
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(req),
      },
    ),
  );
  return res.json();
}

export async function deletePortal(
  sourceMap: string,
  portalObjId: number,
): Promise<void> {
  await checkOk(
    await fetch(
      `/api/portals/${encodeURIComponent(sourceMap)}/${portalObjId}`,
      { method: "DELETE" },
    ),
  );
}
