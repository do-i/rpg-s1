# Git and release workflow

- `dev` is the integration branch. Land and validate changes on `dev`.
- `main` is the release branch. Never commit or push changes directly to `main`.
- Keep history linear. Do not create merge commits; rebase `dev` onto `main` when needed, then push `dev` and wait for CI to pass.
- Before publishing, use **Release → Release Status** in the developer menu to check branch and release readiness.
- Use **Release → Release 1 Cut** to bump the version on `dev` and push it. Wait for CI to pass on the resulting `dev` commit.
- Use **Release → Release 2 Tag/Push** to fast-forward `main` and push the release tag. The tag triggers the release build.
- If release status or the release script reports branch divergence, stop and inspect the histories. Do not bypass the guard by pushing directly to `main`, force-pushing, or skipping CI.
- Before any Git write, check the current branch and working tree. If a change was accidentally committed on `main`, preserve it and move or reproduce the change on `dev`; do not push `main` to fix the mistake.
