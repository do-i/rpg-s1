# Rust content-author validation loop

Rusted Kingdoms content is read directly from
`assets/scenarios/rusted_kingdoms`. YAML, TMX, TSX, images, and audio are
runtime inputs; editing them does not require rebuilding Rust.

## Map editor prerequisites

The portal-graph editors live in the pinned `agentic-rpg` checkout documented
by ADR 0008. Place that checkout beside this repository, or set
`RPG_S1_EDITOR_REPO` to it. Then run the one-time setup and verification:

```sh
scripts/map-editor.sh setup
scripts/map-editor.sh check
```

Setup creates the editor checkout's `.venv`, installs its declared Python
editor dependencies, installs the locked web dependencies, and builds the web
frontend. It does not install a Python dependency into the Rust game.

Launch either portal editor from this repository:

```sh
scripts/map-editor.sh pygame
scripts/map-editor.sh web
```

Both point directly at `assets/scenarios/rusted_kingdoms`. They edit portal
objects and create `.tmx.bak` recovery files. Use Tiled for painting visible
layers, collision/spawn markers, tilesets, and non-portal objects. Review the
resulting TMX diff before validation; backups and editor caches are ignored as
derived artifacts.

To edit a different in-repository package or a disposable copy:

```sh
RPG_S1_SCENARIO_PACKAGE=minimal_demo scripts/map-editor.sh pygame
RPG_S1_SCENARIO_ROOT=/tmp/rpg-s1-scenario-copy scripts/map-editor.sh web
```

The current external editor expects TMX beneath `assets/maps`; the Rust
runtime itself continues to follow each manifest's `refs.tmx` value.

## Edit, validate, and run one map

Build once before the loop, then edit content with Tiled or an editor above:

```sh
cargo build
git diff -- assets/scenarios/rusted_kingdoms
target/debug/rpg-s1 validate-scenario rusted_kingdoms
target/debug/rpg-s1 map-sweep rusted_kingdoms
target/debug/rpg-s1 play rusted_kingdoms --seed 1 \
  --start-map town_01_ardel --start-position 10,0
```

`map-sweep` runs the structural checks and then loads all 45 migrated maps
through the production TMX/TSX/image asset chain for five headless frames
each. The two `sample_*` authoring fixtures are listed but excluded from the
migrated-map pass.

Choose a walkable tile for another map's `--start-position`; an invalid,
outside, or colliding position fails before a window opens. Add debug state
only when the content needs it:

```sh
target/debug/rpg-s1 play rusted_kingdoms --seed 1 \
  --start-map town_01_ardel --start-position 10,0 \
  --party-preset full --set-flag story_quest_started
```

Debug overrides are logged and session-only unless an explicit manual save is
made.

## Validate dialogue and encounters

Run the targeted sweep after changing its content, and run all three before a
cross-cutting content commit:

```sh
target/debug/rpg-s1 dialogue-sweep rusted_kingdoms
target/debug/rpg-s1 encounter-sweep rusted_kingdoms
target/debug/rpg-s1 map-sweep rusted_kingdoms
```

The dialogue sweep traverses every authored root and choice. Every path must
terminate or print its exact cycle. The encounter sweep constructs every
weighted formation and boss through the production battle-entry boundary and
loads every referenced enemy atlas and battle background.

## Record and replay the changed behavior

Use a fresh path because recording never overwrites an existing file:

```sh
target/debug/rpg-s1 record /tmp/rpg-s1-author-check.yaml \
  rusted_kingdoms --seed 13 \
  --start-map town_01_ardel --start-position 10,0
```

Perform the affected actions and exit normally. Then replay without physical
gameplay input:

```sh
target/debug/rpg-s1 replay /tmp/rpg-s1-author-check.yaml
```

Replay validates the recorded game/scenario identity before opening a window,
checks every periodic state hash, reports the first mismatching action, and
exits successfully only after the final recorded frame matches.

Finish the loop with:

```sh
cargo test --workspace
git diff --check
git status --short
```

The same actions are available from `lazymenu-cli`: search for Play, Run map,
Record, Replay, Validate, Map sweep, Dialogue sweep, Encounter sweep, test
suite, Pygame editor, or Web editor.
