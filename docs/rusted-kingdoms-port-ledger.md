# Rusted Kingdoms Port Ledger

This ledger fixes the exact source and target revisions used by the port. Add a
new dated snapshot whenever either pinned revision changes; do not rewrite an
older snapshot.

Repository paths are relative to the target repository:

- source: `../agentic-rpg`
- target: `.`

## Snapshots

### 2026-08-07 — Initial port snapshot

The working-tree states below were captured before this ledger and its plan
checkbox were edited.

| Role | Repository | Branch | Commit | Working tree | Upstream state |
| --- | --- | --- | --- | --- | --- |
| Source | `../agentic-rpg` | `main` | `08970359d6cb03586948625d29b0d3351dbbf785` | Clean | Matches `origin/main` |
| Target | `.` | `main` | `8f4c805761c0f89bc6ea3ad42182a235cc022227` | Clean | One commit ahead of `origin/main` |

## Adding a snapshot

Append a dated subsection under **Snapshots** containing:

1. why the source or target pin changed;
2. the full commit hash for both repositories;
3. branch and working-tree state for both repositories;
4. each branch's relationship to its configured upstream; and
5. the task or decision that approved the new snapshot.

Record dirty working trees explicitly and summarize the affected paths. A new
snapshot changes the port baseline only when its entry says so; merely
observing a newer commit does not move the baseline.
