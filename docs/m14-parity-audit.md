# Milestone 14 parity audit

Status: in progress

This report is the reproducible starting point for M14.01. It records the
strongest non-interactive evidence available for the current candidate without
treating automated coverage as a substitute for the actions in
`plans/rusted-kingdoms-parity-checklist.md`.

## Candidate identity

- Audit date: 2026-08-27
- Target commit: `0fbbe18ee133411a821bdae44936841ee32f8357`
- Pinned Python source: `08970359d6cb03586948625d29b0d3351dbbf785`
- Scenario: `my_rpg_story` version `1.0.0`
- Scenario Git tree: `0b639b372b565a004c525806121939b27174b783`
- Platform: Linux `7.1.8-arch1-3`, x86-64
- Toolchain: `rustc 1.97.1`, `cargo 1.97.1`
- Target worktree before the audit: clean

The owner reported that normal play, manual save, and autosave work on this
candidate. That is useful smoke evidence, but it is not assigned to narrower
checklist rows whose setup, actions, and observations were not recorded.

## Checklist rollup

The player-visible checklist contained 149 rows at the start of this audit:

| Status | Rows |
| --- | ---: |
| Pass | 47 |
| Partial | 5 |
| Not run | 97 |

M14.01 remains open. Its contract requires every row to be `Pass` or to link to
an approved `Accepted difference`; 102 rows do not yet meet that condition.

## Automated evidence

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Pass |
| `cargo test --workspace` | Pass: 677 passed, 0 failed, 24 source-dependent tests ignored |
| Pinned-source ignored suite | Pass: all 20 source-dependent tests at the pinned commit, including the live Python/Rust validator matrix |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass |
| `scripts/check-ardel-screenshot.sh` | Pass: RGBA8 hash `85ce229c04604258debbad65643ebcc62177f084727178d29b968baeb35b2012` |
| `rpg-s1 map-sweep rusted_kingdoms` | Runtime pass: 45 migrated maps, 0 load failures |
| `rpg-s1 dialogue-sweep rusted_kingdoms` | Pass: 91 documents, 217 terminating paths, 0 cycles, 0 errors |
| `rpg-s1 encounter-sweep rusted_kingdoms` | Pass: 16 zones, 157 constructions, 118 assets, 0 failures |
| `rpg-s1 validate-scenario rusted_kingdoms` | Expected fail: 13 errors, 0 warnings; matches the inherited-content inventory below |

The map report also retained its documented authoring findings while the
runtime phase passed: one unsupported non-migrated sample map and two pinned
maps whose painted signs intentionally resolve to missing dialogue. The two
non-migrated sample maps were excluded from the 45-map runtime result.

The initial pinned-source command was incomplete and therefore failed before
providing parity evidence. The corrected run supplied every
focused pinned-data path. Nineteen tests passed against the pinned checkout;
the one test that requires a clean Git worktree passed separately against a
disposable clean clone at the same commit. No source checkout was modified.

## Open release blockers

### Campaign acceptance

Gate 12 is not complete. The active migration ledger still has one W12.3 live
map check and seven W12.4 live map checks open, and W12.5 through W12.8 have not
received their required wave audits. In particular, no clean Rust-only
new-game-to-ending execution record exists yet for RK-CMP-001 through
RK-CMP-009.

### Production validation

The production validator retains 13 errors already classified by
`docs/adr/0007-inherited-scenario-data-debt.md`:

- five undefined producer flags;
- five unavailable item references;
- two stale Jep recruitment references; and
- one missing cursor asset.

These findings are not new M14 regressions, but RK-DBG-010 cannot be marked
`Pass` while the documented validation command exits with failure unless an
approved difference explicitly resolves that checklist contract.

### Redistribution rights

`docs/asset-license-inventory.md` contains 83 concrete asset entries: 10 are
`approved` and 73 remain `needs-evidence`. M14.05 and any public release package
remain blocked until every shipped path has approved rights evidence or is
replaced or excluded from the payload. Local parity authorization is not
public redistribution permission.

### Release candidate and manual execution

M14.08 and M14.09 have not yet produced and exercised a self-contained package.
Rows requiring a real window, audio output, physical input, editor interaction,
or full campaign progression therefore remain manual work. Each run must record
the environment and evidence fields required by the parity checklist.

## Next evidence boundary

Complete the open Gate 12 waves first, beginning with the W12.3 inn live check
and W12.4 live acceptance. Then produce the self-contained M14.08 candidate and
run the remaining M14.01 rows against that exact package. This ordering avoids
certifying a package whose campaign content or shipped asset set is still
changing.
