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

## RF-03 — Atomic editor TMX publication

Completed 2026-10-12. Re-ran interruption and invalid-serialization regressions
against the pre-change writer from Git on disposable scenarios: all nine
create/retarget/delete failure cases failed, including canonical bytes becoming
`b'<?xml'` (`rf03-red.log`). No shipped map was modified.

Changed `tools/map_editor/edit/tmx_writer.py` so all three edit operations use
one publication helper. It serializes into a unique sibling temporary file,
flushes and reparses the XML, verifies the complete parsed tree against the
intended tree independently of the serializer, preserves the map's file mode,
and syncs the temporary file. The first `.bak` is also staged, verified against
the original bytes, synced, and linked without replacing an established backup.
The backup directory entry is synced before atomic `os.replace` publishes the
map, then the directory is synced again. Failed stages clean up temporary files.

`tools/map_editor/service/editor_service.py` refreshes the graph only after
publication. A directory-sync failure after replacement explicitly reports that
the map was saved but its durability is uncertain; the service refreshes the
graph to match those published bytes before propagating the error. The web
backend exposes that message in its HTTP error detail so a user can inspect the
saved graph before repeating an edit.

Added a reusable disposable scenario fixture and behavioral suite under
`tools/map_editor/tests`, plus
`crates/rpg-content/tests/editor_tmx_publication.rs`. The Rust integration test
runs the actual stdlib-only Python writer and parses each successful create,
retarget, and delete publication with the engine's content parser. It verifies
object IDs, source geometry, typed destination properties, and untouched ground
data. It requires `python3`, or `RPG_S1_EDITOR_TEST_PYTHON`, without editor Python
packages.

Validation:

- `PYGAME_HIDE_SUPPORT_PROMPT=1 tools/.venv/bin/python -m unittest discover
  -s tools/map_editor/tests -t . -v`: passed, nine tests. Coverage includes all
  three mutation paths under interrupted/invalid serialization and sync/copy/
  link/replace failures, corrupt/interrupted backups, backup sync failure,
  publication ordering, cleanup, unchanged original file and serialized graph
  bytes before replacement, file permissions, first-backup preservation,
  editor reload compatibility, and the published-but-unsynced graph/HTTP result.
- `cargo test -p rpg-content -j 1`: passed, 196 existing content tests and the
  new Python-to-Rust integration test.
- `cargo clippy -p rpg-content --all-targets -j 1 -- -D warnings`: passed.
- `cargo fmt --all -- --check` and `git diff --check`: passed.
- `npm ci` followed by `npm run build` in `tools/map_editor_web`: passed.
- `bash scripts/map-editor.sh check`: passed, 54 maps and 108 portals.

Raw logs are under ignored `temp/refactoring/rf03-*.log`. Cargo reused the shared
cache with the required sandbox permission. Real power loss, process termination
that bypasses cleanup, and interactive browser acceptance remain manual. A hard
process termination can leave an unreferenced temporary file, while the canonical
map remains complete. Geometry/destination validation, concurrent edit conflicts,
and render cache invalidation remain separate tasks.

## RF-04 — Preserve rejected options files

Completed 2026-10-12. Reproduced the data loss through the real Options input and
persistence systems: load `version: 99`, capture a new Travel key, and inspect the
saved bytes. Before the fix, the test failed because `options.yaml` was replaced
by version 1 settings (`rf04-red.log`). No player configuration was touched.

Added an `OptionsLoadStatus` resource distinguishing missing files, clean loads,
recoverable row warnings, and rejected files. Launch still uses playable defaults
when loading fails. Ordinary option changes remain active in memory, while both
Options screen and field-menu battle settings use guarded persistence. A rejected
startup file stays protected even if session defaults are edited; persistence also
checks the current file for rejection before an ordinary write. The Options
screen displays why the original is preserved and explains that edits apply only
for the current session. Battle setting changes display the same explanation.

For rejected files, Reset To Defaults now opens a confirmation describing the
concrete operation: back up the original to a free
`options.yaml.rejected-001.bak` through `options.yaml.rejected-999.bak` name, then
save shipped defaults. Back cancels without losing session choices. Confirmation
copies the exact original bytes with exclusive creation, syncs and verifies the
backup, and syncs its directory before using the existing atomic options writer.
Existing backups are never replaced. Backup failure leaves the original intact;
the screen retains the active session choices and explains the failure. Successful
recovery reports the backup path and permits ordinary persistence again.

Validation:

- `cargo test --workspace -j 1`: passed, 196 content tests, 848 engine tests, the
  Python-to-Rust editor integration test, and one application integration test.
  Eight new regressions cover the actual rebinding/write reproduction, all three
  field-menu battle settings, missing/warning/rejected load states, future schemas
  with unknown fields, malformed documents, rejection after startup, exact backup
  bytes and existing-backup preservation, exhausted backup names, deliberate reset
  confirmation and cancellation, displayed Options status text, failed recovery,
  and restored persistence after successful recovery.
- `cargo clippy --workspace --all-targets -j 1 -- -D warnings`: passed.
- `cargo fmt --all -- --check` and `git diff --check`: passed.

Raw logs are under ignored `temp/refactoring/rf04-*.log`. Cargo reused the shared
cache with required sandbox permission; one automatic approval review timed out
and its allowed retry succeeded. Options/save formats, scenario assets, and editor
behavior are unchanged. General save/options failure status and retry controls
remain RF-05. Recovery UI behavior was tested headlessly; visual/full-game
acceptance and real power loss remain manual.

## RF-05 — Visible persistence failures and deliberate retry

Completed 2026-10-12. Source inspection confirmed that arrival autosave failures
only reached the tracker getter, while ordinary Options and battle-setting I/O
failures only reached the log. The existing Options dirty flag and autosave
attempted-map guard already prevented per-frame write storms; those protections
remain in place. Reproduction uses disposable save/config directories and
per-store, test-only failure injection, without touching player files.

A shared status overlay persists across screen changes and shows separate autosave
and Options feedback. Failed writes explain that active session progress/choices
have not been durably saved. Successful writes replace stale failure feedback
with confirmation of disk persistence. Pointer Retry and Dismiss controls operate
on the relevant notice; dismissal hides feedback without erasing failure state.
The overlay consumes its pointer hover/click before menu hit tests and does not
intercept configurable gameplay bindings. A new failed write reopens a dismissed
notice.

Options screen writes, field-menu battle setting writes, and deliberate retries
share the RF-04 guarded persistence path and retain error logging. Retry cannot
replace a rejected options
file, even after it is removed externally; the existing explicit backup/reset
confirmation remains the recovery path. Successful explicit reset clears stale
Options write feedback. Autosave retry rebuilds from live GameState only after
the RF-02 publication barrier is settled, requires the attempted map and scenario
identity/version, and is consumed once even if publication is locked or failed.
Title entry clears the autosave attempt, error, and pending retry for the next
session. Pre-publication write/sync failure preserves the previous file exactly;
temporary files are cleaned up. A directory-sync failure after atomic replacement
instead reports that the complete file was published but durability is uncertain.
Autosave requests a slot-catalog refresh for both successful durable publication
and published-but-unsynced outcomes.

