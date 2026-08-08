# Rusted Kingdoms Port Plan

Status: proposed

Target: native Rust on Bevy 0.19

Planning source snapshot: `../agentic-rpg` at `0897035`

Target baseline: `rpg-s1` at `696951a`

## Goal

Port the Python/Pygame **Chronicles of the Lost Flame** game and its
`rusted_kingdoms` scenario to this Bevy application without attempting a
line-for-line rewrite. Preserve player-visible behavior, data-driven scenario
authoring, deterministic debugging, and saveable progression.

The current title-screen prototype is the accepted starting point. The port is
complete only when a new player can start, finish, save, reload, and replay the
campaign using the Rust binary, with the Python game retained only as a parity
oracle and content source.

## Model assignments

Every task below has one of these explicit assignments:

| Code | Model | Best fit |
| --- | --- | --- |
| `T` | `gpt-5.6-terra`, medium reasoning | Small, bounded implementation, fixtures, mechanical data work, and documentation |
| `S` | `gpt-5.6-sol`, high reasoning | Cross-system Rust/Bevy work, state transitions, visual behavior, and difficult parity bugs |
| `X` | `gpt-5.6-sol`, xhigh reasoning | Architecture decisions, security/save reviews, and milestone-wide parity audits |

Use the assigned model as the starting choice. Escalate `T` to `S` only after
the task exposes a cross-system ambiguity; do not expand the task itself.

## Execution contract

- Execute one checkbox at a time. A task should normally produce one small
  commit and take no more than about half a day.
- Read the named Python implementation and its focused tests before porting a
  behavior. Treat observed behavior and scenario data as stronger evidence
  than old prose.
- Keep scenario content as data. Do not hardcode Rusted Kingdoms story rules in
  reusable engine systems.
- Add a focused Rust test in the same task as each logic change. Visual tasks
  instead require a short manual play checklist or a deterministic screenshot.
- Run `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets -- -D
  warnings`, and `git diff --check` before completing a task that changes Rust.
- Use relative repository paths in code and documentation. Never commit a
  developer's absolute local path.
- Do not copy an asset until its license and attribution status is recorded.
- Stop at every playable gate. Fix gate failures before starting the next
  milestone.
- When the Python source moves beyond `0897035`, update the snapshot only in a
  dedicated task after reviewing the diff; never silently chase a moving
  target.

## Definition of parity

Parity means the same rules and reachable outcomes, not identical internal
architecture or pixel-for-pixel rendering. For each feature, capture:

1. source data accepted;
2. state changes caused by input;
3. visible and audible result;
4. persistence behavior;
5. deterministic or edge-case behavior covered by source tests.

## Milestone 0 — Freeze the target and harden the baseline

| ID | Task | Model | Done when |
| --- | --- | --- | --- |
| M0.01 | [x] Record the source and target commit hashes in a port ledger. | `T` | The ledger has hashes, date, and dirty-worktree state. |
| M0.02 | [x] Inventory Python engine features by package. | `T` | Every `engine/` package maps to a milestone or explicit deferral. |
| M0.03 | [x] Inventory scenario data by schema and file count. | `T` | Manifest, maps, dialogue, encounters, enemies, items, classes, recipes, quests, and audio are counted. |
| M0.04 | [x] Create the player-visible parity checklist. | `S` | Every README feature has a manual acceptance row. |
| M0.05 | [x] Decide whether Rust saves must read Python YAML saves. | `X` | An ADR chooses compatibility or a one-time converter. |
| M0.06 | [x] Decide whether runtime reads source YAML/TMX unchanged. | `X` | An ADR fixes the data-compatibility policy. |
| M0.07 | [x] Decide the Rust TMX loading approach. | `X` | An ADR records crate/custom-parser choice and unsupported features. |
| M0.08 | [x] Decide the in-repo scenario asset layout. | `S` | One canonical relative root is documented. |
| M0.09 | [x] Build an asset-license inventory template. | `T` | Each copied asset can record origin, author, license, and destination. |
| M0.10 | [x] Audit title artwork and title music redistribution status. | `S` | Both assets have evidence or are blocked from release. |
| M0.11 | [x] Audit menu SFX redistribution status. | `T` | Attribution and redistribution decision are recorded. |
| M0.12 | [x] Resolve the manifest cursor filename mismatch. | `T` | The chosen source file and compatibility action are recorded and tested by validation. |
| M0.13 | [x] Add a title-menu action unit test. | `T` | New Game, disabled Load, and Quit resolve to distinct actions. |
| M0.14 | [x] Add a headless Bevy app test harness. | `S` | A test can advance the app without opening a window. |
| M0.15 | [x] Add a title-screen spawn smoke test. | `T` | The expected camera, background, menu, status, and audio entities exist. |
| M0.16 | [x] Write the baseline manual play checklist. | `T` | Resize, keyboard, audio, disabled Load, New Game, and Quit are covered. |
| M0.17 | [x] Run and record the baseline manual play check. | `S` | The title screen is played through using a real window/input/audio path on the best available graphics adapter; software rendering is acceptable when the development machine has no GPU. |
| M0.17a | [x] Decide the graceful Quit audio/exit lifecycle. | `T` | A short decision fixes the completion signal, fallback timeout, and test seam. |
| M0.17b | [x] Implement and test graceful Quit. | `S` | Confirm audio starts, then exactly one exit is emitted after playback or a bounded failure fallback. |
| M0.17c | [x] Re-run the targeted Quit audio check. | `S` | Captured output contains the confirm event and the process still exits cleanly. |
| M0.18 | [x] Add CI for format, test, and Clippy. | `T` | A clean checkout runs all three checks. |

