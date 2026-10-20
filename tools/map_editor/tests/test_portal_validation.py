import os
import unittest
from pathlib import Path
from xml.etree import ElementTree as ET

from tools.map_editor.service.editor_service import EditorLookupError, EditorService
from tools.map_editor.service.portal_validation import (
    PortalValidationError, validate_destination, validate_source,
)
from tools.map_editor.tests.support import DisposableScenario, api_request, map_xml


INVALID_INPUTS = (
    ("source_rect_px", (32, 32, -1, 32), "source_rect_px[2]"),
    ("source_rect_px", (32, 32, 32, -1), "source_rect_px[3]"),
    ("source_rect_px", (-33, 32, 32, 32), "source_rect_px[0]"),
    ("source_rect_px", (32, -33, 32, 32), "source_rect_px[1]"),
    ("source_rect_px", (129, 32, 0, 0), "source_rect_px[0]"),
    ("source_rect_px", (32, 129, 0, 0), "source_rect_px[1]"),
    ("source_rect_px", (2**31, 32, 0, 0), "source_rect_px[0]"),
    ("source_rect_px", (32, 32, 2**31 - 1, 0), "source_rect_px[0]"),
    ("target_tile", (-1, 1), "target_tile[0]"),
    ("target_tile", (1, -1), "target_tile[1]"),
    ("target_tile", (4, 1), "target_tile[0]"),
    ("target_tile", (1, 4), "target_tile[1]"),
    ("target_tile", (2**31, 1), "target_tile[0]"),
)


