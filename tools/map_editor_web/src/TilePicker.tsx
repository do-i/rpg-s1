// Click-to-pick a tile on a map's full-resolution render.
//
// Shows the native-pixel map image scaled to fit the given box, existing
// portal positions as markers, a hover cell, and the picked cell. Tile
// coordinates are derived from the click position and the map's tile size.

import { useState } from "react";
import type { MapNodeInfo } from "./api";
import { fullRenderUrl } from "./api";

export interface PickerMarker {
  fx: number; // fraction of map width [0..1]
  fy: number;
  title: string;
}

interface Props {
  map: MapNodeInfo;
  picked: [number, number] | null;
  onPick: (tile: [number, number]) => void;
  markers: PickerMarker[];
  maxWidth: number;
  maxHeight: number;
}

export function TilePicker({
  map,
  picked,
  onPick,
  markers,
  maxWidth,
  maxHeight,
}: Props) {
  const [hover, setHover] = useState<[number, number] | null>(null);

  const [mapW, mapH] = map.map_size_px;
  const [tileW, tileH] = map.tile_size_px;
  if (mapW <= 0 || mapH <= 0 || tileW <= 0 || tileH <= 0) {
    return <p>Map “{map.map_id}” has no readable dimensions.</p>;
  }
  // Fit into the box; allow up to 2x upscale so tiny interiors stay clickable.
  const scale = Math.min(maxWidth / mapW, maxHeight / mapH, 2);
  const viewW = Math.round(mapW * scale);
  const viewH = Math.round(mapH * scale);

  const tileFromEvent = (
    e: React.MouseEvent<HTMLDivElement>,
  ): [number, number] => {
    const rect = e.currentTarget.getBoundingClientRect();
    const x = ((e.clientX - rect.left) / rect.width) * mapW;
    const y = ((e.clientY - rect.top) / rect.height) * mapH;
    const col = Math.max(0, Math.min(Math.floor(mapW / tileW) - 1, Math.floor(x / tileW)));
    const row = Math.max(0, Math.min(Math.floor(mapH / tileH) - 1, Math.floor(y / tileH)));
    return [col, row];
  };

  const cellStyle = (tile: [number, number]) => ({
    left: tile[0] * tileW * scale,
    top: tile[1] * tileH * scale,
    width: tileW * scale,
    height: tileH * scale,
  });

  const showGrid = tileW * scale >= 8;

  return (
    <div className="tile-picker">
      <div
        className="tile-picker-canvas"
        style={{ width: viewW, height: viewH }}
        onMouseMove={(e) => setHover(tileFromEvent(e))}
        onMouseLeave={() => setHover(null)}
        onClick={(e) => onPick(tileFromEvent(e))}
      >
        <img
          src={fullRenderUrl(map.map_id)}
          alt={map.map_id}
          width={viewW}
          height={viewH}
          draggable={false}
        />
        {showGrid && (
          <div
            className="tile-grid"
            style={{
              backgroundSize: `${tileW * scale}px ${tileH * scale}px`,
            }}
          />
        )}
        {markers.map((m, i) => (
          <div
            key={i}
            className="portal-marker"
            title={m.title}
            style={{ left: `${m.fx * 100}%`, top: `${m.fy * 100}%` }}
          />
        ))}
        {hover && <div className="hover-cell" style={cellStyle(hover)} />}
        {picked && <div className="picked-cell" style={cellStyle(picked)} />}
      </div>
      <div className="tile-picker-status">
        {picked ? (
          <>
            tile <code>{picked[0]},{picked[1]}</code>
          </>
        ) : hover ? (
          <>
            tile <code>{hover[0]},{hover[1]}</code>
          </>
        ) : (
          "click a tile"
        )}
      </div>
    </div>
  );
}