**Gate 0:** The existing title screen is reproducibly buildable, tested, and
legally understood. No wider asset copy starts before this gate passes.

Gate 0 passed on 2026-08-08. The development machine has no GPU, so M0.17
accepts its real X11 window/input and ALSA audio run on the best available
adapter, Vulkan llvmpipe. The checklist preserves the unsuccessful GL and
`vulkan-virtio` hardware-driver probes as environment evidence, not as a
release blocker. It also records the original Quit-audio defect, the M0.17b
repair, and the focused M0.17c runtime proof that the repaired build plays the
confirm sound before exiting cleanly.

## Milestone 1 — Application shell

| ID | Task | Model | Done when |
| --- | --- | --- | --- |
| M1.01 | [x] Define the top-level `AppState` variants. | `S` | Boot, Title, NameEntry, Dialogue, World, Battle, FieldMenu, PostBattle, and GameOver are represented. |
| M1.02 | [x] Move title startup into `OnEnter(AppState::Title)`. | `S` | Title entities appear only while Title is active. |
| M1.03 | [x] Add title-screen cleanup on state exit. | `T` | No title UI, sprite, or audio entity survives the transition. |
| M1.04 | [x] Add a single transition request event. | `S` | Systems request transitions without directly constructing the next scene. |
| M1.05 | [x] Route New Game to `NameEntry`. | `T` | Confirming New Game changes state once. |
| M1.06 | [x] Add a reusable Back/Confirm/Up/Down action map. | `S` | Title input uses actions rather than raw keys. |
| M1.07 | [x] Add action-map unit tests. | `T` | Keyboard mappings and simultaneous-key precedence are fixed. |
| M1.08 | [x] Add a UI theme resource. | `T` | Existing title colors and font sizes come from one resource. |
| M1.09 | [x] Add a scenario-root resource. | `T` | Runtime paths resolve under one configured relative root. |
| M1.10 | [x] Add contextual load-error reporting. | `S` | A bad asset/data path reports the scenario-relative path and cause. |
| M1.11 | [x] Add deterministic RNG as a resource. | `S` | A supplied seed repeats the same number sequence in tests. |
| M1.12 | [x] Add playtime tracking independent of pause/menu time. | `T` | Unit tests match the Python playtime rules. |
| M1.13 | [x] Add a fixed gameplay canvas policy. | `S` | Resize behavior has a documented scale/letterbox rule. |
| M1.14 | [x] Verify title rendering under the canvas policy. | `S` | The title remains correctly framed at baseline, smaller, and wider windows. |

**Gate 1:** Selecting New Game leaves a clean title state and reaches an empty
Name Entry state through tested application infrastructure.

Gate 1 passed on 2026-08-08. The headless title lifecycle test confirms that
one New Game confirmation reaches `AppState::NameEntry`, removes every
title-owned camera, sprite, UI, text, and audio entity, and remains in Name
Entry on later updates. The complete 55-test suite and strict Clippy pass, and
M1.14 records the title canvas at baseline, smaller, and wider real X11 window
sizes on the available llvmpipe renderer.

## Milestone 2 — Scenario data foundation

| ID | Task | Model | Done when |
| --- | --- | --- | --- |
| M2.01 | [x] Add Serde YAML support and one fixture. | `T` | A test deserializes a minimal YAML document. |
| M2.02 | [x] Define a validated scenario-relative path type. | `S` | Absolute paths and `..` escapes are rejected. |
| M2.03 | [x] Define the manifest identity/window schema. | `T` | Current id, name, version, and window title load. |
| M2.04 | [x] Define the manifest title/font/UI schema. | `T` | Current title, font, backdrop, and cursor fields load. |
| M2.05 | [x] Define the manifest service-sprite schema. | `T` | Apothecary, inn, item, weapon, armor, and item-box fields load. |
| M2.06 | [x] Define the manifest protagonist/start schema. | `T` | Protagonist, map, position, and intro fields load. |
| M2.07 | [x] Define the manifest flags/refs schema. | `T` | Bootstrap flags, managed flags, and all refs load. |
| M2.08 | [x] Add manifest required-field errors. | `T` | Missing fields identify their YAML location. |
| M2.09 | [x] Add manifest path-existence validation. | `T` | Every referenced file/directory produces a focused result. |
| M2.10 | [x] Define shared condition fields. | `S` | `requires` and `excludes` have one typed representation. |
| M2.11 | [x] Define shared position and direction types. | `T` | Coordinates and four/eight-way directions round-trip in fixtures. |
| M2.12 | [x] Define party-member data types. | `S` | All current `party.yaml` fields load without lossy values. |
| M2.13 | [x] Define class and ability data types. | `S` | One current class file and every ability variant load. |
| M2.14 | [x] Define item catalog data types. | `S` | One file for each current item category loads. |
| M2.15 | [x] Define map metadata data types. | `S` | Ardel's shops, NPCs, animation, inn, and BGM load. |
| M2.16 | [x] Define dialogue data types. | `S` | Intro plus one branching NPC conversation load. |
| M2.17 | [x] Define enemy data types. | `S` | One enemy of each current rule shape loads. |
| M2.18 | [x] Define encounter-zone data types. | `S` | One regular and one boss zone load. |
| M2.19 | [x] Define balance data types. | `T` | The complete balance file loads with defaults made explicit. |
| M2.20 | [x] Define battle-background data types. | `T` | Every background entry loads. |
| M2.21 | [x] Define recipe data types. | `T` | Regular and unique-output recipes load. |
| M2.22 | [ ] Define quest data types. | `S` | Every current quest objective/reward shape loads. |
| M2.23 | [ ] Define audio-index data types. | `T` | BGM and SFX indices load and resolve paths. |
| M2.24 | [ ] Add duplicate-ID detection per catalog. | `T` | A fixture with a duplicate reports both locations. |
| M2.25 | [ ] Add cross-reference validation. | `S` | Party, maps, dialogue, items, enemies, encounters, recipes, quests, flags, and assets are checked. |
| M2.26 | [ ] Add a `validate-scenario` binary subcommand. | `S` | Validation runs without launching Bevy. |
| M2.27 | [ ] Add validator parity fixtures from Python. | `S` | Representative accepted and rejected cases agree with `tools/validate.py`. |
| M2.28 | [ ] Add validation to the developer menu. | `T` | `lazymenu-cli` exposes one Rust scenario-validation action. |

