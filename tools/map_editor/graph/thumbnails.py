"""Render previews keyed by TMX and referenced TSX/image contents.

Memory and disk share one revision for both sizes. Every lookup rechecks the
sources, including removed files, and a render changed in flight is discarded.
"""

from __future__ import annotations

from hashlib import sha256
from pathlib import Path
from tempfile import NamedTemporaryFile
from threading import RLock
from xml.etree import ElementTree as ET

import pygame

from tools.map_editor.graph.tile_renderer import TileRenderer


THUMB_MAX_W = 256
THUMB_MAX_H = 192
CACHE_DIRNAME = ".cache/map_editor/thumbs"
FULL_CACHE_DIRNAME = ".cache/map_editor/full"
RENDER_VERSION = b"editor-flat-render-v2:256x192"


def render_revision(tmx_path: Path) -> str:
    """Hash source identities and bytes; timestamps alone miss replaced files."""
    sources: dict[Path, bytes] = {}

    def visit(path: Path, *, xml: bool):
        path = path.resolve()
        if path in sources:
            return
        data = path.read_bytes()
        sources[path] = data
        if xml:
            root = ET.fromstring(data)
            for tileset in root.iter("tileset"):
                if source := tileset.get("source"):
                    visit(path.parent / source, xml=True)
            for image in root.iter("image"):
                if source := image.get("source"):
                    visit(path.parent / source, xml=False)

    visit(tmx_path, xml=True)
    digest = sha256(RENDER_VERSION)
    for path, data in sorted(sources.items()):
        digest.update(str(path).encode() + b"\0" + sha256(data).digest())
    return digest.hexdigest()


class ThumbnailCache:
    def __init__(self, scenario_root: Path) -> None:
        self._lock = RLock()
        self._cache_dir = (scenario_root / CACHE_DIRNAME).resolve()
        self._full_cache_dir = (scenario_root / FULL_CACHE_DIRNAME).resolve()
        for directory in (self._cache_dir, self._full_cache_dir):
            directory.mkdir(parents=True, exist_ok=True)
        self._revisions: dict[Path, str] = {}
        self._thumb_memory: dict[Path, pygame.Surface] = {}
        self._full_memory: dict[Path, pygame.Surface] = {}

    def invalidate(self, tmx_path: Path) -> None:
        """Forget both sizes, including disk entries from other instances."""
        with self._lock:
            path = tmx_path.resolve()
            self._revisions.pop(path, None)
            self._thumb_memory.pop(path, None)
            self._full_memory.pop(path, None)
            self._prune(path)

    def revision(self, tmx_path: Path) -> str | None:
        """Return the current dependency revision, invalidating obsolete output."""
        with self._lock:
            path = tmx_path.resolve()
            try:
                revision = render_revision(path)
            except (OSError, ET.ParseError):
                self.invalidate(path)
                return None
            if self._revisions.get(path) != revision:
                self._thumb_memory.pop(path, None)
                self._full_memory.pop(path, None)
                self._prune(path, keep=revision)
                self._revisions[path] = revision
            return revision

    def get(self, tmx_path: Path) -> pygame.Surface | None:
        return self._surface(tmx_path, full=False)

    def get_full(self, tmx_path: Path) -> pygame.Surface | None:
        return self._surface(tmx_path, full=True)

    def thumbnail_file(self, tmx_path: Path) -> Path | None:
        return self._file(tmx_path, full=False)

    def full_file(self, tmx_path: Path) -> Path | None:
        return self._file(tmx_path, full=True)

    def png_bytes(self, tmx_path: Path, *, full: bool = False) -> tuple[str, bytes] | None:
        """Read a validated response while holding the cache lock.

        The web server sends these bytes instead of streaming a mutable path
        after releasing the editor publication lock.
        """
        with self._lock:
            path = tmx_path.resolve()
            disk = self._file(path, full=full)
            if disk is None:
                return None
            revision = self._revisions[path]
            try:
                data = disk.read_bytes()
                if render_revision(path) != revision:
                    self.invalidate(path)
                    return None
            except (OSError, ET.ParseError):
                self.invalidate(path)
                return None
            return revision, data

    def _surface(self, tmx_path: Path, *, full: bool) -> pygame.Surface | None:
        with self._lock:
            path = tmx_path.resolve()
            revision = self.revision(path)
            if revision is None:
                return None
            memory = self._full_memory if full else self._thumb_memory
            try:
                surface = memory.get(path)
                if surface is None:
                    disk = self._disk_path(path, revision, full=full)
                    if disk.is_file():
                        try:
                            surface = pygame.image.load(str(disk)).convert_alpha()
                        except (pygame.error, OSError):
                            self.invalidate(path)
                            self._revisions[path] = revision
                    if surface is None:
                        if full:
                            tile_map = TileRenderer(path)
                            surface = pygame.Surface(
                                (tile_map.width_px, tile_map.height_px), pygame.SRCALPHA
                            )
                            tile_map.render(surface)
                        else:
                            native = self._surface(path, full=True)
                            if native is None:
                                return None
                            scale = min(THUMB_MAX_W / native.get_width(),
                                        THUMB_MAX_H / native.get_height(), 1.0)
                            size = (max(1, int(native.get_width() * scale)),
                                    max(1, int(native.get_height() * scale)))
                            surface = pygame.transform.smoothscale(native, size) if scale < 1 else native.copy()
                if render_revision(path) != revision:
                    self.invalidate(path)
                    return None
                memory[path] = surface
                return surface
            except Exception:
                # Failed construction, drawing or dependency reads must never
                # revive a previous successful generation in either size.
                self.invalidate(path)
                return None

    def _file(self, tmx_path: Path, *, full: bool) -> Path | None:
        with self._lock:
            path = tmx_path.resolve()
            surface = self._surface(path, full=full)
            if surface is None:
                return None
            revision = self._revisions[path]
            disk = self._disk_path(path, revision, full=full)
            temporary = None
            try:
                if not disk.is_file():
                    with NamedTemporaryFile(dir=disk.parent, suffix=".png", delete=False) as handle:
                        temporary = Path(handle.name)
                    pygame.image.save(surface, str(temporary))
                    if render_revision(path) != revision:
                        self.invalidate(path)
                        return None
                    temporary.replace(disk)
                return disk
            except Exception:
                self.invalidate(path)
                return None
            finally:
                if temporary is not None:
                    temporary.unlink(missing_ok=True)

    def _key(self, path: Path) -> str:
        return sha256(str(path).encode()).hexdigest()

    def _disk_path(self, path: Path, revision: str, *, full: bool) -> Path:
        directory = self._full_cache_dir if full else self._cache_dir
        return directory / f"{self._key(path)}-{revision}.png"

    def _prune(self, path: Path, *, keep: str | None = None) -> None:
        for directory in (self._cache_dir, self._full_cache_dir):
            for disk in directory.glob(f"{self._key(path)}-*.png"):
                if keep is None or disk.name != f"{self._key(path)}-{keep}.png":
                    disk.unlink(missing_ok=True)
            # Old mtime-based entries have no dependency revision and cannot
            # be promoted into the new cache, even on the first lookup.
            (directory / f"{path.stem}.png").unlink(missing_ok=True)
