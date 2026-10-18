# Ardel Shrine

The shrine and its sealed epilogue stage share an 18 by 18 tile sanctuary
(576 by 576 pixels). A sun medallion, book-covered altar, votive candles,
holy-water font, and three pairs of prayer pews establish the shrine's use.
Scorched pews, cracked pillars, and fallen beams retain the fire damage
described by Keeper Joss's dialogue.

The town entrance arrives at `[8, 16]`, just inside the two-tile-wide south
doorway. The south exit returns to Ardel at `[3, 18]`. The north passage is
closed, and all six pews face north toward the altar. Keeper Joss stands
beside the altar at `[6, 6]` and uses the
existing elderly female villager sprite, matching her dialogue. The
epilogue keeps its arrival at `[14, 5]`, Joss at `[13, 5]`, and no exit portal.
The companions stand on open floor around the sanctuary.

`shrine_props.png` is a 288 by 416 RGBA atlas with 117 tiles, each 32 by 32.
Its TSX marks the top-left tile and footprint of each prop. Rows 9 and 10
(zero-based) contain three 2 by 2 repeating flagstone patterns: nave, pale
aisle, and warm sanctuary. The final two rows add north-facing intact and
damaged pews (local tile IDs 99 and 102), showing the rear of their backrests.
The earlier south-facing variants remain available for the epilogue.
Props use binary alpha; floor tiles are opaque.
The shrine's Tiled stack, from top to bottom, is `foreground`, `decoration`,
`architecture`, `ground`, hidden `collision`, and the `portals` object group.
It has no `terrain` layer; other maps may place one between `architecture`
and `ground`. Walls live in `architecture`, and wall-mounted candles and the
medallion live in `decoration`, so their transparency reveals the stonework
without replacing wall tiles. Collision is authored separately from the
visible artwork.

The playable shrine uses the general `foreground` layer for upper sections
that cover characters walking behind them. Pillar capitals and shafts,
candelabrum tops, the font's upper section, and pew backs live here. Their
base tiles stay in `decoration`, below characters standing in front. Low
objects such as offerings and fallen beams also stay in `decoration`.
The hidden `collision` layer independently marks the solid footprints.
The epilogue now uses the same layer convention and upper/base split, while
retaining its earlier sanctuary layout and south-facing pew artwork.
See [Map layers](map-layers.md) for the authoring convention.

The new artwork was generated with the built-in imagegen tool on 2026-10-17
at the repository owner's direction. Props were cropped separately,
normalized with nearest-neighbor sampling to whole tile footprints, and
assembled into the atlas. The floor was normalized to a 64 by 64 repeat
with three brightness/palette variants. Existing shared atlases were reused.
The home-grown asset ledger records the resulting PNG and TSX under MIT.

The altar's only pixel changes are seven transparent pixels on atlas rows
63 and 64, the shared edge between its upper and lower tile rows. These now
use opaque neighboring outline colors. All other original atlas pixels,
including the altar's outer silhouette away from that seam, are preserved.

## Prop generation prompt

