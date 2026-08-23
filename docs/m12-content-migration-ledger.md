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
| C-MAPDATA-`town_01_ardel_inn_01` | Complete | Content matches pinned source apart from its recorded trailing-newline delta; strict instance validation and the production inn-service fixture pass. |
| C-MAPDATA-`town_01_ardel_shrine` | Complete | Pinned metadata audit and strict instance validation pass; all Keeper Joss branches reach terminals in the production traversal fixture. |
| C-MAPDATA-`zone_01_starting_forest` | Complete | Pinned metadata audit and strict instance validation pass; encounter, sign, audio, portal, and transport references all resolve. |
| C-TMX-`town_01_ardel` | Complete | Exact pinned graph, production loader, referenced-asset resolution, and deterministic Ardel screenshot oracle pass. |
| C-TMX-`town_01_ardel_house_01` | Complete | Exact pinned graph, production loader, and all referenced TSX/image path checks pass. |
| C-TMX-`town_01_ardel_shop_01` | Complete | Exact pinned graph, production loader, referenced-asset checks, and Gate 11 live entry pass. |
| C-TMX-`town_01_ardel_inn_01` | Complete | Exact pinned graph, production loader, and all referenced TSX/image path checks pass. |
| C-TMX-`town_01_ardel_shrine` | Complete | Exact pinned graph, production loader, and all referenced TSX/image path checks pass. |
| C-TMX-`zone_01_starting_forest` | Complete | The recorded Rust delta hides only the semantic `spawn_tile` layer; all other pinned bytes, referenced assets, production loading, and runtime projection audit pass. |
| C-PORTAL-`town_01_ardel` | Complete | All six authored exits parse; house, shop, inn, shrine, and both forest routes have loadable return portals (`world_transition` regression). |
| C-PORTAL-`town_01_ardel_house_01` | Complete | Its sole outgoing portal returns to Ardel at `[3, 4]`, and the Ardel entrance supplies the reverse link (`world_transition` regression). |
| C-PORTAL-`town_01_ardel_shop_01` | Complete | Its sole outgoing portal returns to Ardel at `[15, 4]`, and the Ardel entrance supplies the reverse link; Gate 11 also proved live entry. |
| C-PORTAL-`town_01_ardel_inn_01` | Complete | Its sole outgoing portal returns to Ardel at `[24, 4]`, and the Ardel entrance supplies the reverse link (`world_transition` regression). |
| C-PORTAL-`town_01_ardel_shrine` | Complete | Its sole outgoing portal returns to Ardel at `[3, 18]`, and the Ardel entrance supplies the reverse link (`world_transition` regression). |
| C-PORTAL-`zone_01_starting_forest` | Complete | All three exits parse: two distinct return coordinates lead to Ardel, Ardel has two matching forest entrances, and Open Plains provides the reverse W12.2 boundary link. |
| C-PLAY-`town_01_ardel` | Complete | Prior live world/service evidence plus owner manual live playthrough (2026-08-23) verified the wave route. |
| C-PLAY-`town_01_ardel_house_01` | Complete | Reopened row closed with save evidence: slot 003 (2026-08-23) holds `npc_elder_reward_given` and `story_act2_started`, proving the post-boss elder reward conversation was walked live. |
| C-PLAY-`town_01_ardel_shop_01` | Complete | Gate 11 service replay plus owner manual live playthrough (2026-08-23). |
| C-PLAY-`town_01_ardel_inn_01` | Complete | Owner manual live playthrough (2026-08-23) verified the wave route, including portal, inn cancel/pay/recovery, and return. |
| C-PLAY-`town_01_ardel_shrine` | Complete | Owner manual live playthrough (2026-08-23) verified the wave route, including portal, Keeper Joss branches, and return. |
| C-PLAY-`zone_01_starting_forest` | Complete | Regular encounter/reward replay plus owner manual live playthrough (2026-08-23) verified the wave route, including first boss and boundary save. |

### Gameplay content instances

