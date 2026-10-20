# Corrupted Forest

`zone_08_corrupted_forest` is a 50×35 map of 32×32 tiles. Its 2026-10-20
redesign replaces the previous broad sand-colored clearing and legacy floor
textures with terrain from `terrain-v7.tsx`.

The ground combines dark grass and winding root trails using the atlas's
connected-grass Wang rules. Branches lead to the warden, herbalist, three
chests, enemy-spawn clearings and the western boss/exit area. Dead-grass
patches, dark rock escarpments and four purple poisoned pools break up the
forest floor. The original tree placements and sorting overrides remain.
Obsolete bright grass caps over the rock escarpments were cleared.

All 1,750 ground cells and every nonempty terrain-layer cell use the LPC
terrain atlas. `floor.tsx` is no longer referenced by this map. The three
former floor textures, IDs 176–178, remain unused in the floor sheet.

| Connection | Portal cell | Destination |
| --- | --- | --- |
| North | approximately 29,0 | `zone_07_sunken_cave`, cell 1,32 |
| West | approximately 0,26 | `zone_09_volcanic_region`, cell 2,18 |

NPC metadata, chest loot, spawn markers, map size and layer order are
preserved. Follow-up Tiled edits refine the painted trails and collision
layout, widen the western portal, and adjust the boss trigger. Poisoned water
has authored collision. A cardinal flood fill from the northern entrance
reaches both portals, the boss area, both NPCs, all three chests and all five
spawn-marker cells. Collision and spawn layers are hidden for runtime.

The authored layers remain `ground`, `terrain`, `architecture`, `decoration`,
`foreground`, hidden `collision` and hidden `spawn_tile`, with `portals` and
`boss_enemy` object groups. The existing warden dialogue about the rot and
standing water remains consistent with the new scenery.
