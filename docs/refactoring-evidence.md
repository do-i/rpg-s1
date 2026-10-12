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
