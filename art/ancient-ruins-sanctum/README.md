# Ancient ruins sanctum objects

Generated on 2026-10-22 with the built-in OpenAI image generation tool at the
owner's direction. The two `generated-*.png` files are unmodified sources.
[prompts.json](prompts.json) records the full prompts and source SHA-256 hashes.

The camera looks north from due south, pitched down 45 degrees with zero yaw.
Front edges remain horizontal, with front and top surfaces visible. Chipped
olive-gray limestone, deep joints, muted warm highlights and sparse yellow-green
moss match the accepted ancient-ruins art. The altar's empty carved rear screen
is shattered; the sarcophagus has a cracked lid and worn sword relief.

The production sheet is
`assets/scenarios/rusted_kingdoms/media/tilesets/ancient_ruins_sanctum.png`,
with minimal image-grid metadata in `ancient_ruins_sanctum.tsx`. It is
**512×256 pixels**, with 16 columns and 128 slots of exactly 32×32 pixels.
Complete object groups remain together, separated by transparent gutters:

| Object | Atlas column, row | Width × height in tiles |
| --- | --- | --- |
| Shattered altar and rear screen | 0, 0 | 10×8 |
| Cracked sarcophagus | 12, 5 | 4×3 |

Rebuild from the repository root:

```sh
python3 art/ancient-ruins-sanctum/build_tileset.py
```

The export thresholds alpha at 50%, trims clear margins, resizes with
nearest-neighbor sampling while preserving aspect ratio, aligns each object
to its bottom center and reduces its palette to 128 colors without dithering.
It checks complete object pixels against the packed atlas, clear gutters and
binary alpha. The export does not change any accepted atlas.

[layout.json](layout.json) records the map dimensions, all 39 object placements,
horizontal flips, exact collision bases, patrol starts, caches, NPC anchor and
boss position. Upper rows use the existing `foreground` layer with explicit
base depths; bottom rows use `decoration`. Arcade collision follows pillar
feet, leaving arch openings walkable. Tombs block their lower two rows; the
altar blocks its lower stone footprint. Tree branches and open ground stay
walkable. Water and the visible enclosure also carry collision.

The playable layout and connections are documented in
[the sanctum guide](../../docs/ancient-ruins-sanctum.md).
