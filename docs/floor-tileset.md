# Floor tileset layout

`assets/scenarios/rusted_kingdoms/media/tilesets/floor.png` uses 32×32 tiles,
16 columns and 12 rows. IDs are zero-based Tiled local IDs; rows and columns
below are one-based. The 2026-10-20 reorganization retained 81 visible tiles;
five generated rock-building tiles then brought the count to 86. Removing
five unused tiles leaves 81 visible tiles in the same 192 slots. The vertical
bridge adds nine tiles, bringing the current visible count to 90.
Tile relocations preserve rendering; later pixel repairs are described below.

| Group | Current IDs | Layout |
| --- | --- | --- |
| Frequently used wood | 0, 1 | Former tiles 14 and 30, first two cells |
| Other wood textures | 2–12 | Rest of row 1 |
| Nave flagstones | 16, 17, 32, 33 | 2×2 block, rows 2–3, columns 1–2 |
| Aisle flagstones | 18, 19, 34, 35 | 2×2 block, rows 2–3, columns 3–4 |
| Sanctuary flagstones | 20, 21, 36, 37 | 2×2 block, rows 2–3, columns 5–6 |
| Retained sandy ground | 22 | Former tile 179, originally tile 15 |
| White hex-circle | 39–41, 55–57, 71–73 | 3×3 block starting at row 3, column 8 |
| Cyan hex-circle | 42–44, 58–60, 74–76 | 3×3 block starting at row 3, column 11 |
| Rope bridge | 45–47, 61–63, 77–79 | 3×3 block starting at row 3, column 14 |
| Vertical rope bridge | 93–95, 109–111, 125–127 | 3×3 block starting at row 6, column 14 |
| Bordered cave ground | 80–82, 96–98, 112–114 | 3×3 block, rows 6–8, columns 1–3 |
| Rock object with rubble base | 83–85, 99–101, 115–117, 131–133 | 3×4 block, rows 6–9, columns 4–6 |
| Blue stepping stones | 88, 89, 104, 105 | 2×2 block, rows 6–7, columns 9–10 |
| Additional cave ground | 128–130 | Row 9, columns 1–3 |

Wood order in row 1, expressed as **old IDs → new IDs**:
`14, 30, 16, 1, 17, 0, 2, 18, 19, 20, 21, 22, 23` → `0–12`.

## Provenance review, 2026-10-29

The current `floor.png` is project-authored artwork under the root MIT license.
The repository owner confirmed that tiles not migrated from the retired source
sheets were AI-generated and that no new third-party artwork was downloaded
for this atlas. The old ledger classification described an earlier mixed atlas:
its 56 LPC tiles were removed in two passes and maps now reference `terrain-v7`
directly. The owner clarification, generation history and comparisons below
are the basis for the corrected `ALI-1109` entry.

Compared all 90 visible floor tiles with the last tracked version before deletion
of each of 22 PNG sheets found in this branch's tileset deletion history, plus
current `terrain-v7.png`. Historical PNG bytes were resolved from local Git LFS
objects and checked against their pointer hashes. Compared exact 32x32 RGBA
cells on the source grid, normalizing RGB only where alpha is zero.

| Source | Matching current floor IDs | Result |
| --- | --- | --- |
| Retired `cave.png`, parent of `2cb1967` | 88, 89, 99, 101, 104, 105, 115, 117, 132 | 9 tiles; former ALI-1025 identifies this source as AI-generated |
| Retired `decor.png`, parent of `2cb1967` | 39–44, 55–60, 71–76 | 18 tiles; former ALI-1031 identifies this source as AI-generated |
| Other 20 deleted sheets, including the replaced downloaded sheets | none | No exact cell matches |
| Current `terrain-v7.png` | none | No exact cell matches |
| Remaining current floor tiles | 63 tiles | Classified as AI-generated from the owner's statement and recorded generation history |

The comparison checks final source snapshots and exact cells. It does not
exclude edited, scaled, rotated or offset source fragments and is not, by
itself, proof of generation provenance. Original migration and later generated
repairs remain documented below and in Git history. Temporary machine-readable
results and the comparison script are in `temp/2026-10-29-001-floor-provenance/`.

### Complete upstream folder comparison

