# Alpine foothills object atlas

Generated on 2026-10-22 with the built-in OpenAI image generation tool at the
owner's direction. The six `generated-*.png` files are unmodified generated
sources. [prompts.json](prompts.json) retains every full prompt, the intended
object rectangles and source SHA-256 hashes.

All objects use an orthographic camera from due south looking north, pitched
down 45 degrees with zero yaw. Broad front edges stay horizontal. Slate-gray
granite, dark green pine, muted brown timber, pale snow and small bronze/red
accents form one medieval alpine palette. The atlas includes no characters,
ground plane, text or magic effects.

Production files are
`assets/scenarios/rusted_kingdoms/media/tilesets/mountain_foothills.png` and
`mountain_foothills.tsx`. The sheet is **864×544**, 27 columns by 17 rows of
exactly 32×32 pixels, with 459 slots. Metadata is the minimal Tiled image-grid
format, without per-tile descriptions. Complete objects remain together:

| Object | Column, row | Tile width × height |
| --- | --- | --- |
| Granite ridge | 0, 0 | 16×8 |
| Alpine pine | 18, 0 | 4×7 |
| Trail bell cairn | 23, 1 | 3×5 |
| Boulder cluster | 23, 8 | 4×3 |
| Stone trail refuge | 0, 10 | 10×7 |
| Tinker's workshop | 12, 10 | 9×7 |

Rebuild from the repository root:

```sh
python3 art/mountain-foothills/build_tileset.py
```

The export thresholds alpha at 50%, trims clear margins, resizes with
nearest-neighbor sampling while preserving aspect ratio, aligns each object
to its bottom center and reduces each object's palette to 128 colors without
dithering. Alpha is thresholded again. The builder verifies 0/255 alpha,
fully transparent unused slots/gutters and exact normalized object pixels in
the packed atlas. It never edits any existing atlas.

[build_maps.py](build_maps.py) contains the three complete authored layout
configurations. It reuses existing TMX metadata and layer IDs, resets the
seven established tile layers, preserves original chest identities and
rebuilds the Zone 05 artwork, collision, portals and patrols. It also emits
[layouts.json](layouts.json), recording every placement, horizontal flip,
blocked object cell, chest, arrival, NPC anchor, bridge and patrol start.
Existing terrain Wang corners join snow, stone trails and water edges.
The two neighboring maps' incoming portal coordinates are maintained
separately; this script edits only the three Zone 05 maps.

To deliberately regenerate those maps from the retained design:

```sh
python3 art/mountain-foothills/build_maps.py
```

Regeneration overwrites manual art and placement edits in the three maps.
It checks reachable arrivals, patrols, NPC feet and cache approaches, plus
clear bridge decks. Upper object rows use `foreground` with per-cell depths;
bottom rows use `decoration`. Ridges block only their bottom two tile rows;
their upper silhouettes stay walkable underneath and sort in front of actors
behind them. Tree roots, boulder bases and other lower solid footprints have
collision; open ground, snow and tree branches do not.
The gameplay design, rewards and quest are documented in
[the Zone 05 guide](../../docs/mountain-foothills.md).
