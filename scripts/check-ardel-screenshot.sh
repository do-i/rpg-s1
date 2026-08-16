#!/usr/bin/env bash
set -euo pipefail

script_dir="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(CDPATH= cd -- "$script_dir/.." && pwd)"

map="$repo_root/assets/scenarios/rusted_kingdoms/assets/maps/town_01_ardel.tmx"
aric="$repo_root/assets/scenarios/rusted_kingdoms/assets/sprites/party/01_aric_walk.png"
oracle="$repo_root/tests/fixtures/ardel-screenshot/rgba8.sha256"
output="${1:-$repo_root/target/ardel-new-game.png}"

for required_command in tmxrasterizer magick sha256sum; do
    if ! command -v "$required_command" >/dev/null 2>&1; then
        echo "missing required command: $required_command" >&2
        exit 2
    fi
done

mkdir -p "$(dirname -- "$output")"
work_dir="$(mktemp -d)"
trap 'rm -rf -- "$work_dir"' EXIT

export QT_QPA_PLATFORM="${QT_QPA_PLATFORM:-offscreen}"

# PyTMX and the native runtime both render every authored visible tile layer in source order.
# Ardel intentionally uses visible collision tiles for its buildings and fences.
tmxrasterizer "$map" "$work_dir/map.png"

# The source's feet-aligned 20x18 collision rectangle is centered on spawn tile [14, 5],
# placing the 64px down-idle frame (TSX tile 18) at top-left [432, 126].
magick "$aric" -crop 64x64+0+128 +repage "$work_dir/aric-down-idle.png"
magick "$work_dir/map.png" \
    "$work_dir/aric-down-idle.png" -geometry +432+126 -compose over -composite \
    "$work_dir/map-composite.png"

# Ardel is 960x640, smaller than the 1280x766 logical canvas on both axes.
# The production camera centers it at +160,+63 over UiTheme's clear color.
magick -size 1280x766 canvas:'rgb(10,10,30)' \
    "$work_dir/map-composite.png" -geometry +160+63 -compose over -composite \
    "$output"

expected_hash="$(sed -n 's/^\([0-9a-f]\{64\}\).*/\1/p' "$oracle")"
actual_hash="$(magick "$output" -alpha on -depth 8 RGBA:- | sha256sum | cut -d ' ' -f 1)"

if [[ -z "$expected_hash" ]]; then
    echo "invalid screenshot oracle: $oracle" >&2
    exit 2
fi
if [[ "$actual_hash" != "$expected_hash" ]]; then
    echo "Ardel screenshot mismatch" >&2
    echo "expected RGBA8 hash: $expected_hash" >&2
    echo "actual RGBA8 hash:   $actual_hash" >&2
    echo "rendered artifact:   $output" >&2
    exit 1
fi

echo "Ardel screenshot ok: $actual_hash"
echo "rendered artifact: $output (1280x766 RGBA8 oracle)"