Validation:

- `cargo test --workspace -j 1`: passed on the final source snapshot, 196 content
  tests, 859 engine tests, one Python-to-Rust integration test, and one application
  integration test (1,057 total). Eleven new engine regressions cover injected
  write/temporary-sync failures, exact previous-file preservation, cleanup and
  absence of per-frame retries, read-only directory recovery, live-state retry,
  cleared errors and requested catalog refresh, published-but-unsynced feedback,
  failed publication and obsolete map/scenario retries, title/session reset,
  rejected-file retry protection, pointer Retry/Dismiss and click-through
  prevention, real Options rebinding, and all three battle setting writes. The
  existing explicit-reset regression also checks stale failure clearing.
- `cargo clippy --workspace --all-targets -j 1 -- -D warnings`: passed.
- `cargo fmt --all -- --check` and `git diff --check`: passed.

Raw logs are under ignored `temp/refactoring/rf05-*.log`. The shared Cargo cache is
reused with sandbox permission and one worker. An initial automatic approval
review could not run because of the usage limit; user-authorized resumption
allowed the test run to proceed. A later full-cache linker SIGBUS and Clippy
`No space left on device` were resolved without a full cache clean: remove the
confirmed abandoned linker temporary, then archive this task's engine incremental
snapshots and superseded generated root/test executables to
`/tmp/rpg-s1-rf05-cache-backup`. No compiler/linker was active during archival;
dependency caches and unrelated artifacts were retained. The next link reported
inconsistent hidden LLVM symbols in engine incremental outputs. A subsequent
`cargo clean -p rpg-engine` removed only generated engine artifacts before the
serial rebuild; content/dependency caches remained available. Headless tests
cover feedback and pointer routing; full-game visual acceptance at different window scales, real disk-full behavior,
and real power loss remain manual. No performance claim is made.

## RF-06 — Reject invalid portal geometry and destinations

Completed 2026-10-12. Re-ran the invalid geometry/destination regression against
the pre-change service from commit `6c8e05e` on disposable maps: all 26 negative
extent, disconnected source, coordinate-overflow, and out-of-bounds destination
cases were accepted (`rf06-red.log`). The inherited RF-05 baseline was 1,057
workspace tests and workspace Clippy passing; RF-03's nine publication tests also
passed. No shipped assets were edited during reproduction or implementation.

Create and retarget now share validation before invoking the writer. Request
coordinates must be integers in the i32 domain; booleans, strings, fractions,
nonfinite numbers, malformed tuple sizes, negative extents, and overflowing
rectangle endpoints are rejected with field-specific errors. Fresh TMX headers
provide the finite map dimensions and each map's authored tile sizes, including
different rectangular tile grids. Source rectangles must touch the map's pixel
bounds. Zero-size points and zero-width/height lines remain valid.

The production audit found 44 of 108 runtime portals with fractional rectangles
crossing a source map boundary. Those are valid edge triggers under the existing
inclusive contact rule in `world_transition.rs`; rejecting all negative origins
or requiring complete containment would break authored content. All 108 existing
portals pass the new validation. The graph now retains exact fractional geometry
and zero extents instead of truncating origins and replacing points with 1x1
rectangles. A plain retarget keeps those authored values. The runtime validator
has not been tightened.

Destination validation follows the existing `scenario_map_sweep.rs::check_portals`
arrival policy: a tile must be inside the target grid and open according to
`CollisionOccupancy`. The target must have one matching uncompressed CSV collision
layer. Hidden collision layers still apply; supported Tiled flip flags are masked
when deciding whether a GID is empty, and unsupported rotation flags/malformed
collision data are rejected. This check does not try to predict dynamic actor
occupancy. The authored corpus has no out-of-bounds or blocked destination tiles.

Dedicated lookup and input errors distinguish unknown map/portal IDs (404) from
invalid edits (422). Strict request schemas prevent lossy integer conversion.
Transport validation errors also return readable field text without echoing
NaN/Infinity into an invalid JSON error response. Writer/serialization failures
are no longer incorrectly translated into unknown-ID errors.

Added reusable complete-file/graph snapshots and a bounded ASGI request helper to
the disposable scenario fixture. The behavioral suite checks rejection without
canonical/backup/temp file or graph changes for both mutation paths, plain
retargets, exact authored edge geometry, rectangles/points/lines, differing map
dimensions and tile sizes, runtime collision GID semantics, and real API status
codes. A Rust runtime regression invokes only the stdlib Python writer, parses
its publications, and extracts rectangles and created/plain-retargeted/moved
point portals through `runtime_portals`; it also verifies inclusive exact-touch
triggering and preservation of fractional edge geometry on a 16x24 tile grid.

Validation:

- `PYGAME_HIDE_SUPPORT_PROMPT=1 SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy
  tools/.venv/bin/python -m unittest discover -s tools/map_editor/tests -t . -v`:
  passed, 25 tests (16 new and nine RF-03 publication regressions).
- `cargo test -p rpg-engine --lib world_transition -j 1`: passed, 34 tests,
  including the new actual editor-to-runtime extraction regression.
- `cargo test -p rpg-content -j 1`: passed, 196 parser/content tests and the
  existing Python-to-Rust create/retarget/delete integration.
- `cargo clippy -p rpg-engine -p rpg-content --all-targets -j 1 -- -D warnings`:
  passed.
- `cargo fmt --all -- --check` and `git diff --check`: passed.
- `npm ci` followed by `npm run build` in `tools/map_editor_web`: passed.
- `bash scripts/map-editor.sh check`: passed, 54 maps and 108 portals.
- The create-tiled-map CSV validator and offscreen `tmxrasterizer` accepted
  `tools/map_editor/tests/fixtures/source.tmx`; its preview is a temporary file.

Raw logs are under ignored `temp/refactoring/rf06-*.log`. Cargo reused the shared
cache with sandbox permission and one worker. The sandbox prevented FastAPI's
AnyIO worker-thread wakeups; a minimal threadpool probe reproduced the same
timeout, and the API suite passed immediately with normal local socket/thread
access. The ASGI helper has a five-second request timeout. Interactive browser
acceptance remains manual. Mutation concurrency/conflicts and thumbnail cache
invalidation remain RF-07/RF-09.

## RF-07 — Prevent conflicting editor mutations