| Instance | Status | Evidence / remaining work |
| --- | --- | --- |
| C-CLASS-`hero` | Complete | Pinned class/progression audits, source-initialized new-game construction, and production battle progression fixtures pass. |
| C-CLASS-`cleric` | Complete | Pinned class/progression audit, Elise join fixture, and live source-authored recruitment into the active battle party pass. |
| C-ENEMY-`enemies_rank_8_F` | Complete | All W12.1 regular enemies and Grik load/fight. The target-authored Zone 1 material repair resolves all four dangling pinned drop ids; owner manual live playthrough (2026-08-23) reported the boss fight victorious. |
| C-ENCOUNTER-`zone_01_starting_forest` | Complete | Seeded formations, visible spawns, boss spawn, boss BGM, return context, and completion flag fixtures pass; owner manual live playthrough (2026-08-23) reported the boss fight victorious. |
| C-RECIPE-`all_recipe` | Complete | Every input, output, and flag resolves in the pinned audit; Gate 11 classification and atomic craft fixtures pass against the production catalog. |
| C-ITEM-`accessories` | Complete | Pinned item audit and production catalog addressability pass for every accessory definition. |
| C-ITEM-`body` | Complete | Pinned item audit and production catalog addressability pass for every body-equipment definition. |
| C-ITEM-`consumables_battle_throw` | Complete | Pinned item audit, production catalog addressability, and battle-use fixtures pass for every throwable definition. |
| C-ITEM-`consumables_field` | Complete | Pinned item audit, production catalog addressability, and field-use fixtures pass for every field consumable. |
| C-ITEM-`consumables_recovery` | Complete | Pinned item audit, production catalog addressability, and shop, loot, reward, battle, and field recovery fixtures pass. |
| C-ITEM-`consumables_status_cure` | Complete | Pinned item audit, production catalog addressability, and battle/field status-cure fixtures pass. |
| C-ITEM-`helmets` | Complete | Pinned item/equipment audits, production catalog addressability, slot compatibility, and equipment fixtures pass. |
| C-ITEM-`key_items` | Complete | Pinned item/repository audits and production catalog addressability pass for every key item. |
| C-ITEM-`magic_cores` | Complete | Pinned item audit, production catalog addressability, and Gate 11 Magic Core exchange fixtures pass. |
| C-ITEM-`materials` | Complete | Pinned item audit, production catalog addressability, and recipe-input fixtures pass for every source material. |
| C-ITEM-`migration_zone1_drops` | Complete | Target-authored compatibility catalog defines the pinned-but-missing `goblin_ear`, `goblin_fang`, `rusty_blade`, and `goblin_shield` as non-usable materials with bounded 10-30 GP sale values. Typed catalog and production addressability tests pass; the validator no longer reports any rank-F drop reference. |
| C-ITEM-`shields` | Complete | Pinned item/equipment audits, production catalog addressability, slot compatibility, and equipment fixtures pass. |
| C-ITEM-`weapons` | Complete | Pinned item/equipment audits, production catalog addressability, class/slot compatibility, and equipment fixtures pass. |

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

## W12.2 — Open Plains, caves, and Millhaven

Wave boundary: Open Plains and its two caves, Millhaven with inn/mill/shop,
Reiya's recruitment (join dialogue `reiya_join` on `town_02_millhaven`, gated on
`story_act2_started`), and the W12.1↔W12.2 boundary portals.

### Maps, portals, and play checks

