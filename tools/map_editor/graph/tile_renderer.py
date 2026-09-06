"""Flat TMX rendering for editor thumbnails.

This replaces the `engine.world.tile_map.TileMap` the editor used while it lived in the
`agentic-rpg` checkout. That class loaded a map's collision grid, portal list, enemy spawn
tiles and boss marker on construction, and pulled three more engine modules in behind it --
none of which the thumbnail path ever read. It only wanted the pixel size and a composited
image.

Rendering the tile layers is the whole job, so that is all this does. The editor now has no
dependency on any game engine, Python or Rust: it reads TMX and scenario YAML, and nothing else.
"""

from __future__ import annotations

from pathlib import Path

import pygame
import pytmx


class TileRenderer:
    """A TMX file's visible tile layers, composited once."""

    def __init__(self, tmx_path: str | Path) -> None:
        self._tmx = pytmx.load_pygame(str(tmx_path), pixelalpha=True)
        self.tile_width: int = self._tmx.tilewidth
        self.tile_height: int = self._tmx.tileheight
        self.width: int = self._tmx.width
        self.height: int = self._tmx.height

    @property
    def width_px(self) -> int:
        return self.width * self.tile_width

    @property
    def height_px(self) -> int:
        return self.height * self.tile_height

    def render(self, surface: pygame.Surface) -> None:
        """Blit every visible tile layer onto `surface`, bottom layer first."""
        for layer in self._tmx.visible_layers:
            if not isinstance(layer, pytmx.TiledTileLayer):
                continue
            for x, y, image in layer.tiles():
                surface.blit(image, (x * self.tile_width, y * self.tile_height))