Completed 2026-10-12. The inherited workspace/Clippy baseline remains recorded
above; this task changed Python/TypeScript only. The pre-change focused editor
suite passed all 25 tests. Disposable threaded probes reproduced three failures:
two creates acknowledged object ID 2 but only one new portal survived; a retarget
published after an acknowledged delete resurrected object ID 1; and an external
Tiled-style XML edit between read and publication was overwritten. The bounded
probes and output are in ignored `temp/refactoring/rf07-reproduce.py` and
`rf07-red.log`; no shipped maps were edited.

An `RLock` now encloses service validation, mutation, publication, graph refresh
and JSON snapshots. Concurrent direct service creates retain distinct IDs, and
retarget/delete operations run in a consistent order. Graph readers wait until
publication and graph refresh finish. Successful edits also refresh a destination
whose on-disk graph changed before validation. The writer parses one byte snapshot,
records its SHA-256, and checks source plus validated destination revisions before
backup/publication and again immediately before replacement. Service revisions
are captured before validation, so a change during validation cannot be accepted
as the edit's original revision. The first backup, temporary cleanup and published
but unsynced error behavior remain covered by the RF-03 tests.

Graph nodes and successful mutation replies expose source revisions; browser
mutations send `If-Match`. A stale snapshot or observed external revision change
returns HTTP 409. Graph GET rereads metadata/portals and publishes its snapshot
only after checking its revisions. The browser explains conflicts, discards stale
dialogs, clears undo/redo history and refreshes once without replaying the obsolete
request. It also clears history if an external change is discovered by a graph
refresh after an acknowledged edit. A browser mutation guard prevents overlapping
local history actions. Reciprocal portals remain separate saves; partial create,
undo or redo reports already acknowledged changes, refreshes, and clears partial
history rather than silently retrying or claiming a complete failure.

Deletion undo now restores from a server-held receipt of the original edge. This
preserves RF-06's accepted fractional authored geometry without relaxing ordinary
StrictInt create/resize or destination requests. Restore revalidates source contact
and arrival collision, allocates a fresh object ID, and consumes the receipt after
publication, including a published-but-unsynced restoration. Receipts are bounded
to the most recent 128 deletions; expired, used or restarted-session receipts return
409 and invalidate obsolete history. Frontend redo tracks the new restored IDs.

The supported policy is one process and one service instance per scenario; the
launcher explicitly selects one Uvicorn worker. Multiple browser tabs can share
that instance. The lock is not cross-process, and a hash comparison followed by
replacement is not filesystem compare-and-swap: a noncooperating Tiled save could
still occur after the final comparison. Coordinate Tiled saves with browser
mutations and refresh after the Tiled save. These limitations and conflict/history
behavior are documented in `tools/map_editor_web/README.md`. Render-cache changes
remain RF-09. No interactive browser or simultaneous real Tiled-save acceptance is
claimed by the headless probes.

Validation:

- `PYGAME_HIDE_SUPPORT_PROMPT=1 SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy
  tools/.venv/bin/python -m unittest discover -s tools/map_editor/tests -t . -v`:
  passed, 41 tests (25 existing and 16 conflict/restoration regressions).
- `npm ci`, `npm test`, and `npm run build` in `tools/map_editor_web`: passed.
  The ten frontend behavior cases exercise actual compiled API/history modules
  with response fixtures, including undo/redo conflicts, revision advancement,
  changed graph detection, fresh IDs, fractional deletion restoration, partial
  create/undo/redo outcomes, ordinary input errors, and expired receipts. They
  do not claim DOM or visual acceptance. No new test dependency was added.
- `cargo test -p rpg-content --test editor_tmx_publication -j 1`: passed, one
  Python-to-Rust create/retarget/delete parser integration.
- `cargo test -p rpg-engine --lib
  python_editor_rectangles_points_and_authored_edges_round_trip_runtime_extraction
  -j 1`: passed, one editor-to-runtime geometry extraction regression.
- `cargo fmt --all -- --check` and `git diff --check`: passed.
- `bash scripts/map-editor.sh check`: passed, 54 maps and 108 portals.
- The create-tiled-map CSV validator and offscreen `tmxrasterizer` accepted the
  unchanged disposable-map source fixture; its preview is temporary.

Raw logs are ignored under `temp/refactoring/rf07-*.log`. Cargo reused the shared
cache with permission and one worker; no cache cleanup was needed. Backend ASGI
and frontend Node worker tests ran with normal local thread/process access: the
sandbox's Node test runner reported only a file-level pass, while the normal run
listed and passed all ten behavioral cases. API requests and injected publication
waits remain bounded to five seconds. RF-11 will add CI gates for these checks.

## RF-08 — Publish runtime hot changes as one World transaction

Source inspection and `cargo tree -e features -i bevy_asset` confirmed that the
workspace's Bevy 0.19 build does not enable `file_watcher`. A disposable runtime
regression with watching disabled reproduces the original behavior: overwriting
the current map's TMX leaves the old sprites and collision occupancy in place.
The production `AssetPlugin` now explicitly disables autonomous watching and
`RPG_S1_HOT_RELOAD` enables coordinated 0.5-second package revision polling. An
unset variable installs no polling/completion systems. Linux inode/change-time
signatures detect atomic same-size replacements preserving modification time;
removed paths are included and symlink/backup-file entries are ignored.

Reloads wait for an idle World, retain RF-02's source bundles, and run replacement
publication through its existing lock, timeout, failure UI and source-return path.
The retained generation now includes cloned typed assets, image data, atlas
layouts, inventory, global catalog/load resources and RNG. This matters because
retaining handles alone would allow a successfully reloaded PNG/TSX to change the
supposedly rolled-back source when another file fails. AssetServer completion and
failure events gate rebuilding the same map's render bundles, collision, player,
NPCs, signs, item boxes, encounters and field/transport catalogs. Runtime portal
parsing and the active BGM dependency join the publication check. Audio requests
retain their candidate handles; requesting the same failed audio every frame would
otherwise restart IO and hide the failure behind a timeout.

Rejected generations restore the previous assets and resources, freeze global
manifest/new-game consumers against the failed server states, and retain the whole
rejected file set for explicit retry or a subsequent edit. They do not retry an
unchanged failure automatically. If a timed-out asset request completes later, an
asset-only guard restores the old values after Bevy's asset processing and before
Update/render extraction. Rollback-generated asset messages cannot count as reload
completion. Recovery labels update as pending work drains; title return remains
available throughout. Successful publication refreshes map audio and save-slot
balance/context. No existing save/session is migrated by edited bootstrap data.

The supported package switch remains a fresh `play PACKAGE_KEY` launch of the same
binary. Tests load two distinct packages with identical map IDs into separate real
AssetServers and verify their actors, collision, manifest roots and asset paths.
A live root mutation is restored before consumers run, and changing manifest
ID/version rejects the hot transaction. No in-process scenario-switch UI is
claimed. [Runtime hot-reload instructions](../docs/runtime-hot-reload.md) document the
supported flow, disposable visual/audio procedure, snapshot memory cost and
remaining manual checks under the existing sibling acceptance task
`RK-DBG-006`–`RK-DBG-016`; that read-only tracker is not marked complete.