| Instance | Status | Evidence / remaining work |
| --- | --- | --- |
| C-MAPDATA-`zone_02_open_plains` | Complete | Byte-identical to pinned source; validate-scenario clean. The production `WorldActorPlugin`/`WorldObjectPlugin`/`WorldEncounterPlugin` fixture loads this metadata to `Spawned` and derives the correct enemy formations, boss, and (implicitly, via reaching `Spawned`) its NPC-free and item-box content. |
| C-MAPDATA-`town_02_millhaven` | Complete | Byte-identical to pinned source; validate-scenario clean. A `world_actor::present_npcs` fixture proves the exact production spawn set (all 6 authored NPCs) under fresh flags, `story_act2_started`, and `npc_reiya_joined`, including Reiya's `[12, 7]` position and her `excludes: [npc_reiya_joined]` presence gate. |
| C-MAPDATA-`town_02_millhaven_inn` | Ready | Byte-identical to pinned source; validate-scenario clean. |
| C-MAPDATA-`town_02_millhaven_mill` | Ready | Byte-identical to pinned source; validate-scenario clean. |
| C-MAPDATA-`town_02_millhaven_shop` | Ready | Byte-identical to pinned source; validate-scenario clean. |
| C-MAPDATA-`zone_02_open_plains_cave_01` | Complete | No YAML exists; matches pinned source (TMX-only cave definition). A dedicated production fixture proves `world_actor`/`world_object`/`world_encounter` all resolve the missing file as empty metadata (`MapMetadata::empty`, Python `load_yaml_optional` parity) and reach `Spawned`/`NoEncounters` instead of the `Failed` status a Rust port bug previously produced here — see the runtime-divergence note below. |
| C-MAPDATA-`zone_02_open_plains_cave_02` | Complete | No YAML exists; matches pinned source (TMX-only cave definition). Same production fixture as cave_01; also proves the painted `spawn_tile` layer does not force a metadata load even though 9 tiles are authored. |
| C-TMX-`zone_02_open_plains` | Complete | Byte-identical to pinned source; map-sweep clean. Production `world_transition::runtime_portals` resolves all four authored exits with correct target positions; the W12.2 encounter fixture also loads and renders this TMX end to end (`TmxGroundAssetPlugin`) with the correct spawn/boss tiles. |
| C-TMX-`zone_02_open_plains_cave_01` | Complete | Byte-identical to pinned source; map-sweep clean. Production portal extraction resolves its sole exit; the encounter fixture loads and renders this TMX end to end and confirms it has no `spawn_tile`/`boss_enemy` authoring at all. |
| C-TMX-`zone_02_open_plains_cave_02` | Complete | Byte-identical to pinned source; map-sweep clean. Production portal extraction resolves both exits; the encounter fixture loads and renders this TMX end to end and confirms its 9-tile `spawn_tile` layer without a matching `data/encount/*.yaml`. |
| C-TMX-`town_02_millhaven` | Complete | Byte-identical to pinned source; map-sweep clean. Production `world_transition::runtime_portals` resolves all four authored exits with correct target positions. |
| C-TMX-`town_02_millhaven_inn` | Complete | Byte-identical to pinned source; map-sweep clean. Production portal extraction resolves its sole return exit. |
| C-TMX-`town_02_millhaven_mill` | Complete | Byte-identical to pinned source; map-sweep clean. Production portal extraction resolves its sole return exit. |
| C-TMX-`town_02_millhaven_shop` | Complete | Byte-identical to pinned source; map-sweep clean. Production portal extraction resolves its sole return exit. |
| C-PORTAL-`zone_02_open_plains` | Complete | All four authored exits parse and round-trip in a `world_transition` regression: cave_02 entrance at `[8, 27]`, Starting Forest return at `[1, 26]` (reversible, shared with the W12.1 boundary fixture), Millhaven entry at `[19, 29]` (reversible), and the Marshland W12.3 boundary at `[27, 1]` (parses; no reverse-link claim, W12.3 is out of scope). |
| C-PORTAL-`zone_02_open_plains_cave_01` | Complete | A `world_transition` regression proves its sole outgoing portal leads to cave_02 at `[58, 19]` with a matching reverse link, and that cave_01 has no portal targeting `zone_02_open_plains` directly — the cave chain is plains ↔ cave_02 ↔ cave_01. |
| C-PORTAL-`zone_02_open_plains_cave_02` | Complete | A `world_transition` regression proves both outgoing portals (Open Plains at `[25, 3]`, cave_01 at `[1, 3]`) have matching reverse links. |
| C-PORTAL-`town_02_millhaven` | Complete | A `world_transition` regression proves all four authored exits (mill `[10, 11]`, shop `[7, 10]`, inn `[5, 9]`, Open Plains return `[2, 1]`) and that each interior/Open Plains link is reversible with the exact return position. |
| C-PORTAL-`town_02_millhaven_inn` | Complete | Its sole outgoing portal returns to Millhaven at `[9, 25]`; the Millhaven entrance supplies the reverse link (`world_transition` regression). |
| C-PORTAL-`town_02_millhaven_mill` | Complete | Its sole outgoing portal returns to Millhaven at `[20, 6]`; the Millhaven entrance supplies the reverse link (`world_transition` regression). |
| C-PORTAL-`town_02_millhaven_shop` | Complete | Its sole outgoing portal returns to Millhaven at `[36, 6]`; the Millhaven entrance supplies the reverse link (`world_transition` regression). |
| C-PLAY-`zone_02_open_plains` | Complete | Owner live playthrough, save-verified 2026-08-23 (slot 003): `boss_zone02_defeated` set, both plains chests opened, map visited; route to Millhaven and Marshland boundary walked. |
| C-PLAY-`zone_02_open_plains_cave_01` | Complete | Owner live playthrough, save-verified 2026-08-23 (slot 003): map in visited list — the cave chain was entered live, exercising the TMX-only soft-lock fix in production. |
| C-PLAY-`zone_02_open_plains_cave_02` | Complete | Owner live playthrough, save-verified 2026-08-23 (slot 003): map in visited list — entered live through the plains portal, exercising the TMX-only soft-lock fix in production. |
| C-PLAY-`town_02_millhaven` | Complete | Owner live playthrough, save-verified 2026-08-23 (slot 003): `npc_reiya_joined` set and Reiya present in the party roster; town visited and NPC recruitment dialogue completed live. |
| C-PLAY-`town_02_millhaven_inn` | Complete | Owner live playthrough, save-verified 2026-08-23 (slot 003): map in visited list; owner reports the game playable through the wave. |
| C-PLAY-`town_02_millhaven_mill` | Complete | Owner live playthrough, save-verified 2026-08-23 (slot 003): map in visited list; owner reports the game playable through the wave. |
| C-PLAY-`town_02_millhaven_shop` | Complete | Owner live playthrough, save-verified 2026-08-23 (slot 003): map in visited list; owner reports the game playable through the wave. |