**Gate 2:** The Rust validator accepts the pinned Rusted Kingdoms snapshot, or
every disagreement is listed as a deliberate compatibility decision.

## Milestone 3 — New-game state and intro

| ID | Task | Model | Done when |
| --- | --- | --- | --- |
| M3.01 | [ ] Define runtime flag state. | `T` | Set, unset, requires, and excludes behavior is unit tested. |
| M3.02 | [ ] Define runtime member state. | `S` | Mutable HP/MP/EXP/equipment is separate from immutable catalog data. |
| M3.03 | [ ] Define runtime party state. | `S` | Order, row, membership, and protagonist lookup are tested. |
| M3.04 | [ ] Define runtime repository state. | `S` | GP, item counts, caps, and empty-stack removal are tested. |
| M3.05 | [ ] Define runtime map state. | `T` | Current map, position, facing, and visited maps are represented. |
| M3.06 | [ ] Define opened-box state. | `T` | Box IDs can be recorded idempotently. |
| M3.07 | [ ] Define quest progress state. | `S` | Inactive, active, completed, and objective progress are represented. |
| M3.08 | [ ] Compose the root `GameState`. | `S` | All runtime state has one serializable owner. |
| M3.09 | [ ] Build new-game state from the manifest. | `S` | Start map/position, protagonist, and bootstrap flags match Python. |
| M3.10 | [ ] Add new-game invariant tests. | `T` | Caps, initial equipment, flags, and party size are asserted. |
| M3.11 | [ ] Render the name-entry prompt. | `T` | Default protagonist name and editing hint are visible. |
| M3.12 | [ ] Implement name-entry text editing. | `S` | Insert, delete, cancel, confirm, and length rules are tested. |
| M3.13 | [ ] Apply the confirmed protagonist name. | `T` | Only the protagonist runtime name changes. |
| M3.14 | [ ] Render linear cutscene dialogue. | `S` | Intro lines advance one at a time and fit the canvas. |
| M3.15 | [ ] Apply intro `on_complete` flags. | `T` | Completion changes the expected flag idempotently. |
| M3.16 | [ ] Apply intro `on_complete` transition data. | `S` | Completion requests Ardel and the specified position. |
| M3.17 | [ ] Add Back/Confirm behavior tests for name and intro states. | `T` | Edge inputs cannot duplicate or skip transitions. |

**Gate 3:** New Game accepts a name, plays the intro, and produces a tested
runtime state ready to enter Ardel.

## Milestone 4 — One rendered, walkable map

| ID | Task | Model | Done when |
| --- | --- | --- | --- |
| M4.01 | [ ] Parse the TMX map header. | `T` | Orientation, dimensions, tile size, and finite-map rule load. |
| M4.02 | [ ] Parse external TSX references. | `S` | `firstgid` and normalized source paths load. |
| M4.03 | [ ] Parse TSX image metadata. | `T` | Tile size, columns, count, and image dimensions load. |
| M4.04 | [ ] Parse CSV tile-layer data. | `T` | Row/column count and GIDs are validated. |
| M4.05 | [ ] Decode Tiled flip bits from a GID. | `S` | Horizontal, vertical, and diagonal flags have unit tests. |
| M4.06 | [ ] Resolve a global GID to tileset/local ID. | `S` | Boundary and empty-tile cases are tested. |
| M4.07 | [ ] Parse object groups and rectangles. | `T` | Portal objects retain id, name, bounds, and properties. |
| M4.08 | [ ] Parse typed Tiled properties. | `T` | String, integer, float, and boolean values load. |
| M4.09 | [ ] Parse TSX tile animation frames. | `S` | Aric's durations and tile IDs load unchanged. |
| M4.10 | [ ] Load tileset textures into atlases. | `S` | One external TSX renders its expected tile. |
| M4.11 | [ ] Render one static tile layer. | `S` | Ardel ground appears at the correct coordinates. |
| M4.12 | [ ] Render all visible Ardel layers in source order. | `S` | Ground, terrain, decoration, and top layers visually agree with Python. |
| M4.13 | [ ] Exclude the collision layer from visible rendering. | `T` | Collision tiles never draw. |
| M4.14 | [ ] Build collision occupancy from the collision layer. | `S` | Known blocked and open Ardel cells are tested. |
| M4.15 | [ ] Copy and register Aric's walk sprite assets. | `T` | License ledger and asset references are complete. |
| M4.16 | [ ] Slice Aric's sprite atlas from TSX metadata. | `S` | Four directions select the correct base frames. |
| M4.17 | [ ] Spawn Aric at the new-game position. | `T` | World position matches the scenario tile coordinate. |
| M4.18 | [ ] Implement one-tile cardinal movement. | `S` | Each action moves exactly one legal tile. |
| M4.19 | [ ] Reject movement into collision cells. | `S` | Movement tests cover all four directions. |
| M4.20 | [ ] Animate walking while movement is active. | `S` | Frame timing follows TSX durations and idles cleanly. |
| M4.21 | [ ] Add diagonal movement policy. | `S` | Eight-way input and corner collision match the Python behavior. |
| M4.22 | [ ] Add a clamped camera follow system. | `S` | Small-map and edge positions are tested. |
| M4.23 | [ ] Add layer/entity Y ordering. | `S` | Aric draws in front of and behind the correct map content. |
| M4.24 | [ ] Start Ardel BGM from map metadata. | `T` | Entering Ardel plays the indexed looping track once. |
| M4.25 | [ ] Stop title BGM before Ardel BGM starts. | `T` | Only one BGM player remains. |
| M4.26 | [ ] Add a deterministic Ardel screenshot check. | `S` | A fixed spawn frame can be compared across changes. |

