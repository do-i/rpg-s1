# Changing code

- Preserve save and options formats, scenario boundaries, deterministic RNG ordering, and existing diagnostics unless the task explicitly requires a reviewed change.
- Keep bug fixes and structural moves in separate commits. Add focused tests for identified failures.
- Remove code only after checking production callers, tests, Bevy plugin and system registration, CLI paths, and serialized compatibility. Asset removal needs separate owner approval.
- Keep a performance change only with a reproducible before/after comparison showing a gain beyond run-to-run variation.

# Documentation

- Do not add files to `docs/`; CI fails on anything outside the allowlist in `scripts/check_docs_allowlist.py`. Do not extend that allowlist unless the repository owner explicitly asks.
- Put short-lived plans, notes, and reports in `temp/YYYYMMDD-HHMM-description/` (gitignored), lasting reference in an existing document, and follow-up work in a GitHub issue. See `docs/DO_NOT_ADD_FILES_HERE.md`.

# Git and release workflow

- `dev` is the integration branch. Land and validate changes on `dev`.
- `main` is the release branch. Never commit or push changes directly to `main`.
- Keep history linear. Do not create merge commits; rebase `dev` onto `main` when needed, then push `dev` and wait for CI to pass.
- Before publishing, use **Release → Release Status** in the developer menu to check branch and release readiness.
- Use **Release → Release 1 Cut** to bump the version on `dev` and push it. Wait for CI to pass on the resulting `dev` commit.
- Use **Release → Release 2 Tag/Push** to fast-forward `main` and push the release tag. The tag triggers the release build.
- If release status or the release script reports branch divergence, stop and inspect the histories. Do not bypass the guard by pushing directly to `main`, force-pushing, or skipping CI.
- Before any Git write, check the current branch and working tree. If a change was accidentally committed on `main`, preserve it and move or reproduce the change on `dev`; do not push `main` to fix the mistake.