### Dialogue instances

| Instance | Status | Evidence / remaining work |
| --- | --- | --- |
| C-DIALOGUE-`armor_shop_millhaven` | Complete | Production traversal reaches its sole terminal and emits the Armor shop action. |
| C-DIALOGUE-`inn_millhaven` | Complete | Production traversal reaches its sole terminal and emits the inn action. |
| C-DIALOGUE-`item_shop_millhaven` | Complete | Production traversal reaches its sole terminal and emits the Item shop action. |
| C-DIALOGUE-`millhaven_baker` | Complete | Production traversal reaches all four quest terminals (first meeting, active relay, reward, repeat) and verifies the start flag plus one-time `sq_flour_done`/one-Hi-Potion reward. |
| C-DIALOGUE-`millhaven_carter` | Complete | Production traversal reaches all four executable quest terminals and verifies the start flag plus one-time `sq_millstone_done`/one-Tent reward. An exhaustive 16-state flag fixture proves the two trailing pinned entries [4, 5] are dead under Python-compatible first-match ordering, mirroring `ardel_fisherman`; moved from new-finding to documented-accepted in `scenario_dialogue_report.rs`'s `DOCUMENTED_DEAD_ENTRIES` and pinned inventory test, consistent with `docs/adr/0007-inherited-scenario-data-debt.md`. |
| C-DIALOGUE-`millhaven_elder_hint` | Complete | Production traversal selects and completes the pre-story (none), Act II, Act II-with-boss (sets `story_act3_started`), and Act III branches, each reaching its terminal. |
| C-DIALOGUE-`millhaven_gossip` | Complete | Production traversal selects and completes both the `npc_reiya_joined` and default terminals without effects. |
| C-DIALOGUE-`millhaven_granary` | Complete | Production traversal reaches the before-errand, active, and after-relay terminals and verifies only the active branch sets `sq_flour_relayed`. |
| C-DIALOGUE-`millhaven_miller` | Complete | Production traversal reaches the Act III, Act II, and default terminals without effects. |
| C-DIALOGUE-`reiya_join` | Complete | Production traversal proves there is no pre-`story_quest_started` match, then reaches all four terminals and verifies the offer emits the Reiya join action and `npc_reiya_joined` flag exactly on the `story_act2_started`-gated branch, mirroring the Elise join fixture's idempotent source-initialized recruitment shape. |
| C-DIALOGUE-`sign_town_02_millhaven` | Complete | Production traversal reaches its three-line terminal without effects. |
| C-DIALOGUE-`sign_zone_02_open_plains` | Complete | Production traversal reaches its three-line terminal without effects. |
| C-DIALOGUE-`weapon_shop_millhaven` | Complete | Production traversal reaches its sole terminal and emits the Weapon shop action. |

Audio instances used at this boundary are `town.default` (Millhaven) and
`zone.open_plains` (Open Plains and caves, registered in
`data/audio/bgm_index.yaml` per the W12.2 BGM fix). Their indexes and
materialized assets pass the pinned audit; live transition continuity remains
part of the wave exit check.

Individual image, sprite, tileset, font, and audio-file C-ASSET instances are
already identified with source path, destination, license, and hash in
`docs/asset-license-inventory.md`. This ledger links to that evidence rather
than duplicating hundreds of asset rows.

## W12.3 — Marshland and Harborgate

Wave boundary: Marshland zone and encounters, Harborgate with harbormaster/inn/quarantine/shop, the sail unlock via `port_master_intro` (sets `transport_sail_unlocked`, gated on `story_act2_started`), and the W12.2↔W12.3 boundary portals.

### Maps, portals, and play checks

