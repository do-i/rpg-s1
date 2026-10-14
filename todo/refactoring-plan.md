# Ranked refactoring plan

Prepared: 2026-10-11. Reviewed branch: `dev`, commit `ac5d827`.

This plan covers the Rust engine/content crates, the map editor, and their checks. Fix data loss and gameplay blockers first, then simplify code, remove proven dead code, improve measured performance, and trim comments. Each task is a separately reviewable change with an assigned AI model and a completion condition. Execution is in progress; completed tasks are recorded in refactoring-evidence.md and removed from this queue.

## Execution rules

- **Higher rank executes first.** Select the highest-ranked unfinished task whose dependencies are complete. Dependencies override rank; independent tasks do not require all earlier tasks to finish.
- **Severity:** P0 = data loss or a hard gameplay blocker; P1 = incorrect behavior, persistence failures, or a missing regression gate; P2 = maintainability or measured performance; P3 = editorial cleanup. Severity describes impact if triggered, not frequency.
- **Evidence:** Reproduced = observed with a disposable fault/invalid-input probe; Source = a specific implementation path was inspected, but no full-game reproduction was run; Investigation = establish the problem before changing behavior. A suspected bug may close with evidence that it is already handled.
- Preserve save/options formats, scenario boundaries, deterministic RNG ordering, source-parity behavior, and existing diagnostics unless the task explicitly requires a reviewed correction. Bug fixes and structural moves should be separate commits.
- Before implementation, record the baseline workspace tests, clippy, scenario validation, and applicable editor checks. Add focused tests for identified failures; reuse existing tests for mechanical moves and comment edits.
- Remove code only after checking production callers, tests, Bevy plugin/system registration, supported CLI paths, and serialized compatibility. An unused accessor does not establish that its underlying state is unused.
- Performance changes require a reproducible before/after comparison. Keep an optimization only when the gain exceeds run-to-run variation and correctness remains stable.
- Keep runtime/content assets out of code-deletion tasks. Existing asset-removal review remains separate, with exact scope and owner approval.

## Model assignments