The disposable tests exercise actual filesystem writes and production asset
loaders/World publication rather than calling the invalidation decision alone:
combined same-map TMX/TSX/YAML changes; updated geometry/collision/NPCs/item boxes;
mixed successful PNG/TSX/TMX changes plus failed YAML and exact restoration; removed
PNG and retry; item catalog commit/rollback; manifest identity rejection and retry
suppression; atomic replacement metadata; actual MP3 bytes and logical-loop restart;
new missing audio dependency; timeout followed by a late asset write and refreshed
recovery actions; and two selected packages. Headless sprite/audio entity assertions
do not establish live GPU appearance, audible playback or campaign acceptance.

Validation on the final source snapshot:

- `cargo test --workspace -j 1`: passed, **1,070 tests** (196 content unit tests,
  872 engine unit tests and two integration tests; zero failed/ignored). This
  includes the 12 new hot-change regressions. The preceding focused transition
  run passed 45 tests; the final disabled-watcher regression also passed in the
  workspace run.
- `cargo clippy --workspace --all-targets -j 1 -- -D warnings`: passed.
- `cargo fmt --all -- --check` and `git diff --check`: passed.
- `cargo run -j 1 -- validate-scenario rusted_kingdoms --baseline
  assets/scenarios/rusted_kingdoms/validation-baseline.txt`: baseline **PASS**,
  four accepted undefined ultimate-element flags, zero new or obsolete diagnostics.
  The ungated content report still says FAIL for those four existing errors.
- The final `target/debug/rpg-s1` binary's `map-sweep rusted_kingdoms`: all 54 maps
  clean, zero findings; runtime loading passed all 54 maps.
- `dialogue-sweep rusted_kingdoms`: passed, 128 documents, 304 roots, 318 terminating
  paths, zero cycles/errors.
- `encounter-sweep rusted_kingdoms`: passed, 19 zones, 158 constructions and 119
  assets, zero construction/asset failures.
- `python3 -m unittest -v scripts/test_check_asset_rights.py`: passed, five tests.
- `bash scripts/check-ardel-screenshot.sh temp/refactoring/rf08-ardel.png`:
  **FAILED, inherited oracle mismatch**. The actual RGBA8 hash is
  `e55dd69ac18af8a813f736f755939c63fe235e2db25b41fe2429d399153b0c69`;
  the unchanged oracle expects
  `85ce229c04604258debbad65643ebcc62177f084727178d29b968baeb35b2012`.
  Running the HEAD script against the same unchanged inputs reproduced the actual
  hash. `git diff ac5d827 -- assets scripts/check-ardel-screenshot.sh
  tests/fixtures/ardel-screenshot` was empty. This static Tiled rasterizer does not
  execute the changed Rust runtime. The resulting image was inspected; neither
  assets nor the oracle were changed to make the gate pass. Live-window hot-change
  appearance, movement and audible acceptance remain manual as documented above.

Raw logs and PNGs are ignored under `temp/refactoring/rf08-*`. Cargo reused the
existing shared cache with required permission and one build worker; this task
performed no cache cleanup or dependency/feature upgrade. No canonical scenario,
validation baseline, sibling tracker, or refactoring plan was edited or staged.

## RF-09 — Invalidate stale editor previews

The inherited cache returned old red pixels after a disposable TMX changed to a
green tile, and also republished an old in-memory full render when its disk PNG
was removed. Both stale-pixel assertions fail against the pre-change cache in
`rf09-red.log`; the fixture supplies only a no-op mutation callback so the old
lookup methods execute unchanged. The initial editor baseline passed 41 tests.

Thumbnail and full-resolution entries now share a SHA-256 revision over the map,
referenced external tilesets, and referenced image contents and identities. Each
lookup rechecks dependencies, including missing files and timestamp-preserving
replacements. Memory and both disk directories are invalidated together; the
legacy modification-time PNGs are discarded. Current disk filenames include map
identity and dependency revision, allowing fresh instances to reuse validated
PNG output without rendering and avoiding same-stem map collisions. PNG writes
use a sibling temporary file and replacement. Source revisions are checked after
rendering and after saving; changed candidates and failed output are discarded
instead of promoting old memory pixels to a new disk revision.

The editing service invokes invalidation on publication/refresh, including edits
that are already visible when synchronization fails and external edits rejected
as conflicts. Preview bytes are captured inside RF-07's publication lock and the
cache's lock; a later invalidation cannot unlink a path being streamed to an
already-started response. Graph payloads expose a separate composite render
revision, preserving the TMX revision used for optimistic edit conflicts. The
frontend passes that revision to graph thumbnails, tile pickers, and inspector
images. Changing it replaces the image element, including one hidden by an
earlier failure. Graph fetches and PNG responses use `no-store`; an obsolete
query revision can therefore receive current pixels without incorrectly giving
that URL an immutable browser cache entry. Tileset/image-only refreshes update
previews without clearing otherwise valid portal undo history. No new polling
behavior is introduced.

Validation on the final reviewed source:

- `python -m unittest discover -s tools/map_editor/tests -v` through the existing
  editor environment: passed, **57 tests**, including 16 new lifecycle tests.
  Real disposable TMX/TSX/PNG fixtures cover long-lived TMX/TSX/image changes,
  preserved timestamps, removed/malformed dependencies, constructor/drawing
  failures, PNG-save failure cleanup, changes during rendering/saving, immediate
  mutation/conflict invalidation, cross-instance disk reuse, thumbnail dimensions, map identity,
  legacy/corrupt disk cache, API response pixels/revisions/errors, and concurrent
  edit/preview serialization with a captured response surviving invalidation.
- `npm ci`, `npm test`, and `npm run build` in `tools/map_editor_web`: passed;
  **12 frontend tests**. Two new tests render the actual React graph node, picker,
  and inspector and verify their changed image URLs after dependency refresh and
  missing-dependency repair. Existing ten conflict/undo/recovery tests still pass.
- `bash scripts/map-editor.sh check`: passed, **54 maps and 108 portals**.
- A read-only production smoke check resolved dependency revisions for all
  **54 maps** and rendered Ardel thumbnail/full PNGs into a disposable cache.
- `git diff --check`: passed. RF-08's workspace/clippy/scenario baseline above
  remains the recorded runtime baseline; this task changes no Rust, TMX writer,
  canonical assets, validation baseline, screenshot oracle, or CI configuration.

