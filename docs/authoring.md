# Authoring guide

How to make content for a scenario package: maps, things placed on maps, sprites,
and battle effects. Paths are relative to `assets/scenarios/rusted_kingdoms/`.

- [Map layers](#map-layers)
- [Collision](#collision)
- [Placing art](#placing-art)
- [Buildings and doorways](#buildings-and-doorways)
- [Foreground and tree sorting](#foreground-and-tree-sorting)
- [Treasure boxes](#treasure-boxes)
- [Signs](#signs)
- [NPCs](#npcs)
- [Floor tileset](#floor-tileset)
- [Enemy battler sheets](#enemy-battler-sheets)
- [Haste, Slow, and Regen](#haste-slow-and-regen)
- [Testing edits with hot reload](#testing-edits-with-hot-reload)

## Map layers

All playable maps and the editor template use this order, **top to bottom in Tiled**.
TMX stores it in reverse. Optional layers and object groups may be absent.

| Layer | Rendering | Movement |
| --- | --- | --- |
| `foreground` | Overhead art; complete trees sort at their authored base | Does not block |
| `item_boxes` | Chest tile objects below overhead art | Paint collision beneath each chest's center tile |
| `decoration` | Below characters; candles, signs, low objects, and object bases | Does not block |
| `architecture` | Below decoration; walls, buildings, and fixed supporting surfaces | Does not block |
| `terrain` | Above ground; paths and terrain overlays, where needed | Does not block |
| `ground` | Base floor or landscape | Does not block |
| `spawn_tile` | Hidden encounter markers, where needed | Does not block |
| `collision` | Hidden (`visible="0"`); solid footprints only | Every nonzero tile blocks movement |
| `boss_enemy` | Optional object group for boss markers | Defines boss encounters |
| `portals` | Object group for transition triggers | Defines map connections |

Each name appears at most once per map. **New layer names require the owner's
explicit permission.** Discuss a layout that cannot fit these layers before adding one.

The runtime finds map transitions by the plural name `portals`. Objects and object
groups may use Tiled's `visible="0"` to hide their editor markers; this does not
disable portal triggers or other gameplay objects.

Editor-only map templates live in `media/map_templates`, outside the playable
`media/maps` directory. They keep placeholder portal properties for authors to
replace and are excluded from release payloads.

## Collision

Use the single 32×32 opaque red (`#FF0000`) tile from `collision.tsx` to paint blocked
cells. Select it from the `collision` tileset rather than assuming it has GID 1,
because global IDs vary by map. Show the layer in Tiled to edit it, and hide the art
layers above it or use layer isolation to see the mask clearly.

Keep `visible="0"` when saving. The game blocks on any nonzero tile ID, regardless of
color or visibility. The runtime still honors visible collision layers in older
external maps; move their artwork into visible layers before hiding `collision`.

## Placing art

- Put terrain artwork in `terrain` and its opaque backing in `ground`. Keep
  structural tiles in `architecture`, above terrain on every map.
- Keep wall tiles in `architecture` and candles or signs in `decoration`. Their
  transparent pixels reveal the wall beneath, so an inn sign can share a cell with
  its building wall.
- Fixed surfaces can support decoration in the same cell: a kitchen counter belongs
  in `architecture` when separate cookware occupies `decoration` above it.
  Freestanding low furniture and fallen logs stay in `decoration`.
- Continue interior floor tiles beneath separated furniture to avoid transparent gaps.

**Tall objects:** place the upper tiles in `foreground`, the bottom tiles in
`decoration`, and the solid footprint in `collision`. For a three-tile-tall column,
the upper two tiles can cover a character while their floor cells remain walkable.
The bottom tile is drawn below characters and its cell is blocked, so a character
standing below that base appears in front of it. Wall-backed furniture keeps the
wall's collision even where its upper artwork is in `foreground`.

## Buildings and doorways

Follow the `town_05_ashenveil` layout: leave the topmost roof row walkable so
characters can walk behind it, while the remaining roof and wall footprint stays
solid. Every lower door tile must be free of collision.

Place each building entrance portal in a thin strip inside the bottom of the lower
door tile, spanning both tiles for a double door. In all towns, inset the strip four
pixels from each side, start it 24 pixels down the tile, and make it six pixels tall.
Leave a gap above the pavement: portal contact includes edges, so extending the
trigger to the tile boundary would also catch sideways passersby. Verify sideways
passage and walking north into every entrance with the player's foot rectangle.

Regional map-edge transitions and interior exits keep their separately authored
positions.

## Foreground and tree sorting

`foreground` is the exact, case-sensitive name of the overhead layer. The engine draws
ordinary foreground tiles above players, NPCs, enemies, and world objects, wherever
the layer sits in the TMX. A hidden foreground layer is not rendered.

For trees, keep all four tiles in `foreground`, including the lower canopy pixels that
share tiles with the trunk. The TSX integer property `sort_base_y` sets the sorting
baseline in map pixels below each tile's top edge: `64` for the upper row and `32` for
the lower row. Both rows then sort at the same root position, so characters behind the
tree are covered and characters in front are drawn above it. Tiles without the property
stay fixed overhead art.

Taller trees use repeated crown rows, as in the Starting Forest and Corrupted Forest.
The layer's string property `sort_base_y_overrides` holds whitespace-separated
`column,row:pixel_offset` entries in zero-based map coordinates. For example,
`13,4:96 13,5:64 13,6:32` places three rows at one root baseline.

- An entry overrides the TSX baseline only at its occupied cell.
- Entries on empty cells are ignored, so erasing tiles does not prevent map loading.
  Duplicate or out-of-bounds coordinates and negative or malformed offsets are errors.
- Crowns sort at their actual root or bottom edge. Do not give upper tiles a one-row
  (`32`) baseline: a walkable upper crown must still cover a player behind it. A base
  drawn in `decoration` also counts when determining the baseline.

## Treasure boxes

Chests are tile objects in the `item_boxes` object layer, placed before `foreground`
so canopies and roofs can cover them. Each object has a **Name** unique within its map,
which identifies the physical chest in saves, and a string property `item_box_id`,
which selects a reusable contents definition. Objects on any map may share a definition
while opening independently.

```yaml
# data/item_boxes.yaml, selected by manifest refs.item_boxes
item_boxes:
  supplies:
    loot:
      items:
        - id: potion
          qty: 2
```

Definitions contain loot, optional traps, and optional `present` flag conditions.
Editing a shared definition affects every placement.

To add a chest in Tiled:

1. Select the `item_boxes` object layer.
2. Select the closed tile in the `item_box` tileset and press **T** (Insert Tile).
3. Place a 32×32 tile object, unscaled and unrotated, with a unique Name. The next tile
   after the closed image is its opened frame.
4. Add the string property `item_box_id` with a key from the contents catalog.
5. Paint a `collision` tile at the tile containing the image's center. Keep an adjacent
   standing tile open so the player can interact.

Tile objects use Tiled's bottom-left anchor, so a default object's center is at
`(x + offsetx + 16, y + offsety - 16)` in map pixels. That center tile is used for both
interaction and collision, and collision stays after opening.

Renaming an object makes it a new chest for save purposes, because names are the
opened-box keys in native and imported Python saves. Changing only `item_box_id` keeps
the opened state. Packages without `refs.item_boxes` use the older per-map YAML format.

## Signs

Signs are visible decoration; their tileset IDs identify interactions. Each board can
use `data/dialogue/sign_<map_id>_<x>_<y>.yaml`, where `x,y` are its zero-based tile
coordinates and the dialogue ID matches the filename. Boards without that file use
`sign_<map_id>.yaml`. Moving a board also means moving or rewriting its dialogue file.
Restart after adding files so the scenario inventory discovers them.

## NPCs

**Position.** `position` in map YAML anchors the top-left of a 64×64 sprite, not its
feet. Its 20×18 collision box starts 22 pixels right and 41 pixels down from that
anchor, so its center occupies tile `[x + 1, y + 1]`. To place an NPC on floor tile
`[x, y]`, author `[x - 1, y - 1]` and check all four box corners against the map. The
box spans two tile columns, so checking only its center can miss furniture.

**Appearance.** Every NPC needs a visible face, so no full-face helmets. Prefer a
sprite no other NPC on the map uses.

- Town NPCs should use unarmed civilian sprites, such as
  `village_female_person_apron_keeper` and `village_male_person_plain_dark_hair`.
- NPCs on `zone_*` maps outside towns must carry weapons and wear armor, including in
  caves, ruins, and the final story areas. Use the sword fighters, the axe fighter,
  `knight_01`, `plate_knight_mace_knight`, or `plate_knight_open_helmet`. The
  `female_sword_fighter_armored_katana` sheet has no visible weapon, so it does not
  qualify. Hearth Descent memories use these armed appearances too.

These sprites in `media/sprites/npc/` are reserved for their shop maps:

| Sprite | Role |
| --- | --- |
| `merchant_base` | Item shopkeeper |
| `merchant_wealthy_bowler` | Magic Core shopkeeper |
| `male_blond_01` | Weapon shopkeeper |
| `farmer_male_base` | Armor shopkeeper |
| `female_wiz_01` | Apothecary |
| `merchant_traveling_merchant` | Reserved merchant appearance |

## Floor tileset

`media/tilesets/floor.png` uses 32×32 tiles, 16 columns and 12 rows. IDs are zero-based
Tiled local IDs; rows and columns are one-based. ID 191 is empty and is used for
transparent floor cells.

| Group | IDs | Layout |
| --- | --- | --- |
| Frequently used wood | 0, 1 | First two cells of row 1 |
| Other wood textures | 2–12 | Rest of row 1 |
| Nave flagstones | 16, 17, 32, 33 | 2×2, rows 2–3, columns 1–2 |
| Aisle flagstones | 18, 19, 34, 35 | 2×2, rows 2–3, columns 3–4 |
| Sanctuary flagstones | 20, 21, 36, 37 | 2×2, rows 2–3, columns 5–6 |
| Sandy ground | 22 | Row 2, column 7 |
| White hex-circle | 39–41, 55–57, 71–73 | 3×3 from row 3, column 8 |
| Cyan hex-circle | 42–44, 58–60, 74–76 | 3×3 from row 3, column 11 |
| Rope bridge | 45–47, 61–63, 77–79 | 3×3 from row 3, column 14 |
| Vertical rope bridge | 93–95, 109–111, 125–127 | 3×3 from row 6, column 14 |
| Bordered cave ground | 80–82, 96–98, 112–114 | 3×3, rows 6–8, columns 1–3 |
| Rock object with rubble base | 83–85, 99–101, 115–117, 131–133 | 3×4, rows 6–9, columns 4–6 |
| Blue stepping stones | 88, 89, 104, 105 | 2×2, rows 6–7, columns 9–10 |
| Additional cave ground | 128–130 | Row 9, columns 1–3 |

Place the rock object and rope bridges as complete blocks, and author their collision
separately. Repeat the row `109 110 111` to extend the vertical bridge. The stamps
`stamps/bridge_short.stamp` and `stamps/bridge_long.stamp` place the horizontal bridge.

## Item icons and gendered body armor

Item icons live under `media/images/icons/items/`, named `{item id}.png`:

| Folder | Holds |
| --- | --- |
| `equipment/` | Weapons, shields, helmets, accessories, and co-ed body armor |
| `equipment/male/`, `equipment/female/` | Body armor only that gender can wear |
| `non_equipment/` | Everything else |

The folder decides the gender, so moving an icon is the whole change. `scripts/build.sh`
and `scripts/run.sh` then run `scripts/sync_item_icons.py --write`, which sets `gender:` in
the item's YAML, repoints its [ledger](asset-license-inventory.md) row, and stages the moved
icon. Do not edit `gender:` by hand; CI runs `--check` and fails if it disagrees with the
folder. Gender folders accept body armor only.

Armor already worn in an older save stays on until taken off. A party member may not start
in `party.yaml` wearing armor for another gender; `validate-scenario` reports that.

## Enemy battler sheets

Optional v2 sheets live at `media/sprites/enemies/v2/{id}-status.tsx` and
`{id}-attack.tsx`. An enemy without them draws from its v1 LPC sheet (9 columns, idle
tile 18). Each v2 tile is 256×256. Poses share one scale, set by the widest pose, and
feet sit 4 pixels above the bottom of the cell. Sheets are transparent, with no labels
or grid lines.

Work on sheets under `temp/`, which git ignores. Copy approved sheets into `v2/`, and give
every copied PNG and TSX a row in the [asset ledger](asset-license-inventory.md).

**Status sheet:** 1024×1024, 4 columns, 16 tiles. `index = row * 4 + col`.

| Index | Pose | Engine value |
| ---: | --- | --- |
| 0 | Ready | no status |
| 1 | Poison | `StatusEffect::Poison` |
| 2 | Sleep | `StatusEffect::Sleep` |
| 3 | Stun | `StatusEffect::Stun` |
| 4 | Silence | `StatusEffect::Silence` |
| 5 | Burn | `StatusEffect::Burn` |
| 6 | Freeze | `StatusEffect::Freeze` |
| 7 | Knockback | `StatusEffect::Knockback` |
| 8 | Taunt | `StatusEffect::Taunt` |
| 9 | Guard | `StatusEffect::DamageReduction` |
| 10 | Cover | `StatusEffect::RedirectDamage` |
| 11 | Haste | `StatusEffect::Haste` |
| 12 | Slow | `StatusEffect::Slow` |
| 13 | Regen | `StatusEffect::Regen` |
| 14 | KO | knocked out, not a `StatusEffect` |
| 15 | empty | unused |

`AttackModifier`, `DefenseModifier`, `MagicResistanceModifier`, and `HitChanceModifier`
stay on tile 0, because their badges already carry the sign.

When several statuses are active, the first match wins: KO if health is 0, then Freeze,
Sleep, Stun, Burn, Poison, Silence, Knockback, Cover, Taunt, Guard, Slow, Haste, Regen.

**Attack sheet:** 512×256, 2 columns, 1 row. `index = col`.

| Index | Pose | When it plays |
| ---: | --- | --- |
| 0 | Thrust: a one-hand downward chop | basic attack, `AttackKind::Thrust` |
| 1 | Lunge: a wide step, both hands on the sword, point toward the viewer's right | the lunge ability, `AttackKind::Spellcast` |

## Haste, Slow, and Regen

These timed effects work on living party members and enemies. They are battle-only,
cleared by knockout, and never copied to the party's persistent field statuses.

| Effect | Tuning | Timing |
| --- | --- | --- |
| Haste | 1.5 times turn-priority speed | Begins next round, lasts 3 full rounds |
| Slow | 0.5 times turn-priority speed | Begins next round, lasts 3 full rounds |
| Regen | 5% maximum HP per recipient turn | Heals at the end of the recipient's next 3 turns |

- Turn order is rebuilt between rounds. Every actor still gets one turn per round.
- Speed modifiers do not change the DEX used for accuracy, critical chance, or fleeing.
- Reapplying replaces strength and refreshes duration; it does not stack. Haste and Slow
  replace each other. Cure removes all three. Constructs are immune to Slow.
- Regen rounds down to whole HP with a minimum of 1, caps at maximum HP, and never
  revives. Damage-over-time resolves first, so a lethal tick prevents healing.

Tuning belongs on the individual skill or enemy move, not in `data/balance.yaml`.

```yaml
# Player skill. Haste uses type: buff; Slow uses type: debuff, target: single_enemy,
# and modifier: 0.50.
effect:
  target: single_ally
  stat: speed
  modifier: 1.50
  duration_turns: 3
```

```yaml
# Player skill, type: buff.
effect:
  target: single_ally
  regen_hp_pct: 0.05
  duration_turns: 3
```

```yaml
# Enemy move. Targets are relative to the caster: party means its allies and
# all_enemies means its opponents. An effect move deals no incidental attack damage.
- action: ability
  id: regen
  weight: 20
  effect: { target: self, regen_hp_pct: 0.05, duration_turns: 3 }
```

Speed multipliers must be positive and finite: below 1 is Slow, 1 or above is Haste.
Regen must be above 0 and at most 1. Durations must be positive integers. Invalid or
mixed mappings fail YAML loading.

## Testing edits with hot reload

Set `RPG_S1_HOT_RELOAD` to reload scenario files without rebuilding Rust:

```sh
RPG_S1_HOT_RELOAD=1 cargo run -- play rusted_kingdoms
```

The game polls the package for YAML, TMX, TSX, PNG/WebP, audio, and font changes every
0.5 seconds while the world is idle. Reload waits until overlays, dialogue, battle, and
map transitions have finished, so close menus to pick up an edit.

- **Applied at once:** maps, collision, sprites, NPCs, chests, and catalogs. A changed
  BGM restarts after the reload; a changed SFX applies the next time it plays.
- **Not applied:** session progress is kept, so party and bootstrap YAML only affect the
  next new game. Changing scenario identity needs a restart.
- **Invalid edits:** the replacement is rejected and the previous state is restored. The
  recovery screen offers `Esc` to resume, `R` to retry, and `T` to return to title. A
  further file edit also retries. Finish a group of related edits before resuming, since
  a partly saved group may be rejected.

Hot reload checks only what the runtime needs. Run `validate-scenario` for the full
cross-reference check.

## Battle spell deck

The fixed battle canvas keeps its 468px battlefield. Below it, the five party
cards and command row share the remaining height with a 190px group frame and
280px spell/item list. Spell groups follow Fire, Water, Wind, Earth, Holy, Heal,
Utility; healing, revival, status recovery, and regeneration go under Heal before
checking an element. Battle-ineligible abilities do not enable a group.

Choosing Spell resets to the first enabled group. Up/Down follows the staggered
groups and skips disabled ones; Right or Enter enters the spell list. Left returns
to groups, then Escape returns to commands. Commands also accept Left/Right.
The spell and item lists reuse the shared menu scroll arrows and scrollbar.

`L` cycles the borderless log overlay through left, right, and hidden without
losing history. `LOG_OVERLAY_OPACITY` in `battle/ui/deck.rs` calibrates its 60%
background opacity; `DIAMOND_OPACITY` calibrates the neutral 50% diamond fills.

For a session-only full-party UI review with all learned-class spells and Reiya
acting first, run `scripts/run.sh -- play --seed 23 --battle-fixture spell-groups`.
This fixture does not change saves or the authored ability catalog.
