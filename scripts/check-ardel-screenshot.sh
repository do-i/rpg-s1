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

# Match the runtime's reserved collision-layer exclusion and background policy.
tmxrasterizer \
    --hide-layer collision \
    --hide-layer decoration \
    "$map" "$work_dir/background.png"
tmxrasterizer \
    --hide-layer collision \
    --hide-layer ground \
    --hide-layer terrain \
    "$map" "$work_dir/decoration.png"

# M4's fixed spawn is tile [14, 5], facing down. The Bevy sprite is centered on
# that 32px tile, so the 64px idle frame (TSX tile 18) starts at [432, 144].
# Rows 0-5 of decoration sort behind its 208px bottom edge; rows 6+ sort ahead.
magick "$aric" -crop 64x64+0+128 +repage "$work_dir/aric-down-idle.png"
magick "$work_dir/decoration.png" \
    -crop 960x192+0+0 +repage "$work_dir/decoration-behind.png"
magick "$work_dir/decoration.png" \
    -crop 960x448+0+192 +repage "$work_dir/decoration-ahead.png"

magick "$work_dir/background.png" \
    "$work_dir/decoration-behind.png" -geometry +0+0 -compose over -composite \
    "$work_dir/aric-down-idle.png" -geometry +432+144 -compose over -composite \
    "$work_dir/decoration-ahead.png" -geometry +0+192 -compose over -composite \
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