Raw logs are ignored under `temp/refactoring/rf09-*`. Backend ASGI/thread tests
and frontend Node worker tests ran with normal local process access, following
RF-07's established sandbox diagnosis. API/concurrency test waits are bounded;
fixtures and production smoke caches are disposable. The static Ardel oracle
mismatch recorded under RF-08 remains unchanged; these headless PNG/React checks
do not establish interactive browser or campaign acceptance. No refactoring
plan or sibling tracker was edited or staged.

## RF-10 — Persistence and replay regression coverage

The interrupted tests now compile and run against the existing native save and
normalized-input implementations. Runtime changes are confined to test modules
and assertions. The recorded RF-08 workspace/scenario baseline above remains the
prior runtime baseline; the final RF-10 snapshot passes the checks below.

The authored dialogue fixture uses three fresh OS processes to seed native
saves, advance the dialogues, and reopen saved outcomes. It exercises Pike's
disabled vouch choice, cancellation/reopen, pledge, enabled vouch reward, and
turn-back branch. Elder Maeve's Act II advancement and Hesta's Flame-glyph
completion apply their item rewards once; two conversations after a process
restart leave the complete saved payload unchanged. Six synthesized campaign
boundaries round-trip accumulated flags, maps/visited history, and RNG state;
loading under the wrong scenario identity fails without rewriting the save.
These synthesized states establish serialization coverage, not campaign traversal.

The replay fixture records three seeded encounter choices, physical victories,
and production EXP/GP/loot transactions. Separate processes repeat the same seed,
run a different seed, replay while suppressing physical gameplay input, and
exercise corrupted copies. Matching actions/seed reproduce every hash and the
final native save; the different seed changes gameplay state even after its RNG
cursor is excluded from the comparison. Each battle rejects a second reward
application without mutation. Two corrupt hashes after a valid prefix stop at
the first divergence, **action 1 at frame 3**, with an unsuccessful exit request.
Malformed input fails decoding, and replay preserves both original and copied
input bytes.

Additional headless tests drive the production battle input handler through
victory/reward acknowledgement, defeat, successful/failed/blocked fleeing, and
item/ability target cancellation. They check scene routing, restored map state,
one-time rewards/flags, and complete saved-payload equality for nonrewarding and
canceled paths. Existing Warp tests now compare complete saved payloads for
cancellation, a busy transition, and an unlock lost during animation; accepted
Warp still charges its MP cost once. Late destination asset failure is outside
those Warp assertions and remains an operator observation.

[The operator checklist](persistence-replay-checklist.md) supplies isolated save
and options directories, reset commands, generated slot meanings, natural
campaign-boundary steps, and visible battle/transport/replay checks. The opt-in
export produced **15 native save fixtures** in a fresh ignored directory. The
four before-dialogue saves have start positions validated through production
TMX collision checks. Generated fixtures remain separate from natural campaign
saves. Child workers have 30-second timeouts, retain failure logs, and clean up
successful intermediate files.

Validation on the final source snapshot:

- `cargo test --workspace -j 1`: passed, **1,077 tests** (196 content unit tests,
  879 engine unit tests, and two integration tests; zero failed/ignored). Seven
  new engine test entries comprise five parent regressions and two child-worker
  entrypoints; the existing Warp tests gained saved-state assertions.
- `cargo clippy --workspace --all-targets -j 1 -- -D warnings`: passed.
- `cargo fmt --all -- --check` and `git diff --check`: passed.
- `cargo run -j 1 -- validate-scenario rusted_kingdoms --baseline
  assets/scenarios/rusted_kingdoms/validation-baseline.txt`: baseline **PASS**, four
  accepted undefined ultimate-element flags, zero new or obsolete diagnostics.
  The ungated report continues to say FAIL for those four accepted errors.
- `map-sweep rusted_kingdoms`: all **54 maps** clean with zero findings; headless
  runtime loading passed all 54.
- `dialogue-sweep rusted_kingdoms`: passed, **128 documents**, 304 roots and
  318 terminating paths, zero cycles/errors.
- `encounter-sweep rusted_kingdoms`: passed, **19 zones**, 158 constructions and
  119 loaded assets, zero construction/asset failures.
- The documented absolute-path fixture export passed its focused test and
  produced slots 1–4, 10–15, and 21–25 without touching a normal save profile.
- `python3 -m unittest -v scripts/test_check_asset_rights.py`: five tests passed.
  The rights report has zero missing entries, hash mismatches, or ledger errors;
  its **327 existing unresolved rights entries** remain release blockers.

Raw logs and generated save evidence are ignored under `temp/refactoring/rf10-*`.
Cargo reused the existing shared cache with required process/filesystem access;
no cache cleanup, dependency change, or feature upgrade was performed. RF-10 is
removed from the refactoring queue. The canonical manual tracker remains
unchanged: service/dialogue appearance, natural campaign-to-credits, live
deterministic replay, and packaged campaign acceptance still require user
observations. Headless automation does not complete those gates.

## Save-notice follow-up — Automatic success dismissal

Successful autosave and Options notices previously remained visible until a mouse
click on Dismiss. They now expire after three seconds of real time, matching
service-message timing. New successes restart the lifetime; failures keep their
explicit Retry/Dismiss controls and stored error state. Expiry runs before
pointer handling so hidden notices cannot consume clicks.

Two regressions reproduced the indefinite notice against `def8b25`, then passed
with the fix. They cover both notice kinds, repeated reports, persistent failures,
rendered panel visibility, and pointer input on the expiry frame. Final checks:
`cargo test --workspace -j 1` passed **1,079 tests** (196 content, 881 engine, two
integration), strict workspace/all-targets clippy passed, and formatting/diff
checks passed. Logs are ignored under `temp/refactoring/autosave-notice-*`.
Live-window appearance remains an operator observation.

## RF-11 — Gate editor regressions and frontend builds in CI

The CI workflow now has a separate `editor-checks` job on its existing push and
pull-request triggers. It creates the editor virtual environment from the pinned
`tools/requirements.txt`, runs the existing backend suites with dummy SDL video
and audio drivers, installs the frontend with `npm ci`, runs its behavioral
suite, typechecks/builds it, and checks the production editor graph. The existing
Rust/workspace, clippy, asset-rights and scenario-baseline job remains identical.
Editor dependencies stay in developer CI; runtime and release packaging paths
are unchanged.

The focused suites from RF-03, RF-06, RF-07 and RF-09 already cover atomic TMX
publication and durability failures, geometry/collision, concurrent edits and
stale revisions, graph consistency, render invalidation, and browser conflict
recovery/preview updates. RF-11 wires those suites directly into CI without
adding duplicate tests. Each install, test and build is a normal failing step;
no skip condition or `continue-on-error` bypasses the new checks. The job has a
15-minute timeout and caches pip/npm downloads using their dependency files.

