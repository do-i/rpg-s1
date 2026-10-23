# Mountain Foothills — Zone 05

The three Zone 05 maps form one continuous alpine climb. Snow lies in drifts
around exposed stone trails, granite ridges and dark pines. The generated
objects share an orthographic camera looking north from due south, pitched
down 45 degrees with zero yaw. The terrain and rope bridges reuse existing
tiles without changing their pixels or IDs.

| Map | Previous size | New size | Layout |
| --- | --- | --- | --- |
| `zone_05_mountain_foothills_01` | 60×30 | 76×58 / 2432×1856 px | Lower trailhead, Corin's refuge, winding meltwater and two crossings; woodland and ridge cache spurs |
| `zone_05_mountain_foothills_02` | 56×32 | 80×60 / 2560×1920 px | Windcut Shelf, broad glacial river, upper and lower crossings, northeast lookout refuge and sheltered lower cache loop |
| `zone_05_mountain_foothills_03` | 48×34 | 76×64 / 2432×2048 px | Ticker's Ascent, western bell refuge, switchback trails, meltwater tarn, abandoned tinker workbench and eastern boss plateau |

The first two maps each have two usable crossings. Closing either bridge
still leaves a route to the opposite bank; closing both separates the banks.
All arrivals, NPC approaches, encounters and caches connect to the playable
area. All open cells connect to the trail network. Snow and open ground remain
walkable. Ridges block only their bottom two tile rows, leaving their upper
silhouettes walkable underneath so players can pass behind the rock faces.
Water, bridge rails, ridge and boulder bases, building foundations, tree roots,
boards and chests carry collision. No blank-margin blockers or new map layers
are added.

The existing layer names, IDs, order and object groups are retained. Tall
upper rows use `foreground`; their per-cell depths follow each complete
object's foot. Bottom rows use `decoration`. Collision and patrol markers
remain hidden in Tiled. Each map's atlas references retain the old GIDs and
add `mountain_foothills.tsx` at firstgid 2388.

## Connections and encounters

The graph remains Sanctum ↔ Foothills ↔ Windcut Shelf ↔ Ticker's Ascent ↔
Frostholm. Sanctum and Frostholm change only their incoming Zone 05 positions;
their artwork, object geometry and collision are preserved.

| Source portal | Destination arrival |
| --- | --- |
| Sanctum north exit | Foothills `[12,56]` |
| Foothills south exit | Sanctum `[31,2]` |
| Foothills east exit | Windcut Shelf `[1,43]` |
| Windcut Shelf west exit | Foothills `[74,28]` |
| Windcut Shelf east exit | Ticker's Ascent `[1,47]` |
| Ticker's Ascent west exit | Windcut Shelf `[78,19]` |
| Ticker's Ascent east exit | Frostholm `[19,29]` |
| Frostholm south exit | Ticker's Ascent `[74,15]` |

All Zone 05 portal rectangles lie inside their source maps. The trailhead
south trigger spans columns 10–14. Side triggers span five rows, centered on
the arrival trail's row. Actual 20×18 player/NPC collision boxes, rather than
only YAML anchors, are checked against the collision grid.

The maps have 16, 18 and 16 regular patrol markers respectively. Initial/max
enemy populations become 6/12, 6/12 and 5/10; existing spawn intervals remain
22, 22 and 24 seconds. Formations, weights, battle background and the
`veil_breaker` barrier rule are preserved, with chase ranges of two to four
tiles. Arrivals and NPC feet are separated from patrol starts. Titch the
Ticker waits at `[58,20]`; his once-only fight still sets
`boss_zone05_defeated`. The bell quest does not gate this encounter or travel.

## Caches

There are six independently persistent chests per map, eighteen total. The
six original names, numeric object IDs and reward definitions are preserved.
Twelve new instances have their own catalog definitions. Every chest has
collision beneath its center and a reachable adjacent interaction tile.

| Map | New chest suffix | Reward |
| --- | --- | --- |
| Foothills | `_03` | Steel Sword |
| Foothills | `_04` | Kite Shield |
| Foothills | `_05` | 2 Rare Coins |
| Foothills | `_06` | 2 Ether, Remedy |
| Windcut Shelf | `_03` | War Hammer |
| Windcut Shelf | `_04` | Shadow Buckler |
| Windcut Shelf | `_05` | Elixir; 30-damage trap |
| Windcut Shelf | `_06` | Large Magic Core |
| Ticker's Ascent | `_03` | War Mace |
| Ticker's Ascent | `_04` | Blessed Shield |
| Ticker's Ascent | `_05` | 3 Rare Coins, Elixir; 35-damage trap |
| Ticker's Ascent | `_06` | Silver Ring |

Equipment is ordinary inventory loot using existing item definitions; it
does not auto-equip or alter item balance. New caches do not require quest
flags. Traps use the existing nonlethal field-trap behavior.

## The Answering Bells

Corin at the lower refuge asks the party to check two bell keepers. Mara is
at the northeast refuge on Windcut Shelf; Iven is at the western refuge on
Ticker's Ascent. Both have visible-face, armed outdoor NPC sprites. Corin's
existing sprite and original snow warning are retained.

| NPC | YAML anchor | Feet tile |
| --- | --- | --- |
| Trailhand Corin | `[19,39]` | `[20,40]` |
| Lookout Mara | `[60,14]` | `[61,15]` |
| Keeper Iven | `[12,18]` | `[13,19]` |

The quest registry uses `sq_trail_bells_started` and `sq_trail_bells_done`.
Mara and Iven set `sq_trail_bells_shelf` and `sq_trail_bells_ascent`.
Reports can be collected in either order, including before speaking to
Corin. Corin introduces the quest on the first conversation, then rewards
both reports on a later conversation. The one-time reward is a Guardian
Pendant and a Rare Coin. Repeat conversations give flavor and directions.
The quest uses NPC interaction beside the bell props, with no new engine
interaction type or progression item. Disk-save regressions cover both visit
orders, early reports, unfinished saves and reopening completed dialogue.
Separate regressions open all six weapon/shield caches and verify that
reloading preserves both the loot and their opened state.

## Art sources

The new atlas contains complete grouped objects with binary alpha and clear
gutters. Generated sources, exact prompts, export scripts, placement records
and map regeneration instructions are in
[the foothills art sources](../art/mountain-foothills/README.md).