**Gate 4:** A player can start a new game and walk around a correctly rendered
Ardel with collision, animation, camera behavior, and BGM.

## Milestone 5 — World interaction vertical slice

| ID | Task | Model | Done when |
| --- | --- | --- | --- |
| M5.01 | [ ] Convert TMX portal rectangles to runtime triggers. | `S` | Ardel's portal bounds and destinations match source data. |
| M5.02 | [ ] Detect player entry into a portal. | `T` | Entry emits one transition request, not one per frame. |
| M5.03 | [ ] Add fade-out before map unload. | `S` | Input is locked during the fade. |
| M5.04 | [ ] Despawn all map-scoped entities. | `S` | No entity or audio leak remains after a map transition. |
| M5.05 | [ ] Load the destination map and position. | `S` | One Ardel interior round trip succeeds. |
| M5.06 | [ ] Add fade-in after destination spawn. | `T` | Input unlocks only after the fade completes. |
| M5.07 | [ ] Record visited maps after successful entry. | `T` | Failed loads do not mark a map visited. |
| M5.08 | [ ] Spawn one static NPC from map YAML. | `S` | Name, position, facing, and sprite match Ardel data. |
| M5.09 | [ ] Apply NPC `present` conditions. | `T` | Requires/excludes combinations are tested. |
| M5.10 | [ ] Add NPC occupancy to movement collision. | `S` | Player and NPC cannot overlap. |
| M5.11 | [ ] Select the adjacent interactable in facing direction. | `S` | Only the nearest valid target receives interaction. |
| M5.12 | [ ] Render an NPC dialogue box. | `S` | Speaker, text, continuation marker, and backdrop match the UI policy. |
| M5.13 | [ ] Implement dialogue typewriter timing. | `T` | Confirm first completes text, then advances. |
| M5.14 | [ ] Implement linear dialogue progression. | `T` | First/last-line transitions are tested. |
| M5.15 | [ ] Implement dialogue choice navigation. | `S` | Disabled and conditional choices behave correctly. |
| M5.16 | [ ] Implement dialogue node jumps. | `S` | Branch and terminal nodes are tested. |
| M5.17 | [ ] Apply dialogue flag effects. | `T` | Set/unset behavior is idempotent. |
| M5.18 | [ ] Apply dialogue party-join effects. | `S` | Elise joins once with source-defined initial state. |
| M5.19 | [ ] Implement static/step NPC animation. | `T` | Timing and idle facing match metadata. |
| M5.20 | [ ] Implement bounded wander NPC movement. | `S` | Range, collision, and deterministic RNG are tested. |
| M5.21 | [ ] Detect configured sign tiles. | `S` | Known Ardel sign cells become interactable. |
| M5.22 | [ ] Route sign interaction to per-map dialogue. | `T` | The correct `sign_<map-id>` file opens. |
| M5.23 | [ ] Spawn one item box from map data. | `S` | Position, sprite, and stable box ID load. |
| M5.24 | [ ] Open a box and grant its item. | `S` | Inventory changes once and the opened state persists in memory. |
| M5.25 | [ ] Render the already-open box state. | `T` | Reinteraction cannot duplicate loot. |
| M5.26 | [ ] Add interaction SFX routing. | `T` | Confirm, blocked, dialogue, and box sounds resolve through the SFX index. |

**Gate 5:** The Ardel slice supports an interior portal, NPC conversation,
Elise recruitment, a readable sign, and a one-time treasure box.

## Milestone 6 — Field menus, party, items, equipment, and spells

