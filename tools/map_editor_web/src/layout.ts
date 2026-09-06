// Zone-clustered layout for the portal graph.
//
// Maps are grouped by story zone: `zone_NN_*` maps rank NN directly, and
// every other map (towns, interiors, caves) takes the rank of the nearest
// zone map in the portal graph (BFS, ties to the lower zone). Clusters are
// laid out internally with dagre and placed side by side, lowest zone on the
// left, highest on the right; maps with no path to any zone trail at the end.
//
// Node display size mirrors the pygame editor's thumbnail bounds: each map is
// scaled into a THUMB_MAX_W x THUMB_MAX_H box (never upscaled) plus a label
// strip, so layout spacing tracks real thumbnail footprints.

import dagre from "dagre";
import type { MapNodeInfo, PortalEdgeInfo } from "./api";

export const THUMB_MAX_W = 256;
export const THUMB_MAX_H = 192;
export const LABEL_STRIP_H = 22;

const CLUSTER_GAP = 200;
const UNRANKED = Number.POSITIVE_INFINITY;

export interface NodeBox {
  width: number;
  height: number;
  thumbWidth: number;
  thumbHeight: number;
}

export function nodeBox(info: MapNodeInfo): NodeBox {
  const [mapW, mapH] = info.map_size_px;
  const safeW = Math.max(1, mapW);
  const safeH = Math.max(1, mapH);
  const scale = Math.min(THUMB_MAX_W / safeW, THUMB_MAX_H / safeH, 1.0);
  const thumbWidth = Math.max(48, Math.round(safeW * scale));
  const thumbHeight = Math.max(36, Math.round(safeH * scale));
  return {
    width: thumbWidth,
    height: thumbHeight + LABEL_STRIP_H,
    thumbWidth,
    thumbHeight,
  };
}

function directZoneRank(mapId: string): number | null {
  const m = /^zone_(\d+)/.exec(mapId);
  return m ? parseInt(m[1], 10) : null;
}

/** Rank every map by BFS distance to the nearest zone map (ties → lower zone). */
function zoneRanks(
  nodes: MapNodeInfo[],
  edges: PortalEdgeInfo[],
): Map<string, number> {
  const adjacency = new Map<string, string[]>();
  const link = (a: string, b: string) => {
    let list = adjacency.get(a);
    if (!list) {
      list = [];
      adjacency.set(a, list);
    }
    list.push(b);
  };
  for (const edge of edges) {
    if (edge.source === edge.target) continue;
    link(edge.source, edge.target);
    link(edge.target, edge.source);
  }

  const ranks = new Map<string, number>();
  const seeds = nodes
    .map((n) => ({ id: n.map_id, rank: directZoneRank(n.map_id) }))
    .filter((s): s is { id: string; rank: number } => s.rank !== null)
    .sort((a, b) => a.rank - b.rank);
  const queue: string[] = [];
  for (const seed of seeds) {
    ranks.set(seed.id, seed.rank);
    queue.push(seed.id);
  }
  while (queue.length > 0) {
    const current = queue.shift()!;
    const rank = ranks.get(current)!;
    for (const next of adjacency.get(current) ?? []) {
      if (!ranks.has(next)) {
        ranks.set(next, rank);
        queue.push(next);
      }
    }
  }
  for (const node of nodes) {
    if (!ranks.has(node.map_id)) ranks.set(node.map_id, UNRANKED);
  }
  return ranks;
}

function layoutCluster(
  members: MapNodeInfo[],
  edges: PortalEdgeInfo[],
): { positions: Map<string, { x: number; y: number }>; width: number; height: number } {
  const memberIds = new Set(members.map((n) => n.map_id));
  const g = new dagre.graphlib.Graph();
  g.setGraph({ rankdir: "TB", nodesep: 50, ranksep: 70, marginx: 0, marginy: 0 });
  g.setDefaultEdgeLabel(() => ({}));
  for (const node of members) {
    const box = nodeBox(node);
    g.setNode(node.map_id, { width: box.width, height: box.height });
  }
  const seen = new Set<string>();
  for (const edge of edges) {
    if (edge.source === edge.target) continue;
    if (!memberIds.has(edge.source) || !memberIds.has(edge.target)) continue;
    const key =
      edge.source < edge.target
        ? `${edge.source}|${edge.target}`
        : `${edge.target}|${edge.source}`;
    if (seen.has(key)) continue;
    seen.add(key);
    g.setEdge(edge.source, edge.target);
  }
  dagre.layout(g);

  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  const positions = new Map<string, { x: number; y: number }>();
  for (const node of members) {
    const placed = g.node(node.map_id);
    const box = nodeBox(node);
    const x = placed.x - box.width / 2;
    const y = placed.y - box.height / 2;
    positions.set(node.map_id, { x, y });
    minX = Math.min(minX, x);
    minY = Math.min(minY, y);
    maxX = Math.max(maxX, x + box.width);
    maxY = Math.max(maxY, y + box.height);
  }
  // Normalize the cluster to its own (0, 0) origin.
  for (const [id, pos] of positions) {
    positions.set(id, { x: pos.x - minX, y: pos.y - minY });
  }
  return { positions, width: maxX - minX, height: maxY - minY };
}

export function layoutGraph(
  nodes: MapNodeInfo[],
  edges: PortalEdgeInfo[],
): Map<string, { x: number; y: number }> {
  if (nodes.length === 0) return new Map();
  const ranks = zoneRanks(nodes, edges);

  const clusters = new Map<number, MapNodeInfo[]>();
  for (const node of nodes) {
    const rank = ranks.get(node.map_id)!;
    let members = clusters.get(rank);
    if (!members) {
      members = [];
      clusters.set(rank, members);
    }
    members.push(node);
  }
  const orderedRanks = [...clusters.keys()].sort((a, b) => a - b);

  const result = new Map<string, { x: number; y: number }>();
  let xCursor = 0;
  for (const rank of orderedRanks) {
    const members = clusters.get(rank)!;
    const { positions, width, height } = layoutCluster(members, edges);
    for (const [id, pos] of positions) {
      // Vertically center every cluster on y = 0.
      result.set(id, { x: xCursor + pos.x, y: pos.y - height / 2 });
    }
    xCursor += width + CLUSTER_GAP;
  }
  return result;
}
