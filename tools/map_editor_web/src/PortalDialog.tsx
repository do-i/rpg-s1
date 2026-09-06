// Placement dialog after a drag-to-connect gesture: pick the door tile on
// the source map and the arrival tile on the destination map, side by side,
// with an option to create the reciprocal (return) portal in one go.

import { useState } from "react";
import type {
  CreatePortalRequest,
  MapNodeInfo,
  PortalEdgeInfo,
} from "./api";
import { TilePicker } from "./TilePicker";
import type { PickerMarker } from "./TilePicker";

interface Props {
  source: MapNodeInfo;
  target: MapNodeInfo;
  allEdges: PortalEdgeInfo[];
  // Prefilled door tile (set when the drag started from a portal marker).
  initialSourceTile: [number, number] | null;
  onCancel: () => void;
  onCreate: (
    request: CreatePortalRequest,
    reciprocal: CreatePortalRequest | null,
  ) => void;
}

function portalMarkers(mapId: string, edges: PortalEdgeInfo[], mapSize: [number, number]): PickerMarker[] {
  const [w, h] = mapSize;
  if (w <= 0 || h <= 0) return [];
  return edges
    .filter((e) => e.source === mapId)
    .map((e) => ({
      fx: (e.source_rect_px[0] + e.source_rect_px[2] / 2) / w,
      fy: (e.source_rect_px[1] + e.source_rect_px[3] / 2) / h,
      title: `existing portal → ${e.target}`,
    }));
}

export function PortalDialog({
  source,
  target,
  allEdges,
  initialSourceTile,
  onCancel,
  onCreate,
}: Props) {
  const [sourceTile, setSourceTile] = useState<[number, number] | null>(
    initialSourceTile,
  );
  const [targetTile, setTargetTile] = useState<[number, number] | null>(null);
  const [reciprocal, setReciprocal] = useState(true);

  const [sTileW, sTileH] = source.tile_size_px;
  const [tTileW, tTileH] = target.tile_size_px;

  const create = () => {
    if (!sourceTile || !targetTile) return;
    const request: CreatePortalRequest = {
      source_map: source.map_id,
      source_rect_px: [
        sourceTile[0] * sTileW,
        sourceTile[1] * sTileH,
        sTileW,
        sTileH,
      ],
      target_map: target.map_id,
      target_tile: targetTile,
    };
    const back: CreatePortalRequest | null = reciprocal
      ? {
          source_map: target.map_id,
          source_rect_px: [
            targetTile[0] * tTileW,
            targetTile[1] * tTileH,
            tTileW,
            tTileH,
          ],
          target_map: source.map_id,
          target_tile: sourceTile,
        }
      : null;
    onCreate(request, back);
  };

  return (
    <div className="modal-backdrop" onClick={onCancel}>
      <div className="modal portal-dialog" onClick={(e) => e.stopPropagation()}>
        <h2>
          New portal: {source.display_name} → {target.display_name}
        </h2>
        <div className="picker-row">
          <div className="picker-col">
            <h3>
              1. Door tile on <code>{source.map_id}</code>
            </h3>
            <TilePicker
              map={source}
              picked={sourceTile}
              onPick={setSourceTile}
              markers={portalMarkers(source.map_id, allEdges, source.map_size_px)}
              maxWidth={430}
              maxHeight={380}
            />
          </div>
          <div className="picker-col">
            <h3>
              2. Arrival tile on <code>{target.map_id}</code>
            </h3>
            <TilePicker
              map={target}
              picked={targetTile}
              onPick={setTargetTile}
              markers={portalMarkers(target.map_id, allEdges, target.map_size_px)}
              maxWidth={430}
              maxHeight={380}
            />
          </div>
        </div>
        <div className="dialog-footer">
          <label className="toggle">
            <input
              type="checkbox"
              checked={reciprocal}
              onChange={(e) => setReciprocal(e.target.checked)}
            />
            also create return portal ({target.display_name} →{" "}
            {source.display_name})
          </label>
          <div className="dialog-buttons">
            <button onClick={onCancel}>Cancel</button>
            <button
              className="primary"
              disabled={!sourceTile || !targetTile}
              onClick={create}
            >
              Create portal{reciprocal ? "s" : ""}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

// Retarget flow: pick a new arrival tile for an existing portal.
interface RetargetProps {
  edge: PortalEdgeInfo;
  target: MapNodeInfo;
  allEdges: PortalEdgeInfo[];
  onCancel: () => void;
  onRetarget: (tile: [number, number]) => void;
}

export function RetargetDialog({
  edge,
  target,
  allEdges,
  onCancel,
  onRetarget,
}: RetargetProps) {
  const [tile, setTile] = useState<[number, number] | null>(null);
  return (
    <div className="modal-backdrop" onClick={onCancel}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <h2>
          Move arrival: {edge.source} → {edge.target}
        </h2>
        <p className="dim">
          Currently arrives at tile {edge.target_tile[0]},{edge.target_tile[1]}.
          Pick the new arrival tile.
        </p>
        <TilePicker
          map={target}
          picked={tile}
          onPick={setTile}
          markers={portalMarkers(target.map_id, allEdges, target.map_size_px)}
          maxWidth={640}
          maxHeight={460}
        />
        <div className="dialog-footer">
          <span />
          <div className="dialog-buttons">
            <button onClick={onCancel}>Cancel</button>
            <button
              className="primary"
              disabled={!tile}
              onClick={() => tile && onRetarget(tile)}
            >
              Move arrival
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
