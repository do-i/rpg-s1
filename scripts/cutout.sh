#!/usr/bin/env bash
# Cut every pose JPG/PNG in a directory out of its magenta field.
# usage: scripts/cutout.sh DIR [PATTERN]     (default pattern: *.jpg)
# writes DIR/cutouts/<name>.png plus DIR/cutouts/check-green.png (all cutouts on green, for eyeballing)
set -u
dir=${1:?usage: cutout.sh DIR [PATTERN]}
pat=${2:-*.jpg}
root=$(cd "$(dirname "$0")/.." && pwd)
tool="$root/scripts/cutout.py"
out="$dir/cutouts"; mkdir -p "$out"
ok=0; bad=()
for f in "$dir"/$pat; do
  [ -e "$f" ] || continue
  n=$(basename "${f%.*}")
  if python3 -I "$tool" "$f" "$out/$n.png" > "$out/$n.report.txt" 2>&1; then
    ok=$((ok+1)); echo "ok    $n"
  else
    bad+=("$n"); echo "FAIL  $n  (see $out/$n.report.txt)"
  fi
done
# green-screen check sheet from the cutouts
shopt -s nullglob
list=()
for p in "$out"/*.png; do [ "$(basename "$p")" = check-green.png ] || list+=("$p"); done
if [ ${#list[@]} -gt 0 ]; then
  magick montage "${list[@]}" -tile 4x -geometry 256x256+2+2 -background '#00ff40' "$out/check-green.png"
fi
echo "done: $ok ok, ${#bad[@]} failed -> $out"
[ ${#bad[@]} -eq 0 ]