The later comparison covered every PNG recursively under
[`rusted_kingdoms/assets/tilesets` in `do-i/agentic-rpg`](https://github.com/do-i/agentic-rpg/tree/08970359d6cb03586948625d29b0d3351dbbf785/rusted_kingdoms/assets/tilesets),
pinned to main commit `08970359d6cb03586948625d29b0d3351dbbf785`.
The upstream inventory contains 19 PNGs: eight Astral Pixels sheets, one Beeler
sheet, three Schwarnhild sheets, and seven sheets elsewhere, including the
original `ground/terrain-v7.png`. All 19 PNGs were resolved and verified against
their upstream LFS pointer SHA-256 hashes; none were skipped.

All 90 visible current floor tiles were searched against every pixel position
in every upstream PNG, including off-grid positions, horizontal/vertical flips
and 90-degree rotations. Where a floor tile was an exact nearest-neighbor 2x
enlargement, its 16x16 source form was also searched. RGB under fully transparent
pixels was ignored. **No matches were found in any of the 19 upstream PNGs.**

This supersedes the narrower on-grid source comparison above for detecting
unchanged migrated artwork. It does not rule out recolored, repainted or
otherwise modified derivatives; project-authored provenance also relies on
the owner's statement and generation records. The pinned inventory, LFS
manifest, source PNGs, comparison script and full results are retained in
`temp/2026-10-29-002-floor-upstream-audit/`.

## Removed tiles and opaque ground

Unused broken stone edges 92–93 and former terrain review tiles 176–178
were cleared on 2026-10-20. Their tile metadata was removed after verifying
no map, template, tile-object or stamp references. Atlas dimensions and every
remaining ID are unchanged. The former review textures were dark grass,
cave floor and alternate grass; the Corrupted Forest uses `terrain-v7.tsx`.

The nine-tile ground block is now fully opaque:

```text
 80  81  82
 96  97  98
112 113 114
```

Filled 936 transparent edge/corner pixels in this block and 56 transparent
mortar pixels in rock tile 116. Tile 97 was already opaque. Every previously
opaque pixel is preserved exactly, as are all unrelated tiles. The fills use
built-in `imagegen`, remapped to existing dark border/mortar colors and applied
only through the original transparency masks.

Opacity repair prompt: fill only the transparent outer-edge and corner gaps
in the 96×96 bordered floor block, and transparent holes in tile 116, with
matching brown border/mortar; preserve every opaque pixel, tile location,
palette and all unrelated transparency; no added details, labels or text.

The original rough inner border was restored across the whole 3×3 block,
superseding the earlier generated thin border on tile 112. The current border
uses exact pixel copies, with no newly generated artwork:

- Left tiles 80/96/112: move original columns 4–6 into columns 1–3.
- Right tiles 82/98/114: mirror the move, copying columns 4–6 counted from the
  right into the three outermost right columns.
- Fill the vacated side strips with columns 15–17 of middle floor tile 97.
- Bottom tiles 112–114: extend the middle floor through the former dark band;
  rotate tile 96's original three-column rough inner edge onto the bottom
  three rows. Join the two corner intersections using the same source strips.

Column/row positions in these copy instructions are one-based within each
32×32 tile. The original border source is the opaque atlas before the isolated
tile-112 generated-border edit. Tiles 81 and 97 retain their pixels; only the
seven side/bottom tiles change. The block remains fully opaque and retains
the source palette, tile IDs and dimensions. All map data remains unchanged.
The result was previewed in `zone_02_open_plains_cave_01`, including the join
between floor tile 112 and masonry tile 115 below it.

The sandy tile remains at ID 22 (former 179, originally 15). Its uses are
`zone_10_hearth_core`: ground 52; each of `zone_10_hearth_descent_01` through
`zone_10_hearth_descent_06`: ground 56.

## Rock object

Select this complete 3×4 rectangle in Tiled (96×128 pixels):

```text
 83  84  85
 99 100 101
115 116 117
131 132 133
```

The nine upper tiles moved one row down and one column left to join the
existing rubble base at 131–133. Their former IDs map as follows:

```text
68–70   → 83–85
84–86   → 99–101
100–102 → 115–117
```

The former top corners 68 and 70 (now 83 and 85) had inset lower side edges.
Their masonry now extends to the outer edges and their brown inset borders
line up with the middle side tiles. The other seven moved tiles retain their
opaque pixels; the later opacity fill affects only transparent pixels in 116. All 22 existing placements in `zone_02_open_plains_cave_01` use the
relocated IDs; relocation alone preserves their rendering.

The upper halves of base corners 131 and 133 were subsequently repaired to
continue the masonry of 115 and 117. Their top pixel rows match the bottom
rows above exactly, and their outer silhouettes taper into the existing
rubble. Both lower halves and center base tile 132 retain their pixels. The
map render changes only at the ten existing base-corner placements. Object
alpha remains binary and all colors come from the existing palette.
Collision for new placements must be authored separately.

The original five top/center additions were generated with the built-in
`imagegen` tool on 2026-10-20. The corner repair also uses built-in `imagegen`,
with generated side strips normalized to the source palette and pixel grid,
and aligned to the preserved side tiles. The base-corner repair also uses
built-in `imagegen`, normalized to the same palette and pixel grid, with exact
joining rows and binary silhouette masks. All other tiles are preserved.

Corner repair prompt: fix only the top-left and top-right caps so their lower
outer edges and brown inset borders join the vertical stone sides seamlessly;
preserve the rounded upper corners, original coarse brown/taupe pixel style,
all other tiles and transparency; no added details, labels or text.

Base repair prompt: repaint only the upper halves of lower-left tile 131 and
lower-right tile 133 so stone faces, highlights, mortar and silhouettes
continue from 115 and 117; preserve the upper row, lower center and lower
rubble halves, coarse pixel style and palette; no ledge, border or added detail.

Top trim at 83–85 was subsequently flattened and reduced to five pixels:
four rows of masonry and one row of subtle brown inner shadow, versus the
approximately seven-pixel side borders. The top silhouette now reaches a
straight, fully opaque row at the tile boundary, removing the raised corner
lobes, inset shoulders and gaps. The lower twelve rows of all three tiles,
all other tiles, the source palette, IDs and map data remain unchanged.

Top-trim repair prompt (built-in `imagegen`): flatten the top edge across
83–85, remove raised corners and indents, reduce the top band to 1–2 pixels
less than the side borders, add subtle highlights and a light inner shadow;
preserve the floor/side style and lower joins, original palette and pixel
grid; no new details or heavy shadows. Normalize the generated stone strip
to four rows with one shadow row, preserving the original lower joins.

`bridge_short.stamp` and `bridge_long.stamp` now use the relocated bridge tiles.
The 40 transparent floor-tile references formerly using ID 7 in
`zone_02_open_plains_cave_01` now use empty ID 191, keeping them transparent
while the first row is filled with wood textures. Map dimensions, portals,
collision layers and spawn data were unchanged by those relocations. The later
[Corrupted Forest redesign](corrupted-forest.md) adds collision for poisoned
water while preserving access to its authored gameplay destinations.


## Vertical rope bridge

Added a north–south counterpart on 2026-10-20 with built-in `imagegen`,
normalized to 96×96 pixels, the original bridge palette and binary alpha.
Select this 3×3 block in Tiled (rows 6–8, columns 14–16):

```text
 93  94  95
109 110 111
125 126 127
```

Horizontal planks cross the deck, rope rails run along both sides, and four
upright posts retain the original bridge's wood and metal-band styling.
The destination slots were transparent and unreferenced in maps/templates
and stamps. All existing atlas pixels, IDs, dimensions and map data are
preserved. New placements need their own collision data.

Generation prompt: create a north–south counterpart of the existing wooden
rope bridge in the same overhead RPG perspective and chunky pixel style;
use horizontal deck planks, rope rails along the sides, four upright capped
posts with metal bands, warm tan wood and brown ropes. Keep the north/south
entrances open and the background transparent; fit one 3×3 block of 32×32
tiles. No water, ground, scenery, grid, labels, text, blur or soft shadows.


Tiles 109 and 111 were subsequently repaired to support vertical repetition.
The ropes now run vertically through the middle section with identical RGBA
profiles at their top and bottom edges; the full-tile alpha profiles also
match. Repeat the complete row `109 110 111` between the existing end rows
to extend the bridge. Tile 110, both end rows and every unrelated atlas pixel
are preserved. A six-row preview with four middle repeats verified continuity.

Repeatability repair prompt (built-in `imagegen`): replace only the drifting
side ropes with straight vertical braided ropes of constant thickness and
matching top/bottom highlights, shadows and transparency; preserve the wood
planks, center tile, source palette and pixel style. No posts, end caps,
background, text or extra details. Generated rail strips were normalized to
the original palette and fitted to tiles 109/111 with exact matching boundary
pixels; the original plank interiors remain unchanged.


The lower anchors were subsequently aligned inward to the accepted upper
anchors, correcting the original wider bottom span. This supersedes the
previous middle-only fit: only tiles 109/111/125/127 change. The top row
93/94/95, center-column tiles 110/126, all other tiles and map data remain
unchanged. The bottom posts reuse the accepted upper post footprints at the
same horizontal positions. Generated rail strips fit the upper rope exits;
RGBA profiles match exactly across 93->109, 109->109, 109->125 and the right
side equivalents 95->111, 111->111, 111->127. Both short and extended bridge
previews verify the same rail span throughout.

Alignment repair prompt (built-in `imagegen`): preserve the correct top
corners; move the bottom ropes and posts inward to line up with their upper
counterparts; put straight, vertically repeatable middle ropes between them.
Preserve the center column, horizontal planks, palette, pixel grid and alpha.
Generated lower side strips were fitted to the original palette, accepted
upper exit pixels and copied upper anchor footprints.
