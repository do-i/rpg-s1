# Refactoring evidence

## Baseline — 2026-10-11, `ac5d827`

Recorded before RF-01 implementation. Raw logs are ignored under
`temp/refactoring/baseline-*.log`. Cargo reused the existing shared target cache;
the sandbox required permission to write that cache.

- `cargo fmt --all -- --check`: passed.
- `cargo test --workspace`: passed, 196 content tests, 823 engine tests, and
  one integration test; no failures or ignored tests.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo run -- validate-scenario rusted_kingdoms --baseline assets/scenarios/rusted_kingdoms/validation-baseline.txt`:
  passed against the unchanged baseline (four accepted `flag.undefined` errors
  for the sorcerer's ultimate fire, water, wind, and earth unlock flags).
- `cargo run -- map-sweep rusted_kingdoms`: all 54 maps passed five headless frames.
- `cargo run -- dialogue-sweep rusted_kingdoms`: 128 documents, 304 root branches,
  318 terminating paths, no cycles or errors.
- `cargo run -- encounter-sweep rusted_kingdoms`: 19 zones, 158 encounters,
  119 assets, no construction or asset failures.
- `python3 -m unittest -v scripts/test_check_asset_rights.py`: passed.
- `npm run build` in `tools/map_editor_web`: passed using installed dependencies.
- `bash scripts/map-editor.sh check`: passed, 54 maps and 108 portals.

The sweeps used `RPG_S1_MUTE_AUDIO=1`. No editor behavior test suite exists at
this baseline; the existing editor build and service load were checked. These
headless checks do not establish visual, interactive, or full campaign acceptance.

## RF-01 — Atomic save creation without replacement

Reproduced with `before_replace`: creating a destination after the initial
existence check caused the old implementation to return success and replace the
incumbent. The regression failed before the fix (`rf01-red.log`).

Changed `crates/rpg-engine/src/save_store.rs` to publish the verified temporary
file with a same-directory hard link when overwrite is forbidden. Linux creates
that directory entry atomically or refuses an occupied name, including dangling
symlinks. Existing entries return `DestinationExists`; explicit overwrite retains
atomic rename. Temporary-file verification, file/directory synchronization, and
cleanup remain in place. Filesystems that reject hard links fail safely without
falling back to replacement.

Inspected `crates/rpg-engine/src/python_save_import.rs`: replacement still creates
and verifies the backup before publication. An initially absent destination now
uses no-replace publication even with `--replace`, since there is no backup of a
slot that arrives concurrently.

Validation after the fix:

- `cargo test --workspace`: passed, 196 content tests, 828 engine tests, and one
  integration test. This includes five new regressions for late save creation,
  existing/late dangling symlinks, publication failure, interruption cleanup, and
  late import creation with and without `--replace`.
- Existing explicit-overwrite, verified-import-backup, import interruption,
  save interruption, and native format/golden tests passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo fmt --all -- --check` and `git diff --check`: passed.

Raw check logs are in ignored `temp/refactoring/rf01-*.log`. Tests use disposable
save directories. No scenario assets, save formats, accepted validation baseline,
or editor code changed. Real power loss and filesystem-specific sync failures
were not simulated; interactive save/import UI acceptance remains manual.

## RF-02 — Recover failed World map publication

Completed 2026-10-12. Disposable scenarios now exercise the real Bevy asset
loaders and World plugins with successful TMX preflight followed by malformed map
metadata, an absent NPC image dependency, failed object art, a malformed encounter
zone, and missing protagonist art. Both unresolved preflight and unresolved
publication have a tested 30-second deadline per phase.

The publication barrier observes pending/ready/failed outcomes from rendering,
player, actors, objects, collision, and encounters. Sprite and map checks include
recursive dependency failures. Initial World entry also waits for this barrier;
its original session and pending encounter-return data are captured before loader
updates so a retry does not repeat mutations against an already modified session.

The existing loaders publish against `GameState`, so recovery uses a complete
rollback transaction. After TMX preflight and source readiness succeed, the source
map's flat entity bundles are detached into a backup while their Entity IDs remain
reserved. The backup also retains the full session, loader resources, collision,
and encounter state/catalog. Dormant entities have no live Sprite, Camera, or
World simulation marker. A failure removes the partial destination and restores
the original bundles and resources atomically, preserving sub-tile motion,
facing, NPC state, party, visited history, and RNG. Success and World exit discard
the dormant source. Map music follows only a completed publication.

A persistent error screen has its own camera and offers `R` to retry, `Esc` to
return to a complete restored source when available, and `T` to return to title.
Retry explicitly reloads participating asset paths and their TMX/TSX dependencies;
loaders wait briefly for the asynchronous reload to replace cached failure states.
No automatic retry loop runs. Simulation and autosave stay locked throughout a
failed or incomplete publication. A destination that dismisses companions must
also publish the sprite for the final controlled member before the barrier opens.

Validation:

- `cargo test --workspace -j 1`: passed, 196 content tests, 840 engine tests,
  and one integration test (1,037 total). The 12 new tests cover late NPC image
  failure, exact source Entity IDs and sub-tile/session restoration, unchanged
  autosave bytes, repaired dependency retry and successful source disposal,
  malformed metadata, object art failure with companion dismissal, initial
  protagonist failure and title return, initial retry equivalence to a clean
  arrival, malformed encounter data, both timeout phases, preflight dependency
  failure, World exit during publication, and final controlled-player readiness.
- `cargo clippy --workspace --all-targets -j 1 -- -D warnings`: passed.
- `cargo fmt --all -- --check` and `git diff --check`: passed.
- The freshly built `target/debug/rpg-s1 map-sweep rusted_kingdoms`: passed,
  all 54 maps over five headless frames each.
- The freshly built `target/debug/rpg-s1 encounter-sweep rusted_kingdoms`: passed,
  19 zones, 158 constructions, 119 assets, no failures.
- The freshly built `target/debug/rpg-s1 validate-scenario rusted_kingdoms
  --baseline assets/scenarios/rusted_kingdoms/validation-baseline.txt`: passed
  against the unchanged four accepted diagnostics, with no new diagnostics.

Raw logs are under ignored `temp/refactoring/rf02-*.log`. An initial workspace
link exited with SIGBUS while cache space was low; only two abandoned linker
`.tmp` artifacts were removed before the successful serial build. No full cache
clean or scenario asset changes were needed. The recovery UI, dedicated camera,
keyboard actions, and persistence behavior were tested headlessly; visual and
interactive full-game acceptance remain manual. Runtime hot-change invalidation
is still RF-08 and was not implemented here.
