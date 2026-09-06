"""UI-independent editing service for the map editor.

Wraps scenario loading (`io/scenario_loader`), graph building
(`graph/portal_graph`) and TMX mutation (`edit/tmx_writer`) behind one object
with JSON-serializable inputs/outputs, so any frontend (pygame scene, web UI,
tests) can drive portal editing without touching pygame or ElementTree.

Mutations write straight to the TMX file (a sibling .bak is created on the
first edit per file) and refresh only the affected map's edges in the
in-memory graph.
"""

from __future__ import annotations

from pathlib import Path

from tools.map_editor.edit.tmx_writer import (
    create_portal,
    delete_portal,
    save_portal_target,
)
from tools.map_editor.graph.portal_graph import (
    GraphEdge,
    GraphNode,
    PortalGraph,
    build_portal_graph,
    is_world_map,
)
from tools.map_editor.io.scenario_loader import ScenarioMaps, load_scenario_maps


class EditorService:
    def __init__(self, scenario_root: Path) -> None:
        self._scenario: ScenarioMaps = load_scenario_maps(scenario_root)
        self._graph: PortalGraph = build_portal_graph(
            tmx_paths=self._scenario.tmx_paths,
            yaml_for=self._scenario.yaml_for,
        )

    @property
    def scenario_root(self) -> Path:
        return self._scenario.scenario_root

    @property
    def graph(self) -> PortalGraph:
        return self._graph

    # ── queries ──────────────────────────────────────────────────────────

    def graph_dict(self) -> dict:
        """The full portal graph as a JSON-serializable dict."""
        return {
            "nodes": [_node_dict(n) for n in self._graph.nodes],
            "edges": [_edge_dict(e) for e in self._graph.edges],
        }

    def node(self, map_id: str) -> GraphNode:
        node = self._graph.nodes_by_id.get(map_id)
        if node is None:
            raise ValueError(
                f"Unknown map id '{map_id}'. Known ids come from the TMX file "
                f"stems under {self._scenario.maps_dir} (example: 'zone1')."
            )
        return node

    # ── mutations ────────────────────────────────────────────────────────

    def create_portal(
        self,
        source_map: str,
        source_rect_px: tuple[int, int, int, int],
        target_map: str,
        target_tile: tuple[int, int],
    ) -> dict:
        """Create a portal on `source_map` and return the new edge dict."""
        source = self.node(source_map)
        self.node(target_map)  # validate the target exists
        new_id = create_portal(
            tmx_path=source.tmx_path,
            source_rect_px=source_rect_px,
            new_target_map=target_map,
            new_target_tile=target_tile,
        )
        self._refresh_map_edges(source_map)
        return _edge_dict(self._edge(source_map, new_id))

    def retarget_portal(
        self,
        source_map: str,
        portal_obj_id: int,
        target_map: str,
        target_tile: tuple[int, int],
        source_rect_px: tuple[int, int, int, int] | None,
    ) -> dict:
        """Point an existing portal at a new destination (and optionally move
        or resize its source rect). Returns the updated edge dict."""
        source = self.node(source_map)
        self.node(target_map)
        self._edge(source_map, portal_obj_id)  # validate the portal exists
        save_portal_target(
            tmx_path=source.tmx_path,
            portal_obj_id=portal_obj_id,
            new_target_map=target_map,
            new_target_tile=target_tile,
            new_source_rect_px=source_rect_px,
        )
        self._refresh_map_edges(source_map)
        return _edge_dict(self._edge(source_map, portal_obj_id))

    def delete_portal(self, source_map: str, portal_obj_id: int) -> None:
        source = self.node(source_map)
        self._edge(source_map, portal_obj_id)
        delete_portal(tmx_path=source.tmx_path, portal_obj_id=portal_obj_id)
        self._refresh_map_edges(source_map)

    # ── internals ────────────────────────────────────────────────────────

    def _edge(self, source_map: str, portal_obj_id: int) -> GraphEdge:
        for edge in self._graph.edges:
            if edge.source == source_map and edge.portal_obj_id == portal_obj_id:
                return edge
        raise ValueError(
            f"No portal with object id {portal_obj_id} on map '{source_map}'. "
            f"Portal ids are the Tiled object ids in the 'portals' object "
            f"group of {source_map}.tmx."
        )

    def _refresh_map_edges(self, map_id: str) -> None:
        """Re-read one map's TMX and replace its outgoing edges in the graph."""
        node = self.node(map_id)
        fresh = build_portal_graph(
            tmx_paths=[node.tmx_path], yaml_for=self._scenario.yaml_for
        )
        kept = [e for e in self._graph.edges if e.source != map_id]
        self._graph.edges[:] = kept + fresh.edges
        # Map metadata (size, tile size) may have changed too.
        if fresh.nodes:
            self._graph.nodes_by_id[map_id] = fresh.nodes[0]
            self._graph.nodes[:] = [
                fresh.nodes[0] if n.map_id == map_id else n
                for n in self._graph.nodes
            ]


def _node_dict(node: GraphNode) -> dict:
    return {
        "map_id": node.map_id,
        "display_name": node.display_name,
        "tmx_file": node.tmx_path.name,
        "yaml_file": node.yaml_path.name if node.yaml_path else None,
        "is_world": is_world_map(node),
        "bgm": node.bgm,
        "has_inn": node.has_inn,
        "has_shop": node.has_shop,
        "has_apothecary": node.has_apothecary,
        "has_magic_core_shop": node.has_magic_core_shop,
        "npcs": [
            {
                "npc_id": npc.npc_id,
                "name": npc.name,
                "npc_type": npc.npc_type,
                "dialogue": npc.dialogue,
                "position": list(npc.position) if npc.position else None,
                "sprite": npc.sprite,
            }
            for npc in node.npcs
        ],
        "item_boxes": [
            {
                "box_id": box.box_id,
                "position": list(box.position) if box.position else None,
            }
            for box in node.item_boxes
        ],
        "encounter": node.encounter,
        "transport": node.transport,
        "map_size_px": list(node.map_size_px),
        "tile_size_px": list(node.tile_size_px),
    }


def _edge_dict(edge: GraphEdge) -> dict:
    return {
        "id": f"{edge.source}#{edge.portal_obj_id}",
        "source": edge.source,
        "target": edge.target,
        "source_tile": list(edge.source_tile),
        "target_tile": list(edge.target_tile),
        "portal_obj_id": edge.portal_obj_id,
        "source_rect_px": list(edge.source_rect_px),
    }