| ID | Task | Model | Done when |
| --- | --- | --- | --- |
| M6.01 | [ ] Open and close the field-menu shell. | `S` | World input pauses and resumes without state loss. |
| M6.02 | [ ] Add field-menu command navigation. | `T` | Wrap, confirm, cancel, and disabled rows are tested. |
| M6.03 | [ ] Render the party summary panel. | `S` | Names, levels, HP/MP, row, and GP display. |
| M6.04 | [ ] Add the status member selector. | `T` | Cycling members handles a one- and five-member party. |
| M6.05 | [ ] Render base and derived stats. | `S` | Displayed values agree with Python fixtures. |
| M6.06 | [ ] Load the item catalog into a runtime resource. | `T` | All current item IDs are addressable. |
| M6.07 | [ ] Render inventory tabs. | `S` | All/New/Recovery/Status/Battle/Material/Core/Key filters work. |
| M6.08 | [ ] Add scrollable inventory rows. | `T` | Empty, short, and over-one-page lists behave correctly. |
| M6.09 | [ ] Add the session-only item visibility filter. | `S` | Hidden items stay out of every tab until restart. |
| M6.10 | [ ] Track the latest loot batch. | `T` | The New tab matches repository batch semantics. |
| M6.11 | [ ] Add discard quantity selection. | `T` | Bounds and whole-stack removal are tested. |
| M6.12 | [ ] Block discard for locked/key items. | `T` | Both rule paths show a reason and preserve inventory. |
| M6.13 | [ ] Implement one field healing item. | `S` | Targeting, consumption, cap, and invalid-target rules match Python. |
| M6.14 | [ ] Implement one field status-cure item. | `S` | Only supported status effects clear. |
| M6.15 | [ ] Load equipment slot compatibility. | `S` | Class restrictions resolve from current class/item data. |
| M6.16 | [ ] Render equipped items by slot. | `T` | Empty and populated slots display correctly. |
| M6.17 | [ ] Preview equipment stat deltas. | `S` | Every modified derived stat agrees with Python fixtures. |
| M6.18 | [ ] Equip an owned compatible item. | `S` | Repository and prior equipment update atomically. |
| M6.19 | [ ] Reject incompatible equipment. | `T` | State is unchanged and the reason is visible. |
| M6.20 | [ ] Load class abilities into a spell catalog. | `S` | Level- and flag-gated unlocks resolve. |
| M6.21 | [ ] Render field-usable spells. | `T` | Battle-only and locked abilities are excluded. |
| M6.22 | [ ] Cast one field healing spell. | `S` | Target, MP cost, and cap behavior match Python. |
| M6.23 | [ ] Add the teleport destination picker. | `S` | Only eligible visited maps appear. |
| M6.24 | [ ] Execute field teleport through the map transition path. | `S` | MP changes only after a valid destination is accepted. |

**Gate 6:** The Ardel slice has useful party, status, inventory, equipment, and
spell screens, all operating on the same runtime state.

## Milestone 7 — Save, load, and recovery

| ID | Task | Model | Done when |
| --- | --- | --- | --- |
| M7.01 | [ ] Define a versioned Rust save envelope. | `X` | Format version, scenario id/version, timestamp, and payload are explicit. |
| M7.02 | [ ] Serialize the root game state. | `S` | A new-game state round-trips without loss. |
| M7.03 | [ ] Add save-schema golden fixtures. | `T` | Accidental field/name changes fail tests. |
| M7.04 | [ ] Choose the platform save directory. | `S` | Linux path and override behavior are documented and tested. |
| M7.05 | [ ] Write saves through a temporary file. | `S` | Interrupted writes cannot replace a valid slot. |
| M7.06 | [ ] Atomically replace the destination slot. | `S` | Success leaves one valid destination and no stale temporary file. |
| M7.07 | [ ] Enumerate save slots. | `T` | Empty, valid, corrupt, and incompatible slots are distinguished. |
| M7.08 | [ ] Render the field save-slot picker. | `T` | Save metadata and empty slots display. |
| M7.09 | [ ] Add overwrite confirmation. | `T` | Cancel preserves the old file; confirm replaces it. |
| M7.10 | [ ] Enable Load Game only when a valid save exists. | `T` | Title color/navigation/action update from discovered slots. |
| M7.11 | [ ] Render the title load-slot picker. | `T` | Valid slots can be selected and corrupt slots explain failure. |
| M7.12 | [ ] Restore a saved world session. | `S` | Map, position, party, flags, inventory, quests, boxes, RNG, and playtime restore. |
| M7.13 | [ ] Add unknown-field forward tolerance. | `S` | A fixture with a harmless new field still loads. |
| M7.14 | [ ] Add explicit old-version rejection/migration routing. | `X` | Unsupported versions never partially load. |
| M7.15 | [ ] Implement the M0.05 Python-save decision. | `X` | Compatibility fixtures or converter tests cover one real Python save. |
| M7.16 | [ ] Add a corrupt-save recovery test. | `S` | A bad slot does not block other slots or crash the game. |

**Gate 7:** A player can save the Ardel slice, quit, restart, load from the
title screen, and continue with identical state.

## Milestone 8 — Encounters and enemy presence

| ID | Task | Model | Done when |
| --- | --- | --- | --- |
| M8.01 | [ ] Load encounter-zone rules for the current map. | `S` | Zone lookup and no-encounter maps are tested. |
| M8.02 | [ ] Implement deterministic weighted encounter selection. | `S` | Fixed seeds reproduce Python fixture selections. |
| M8.03 | [ ] Implement step/cadence encounter checks. | `S` | Standing still cannot trigger random encounters. |
| M8.04 | [ ] Apply party encounter modifiers. | `T` | Rogue and non-Rogue fixtures produce expected rates. |
| M8.05 | [ ] Load enemy definitions into a catalog. | `S` | Stats, actions, drops, sprite, and conditions are retained. |
| M8.06 | [ ] Spawn one visible world enemy. | `S` | Sprite, position, movement mode, and encounter id match data. |
| M8.07 | [ ] Add deterministic enemy wandering. | `S` | Movement stays in bounds and respects collision. |
| M8.08 | [ ] Detect player/enemy contact. | `T` | Contact emits exactly one battle request. |
| M8.09 | [ ] Build battle participants from an encounter. | `S` | Party and enemy combatants copy the correct initial stats. |
| M8.10 | [ ] Select battle background and BGM. | `T` | Map/encounter metadata resolves the expected assets. |
| M8.11 | [ ] Add the world-to-battle transition effect. | `S` | World input freezes and entities remain recoverable. |
| M8.12 | [ ] Persist the pre-battle return context. | `S` | Map, position, facing, and BGM can be restored. |

