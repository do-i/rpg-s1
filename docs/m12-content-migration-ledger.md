# Milestone 12 Content Migration Ledger

Source: `../agentic-rpg` at
`08970359d6cb03586948625d29b0d3351dbbf785`

This ledger instantiates the wildcard task families from
`docs/rusted-kingdoms-port-plan.md`. A row is Complete only when its individual
acceptance contract has evidence; merely finding a file in the repository is
not completion. Content is processed in wave order and committed in small,
independently validated slices.

## Status vocabulary

| Status | Meaning |
| --- | --- |
| Inventory | The pinned source instance and target path are identified. |
| Ready | The source bytes/schema are present, but instance acceptance is not complete. |
| Complete | The instance-specific automated and required live checks pass. |
| Blocked | A concrete source/content decision is required before acceptance can pass. |

## W12.1 — Ardel and Starting Forest

Wave boundary: Ardel, house/shop/inn/shrine, Starting Forest, first boss, elder
reward, and the resulting `story_act2_started` save/load state.

### Maps, portals, and play checks

| Instance | Status | Evidence / remaining work |
| --- | --- | --- |
| C-MAPDATA-`town_01_ardel` | Complete | Pinned metadata audit and strict instance validation pass; the production replay exercised its NPC, service, sign, audio, and forest-link references. |
| C-MAPDATA-`town_01_ardel_house_01` | Complete | Pinned metadata audit and strict instance validation pass; the production elder document and reward/save fixture resolve its sole NPC reference. |
| C-MAPDATA-`town_01_ardel_shop_01` | Complete | Pinned metadata audit and strict instance validation pass; Gate 11 exercised every distinct service reference through production routing. |
| C-MAPDATA-`town_01_ardel_inn_01` | Ready | Content matches pinned source (ignoring final newline); live wave traversal remains. |
| C-MAPDATA-`town_01_ardel_shrine` | Ready | Pinned metadata audit passes; shrine dialogue traversal remains. |
| C-MAPDATA-`zone_01_starting_forest` | Ready | Pinned metadata audit passes; boss-to-boundary replay remains. |
| C-TMX-`town_01_ardel` | Ready | Exact pinned graph and deterministic Ardel screenshot oracle pass. |
| C-TMX-`town_01_ardel_house_01` | Ready | Exact pinned graph and production loader fixture pass. |
| C-TMX-`town_01_ardel_shop_01` | Ready | Exact pinned graph; Gate 11 live entry passed. |
| C-TMX-`town_01_ardel_inn_01` | Ready | Exact pinned graph; live wave traversal remains. |
| C-TMX-`town_01_ardel_shrine` | Ready | Exact pinned graph; live wave traversal remains. |
| C-TMX-`zone_01_starting_forest` | Ready | Intentional Rust delta hides the semantic `spawn_tile` layer; all other pinned bytes and the runtime projection audit pass. |
| C-PORTAL-`town_01_ardel` | Complete | All six authored exits parse; house, shop, inn, shrine, and both forest routes have loadable return portals (`world_transition` regression). |
| C-PORTAL-`town_01_ardel_house_01` | Complete | Its sole outgoing portal returns to Ardel at `[3, 4]`, and the Ardel entrance supplies the reverse link (`world_transition` regression). |
| C-PORTAL-`town_01_ardel_shop_01` | Complete | Its sole outgoing portal returns to Ardel at `[15, 4]`, and the Ardel entrance supplies the reverse link; Gate 11 also proved live entry. |
| C-PORTAL-`town_01_ardel_inn_01` | Complete | Its sole outgoing portal returns to Ardel at `[24, 4]`, and the Ardel entrance supplies the reverse link (`world_transition` regression). |
| C-PORTAL-`town_01_ardel_shrine` | Complete | Its sole outgoing portal returns to Ardel at `[3, 18]`, and the Ardel entrance supplies the reverse link (`world_transition` regression). |
| C-PORTAL-`zone_01_starting_forest` | Complete | All three exits parse: two distinct return coordinates lead to Ardel, Ardel has two matching forest entrances, and Open Plains provides the reverse W12.2 boundary link. |
| C-PLAY-`town_01_ardel` | Ready | Prior live world/service evidence exists; complete wave route remains. |
| C-PLAY-`town_01_ardel_house_01` | Inventory | Elder before/after-boss dialogue and reward. |
| C-PLAY-`town_01_ardel_shop_01` | Ready | Gate 11 service replay passed. |
| C-PLAY-`town_01_ardel_inn_01` | Inventory | Portal, inn cancel/pay/recovery, and return. |
| C-PLAY-`town_01_ardel_shrine` | Inventory | Portal, Keeper Joss branches, and return. |
| C-PLAY-`zone_01_starting_forest` | Ready | Regular encounter/reward replay passed; first-boss and boundary save remain. |

