// Custom edge that renders the centrally-computed Manhattan route (see
// edgeRouting.ts). The wide invisible interaction path makes edges easy to
// click; the tile label appears when the edge is selected.

import { BaseEdge, EdgeLabelRenderer } from "@xyflow/react";
import type { Edge, EdgeProps } from "@xyflow/react";

export interface ManhattanEdgeData extends Record<string, unknown> {
  path: string;
  labelX: number;
  labelY: number;
  label: string;
}

export type ManhattanFlowEdge = Edge<ManhattanEdgeData, "manhattan">;

export function ManhattanEdge({
  id,
  data,
  selected,
  markerEnd,
  sourceX,
  sourceY,
  targetX,
  targetY,
}: EdgeProps<ManhattanFlowEdge>) {
  // Fallback straight line if the route isn't computed yet (first frame).
  const d = data?.path ?? `M ${sourceX} ${sourceY} L ${targetX} ${targetY}`;
  return (
    <>
      <BaseEdge id={id} path={d} markerEnd={markerEnd} interactionWidth={26} />
      {selected && data && (
        <EdgeLabelRenderer>
          <div
            className="edge-label"
            style={{
              transform: `translate(-50%, -50%) translate(${data.labelX}px, ${data.labelY}px)`,
            }}
          >
            {data.label}
          </div>
        </EdgeLabelRenderer>
      )}
    </>
  );
}