class PortalValidationTests(unittest.TestCase):
    def setUp(self):
        self.scenario = DisposableScenario()
        self.addCleanup(self.scenario.close)
        self.service = self.scenario.service

    def edit(self, operation, **changes):
        fields = dict(source_map="source", source_rect_px=(32, 32, 32, 32),
                      target_map="target", target_tile=(1, 1))
        fields.update(changes)
        if operation == "create":
            return self.service.create_portal(**fields)
        return self.service.retarget_portal(portal_obj_id=1, **fields)

    def test_invalid_geometry_destinations_preserve_every_file_and_graph(self):
        for operation in ("create", "retarget"):
            for field, value, error_field in INVALID_INPUTS:
                with self.subTest(operation=operation, field=field, value=value):
                    before = self.scenario.snapshot()
                    with self.assertRaises(PortalValidationError) as error:
                        self.edit(operation, **{field: value})
                    self.assertEqual(error.exception.field, error_field)
                    self.assertEqual(self.scenario.snapshot(), before)

    def test_no_silent_integer_conversion_in_direct_service(self):
        for operation in ("create", "retarget"):
            for field, base in (("source_rect_px", [32, 32, 32, 32]), ("target_tile", [1, 1])):
                for value in (True, "1", 1.0, 1.5, float("nan"), float("inf"), None):
                    changed = base.copy()
                    changed[0] = value
                    with self.subTest(operation=operation, field=field, value=value):
                        before = self.scenario.snapshot()
                        with self.assertRaises(PortalValidationError) as error:
                            self.edit(operation, **{field: changed})
                        self.assertEqual(error.exception.field, f"{field}[0]")
                        self.assertEqual(self.scenario.snapshot(), before)
            for field, value in (("source_rect_px", (1, 2)), ("target_tile", (1, 2, 3))):
                with self.assertRaises(PortalValidationError):
                    self.edit(operation, **{field: value})

    def test_unknown_ids_are_lookup_errors_before_semantic_validation(self):
        for changes in ({"source_map": "missing"}, {"target_map": "missing"}):
            for operation in ("create", "retarget"):
                with self.assertRaises(EditorLookupError):
                    self.edit(operation, target_tile=(-1, -1), **changes)
        with self.assertRaises(EditorLookupError):
            self.service.retarget_portal("source", 999, "target", (-1, -1), None)

    def test_rectangles_points_lines_and_edge_triggers_reload_exactly(self):
        for rect in ((16, 24, 48, 32), (0, 0, 0, 0), (128, 128, 0, 0),
                     (32, 32, 0, 16), (-4, 32, 12, 32), (124, 32, 12, 32)):
            with self.subTest(rect=rect):
                edge = self.edit("create", source_rect_px=rect)
                self.assertEqual(edge["source_rect_px"], list(rect))
                updated = self.service.retarget_portal("source", edge["portal_obj_id"], "source", (3, 3), None)
                self.assertEqual(updated["source_rect_px"], list(rect))
                reloaded = EditorService(self.scenario.root)
                self.assertEqual(reloaded._edge("source", edge["portal_obj_id"]).source_rect_px, rect)
                moved = self.service.retarget_portal("source", edge["portal_obj_id"], "target", (0, 0), (64, 64, 0, 0))
                self.assertEqual(moved["source_rect_px"], [64, 64, 0, 0])

    def test_dimensions_and_tile_sizes_come_from_each_authored_map(self):
        scenario = DisposableScenario(map_xml(5, 3, 16, 24), map_xml(3, 5, 48, 20))
        self.addCleanup(scenario.close)
        self.scenario, self.service = scenario, scenario.service
        for operation in ("create", "retarget"):
            self.edit(operation, source_rect_px=(70, 60, 10, 12), target_tile=(2, 4))
            before = scenario.snapshot()
            for changes in ({"source_rect_px": (81, 60, 0, 0)},
                            {"source_rect_px": (70, 73, 0, 0)},
                            {"target_tile": (3, 4)}, {"target_tile": (2, 5)}):
                with self.assertRaises(PortalValidationError):
                    self.edit(operation, **changes)
                self.assertEqual(scenario.snapshot(), before)

    def test_fractional_authored_edge_geometry_is_not_rounded_on_plain_retarget(self):
        rect = (-3.5, 30.25, 9.75, 32.125)
        scenario = DisposableScenario(map_xml(rect=rect))
        self.addCleanup(scenario.close)
        edge = scenario.service.retarget_portal("source", 1, "target", (3, 3), None)
        self.assertEqual(edge["source_rect_px"], list(rect))
        self.assertEqual(edge["source_tile"], [-1, 0])
        obj = ET.parse(scenario.source).getroot().find("objectgroup/object")
        self.assertEqual(tuple(float(obj.get(key)) for key in ("x", "y", "width", "height")), rect)

    def test_plain_retarget_rejects_existing_negative_extent_without_publication(self):
        scenario = DisposableScenario(map_xml(rect=(32, 32, -1, 32)))
        self.addCleanup(scenario.close)
        before = scenario.snapshot()
        with self.assertRaises(PortalValidationError):
            scenario.service.retarget_portal("source", 1, "target", (1, 1), None)
        self.assertEqual(scenario.snapshot(), before)

    def test_invalid_authored_dimensions_are_rejected_before_writing(self):
        for map_id in ("source", "target"):
            path = self.scenario.maps / f"{map_id}.tmx"
            original = path.read_bytes()
            for attribute, value in (("width", "0"), ("tilewidth", "0"),
                                     ("tileheight", str(2**31)), ("height", str(2**32)),
                                     ("orientation", "isometric"), ("infinite", "1")):
                root = ET.fromstring(original)
                root.set(attribute, value)
                path.write_bytes(ET.tostring(root))
                for operation in ("create", "retarget"):
                    with self.subTest(map=map_id, attribute=attribute, operation=operation):
                        before = self.scenario.snapshot()
                        with self.assertRaises(PortalValidationError) as error:
                            self.edit(operation)
                        self.assertEqual(error.exception.field, f"{map_id}_map")
                        self.assertEqual(self.scenario.snapshot(), before)
            path.write_bytes(original)

    def test_collision_visibility_and_empty_gid_transform_flags_match_runtime(self):
        target = self.scenario.maps / "target.tmx"
        root = ET.parse(target).getroot()
        layer = root.find("layer[@name='collision']")
        layer.set("visible", "0")
        data = layer.find("data")
        data.text = data.text.replace("0,0,0,0,", "2147483648,0,0,0,", 1)
        target.write_bytes(ET.tostring(root))
        for operation in ("create", "retarget"):
            self.edit(operation, target_tile=(0, 0))
        data.text = data.text.replace("2147483648", "2147483649")
        target.write_bytes(ET.tostring(root))
        before = self.scenario.snapshot()
        for operation in ("create", "retarget"):
            with self.assertRaises(PortalValidationError) as error:
                self.edit(operation, target_tile=(0, 0))
            self.assertEqual(error.exception.field, "target_tile")
            self.assertEqual(self.scenario.snapshot(), before)

    def test_target_collision_policy_matches_runtime_open_tile_policy(self):
        target = self.scenario.maps / "target.tmx"
        original = target.read_bytes()
        for kind in ("missing", "duplicate", "shape", "encoding", "rotation", "blocked",
                     "double-comma", "compression", "bad-number", "nested-data"):
            root = ET.fromstring(original)
            layer = root.find("layer[@name='collision']")
            if kind == "missing":
                root.remove(layer)
            elif kind == "duplicate":
                root.append(ET.fromstring(ET.tostring(layer)))
            elif kind == "shape":
                layer.set("width", "5")
            elif kind == "encoding":
                layer.find("data").set("encoding", "base64")
            elif kind == "double-comma":
                layer.find("data").text = layer.find("data").text.replace("0,0,0,0,", "0,0,0,0,,", 1)
            elif kind == "compression":
                layer.find("data").set("compression", "")
            elif kind == "bad-number":
                layer.find("data").text = layer.find("data").text.replace("0,0,0,0,", "0_0,0,0,0,", 1)
            elif kind == "nested-data":
                ET.SubElement(layer.find("data"), "tile", gid="0")
            else:
                cells = [[0] * 4 for _ in range(4)]
                cells[1][1] = 0x10000000 if kind == "rotation" else 0x80000001
                layer.find("data").text = "\n" + ",\n".join(
                    ",".join(map(str, row)) for row in cells) + "\n"
            target.write_bytes(ET.tostring(root))
            for operation in ("create", "retarget"):
                with self.subTest(kind=kind, operation=operation):
                    before = self.scenario.snapshot()
                    with self.assertRaises(PortalValidationError) as error:
                        self.edit(operation, source_rect_px=None if operation == "retarget" else (32, 32, 32, 32))
                    self.assertEqual(error.exception.field, "target_tile" if kind == "blocked" else "target_map")
                    self.assertEqual(self.scenario.snapshot(), before)

    def test_current_production_portals_remain_valid_including_authored_exceptions(self):
        maps = Path("assets/scenarios/rusted_kingdoms/media/maps")
        count = 0
        for path in maps.glob("*.tmx"):
            root = ET.parse(path).getroot()
            for obj in root.findall("objectgroup[@name='portals']/object"):
                props = {prop.get("name"): prop.get("value", prop.text)
                         for prop in obj.findall("properties/property")}
                if not props:
                    continue
                with self.subTest(map=path.stem, portal=obj.get("id")):
                    validate_source(path, tuple(float(obj.get(key, "0"))
                                               for key in ("x", "y", "width", "height")), authored=True)
                    validate_destination(maps / f"{props['target_map']}.tmx",
                                         (int(props['target_position_x']), int(props['target_position_y'])))
                count += 1
        self.assertEqual(count, 146)