```text
Use case: stylized-concept
Asset type: production pixel-art prop atlas for a 32x32-tile top-down fantasy RPG, transparent background.
Create ONE square sheet of NINE isolated props in an exactly evenly spaced THREE COLUMN by THREE ROW grid. Every grid cell has equal width and height. Leave generous transparent margins between objects, nothing touches cell boundaries. No grid lines, labels, lettering, text, background, floor, characters, scenery, watermark.
Art direction: authentic crisp hand-pixelled SNES RPG furniture viewed from above and slightly in front (orthographic three-quarter top-down, NOT isometric). Muted warm grey stone, weathered dark oak, faded burgundy cloth, antique gold, small amber candle flames. Hard pixel clusters, limited palette, dark one-pixel outlines, light from upper left, no blur, no smooth digital illustration. Objects are for a humble fire-damaged village shrine which is still tended.
Row 1 left: wide LOW stone altar, rectangular slab, cracked corners, short stone base, faded burgundy cloth draped over front, a small open book on top, two small lit candles. Width about twice height.
Row 1 middle: slender standing brass candelabrum with three lit candles, tall narrow silhouette.
Row 1 right: round grey stone holy-water font on a short carved pedestal, shallow turquoise water visible from above.
Row 2 left: short horizontal dark oak prayer pew, upright backrest behind bench seat, sturdy legs visible, front facing south. Width about three times height.
Row 2 middle: matching damaged horizontal prayer pew, one broken backrest and a scorched end, dark charred wood, still recognizable as a pew.
Row 2 right: short fallen charred roof beam with a few small ash rubble stones, horizontal.
Row 3 left: carved stone wall medallion containing a simple gold eight-point sun/star motif, weathered and slightly cracked, no text.
Row 3 middle: small bronze offering bowl holding a few gold coins.
Row 3 right: cluster of five short ivory votive candles, melted wax at base, warm amber flames.
Consistency: All nine objects in precisely equal invisible cells, centered, consistent front-down orientation. Fully transparent background with clean alpha. Only tiny contact shadows within each object's own footprint.
```

## Directional pew and altar seam edit prompt

The built-in imagegen tool edited a reference containing the original
intact pew, damaged pew, and altar. The rear-view pews were normalized to
96 by 64 pixel footprints and appended to the atlas. The altar was retained
from the original, with only the seven transparent shared-edge pixels
filled using neighboring opaque outline colors.

```text
Use case: precise-object-edit
Edit target: the attached three-panel pixel-art sheet. Preserve its transparent background and the equal three-panel horizontal layout. It contains an intact wooden pew on the LEFT, a scorched broken pew in the CENTER, and a stone altar on the RIGHT.
LEFT AND CENTER: change ONLY their viewing direction. These prayer pews must now face NORTH, away from the viewer toward a shrine altar above them. Show the BACK/REAR of each upright backrest nearest the viewer at the SOUTH/bottom of the image, with the seating side facing away toward the top. Keep furniture upright, feet at the bottom, NOT an upside-down flip and NOT isometric. Rear planks and simple rear support details should be visible; do not show the current front-facing carved arches and broad seat opening looking south. Maintain the dark weathered oak, short legs, dimensions, pixel scale, shading style, and fire damage. The intact pew remains intact; the broken pew retains a missing/burned right section.
RIGHT: retain the altar design, book, candles, burgundy sun cloth and silhouette. Repair transparent gaps ONLY at the horizontal join halfway down its 64-pixel-tall footprint, where the upper 32-pixel tiles meet the lower 32-pixel tiles. Make the two touching pixel rows at that join solid continuous altar material, extending to the left and right ends. Keep all other transparency unchanged.
Authentic crisp SNES-style hand pixel art, limited palette, light upper left. Keep the three assets isolated with no new props, no floor, no text, no borders, no shadows outside their original footprints. Transparent background, not black or white.
```

## Floor generation prompt

```text
Use case: stylized-concept. Asset type: seamless repeating pixel-art floor texture for a 32-pixel-tile SNES top-down RPG shrine.
Make a square, full-bleed, edge-to-edge texture of old interior stone flagstones viewed exactly from directly overhead. Warm muted medium-grey limestone, square and rectangular dressed slabs laid in an orderly staggered pattern, fine dark mortar seams, subtle chisel marks, a few hairline cracks and worn corners. About FOUR large stone blocks across the complete image, not many tiny cobbles. Completely flat floor, no perspective, no bevelled blocks, no wall faces, no grass, no moss, no pebbles, no props, no borders, no labels, no lettering. Low contrast weathering, restrained grey taupe palette. Authentic hand-pixelled crisp pixel clusters and tiny one-pixel highlights, as if the ENTIRE square were only 64 by 64 pixels enlarged with nearest-neighbor. Left and right edges must tile seamlessly, top and bottom must tile seamlessly. No lighting gradient or vignette. Fully opaque.
```
