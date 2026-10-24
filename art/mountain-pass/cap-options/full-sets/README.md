# Decorative bridge cap sets

All three approved southwest designs have matching northwest, northeast and
southeast caps. The nine new sources were generated with the built-in OpenAI
image generation tool on 2026-10-23, before adding any of these sets to the
production tilesheet. The approved southwest PNGs remain byte-for-byte intact.

| Set | Design | NW, NE, SW, SE production IDs |
| --- | --- | --- |
| A | Warded pier: octagonal stone and recessed diamond ward | 6, 8, 18, 20 |
| B | Battlement crown: three merlons and carved chevron | 0, 2, 12, 14 |
| C | Ram guardian: curled horns and carved face | 3, 5, 15, 17 |

All twelve occupy the first two rows of the owner-arranged `mountain_pass.png`. Zone 06 uses B, at map cells
NW `[34,25]`, NE `[51,25]`, SW `[34,29]`, SE `[51,29]`. Only those four
architecture GIDs change. The six repeated middle tiles, deck, terrain,
collision and gameplay objects retain their existing data. The original
plain caps (previous IDs 12, 13, 54 and 55) were omitted from the owner's
sample and removed from the production atlas.

[prompts.json](prompts.json) retains all nine full prompts and source/tile
SHA-256 values. Each generation used its approved southwest design, an enlarged
approved tile and the matching native cap/rail join as references.
`generated-*.png` files are the unmodified sources; `a-*.png`, `b-*.png` and
`c-*.png` are the normalized 32×32 tiles. [sets.json](sets.json) records all
four corners, including paths to the approved southwest tiles, and selects B.

`build_sets.py` thresholds alpha at 50%, trims transparent margins, fits to
32×32 with nearest-neighbor sampling and remaps without dithering to the
native cap and approved southwest palette. It restores the exact four-column
connector from `native-*.png`: east on western caps, west on eastern caps.
The north and south masonry connectors retain their separate elevations.
Validation checks dimensions, binary alpha, distinct corners, all twelve
connector profiles and the approved southwest file hashes.

All three styles have upper-cap connector repairs matching the lowered upper
middle tiles. The northwest cap changes only its rightmost four columns; the
northeast cap changes only its leftmost four columns. Each cap's other 28
columns, all lower caps and the middle tiles remain pixel-identical.

The six `<style>-<corner>-join-repair.json` files retain the built-in imagegen
prompts and source/output hashes, for example [a-nw-join-repair.json](a-nw-join-repair.json)
and [c-ne-join-repair.json](c-ne-join-repair.json). Corresponding
`generated-*-join.png` files are the generated repairs, `*-before-join.png`
preserve the approved cap anchors, and `*-middle-reference.png` freeze the
adjacent bare or heavy-snow middle tile. The exporter fits the edited strip's
colors and alpha to the middle tile's row profiles. Its outermost column
matches the adjacent middle tile edge exactly. Repair inputs use the previous
IDs (A 126–127, B 130–131, C 134–135); the current IDs are in the table above.
All cap pixels are preserved when applying the sample layout.

From the repository root:

```sh
python3 art/mountain-pass/cap-options/full-sets/build_sets.py
python3 art/mountain-pass/build_tileset.py
python3 art/mountain-pass/build_maps.py --segment 02
QT_QPA_PLATFORM=offscreen python3 art/mountain-pass/cap-options/full-sets/render_sets.py
python3 art/mountain-pass/audit_maps.py
```

The main tilesheet exporter also runs `build_sets.py` before packing. The
upper middle tiles move down ten pixels by transferring transparent bottom
rows to the top (`north.middle_offset_y` in `bridge_tiles.json`). The final
[placement manifest](../../atlas_placement.json) matches the owner's sample:
384×352, 12 columns, 11 rows, 132 slots and 106 visible tiles. All retained cap
pixels stay intact; the active bridge still uses ten distinct tiles.

Previews and validation reports are written under ignored
`temp/mountain-pass/cap-options/full-sets/`:

- `all-sets.png`: enlarged tiles, grouped by style and corner.
- `bridges-comparison.png`: all three complete bridges rendered by Tiled.
- `b/map.png` and `b/bridge.png`: the selected Zone 06 style in context.
- `validation.json`: dimensions, alpha and connector checks for the nine new tiles.

The render script makes independent map copies and uses the selected B caps
as its baseline, since the plain caps were removed. Every rendered difference must stay inside the four cap
cells. Production map references, dimensions and the portal graph are preserved.