| Instance | Status | Evidence / remaining work |
| --- | --- | --- |
| C-MAPDATA-`zone_03_marshland` | Complete | Byte-identical to pinned source; validate-scenario clean. Map has no NPCs and authors no `bgm:` key at all (unlike every W12.1/W12.2 map); the production encounter fixture loads this metadata through `WorldEncounterPlugin`/`WorldAudioPlugin` to `Spawned` and its boss battle handoff confirms `return_context.world_bgm_key` resolves to `None` — see the runtime-divergence note below for the `world_audio` fix this exposed. Its 2 painted sign tiles route through `sign_zone_03_marshland`, which exists in neither repository (`docs/adr/0007-inherited-scenario-data-debt.md`); the production interaction fixture proves the port now matches the source's full silence — see the second runtime-divergence note below. |
| C-MAPDATA-`port_town_harborgate` | Complete | Byte-identical to pinned source; validate-scenario clean. A `world_actor::present_npcs` fixture proves the exact production 5-NPC spawn set (harborgate_dockhand, harborgate_clerk, harborgate_stevedore, harborgate_sailor, harborgate_fishwife — ids, positions, dialogue refs) is stable across fresh, Act II, and Act III flags: none of the five authors a `present:` gate. |
| C-MAPDATA-`port_town_harborgate_harbormaster` | Complete | Byte-identical to pinned source; validate-scenario clean. The production `port_master_intro` dialogue fixture (pre-Act-II silence, the sail-unlock offer, and post-unlock idempotence) resolves its sole NPC's dialogue reference. |
| C-MAPDATA-`port_town_harborgate_inn` | Complete | Byte-identical to pinned source; validate-scenario clean. The production `inn_harborgate` dialogue fixture resolves its sole NPC's dialogue reference and emits the inn service action. |
| C-MAPDATA-`port_town_harborgate_quarantine` | Complete | Byte-identical to pinned source; validate-scenario clean. A `world_actor::present_npcs` fixture proves the exact 2-NPC spawn set (quarantine_priestess, quarantine_patient — ids, positions, dialogue refs) is stable across fresh, Act II, and Act III flags: neither authors a `present:` gate. The `harborgate_priestess`/`harborgate_patient` dialogue fixtures resolve both NPCs' dialogue references. |
| C-MAPDATA-`port_town_harborgate_shop` | Complete | Byte-identical to pinned source; validate-scenario clean. A `world_actor::present_npcs` fixture proves the exact 4-NPC spawn set (item_shop_keeper, mc_shop_keeper, weapon_shop_keeper, armor_shop_keeper — ids, positions, dialogue refs) is stable across fresh, quest-started, and Act II/III flags: none authors a `present:` gate. The `item_shop_harborgate`/`weapon_shop_harborgate`/`armor_shop_harborgate` dialogue fixtures resolve three of the four NPCs' dialogue references; `mc_shop_keeper`'s shared `mc_shop_intro` reference was already resolved in W12.1. |
| C-TMX-`zone_03_marshland` | Complete | Byte-identical to pinned source; map-sweep clean. Production `world_transition::runtime_portals` resolves all three authored exits with correct target positions; the production encounter fixture also loads and renders this TMX end to end (`TmxGroundAssetPlugin`) with the correct 18-entry `spawn_tile` layer and `boss_enemy` tile. |
| C-TMX-`port_town_harborgate` | Complete | Byte-identical to pinned source; map-sweep clean. Production `world_transition::runtime_portals` resolves all five authored exits with correct target positions. |
| C-TMX-`port_town_harborgate_harbormaster` | Complete | Byte-identical to pinned source; map-sweep clean. Production portal extraction resolves its sole return exit. |
| C-TMX-`port_town_harborgate_inn` | Complete | Byte-identical to pinned source; map-sweep clean. Production portal extraction resolves its sole return exit. |
| C-TMX-`port_town_harborgate_quarantine` | Complete | Byte-identical to pinned source; map-sweep clean. Production portal extraction resolves its sole return exit. |
| C-TMX-`port_town_harborgate_shop` | Complete | Byte-identical to pinned source; map-sweep clean. Production portal extraction resolves its sole return exit. |
| C-ENCOUNTER-`zone_03_marshland` | Complete | Byte-identical to pinned source; validate-scenario clean. The production `WorldEncounterPlugin` fixture spawns all 18 authored `spawn_tile` formations plus the boss (`ratkin_plague_doctor_black_mask_doctor` at `[0, 31]`), round-trips through a neighboring encounter-free town (`port_town_harborgate`, no `data/encount/*.yaml`) and back, and drives one full boss contact -> `AppState::Battle` handoff: correct party stats, `background_id` `zone3-bg-1280x468`, `bgm_key` `battle.boss`, `boss_completion_flag` `boss_zone03_defeated`, and a `return_context` back to Marshland at the boss tile. |
| C-PORTAL-`zone_03_marshland` | Complete | All three authored exits parse and round-trip in a `world_transition` regression: Open Plains return at `[3, 18]` (reversible, shared with the W12.2 boundary fixture, exact plains-side arrival at `[27, 1]`), the Ancient Ruins W12.4 boundary at `[39, 14]` (parses; no reverse-link claim, W12.4 is out of scope), and Harborgate entry at `[4, 1]` (reversible). |
| C-PORTAL-`port_town_harborgate` | Complete | All five authored exits parse and round-trip: quarantine `[5, 9]`, shop `[7, 10]`, inn `[5, 9]`, harbormaster `[10, 11]`, Marshland return `[21, 37]` — a `world_transition` regression proves each interior/Marshland link is reversible with the exact return position. |
| C-PORTAL-`port_town_harborgate_harbormaster` | Complete | Its sole outgoing portal returns to Harborgate at `[35, 26]`; the Harborgate entrance supplies the reverse link (`world_transition` regression). |
| C-PORTAL-`port_town_harborgate_inn` | Complete | Its sole outgoing portal returns to Harborgate at `[9, 25]`; the Harborgate entrance supplies the reverse link (`world_transition` regression). |
| C-PORTAL-`port_town_harborgate_quarantine` | Complete | Its sole outgoing portal returns to Harborgate at `[20, 6]`; the Harborgate entrance supplies the reverse link (`world_transition` regression). |
| C-PORTAL-`port_town_harborgate_shop` | Complete | Its sole outgoing portal returns to Harborgate at `[36, 6]`; the Harborgate entrance supplies the reverse link (`world_transition` regression). |
| C-PLAY-`zone_03_marshland` | Complete | Owner live playthrough, save-verified 2026-08-23 (slot 002): map in visited list; traversed live after the BGM-continuity and sign-silence fixes. |
| C-PLAY-`port_town_harborgate` | Complete | Owner live playthrough, save-verified 2026-08-23 (slot 002): map in visited list; town route and interior portals walked live. |
| C-PLAY-`port_town_harborgate_harbormaster` | Complete | Owner live playthrough, save-verified 2026-08-23 (slot 002): `transport_sail_unlocked` is set, proving the port master recruitment dialogue completed live — the wave story payload. |
| C-PLAY-`port_town_harborgate_inn` | Inventory | The only W12.3 map absent from the 2026-08-23 slot-002 visited list; inn portal, pay/cancel/recovery, and return still need a live pass. |
| C-PLAY-`port_town_harborgate_quarantine` | Complete | Owner live playthrough, save-verified 2026-08-23 (slot 002): map in visited list; priestess and patient reachable. |
| C-PLAY-`port_town_harborgate_shop` | Complete | Owner live playthrough, save-verified 2026-08-23 (slot 002): map in visited list; shop interior entered live. |