Toolchain versions are explicit: Python 3.13 and Node 24. The setup inputs and
cache paths were checked against the official
[setup-python](https://github.com/actions/setup-python) and
[setup-node](https://github.com/actions/setup-node) documentation. The editor
[local verification guide](../tools/map_editor_web/README.md#ci-and-local-verification)
documents the same commands for developers.

Validation:

- Backend baseline and final CI command: **57 tests passed** in each run with
  temporary TMX/TSX/PNG fixtures and real ASGI/thread behavior.
- Frontend baseline and final `npm ci`, `npm test`, `npm run build`: passed;
  **12 behavioral tests**, TypeScript check and Vite build. The lockfile is
  unchanged. npm's existing audit reports five findings (two moderate and three
  high); this task does not upgrade dependencies.
- `bash scripts/map-editor.sh check`: passed, **54 maps and 108 portals**.
- Binary-only resolution/download of `tools/requirements.txt` for CPython 3.13
  on Linux x86-64 succeeded, including native pygame and pydantic-core wheels.
- Workflow YAML and each new shell command parsed successfully. A structural
  comparison verified that the existing checks job, triggers, permissions and
  concurrency configuration are unchanged; runtime/release manifests, code,
  assets and package scripts have no RF-11 diff.
- `git diff --check`: passed. The immediately preceding notice fix passed
  **1,079 workspace tests**, strict workspace/all-targets clippy and formatting;
  RF-11 changes no Rust or scenario content.

Local test execution used Python 3.14.7 and Node 26.11.1. The hosted Python
3.13/Node 24 job will run after push; a hosted Actions result is not claimed.
Raw logs/downloads are ignored under `temp/refactoring/rf11-*`. RF-11 is removed
from the queue; the sibling manual tracker remains unchanged. Interactive editor
and full-campaign acceptance still require their existing operator observations.

## RF-21 — Remove unused root-package dependencies

The root `rpg-s1` package now declares only `rpg-engine`. `src/main.rs` calls
only `rpg_engine::run_with_game_version`, the root integration guard uses only
the standard library, and the root has no build script, examples or benches.
Its `rpg-content` mentions are file-path strings, not crate imports. Bevy,
quick-xml, crc32fast, serde, serde_path_to_error, serde_yaml_ng, sha2 and
rpg-content remain declared by the crates that use them.

`rpg-engine` already requests the identical Bevy feature list, so feature
unification is unchanged: a sorted `cargo tree --workspace -e normal,build`
package/feature listing differs only by the root's duplicate `sha2 (*)`
marker, and `cargo tree -e features -i sha2` still resolves `default,std`
through `rpg-engine`. The lockfile change only removes the eight entries from
the root package's own dependency list; no version changed.

Validation:

- `cargo fmt --all -- --check`, strict workspace/all-targets clippy and
  `cargo check --workspace --all-targets --locked`: passed.
- `cargo test --workspace --no-fail-fast`: **1,075 passed, 4 failed**. All four
  failures (`audits_every_shipped_csv_tile_layer`,
  `pinned_ardel_known_blocked_and_open_cells_match_source` and two
  `real_ardel_*` ground tests) read `town_01_ardel.tmx`, which had an unrelated
  uncommitted edit in the working tree. The same four fail identically with
  the original root manifest restored, so they are not caused by RF-21.
- Scenario validation against the accepted baseline: PASS (4 accepted, 0 new).
  `map-sweep`: PASS, 54/54. The dev binary launches and prints its usage.

- `cargo build --release --locked`: passed. A payload staged as the release
  workflow does (binary plus `scripts/package_release_assets.py`, 1,001 assets)
  passed validation, map, encounter and dialogue sweeps from an unrelated
  working directory with audio muted.

RF-21 is removed from the queue.

## RF-12 — Split the field-menu controller by responsibility

`field_menu.rs` went from 4,502 lines (2,023 before its test module) to an
80-line entry module holding the module list, plugin registration and ordering,
the World-entry reset, and two party lookups. The controller now lives in
focused modules under `field_menu/`:

- `state.rs`: screens, modes, `FieldMenuState`, back navigation, and cursor
  wrapping. `markers.rs`: UI marker components. `assets.rs`: the scenario
  font, status portraits, and party walk sheets.
- `input.rs`: the input system. It keeps the open/close, message-clearing, and
  sound-cue logic, reads one `MenuInput` snapshot per frame, and dispatches by
  screen. Quests and Recipes, which only scroll, stay inline there.
- One handler module per screen, each owning its constants and helpers:
  `main_deck.rs` (command deck, character switch, quit), `status.rs`,
  `inventory.rs` (items, targeting, discard, tags, show/hide manager),
  `equipment.rs`, `spells.rs` (heal targeting, teleport), `battle.rs`
  (tactics, battle options and their persistence, log), and `save.rs`.

Each former `(screen, mode)` match arm became an arm of its screen's handler
with its body unchanged, apart from reading input from the snapshot. Handlers
receive the `ResMut` wrappers rather than plain references, so `GameState`,
`FieldMenuState`, `TransportUiState`, and the save catalog are still marked
changed only where the old code mutated them. A disposable probe confirmed this:
arrow navigation through every screen, including the equipment slot and picker
columns, left `GameState`'s change tick untouched, while a confirmed row change
moved it. The probe was removed. Constants used only by the UI (layout widths,
card heights, visible-row budgets, `inventory_window`) moved to `ui.rs`.
`ui.rs` now holds the UI's glob imports of the controller modules, so the new
modules use explicit imports and the page modules are unchanged. External
paths (`field_menu::FieldMenuState`, `FieldMenuPlugin`) are preserved.

Tests moved rather than changed. 19 controller unit tests now sit in their
owning modules: state 4, main deck 6, inventory 6, status 1, equipment 1,
battle 1, and `ui.rs` 1. The 44 UI-rendering and whole-system input tests share
fixtures and moved to `field_menu/tests.rs`. A misplaced doc comment
describing the recipe-book test, which sat above the equip-refusal test since
that test was inserted, was moved back to the recipe test.

Validation:

- `cargo fmt --all -- --check` and strict workspace/all-targets clippy: passed.
- `cargo test --workspace --no-fail-fast`: **1,075 passed, 4 failed**, the same
  as the pre-split baseline. A comparison of test function names and results
  before and after is identical: all 71 field-menu tests still exist and pass.
  The 4 failures are the `town_01_ardel.tmx` working-tree edit noted under
  RF-21.
- Scenario validation against the accepted baseline: PASS. `map-sweep` 54/54,
  `dialogue-sweep` and `encounter-sweep`: PASS.

The game was not driven interactively for a visual spot check; the menu's
navigation, save, teleport, and quit paths are covered by the moved
whole-system input tests. RF-12 is removed from the queue.

## RF-13 — Split World encounter loading and battle handoff

`world_encounter.rs` went from 3,040 lines (1,738 before its test module) to an
89-line facade holding the two plugins with their unchanged system ordering, the
shared tile size, and re-exports of the crate-visible types. Its code now lives
under `world_encounter/`:

- `state.rs`: encounter status and state, pending and restored enemies,
  `WorldEnemy`, and the World enter/exit lifecycle systems.
- `loading.rs`: encounter asset requests and transactional publication,
  spawn tiles, and the boss spawn.
- `simulation.rs`: respawn cadence, chase, wander, collision checks, and enemy
  sprite geometry.
- `handoff.rs`: contact detection, scripted battles, battle balance, and the
  flash transition into Battle.
- `fixtures.rs`: the session-only debug battle fixtures.
- `presentation.rs`: the battle-entry camera and battle BGM with its fade.

Function bodies moved verbatim. A line-level comparison found every moved code
line in the original file apart from added `pub(super)` visibility and five
rustfmt rewraps. The plugin registration, `WorldPublicationSet` membership,
pause gating, and transition-settled conditions are unchanged, so ECS ordering
and RNG call order are preserved. External paths are unchanged:
`BattleTransition`, `ScriptedBattleRequest`, `BattleBgmFadeOut`,
`WorldEncounterRestore`, `WorldEncounterState`, and `WorldEnemy` are re-exported.
One prose reference in `world_actor.rs` was updated to the moved
`enemy_top_left` path.

Tests moved rather than changed: fixtures 1 (with its two helpers), handoff 1,
and simulation 3. The five headless production and restore integration tests
stay together in `world_encounter/tests.rs`.

Cross-build determinism: a disposable probe booted the production first-zone
harness (new game, Starting Forest, encounter assets published), ran 300 more
frames, and dumped every World enemy's id, formation, and tile plus the full
save payload, including the RNG state. The probe output from the original file
(the split paths stashed) and from the split was byte-identical. The probe was
removed. That harness has no World collision resource, so enemy movement did
not run in it; seeded chase and wander behaviour is covered by the moved
simulation tests. Battle-return restoration is covered by the
restored-hidden-enemy test and the boss handoff tests.

Validation:

- `cargo fmt --all -- --check` and strict workspace/all-targets clippy: passed.
- `cargo test --workspace --no-fail-fast`: **1,079 passed**. Test function
  names and results before and after are identical.
- Scenario validation against the accepted baseline: PASS. `map-sweep` 54/54,
  `dialogue-sweep` and `encounter-sweep`: PASS.

RF-13 is removed from the queue.

## RF-14 — Split scenario cross-reference validation

`scenario_cross_reference.rs` went from 3,539 lines (2,404 before its test
module) to a 51-line facade: the module documentation, the module list, the
re-exported public report types, and the unchanged
`validate_scenario_directory` entry point. The code now lives under
`scenario_cross_reference/`:

- `report.rs`: the one diagnostic type, locations, catalog counts, and the
  deterministic severity/location/code/message sort.
- `context.rs`: the validator, its typed catalogs, the diagnostic and
  reference-counting helpers, path probing, and the fixed-order `validate`
  orchestrator.
- `loading.rs`: typed catalog loading, the contained YAML walk, and count
  aggregation. `index.rs`: the reference index.
- Domain validators as further `impl Validator` blocks: `manifest.rs` (manifest
  ids, duplicate ids), `maps.rs` (maps, transport, landing tiles), `dialogue.rs`
  (dialogue actions, flags, quest flag edges), `combat.rs` (party, classes,
  enemies, enemy AI and sprites, encounters), `items.rs` (items, recipes), and
  `audio.rs`.

Every item moved verbatim. A line-level comparison found every production line
in the original file apart from added `pub(super)` visibility, given only to
items another module references. No generic YAML reparsing was added, and the
accepted validation baseline is unchanged. The public API used by the engine
CLI is re-exported unchanged. The 20 validator tests stay together in
`scenario_cross_reference/tests.rs`, because they all drive the whole validator
through the shared invented-scenario fixture.

Report equivalence:

- Production: the complete `validate-scenario rusted_kingdoms` output is
  byte-identical before and after, ungated (exit 1) and gated by the accepted
  baseline (exit 0).
- Malformed fixtures: a disposable probe, added identically to the original
  and the split test modules, built the 11 invented parity cases plus the duel
  enemy mutation and dumped each complete report (`{:#?}`). The 12 reports were
  byte-identical. The probe was removed.

The first probe run caught a defect in the extraction itself: un-indenting the
test module also stripped indentation inside multi-line YAML string literals,
which made the invented manifest invalid. The test module was regenerated
without un-indenting, leaving rustfmt to re-indent code only. A literal-aware
scan confirmed that all 152 whitespace-significant literal lines are preserved
exactly. The same scan found no whitespace-significant literal lines in the
original RF-12 (`field_menu.rs`) and RF-13 (`world_encounter.rs`) test modules,
so their un-indented test files are unaffected.

Validation:

- `cargo fmt --all -- --check` and strict workspace/all-targets clippy: passed.
- `cargo test --workspace --no-fail-fast`: **1,079 passed**. Test function
  names and results before and after are identical.
- `map-sweep` 54/54, `dialogue-sweep` and `encounter-sweep`: PASS.

RF-14 is removed from the queue.

## RF-15 — Separate CLI parsing, execution, and reporting

`cli.rs` went from 3,200 lines (1,798 before its test module) to a 75-line
facade: the exit codes, `run`, the production validator, and the test entry
point, with `PlayArguments` and `production_asset_base` re-exported for their
existing callers. The code now lives under `cli/`:

- `args.rs`: usage text, validated command types, and all argument and
  environment parsing. Nothing here touches the filesystem.
- `execute.rs`: the command dispatcher and the record, replay, debug-launch,
  Python-import, and validation runners.
- `packages.rs`: package selection beneath the bounded collection and
  asset-base resolution, still in one place.
- `report.rs`: every output formatter (validation, baseline gate, map and
  dialogue reports, sweeps, debug-launch log).

Items moved verbatim; the only differing lines are rustfmt rewraps. The
dispatcher keeps its short inline status lines ("Gameplay seed", "Recording…",
import results), because play, record, and replay print slightly different sets
and a shared helper would have changed that contract. Injected test IO, the
injected validator, and the test collection root are unchanged.

CLI equivalence: 33 representative invocations were captured before and after
from the built binary, run from a scratch directory with
`RPG_S1_SAVE_DIR` pointed at it. They covered help (`--help`, `-h`, unknown
`help`), unknown commands, validation ungated and gated, missing or unreadable
baselines, unknown and path-like package keys, extra arguments, map and
dialogue reports, all three sweeps, invalid play and debug options, record and
replay argument and file errors, and import argument, slot, missing-file, and
malformed-input errors. Every stdout, stderr, and exit code was identical.
No import wrote a save in either run.

Tests moved rather than changed. Eight tests that call private helpers now sit
with them: the baseline-argument parser test in `args.rs`, the import-path and
package-symlink tests in `execute.rs`, and the five asset-resolution tests in
`packages.rs`. These use the shared `TempCollection` helper, now `pub(super)`.
The 35 command-level tests stay in `cli/tests.rs`. The test module was moved
without un-indenting, and a literal-aware check confirmed all 29
whitespace-significant string-literal lines are preserved exactly.

Validation:

- `cargo fmt --all -- --check` and strict workspace/all-targets clippy: passed.
- `cargo test --workspace --no-fail-fast`: **1,079 passed**. Test function
  names and results before and after are identical.

RF-15 is removed from the queue.

## RF-16 — Deduplicate verified atomic-file mechanics

Two commits: a behaviour-preserving extraction, then a separate correction.

**Extraction.** New `crates/rpg-engine/src/atomic_file.rs` owns the one
filesystem sequence the save store, the options store, and the Python importer's
slot backup each implemented: a uniquely named temporary file beside the
destination with exclusive creation, write, sync, read-back, caller
verification, publication, and directory sync, plus the shared cleanup guard.
`PublishPolicy::Replace` renames; `PublishPolicy::CreateNew` hard-links the
verified inode (RF-01's no-clobber guarantee, including dangling symlinks, with
no fallback to replacement) and then unlinks the temporary name. Callers keep
their own rules: save slot bounds, the early existence check, encode/decode
checksum and schema verification; options round-trip verification; importer
backup naming, the existing-backup check, and byte-equality verification. A
single hook runs before the write, sync, publish, and directory-sync steps, and
carries the stores' existing fault injection and the save store's
`before_replace` race hook unchanged. Each caller maps steps back to its
existing operation names, so every error variant and message is unchanged. All
existing tests passed unchanged, including RF-01's late-creation, dangling
symlink, failed-publication, and interruption tests, the options and import
fault tests, and RF-05's retry tests. Five unit tests cover the module. The
options store's rejected-file backup keeps its own exclusive-create name probing,
because it publishes directly under a free final name rather than through a
temporary file. The importer's transient temporary name changed from
`.{slot}-{digest}.{pid}-{n}.tmp` to the shared `.{slot}-{digest}.{pid}.{n}.tmp`.
No save or options format changed.

**Correction.** The module reports `NotPublished` (the destination is exactly
as before) separately from `NotDurable` (the destination already holds the
verified bytes, but unlinking the temporary name or the directory sync failed).
`SaveStoreError::NotDurable` and `OptionsStoreError::NotDurable` carry this, with
`published()`. Their messages read exactly like the previous I/O errors, so
existing wording is unchanged. This replaces RF-05's string matching on
"sync save directory" and "sync config directory", which missed a failure to
unlink the temporary name after a `CreateNew` link. Before this change, three
callers misreported a published slot:

- Autosave said the previous save was preserved after a post-link unlink
  failure. It now reports the published-but-unconfirmed state for every
  post-publication failure.
- The field-menu Save showed a plain error, stayed on the overwrite prompt, and
  did not refresh the slot list, although the slot held the new save. It now
  returns to browsing, refreshes the list, and reports that the slot was saved
  but durability could not be confirmed. A regression test fails against the
  old behaviour (it stays on `SaveConfirm`) and passes with the fix.
- `import-python-save` reported "import installation failed". It now reports
  `007.yaml was imported, but disk durability could not be confirmed: …`
  through a new `PythonImportError::NotDurable` (exit status unchanged).

A failed post-publication step of the importer's backup still stops the import
before the slot is written; that conservative rule stays with the importer.

Validation:

- `cargo fmt --all -- --check` and strict workspace/all-targets clippy: passed.
- `cargo test --workspace --no-fail-fast`: **1,089 passed**: the 1,079 existing
  tests, 5 `atomic_file` unit tests, and 5 regressions (save store not-durable
  with and without overwrite, pre-publication failures never published, options
  store, importer, and field-menu Save).

Real power loss and filesystem-specific sync failures were not simulated;
failures were injected at the same steps as RF-01 and RF-05. RF-16 is removed
from the queue.

## RF-17 — Deduplicate repeated menu rendering patterns

Two commits, plus a visual spot check on 2026-10-16.

**Structure (`239d0c4`).** `menu_chrome::visible_rows(first, len, rows)` yields each
visible row's index with the `ScrollFades` its card needs. It replaces 14 hand-written
window loops (compute `shown`, `skip`/`take`, `ScrollFades::for_row`) in the field
menu's items list, tag editor, show/hide manager, equipment picker, spellbook, quests,
recipes, and Save page, the four service list pages, the Ember Atlas destination list,
and the title Load picker. The spellbook and equipment picker now use the shared
`window_start` instead of an equivalent inline formula. Both save screens share
`save_ui::spawn_save_slot_summary` (a byte-identical duplicate before) and
`spawn_save_message`, whose border colour remains each screen's own. Row heights, label
widths, the Load picker's LATEST badge, and every page layout stay page-specific. The
three modal backdrops keep their distinct dimming.

**Colours (this commit).** `menu_chrome::selectable_card_colors(selected)` replaces 14
copies of the selected/unselected card background and border. The copies had drifted:
selected background alpha 224 in the two save cards versus 218 elsewhere, and resting
border alpha 80 (three), 95 (two), and 96 (one) versus 90 (eight). The helper uses the
majority 218/90, so six cards change by at most 16/255 alpha on a faint border.

Equivalence: a disposable probe rendered every affected page into a plain `World` and
dumped the full entity tree (component names, `Node`, background, border, text, font
size, text colour, visibility). It covered 80 cases: eight field-menu pages and modals
at five cursor positions with a 40-item inventory, the field Save page and overwrite
prompt at six positions over 101 varied slots, the title Load picker at six positions,
four service pages at four positions, and the atlas list over 20 destinations.
Scrolled windows and edge fades were exercised throughout (about 6,000 fade-node
lines). The structural commit's trees were byte-identical to the original (about
38,000 lines). The colour commit's trees differed only in 884 background/border lines,
with no other change. The probe was removed.

Validation:

- `cargo fmt --all -- --check` and strict workspace/all-targets clippy: passed.
- `cargo test --workspace --no-fail-fast`: **1,090 passed** (1,089 before plus a
  `visible_rows` unit test).

Visual spot check (2026-10-16, real window on an unlocked display): the title Load
picker, field menu, Items (pouch list, item list scrolled to row 16 of 65, empty pouch),
Status, Quests (scrolled), field Save (scrolled to slot 11), and the Magic Core Exchange
service list and quantity modal all rendered correctly. Selection highlights, slot and
row counters, scroll-to-selection, and edge fades behaved as before; a single Enter after
the shop dialogue opened the list without auto-confirming.
