// Manhattan (orthogonal) edge routing, ported from the pygame editor
// (tools/map_editor/scenes/graph_scene.py).
//
// Each portal edge is anchored at its REAL tile positions: the exit point is
// where the ray from the portal's source tile (on the source thumbnail)
// toward the arrival tile crosses the source node's border, and vice versa —
// so edge endpoints reflect the actual map data. Edges between the same pair
// of maps get their own riser "lane" so reciprocal/parallel portals fan out,
// and horizontal segments hop over vertical segments of other edges with a
// small semicircle bump where they cross.

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export type Pt = [number, number];
type Side = "left" | "right" | "top" | "bottom";

export interface EdgeRouteInput {
  id: string;
  srcInner: Pt; // portal tile center inside the source node rect
  dstInner: Pt; // arrival tile center inside the target node rect
  srcRect: Rect;
  dstRect: Rect;
  pairKey: string; // undirected map-pair key, for lane grouping
  orderKey: string; // stable sort key within a lane group
}

export interface RoutedEdge {
  d: string; // SVG path
  labelX: number;
  labelY: number;
}

const STUB = 24; // perpendicular stub so the arrowhead meets the side squarely
const LANE_STEP = 22; // spacing between parallel-edge riser lanes
const JUMP_R = 7; // radius of the crossing bump

const SIDE_NORMAL: Record<Side, Pt> = {
  left: [-1, 0],
  right: [1, 0],
  top: [0, -1],
  bottom: [0, 1],
};

function exitPoint(
  inside: Pt,
  outside: Pt,
  r: Rect,
): { pt: Pt; side: Side } | null {
  const [sx, sy] = inside;
  const dx = outside[0] - sx;
  const dy = outside[1] - sy;
  if (dx === 0 && dy === 0) return null;
  let bestT: number | null = null;
  let side: Side | null = null;
  if (dx > 0) {
    bestT = (r.x + r.w - sx) / dx;
    side = "right";
  } else if (dx < 0) {
    bestT = (r.x - sx) / dx;
    side = "left";
  }
  if (dy > 0) {
    const t = (r.y + r.h - sy) / dy;
    if (bestT === null || t < bestT) {
      bestT = t;
      side = "bottom";
    }
  } else if (dy < 0) {
    const t = (r.y - sy) / dy;
    if (bestT === null || t < bestT) {
      bestT = t;
      side = "top";
    }
  }
  if (bestT === null || bestT <= 0 || side === null) return null;
  return { pt: [sx + bestT * dx, sy + bestT * dy], side };
}

function sideOf(p: Pt, r: Rect): Side {
  const candidates: [Side, number][] = [
    ["left", Math.abs(p[0] - r.x)],
    ["right", Math.abs(p[0] - (r.x + r.w))],
    ["top", Math.abs(p[1] - r.y)],
    ["bottom", Math.abs(p[1] - (r.y + r.h))],
  ];
  candidates.sort((a, b) => a[1] - b[1]);
  return candidates[0][0];
}

/** Drop duplicate and collinear interior points from an axis-aligned path. */
function simplifyOrthogonal(points: Pt[]): Pt[] {
  const out: Pt[] = [];
  for (const p of points) {
    const last = out[out.length - 1];
    if (last && Math.abs(p[0] - last[0]) < 0.5 && Math.abs(p[1] - last[1]) < 0.5) {
      continue;
    }
    out.push(p);
  }
  if (out.length <= 2) return out;
  const res: Pt[] = [out[0]];
  for (let i = 1; i < out.length - 1; i++) {
    const [ax, ay] = res[res.length - 1];
    const [bx, by] = out[i];
    const [cx, cy] = out[i + 1];
    const horizontal = Math.abs(ay - by) < 0.5 && Math.abs(by - cy) < 0.5;
    const vertical = Math.abs(ax - bx) < 0.5 && Math.abs(bx - cx) < 0.5;
    if (horizontal || vertical) continue;
    res.push(out[i]);
  }
  res.push(out[out.length - 1]);
  return res;
}

/** Orthogonal route leaving/entering each node perpendicular to its side. */
function edgePath(
  src: Pt,
  dst: Pt,
  srcSide: Side,
  dstSide: Side,
  laneOffset: number,
): Pt[] {
  const ns = SIDE_NORMAL[srcSide];
  const nd = SIDE_NORMAL[dstSide];
  const a1: Pt = [src[0] + ns[0] * STUB, src[1] + ns[1] * STUB];
  const a2: Pt = [dst[0] + nd[0] * STUB, dst[1] + nd[1] * STUB];
  const srcHoriz = srcSide === "left" || srcSide === "right";
  const dstHoriz = dstSide === "left" || dstSide === "right";

  const pts: Pt[] = [src, a1];
  if (srcHoriz && dstHoriz) {
    const midX = (a1[0] + a2[0]) / 2 + laneOffset;
    pts.push([midX, a1[1]], [midX, a2[1]]);
  } else if (!srcHoriz && !dstHoriz) {
    const midY = (a1[1] + a2[1]) / 2 + laneOffset;
    pts.push([a1[0], midY], [a2[0], midY]);
  } else if (srcHoriz && !dstHoriz) {
    pts.push([a2[0], a1[1]]);
  } else {
    pts.push([a1[0], a2[1]]);
  }
  pts.push(a2, dst);
  return simplifyOrthogonal(pts);
}

