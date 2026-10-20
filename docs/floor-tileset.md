# Floor tileset layout

`assets/scenarios/rusted_kingdoms/media/tilesets/floor.png` uses 32×32 tiles,
16 columns and 12 rows. IDs are zero-based Tiled local IDs; rows and columns
below are one-based. The 2026-10-20 reorganization preserves all 81 visible
tiles, their pixels, tile properties, and map/stamp rendering.

| Group | Current IDs | Layout |
| --- | --- | --- |
| Frequently used wood | 0, 1 | Former tiles 14 and 30, first two cells |
| Other wood textures | 2–12 | Rest of row 1 |
| Nave flagstones | 16, 17, 32, 33 | 2×2 block, rows 2–3, columns 1–2 |
| Aisle flagstones | 18, 19, 34, 35 | 2×2 block, rows 2–3, columns 3–4 |
| Sanctuary flagstones | 20, 21, 36, 37 | 2×2 block, rows 2–3, columns 5–6 |
| White hex-circle | 39–41, 55–57, 71–73 | 3×3 block starting at row 3, column 8 |
| Cyan hex-circle | 42–44, 58–60, 74–76 | 3×3 block starting at row 3, column 11 |
| Rope bridge | 45–47, 61–63, 77–79 | 3×3 block starting at row 3, column 14 |
| Bordered cave ground | 80–82, 96–98, 112–114 | 3×3 block, rows 6–8, columns 1–3 |
| Cave ground with stone edges | 84, 85, 100, 101 | 2×2 block, rows 6–7, columns 5–6 |
| Blue stepping stones | 88, 89, 104, 105 | 2×2 block, rows 6–7, columns 9–10 |
| Broken stone edges and rubble | 92, 93, 108–110 | Rows 6–7, columns 13–15 |
| Additional cave ground | 128–130 | Row 9, columns 1–3 |
| Tiles reserved for replacement review | 176–179 | Last row, columns 1–4 |

Wood order in row 1, expressed as **old IDs → new IDs**:
`14, 30, 16, 1, 17, 0, 2, 18, 19, 20, 21, 22, 23` → `0–12`.

## Tiles being considered for terrain replacement

These tiles were moved, with no terrain replacement performed. Their map uses
were remapped to the IDs below, preserving all tile transforms.

| Old ID | Current ID | Existing description | Maps and placement counts |
| --- | --- | --- | --- |
| 3 | 176 | dark grass | `zone_08_corrupted_forest`: ground 1,735 |
| 4 | 177 | cave floor | `zone_08_corrupted_forest`: ground 11, terrain 114 |
| 5 | 178 | grass alt | `zone_08_corrupted_forest`: ground 4, terrain 526 |
| 15 | 179 | town tile 152 | `zone_10_hearth_core`: ground 52; each of `zone_10_hearth_descent_01` through `zone_10_hearth_descent_06`: ground 56 |

There are no other map, template, tile-object, or stamp uses of these four
original IDs in this reorganization's usage audit.

`bridge_short.stamp` and `bridge_long.stamp` now use the relocated bridge tiles.
The 40 transparent floor-tile references formerly using ID 7 in
`zone_02_open_plains_cave_01` now use empty ID 191, keeping them transparent
while the first row is filled with wood textures. Map dimensions, portals,
collision layers and spawn data are otherwise unchanged.
