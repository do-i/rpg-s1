# Open Plains caves

The cave chain remains Open Plains ↔ cave 02 ↔ cave 01. Cave 01 is 30×23
and cave 02 is 60×30, using 32×32 tiles. Existing ground, bridge layouts,
foreground tiles and portal destinations are preserved.

Both maps have hidden `collision` and `spawn_tile` tile layers and a visible
`item_boxes` object layer before `foreground`. Chest tile objects store their
images, positions and unique instance names. Their `item_box_id` properties
refer to the shared [contents catalog](item-boxes.md), which has no coordinates.

Cave 01 has six enemy markers and four chests, including the two manually added
placements. Their fractional pixel positions are preserved. Three instances
share `plains_cave_01_chest_01` supplies; the other references
`plains_cave_01_chest_02`. Cave 02 retains its nine enemy markers and three
chests at (16,7), (39,10) and (51,24). Coordinates are zero-based. Each cave now has its own
encounter YAML with early Open Plains mouse thieves and knife bandits,
short chase ranges and a 28-second respawn interval. Loot provides potions,
antidotes, ether and small magic cores without adding a boss.

Mushrooms, scattered stones and a few pale mineral clusters mark alcoves and
room edges. They remain passable and leave bridge decks, signs and portal
landings clear. Four stale collision markers at (12,6), (13,6), (12,7) and
(13,7) are cleared across cave 01's central bridge landing, connecting the
southern treasure platform to the entrance.
