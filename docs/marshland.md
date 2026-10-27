# Marshland

`zone_03_marshland` is a 76×60 map of peat islands, still green water,
willows, cypresses, reed beds, and an old stone causeway. Its 2026-10-26
redesign uses the current Zone 05/06 maps as references, including the owner's
cosmetic edits. Those reference maps and their shared atlases are unchanged.

The eastern raised path links the Open Plains entrance to Harborgate. Six
boardwalks connect the reedcutter's bank, central willow island, western
boss court, northern cache, and southern fen loop. Optional branches retain
the original three chest rewards. Small pools and reeds interrupt the larger
eastern bank without obscuring the main path.

## Gameplay contracts

| Connection | Arrival in Marshland | Outgoing destination |
| --- | --- | --- |
| Open Plains | `[40, 1]` | `zone_02_open_plains [3, 18]` |
| Ancient Ruins gate | `[1, 43]` | `zone_04_ancient_ruins_01_gate [70, 29]` |
| Harborgate | `[57, 58]` | `town_03_harborgate [4, 1]` |

Neighboring maps change only their Marshland destination coordinates. Portal
identities and target maps remain intact; arrival feet clear the exit triggers.

Fen Farmer Olan and Reedcutter Sela retain their IDs, sprites, dialogue, and
rewards. Their YAML sprite anchors are `[56, 47]` and `[57, 32]`. Sela stands
outside the reedcutter shelter. The Black Mask Doctor occupies the western
court at `[10, 43]`, retaining the once-only encounter and
`boss_zone03_defeated` completion flag. The 18 encounter markers keep clear of
arrivals, NPCs, scenery, and the boss court. Encounter balance and music are
unchanged.

The original chest object IDs and `item_box_id` values are retained:

| Chest | Floor tile |
| --- | --- |
| `marshland_chest_01` | `[13, 16]` |
| `marshland_chest_02` | `[34, 27]` |
| `marshland_chest_03` | `[28, 50]` |

## Layers and collision

The existing layer/group names, order, and IDs are preserved. Both semantic
layers remain hidden; `collision` uses the single red tile from `collision.tsx`.

All 14 complete trees use `foreground`, including lower canopy pixels.
TSX `sort_base_y` and the map's per-cell overrides place every part of a tree
at its root baseline. Only the two central root cells block movement; upper
canopies leave otherwise clear ground walkable behind them. Existing water
and boundary reasons remain solid beneath a canopy.

Mossy walls put their upper silhouettes in `foreground`, their lowest row in
`decoration`, and collision only at that bottom row. The shelter has a solid
lower footprint and an overhead roof. Fallen logs are low decoration with
solid bases. Reeds are soft vegetation. Every chest has collision beneath its
center tile. Water and boardwalk rails block movement; deck cells remain clear,
including the southern junction. See [map-layers.md](map-layers.md).

## Artwork and authoring

Production assets are `media/tilesets/marshland.png` and `marshland.tsx` under
the Rusted Kingdoms scenario. The atlas is 704×448, with 22 columns of 32×32
cells and binary alpha. Complete sprite groups have transparent gutters.

| Group | Atlas rectangle, in tiles |
| --- | --- |
| Willow | `0,0 6×7` |
| Cypress | `7,0 4×7` |
| Ruined wall | `12,0 9×4` |
| Shelter | `12,5 7×5` |
| Reeds | `0,9 2×2` |
| Fallen log | `3,9 5×2` |
| Turf / water / peat variants | Row 11, columns 0–11 |
| Peat transitions | Row 12, columns 1–15 |
| Water transitions | Row 13, columns 1–15 |

Built-in imagegen produced two source images: six isolated marsh objects and
three terrain textures. Prompts requested detailed pixel art, muted sage/olive
foliage, peat, cool still water, mossy stone, and an orthographic camera looking
north from due south at a 45-degree downward pitch. ImageMagick normalizes the
sprites, palettes, tile sizes, and alpha. Analytic four-corner masks create the
peat and water transitions without copying shared-atlas pixels. Existing
terrain and bridge art remains referenced from its original credited atlases.
New atlas provenance is recorded in ALI-1167/1168.

Sources, exact prompts, build scripts, layout records, audits, baseline copies,
and previews remain ignored in `temp/art/013-marshland/`. The exact prompt sets
are `prompts.json` and `terrain-prompt.json`. To reproduce local work:

```sh
python3 temp/art/013-marshland/build_tileset.py
python3 temp/art/013-marshland/build_map.py
python3 temp/art/013-marshland/audit_map.py
QT_QPA_PLATFORM=offscreen tmxrasterizer \
  assets/scenarios/rusted_kingdoms/media/maps/zone_03_marshland.tmx \
  temp/art/013-marshland/preview.png
```

The map builder uses the retained baseline to preserve original object and
layer identities. It deliberately rebuilds only Marshland; later manual map
edits must be incorporated into the builder before regenerating.

## Validation

The saved-map audit checks swept 20×18 player foot rectangles, all 2,034
connected walkable cells, both full NPC foot rectangles and interaction
approaches, all three reciprocal portals, safe arrivals, boardwalk decks and
rails, signs, chests, encounter markers, boss access, and consistent tree depth.
All affected TMX files pass the CSV validator and Tiled rasterization.

Portal regression tests pass 24/24; the production Marshland encounter/boss
handoff regression passes. The runtime map sweep passes 88/88. Scenario
validation matches its baseline with four accepted flag diagnostics and no new
ones. Asset hashes and the scoped diff checks pass. The broader asset-rights
audit retains 327 pre-existing unapproved third-party payload entries.