**Gate 8:** Walking in the first encounter zone deterministically enters a
battle with the expected enemies, background, and audio.

## Milestone 9 — Minimum complete battle loop

| ID | Task | Model | Done when |
| --- | --- | --- | --- |
| M9.01 | [ ] Define battle phases and phase transitions. | `X` | Start, command, target, resolve, advance, victory, defeat, and flee are explicit. |
| M9.02 | [ ] Render enemy sprites and party panels. | `S` | HP/MP, row, active member, and KO state display. |
| M9.03 | [ ] Build the Attack/Spell/Item/Run command menu. | `S` | Availability matches actor state. |
| M9.04 | [ ] Implement living-enemy target selection. | `T` | Single, wrap, cancel, and no-target cases are tested. |
| M9.05 | [ ] Implement living-party target selection. | `T` | Ally targeting and KO eligibility are tested. |
| M9.06 | [ ] Implement turn-order calculation. | `S` | Dexterity ties and deterministic RNG match fixtures. |
| M9.07 | [ ] Implement physical hit chance. | `S` | Boundary and seeded-roll cases match Python. |
| M9.08 | [ ] Implement physical damage. | `S` | Attack, defense, row, minimum, and cap rules match fixtures. |
| M9.09 | [ ] Apply HP damage and KO. | `T` | HP clamps at zero and KO actors cannot act. |
| M9.10 | [ ] Implement one basic enemy action. | `S` | Target selection and damage use the same resolver path. |
| M9.11 | [ ] Advance to the next living actor. | `S` | KO and removed actors are skipped. |
| M9.12 | [ ] Detect enemy-party defeat. | `T` | Victory triggers only after every enemy is defeated. |
| M9.13 | [ ] Detect player-party defeat. | `T` | Game Over triggers only after every member is KO. |
| M9.14 | [ ] Implement flee success calculation. | `S` | Rate, Rogue modifier, failure turn cost, and guaranteed restrictions match. |
| M9.15 | [ ] Restore the world after victory. | `S` | Return context and world BGM resume once. |
| M9.16 | [ ] Restore the world after successful flee. | `S` | Player receives the source-defined separation/safety behavior. |
| M9.17 | [ ] Add a Game Over screen. | `S` | Retry/load/title choices route correctly. |
| M9.18 | [ ] Add a deterministic battle transcript test. | `S` | A fixed seed and action list produce a stable outcome. |

**Gate 9:** A player can win, lose, or flee one complete basic battle and
return to the correct application state.

## Milestone 10 — Full combat and progression parity

Each task ports one rule family and its focused Python tests; never combine
families merely because they share a renderer.

| ID | Task | Model | Done when |
| --- | --- | --- | --- |
| M10.01 | [ ] Implement critical-hit calculation. | `S` | Seeded fixtures match Python. |
| M10.02 | [ ] Implement miss/critical battle feedback. | `T` | The correct message and animation event emit. |
| M10.03 | [ ] Implement front/back physical modifiers. | `S` | Attacker and defender row cases are tested. |
| M10.04 | [ ] Implement elemental damage affinity. | `S` | Weak, neutral, and resistant fixtures match. |
| M10.05 | [ ] Implement offensive spell resolution. | `S` | Cost, target shape, damage, and invalid casts match. |
| M10.06 | [ ] Implement healing spell resolution. | `S` | Living/KO target rules and HP caps match. |
| M10.07 | [ ] Implement buff application. | `S` | Modifier and duration are recorded. |
| M10.08 | [ ] Implement debuff application. | `S` | Resistance and duration behavior match. |
| M10.09 | [ ] Tick and expire timed effects. | `S` | Turn-boundary semantics match source tests. |
| M10.10 | [ ] Implement poison. | `T` | Application, tick damage, cure, and KO behavior are tested. |
| M10.11 | [ ] Implement sleep. | `T` | Turn skipping and wake conditions are tested. |
| M10.12 | [ ] Implement stun. | `T` | Duration and turn skipping are tested. |
| M10.13 | [ ] Implement silence. | `T` | Spell command availability updates correctly. |
| M10.14 | [ ] Implement taunt. | `S` | Enemy targeting honors and expires the effect. |
| M10.15 | [ ] Implement revive. | `S` | Only KO allies qualify and restore amount matches. |
| M10.16 | [ ] Implement battle recovery items. | `S` | Inventory consumption and target rules match. |
| M10.17 | [ ] Implement battle status-cure items. | `T` | Supported effects clear and items consume once. |
| M10.18 | [ ] Implement battle throw items. | `S` | Element, target shape, damage, and consumption match. |
| M10.19 | [ ] Implement enemy action selection. | `S` | Weighted, conditional, and fallback actions are deterministic. |
| M10.20 | [ ] Implement boss action restrictions. | `S` | Boss-only rules have focused fixtures. |
| M10.21 | [ ] Implement damage floats. | `T` | Spawn, rise, fade, and cleanup are bounded. |
| M10.22 | [ ] Implement hit flash. | `T` | Flash timing restores the original material/color. |
| M10.23 | [ ] Implement battle FX event routing. | `S` | Resolver logic is independent of animation completion. |
| M10.24 | [ ] Calculate GP and EXP rewards. | `S` | Multi-enemy and KO-member fixtures match. |
| M10.25 | [ ] Calculate deterministic loot drops. | `S` | Seeded rolls and quantity bounds match. |
| M10.26 | [ ] Apply rewards atomically. | `S` | Caps, loot batch, GP, and EXP update once. |
| M10.27 | [ ] Implement EXP thresholds. | `T` | Every class fixture matches its curve. |
| M10.28 | [ ] Implement one level-up. | `S` | Stats, HP/MP maxima, restore, and ability unlock match. |
| M10.29 | [ ] Implement multiple level-ups from one reward. | `S` | Each crossed level applies exactly once. |
| M10.30 | [ ] Render post-battle rewards. | `S` | GP, loot, EXP, levels, and learned abilities display. |
| M10.31 | [ ] Add a boss-defeat flag hook. | `S` | Only configured boss outcomes set managed flags. |
| M10.32 | [ ] Add full battle parity fixtures. | `X` | Representative physical, spell, item, status, boss, and reward transcripts agree. |

