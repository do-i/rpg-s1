// Custom React Flow node: map thumbnail + portal-tile markers + label.
//
// Green dots sit on the actual portal tiles. Each one is a live handle:
// click it to select that portal's edge, or drag it onto another map to
// create a new portal starting from that door tile. The dot on the right
// edge starts a portal from anywhere. While a connection is being dragged,
// the whole node becomes the drop target.

import { Handle, Position, useConnection } from "@xyflow/react";
import type { NodeProps, Node } from "@xyflow/react";
import { thumbnailUrl } from "./api";

export const MARKER_HANDLE_PREFIX = "portal:";

export interface PortalMarker {
  edgeId: string;
  targetMap: string;
  // Center of the portal rect, in thumbnail-relative fractions [0..1].
  fx: number;
  fy: number;
}

export interface ArrivalMarker {
  edgeId: string;
  fromMap: string;
  fx: number;
  fy: number;
}

export interface MapNodeData extends Record<string, unknown> {
  mapId: string;
  displayName: string;
  isWorld: boolean;
  thumbWidth: number;
  thumbHeight: number;
  markers: PortalMarker[];
  arrivals: ArrivalMarker[];
  badges: string[];
  onSelectEdge: (edgeId: string) => void;
}

export type MapFlowNode = Node<MapNodeData, "map">;

export function MapNode({ data, selected }: NodeProps<MapFlowNode>) {
  const connection = useConnection();
  const isConnectTarget =
    connection.inProgress && connection.fromNode?.id !== data.mapId;

  return (
    <div
      className={`map-node${selected ? " selected" : ""}${
        data.isWorld ? "" : " interior"
      }${isConnectTarget ? " connect-target" : ""}`}
      style={{ width: data.thumbWidth }}
    >
      <div
        className="thumb-wrap"
        style={{ width: data.thumbWidth, height: data.thumbHeight }}
      >
        <img
          src={thumbnailUrl(data.mapId)}
          alt={data.mapId}
          draggable={false}
          width={data.thumbWidth}
          height={data.thumbHeight}
          onError={(e) => {
            // Maps that fail to render server-side keep a plain dark node.
            e.currentTarget.style.visibility = "hidden";
          }}
        />
        {data.arrivals.map((m) => (
          <div
            key={`in:${m.edgeId}`}
            className="arrival-marker"
            title={`arrival ← ${m.fromMap}`}
            style={{ left: `${m.fx * 100}%`, top: `${m.fy * 100}%` }}
          />
        ))}
        {data.markers.map((m) => (
          <Handle
            key={`out:${m.edgeId}`}
            id={`${MARKER_HANDLE_PREFIX}${m.edgeId}`}
            type="source"
            position={Position.Top}
            className="portal-marker-handle"
            title={`portal → ${m.targetMap} — click to select, drag to link`}
            style={{ left: `${m.fx * 100}%`, top: `${m.fy * 100}%` }}
            onClick={(e) => {
              e.stopPropagation();
              data.onSelectEdge(m.edgeId);
            }}
          />
        ))}
      </div>
      <div className="map-node-label">
        <span className="name">{data.displayName}</span>
        {data.badges.length > 0 && (
          <span className="badges">{data.badges.join(" ")}</span>
        )}
      </div>
      <Handle
        type="target"
        position={Position.Left}
        className={isConnectTarget ? "target-cover" : "target-hidden"}
      />
      <Handle
        type="source"
        position={Position.Right}
        className="source-handle"
        title="Drag onto another map to create a portal"
      />
    </div>
  );
}