### Dialogue instances

| Instance | Status | Evidence / remaining work |
| --- | --- | --- |
| C-DIALOGUE-`armor_shop_harborgate` | Complete | Production traversal reaches its sole terminal and emits the Armor shop action. |
| C-DIALOGUE-`harborgate_clerk` | Complete | Production traversal reaches all four quest terminals (first meeting, waiting on the stevedore, reward, repeat) and verifies the start flag plus the one-time `sq_manifest_done`/3-Antidote reward. |
| C-DIALOGUE-`harborgate_dockhand` | Complete | Production traversal selects and completes the Act III, Act II, and default terminals without effects. |
| C-DIALOGUE-`harborgate_fishwife` | Complete | Production traversal reaches its four executable quest terminals (first meeting, active relay, reward, repeat) and verifies the start flag plus the one-time `sq_catch_done`/2-Ice-Vial reward. Entry [4] is a dead flavor fallback, unreachable under Python-compatible first-match ordering; an exhaustive 8-state flag fixture proves it, mirroring `millhaven_carter`'s; moved from new-finding to documented-accepted in `scenario_dialogue_report.rs`'s `DOCUMENTED_DEAD_ENTRIES` and pinned inventory test, consistent with `docs/adr/0007-inherited-scenario-data-debt.md`. |
| C-DIALOGUE-`harborgate_patient` | Complete | Production traversal selects and completes both the Act III and default terminals without effects. |
| C-DIALOGUE-`harborgate_priestess` | Complete | Production traversal selects and completes the Act III, Act II, and default terminals without effects. |
| C-DIALOGUE-`harborgate_sailor` | Complete | Production traversal reaches the fishwife-relay (sets `sq_catch_relayed`), waiting, sail-unlocked, and default terminals, and confirms a completed-but-unsailed quest state falls through to the default flavor entry rather than sticking on a stale branch. |
| C-DIALOGUE-`harborgate_stevedore` | Complete | Production traversal reaches the before-errand, active (sets `sq_manifest_relayed`), and after-relay terminals without unexpected effects. |
| C-DIALOGUE-`inn_harborgate` | Complete | Production traversal reaches its sole terminal and emits the inn action. |
| C-DIALOGUE-`item_shop_harborgate` | Complete | Production traversal reaches its sole terminal and emits the Item shop action. |
| C-DIALOGUE-`port_master_intro` | Complete | This wave's story payload. Production traversal proves there is no match before `story_act2_started`, reaches the sail-unlock offer branch (sets `transport_sail_unlocked`) once Act II starts, reaches the post-unlock branch once the flag is applied (no repeated flag-set action — idempotent), and proves the post-unlock branch stays reachable on the flag alone, without Act II. |
| C-DIALOGUE-`sign_port_town_harborgate` | Complete | Production traversal reaches its three-line terminal without effects. |
| C-DIALOGUE-`weapon_shop_harborgate` | Complete | Production traversal reaches its sole terminal and emits the Weapon shop action. |