### Gameplay content instances

| Instance | Status | Evidence / remaining work |
| --- | --- | --- |
| C-CLASS-`hero` | Ready | Pinned class/progression audits and battle fixtures pass. |
| C-CLASS-`cleric` | Ready | Pinned class audit and Elise runtime join fixture pass; live recruitment remains. |
| C-ENEMY-`enemies_rank_8_F` | Ready | All W12.1 regular enemies and Grik load/fight. The target-authored Zone 1 material repair resolves all four dangling pinned drop ids; live boss victory remains. |
| C-ENCOUNTER-`zone_01_starting_forest` | Ready | Seeded formations, visible spawns, boss spawn, boss BGM, return context, and completion flag fixtures pass; live boss victory remains. |
| C-RECIPE-`all_recipe` | Ready | Gate 11 classification/craft fixture and pinned audit pass; wave service acceptance remains. |
| C-ITEM-`accessories` | Ready | Pinned item audit passes. |
| C-ITEM-`body` | Ready | Pinned item audit passes. |
| C-ITEM-`consumables_battle_throw` | Ready | Pinned item audit and battle usage fixtures pass. |
| C-ITEM-`consumables_field` | Ready | Pinned item audit and field usage fixtures pass. |
| C-ITEM-`consumables_recovery` | Ready | Pinned item audit; shop/loot/reward fixtures pass. |
| C-ITEM-`consumables_status_cure` | Ready | Pinned item audit and battle/field usage fixtures pass. |
| C-ITEM-`helmets` | Ready | Pinned item/equipment audits pass. |
| C-ITEM-`key_items` | Ready | Pinned item/repository audits pass. |
| C-ITEM-`magic_cores` | Ready | Pinned item audit and Gate 11 exchange fixtures pass. |
| C-ITEM-`materials` | Ready | Pinned item audit and recipe input fixtures pass. |
| C-ITEM-`migration_zone1_drops` | Complete | Target-authored compatibility catalog defines the pinned-but-missing `goblin_ear`, `goblin_fang`, `rusty_blade`, and `goblin_shield` as non-usable materials with bounded 10-30 GP sale values. Typed catalog and production addressability tests pass; the validator no longer reports any rank-F drop reference. |
| C-ITEM-`shields` | Ready | Pinned item/equipment audits pass. |
| C-ITEM-`weapons` | Ready | Pinned item/equipment audits pass. |

### Dialogue instances