class PortalApiTests(unittest.TestCase):
    def setUp(self):
        os.environ["SDL_AUDIODRIVER"] = "dummy"
        from tools.map_editor.web.server import create_app
        self.scenario = DisposableScenario()
        self.addCleanup(self.scenario.close)
        self.app = create_app(self.scenario.root)

    def body(self, **changes):
        fields = dict(source_map="source", source_rect_px=[32, 32, 32, 32],
                      target_map="target", target_tile=[1, 1])
        fields.update(changes)
        return fields

    def snapshot(self):
        return self.scenario.snapshot()[0], api_request(self.app, "GET", "/api/graph")

    def test_semantic_errors_are_field_specific_422_without_file_or_graph_changes(self):
        for method, path in (("POST", "/api/portals"), ("PATCH", "/api/portals/source/1")):
            for field, value, error_field in INVALID_INPUTS:
                with self.subTest(method=method, field=field, value=value):
                    before = self.snapshot()
                    status, response = api_request(self.app, method, path, self.body(**{field: value}))
                    self.assertEqual(status, 422)
                    self.assertIn(error_field, response["detail"])
                    self.assertEqual(self.snapshot(), before)
            if method == "PATCH":
                status, response = api_request(self.app, method, path, self.body(source_rect_px=None, target_tile=[4, 1]))
                self.assertEqual(status, 422)
                self.assertIn("target_tile[0]", response["detail"])

    def test_transport_rejects_coercion_shapes_and_nonfinite_coordinates(self):
        for method, path in (("POST", "/api/portals"), ("PATCH", "/api/portals/source/1")):
            for field, values in (("source_rect_px", [32, 32, 32, 32]), ("target_tile", [1, 1])):
                for value in (True, "1", 1.0, 1.5, float("nan"), float("inf"), None):
                    changed = values.copy()
                    changed[0] = value
                    with self.subTest(method=method, field=field, value=value):
                        before = self.snapshot()
                        status, response = api_request(self.app, method, path, self.body(**{field: changed}))
                        self.assertEqual(status, 422)
                        self.assertIn(f"{field}[0]", response["detail"])
                        self.assertEqual(self.snapshot(), before)
            for field, value in (("source_rect_px", [1, 2]), ("target_tile", [1, 2, 3])):
                self.assertEqual(api_request(self.app, method, path, self.body(**{field: value}))[0], 422)

    def test_unknown_ids_remain_404(self):
        cases = (("POST", "/api/portals", {"source_map": "missing"}),
                 ("POST", "/api/portals", {"target_map": "missing"}),
                 ("PATCH", "/api/portals/missing/1", {}),
                 ("PATCH", "/api/portals/source/999", {}),
                 ("PATCH", "/api/portals/source/1", {"target_map": "missing"}),
                 ("DELETE", "/api/portals/source/999", {}))
        for method, path, changes in cases:
            before = self.snapshot()
            self.assertEqual(api_request(self.app, method, path, self.body(**changes))[0], 404)
            self.assertEqual(self.snapshot(), before)

    def test_valid_point_create_and_plain_retarget_keep_zero_extents(self):
        status, edge = api_request(self.app, "POST", "/api/portals", self.body(source_rect_px=[64, 64, 0, 0]))
        self.assertEqual(status, 201)
        self.assertEqual(edge["source_rect_px"], [64, 64, 0, 0])
        status, edge = api_request(self.app, "PATCH", f"/api/portals/source/{edge['portal_obj_id']}",
                                   self.body(source_rect_px=None, target_tile=[3, 3]))
        self.assertEqual(status, 200)
        self.assertEqual(edge["source_rect_px"], [64, 64, 0, 0])
        self.assertEqual(edge["target_tile"], [3, 3])

    def test_blocked_arrival_is_422_for_create_and_plain_retarget(self):
        target = self.scenario.maps / "target.tmx"
        root = ET.parse(target).getroot()
        data = root.find("layer[@name='collision']/data")
        data.text = data.text.replace("0,0,0,0,", "1,0,0,0,", 1)
        target.write_bytes(ET.tostring(root))
        for method, path, rect in (("POST", "/api/portals", [32, 32, 32, 32]),
                                   ("PATCH", "/api/portals/source/1", None)):
            before = self.snapshot()
            status, response = api_request(self.app, method, path, self.body(source_rect_px=rect, target_tile=[0, 0]))
            self.assertEqual(status, 422)
            self.assertIn("target_tile", response["detail"])
            self.assertIn("blocked", response["detail"])
            self.assertEqual(self.snapshot(), before)


if __name__ == "__main__":
    unittest.main()
