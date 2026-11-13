#!/usr/bin/env python3
"""List every encounter enemy missing a v2 status sprite, grouped by zone.

Uses only the Python standard library and reads the encounter files' current
simple YAML layout (inline formation lists and an optional boss mapping).
Paths are relative to this script's repository, regardless of the working directory.
"""

from pathlib import Path
import re


SCENARIO = (
    Path(__file__).resolve().parent.parent / "assets/scenarios/rusted_kingdoms"
)


def enemy_ids(path: Path) -> set[str]:
    """Read entries[].formation[] and boss.id, ignoring other sections' IDs."""
    ids: set[str] = set()
    section = None
    for line in path.read_text(encoding="utf-8").splitlines():
        header = re.match(r"^([a-z_]+):", line)
        if header:
            section = header.group(1)
            continue
        if section == "entries":
            formation = re.match(r"\s*-\s+formation:\s*\[([^\]]*)\]", line)
            if formation:
                ids.update(re.findall(r"[a-z][a-z0-9_]*", formation.group(1)))
        elif section == "boss":
            boss = re.match(r"^  id:\s*([a-z][a-z0-9_]*)\s*(?:#.*)?$", line)
            if boss:
                ids.add(boss.group(1))
    return ids


def main() -> None:
    encounter_dir = SCENARIO / "data/encount"
    sprite_dir = SCENARIO / "media/sprites/enemies/v2"
    paths = sorted(encounter_dir.glob("zone_*.yaml"))
    if not paths:
        raise SystemExit(f"No zone encounter files found in {encounter_dir}")
    if not sprite_dir.is_dir():
        raise SystemExit(f"Sprite directory not found: {sprite_dir}")

    by_zone: dict[int, set[str]] = {}
    for path in paths:
        number = int(re.match(r"zone_(\d+)", path.stem).group(1))
        missing = {
            enemy_id
            for enemy_id in enemy_ids(path)
            if not (sprite_dir / f"{enemy_id}-status.png").is_file()
        }
        if missing:
            by_zone.setdefault(number, set()).update(missing)

    sections = []
    for number, ids in sorted(by_zone.items()):
        ordered = sorted(ids)
        rows = [" ".join(ordered[start:start + 5]) for start in range(0, len(ordered), 5)]
        sections.append("\n".join([f"Zone {number}", *rows]))
    print("\n\n".join(sections) if sections else "None")


if __name__ == "__main__":
    main()