Assignments use the models exposed in this session. They are task-fit judgments, not repository-specific benchmark results. OpenAI describes [GPT-6 Astra](https://developers.openai.com/api/docs/models) as the model for demanding reasoning and coding, GPT-6.1 Sol as a balance for complex work, and GPT-6 Luna as efficient for focused tasks. The [official model-selection guidance](https://developers.openai.com/api/docs/guides/model-selection) recommends evaluating models on representative work rather than assuming one model wins every task.

- **Astra** = `gpt-6-astra`: multi-system failure recovery, persistence correctness, uncertain compatibility, and performance architecture.
- **Sol** = `gpt-6.1-sol`: scoped Rust/Bevy refactoring, editor repairs, test integration, and deduplication.
- **Luna** = `gpt-6-luna`: bounded manifest cleanup and comment edits with clear preservation rules.

Use the effort specified per task. Escalate a task to Astra if reproductions expose cross-system behavior that exceeds its original scope. These assignments do not authorize automatic task execution or model changes in the current session.

## Ranked queue

All tasks listed below are pending; completed tasks have been removed. A dash in the dependency column means no task prerequisite beyond the baseline checks.

| Rank | ID | Severity | Task | AI model | Effort | Depends on |
| ---: | --- | --- | --- | --- | --- | --- |
| 770 | RF-15 | P2 | Separate CLI parsing, execution, and reporting | Sol | high | RF-10 |
| 760 | RF-16 | P2 | Deduplicate verified atomic-file mechanics | Astra | high | RF-01, RF-04, RF-05 |
| 750 | RF-17 | P2 | Deduplicate repeated menu rendering patterns | Sol | high | RF-12 |
| 740 | RF-18 | P2 | Reuse typed catalog-loading operations | Sol | high | RF-08, RF-12, RF-13, RF-14 |
| 700 | RF-19 | P2 | Resolve obsolete intro-transition staging state | Astra | high | RF-02, RF-10 |
| 690 | RF-20 | P2 | Remove proven-unused accessors and stale suppressions | Sol | medium | RF-05, RF-08, RF-19 |
| 600 | RF-22 | P2 | Establish reproducible performance measurements | Sol | high | RF-10 |
| 590 | RF-23 | P2 | Avoid repeated World flag allocation and respawning | Sol | high | RF-08, RF-13, RF-22 |
| 580 | RF-24 | P2 | Reduce menu rebuild and list allocation costs | Sol | high | RF-12, RF-17, RF-22 |
| 570 | RF-25 | P2 | Reduce save/autosave stalls without weakening durability | Astra | xhigh | RF-01, RF-05, RF-16, RF-22 |
| 560 | RF-26 | P2 | Optimize map load/render work demonstrated by profiles | Astra | high | RF-02, RF-08, RF-22 |
| 400 | RF-27 | P3 | Trim historical and redundant code comments | Luna | medium | RF-12, RF-13, RF-14, RF-15, RF-19, RF-20 |

## Code cleanup and deduplication

### RF-15 — Separate CLI parsing, execution, and reporting

**Evidence: Source.** [cli.rs](../crates/rpg-engine/src/cli.rs) has 3,200 total lines and contains command parsing, launch routing, import/sweep/validation behavior, formatting, and tests.

- Keep validated command types and argument parsing separate from command runners and output formatting. Maintain the current help/usage contract and exit codes.
- Keep packaged scenario-root resolution centralized. Preserve record/replay modes, debug launches, and injected test IO rather than duplicating argument logic.
- **Done when:** existing CLI tests pass and representative help, validation, import, and sweep outputs/exit codes are equivalent.
- **Model fit:** Sol/high for compatibility-conscious CLI decomposition.

### RF-16 — Deduplicate verified atomic-file mechanics

**Evidence: Source.** [save_store.rs](../crates/rpg-engine/src/save_store.rs) and [options_store.rs](../crates/rpg-engine/src/options_store.rs) independently implement temporary naming, exclusive creation, write/sync, reread, cleanup guards, replacement, and directory synchronization. The importer has related backup mechanics.

- Extract only the proven common filesystem operations, with caller-supplied validation and explicit replace/no-replace policy. Retain save-specific checksum/schema validation and importer-specific backup rules in their owners.
- Distinguish failure before publication from a directory-sync failure after publication so callers do not misreport the slot's actual state.
- Carry RF-01's race/fault tests through the extraction. Avoid a general storage framework or cross-language abstraction.
- **Done when:** there is one shared Rust publication mechanism with all durability and no-clobber guarantees intact.
- **Model fit:** Astra/high because the abstraction must preserve data-integrity distinctions.

### RF-17 — Deduplicate repeated menu rendering patterns

**Evidence: Source.** [field_menu/ui](../crates/rpg-engine/src/field_menu/ui), [service_ui/view.rs](../crates/rpg-engine/src/service_ui/view.rs), and [save_ui.rs](../crates/rpg-engine/src/save_ui.rs) repeat panel, row, selection, message, and scrolling patterns. Shared menu helpers already exist.

- Identify repeated structures before extracting them. Extend existing `ui/shared.rs` and `ui_theme.rs` for compatible panel/row/selection helpers; keep page-specific behavior in each page.
- Consolidate shared save-slot presentation and compatible window-index calculations. Avoid a generic widget framework or one helper with many boolean options.
- Preserve keyboard navigation, row budgets, scroll-to-selection, modal placement, and distinct page layouts.
- **Done when:** targeted duplicated implementations are replaced by clear helpers and affected layouts/navigation pass existing checks plus a visual spot check.
- **Model fit:** Sol/high for reuse that respects different UI ownership and layouts.

### RF-18 — Reuse typed catalog-loading operations

**Evidence: Investigation.** [field_menu_domain.rs](../crates/rpg-engine/src/field_menu_domain.rs), [encounter_assets.rs](../crates/rpg-engine/src/encounter_assets.rs), [scenario_new_game_assets.rs](../crates/rpg-engine/src/scenario_new_game_assets.rs), and the World loaders contain related load/readiness/error paths.

- Inventory genuinely identical typed parse/validate/path operations and repeated catalog indexes after the earlier splits.
- Extract small shared operations and reusable immutable catalog views. Retain subsystem-specific readiness, failure, and publication state machines.
- Ensure shared caches are scoped by scenario identity and asset revision, consistent with RF-08. Preserve detailed error ownership and hot-change behavior.
- **Done when:** demonstrably duplicated loading/indexing code is removed without combining unrelated lifecycle state machines.
- **Model fit:** Sol/high for typed reuse with clear boundaries.

## Dead code and unused dependencies

### RF-19 — Resolve obsolete intro-transition staging state

**Evidence: Source; behavior decision required.** [intro_transition.rs](../crates/rpg-engine/src/intro_transition.rs) retains `PendingWorldTransition` and multiple accessors described as needed by a “future World renderer.” The current World renderer exists; the repository search found no production consumer of `get`/`take`. The staged fade could represent an unimplemented parity requirement rather than disposable state.

- Verify the authored intro fade contract and compare it with the current World fade behavior.
- If the arrival/fade must be consumed, connect it to the existing transition lifecycle and test that behavior. If equivalent arrival state is already applied elsewhere, remove only the redundant resource/accessors and update tests to assert observable arrival.
- If this reveals a real intro behavior bug, promote that fix into the P1 queue before deleting the staging state.
- **Done when:** no stale “future renderer” scaffold remains and authored map/position/facing/fade behavior has an explicit tested owner.
- **Model fit:** Astra/high to distinguish dead scaffolding from a missing feature.

### RF-20 — Remove proven-unused accessors and stale suppressions

**Evidence: Source.** Test-facing `dead_code` suppressions occur in [world_audio.rs](../crates/rpg-engine/src/world_audio.rs), [world_encounter.rs](../crates/rpg-engine/src/world_encounter.rs), [scenario_manifest_asset.rs](../crates/rpg-engine/src/scenario_manifest_asset.rs), and intro transition code. Some suppressed state should become real diagnostics under earlier bug fixes.

- Inventory each suppression with production callers, test callers, and the purpose of the underlying state. Remove unused private helpers; use `#[cfg(test)]` for accessors that exist solely for tests.
- Retain production diagnostics wired by RF-05/RF-08 and serialized fields required for compatibility. Narrow broad impl-level suppressions rather than hiding new warnings.
- Check module/plugin registration before removing apparently unreferenced Bevy code.
- **Done when:** each remaining suppression has a current justification, removable code is gone, and workspace clippy passes without new broad allowances.
- **Model fit:** Sol/medium for a bounded audit with caller and compatibility checks.

## Execution performance

### RF-22 — Establish reproducible performance measurements

**Evidence: Existing measurement infrastructure and unfinished M14.07 acceptance.** [frame_timing.rs](../crates/rpg-engine/src/frame_timing.rs) reports selected system averages/maxima; the [manual tracker](../../rpg-s1-plans/manual-verification.md#release-performance) still requires release startup, frame pacing, and memory evidence.

- Capture a release-build baseline for startup, settled World idle/movement, dense NPC/enemy maps, menu navigation, battle effects, map transitions, save/autosave, and memory after repeated transitions.
- Record build/configuration, hardware, display/backend, seed, map/party state, warm-up, repetitions, median and p95/p99 frame times, worst hitches, and peak memory. System average/max reports alone are insufficient for tail latency.
- Define workload-specific comparison thresholds before optimizing. Retain raw measurements and scripts under ignored `temp/`, with reproducible commands in the evidence summary.
- Developer measurements may proceed before a rights-approved package exists. Formal M14.07 acceptance still depends on M14.08 and owner disposition of regressions.
- **Done when:** later tasks have reproducible before/after inputs and declared budgets; no improvement is claimed from unmeasured code appearance.
- **Model fit:** Sol/high for practical instrumentation and measurement design.

### RF-23 — Avoid repeated World flag allocation and respawning

**Evidence: Source; cost unmeasured.** [world_actor.rs](../crates/rpg-engine/src/world_actor.rs), `sync_world_actor_request`, allocates/clones every flag before comparing with previous flags. Any flag change despawns and reloads all NPCs, including changes unrelated to NPC presence.

- Profile flag snapshot allocation and the equivalent object/encounter paths. Introduce a narrow flags revision or borrowed comparison only if it demonstrably helps.
- Track presence-relevant conditions and update changed entities rather than resetting every actor for an unrelated flag. Invalidate on asset revisions as established by RF-08.
- Preserve wander state for unaffected NPCs, pause semantics, and deterministic RNG consumption. Avoid parallel changes that reorder simulation.
- **Done when:** the representative workload allocates/rebuilds less, with matching gameplay hashes and immediate quest-driven presence updates.
- **Model fit:** Sol/high for targeted ECS change detection and state preservation.

### RF-24 — Reduce menu rebuild and list allocation costs

**Evidence: Source; cost unmeasured.** [items_page.rs](../crates/rpg-engine/src/field_menu/ui/items_page.rs) and other pages already gate rebuilds, but use broad `GameState`/menu/catalog change detection and despawn/recreate page trees. Derived list IDs and labels can be rebuilt repeatedly.

- Measure selection-only navigation, tab changes, and unrelated session mutations. Inspect whether mutable borrows cause spurious change detection before adding another cache.
- Update selection/text in place where cheaper; reserve full rebuilds for structural changes. Cache derived lists by the repository/party/quest/catalog revisions they actually use.
- Bound caches and preserve current visible-row limits, sorted item order, lock/tag semantics, and scroll-to-selection.
- **Done when:** measured navigation hitches/entity churn decrease without stale labels, selection drift, or a higher steady memory cost.
- **Model fit:** Sol/high for focused UI invalidation and allocation work.

### RF-25 — Reduce save/autosave stalls without weakening durability

**Evidence: Source; cost unmeasured.** [autosave.rs](../crates/rpg-engine/src/autosave.rs) calls synchronous encode/verify/fsync/replacement during `Update`. [save_ui.rs](../crates/rpg-engine/src/save_ui.rs) refreshes the full slot catalog; the store reads/decodes slots 0–100.

- Measure checkpoint duration and catalog refresh separately on representative storage and populated slots. First eliminate redundant serialization or refresh work supported by the measurements.
- If worker-thread IO is justified, snapshot immutable game state at the settled-arrival barrier, serialize writes per store/slot, and report completion/failure back to the main thread.
- Preserve checksum validation, no-overwrite behavior, temporary verification, fsync policy, session identity, and request ordering. Define quit/map-change behavior while a write is outstanding.
- Refresh affected slot metadata incrementally only with a reliable invalidation path for external changes.
- **Done when:** measured save hitches decrease, failed writes retain the last valid file, and shutdown/out-of-order completions cannot publish a stale save or false success.
- **Model fit:** Astra/xhigh for concurrency, durability, and session-lifetime interactions.

### RF-26 — Optimize map load/render work demonstrated by profiles

**Evidence: Investigation.** [tmx_ground_asset.rs](../crates/rpg-engine/src/tmx_ground_asset.rs) produces visible tile bundles and already uses `spawn_batch`; bundle building, dependency validation, and entity churn need measurement before further changes.

- Profile TMX/TSX parsing, `visible_bundles`, collision projection, per-tile entity creation, publication, and steady draw costs on the largest representative maps.
- Reuse validated immutable projections by asset revision or reduce repeated preflight work if that is the measured bottleneck. Consider chunked rendering only if entity/draw cost warrants it.
- Preserve flip flags, animation, transparent/object layers, y-ordering, camera bounds, and atomic transitions. Bound/evict caches to avoid trading startup speed for growing memory.
- **Done when:** a named measured bottleneck improves beyond noise, visual/map-sweep checks agree, hot-change invalidation works, and peak memory stays within budget.
- **Model fit:** Astra/high for rendering architecture and cache tradeoffs.

## Comment cleanup

### RF-27 — Trim historical and redundant code comments

**Evidence: Source.** [options_store.rs](../crates/rpg-engine/src/options_store.rs) opens with an extended explanation of rejected storage choices; [world_pause.rs](../crates/rpg-engine/src/world_pause.rs) recounts an already-fixed bug; [scenario_map_sweep.rs](../crates/rpg-engine/src/scenario_map_sweep.rs) describes removed scratch maps; intro comments still refer to a future renderer. Some other long comments explain essential parity and coordinate rules and should stay.

- Replace implementation history and repeated narration with concise statements of the current contract. Remove obsolete milestone/future-tense comments after the associated behavior is resolved.
- Keep safety/durability reasoning, source-parity evidence, public API docs, coordinate conventions, scheduler ordering, serialized-compatibility rules, and licensing/attribution.
- Put enduring architectural rationale in the appropriate documentation only when it remains useful; avoid relocating obsolete commentary wholesale.
- Use a comment-only diff and formatting checks. Set no blanket deletion percentage or line-count target.
- **Done when:** targeted comments explain current behavior succinctly, preserve required rationale, and introduce no executable or data changes.
- **Model fit:** Luna/medium for controlled editorial edits with explicit preservation criteria.

## Validation commands and completion evidence

Run from the repository root. If the shared `target` location is not writable, prefix Cargo commands with `CARGO_TARGET_DIR=temp/cargo-target`; use an existing writable cache when available rather than forcing an unnecessary full rebuild.

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -- validate-scenario rusted_kingdoms --baseline assets/scenarios/rusted_kingdoms/validation-baseline.txt
cargo run -- map-sweep rusted_kingdoms
cargo run -- dialogue-sweep rusted_kingdoms
cargo run -- encounter-sweep rusted_kingdoms
python3 -m unittest -v scripts/test_check_asset_rights.py
```

For editor changes, run the backend and frontend checks documented in [the editor verification guide](../tools/map_editor_web/README.md#ci-and-local-verification), including the focused suites, lockfile install, build, and production editor check. Use isolated test maps, save directories, and config directories. For rendering changes, run `scripts/check-ardel-screenshot.sh` and the affected visual spot checks. Sweep reports are informational: inspect their findings rather than treating a zero exit code as proof that all maps passed.

Each completed task should report its reproduction or preserved behavior, changed paths, checks run and their results, and remaining manual limitations. Performance tasks additionally attach raw before/after measurements. Do not expand the accepted validation baseline merely to hide a regression.

## Review evidence and existing-plan coordination

- The initial Git worktree was clean at `ac5d827`. No production code, content, existing trackers, or assets were changed to prepare this plan.
- `cargo fmt --all -- --check` passed during planning.
- Two isolated Python probes reproduced interrupted canonical TMX publication and acceptance of invalid portal geometry/destinations. The probes used temporary maps and did not alter shipped assets.
- Remaining bug entries are source findings or investigations as labeled. Workspace tests, clippy, full-game reproductions, and performance measurements were not run during this planning pass.
- Structural estimates above are approximate line counts at the reviewed commit; tests explain a substantial part of the larger files. File size alone does not justify deleting behavior or tests.
- Existing plans were checked live in the sibling planning directory: [master index](../../rpg-s1-plans/000-master-todo.md), [implementation](../../rpg-s1-plans/implementation.md), [manual verification](../../rpg-s1-plans/manual-verification.md), and [parity](../../rpg-s1-plans/parity.md). Those local links assume the current sibling-directory layout.
- Completed RF-10 provides [persistence/replay fixtures and operator steps](persistence-replay-checklist.md); RF-08 supports existing scenario hot-change acceptance; completed RF-11 gates editor regressions in CI and supports existing editor acceptance; RF-22 supports M14.07. This document does not duplicate or close those canonical entries.
- Asset rights, package approval, visual effect judgment, actual gamepad/mouse acceptance, and a complete campaign remain under their existing owners. Do not treat refactoring completion as release acceptance.