**Gate 10:** The first zone's complete enemy roster and boss can be fought with
all relevant party commands, progression, loot, and feedback.

## Milestone 11 — Economy, services, and quests

| ID | Task | Model | Done when |
| --- | --- | --- | --- |
| M11.01 | [ ] Route dialogue `open_shop` to a service request. | `S` | Item, weapon, armor, and magic-core types remain distinct. |
| M11.02 | [ ] Filter shop stock by unlock flag. | `T` | Locked entries remain hidden and stock is stateless. |
| M11.03 | [ ] Render item shop buy rows and details. | `S` | Price, description, balance, and affordability display. |
| M11.04 | [ ] Buy one item. | `S` | GP and quantity update atomically with caps enforced. |
| M11.05 | [ ] Render sellable repository rows. | `T` | Locked, key, and zero-value items are excluded/disabled correctly. |
| M11.06 | [ ] Sell one item. | `S` | Explicit sell price, quantity, and GP cap rules match. |
| M11.07 | [ ] Add equipment compatibility to shop details. | `S` | Each party member shows compatible/blocked and stat deltas. |
| M11.08 | [ ] Add inn confirmation and affordability rules. | `T` | Cost and cancel paths preserve state. |
| M11.09 | [ ] Apply inn recovery. | `S` | GP is charged and eligible HP/MP/status state matches Python. |
| M11.10 | [ ] Load and classify apothecary recipes. | `T` | Locked, missing-input, ready, and unique-owned states resolve. |
| M11.11 | [ ] Render the apothecary recipe list. | `S` | Hidden mechanical details and lock icons match state. |
| M11.12 | [ ] Craft one recipe. | `S` | Inputs and GP are consumed and output granted atomically. |
| M11.13 | [ ] Block duplicate unique output. | `T` | No resources change on rejection. |
| M11.14 | [ ] Load quest definitions into a catalog. | `T` | Every current quest is addressable. |
| M11.15 | [ ] Start a quest from dialogue effects. | `S` | Duplicate starts are idempotent. |
| M11.16 | [ ] Advance flag-based quest objectives. | `S` | Progress updates only on relevant flag changes. |
| M11.17 | [ ] Advance item-based quest objectives. | `S` | Quantity thresholds and item removal rules match. |
| M11.18 | [ ] Complete a quest and grant rewards. | `S` | State and rewards apply once. |
| M11.19 | [ ] Render the quest board/list. | `S` | Active/completed state and objectives display. |
| M11.20 | [ ] Add service and quest save-roundtrip tests. | `S` | Mid-shop-independent state and quest progress survive reload. |

**Gate 11:** Ardel's shop, inn, apothecary path, equipment economy, and quest
progression work against shared state and survive save/load.

## Milestone 12 — Content migration backlog

Do not turn any wildcard below into a bulk commit. Each wildcard defines a
family of separately tracked tasks. Instantiate the ID with the source stem,
for example `C-MAP-town_01_ardel`. Every instance inherits the listed model
assignment and acceptance criteria.

| ID template | One task per… | Model | Done when |
| --- | --- | --- | --- |
| C-CLASS-`<id>` | class YAML file | `T` | File is copied/registered, validates, and its progression fixture passes. |
| C-ITEM-`<stem>` | item YAML file | `T` | File validates and every item resolves in the catalog. |
| C-ENEMY-`<stem>` | enemy-rank YAML file | `T` | File validates and every referenced sprite/action/drop resolves. |
| C-ENCOUNTER-`<id>` | encounter YAML file | `T` | File validates and a seeded selection fixture passes. |
| C-RECIPE-`<stem>` | recipe YAML file | `T` | Every input/output/flag resolves and one craft fixture passes. |
| C-DIALOGUE-`<id>` | dialogue YAML file | `T` | Every node/effect/reference validates and a traversal fixture reaches each terminal. |
| C-MAPDATA-`<id>` | map YAML file | `T` | NPCs, shops, audio, boxes, signs, and encounter refs validate. |
| C-TMX-`<id>` | TMX map and only its newly required TSX/images | `T` | Assets are licensed, copied, loadable, and all paths resolve. |
| C-PORTAL-`<id>` | map's outgoing portal set | `T` | Every destination exists and a round-trip test covers each reversible link. |
| C-PLAY-`<id>` | playable map | `S` | Spawn, bounds, collision, portals, interactions, audio, and screenshot are manually checked. |
| C-AUDIO-`<id>` | BGM/SFX index entry | `T` | License is recorded and runtime playback resolves once. |
| C-ASSET-`<path>` | otherwise unclaimed asset | `T` | Origin/license/destination/hash are in the ledger and the asset loads. |

Apply those tasks in these playable waves; finish all applicable instances for
one wave before starting the next.

