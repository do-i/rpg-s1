# Ashenveil town expansion

Ashenveil is 73×31 tiles (previously 41×31). The eastern market and residential
lanes reuse Ardel and Ruinwatch architecture, shop emblems, and furniture. The
brown terrain, western town, inn, Oracle Sanctum, story NPCs, and regional
connections retain Ashenveil's identity. A kitchen garden sits beside the new
homes, and the chief's double-door house has a larger roof and a 25×13 interior.
No new tilesheets are required.

The five merchants from the combined shop now have individual storefronts:
items, weapons, armor, Magic Cores, and an apothecary. Their identities,
dialogue, prices, and story unlock flags are preserved. The existing `_shop`
map ID remains the item shop; the apothecary shares its consumable stock.
Three furnished homes and the chief's house each have a resident and ambient
dialogue. New interiors are registered in the Ashen Road transport region.

Coordinates are zero-based tiles. Door coordinates mark the lower door artwork.
Each entrance portal is a six-pixel-high strip inside the bottom of the door:
four pixels inset horizontally, starting 24 pixels down the lower door tile,
and ending two pixels above the pavement. The chief's strip spans both lower
door tiles with the same outer insets. Portal contact includes rectangle edges,
so this gap lets characters pass sideways along the pavement without entering.
Walking north into the open doorway reaches the trigger. Arrival coordinates
center the player's runtime 20×18 foot rectangle in the target tile. Existing
town return coordinates remain one row below the lower door artwork, safely
outside the entrance trigger.

| Building | Interior suffix | Interior size | Town door | Interior arrival |
| --- | --- | --- | --- | --- |
| Oracle Sanctum | `_oracle_sanctum` | 18×11 | `[20,5]` | `[5,5]` |
| Inn | `_inn` | 18×11 | `[9,24]` | `[7,5]` |
| Item shop | `_shop` | 15×12 | `[36,5]` | `[7,9]` |
| Weapon shop | `_shop_weapon` | 15×12 | `[46,5]` | `[7,9]` |
| Armor shop | `_shop_armor` | 15×12 | `[56,5]` | `[7,9]` |
| Apothecary | `_shop_apothecary` | 15×12 | `[66,5]` | `[7,9]` |
| Magic Core shop | `_shop_mc` | 15×12 | `[35,25]` | `[7,9]` |
| Red-roof home | `_house_01` | 16×11 | `[46,16]` | `[6,6]` |
| Blue-roof home | `_house_02` | 10×9 | `[56,16]` | `[4,4]` |
| Copper-roof home | `_house_03` | 16×11 | `[47,25]` | `[6,6]` |
| Chief's house | `_chief_house` | 25×13 | `[61,25]`, `[62,25]` | `[9,7]` |

Collision follows the owner-corrected town layout: the topmost roof row has no
collision so characters can walk behind it. The remaining roof and wall cells
stay solid, except that every lower door tile is clear. Portal movement keeps
all authored collision cells intact; the collision layer is hidden for gameplay.

Each interior has a reciprocal exterior portal. Together with the Sunken Cave
and Mountain Pass links, the town has 13 portals. The southern regional trigger
occupies the last four pixels of the map so the existing Mountain Pass arrival
does not immediately overlap it. Regional target maps and destination coordinates
are unchanged. Inn and sanctum arrivals now use clear floor instead of narrow
exit corridors. The sanctum's former sword badge is replaced by a crystal.

Validation covers strict CSV/GID parsing, Tiled rendering of all twelve maps,
scenario references, and the production map sweep. A navigation audit uses
20×18 foot rectangles on a four-pixel grid to check connected arrivals,
reachable portals, clear NPC placement, merchant interaction range, and absence
of arrival/exit overlap. Collision remains hidden; roofs, walls, and props retain
the project's established layer separation.
