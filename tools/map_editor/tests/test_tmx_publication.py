import json
import os
import unittest
from unittest.mock import patch
from xml.etree import ElementTree as ET

from tools.map_editor.edit import tmx_writer
from tools.map_editor.service.editor_service import EditorService
from tools.map_editor.tests.support import DisposableScenario


class TMXPublicationTests(unittest.TestCase):
    def setUp(self):
        self.scenario = DisposableScenario()
        self.addCleanup(self.scenario.close)
        self.service = self.scenario.service
        self.path = self.scenario.source
        self.original = self.path.read_bytes()
        self.graph = self.graph_bytes()

    def graph_bytes(self):
        return json.dumps(self.service.graph_dict(), sort_keys=True).encode()

    def edit(self, operation):
        if operation == "create":
            return self.service.create_portal("source", (64, 32, 32, 32), "target", (2, 1))
        if operation == "retarget":
            return self.service.retarget_portal("source", 1, "target", (2, 1), (64, 32, 32, 32))
        return self.service.delete_portal("source", 1)

    def assert_original_intact(self):
        self.assertEqual(self.path.read_bytes(), self.original)
        ET.parse(self.path)
        self.assertEqual(self.graph_bytes(), self.graph)
        self.assertFalse(list(self.path.parent.glob(".*.tmp")))

    def test_interrupted_serialization_preserves_map_and_graph(self):
        def interrupt(tree, destination, **kwargs):
            if hasattr(destination, "write"):
                destination.write(b"<?xml")
            else:
                destination.write_bytes(b"<?xml")
            raise InterruptedError("interrupted serializer")

        for operation in ("create", "retarget", "delete"):
            self.path.write_bytes(self.original)
            self.service = EditorService(self.scenario.root)
            with self.subTest(operation=operation), patch.object(ET.ElementTree, "write", interrupt):
                with self.assertRaises(InterruptedError):
                    self.edit(operation)
                self.assert_original_intact()

    def test_invalid_serialization_preserves_map_and_graph(self):
        for data in (b"<?xml", b"<map/>"):
            def corrupt(tree, destination, **kwargs):
                if hasattr(destination, "write"):
                    destination.write(data)
                else:
                    destination.write_bytes(data)

            for operation in ("create", "retarget", "delete"):
                self.path.write_bytes(self.original)
                self.service = EditorService(self.scenario.root)
                with self.subTest(data=data, operation=operation), patch.object(ET.ElementTree, "write", corrupt):
                    with self.assertRaises((ET.ParseError, ValueError)):
                        self.edit(operation)
                    self.assert_original_intact()

    def test_successful_edits_reload_and_keep_first_backup(self):
        self.path.chmod(0o640)
        created = self.edit("create")
        self.assertEqual(created["portal_obj_id"], 2)
        self.assertEqual(created["target_tile"], [2, 1])
        self.assertEqual(self.path.stat().st_mode & 0o777, 0o640)
        for operation in ("create", "retarget", "delete"):
            self.edit(operation)
            reloaded = EditorService(self.scenario.root)
            expected = reloaded.graph_dict()
            actual = self.service.graph_dict()
            expected["edges"].sort(key=lambda edge: edge["id"])
            actual["edges"].sort(key=lambda edge: edge["id"])
            self.assertEqual(expected, actual)
            self.assertEqual(self.path.with_suffix(".tmx.bak").read_bytes(), self.original)
            self.assertFalse(list(self.path.parent.glob(".*.tmp")))

    def test_failures_before_replacement_preserve_map_and_graph(self):
        failure = OSError("injected persistence failure")
        for operation in ("create", "retarget", "delete"):
            for target in ("os.fsync", "shutil.copy2", "os.replace", "os.link", "_sync_directory"):
                with self.subTest(operation=operation, target=target):
                    self.path.with_suffix(".tmx.bak").unlink(missing_ok=True)
                    with patch(f"{tmx_writer.__name__}.{target}", side_effect=failure):
                        with self.assertRaises(OSError):
                            self.edit(operation)
                    self.assert_original_intact()
                    backup = self.path.with_suffix(".tmx.bak")
                    if backup.exists():
                        self.assertEqual(backup.read_bytes(), self.original)

    def test_interrupted_backup_does_not_claim_backup_name(self):
        def interrupt(source, destination):
            destination.write_bytes(b"<?xml")
            raise InterruptedError("interrupted backup copy")

        with patch.object(tmx_writer.shutil, "copy2", interrupt):
            with self.assertRaises(InterruptedError):
                self.edit("create")
        self.assert_original_intact()
        self.assertFalse(self.path.with_suffix(".tmx.bak").exists())
        self.edit("create")
        self.assertEqual(self.path.with_suffix(".tmx.bak").read_bytes(), self.original)

    def test_invalid_backup_does_not_publish_map(self):
        def corrupt(source, destination):
            destination.write_bytes(b"<?xml")

        with patch.object(tmx_writer.shutil, "copy2", corrupt):
            with self.assertRaises(ValueError):
                self.edit("retarget")
        self.assert_original_intact()
        self.assertFalse(self.path.with_suffix(".tmx.bak").exists())

    def test_backup_sync_failure_does_not_claim_backup_name(self):
        fsync = tmx_writer.os.fsync
        calls = 0

        def fail_backup(descriptor):
            nonlocal calls
            calls += 1
            if calls == 2:
                raise OSError("backup sync failed")
            return fsync(descriptor)

        with patch.object(tmx_writer.os, "fsync", fail_backup):
            with self.assertRaises(OSError):
                self.edit("delete")
        self.assert_original_intact()
        self.assertFalse(self.path.with_suffix(".tmx.bak").exists())

    def test_replacement_observes_synced_verified_map_and_backup(self):
        replace = tmx_writer.os.replace
        fsync = tmx_writer.os.fsync
        synced = []

        def sync(descriptor):
            synced.append(os.fstat(descriptor).st_ino)
            fsync(descriptor)

        def publish(temporary, destination):
            self.assertEqual(temporary.parent, self.path.parent)
            self.assertNotEqual(temporary, self.path)
            self.assertIn(temporary.stat().st_ino, synced)
            ET.parse(temporary)
            self.assertEqual(self.path.read_bytes(), self.original)
            self.assertEqual(self.graph_bytes(), self.graph)
            backup = self.path.with_suffix(".tmx.bak")
            self.assertEqual(backup.read_bytes(), self.original)
            self.assertIn(backup.stat().st_ino, synced)
            self.assertIn(self.path.parent.stat().st_ino, synced)
            replace(temporary, destination)

        with patch.object(tmx_writer.os, "replace", publish), patch.object(tmx_writer.os, "fsync", sync):
            self.edit("create")

    def test_post_replacement_sync_error_refreshes_graph_and_reports_saved(self):
        from fastapi import HTTPException
        from tools.map_editor.web.server import _or_404

        for operation in ("create", "retarget", "delete"):
            with self.subTest(operation=operation):
                self.path.write_bytes(self.original)
                self.service = EditorService(self.scenario.root)
                self.path.with_suffix(".tmx.bak").write_bytes(self.original)
                with patch.object(tmx_writer, "_sync_directory", side_effect=OSError("sync failed")):
                    with self.assertRaises(HTTPException) as error:
                        _or_404(lambda: self.edit(operation))
                self.assertEqual(error.exception.status_code, 500)
                self.assertIn("was saved", error.exception.detail)
                self.assertNotEqual(self.path.read_bytes(), self.original)
                reloaded = EditorService(self.scenario.root).graph_dict()
                actual = self.service.graph_dict()
                reloaded["edges"].sort(key=lambda edge: edge["id"])
                actual["edges"].sort(key=lambda edge: edge["id"])
                self.assertEqual(actual, reloaded)
                self.assertFalse(list(self.path.parent.glob(".*.tmp")))


if __name__ == "__main__":
    unittest.main()
