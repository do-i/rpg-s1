# Ancient Ruins Sanctum

`zone_04_ancient_ruins_03_sanctum` is a 60×54-tile roofless sanctuary
(1920×1728 pixels), replacing the 38×28 placeholder. Its weathered stone,
broken arcades and dead vegetation continue the accepted gate and courtyard.

A shattered altar and carved rear screen face a pale stone arena at the
north end of a long processional nave. The Skeleton Knight waits at `[30,21]`.
Paired broken arcades frame the nave and transept without closing their arch
openings. Western and southeastern burial chapels contain cracked stone
sarcophagi. A flooded southwest memorial garden provides a separate loop;
the eastern chapel shelters the Penitent. Northern wall fragments, southern
masonry and the inherited palisade enclose the abandoned sacred grounds.

The Penitent retains the same sprite, dialogue, blessing and post-boss lines.
The YAML sprite anchor moves to `[47,18]`, placing the feet at `[48,19]`.
Both collision-box columns are open, with a reachable interaction approach.
The boss remains once-only and sets `boss_zone04_defeated`. The existing
`veil_breaker` barrier rule, encounter formations, rewards and battle backdrop
are preserved. Sixteen regular encounter markers now use chase ranges of two
to four tiles. Arrivals, the Penitent and the immediate altar arena have clear
standing areas away from regular patrol starts.

Four independent caches use the existing shared contents catalog. Original
`ruins_sanctum_chest_01` and `_02` retain their names, numeric object IDs and
reward definitions; `_02` remains trapped. Two additional caches reuse the
courtyard and gate supply rewards. Every chest blocks its own tile and has a
reachable adjacent interaction tile.

The graph remains Courtyard ↔ Sanctum ↔ Mountain Foothills. The eastern portal
spans rows 32–36; Courtyard arrivals enter at `[58,34]`. The northern portal
spans columns 29–33; Mountain arrivals enter at `[31,2]`. Both trigger strips
are inside the map. Outgoing destinations remain Courtyard `[1,24]` and
Mountain Foothills `[12,56]`, following the
[Zone 05 redesign](mountain-foothills.md). The courtyard's artwork and collision
remain unchanged.

All existing layer names, IDs and order are retained. Collision and spawn
layers remain hidden. Upper silhouettes use `foreground` with per-cell base
depths; bottom rows use `decoration`. Solid bases, water and visible enclosure
sections carry collision. Open ground and space behind tall silhouettes stay
walkable. No new map layers or progression requirements are introduced.

The new altar and sarcophagus are grouped in a separate transparent atlas.
Generated sources, full prompts, export instructions and exact placements are
retained in [the sanctum art sources](../art/ancient-ruins-sanctum/README.md).
