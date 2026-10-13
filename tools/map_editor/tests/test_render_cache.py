"""Actual PNG/TMX previews through long-lived caches and editor publication."""

from concurrent.futures import ThreadPoolExecutor
from io import BytesIO
import os
from pathlib import Path
from threading import Event
import unittest
from unittest.mock import patch
from xml.etree import ElementTree as ET

import pygame

from tools.map_editor.edit import tmx_writer
from tools.map_editor.graph import thumbnails
from tools.map_editor.graph.thumbnails import ThumbnailCache
from tools.map_editor.graph.tile_renderer import TileRenderer
from tools.map_editor.tests.support import DisposableScenario, api_request, map_xml


RED, GREEN, BLUE = (220, 20, 20, 255), (20, 220, 20, 255), (20, 20, 220, 255)


class RenderCacheTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        os.environ['SDL_VIDEODRIVER'] = 'dummy'
        os.environ['SDL_AUDIODRIVER'] = 'dummy'
        pygame.init()
        pygame.display.set_mode((1, 1))

    def setUp(self):
        self.scenario = DisposableScenario()
        self.addCleanup(self.scenario.close)
        self.path = self.scenario.source
        self.tsx = self.scenario.maps / 'tiles.tsx'
        self.image = self.scenario.maps / 'tiles.png'
        self.write_image(self.image)
        self.write_tileset()
        self.write_map()
        self.cache = ThumbnailCache(self.scenario.root)
        self.scenario.service.set_render_invalidator(self.cache.invalidate)

    def write_image(self, path, first=RED, second=GREEN):
        image = pygame.Surface((64, 32), pygame.SRCALPHA)
        image.fill(first, (0, 0, 32, 32))
        image.fill(second, (32, 0, 32, 32))
        pygame.image.save(image, str(path))

    def write_tileset(self, image='tiles.png'):
        self.tsx.write_text(
            '<tileset version="1.10" name="test" tilewidth="32" tileheight="32" '
            'tilecount="2" columns="2"><image source="' + image + '" width="64" height="32"/></tileset>',
            encoding='utf-8',
        )

    def write_map(self, gid=1, width=4, height=4):
        root = ET.fromstring(map_xml(width=width, height=height))
        root.insert(0, ET.Element('tileset', firstgid='1', source='tiles.tsx'))
        root.find("layer[@name='ground']/data").text = ','.join([str(gid)] * (width * height))
        root.find("layer[@name='collision']").set('visible', '0')
        self.path.write_bytes(ET.tostring(root))

    def both_files(self, cache=None):
        cache = cache or self.cache
        files = (cache.thumbnail_file(self.path), cache.full_file(self.path))
        self.assertTrue(all(path is not None and path.is_file() for path in files))
        return files

    def assert_pixels(self, color, cache=None):
        cache = cache or self.cache
        for surface in (cache.get(self.path), cache.get_full(self.path)):
            self.assertIsNotNone(surface)
            self.assertEqual(tuple(surface.get_at((0, 0))), color)
        for file in self.both_files(cache):
            self.assertEqual(tuple(pygame.image.load(str(file)).get_at((0, 0))), color)

    def assert_empty(self, old_files):
        self.assertFalse(any(path.exists() for path in old_files))
        self.assertNotIn(self.path, self.cache._thumb_memory)
        self.assertNotIn(self.path, self.cache._full_memory)
        for directory in (self.cache._cache_dir, self.cache._full_cache_dir):
            self.assertFalse(list(directory.glob('*.png')))

    def test_long_lived_cache_tracks_tmx_tsx_and_same_timestamp_image(self):
        self.assert_pixels(RED)
        old = self.both_files()
        self.write_map(gid=2)
        self.assert_pixels(GREEN)
        self.assertFalse(any(file.exists() for file in old))
        alternate = self.scenario.maps / 'alternate.png'
        self.write_image(alternate, second=BLUE)
        old = self.both_files()
        self.write_tileset('alternate.png')
        self.assert_pixels(BLUE)
        self.assertFalse(any(file.exists() for file in old))
        old = self.both_files()
        stat = alternate.stat()
        self.write_image(alternate, second=RED)
        os.utime(alternate, ns=(stat.st_atime_ns, stat.st_mtime_ns))
        self.assert_pixels(RED)
        self.assertFalse(any(file.exists() for file in old))

    def test_memory_is_not_republished_as_fresh_after_source_change(self):
        old = self.both_files()
        self.write_map(gid=2)
        # Reproduce the old full_file path: disk missing, old full surface held.
        old[1].unlink()
        self.assert_pixels(GREEN)
        self.assertFalse(old[0].exists())

    def test_same_timestamp_tmx_and_tsx_changes_refresh_pixels(self):
        self.assert_pixels(RED)
        stat = self.path.stat()
        self.write_map(gid=2)
        os.utime(self.path, ns=(stat.st_atime_ns, stat.st_mtime_ns))
        self.assert_pixels(GREEN)
        alternate = self.scenario.maps / 'other.png'
        self.write_image(alternate, second=BLUE)
        stat = self.tsx.stat()
        self.write_tileset('other.png')
        os.utime(self.tsx, ns=(stat.st_atime_ns, stat.st_mtime_ns))
        self.assert_pixels(BLUE)

    def test_cross_instance_reuses_current_disk_without_rendering(self):
        self.assert_pixels(RED)
        fresh = ThumbnailCache(self.scenario.root)
        with patch.object(thumbnails, 'TileRenderer', side_effect=AssertionError('disk was not reused')):
            self.assert_pixels(RED, fresh)
        self.write_image(self.image, first=BLUE)
        self.assert_pixels(BLUE, fresh)
        # The first instance still holds red pixels; it must independently refresh.
        self.assert_pixels(BLUE)

    def test_removed_tmx_tsx_or_image_invalidates_both_sizes_and_recovers(self):
        for missing in (self.path, self.tsx, self.image):
            with self.subTest(missing=missing.name):
                self.assert_pixels(RED)
                files, data = self.both_files(), missing.read_bytes()
                missing.unlink()
                self.assertIsNone(self.cache.get(self.path))
                self.assertIsNone(self.cache.full_file(self.path))
                self.assertIsNone(self.cache.png_bytes(self.path))
                self.assert_empty(files)
                missing.write_bytes(data)
                self.assert_pixels(RED)

    def test_invalid_tmx_tsx_and_image_never_return_previous_pixels(self):
        for broken in (self.path, self.tsx, self.image):
            with self.subTest(broken=broken.name):
                files, data = self.both_files(), broken.read_bytes()
                broken.write_bytes(b'invalid content')
                self.assertIsNone(self.cache.thumbnail_file(self.path))
                self.assertIsNone(self.cache.get_full(self.path))
                self.assert_empty(files)
                broken.write_bytes(data)
                self.assert_pixels(RED)

    def test_drawing_failure_invalidates_all_output_and_next_request_recovers(self):
        failures = (
            patch.object(thumbnails, 'TileRenderer', side_effect=pygame.error('injected load failure')),
            patch.object(TileRenderer, 'render', side_effect=pygame.error('injected draw failure')),
        )
        for failure in failures:
            files = self.both_files()
            self.write_map(gid=2)
            with failure:
                self.assertIsNone(self.cache.get(self.path))
                self.assertIsNone(self.cache.get_full(self.path))
                self.assert_empty(files)
            self.assert_pixels(GREEN)
            self.write_map(gid=1)

    def test_png_save_failure_discards_both_sizes_and_temporary_file(self):
        files = self.both_files()
        self.write_map(gid=2)
        with patch.object(pygame.image, 'save', side_effect=pygame.error('injected PNG failure')):
            self.assertIsNone(self.cache.full_file(self.path))
        self.assert_empty(files)
        self.assertFalse(list(self.cache._full_cache_dir.iterdir()))
        self.assert_pixels(GREEN)

    def test_change_during_render_discards_candidate_without_disk_publication(self):
        files = self.both_files()
        self.write_map(gid=2)
        render = TileRenderer.render
        def changed(renderer, surface):
            render(renderer, surface)
            self.write_image(self.image, second=BLUE)
        with patch.object(TileRenderer, 'render', changed):
            self.assertIsNone(self.cache.full_file(self.path))
        self.assert_empty(files)
        self.assert_pixels(BLUE)

    def test_change_during_disk_save_does_not_publish_old_memory(self):
        self.cache.get_full(self.path)
        save = pygame.image.save
        def changed(surface, file):
            save(surface, file)
            self.write_map(gid=2)
        with patch.object(pygame.image, 'save', changed):
            self.assertIsNone(self.cache.full_file(self.path))
        self.assertFalse(list(self.cache._full_cache_dir.iterdir()))
        self.assert_pixels(GREEN)

    def test_editor_mutations_and_published_failure_invalidate_immediately(self):
        service = self.scenario.service
        operations = (
            lambda: service.create_portal('source', (64, 32, 32, 32), 'target', (1, 1)),
            lambda: service.retarget_portal('source', 1, 'target', (2, 1), None),
            lambda: service.delete_portal('source', 1),
        )
        for operation in operations:
            files = self.both_files()
            operation()
            self.assert_empty(files)
            self.assert_pixels(RED)
        receipt = service.delete_portal('source', 2)
        files = self.both_files()
        service.restore_portal(receipt['restore_token'])
        self.assert_empty(files)
        files = self.both_files()
        with patch.object(tmx_writer, '_sync_directory', side_effect=OSError('sync failed')):
            with self.assertRaises(tmx_writer.TMXPublishedDurabilityError):
                service.create_portal('source', (64, 32, 32, 32), 'target', (1, 1))
        self.assert_empty(files)

    def test_conflict_and_external_graph_refresh_invalidate_immediately(self):
        service = self.scenario.service
        revision = service.graph_dict(refresh=True)['nodes'][0]['revision']
        files = self.both_files()
        self.write_map(gid=2)
        with self.assertRaises(tmx_writer.TMXConflictError):
            service.delete_portal('source', 1, expected_revision=revision)
        self.assert_empty(files)
        self.assert_pixels(GREEN)
        files = self.both_files()
        self.write_map(gid=1)
        service.graph_dict(refresh=True)
        self.assert_empty(files)

    def test_full_and_thumbnail_sizes_and_map_path_identity(self):
        self.write_map(width=12, height=8)
        self.assertEqual(self.cache.get_full(self.path).get_size(), (384, 256))
        self.assertEqual(self.cache.get(self.path).get_size(), (256, 170))
        other = self.scenario.maps / 'nested' / self.path.name
        other.parent.mkdir()
        other.write_bytes(self.path.read_bytes().replace(b'tiles.tsx', b'../tiles.tsx'))
        self.assertNotEqual(self.cache.full_file(self.path), self.cache.full_file(other))

    def test_old_mtime_cache_and_corrupt_current_png_are_not_trusted(self):
        for directory in (self.cache._cache_dir, self.cache._full_cache_dir):
            legacy = directory / 'source.png'
            self.write_image(legacy, first=BLUE)
        self.assert_pixels(RED)
        files = self.both_files()
        files[0].write_bytes(b'not a PNG')
        fresh = ThumbnailCache(self.scenario.root)
        self.assert_pixels(RED, fresh)

    def app(self):
        from tools.map_editor.web.server import create_app
        return create_app(self.scenario.root)

    def test_api_dependency_revisions_pixels_failure_and_no_store(self):
        app = self.app()
        status, graph = api_request(app, 'GET', '/api/graph')
        self.assertEqual(status, 200)
        node = graph['nodes'][0]
        previous = node['render_revision']
        for filename in ('thumbnail', 'full'):
            status, data, headers = api_request(app, 'GET', f'/api/maps/source/{filename}.png', raw=True, include_headers=True)
            self.assertEqual(status, 200)
            self.assertEqual(tuple(pygame.image.load(BytesIO(data)).get_at((0, 0))), RED)
            self.assertEqual(headers[b'x-render-revision'].decode(), previous)
            self.assertEqual(headers[b'cache-control'], b'no-store')
        self.write_image(self.image, first=BLUE)
        _, graph = api_request(app, 'GET', '/api/graph')
        current = graph['nodes'][0]
        self.assertEqual(current['revision'], node['revision'])
        self.assertNotEqual(current['render_revision'], previous)
        for filename in ('thumbnail', 'full'):
            # Even an old URL is a current, non-cacheable response; it must
            # never acquire an immutable cache entry for the obsolete key.
            status, data, headers = api_request(app, 'GET', f'/api/maps/source/{filename}.png?revision={previous}',
                                              raw=True, include_headers=True)
            self.assertEqual(status, 200)
            self.assertEqual(tuple(pygame.image.load(BytesIO(data)).get_at((0, 0))), BLUE)
            self.assertEqual(headers[b'cache-control'], b'no-store')
            self.assertEqual(headers[b'x-render-revision'].decode(), current['render_revision'])
        self.image.unlink()
        for filename in ('thumbnail', 'full'):
            status, body = api_request(app, 'GET', f'/api/maps/source/{filename}.png')
            self.assertEqual(status, 422)
            self.assertIn('failed to render', body['detail'])
        _, graph = api_request(app, 'GET', '/api/graph')
        self.assertIsNone(graph['nodes'][0]['render_revision'])

    def test_preview_waits_for_publication_and_returns_coherent_snapshot(self):
        service = self.scenario.service
        entered, release, attempted = Event(), Event(), Event()
        publish = tmx_writer._publish_tmx
        def held(*args):
            entered.set()
            if not release.wait(5):
                raise TimeoutError('publication was not released')
            return publish(*args)
        def preview():
            attempted.set()
            return service.preview('source', self.cache.png_bytes)
        with patch.object(tmx_writer, '_publish_tmx', held), ThreadPoolExecutor(2) as pool:
            edit = pool.submit(service.create_portal, 'source', (64, 32, 32, 32), 'target', (1, 1))
            try:
                self.assertTrue(entered.wait(5))
                read = pool.submit(preview)
                self.assertTrue(attempted.wait(5))
                self.assertFalse(read.done())
            finally:
                release.set()
            edit.result(timeout=5)
            revision, data = read.result(timeout=5)
        self.assertEqual(revision, self.cache.revision(self.path))
        # Subsequent invalidation cannot remove an already captured response.
        self.cache.invalidate(self.path)
        self.assertEqual(tuple(pygame.image.load(BytesIO(data)).get_at((0, 0))), RED)


if __name__ == '__main__':
    unittest.main()
