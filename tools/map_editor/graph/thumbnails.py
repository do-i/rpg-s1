"""On-demand thumbnail rendering for graph nodes.

Each TMX is rendered to a small surface and cached to disk under the
scenario root (so the cache is portable, and ignorable in .gitignore).
The cache key is (tmx mtime, thumbnail size); a stale cache simply
re-renders.
"""

from __future__ import annotations

from pathlib import Path

import pygame

from tools.map_editor.graph.tile_renderer import TileRenderer


THUMB_MAX_W = 256
THUMB_MAX_H = 192
CACHE_DIRNAME = ".cache/map_editor/thumbs"
FULL_CACHE_DIRNAME = ".cache/map_editor/full"


class ThumbnailCache:
    def __init__(self, scenario_root: Path) -> None:
        self._cache_dir = (scenario_root / CACHE_DIRNAME).resolve()
        self._cache_dir.mkdir(parents=True, exist_ok=True)
        self._full_cache_dir = (scenario_root / FULL_CACHE_DIRNAME).resolve()
        self._full_cache_dir.mkdir(parents=True, exist_ok=True)
        self._thumb_memory: dict[Path, pygame.Surface] = {}
        self._full_memory: dict[Path, pygame.Surface] = {}

    def get(self, tmx_path: Path) -> pygame.Surface | None:
        """Return the small thumbnail (bounded by THUMB_MAX_W/H), cached to disk."""
        if tmx_path in self._thumb_memory:
            return self._thumb_memory[tmx_path]

        disk_path = self._disk_path(tmx_path)
        if disk_path.is_file() and disk_path.stat().st_mtime >= tmx_path.stat().st_mtime:
            try:
                surf = pygame.image.load(str(disk_path)).convert_alpha()
                self._thumb_memory[tmx_path] = surf
                return surf
            except pygame.error:
                pass

        full = self.get_full(tmx_path)
        if full is None:
            return None
        scale = min(THUMB_MAX_W / full.get_width(), THUMB_MAX_H / full.get_height(), 1.0)
        if scale < 1.0:
            new_size = (
                max(1, int(full.get_width() * scale)),
                max(1, int(full.get_height() * scale)),
            )
            thumb = pygame.transform.smoothscale(full, new_size)
        else:
            thumb = full.copy()
        pygame.image.save(thumb, str(disk_path))
        self._thumb_memory[tmx_path] = thumb
        return thumb

    def get_full(self, tmx_path: Path) -> pygame.Surface | None:
        """Return the map rendered at its native pixel size (cached in memory only)."""
        if tmx_path in self._full_memory:
            return self._full_memory[tmx_path]
        try:
            tile_map = TileRenderer(tmx_path)
        except Exception:
            return None
        full = pygame.Surface(
            (tile_map.width_px, tile_map.height_px), pygame.SRCALPHA
        )
        tile_map.render(full)
        self._full_memory[tmx_path] = full
        return full

    def thumbnail_file(self, tmx_path: Path) -> Path | None:
        """Ensure the small thumbnail PNG exists on disk and return its path.

        Used by the web backend, which serves files rather than surfaces.
        Returns None when the map cannot be rendered.
        """
        if self.get(tmx_path) is None:
            return None
        return self._disk_path(tmx_path)

    def full_file(self, tmx_path: Path) -> Path | None:
        """Ensure a native-resolution PNG render exists on disk; return its path.

        Full renders back the web UI's tile pickers, where portal tiles are
        chosen by clicking on the actual map pixels. Returns None when the map
        cannot be rendered.
        """
        disk_path = self._full_cache_dir / f"{tmx_path.stem}.png"
        if disk_path.is_file() and disk_path.stat().st_mtime >= tmx_path.stat().st_mtime:
            return disk_path
        full = self.get_full(tmx_path)
        if full is None:
            return None
        pygame.image.save(full, str(disk_path))
        return disk_path

    def _disk_path(self, tmx_path: Path) -> Path:
        return self._cache_dir / f"{tmx_path.stem}.png"
