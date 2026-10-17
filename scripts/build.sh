#!/usr/bin/env bash
#
# Project build: refreshes stale home-grown asset hashes in the ledger, then runs `cargo build`.
#
# Editing a map in Tiled and building is enough to keep its `ALI-` row current; there is no
# separate hashing step. Every argument goes to Cargo unchanged, e.g.
#   scripts/build.sh --release --locked
#
# The refresh rewrites docs/asset-license-inventory.md in place and never stages or commits it.
# It refuses, before Cargo starts, if a third-party hash is stale or a tracked asset has no
# ledger row. Bare `cargo build`, `cargo test`, and clippy do not refresh anything; use this
# script or scripts/run.sh. Requires Python 3 and Git.
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly project_root="$(cd -- "$script_dir/.." && pwd)"

python3 "$project_root/scripts/update_asset_hashes.py" --write --repo-root "$project_root"
cd -- "$project_root"
exec cargo build "$@"
