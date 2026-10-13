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
claimed. [Runtime hot-reload instructions](runtime-hot-reload.md) document the
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
