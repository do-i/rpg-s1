# Open Plains caves

The cave chain remains Open Plains ↔ cave 02 ↔ cave 01. Cave 01 is 30×23
and cave 02 is 60×30, using 32×32 tiles. Existing ground, bridge layouts,
foreground tiles and portal destinations are preserved.

Both maps have hidden `collision`, `spawn_tile` and `item_boxes` tile layers.
Toggle the latter two in Tiled to inspect spawn and treasure placements. The
`item_boxes` layer is an editor guide; runtime chest sprites and loot come
from each map's YAML `item_boxes`. Move both when relocating a chest so the
editor guide continues to match gameplay, and leave the guide hidden to avoid
drawing a second chest beneath the runtime sprite.

Cave 01 has six enemy markers and two chests, at (26,14) and (13,20).
Cave 02 retains its nine enemy markers and has three chests, at (16,7),
(39,10) and (51,24). Coordinates are zero-based. Each cave now has its own
encounter YAML with early Open Plains mouse thieves and knife bandits,
short chase ranges and a 28-second respawn interval. Loot provides potions,
antidotes, ether and small magic cores without adding a boss.

Mushrooms, scattered stones and a few pale mineral clusters mark alcoves and
room edges. They remain passable and leave bridge decks, signs and portal
landings clear. Four stale collision markers at (12,6), (13,6), (12,7) and
(13,7) are cleared across cave 01's central bridge landing, connecting the
southern treasure platform to the entrance.
