#!/usr/bin/env bash
#
# `cargo run` with the same asset-hash refresh as scripts/build.sh, so launching the game after
# editing a map keeps the ledger current too. Arguments go to Cargo unchanged, e.g.
#   scripts/run.sh --release -- play rusted_kingdoms --seed 1
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly project_root="$(cd -- "$script_dir/.." && pwd)"

python3 "$project_root/scripts/update_asset_hashes.py" --write --repo-root "$project_root"
cd -- "$project_root"
exec cargo run "$@"
