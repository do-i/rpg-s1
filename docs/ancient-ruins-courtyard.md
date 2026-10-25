# Ancient Ruins Courtyard

`zone_04_ancient_ruins_02_courtyard` is a 68×52-tile outdoor ruin
(2176×1664 pixels), replacing the former 42×30 placeholder. It continues the
gate's abandoned medieval setting as a civic square ruined by siege and neglect.

A headless guardian stands above a cracked fountain at the center of a worn
flagstone square. Broken three-bay cloisters flank the crossing; northern
arcade and residential remnants continue beyond the map edge. The southern
quarter contains three collapsed market stalls, wrecked carts, supply debris,
and ruined homes. A flooded southwest garden holds a trapped cache on a small
island reached by a narrow stone crossing. Dead trees, broken fences and mossy
wall sections give the perimeter a visible edge.

The main east-west route leads around the fountain. Northern cloister lanes,
the southern market and the garden shore provide optional loops and guarded
cache approaches. Twenty encounter markers use the existing undead formations,
now with chase ranges of two to four tiles. Arrival areas remain clear of
encounter markers. The existing `veil_breaker` barrier-enemy rule is preserved;
no boss, quest flag or progression item requirement is added.

Six independent chest instances use the shared contents catalog. The original
`ruins_courtyard_chest_01` and `_02` keep their names, numeric object IDs and
reward definitions. Four additional instances reuse existing reward definitions;
two of the six caches are trapped. Every chest tile has collision and an open,
reachable interaction tile beside it.

The graph remains Gate ↔ Courtyard ↔ Sanctum. Gate arrivals enter the courtyard
at `[66,24]`; Sanctum arrivals enter at `[1,24]`. The two portal strips span
rows 22–26 at the east and west edges. Outgoing destinations remain Gate
`[1,29]` and Sanctum `[58,34]`, following the
[sanctum redesign](ancient-ruins-sanctum.md). The gate's artwork and collision
remain unchanged.

The map reuses the accepted building, aftermath-prop and palisade atlases and
adds a separate grouped civic-object atlas. Generated sources, full prompts,
reproducible export and placement/collision details are retained in
[the courtyard art sources](../temp/art/003-ancient-ruins-courtyard/README.md).

Existing layer IDs and order are retained: `portals`, `collision`, `spawn_tile`,
`ground`, `terrain`, `architecture`, `decoration`, `item_boxes`, `foreground`.
Collision and spawn layers remain hidden. There are no new map layers.
Upper silhouettes use foreground depth overrides following the lower-zone
Marshland convention. Collision follows object bases and water, leaving grass,
open ground, arch openings and space behind upper silhouettes walkable.
Small grass pockets outside the perimeter may be inaccessible; all arrivals,
encounters, optional routes and chest approaches connect to the playable area.
Higher-zone placeholder maps were not used as design references.
