# Ancient ruins courtyard objects

Generated on 2026-10-22 with the built-in OpenAI image generation tool at the
owner's direction. The three `generated-*.png` files are unmodified sources.
[prompts.json](prompts.json) records the complete prompt set and source hashes.

The camera looks north from due south, pitched down 45 degrees with zero yaw.
The arcade and market stall show their front and top without exterior side
walls. Chipped olive-gray limestone, muted moss, rotten brown timber and faded
ochre canvas match the accepted gate map. The ruined fountain has a headless,
partly broken guardian statue, stagnant water and a damaged basin rim.

The separate production sheet is
`assets/scenarios/rusted_kingdoms/media/tilesets/ancient_ruins_courtyard.png`,
with minimal image-grid metadata in `ancient_ruins_courtyard.tsx`. It is
**768×480 pixels**, with 24 columns and 360 slots of 32×32 pixels. Complete
objects remain together, with transparent gutters:

| Object | Atlas column, row | Width × height in tiles |
| --- | --- | --- |
| Broken three-bay arcade | 0, 0 | 14×7 |
| Guardian fountain | 16, 0 | 8×8 |
| Collapsed market stall | 0, 10 | 6×5 |

Rebuild from the repository root:

```sh
python3 art/ancient-ruins-courtyard/build_tileset.py
```

The export uses ImageMagick to threshold alpha at 50%, trim clear margins,
resize with nearest-neighbor sampling while preserving aspect ratio, align
each object to its bottom center, and reduce each object's palette to 128
colors without dithering. Atlas packing preserves every normalized object
pixel. Empty slots and gutters are transparent; final alpha is exactly 0/255.

[layout.json](layout.json) records the map dimensions, all 79 object placements,
flips, exact collision bases, encounter markers and chest identities. Original
gate atlases are reused without editing their pixels or metadata. Tall upper
rows use the existing `foreground` layer with per-placement depth overrides;
base rows use `decoration`. Arcades block their pillar feet while their arch
openings remain walkable. Walls, house foundations, stall counters, fountain
basin, prop bases and water carry collision. Tree branches and open grass do
not. No map layers were added.

The playable layout and validation notes are documented in
[the courtyard guide](../../docs/ancient-ruins-courtyard.md).