| Wave | Content boundary | Model for wave audit | Exit check |
| --- | --- | --- | --- |
| W12.1 | Ardel, its house/shop/inn/shrine, Starting Forest, and first boss | `gpt-5.6-sol`, high | Act I opening is playable without debug injection. |
| W12.2 | Open Plains, both caves, Millhaven and interiors | `gpt-5.6-sol`, high | Act I/II transition and Reiya/Jep-related prerequisites are reachable. |
| W12.3 | Marshland and Harborgate interiors | `gpt-5.6-sol`, high | Harborgate story and recruitment flow is playable. |
| W12.4 | Ancient Ruins gate/courtyard/sanctum and Ruinwatch | `gpt-5.6-sol`, high | Act III progression and boss flag are reachable. |
| W12.5 | Mountain Foothills maps, Frostholm, palace, and vault | `gpt-5.6-sol`, high | Frostholm story, Kael flow, and boss progression work. |
| W12.6 | Mountain Pass maps, Ashenveil, and oracle sanctum | `gpt-5.6-sol`, high | Act IV setup is playable. |
| W12.7 | Sunken Cave, Corrupted Forest, Volcanic Region | `gpt-5.6-sol`, high | Late-game encounters, loot, and services are balanced and reachable. |
| W12.8 | Final Stronghold and campaign ending path | `gpt-5.6-sol`, xhigh | A clean new save can reach credits/game-complete state. |

For each wave audit, compare a Rust playthrough with the pinned Python build,
record intentional differences, run the Rust validator, and save/reload at the
wave boundary.

## Milestone 13 — Debuggability and authoring workflow

| ID | Task | Model | Done when |
| --- | --- | --- | --- |
| M13.01 | [ ] Add a CLI seed option. | `T` | The seed is logged and controls all gameplay randomness. |
| M13.02 | [ ] Define a versioned input-record format. | `X` | Header captures game/source version, seed, and action schema. |
| M13.03 | [ ] Record normalized gameplay actions. | `S` | Recording is independent of physical key mappings. |
| M13.04 | [ ] Replay normalized gameplay actions. | `S` | One title-to-world trace ends at the same state hash. |
| M13.05 | [ ] Add periodic state hashes to replay. | `S` | Divergence identifies the first mismatching action. |
| M13.06 | [ ] Add debug start-map and start-position options. | `T` | Invalid map/position fails before the window opens. |
| M13.07 | [ ] Add debug party presets. | `S` | Presets are isolated from normal new-game construction. |
| M13.08 | [ ] Add debug flag overrides. | `T` | Overrides are logged and never written unless explicitly saved. |
| M13.09 | [ ] Add per-system frame timing. | `S` | World and battle hotspots can be measured without changing behavior. |
| M13.10 | [ ] Add an automated map-load sweep. | `S` | Every migrated TMX loads and advances several frames headlessly. |
| M13.11 | [ ] Add an automated dialogue traversal sweep. | `S` | Every branch terminates or reports a cycle explicitly. |
| M13.12 | [ ] Add an automated encounter construction sweep. | `T` | Every encounter builds valid combatants and assets. |
| M13.13 | [ ] Decide how the existing map editors target Rust content. | `X` | Reuse, adapt, or replace is recorded with file-format constraints. |
| M13.14 | [ ] Document the Rust content-author validation loop. | `T` | Edit, validate, run-map, and replay commands are copy-pasteable. |

**Gate 13:** A reported bug can be reproduced from a seed or replay, and
content authors can validate a change without launching a full campaign.

## Milestone 14 — Final parity, packaging, and handoff

| ID | Task | Model | Done when |
| --- | --- | --- | --- |
| M14.01 | [ ] Run the complete feature parity checklist. | `X` | Every row passes or links to an accepted difference. |
| M14.02 | [ ] Run a new-game-to-ending clean playthrough. | `X` | No debug option or source checkout is required. |
| M14.03 | [ ] Run a save/load soak across every wave boundary. | `S` | All saves restore and progress without drift. |
| M14.04 | [ ] Run deterministic replay checks on representative battles. | `S` | Replays remain stable in debug and release builds. |
| M14.05 | [ ] Audit all shipped asset licenses and attributions. | `X` | No unknown or prohibited asset ships. |
| M14.06 | [ ] Remove unused prototype/source assets. | `T` | The asset sweep and runtime tests find no missing references. |
| M14.07 | [ ] Measure release build startup, frame pacing, and memory. | `S` | Budgets are recorded and regressions have owners. |
| M14.08 | [ ] Package a self-contained release build. | `S` | Binary finds content without the source repository or working-directory assumptions. |
| M14.09 | [ ] Test the release package on a clean Linux user profile. | `S` | New game, save, load, audio, and finish path launch correctly. |
| M14.10 | [ ] Update README from prototype to game instructions. | `T` | Requirements, run, controls, saves, validation, and credits are current. |
| M14.11 | [ ] Archive the final Python-source hash and parity report. | `T` | Future maintainers can reproduce what “ported” meant. |
| M14.12 | [ ] Declare Python runtime independence. | `X` | CI and packaged playthrough use no Python engine code at runtime. |

## Explicit deferrals

These do not block the first parity release unless the pinned Python game uses
them on the critical completion path:

- new art replacing source placeholders;
- redesigning scenario schemas for Rust aesthetics;
- multiplayer, networking, mobile, or web targets;
- a new in-Bevy map editor;
- localization beyond preserving source text;
- save-cloud synchronization;
- gameplay or balance changes not required for source parity.

Record any newly discovered deferral in this section with its affected parity
row. Do not silently omit it from a milestone.

## Next task

Start with **M0.01**. Do not begin engine architecture or bulk asset copying
until Gate 0 is complete.