| Instance | Status | Evidence / remaining work |
| --- | --- | --- |
| C-DIALOGUE-`elise_join` | Complete | Production traversal reaches all four terminals and verifies the offer emits the Elise join/flag action; the production GameState fixture proves source-initialized, idempotent recruitment. |
| C-DIALOGUE-`guide_ardel` | Complete | A production-document fixture selects and traverses the quest-started, post-boss, Act II, and sail-unlocked branches through their terminals. |
| C-DIALOGUE-`ardel_smith` | Complete | Production traversal reaches all four terminals and verifies the start flag plus one-time `sq_smith_done`/three-Potion reward action. |
| C-DIALOGUE-`ardel_apprentice` | Complete | Production traversal reaches the pre-errand, active relay, and post-relay terminals and verifies the active branch sets `sq_smith_relayed`. |
| C-DIALOGUE-`ardel_fisherman` | Complete | Production traversal reaches all four executable quest terminals and verifies the start flag plus one-time `sq_stream_done`/two-Lure-Charm reward. An exhaustive flag fixture proves the two trailing pinned flavor entries are dead under Python-compatible first-match ordering and retains their bytes as accounted source content. |
| C-DIALOGUE-`ardel_child` | Complete | Production traversal selects and completes both the default and Elise-joined terminals without effects. |
| C-DIALOGUE-`elder_intro` | Complete | Production document selects the post-Grik reward branch, sets `npc_elder_reward_given` and `story_act2_started`, grants two Hi-Potions and one Tent exactly once, selects the post-reward branch on repeat, and round-trips the boundary through native save encoding. |
| C-DIALOGUE-`mc_shop_intro` | Complete | Production traversal proves there is no pre-story match, then reaches the story-gated terminal and emits the Magic Core shop action; Gate 11 routed that action live. |
| C-DIALOGUE-`item_shop_ardel` | Complete | Production traversal reaches its sole terminal and emits the Item shop action; Gate 11 proved the dialogue-to-service handoff live. |
| C-DIALOGUE-`apothecary_ardel` | Complete | Production traversal reaches the locked and story-available terminals, with only the latter emitting the apothecary action; Gate 11 proved that handoff live. |
| C-DIALOGUE-`weapon_shop_ardel` | Complete | Production traversal reaches its sole terminal and emits the Weapon shop action; the distinct service route has its own runtime fixture. |
| C-DIALOGUE-`armor_shop_ardel` | Complete | Production traversal reaches its sole terminal and emits the Armor shop action; Gate 11 proved its distinct live service route. |
| C-DIALOGUE-`inn_ardel` | Complete | Production traversal reaches its sole terminal and emits the inn action; the service route and cancel/pay/recovery behavior have dedicated Gate 11 fixtures. |
| C-DIALOGUE-`ardel_shrine_keeper` | Complete | Production traversal reaches the default, stream relay, post-relay, Act II, and endgame terminals and verifies only the active relay sets `sq_stream_relayed`. |
| C-DIALOGUE-`bridge_guard_zone5` | Complete | Production traversal proves no match before Act II or after the Zone 4 boss, and reaches the sole blocking terminal only inside its authored gate window. |
| C-DIALOGUE-`stronghold_gate_guard` | Complete | Production traversal proves no match before Act IV or after the Zone 9 boss, and reaches the sole blocking terminal only inside its authored gate window. |
| C-DIALOGUE-`sign_town_01_ardel` | Complete | Production traversal reaches its three-line terminal without effects; prior runtime sign interaction replay also passed. |
| C-DIALOGUE-`sign_zone_01_starting_forest` | Complete | Production traversal reaches its three-line terminal without effects through the same dialogue-session path used by world signs. |

Audio instances used at this boundary are `town.default`,
`zone.starting_forest`, `battle.normal`, `battle.boss`, and their referenced UI,
world, and battle SFX. Their indexes and materialized assets pass the pinned
audit; live transition continuity remains part of the wave exit check.

Individual image, sprite, tileset, font, and audio-file C-ASSET instances are
already identified with source path, destination, license, and hash in
`docs/asset-license-inventory.md`. This ledger links to that evidence rather
than duplicating hundreds of asset rows.

## Pinned-source differences affecting W12.1

- `zone_01_starting_forest.tmx` differs only by marking `spawn_tile` invisible
  and adding a final newline. Python treats this layer as semantic spawn data;
  the Rust renderer must hide it. A dedicated projection test covers the
  accepted rendering delta.
- `town_01_ardel_inn_01.yaml` differs only by a trailing blank line.
- The source consumes `transport_warp_unlocked` without producing it. This is
  outside the W12.1 walkable boundary because warp remains locked, but must be
  resolved or recorded before the relevant transport wave closes.
- The pinned rank-F enemy file references four item ids that do not exist in
  any pinned item catalog. The target keeps the source enemy probabilities and
  supplies project-authored material metadata in
  `data/items/migration_zone1_drops.yaml`. No equipment or use behavior is
  inferred. This repair reduces strict target validation from 37 errors to 14
  while increasing the runtime item catalog from 172 to 176 entries.
- `ardel_fisherman.yaml` ends with two flavor entries that are unreachable for
  every combination of their five relevant flags: the four preceding quest
  conditions partition all states under the pinned Python engine's first-match
  rule. The target retains the source bytes and has an exhaustive 32-state
  regression so the dead content cannot be mistaken for a Rust traversal bug.
