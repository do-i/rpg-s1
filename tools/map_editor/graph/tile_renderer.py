"""Flat TMX rendering for editor thumbnails.

This replaces the `engine.world.tile_map.TileMap` the editor used while it lived in the
`agentic-rpg` checkout. That class loaded a map's collision grid, portal list, enemy spawn
tiles and boss marker on construction, and pulled three more engine modules in behind it --
none of which the thumbnail path ever read. It only wanted the pixel size and a composited
image.

Rendering visible tile layers and tile objects is the whole job. The editor now has no
dependency on any game engine, Python or Rust: it reads TMX and scenario YAML, and nothing else.
"""

from __future__ import annotations

from pathlib import Path
from xml.etree import ElementTree as ET

import pygame
import pytmx


class TileRenderer:
    """A TMX file's visible tiles and tile objects, composited once."""

    def __init__(self, tmx_path: str | Path) -> None:
        self._tmx = pytmx.load_pygame(str(tmx_path), pixelalpha=True)
        self.tile_width: int = self._tmx.tilewidth
        self.tile_height: int = self._tmx.tileheight
        self.width: int = self._tmx.width
        self.height: int = self._tmx.height
        # PyTMX appends object groups after tile layers; retain the TMX order explicitly.
        self._layer_order = {
            child.get("id"): index
            for index, child in enumerate(ET.parse(tmx_path).getroot())
            if child.tag in ("layer", "objectgroup")
        }

    @property
    def width_px(self) -> int:
        return self.width * self.tile_width

    @property
    def height_px(self) -> int:
        return self.height * self.tile_height

    def render(self, surface: pygame.Surface) -> None:
        """Composite layers in authored order, including chest tile objects."""
        for layer in sorted(self._tmx.visible_layers,
                            key=lambda layer: self._layer_order.get(str(layer.id), 0)):
            if isinstance(layer, pytmx.TiledTileLayer):
                for x, y, image in layer.tiles():
                    surface.blit(image, (x * self.tile_width, y * self.tile_height))
            elif isinstance(layer, pytmx.TiledObjectGroup):
                objects = layer
                if getattr(layer, "draworder", "topdown") == "topdown":
                    objects = sorted(layer, key=lambda obj: obj.y + obj.height)
                for obj in objects:
                    if obj.visible and obj.image is not None:
                        # PyTMX has already converted tile anchors to top-left.
                        surface.blit(obj.image, (obj.x, obj.y))
