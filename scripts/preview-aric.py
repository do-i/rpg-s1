#!/usr/bin/env python3
"""Launch an Aric sprite variant in a disposable copy of Rusted Kingdoms."""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET


REPO_ROOT = Path(__file__).resolve().parent.parent
VARIANTS = {
    "classic": "01_aric_classic_walk",
    "traveler": "01_aric_traveler_walk",
    "armored": "01_aric_armored_walk",
}


def check_atlas(tsx_path: Path) -> None:
    atlas = ET.parse(tsx_path).getroot()
    expected = {"tilewidth": "64", "tileheight": "64", "tilecount": "36", "columns": "9"}
    if any(atlas.get(key) != value for key, value in expected.items()):
        raise ValueError("sprite must contain 9 by 4 cells of 64 by 64 pixels")
    image = atlas.find("image")
    if image is None or image.get("width") != "576" or image.get("height") != "256":
        raise ValueError("sprite metadata must declare a 576 by 256 image")
    source = image.get("source", "")
    if not source or Path(source).name != source:
        raise ValueError("sprite image must be beside its TSX file")
    with (tsx_path.parent / source).open("rb") as stream:
        header = stream.read(24)
    if header[:8] != b"\x89PNG\r\n\x1a\n" or len(header) != 24:
        raise ValueError("sprite image is not a PNG")
    if struct.unpack(">II", header[16:24]) != (576, 256):
        raise ValueError("sprite PNG must measure 576 by 256 pixels")
    tiles = atlas.findall("tile")
    if [tile.get("id") for tile in tiles] != ["0", "9", "18", "27"]:
        raise ValueError("sprite must define all four directional animation owners")
    for row, tile in enumerate(tiles):
        animation = tile.find("animation")
        if animation is None:
            raise ValueError("direction is missing its walk animation")
        frames = animation.findall("frame")
        if [frame.get("tileid") for frame in frames] != [str(row * 9 + n) for n in range(1, 9)]:
            raise ValueError("each direction must animate its eight walking columns")
        if any(int(frame.get("duration", "0")) <= 0 for frame in frames):
            raise ValueError("animation durations must be positive")


def skip_scratch_and_links(directory: str, names: list[str]) -> set[str]:
    return {
        name for name in names
        if name in {".cache", "__pycache__"} or (Path(directory) / name).is_symlink()
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("variant", choices=[*VARIANTS, "original"])
    parser.add_argument("--check", action="store_true", help="validate the preview and load its maps headlessly")
    parser.add_argument(
        "--binary", type=Path, default=REPO_ROOT / "target/debug/rpg-s1",
        help="game executable to use (default: target/debug/rpg-s1; build first with scripts/build.sh)",
    )
    args = parser.parse_args()
    binary = args.binary.resolve()
    if not binary.is_file() or not os.access(binary, os.X_OK):
        parser.error("game executable is unavailable; run scripts/build.sh first, or pass --binary PATH")
    collection = REPO_ROOT / "assets/scenarios"
    source = collection / "rusted_kingdoms"
    sprite = source / "media/sprites/party/01_aric_walk.tsx"
    if args.variant != "original":
        sprite = source / "media/sprites/party" / f"{VARIANTS[args.variant]}.tsx"
    check_atlas(sprite)

    # A real directory beneath the collection satisfies the game's bounded-package loader.
    # Copying instead of linking keeps every preview write away from the source package.
    with tempfile.TemporaryDirectory(prefix="aric_preview_", dir=collection) as temporary:
        package = Path(temporary)
        shutil.copytree(source, package, dirs_exist_ok=True, ignore=skip_scratch_and_links)
        check_atlas(package / sprite.relative_to(source))
        manifest = package / "manifest.yaml"
        text = manifest.read_text()
        lines = text.splitlines(keepends=True)
        protagonist = False
        for index, line in enumerate(lines):
            if line and not line[0].isspace():
                protagonist = line.strip() == "protagonist:"
            if protagonist and line.startswith("  sprite:"):
                lines[index] = f"  sprite: {sprite.relative_to(source).as_posix()}\n"
                break
        else:
            raise ValueError("manifest protagonist sprite is missing")
        text = "".join(lines)
        original_title = 'window_title: "Rusted Kingdoms"'
        if text.count(original_title) != 1:
            raise ValueError("expected Rusted Kingdoms window title is missing or ambiguous")
        manifest.write_text(text.replace(original_title, f'window_title: "Rusted Kingdoms - Aric {args.variant} preview"'))
        session = package / ".preview-session"
        env = os.environ.copy()
        env["RPG_S1_SAVE_DIR"] = str(session / "saves")
        env["RPG_S1_CONFIG_DIR"] = str(session / "config")
        env.setdefault("RPG_S1_MUTE_AUDIO", "1")
        if args.check:
            command = [str(binary), "map-sweep", package.name]
        else:
            command = [
                str(binary), "play", package.name,
                "--party-preset", "solo",
                "--start-map", "town_03_harborgate", "--start-position", "7,17",
            ]
        print(f"Aric preview: {args.variant}; temporary scenario, saves and settings.", flush=True)
        print("Close the game to finish; the preview is removed automatically.", flush=True)
        return subprocess.run(command, cwd=REPO_ROOT, env=env, check=False).returncode


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except KeyboardInterrupt:
        raise SystemExit(130) from None
    except (OSError, ValueError, ET.ParseError) as error:
        print(f"Aric preview: {error}", file=sys.stderr)
        raise SystemExit(1) from None
