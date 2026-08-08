# Rusted Kingdoms Python Engine Inventory

This inventory records the player-visible responsibilities in the pinned
Python implementation, not a proposed Rust module structure. It is the source
coverage record for M0.02; exact port behavior remains governed by the tasks
listed below.

## Source snapshot

| Field | Value |
| --- | --- |
| Source repository | `../agentic-rpg` |
| Pinned commit | `08970359d6cb03586948625d29b0d3351dbbf785` |
| Snapshot ledger | `docs/rusted-kingdoms-port-ledger.md` (2026-08-07 initial port snapshot) |
| Inventory scope | Every Python package under `../agentic-rpg/engine/`, including `common/scene` and `common/ui` |

## Package coverage

| Python package | Player-visible responsibility | Representative source and tests | Target milestone(s) / disposition |
| --- | --- | --- | --- |
| `engine` (root/core) | Starts the game, creates the resizable Pygame window, drives input/update/render, wires services and scenario dependencies. | `engine/main.py`, `engine/game.py`, `engine/app_module.py`; coverage is distributed through `tests/unit/`. | M1 (app shell, input, canvas, errors), M13 (CLI/profiling); the Pygame/Injector implementation itself is deliberately replaced by Bevy resources and schedules. |
| `engine/audio` | Resolves logical BGM/SFX names, loops/stops music, and plays menu/world/battle feedback. | `engine/audio/bgm_manager.py`, `engine/audio/sfx_manager.py`; `tests/unit/audio/test_bgm_manager.py`, `tests/unit/audio/test_sfx_manager.py`. | M2.23, M4.24-M4.25, M5.26, M8.10, M9-M10, M14.05. |
| `engine/battle` | Defines combatants and battle phases; resolves actions, targeting, hit/damage/status effects, rewards, battle UI/FX, post-battle, and game over. | `engine/battle/battle_state.py`, `engine/battle/action_resolver.py`, `engine/battle/battle_scene.py`, `engine/battle/battle_rewards.py`; `tests/unit/battle/test_battle_logic.py`, `tests/unit/battle/test_battle_rewards.py`, `tests/unit/battle/test_battle_state.py`. | M8.09-M8.12, M9, M10. |
| `engine/common` | Owns shared runtime state (flags, map, boxes, save slots), reusable menu/selection widgets, typography, and UI helpers. | `engine/common/game_state.py`, `engine/common/flag_state.py`, `engine/common/menu_popup.py`, `engine/common/scroll_list.py`; `tests/unit/common/test_flag_state.py`, `tests/unit/common/test_opened_boxes_state.py`, `tests/unit/common/test_scroll_list.py`. | M1.08-M1.12, M3.01-M3.08, M5-M7, M11; reusable widgets are re-expressed in Bevy UI rather than copied. |
| `engine/common/scene` | Provides the Python scene lifecycle, registry, and switching facade. | `engine/common/scene/scene.py`, `engine/common/scene/scene_manager.py`, `engine/common/scene/scene_registry.py`; `tests/unit/common/scene/test_scene_manager.py`, `tests/unit/common/scene/test_scene_registry.py`. | M1.01-M1.05; replace with Bevy `AppState`, transition events, and enter/exit cleanup. |
| `engine/common/ui` | Supplies palette/theme assets, chrome drawing, an opaque framebuffer presentation path, and image caching. | `engine/common/ui/theme.py`, `engine/common/ui/chrome.py`, `engine/common/ui/framebuffer.py`, `engine/common/ui/image_cache.py`; `tests/unit/common/ui/test_ui_theme.py`. | M0.14-M0.17, M1.08, M1.13-M1.14, M3-M11 UI tasks; Pygame-surface caching/framebuffer mechanics are explicitly replaced by Bevy rendering. |
| `engine/debug` | Enables debug bootstrap, notably a full-party start used for development checks. | `engine/debug/debug_bootstrap.py`; behavior also exercises `tests/unit/party/test_party_state.py`. | M13.06-M13.08; no separate production gameplay port before those debug options exist. |
| `engine/dialogue` | Loads conditional dialogue, advances lines/choices, and yields flags, items, party joins, transitions, and service requests. | `engine/dialogue/dialogue_engine.py`, `engine/dialogue/dialogue_scene.py`; `tests/unit/dialogue/test_dialogue_engine.py`, `tests/unit/dialogue/test_dialogue_scene.py`. | M2.16, M3.14-M3.16, M5.12-M5.18, M11.01 and M11.15. |
| `engine/encounter` | Loads encounter zones, performs deterministic random selection, spawns visible enemies, and builds battle participants. | `engine/encounter/encounter_manager.py`, `engine/encounter/encounter_resolver.py`, `engine/encounter/enemy_spawner.py`; `tests/unit/encounter/test_encounter_manager.py`, `tests/unit/encounter/test_encounter_resolver.py`, `tests/unit/encounter/test_enemy_spawner.py`. | M2.17-M2.18, M8.01-M8.09, M8.11-M8.12. |
| `engine/equipment` | Checks class/slot compatibility, swaps equipped items atomically, calculates stat totals, and renders equipment interaction. | `engine/equipment/equipment_logic.py`, `engine/equipment/equip_scene.py`; `tests/unit/equipment/test_equipment_logic.py`, `tests/unit/equipment/test_starting_equipment.py`. | M3.02-M3.04, M6.15-M6.19, M11.07. |
| `engine/field_menu` | Pauses world play and routes Status, Spells, Items, Equipment, Save, and related field actions. | `engine/field_menu/field_menu_scene.py`; `tests/unit/field_menu/test_field_menu_scene.py`. | M6.01-M6.02, M6.24, M7.08-M7.09. |
| `engine/inn` | Presents inn confirmation and applies paid party recovery. | `engine/inn/inn_scene.py`; service behavior is covered through scenario/play checks. | M11.08-M11.09. |
| `engine/io` | Loads YAML/manifest data, validates required values, constructs game state, and manages saves. | `engine/io/manifest_loader.py`, `engine/io/yaml_loader.py`, `engine/io/game_state_loader.py`, `engine/io/save_manager.py`; `tests/unit/io/test_manifest_loader.py`, `tests/unit/io/test_game_state_loader.py`, `tests/unit/io/test_yaml_loader.py`. | M0.05-M0.08 and M0.12, M2.01-M2.28, M3.08-M3.10, M7.01-M7.16. |
| `engine/item` | Loads item definitions/catalogs and implements inventory tabs, loot/new state, use/discard behavior, item UI, and magic-core catalog state. | `engine/item/item_catalog.py`, `engine/item/item_logic.py`, `engine/item/item_effect_handler.py`, `engine/item/item_scene.py`; `tests/unit/item/test_item_catalog.py`, `tests/unit/item/test_item_logic.py`, `tests/unit/item/test_item_scene_modals.py`. | M2.14, M3.04, M5.23-M5.25, M6.05-M6.14, M10.16-M10.18, M11.03-M11.06. |
| `engine/party` | Loads party/class data; tracks members, stats, EXP/levels, and repository inventory/GP. | `engine/party/member_state.py`, `engine/party/party_state.py`, `engine/party/repository_state.py`; `tests/unit/party/test_member_state.py`, `tests/unit/party/test_party_state.py`, `tests/unit/party/test_repository_state.py`. | M2.12-M2.13, M3.02-M3.04, M6.03-M6.05, M9.02, M10.24-M10.29. |
| `engine/quest` | Loads quest definitions, derives state from flags, and renders the quest board. | `engine/quest/quest_catalog.py`, `engine/quest/quest_board_scene.py`; `tests/unit/quest/test_quest_catalog.py`. | M2.22, M3.07, M11.14-M11.20. |
| `engine/record` | Records/replays input frames with versioned session data for deterministic debugging. | `engine/record/record_format.py`, `engine/record/recorder.py`; `tests/unit/record/test_recorder.py`. | M13.02-M13.05; raw Pygame key-frame serialization is explicitly replaced by normalized actions. |
| `engine/scenes` | Registers concrete Python scenes and their dependency wiring. | `engine/scenes/scene_registrar.py`; registration is exercised by scene tests across `tests/unit/`. | M1.01-M1.05; replace with Bevy plugin/system registration, no direct scene-registry port. |
| `engine/settings` | Loads engine display, input-adjacent, audio, balance, and debug configuration. | `engine/settings/engine_config_data.py`, `engine/settings/balance_data.py`; `tests/unit/settings/test_engine_config_data.py`. | M1.08-M1.13, M2.19, M13.06-M13.09; obsolete Pygame-only settings are explicit compatibility decisions under M0.06. |
| `engine/shop` | Implements buy/sell, equipment previews, magic-core exchange, apothecary recipes, and service UI. | `engine/shop/item_shop_scene.py`, `engine/shop/apothecary_scene.py`, `engine/shop/magic_core_shop_scene.py`; `tests/unit/shop/test_item_shop_scene.py`, `tests/unit/shop/test_apothecary_scene.py`, `tests/unit/shop/test_magic_core_shop_scene.py`. | M2.21, M11.01-M11.13. |
| `engine/spell` | Filters learned/field-usable spells and supplies field casting UI/logic. | `engine/spell/spell_logic.py`, `engine/spell/field_cast_mixin.py`, `engine/spell/spell_scene.py`; `tests/unit/spell/test_spell_logic.py`, `tests/unit/spell/test_spell_scene.py`. | M2.13, M6.20-M6.24, M9.03, M10.05-M10.15. |
| `engine/status` | Renders member status and applies field spell/status views. | `engine/status/status_logic.py`, `engine/status/status_renderer.py`, `engine/status/status_scene.py`; `tests/unit/status/test_status_logic.py`, `tests/unit/status/test_status_scene.py`. | M6.03-M6.05 and M6.20-M6.22; it shares the field-spell rules with `engine/spell`. |
| `engine/title` | Boots into title, renders title/menu/name entry/load/save flows, and routes New Game/Load/Quit. | `engine/title/boot_scene.py`, `engine/title/title_scene.py`, `engine/title/name_entry_scene.py`, `engine/title/load_game_scene.py`; `tests/unit/title/test_save_modal_scene.py`. | M0.10-M0.18, M1.02-M1.05, M3.11-M3.13, M7.10-M7.11. |
| `engine/util` | Provides frame/playtime clocks, seeded randomness, and weighted selection helpers. | `engine/util/frame_clock.py`, `engine/util/playtime.py`, `engine/util/pseudo_random.py`, `engine/util/weighted_pick.py`; behavior is covered by dependent unit tests. | M1.11-M1.12, M8.02-M8.04, M9.06-M9.18, M10, M13.01-M13.05. |
| `engine/world` | Loads/renders tile maps, collision, player/NPCs/boxes/signs/portals, camera/fades/animation, world overlays, and world-to-battle transitions. | `engine/world/tile_map_factory.py`, `engine/world/world_map_scene.py`, `engine/world/world_map_logic.py`, `engine/world/player.py`, `engine/world/portal_loader.py`; `tests/unit/world/test_tile_map_factory.py`, `tests/unit/world/test_world_map_logic.py`, `tests/unit/world/test_world_map_scene.py`, `tests/unit/world/test_portal_loader.py`. | M2.15, M4, M5, M6.23-M6.24, M8.06-M8.12, M12, M13.10. |

## Completeness rule

The 25 rows above correspond one-for-one with every directory in the pinned
source tree that is a Python package (`engine/` plus each directory containing
`__init__.py`). Every row names at least one target milestone; therefore there
are no unclassified packages or unrecorded deferrals in this snapshot.
