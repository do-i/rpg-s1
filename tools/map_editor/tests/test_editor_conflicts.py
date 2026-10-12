"""Bounded concurrency and external-edit probes against disposable maps."""

from concurrent.futures import ThreadPoolExecutor
from threading import Event
import os
import unittest
from unittest.mock import patch
from xml.etree import ElementTree as ET

from tools.map_editor.edit import tmx_writer
from tools.map_editor.edit.tmx_writer import TMXConflictError, map_revision
from tools.map_editor.service import editor_service
from tools.map_editor.service.editor_service import EditorService
from tools.map_editor.tests.support import DisposableScenario, api_request, map_xml


class EditorConflictTests(unittest.TestCase):
    def setUp(self):
        self.scenario = DisposableScenario()
        self.addCleanup(self.scenario.close)
        self.service, self.path = self.scenario.service, self.scenario.source

    def create(self, x=64, **kwargs):
        return self.service.create_portal('source', (x, 32, 32, 32), 'target', (2, 1), **kwargs)

    def assert_consistent(self):
        expected = EditorService(self.scenario.root).graph_dict()
        actual = self.service.graph_dict()
        expected['edges'].sort(key=lambda edge: edge['id'])
        actual['edges'].sort(key=lambda edge: edge['id'])
        self.assertEqual(actual, expected)
        self.assertFalse(list(self.path.parent.glob('.*.tmp')))

    def run_overlapping(self, first, second):
        """Hold first publication while another call tries to enter the service."""
        entered, release, second_started = Event(), Event(), Event()
        publish = tmx_writer._publish_tmx
        calls = 0

        def held(*args):
            nonlocal calls
            calls += 1
            if calls == 1:
                entered.set()
                if not release.wait(5):
                    raise TimeoutError('publication was not released')
            return publish(*args)

        def following():
            second_started.set()
            return second()

        with patch.object(tmx_writer, '_publish_tmx', held), ThreadPoolExecutor(2) as pool:
            a = pool.submit(first)
            try:
                self.assertTrue(entered.wait(5))
                b = pool.submit(following)
                self.assertTrue(second_started.wait(5))
                # A query cannot observe publication before its graph refresh.
                self.assertFalse(b.done())
            finally:
                release.set()
            return a.result(timeout=5), b.result(timeout=5)

    def test_concurrent_creates_keep_unique_ids_and_graph(self):
        a, b = self.run_overlapping(lambda: self.create(32), self.create)
        self.assertEqual([a['portal_obj_id'], b['portal_obj_id']], [2, 3])
        self.assertEqual([obj.get('id') for obj in ET.parse(self.path).getroot().findall('objectgroup/object')], ['1', '2', '3'])
        self.assert_consistent()

    def test_retarget_then_delete_is_serial_and_does_not_resurrect(self):
        edge, receipt = self.run_overlapping(
            lambda: self.service.retarget_portal('source', 1, 'target', (2, 1), None),
            lambda: self.service.delete_portal('source', 1),
        )
        self.assertEqual(edge['target_tile'], [2, 1])
        self.assertTrue(receipt['restore_token'])
        self.assertEqual(ET.parse(self.path).getroot().findall('objectgroup/object'), [])
        self.assert_consistent()

    def test_graph_reader_waits_for_publication_and_refresh(self):
        edge, graph = self.run_overlapping(self.create, self.service.graph_dict)
        self.assertIn(edge['portal_obj_id'], [item['portal_obj_id'] for item in graph['edges'] if item['source'] == 'source'])
        self.assertEqual(graph['nodes'][0]['revision'], map_revision(self.path))
        self.assert_consistent()

    def test_same_browser_revision_conflicts_after_concurrent_create(self):
        revision = map_revision(self.path)
        def stale():
            with self.assertRaises(TMXConflictError):
                self.create(32, expected_revision=revision)
        edge, _ = self.run_overlapping(lambda: self.create(expected_revision=revision), stale)
        self.assertEqual(edge['portal_obj_id'], 2)
        self.assert_consistent()

    def test_delete_first_rejects_stale_retarget_without_resurrection(self):
        revision = map_revision(self.path)
        def stale():
            with self.assertRaises(TMXConflictError):
                self.service.retarget_portal('source', 1, 'target', (2, 1), None, expected_revision=revision)
        self.run_overlapping(lambda: self.service.delete_portal('source', 1, expected_revision=revision), stale)
        self.assertEqual(ET.parse(self.path).getroot().findall('objectgroup/object'), [])
        self.assert_consistent()

    def external_edit(self, path):
        root = ET.parse(path).getroot()
        root.set('tiled-edit', 'preserve-me')
        path.write_bytes(ET.tostring(root))
        return path.read_bytes()

    def test_external_change_between_read_and_replace_wins_for_all_mutations(self):
        for operation in ('create', 'retarget', 'delete'):
            scenario = DisposableScenario()
            self.addCleanup(scenario.close)
            self.scenario, self.service, self.path = scenario, scenario.service, scenario.source
            external = None
            backup = tmx_writer._ensure_backup
            def tiled_save(path):
                nonlocal external
                backup(path)
                external = self.external_edit(path)
            with self.subTest(operation=operation), patch.object(tmx_writer, '_ensure_backup', tiled_save):
                with self.assertRaises(TMXConflictError):
                    if operation == 'create':
                        self.create()
                    elif operation == 'retarget':
                        self.service.retarget_portal('source', 1, 'target', (2, 1), None)
                    else:
                        self.service.delete_portal('source', 1)
            self.assertEqual(self.path.read_bytes(), external)
            self.assert_consistent()

    def test_source_or_destination_change_during_validation_is_not_accepted_as_revision(self):
        for changed_map in ('source', 'target'):
            scenario = DisposableScenario()
            self.addCleanup(scenario.close)
            self.scenario, self.service, self.path = scenario, scenario.service, scenario.source
            changed = scenario.maps / f'{changed_map}.tmx'
            validate = editor_service.validate_destination
            external = None
            def tiled_save(path, tile):
                nonlocal external
                validate(path, tile)
                external = self.external_edit(changed)
            with self.subTest(map=changed_map), patch.object(editor_service, 'validate_destination', tiled_save):
                with self.assertRaises(TMXConflictError):
                    self.create()
            self.assertEqual(changed.read_bytes(), external)
            self.service.graph_dict(refresh=True)
            self.assert_consistent()

    def test_successful_mutation_also_refreshes_preexisting_external_target_edges(self):
        target = self.scenario.maps / 'target.tmx'
        tmx_writer.create_portal(target, (64, 32, 32, 32), 'source', (1, 1))
        self.create()
        self.assert_consistent()

    def test_disk_content_revision_detects_change_with_same_mtime(self):
        revision, before = map_revision(self.path), self.path.stat()
        self.external_edit(self.path)
        os.utime(self.path, ns=(before.st_atime_ns, before.st_mtime_ns))
        with self.assertRaises(TMXConflictError):
            self.create(expected_revision=revision)
        self.assert_consistent()

    def test_low_level_writer_also_rejects_external_edit(self):
        publish = tmx_writer._publish_tmx
        external = None
        def tiled_save(*args):
            nonlocal external
            external = self.external_edit(self.path)
            return publish(*args)
        with patch.object(tmx_writer, '_publish_tmx', tiled_save), self.assertRaises(TMXConflictError):
            tmx_writer.create_portal(self.path, (64, 32, 32, 32), 'target', (2, 1))
        self.assertEqual(self.path.read_bytes(), external)
        self.assertFalse(list(self.path.parent.glob('.*.tmp')))

    def app(self):
        from tools.map_editor.web.server import create_app
        with patch('tools.map_editor.web.server.EditorService', return_value=self.service):
            return create_app(self.scenario.root)

    def test_api_returns_409_and_get_graph_refreshes_external_changes(self):
        app, revision = self.app(), map_revision(self.path)
        self.external_edit(self.path)
        status, body = api_request(app, 'POST', '/api/portals', {
            'source_map': 'source', 'source_rect_px': [64, 32, 32, 32],
            'target_map': 'target', 'target_tile': [2, 1],
        }, headers=[(b'if-match', revision.encode())])
        self.assertEqual(status, 409)
        self.assertIn('Refresh', body['detail'])
        status, body = api_request(app, 'GET', '/api/graph')
        self.assertEqual(status, 200)
        self.assertEqual(body['nodes'][0]['revision'], map_revision(self.path))
        self.assert_consistent()

    def test_api_detects_external_edit_during_publication_without_header(self):
        app = self.app()
        backup = tmx_writer._ensure_backup
        def tiled_save(path):
            backup(path)
            self.external_edit(path)
        with patch.object(tmx_writer, '_ensure_backup', tiled_save):
            status, body = api_request(app, 'DELETE', '/api/portals/source/1')
        self.assertEqual(status, 409)
        self.assertIn('changed', body['detail'])
        self.assert_consistent()

    def test_fractional_deleted_portal_restoration_preserves_geometry_and_revision_guards(self):
        rect = (-3.5, 30.25, 9.75, 32.125)
        scenario = DisposableScenario(map_xml(rect=rect))
        self.addCleanup(scenario.close)
        self.scenario, self.service, self.path = scenario, scenario.service, scenario.source
        app = self.app()
        status, _, headers = api_request(app, 'DELETE', '/api/portals/source/1',
                                        headers=[(b'if-match', map_revision(self.path).encode())], include_headers=True)
        self.assertEqual(status, 204)
        token, revision = headers[b'x-restore-token'].decode(), headers[b'x-map-revision'].decode()
        status, body = api_request(app, 'POST', '/api/portals/restore', {'token': token},
                                   headers=[(b'if-match', revision.encode())])
        self.assertEqual(status, 201)
        self.assertEqual(body['source_rect_px'], list(rect))
        self.assertEqual(body['portal_obj_id'], 2)
        self.assert_consistent()
        status, _ = api_request(app, 'POST', '/api/portals/restore', {'token': token})
        self.assertEqual(status, 409)
        receipt = self.service.delete_portal('source', 2)
        self.create()
        before = self.scenario.snapshot()
        with self.assertRaises(TMXConflictError):
            self.service.restore_portal(receipt['restore_token'], expected_revision=receipt['revision'])
        self.assertEqual(self.scenario.snapshot(), before)

    def test_published_restore_durability_failure_consumes_receipt(self):
        receipt = self.service.delete_portal('source', 1)
        with patch.object(tmx_writer, '_sync_directory', side_effect=OSError('sync failed')):
            with self.assertRaises(tmx_writer.TMXPublishedDurabilityError):
                self.service.restore_portal(receipt['restore_token'], expected_revision=receipt['revision'])
        with self.assertRaises(TMXConflictError):
            self.service.restore_portal(receipt['restore_token'])
        self.assertEqual(len(ET.parse(self.path).getroot().findall('objectgroup/object')), 1)
        self.assert_consistent()

    def test_restore_receipts_are_bounded_and_expiry_is_a_conflict(self):
        self.service.MAX_RESTORE_RECEIPTS = 2
        oldest = self.service.delete_portal('source', 1)
        for _ in range(2):
            edge = self.create()
            self.service.delete_portal('source', edge['portal_obj_id'])
        self.assertEqual(len(self.service._deleted_portals), 2)
        before = self.scenario.snapshot()
        status, body = api_request(self.app(), 'POST', '/api/portals/restore', {'token': oldest['restore_token']})
        self.assertEqual(status, 409)
        self.assertIn('expired', body['detail'])
        self.assertEqual(self.scenario.snapshot(), before)
        self.assert_consistent()

    def test_restore_revalidates_destination_and_rejects_unknown_tokens(self):
        receipt = self.service.delete_portal('source', 1)
        target = self.scenario.maps / 'target.tmx'
        tree = ET.parse(target)
        tree.getroot().find("layer[@name='collision']/data").text = '\n' + ',\n'.join(','.join(['1']*4) for _ in range(4)) + '\n'
        tree.write(target)
        from tools.map_editor.service.portal_validation import PortalValidationError
        with self.assertRaises(PortalValidationError):
            self.service.restore_portal(receipt['restore_token'], expected_revision=receipt['revision'])
        self.assertEqual(ET.parse(self.path).getroot().findall('objectgroup/object'), [])
        status, _ = api_request(self.app(), 'POST', '/api/portals/restore', {'token': 'invented', 'source_rect_px': [1.2]*4})
        self.assertEqual(status, 409)


if __name__ == '__main__':
    unittest.main()
