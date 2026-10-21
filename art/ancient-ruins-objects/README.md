# Ancient ruins object tiles

Created on 2026-10-21 with the built-in OpenAI image generation tool at the
owner's direction. The `generated-*.png` files are unmodified generated
sources. Full prompts are recorded in [prompts.json](prompts.json).

The production sheet is
`assets/scenarios/rusted_kingdoms/media/tilesets/ancient_ruins.png`, with matching
Tiled metadata in `ancient_ruins.tsx`. It contains a heavy damaged defensive
gatehouse, two broken residential-house variants, and a mossy curtain wall.
The objects share weathered olive-gray masonry, deep dark joints, ivy, and
chipped stone; houses have missing clay roof tiles and exposed rafters.

The atlas is **640×896**, with 20 columns and 560 slots of exactly 32×32 pixels.
Whole objects are grouped in bands: gatehouse, defensive wall, then residences.
Empty slots are transparent. Object rectangles use zero-based atlas coordinates:

| Object | Column, row | Width × height in tiles | Export size |
| --- | --- | --- | --- |
| Gatehouse | 0, 0 | 18×14 | 576×448 |
| Curtain wall | 0, 14 | 12×5 | 384×160 |
| Half-roof house | 0, 20 | 9×8 | 288×256 |
| Collapsed-roof house | 10, 22 | 10×6 | 320×192 |

The residential grouping follows complete building footprints reconstructed
from the map's `foreground` and `decoration` tiles. Both houses end on atlas
row 27. Column 9 separates them, and row 19 separates residences from the
defensive wall. These gutters are entirely transparent. Every object retains
its exact pixels and internal tile arrangement.

[grouping.json](grouping.json) records each object's atlas rectangle and every
map placement, including horizontal flips. The half-roof block supplies the
northwest `[4,4]`, northeast `[54,4]`, and southeast `[50,28]` houses. The
collapsed-roof block supplies the west `[8,20]`, southwest `[13,37]`, and south
`[38,34]` houses. All coordinates are zero-based tile origins. Map appearances,
layer order and collision are preserved
through explicit GID remapping.

The half-roof house uses `generated-house-a-front-v2.png`: a due-south camera
pitched down 45 degrees, showing its roof and front facade without either
exterior side wall. This corrects the northwest, northeast, and southeast
instances together. The original `generated-house-a.png` is retained as the
superseded source. Gatehouse, curtain-wall, and collapsed-roof-house pixels
are preserved exactly when repacking; all map GIDs are remapped explicitly.

The four objects were exported with ImageMagick: threshold source alpha at
50%, trim transparent margins, resize with nearest-neighbor sampling while
preserving aspect ratio, align to the bottom center of each export rectangle,
and reduce each object's palette to 128 colors without dithering. Alpha is
thresholded again after palette reduction. Whole objects are composited into
the atlas without gaps between their constituent tiles. Every final alpha
value is either 0 or 255; no matte/background is baked into the sheet.

For example, from the repository root:

```sh
mkdir -p temp/ancient-ruins
magick art/ancient-ruins-objects/generated-gate.png \
  -channel A -threshold 50% +channel -trim +repage \
  -filter point -resize 576x448 -gravity South -background none -extent 576x448 \
  +dither -colors 128 -channel A -threshold 50% +channel \
  PNG32:temp/ancient-ruins/gate.png
```

The owner-regenerated TSX contains image-grid metadata only, without per-tile
descriptions or `sort_base_y` properties. Upper object rows belong in
`foreground`; the last row belongs in `decoration`. Collision
is authored independently, with a clear two-tile gate passage. No map layer
was added. Lower-zone maps supplied collision/layer conventions; higher-zone
placeholder maps were not used as references.

Source SHA-256 values:

| Source | SHA-256 |
| --- | --- |
| `generated-gate.png` | `c8604692ad45c894eb22117210c35c94fcbd35b0349639bb66902f089e180813` |
| `generated-house-a.png` | `d7ebb124f2845d656f1f6a89feb137779e91df1ff5628296a368273af959c8e0` |
| `generated-house-a-front-v2.png` | `b27dc31e38e98f4334332bc0230fe6b85cc529f2f02595dddaf7b2be0d88f740` |
| `generated-house-b.png` | `e42780718579b158f08f52475a95d885cef8ce255bd664f82c2a587417e16450` |
| `generated-wall.png` | `f137e19e116473e80639d12264a59621cc89b1715a0b36eeb297bee5d0037d1e` |
