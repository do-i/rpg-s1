# Mountain Pass — Zone 06

Three alpine maps connect Frostholm to Ashenveil. Exposed slate trails,
snowfields, sparse dark pines and snow-capped granite continue Zone 05's
visual language. Each map has a different landmark and route structure.

| Map | Previous size | Redesigned size | Layout |
| --- | --- | --- | --- |
| `zone_06_mountain_pass_01` | 58×32 | 76×58 / 2432×1856 px | Lower pass, stone pilgrim refuge, climbing switchback, northern ridge loop and southeast cache spur |
| `zone_06_mountain_pass_02` | 52×36 | 84×64 / 2688×2048 px | Deep ravine, broad stone crossing, broken lower rope bridge, western overlook and eastern supply ledge |
| `zone_06_mountain_pass_03` | 50×34 | 80×64 / 2560×2048 px | Icy tarn loop, broad shaman court, carved ward gateway and southern cache trail |

Snow and exposed ground remain walkable. All open cells form one connected
area on each map. Water/ice and the ravine block movement. Ridges block only
their lower two tile rows; their upper silhouettes sort in front of actors
walking behind them. Tree roots, boulders, refuge foundations, gate pillars,
boards and chests have collision under their visible solid bases. No extra
layers or invisible perimeter walls are added.

The stone bridge is the ravine crossing, with three walkable deck rows
between shallow parapets. A higher, nearly overhead camera removes the
exposed piers and tall front wall. The bridge sits flush with the banks;
its deck reuses the native trail paving in the same tile phase, with matching
three-row approaches. Its atlas rectangle begins at `[34,24]`, with the deck
on rows 26–28 and parapets on rows 25 and 29. The lower rope bridge
starts at `[30,47]`, with its deck on row 48: one tile north and two west.
Its middle two columns, 40–41, are missing across all three rows. Snapped
ropes and splintered planks face the gap; the partial inner deck tiles block
movement before the drop. Both surviving bridge halves are reachable from
their banks, but cannot connect them. Closing the stone deck separates the
banks. The gate has a four-tile-wide passage beneath its overhead lintel.

## Connections

Frostholm ↔ Mountain Pass ↔ Ravine Crossing ↔ Shaman Gate ↔ Ashenveil remains
the complete Zone 06 graph. The two towns change only their incoming arrival
coordinates; their art, collision and portal rectangles remain intact.
All six Zone 06 portal strips are inside their maps and span five rows.

| Source | Destination arrival |
| --- | --- |
| Frostholm north exit | Mountain Pass `[1,44]` |
| Mountain Pass west exit | Frostholm `[4,2]` |
| Mountain Pass east exit | Ravine Crossing `[1,44]` |
| Ravine Crossing west exit | Mountain Pass `[74,16]` |
| Ravine Crossing east exit | Shaman Gate `[1,46]` |
| Shaman Gate west exit | Ravine Crossing `[82,18]` |
| Shaman Gate east exit | Ashenveil `[19,29]` |
| Ashenveil south exit | Shaman Gate `[78,15]` |

## Actors, encounters and caches

Pilgrim Edras retains his identity, dialogue, visible-face armored sprite and
left-facing idle. His YAML anchor moves to `[18,37]`, placing his actual feet
near `[19,38]`, south of the refuge. His entire 20×18 collision rectangle is
clear and has a reachable interaction approach.

The Red Hood Shaman waits in the open court at `[58,28]`. The once-only fight
and `boss_zone06_defeated` completion flag are preserved. The gateway adds no
new travel requirement. Existing enemy formations, weights, zero chase ranges,
battle background and spawn populations/intervals are unchanged. Patrol
markers increase from six each to 14, 16 and 14, with clear arrivals and no
regular patrol start inside the immediate boss court.

Each map retains its two original persistent chests: six total. Object names,
numeric IDs, catalog reward references and trap behavior are unchanged.
Every chest blocks its own tile and has a reachable adjacent interaction cell.
Each existing trail board remains reachable with its map-specific dialogue.

## Layers and art

All existing layer names, IDs, order and object groups are retained. Collision
and spawn markers remain hidden. Upper object rows use `foreground` with
per-cell sorting at each object's base; lower rows use `decoration`. The
stone parapets use `architecture`; the walkable deck reuses the existing
trail tiles in `terrain` underneath them.

Existing tileset GIDs and source pixels are preserved. The first two maps add
`mountain_foothills.tsx` at 2388; Shaman Gate adds it at 2628. Ravine Crossing
adds `mountain_pass.tsx` at 2847 and Shaman Gate at 3087. New source artwork,
full prompts, normalization, regeneration and navigation checks are retained
in [the Mountain Pass art directory](../art/mountain-pass/README.md).

The bridge parapets use ten tiles instead of 36: separate upper and lower
sets with two end caps and three middle variants (bare, light snow and heavy
snow). Different middle sequences preserve the varied snow coverage.
`art/mountain-pass/bridge_tiles.json` retains the source cuts and sequences.
The production atlas follows the owner's `sample.tmx`: **384×352**, 12×11
tiles, **132 slots and 106 visible tiles**. The first two rows group bridge
parts by style: B caps at IDs 0/2/12/14, C ram caps at 3/5/15/17, and A caps
at 6/8/18/20. Bare, light-snow and heavy-snow middle tiles occupy 1/13, 4/16
and 7/19. Six rope-end tiles form a 3×2 block at column 9, with the sample's
transforms baked into pixels. The gateway starts at `[0,2]`.

The four unused plain caps (previous IDs 12/13/54/55) are omitted. All retained
tiles preserve their exact pixels and sample transforms. The 48 Ravine
Crossing and 82 Shaman Gate placements are remapped, with compensated rope
flip flags; both full-map renders stay pixel-identical. The sample's layout
also renders identically after remapping its references.

Upper middle tiles move their ten transparent bottom rows above the unchanged
stonework. Upper-cap connectors fit this lower elevation while preserving each
cap's other 28 columns and all lower-cap pixels. Zone 06 uses B, with ten
active bridge tiles. Sources, prompts, placement data and previews are retained
in [the art directory](../art/mountain-pass/README.md).
The gateway stays a complete 12×9 object and the broken rope end stays 2×3.
`art/mountain-pass/tile_layout.json` maps the authoring layout to production
IDs. The deck and both banks still share their native terrain tile, with
unchanged collision. Only the two stone parapet strips change appearance;
the rest of Ravine Crossing and the whole Shaman Gate render are preserved.

## Validation

The three saved maps pass the skill's strict TMX CSV validator and render
successfully with Tiled's `tmxrasterizer`. The navigation audit verifies all
arrivals, portals, boards, patrols, chest approaches, NPC feet and the boss
court, plus the stone crossing, impassable rope gap and the gate opening. The game's map
sweep loads all 74 maps with no findings or runtime failures. Encounter and
dialogue sweeps pass. Scenario validation matches the existing four accepted
Sorcerer ultimate-flag diagnostics, with no new findings.

For the sample-layout migration, strict CSV validation, pixel-identical Tiled
renders, isolated map rebuilding and the navigation audit pass. The runtime
sweep was not rerun for this migration because the local game executable was
unavailable.
