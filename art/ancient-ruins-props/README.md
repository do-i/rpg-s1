# Ancient ruins aftermath props

Generated on 2026-10-21 using the built-in OpenAI image generation tool at the
owner's direction. The eight `generated-*.png` files are unmodified sources.
[prompts.json](prompts.json) records the full prompts and source SHA-256 hashes.

All props use an orthographic camera looking north from due south, pitched
down 45 degrees with zero yaw. Front edges remain horizontal; box-shaped
objects show the front and top, without an exterior side face. Gray-brown
splintered timber, rust, scorched straw, and muted moss match the ruined
buildings. Wheelbarrows and wagons face the viewer directly.

The production atlas is
`assets/scenarios/rusted_kingdoms/media/tilesets/ancient_ruins_props.png`, with
matching `ancient_ruins_props.tsx`. It is **320×416 pixels**, 10 columns and
13 rows of exactly 32×32 pixels, with 130 slots. Complete objects stay together;
transparent gutters separate vegetation/hay, supplies, and wreckage bands.

| Object | Atlas column, row | Width × height in tiles | Map instances |
| --- | --- | --- | --- |
| Dead tree | 0, 0 | 3×5 | 7 |
| Charred stump | 4, 3 | 2×2 | 4 |
| Old haystack | 7, 3 | 3×2 | 6 |
| Broken wheelbarrow | 0, 6 | 2×3 | 4 |
| Wooden crate cluster | 3, 7 | 2×2 | 7 |
| Broken barrel | 6, 7 | 2×2 | 7 |
| Wagon wreck | 0, 10 | 4×3 | 3 |
| Broken fence | 5, 11 | 4×2 | 3 |

Normalization uses ImageMagick: threshold source alpha at 50%, trim clear
margins, resize with nearest-neighbor sampling while preserving aspect ratio,
align to the bottom center of the export rectangle, and reduce each object
to 96 colors without dithering. Threshold alpha again after palette reduction.
All production alpha values are exactly 0 or 255. Objects are packed losslessly
as complete blocks, without gutters between their constituent tiles.

For example, from the repository root:

```sh
mkdir -p temp/ancient-ruins-props
magick art/ancient-ruins-props/generated-wheelbarrow.png \
  -channel A -threshold 50% +channel -trim +repage \
  -filter point -resize 64x96 -gravity South -background none -extent 64x96 \
  +dither -colors 96 -channel A -threshold 50% +channel \
  PNG32:temp/ancient-ruins-props/wheelbarrow.png
```

[layout.json](layout.json) records atlas rectangles, all 41 map origins and
horizontal flips, and exact collision cells. Placements occupy former supply
yards beside houses and gate approaches; dead trees and stumps follow the
outer lanes and abandoned edges. Upper rows use the existing `foreground`
layer. Its `sort_base_y_overrides` property anchors every upper tile to the
whole object's base, following `zone_03_marshland.tmx`. Bottom rows use
`decoration`. Only tree trunks block movement; other solid props block their
base rows, with two rows for the deeper wheelbarrow/wagon footprints.

The map retains its existing layers, dimensions, portals, treasure objects,
NPCs, encounter markers, ground, terrain, and architecture. All walkable cells
remain connected, all original shortest exit/NPC approaches are preserved,
and all six chest standing areas remain accessible. The building PNG and
owner-regenerated building TSX are unchanged. Higher-zone placeholder maps
were not used as references.
