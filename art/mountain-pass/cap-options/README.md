# Southwest bridge cap proposals

Three southwest alternatives requested on 2026-10-23, targeting tile
`[34,29]` of `zone_06_mountain_pass_02`. All three were approved and expanded
into [complete four-corner sets](full-sets/README.md). The production tilesheet
contains all twelve caps, and Zone 06 uses **B — Battlement crown**.
The approved southwest PNGs below are retained byte-for-byte.

| Option | Design | Character |
| --- | --- | --- |
| A | Warded pier | Broad beveled octagonal stone, large recessed diamond ward, small snow lip |
| B | Battlement crown | Three strong merlons, deep notches, stepped plinth and carved chevron |
| C | Ram guardian | Curled alpine ram horns and a carved face on a solid stone terminal |

The built-in OpenAI image generation tool created the unmodified
`generated-a.png`, `generated-b.png` and `generated-c.png` sources. Full prompts
and source/tile hashes are in [prompts.json](prompts.json). The two reference
PNGs contain the current southwest cap and ordinary lower rail.

`option-a.png`, `option-b.png` and `option-c.png` are actual **32×32** proposals,
not the large generated sources. Export thresholds alpha at 50%, trims clear
margins, fits each proposal to the tile with nearest-neighbor sampling,
remaps to the current cap/rail palette without dithering and restores the
current cap's last four pixel columns exactly. This east connector preserves
the join with the next rail tile. Alpha remains 0/255.

Regenerate the proposals and isolated map previews from the repository root:

```sh
QT_QPA_PLATFORM=offscreen python3 art/mountain-pass/cap-options/build_previews.py
```

The script creates copies of the map, atlas and TSX under ignored
`temp/mountain-pass/cap-options/a`, `b` and `c`. All references in these copies
are relative. Each copy replaces only the southwest cap tile in its own
atlas and is rendered by Tiled. Pixel comparison against the current live map
must confine every difference to that one 32×32 tile. It also verifies exact
east-connector pixels, dimensions and binary alpha. Validation results are
saved in `temp/mountain-pass/cap-options/validation.json`.

`temp/mountain-pass/cap-options/comparison.png` shows all three enlarged tile
previews above matching map crops. Each option also has an individual
`panel.png`, full `map.png`, and `context.png` inside its preview directory.
These preview maps retain the live map's 84×64 dimensions and portal graph;
no YAML changes are needed. The full-set preview script shows all four corners together; this historical
script continues to preview only southwest alternatives against the live map.
