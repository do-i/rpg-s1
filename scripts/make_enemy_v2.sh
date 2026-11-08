#!/usr/bin/env bash
# One-command pipeline for an enemy's v2 battler art:
#   generate -> cut out -> pack sheets -> (optionally) promote into v2/ + ledger rows
#
# Output is also appended to WORKDIR/ID.log.
# usage: scripts/make_enemy_v2.sh ID [--work DIR] [--from generate|cutout|pack|promote] [--promote]
#
# Stages (each re-runnable; use --from to start midway):
#   generate  runs $ENEMY_GEN_CMD ID WORKDIR. Skipped when ENEMY_GEN_CMD is unset. The command
#             must leave in WORKDIR: ready.jpg (the approved canonical image) and one
#             ID-NN-<pose>.jpg per pose (names in scripts/pack_enemy_sheets.py, NN = 01..16).
#   cutout    scripts/cutout.sh WORKDIR          -> WORKDIR/cutouts/
#   pack      scripts/pack_enemy_sheets.py       -> WORKDIR/export/ID/
#   promote   scripts/promote_enemy_sheets.py    -> v2/ + ledger (only with --promote; never commits)
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
id=${1:?usage: make_enemy_v2.sh ID [--work DIR] [--from STAGE] [--promote]}; shift
work="$root/temp/$(date +%Y%m%d-%H%M)-$id"; from=generate; promote=0
while [ $# -gt 0 ]; do
  case $1 in
    --work) work=$2; shift 2;;
    --from) from=$2; shift 2;;
    --promote) promote=1; shift;;
    *) echo "unknown option: $1" >&2; exit 2;;
  esac
done
stages=(generate cutout pack promote)
go=0; for s in "${stages[@]}"; do [ "$s" = "$from" ] && go=1; done
[ $go = 1 ] || { echo "bad --from: $from" >&2; exit 2; }
mkdir -p "$work"
run() { local s=$1; local i; for i in "${stages[@]}"; do [ "$i" = "$from" ] && break; [ "$i" = "$s" ] && return 1; done; return 0; }

main() {
  if run generate; then
    if [ -n "${ENEMY_GEN_CMD:-}" ]; then echo "== generate"; $ENEMY_GEN_CMD "$id" "$work"
    else echo "== generate: skipped (ENEMY_GEN_CMD unset)"; fi
  fi
  if run cutout; then echo "== cutout"; "$root/scripts/cutout.sh" "$work" "$id-*.jpg"; fi
  if run pack; then
    echo "== pack"
    python3 -I "$root/scripts/pack_enemy_sheets.py" "$id" "$work" --ready "$work/ready.jpg"
  fi
  if run promote; then
    if [ $promote = 1 ]; then echo "== promote"; python3 -I "$root/scripts/promote_enemy_sheets.py" "$id" "$work/export"
    else echo "== promote: skipped (pass --promote; try scripts/promote_enemy_sheets.py --dry-run first)"; fi
  fi
  echo "work dir: $work"
}
set -o pipefail
main 2>&1 | tee -a "$work/$id.log"