Audio instance used at Harborgate is `town.default`; its index and materialized asset pass the pinned audit. Marshland authors no `bgm:` key at all in `data/maps/zone_03_marshland.yaml` (byte-identical to source), so it names no audio instance of its own — a boundary crossing into Marshland leaves whatever World track was already playing running, matching the pinned engine's `if bgm_key: bgm_manager.play_key(bgm_key)` guard; see the runtime-divergence note below for the `world_audio` fix this exposed. Live transition continuity remains part of the wave exit check.

Individual image, sprite, tileset, font, and audio-file C-ASSET instances are already identified with source path, destination, license, and hash in `docs/asset-license-inventory.md`.

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

## Pinned-source differences affecting W12.2 (recorded 2026-08-22)

- The pinned source references BGM id `zone.open_plains` from
  `data/maps/zone_02_open_plains.yaml` but never defines it in
  `data/audio/bgm_index.yaml` (both files verified byte-identical to source
  before this change). The target adds
  `zone: open_plains: bgm/Sun_Kissed_Horizon.mp3`, mapping the id to a shipped
  track that no map otherwise references, as an intentional improvement over
  pinned behavior (the source would play Open Plains without its own theme).
  Strict target validation drops from 14 errors to 13; the BGM index grows
  from 12 to 13 keys. Inherited-debt triage for the remaining errors is
  recorded in `docs/adr/0007-inherited-scenario-data-debt.md`.

## Runtime divergence found and fixed during W12.2 acceptance (recorded 2026-08-23)

- **Port bug, not a source difference.** `zone_02_open_plains_cave_01` and
  `_cave_02` are TMX-only content (no `data/maps/*.yaml`), matching the pinned
  source. The pinned Python engine treats a missing per-map YAML as a valid
  runtime state — `load_yaml_optional(path) or {}`
  (`engine/world/world_map_init.py`) — and `EncounterManager.set_zone`
  likewise treats a missing `data/encount/*.yaml` as "encounters disabled"
  even when spawn tiles are painted (`# towns, inns — encounters disabled`).
  The Rust port's `world_actor::drive_world_actor_load`,
  `world_object::drive_world_object_load`, and
  `world_encounter::drive_active_encounter_assets` instead treated any
  `LoadState::Failed` on that optional file — including a plain "file not
  found" — as a fatal, permanent `Failed` status. Because
  `world_transition::drive_transition_loading`'s `Publishing` barrier
  requires `WorldActorState`/`WorldObjectState` to reach `Spawned`, this would
  have soft-locked any portal transition into either cave behind a black
  fade screen that never clears — a real player-facing regression, not a
  cosmetic difference.
  Fix: `scenario_map::MapMetadata::empty()` plus
  `scenario_map::optional_scenario_asset_is_missing` (which narrows on
  `AssetLoadError::AssetReaderError(AssetReaderError::NotFound(_))`) let all
  three systems fall back to empty metadata / disabled encounters exactly
  like the pinned engine, while any other load failure (a real I/O or parse
  error) still surfaces as fatal. Covered by
  `world_encounter::tests::production_open_plains_and_caves_spawn_or_gracefully_skip_encounters_without_metadata_soft_lock`,
  which drives the full `WorldActorPlugin`/`WorldObjectPlugin`/
  `WorldEncounterPlugin` stack into both caves and asserts `Spawned`/
  `NoEncounters` instead of `Failed`.

