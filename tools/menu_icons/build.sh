#!/usr/bin/env bash
# Renders the field-menu command icons into one horizontal sheet, one 64px cell per command in
# the deck's order (Status, Spells, Items, Quit, Save, Equipment, Quests, Character, Recipes,
# Battle). The icons are white; the engine tints each with its command's accent colour.
set -euo pipefail
cd "$(dirname "$0")"
out=../../assets/scenarios/rusted_kingdoms/media/images/ui/menu_icons.png
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
for svg in [0-9][0-9]_*.svg; do
    rsvg-convert -w 64 -h 64 "$svg" -o "$tmp/${svg%.svg}.png"
done
magick "$tmp"/*.png -background none +append -strip PNG32:"$out"
echo "wrote $out"
