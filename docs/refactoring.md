# Refactoring guidelines

Open refactoring and performance work is tracked as GitHub issues
(`refactor` and `performance` labels). These rules apply to all of it.

- **Severity:** P0 = data loss or a hard gameplay blocker; P1 = incorrect
  behavior, persistence failures, or a missing regression gate; P2 =
  maintainability or measured performance; P3 = editorial cleanup.
- Preserve save/options formats, scenario boundaries, deterministic RNG
  ordering, and existing diagnostics unless the task explicitly requires a
  reviewed change. Keep bug fixes and structural moves in separate commits.
- Add focused tests for identified failures; reuse existing tests for
  mechanical moves and comment edits.
- Remove code only after checking production callers, tests, Bevy
  plugin/system registration, supported CLI paths, and serialized
  compatibility. An unused accessor does not prove its state is unused.
- Performance changes need a reproducible before/after comparison. Keep an
  optimization only when the gain exceeds run-to-run variation.
- Keep runtime/content assets out of code-deletion work; asset removal needs
  separate owner approval.

Run the checks in the README's
[Validation and developer tools](../README.md#validation-and-developer-tools)
section. For editor changes, also run the
[editor verification](../tools/map_editor_web/README.md#ci-and-local-verification)
checks; for rendering changes, run `scripts/check-ardel-screenshot.sh` and the
affected visual spot checks. Sweep reports are informational: read their
findings rather than trusting a zero exit code.