## Runtime divergences found and fixed during W12.3 acceptance (recorded 2026-08-23)

- **Port bug: World BGM stopped unconditionally on every map change, before
  it even knew the destination had a track.** `zone_03_marshland` is the
  first map in wave order with no `bgm:` field at all
  (`data/maps/zone_03_marshland.yaml` is two lines — `name` and
  `warp_order` — byte-identical to the pinned source). The pinned Python
  engine only calls `bgm_manager.play_key(bgm_key)` when the destination
  names a key at all (`if bgm_key: bgm_manager.play_key(bgm_key)`,
  `engine/world/world_map_init.py`); a silent map leaves whatever track was
  already playing running — `BgmManager` never receives a stop call. The
  Rust port's `world_audio::drive_world_audio` instead despawned the
  currently playing logical BGM player the instant any map change was
  detected, before the destination's metadata had even loaded, so walking
  from Open Plains into Marshland cut the music to silence every time — a
  real, easily reachable player-facing difference, not a cosmetic one (every
  W12.2↔W12.3 boundary crossing hits it). Fix: `drive_world_audio` now only
  *loads* the destination's metadata on a map change; it stops and replaces
  the current loop only once that metadata is loaded and confirmed to name a
  `bgm` key, and leaves an already-playing loop alone when the destination
  has none. The published `WorldBgmStatus` reflects reality: `Playing` when
  a previous map's track is still looping through a silent map, `Silent`
  only when nothing is genuinely playing. Covered by
  `world_audio::tests::a_map_with_no_authored_bgm_leaves_the_previous_track_playing`,
  which drives a two-map package (a keyed map into a `bgm`-less one) and
  proves the same logical player, entity, and asset path survive the
  crossing. The pre-existing "restart on every keyed-to-keyed transition,
  even when both maps share one `bgm` key" behavior (e.g. a town and its
  interiors) is unchanged and out of this fix's scope — it already shipped
  in W12.1/W12.2 and the ledger's "live transition continuity" note covers
  it.
- **Port bug: the sign interaction sound played before the async dialogue
  load could fail.** `zone_03_marshland` paints 2 sign tiles whose dialogue
  document, `sign_zone_03_marshland`, exists in neither repository — known,
  accepted inherited debt (`docs/adr/0007-inherited-scenario-data-debt.md`).
  The pinned Python engine's `DialogueEngine.resolve` returns `None` for a
  missing document, and `world_map_scene.py::_try_interact` only creates its
  dialogue overlay — the only place any sound is tied to the interaction —
  once `resolve` returns a match, so a missing sign is a fully silent no-op
  in the source. The Rust port's `request_npc_dialogue` queued the "confirm"
  interaction sound the instant a sign (or NPC) was selected as a target,
  synchronously, before the async YAML load even had a chance to fail — so a
  missing sign clicked and then showed nothing, audibly diverging from the
  silent original. Fix: the interaction sound is now queued in
  `resolve_dialogue_request`, only once a session actually opens
  (`Ok(Some(session))`), never eagerly at target-selection time. As a
  side effect this also silences the (already rare, previously also
  incorrect) case of a present NPC/sign whose dialogue loads but has no
  entry matching the current flags — matching the same Python code path.
  Covered by two production `WorldInteractionPlugin` fixtures:
  `world_interaction::tests::interacting_with_a_missing_sign_dialogue_stays_fully_silent`
  (Marshland's real missing `sign_zone_03_marshland`: no session, no
  interaction sound at all) and
  `world_interaction::tests::interacting_with_an_authored_sign_dialogue_opens_and_plays_its_sound`
  (Harborgate's real `sign_port_town_harborgate`: still opens and still
  plays exactly one `Dialogue` sound, just resolved once the load is
  confirmed rather than eagerly). ADR 0007's "Missing sign dialogues on
  Marshland and Mountain Pass segment 1" entry is updated to record this
  resolution — the port now matches the source's silence instead of
  clicking.
