# Mountain Pass art and authored layouts

The bridge and warded gateway were generated on 2026-10-23 using the built-in
OpenAI image generation tool for the Zone 06 redesign. The current bridge
source is `generated-bridge-high-angle.png`; `generated-bridge.png` retains
the superseded low-camera version. The gateway source is `generated-gate.png`.
`generated-broken-rope-end.png` adds the snapped wooden bridge ends, using the
existing floor bridge pixels in `rope-break-reference.png` as a reference.
Full prompts, source SHA-256 values and the previous bridge prompt are retained
in [prompts.json](prompts.json).

The stone bridge uses a nearly overhead orthographic camera, about 75–80
degrees above ground, looking due north with no yaw. The gateway retains the
45-degree camera. Both keep horizontal edges horizontal and share the alpine
granite, pale snow and muted timber palette. The bridge's shallow parapets
frame a broad trail surface with no exposed piers or legs.

The production atlas is
`assets/scenarios/rusted_kingdoms/media/tilesets/mountain_pass.png`, with
companion `mountain_pass.tsx` metadata. It is **384×352 pixels**,
12 columns by 11 rows: **132 slots and 106 visible tiles**. It matches the
owner-arranged `assets/scenarios/rusted_kingdoms/media/tilesets/sample.tmx`.

The stone bridge uses exactly **ten reusable tiles**, reduced from 36.
Its four decorative caps use **B — Battlement crown**. All three approved
designs have four-corner sets. The four unused plain caps were omitted in
the owner's sample and are removed from the production atlas.
The upper and lower parapets retain separate masonry designs. Each set has
a left cap, right cap, bare middle, light-snow middle and heavy-snow middle.
[bridge_tiles.json](bridge_tiles.json) records the ten 32×32 source cuts and
two different sequences of sixteen middle placements. The light-snow cuts
are offset slightly from the original grid to keep each snow patch inside
its tile; middle variants have continuous opaque masonry at both joins.
Source cuts preserve the generated palette and binary alpha.

| Production group | Column, row (zero-based) | Layout |
| --- | --- | --- |
| B — Battlement crown | 0, 0 | 3×2; caps at IDs 0/2/12/14; bare middle at 1/13 |
| C — Ram guardian | 3, 0 | 3×2; caps at IDs 3/5/15/17; light snow at 4/16 |
| A — Warded pier | 6, 0 | 3×2; caps at IDs 6/8/18/20; heavy snow at 7/19 |
| Broken rope end | 9, 0 | 3×2; sample transforms baked into pixels |
| Shaman gateway | 0, 2 | Complete 12×9 object |

[atlas_placement.json](atlas_placement.json) freezes every placement and flip
from the owner's sample before remapping it. The exporter reconstructs the
previous source groups, then copies each retained tile into these exact
positions. Previous plain-cap IDs 12, 13, 54 and 55 are the only visible tiles
omitted. No source-art files are deleted. The sample's references now point
to the rearranged IDs, and its rendered pixels stay identical.

The ten active bridge tiles appear in the map. Upper middle tiles 1, 4 and 7
move ten fully transparent bottom rows to the top, preserving their stone and
snow pixels; `north.middle_offset_y` in [bridge_tiles.json](bridge_tiles.json)
records this adjustment. All three styles' upper-left caps' last four columns
and upper-right caps' first four columns fit the lowered rail. Each cap's
other 28 columns and all lower caps remain intact.
Full prompts, generated sources, exact connector checks and preview commands
are in [the four-corner cap source directory](cap-options/full-sets/README.md).
Snow coverage varies along the
bridge by mixing the three middle variants. The 36 rail placements use ten
IDs; the deck retains its native terrain tile. Both banks, deck, collision,
portals, chests, broken wooden bridge and all artwork outside the two rail
strips remain unchanged. Shaman Gate's full-map render stays pixel-identical.
The atlas remains a lossless indexed PNG after selecting the reusable parts.

The `atlas_rect` values in [prompts.json](prompts.json) describe the unchanged
18×18 **authoring layout**, not production IDs. The export reconstructs that
sheet at `temp/mountain-pass/export/object-layout.png`, normalizes each whole
object, then [atlas_layout.py](atlas_layout.py) selects the ten bridge parts,
assigns repeated placements and applies the sample placement manifest.
[tile_transforms.py](tile_transforms.py) compensates map flags for the six
rope tiles whose transforms are baked into the PNG. [tile_layout.json](tile_layout.json) records every
logical source ID to production ID. Map builders resolve that mapping during
placement, so regeneration retains the optimization.

In the authoring layout, the bridge occupies an 18×7 rectangle at `[0,0]`.
Its visible content is normalized to 576×160 and placed 32 pixels below the
top. Rows 1 and 5 contain the shallow parapets and near fascia. The deck
interior at local pixel rows 54–167 is transparent, revealing canonical trail
tiles in the map's `terrain` layer. Those pixels remain in `terrain-v7.png`.
Rows 2–4 form the three-tile-deep walkable deck. The native terrain retains
its world tile phase, so paving joins both banks exactly. Rows 0 and 6 stay
transparent, with no extra collision beyond the ravine.

The gate's central passage occupies local object columns 4–7; its lintel sorts
above actors passing beneath it. The broken rope end uses fixed-canvas
nearest-neighbor normalization to 64×96, the original bridge palette, binary
alpha and the reference's exact leftmost 16 pixel columns for a matching join.
The map's compensated flip flags restore the original right-facing object,
and horizontal flipping supplies the opposite end. The reference repeats floor
tiles 46/62/78 without editing those source pixels. The gateway, broken rope
art and their placements retain every original pixel after production packing.

Rebuild and audit from the repository root:

```sh
python3 art/mountain-pass/build_tileset.py
python3 art/mountain-pass/build_maps.py
# Rebuild only Ravine Crossing:
python3 art/mountain-pass/build_maps.py --segment 02
python3 art/mountain-pass/audit_maps.py
```

The export thresholds alpha at 50%, trims clear margins, resizes with nearest
neighbor sampling and reduces generated objects to 128 colors without
dithering. The bridge uses its fixed content rectangle and clears the deck
interior; the gateway aligns to the bottom center. The broken rope end uses
its preserved source seam and reference palette. It verifies exact
normalized object pixels, preserved gateway/rope pixels, exactly ten bridge
variants, continuous masonry joins, binary alpha and transparent unused slots.
Existing terrain and alpine atlas images are not edited.

`build_maps.py` deliberately overwrites artwork, placements and route geometry
in the three Zone 06 TMX files. It retains their existing layers, IDs, order,
chest identities and reward references. [layouts.json](layouts.json) records
placements, flips, object collision, arrivals, chests, boards, patrols and bridge
decks. NPC YAML and the two towns' incoming arrival coordinates are maintained
separately, as documented in [the zone guide](../../docs/mountain-pass.md).

`audit_maps.py` reads the saved TMX, rather than an in-memory design. It checks
20×18 player/enemy foot rectangles, Edras's offset NPC foot rectangle, connected
open ground, arrivals, portals, chest collision/approaches, boards, encounters,
the gate passage, traversable stone crossing and broken rope bridge. Both
rope halves remain reachable from their own banks; the gap and partial
planks are blocked. Closing the stone deck must separate the banks. It writes its report to the ignored
`temp/mountain-pass/navigation-audit.json`.

To render a full-size preview with Tiled:

```sh
mkdir -p temp/mountain-pass/previews
QT_QPA_PLATFORM=offscreen tmxrasterizer \
  assets/scenarios/rusted_kingdoms/media/maps/zone_06_mountain_pass_02.tmx \
  temp/mountain-pass/previews/02.png
```
