# Ruinwatch town interiors

Ruinwatch keeps its authored exterior tiles and shop badges. The former northwest
Monastery Vaults building now opens the weapon shop, and the former item shop
next to it opens the armor shop. Both interiors follow Ardel's established
shop layouts and keep Ruinwatch's merchants, dialogue, prices and story gates.
The northeast bag-badged building houses Ansel; the southern
crystal-badged building houses Dunstan's Magic Core shop. The previously unused
red-roof building north of the monastery houses Sedge's dedicated apothecary,
marked with a mortar, pestle and herbs. The old combined-shop
map ID remains the item shop so existing references stay valid.

The middle-east house opens Monastery Vaults: a stone nave with an altar, sun
medallion, north-facing pews, candle stands, a western scriptorium and an eastern
font/archive wing. Jep wanders within the clear archive alcove and disappears
only after recruitment, as before. The small southeast hut opens a compact
storehouse with crates, barrels, sacks, hay, rope and timber. Both reuse existing
project tilesheets.

Coordinates below are zero-based tiles. Entrance coordinates identify the
exterior trigger; return coordinates are the player sprite's top-left anchor,
whose collision box extends into the next row and adjacent column.

| Building | Interior map | Size | Town entrance | Interior arrival | Town return |
| --- | --- | --- | --- | --- | --- |
| North apothecary | `town_03_ruinwatch_shop_apothecary` | 15×12 | `[45, 6]` | `[7, 9]` | `[45, 6]` |
| Northwest weapon shop | `town_03_ruinwatch_shop_weapon` | 15×12 | `[19.5, 6]` | `[7, 9]` | `[20, 6]` |
| North armor shop | `town_03_ruinwatch_shop_armor` | 15×12 | `[35.5, 6]` | `[7, 9]` | `[36, 6]` |
| Northeast item shop | `town_03_ruinwatch_shop` | 15×12 | `[54, 7]` | `[7, 9]` | `[54, 7]` |
| Southwest inn | `town_03_ruinwatch_inn` | 18×11 | `[8.5, 25]` | `[7, 5]` | `[9, 25]` |
| South Magic Core shop | `town_03_ruinwatch_shop_mc` | 15×12 | `[35, 26]` | `[7, 9]` | `[35, 26]` |
| Middle-east monastery | `town_03_ruinwatch_monastery_vaults` | 25×23 | `[45, 16]` | `[11, 20]` | `[45, 16]` |
| Southeast storage hut | `town_03_ruinwatch_storage` | 11×11 | `[55, 25]` | `[4, 8]` | `[55, 25]` |

Every interior has one reciprocal town exit. Ruinwatch has nine runtime
portals: these eight entrances and the existing Ancient Ruins gate. New maps
belong to the Ruinwatch atlas region. Collision layers remain hidden, and tall
furniture uses the established `foreground`/`decoration` split.

Validation includes Tiled rasterization, strict TMX CSV validation, the scenario
baseline and production map sweep. A separate geometry audit checks 20×18
character footprints, safe arrivals outside exit triggers, reachable exits and
merchant interactions, and Jep's complete wander area.

Shared shop badges use a diagonal sword for weapons, a supply bag for items, a
faceted cyan crystal for Magic Cores, and a mortar with herbs for apothecaries.
The armor badge remains a shield. See `art/ruinwatch-shop-badges/README.md` for
the generation prompt and atlas normalization. The user-authored inn layout is
preserved tile for tile; its collision mask is hidden and an empty required
`foreground` layer is restored.
