"""UI-independent editing service for the map editor.

Wraps scenario loading (`io/scenario_loader`), graph building
(`graph/portal_graph`) and TMX mutation (`edit/tmx_writer`) behind one object
with JSON-serializable inputs/outputs, so any frontend (pygame scene, web UI,
tests) can drive portal editing without touching pygame or ElementTree.

Mutations atomically replace the TMX file (a sibling .bak is created on the
first edit per file) and refresh only the affected map's edges in the
in-memory graph. One service instance in one process owns a scenario's writes.
The reentrant lock covers validation, publication, graph refresh and snapshots.
External writers must coordinate saves; revision checks are not filesystem CAS.
"""

from __future__ import annotations

from contextlib import contextmanager
from functools import wraps
from pathlib import Path
from secrets import token_urlsafe
from threading import RLock

from tools.map_editor.edit.tmx_writer import (
    TMXPublishedDurabilityError,
    TMXConflictError,
    map_revision,
    check_revisions,
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
from tools.map_editor.service.portal_validation import validate_destination, validate_source


class EditorLookupError(ValueError):
    """An unknown map or portal id, distinct from invalid edit input."""


def _serialized(method):
    @wraps(method)
    def guarded(self, *args, **kwargs):
        with self._lock:
            return method(self, *args, **kwargs)
    return guarded


class EditorService:
    MAX_RESTORE_RECEIPTS = 128

    def __init__(self, scenario_root: Path) -> None:
        self._lock = RLock()
        self._deleted_portals: dict[str, GraphEdge] = {}
        self._scenario: ScenarioMaps = load_scenario_maps(scenario_root)
        self._revisions = {path: map_revision(path) for path in self._scenario.tmx_paths}
        self._graph: PortalGraph = build_portal_graph(
            tmx_paths=self._scenario.tmx_paths,
            yaml_for=self._scenario.yaml_for,
        )
        check_revisions(self._revisions)

    @property
    def scenario_root(self) -> Path:
        return self._scenario.scenario_root

    @property
    def graph(self) -> PortalGraph:
        return self._graph

    # ── queries ──────────────────────────────────────────────────────────

    @_serialized
    def graph_dict(self, *, refresh: bool = False) -> dict:
        """The full portal graph as a JSON-serializable dict."""
        if refresh:
            revisions = {path: map_revision(path) for path in self._scenario.tmx_paths}
            fresh = build_portal_graph(
                tmx_paths=self._scenario.tmx_paths, yaml_for=self._scenario.yaml_for,
            )
            check_revisions(revisions)
            self._graph, self._revisions = fresh, revisions
        return {
            "nodes": [{**_node_dict(n), "revision": self._revisions[n.tmx_path]}
                      for n in self._graph.nodes],
            "edges": [_edge_dict(e) for e in self._graph.edges],
        }

    @_serialized
    def node(self, map_id: str) -> GraphNode:
        node = self._graph.nodes_by_id.get(map_id)
        if node is None:
            raise EditorLookupError(
                f"Unknown map id '{map_id}'. Known ids come from the TMX file "
                f"stems under {self._scenario.maps_dir} (example: 'zone1')."
            )
        return node

    # ── mutations ────────────────────────────────────────────────────────

    @_serialized
    def create_portal(
        self,
        source_map: str,
        source_rect_px: tuple[int, int, int, int],
        target_map: str,
        target_tile: tuple[int, int],
        *, expected_revision: str | None = None,
    ) -> dict:
        """Create a portal on `source_map` and return the new edge dict."""
        source = self.node(source_map)
        target = self.node(target_map)
        revisions = self._edit_revisions(source, target, expected_revision)
        validate_source(source.tmx_path, source_rect_px)
        validate_destination(target.tmx_path, target_tile)
        with self._publication(source_map, revisions):
            new_id = create_portal(
                tmx_path=source.tmx_path,
                source_rect_px=source_rect_px,
                new_target_map=target_map,
                new_target_tile=target_tile,
                revisions=revisions,
            )
        return self._edge_result(source_map, new_id)

    @_serialized
    def retarget_portal(
        self,
        source_map: str,
        portal_obj_id: int,
        target_map: str,
        target_tile: tuple[int, int],
        source_rect_px: tuple[int, int, int, int] | None,
        *, expected_revision: str | None = None,
    ) -> dict:
        """Point an existing portal at a new destination (and optionally move
        or resize its source rect). Returns the updated edge dict."""
        source = self.node(source_map)
        target = self.node(target_map)
        revisions = self._edit_revisions(source, target, expected_revision)
        edge = self._edge(source_map, portal_obj_id)
        validate_source(
            source.tmx_path,
            edge.source_rect_px if source_rect_px is None else source_rect_px,
            authored=source_rect_px is None,
        )
        validate_destination(target.tmx_path, target_tile)
        with self._publication(source_map, revisions):
            save_portal_target(
                tmx_path=source.tmx_path,
                portal_obj_id=portal_obj_id,
                new_target_map=target_map,
                new_target_tile=target_tile,
                revisions=revisions,
                new_source_rect_px=source_rect_px,
            )
        return self._edge_result(source_map, portal_obj_id)

    @_serialized
    def delete_portal(self, source_map: str, portal_obj_id: int, *, expected_revision: str | None = None) -> dict:
        source = self.node(source_map)
        revisions = self._edit_revisions(source, source, expected_revision)
        edge = self._edge(source_map, portal_obj_id)
        with self._publication(source_map, revisions):
            delete_portal(tmx_path=source.tmx_path, portal_obj_id=portal_obj_id, revisions=revisions)
        token = token_urlsafe(24)
        if len(self._deleted_portals) >= self.MAX_RESTORE_RECEIPTS:
            del self._deleted_portals[next(iter(self._deleted_portals))]
        self._deleted_portals[token] = edge
        return {"revision": self._revisions[source.tmx_path], "restore_token": token}

    @_serialized
    def restore_portal(self, token: str, *, expected_revision: str | None = None) -> dict:
        """Restore only geometry held from an acknowledged deletion this session."""
        edge = self._deleted_portals.get(token)
        if edge is None:
            raise TMXConflictError(
                "Portal restoration receipt is no longer available (expired, "
                "already used, or editor restarted). Refresh the graph and "
                "review the portal; undo/redo history must be cleared."
            )
        source, target = self.node(edge.source), self.node(edge.target)
        revisions = self._edit_revisions(source, target, expected_revision)
        validate_source(source.tmx_path, edge.source_rect_px, authored=True)
        validate_destination(target.tmx_path, edge.target_tile)
        try:
            with self._publication(edge.source, revisions):
                new_id = create_portal(source.tmx_path, edge.source_rect_px, edge.target,
                                       edge.target_tile, revisions=revisions)
        except TMXPublishedDurabilityError:
            # The restore is already visible; never let this receipt duplicate it.
            del self._deleted_portals[token]
            raise
        del self._deleted_portals[token]
        return self._edge_result(edge.source, new_id)

    # ── internals ────────────────────────────────────────────────────────

    def _edge_result(self, source_map: str, portal_obj_id: int) -> dict:
        path = self.node(source_map).tmx_path
        return {**_edge_dict(self._edge(source_map, portal_obj_id)),
                "revision": self._revisions[path]}

    def _edit_revisions(self, source: GraphNode, target: GraphNode, expected: str | None) -> dict:
        revisions = {path: map_revision(path) for path in (source.tmx_path, target.tmx_path)}
        if expected is not None and revisions[source.tmx_path] != expected:
            self._refresh_map_edges(source.map_id)
            raise TMXConflictError(f"Map {source.tmx_path.name} changed since this graph was loaded. Refresh and review the edit.")
        return revisions

    @contextmanager
    def _publication(self, map_id: str, revisions: dict[Path, str]):
        try:
            yield
        except (TMXPublishedDurabilityError, TMXConflictError):
            # Reflect either a published map or an external edit that won the
            # conflict; the caller must not retry its old intent silently.
            self._refresh_map_edges(map_id)
            raise
        else:
            self._refresh_map_edges(map_id)
            for path, revision in revisions.items():
                if path.stem != map_id and self._revisions[path] != revision:
                    self._refresh_map_edges(path.stem)

    def _edge(self, source_map: str, portal_obj_id: int) -> GraphEdge:
        for edge in self._graph.edges:
            if edge.source == source_map and edge.portal_obj_id == portal_obj_id:
                return edge
        raise EditorLookupError(
            f"No portal with object id {portal_obj_id} on map '{source_map}'. "
            f"Portal ids are the Tiled object ids in the 'portals' object "
            f"group of {source_map}.tmx."
        )

    def _refresh_map_edges(self, map_id: str) -> None:
        """Re-read one map's TMX and replace its outgoing edges in the graph."""
        node = self.node(map_id)
        revision = map_revision(node.tmx_path)
        fresh = build_portal_graph(
            tmx_paths=[node.tmx_path], yaml_for=self._scenario.yaml_for
        )
        check_revisions({node.tmx_path: revision})
        self._revisions[node.tmx_path] = revision
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