interface VSeg {
  id: string;
  x: number;
  y1: number;
  y2: number;
}

/** Point at `frac` of the total arc length along a polyline. */
function pointAlong(path: Pt[], frac: number): Pt {
  const lens: number[] = [];
  let total = 0;
  for (let i = 0; i < path.length - 1; i++) {
    const len = Math.hypot(
      path[i + 1][0] - path[i][0],
      path[i + 1][1] - path[i][1],
    );
    lens.push(len);
    total += len;
  }
  if (total <= 0) return path[0];
  let target = total * frac;
  for (let i = 0; i < lens.length; i++) {
    if (lens[i] >= target) {
      const t = lens[i] > 0 ? target / lens[i] : 0;
      return [
        path[i][0] + (path[i + 1][0] - path[i][0]) * t,
        path[i][1] + (path[i + 1][1] - path[i][1]) * t,
      ];
    }
    target -= lens[i];
  }
  return path[path.length - 1];
}

const fmt = (n: number) => Math.round(n * 10) / 10;

/**
 * Emit the SVG path for one polyline, hopping horizontal segments over
 * vertical segments of OTHER edges with semicircle arcs.
 */
function toSvgPath(id: string, path: Pt[], verticals: VSeg[]): string {
  let d = `M ${fmt(path[0][0])} ${fmt(path[0][1])}`;
  for (let i = 0; i < path.length - 1; i++) {
    const [x1, y1] = path[i];
    const [x2, y2] = path[i + 1];
    const isHorizontal = Math.abs(y1 - y2) <= 1 && Math.abs(x1 - x2) > 1;
    if (!isHorizontal) {
      d += ` L ${fmt(x2)} ${fmt(y2)}`;
      continue;
    }
    const lo = Math.min(x1, x2);
    const hi = Math.max(x1, x2);
    const goingRight = x2 >= x1;
    const crossings = [
      ...new Set(
        verticals
          .filter(
            (v) =>
              v.id !== id &&
              v.x > lo + JUMP_R &&
              v.x < hi - JUMP_R &&
              v.y1 + 1 < y1 &&
              v.y2 - 1 > y1,
          )
          .map((v) => v.x),
      ),
    ].sort((a, b) => (goingRight ? a - b : b - a));
    const sign = goingRight ? 1 : -1;
    const sweep = goingRight ? 1 : 0; // bump always arcs upward
    for (const cx of crossings) {
      d += ` L ${fmt(cx - sign * JUMP_R)} ${fmt(y1)}`;
      d += ` A ${JUMP_R} ${JUMP_R} 0 0 ${sweep} ${fmt(cx + sign * JUMP_R)} ${fmt(y1)}`;
    }
    d += ` L ${fmt(x2)} ${fmt(y2)}`;
  }
  return d;
}

export function computeEdgeRoutes(
  items: EdgeRouteInput[],
): Map<string, RoutedEdge> {
  // Riser lanes: edges between the same pair of maps (either direction)
  // spread symmetrically around the centre line.
  const groups = new Map<string, EdgeRouteInput[]>();
  for (const item of items) {
    const list = groups.get(item.pairKey) ?? [];
    list.push(item);
    groups.set(item.pairKey, list);
  }
  const laneOffsets = new Map<string, number>();
  for (const group of groups.values()) {
    group.sort((a, b) => a.orderKey.localeCompare(b.orderKey));
    group.forEach((item, rank) => {
      laneOffsets.set(item.id, LANE_STEP * (rank - (group.length - 1) / 2));
    });
  }

  const polylines = new Map<string, Pt[]>();
  const verticals: VSeg[] = [];
  for (const item of items) {
    const exit = exitPoint(item.srcInner, item.dstInner, item.srcRect);
    const entry = exitPoint(item.dstInner, item.srcInner, item.dstRect);
    const src = exit ? exit.pt : item.srcInner;
    const srcSide = exit ? exit.side : sideOf(item.srcInner, item.srcRect);
    const dst = entry ? entry.pt : item.dstInner;
    const dstSide = entry ? entry.side : sideOf(item.dstInner, item.dstRect);
    const path = edgePath(
      src,
      dst,
      srcSide,
      dstSide,
      laneOffsets.get(item.id) ?? 0,
    );
    polylines.set(item.id, path);
    for (let i = 0; i < path.length - 1; i++) {
      const [x1, y1] = path[i];
      const [x2, y2] = path[i + 1];
      if (Math.abs(x1 - x2) <= 1 && Math.abs(y1 - y2) > 1) {
        verticals.push({
          id: item.id,
          x: x1,
          y1: Math.min(y1, y2),
          y2: Math.max(y1, y2),
        });
      }
    }
  }

  const routes = new Map<string, RoutedEdge>();
  for (const item of items) {
    const path = polylines.get(item.id);
    if (!path || path.length < 2) continue;
    const [labelX, labelY] = pointAlong(path, 0.5);
    routes.set(item.id, {
      d: toSvgPath(item.id, path, verticals),
      labelX,
      labelY,
    });
  }
  return routes;
}
