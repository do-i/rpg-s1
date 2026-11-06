#!/usr/bin/env python3
"""Make the item icon folders the source of truth for body-armor gender.

Each scenario keeps its item icons in `media/images/icons/items/`:

    equipment/          anything equippable; no folder below this means co-ed
    equipment/male/     body armor only a male member can wear
    equipment/female/   body armor only a female member can wear
    non_equipment/      everything else

Moving an icon between folders is the whole authoring step. With --write this
script then sets or removes `gender:` on the item's YAML entry, repoints the
icon's asset-ledger row, and stages the moved icon (the ledger's release
payload comes from `git ls-files`, so an unstaged move would look like a missing
asset). Nothing else is staged. scripts/build.sh and scripts/run.sh run it first;
CI runs --check.

Exit status: 0 success; 1 --check found drift; 2 invalid layout or content.
"""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
import tempfile
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import Sequence

try:
    from scripts.update_asset_hashes import (
        DEFAULT_LOCK_TIMEOUT,
        DEFAULT_INVENTORY,
        ConflictError,
        acquire_lock,
    )
except ModuleNotFoundError:  # Direct execution adds scripts/, not the repo root.
    from update_asset_hashes import (  # type: ignore[no-redef]
        DEFAULT_LOCK_TIMEOUT,
        DEFAULT_INVENTORY,
        ConflictError,
        acquire_lock,
    )


SCENARIOS = Path("assets/scenarios")
ICONS = Path("media/images/icons/items")
ITEMS = Path("data/items")
# Lives beside the item catalogs but dispatches item use; it defines no items.
NOT_AN_ITEM_CATALOG = "field_use.yaml"
EQUIPMENT_TYPES = {"weapon", "shield", "helmet", "body", "accessory"}
# Folder relative to ICONS -> the gender it means. Absent from this map: invalid.
EQUIPMENT_FOLDERS = {"equipment": None, "equipment/male": "male", "equipment/female": "female"}
NON_EQUIPMENT_FOLDER = "non_equipment"

ITEM_START = re.compile(r"^- ")
ITEM_ID = re.compile(r"^(?:- |  )id:\s*[\"']?([A-Za-z0-9_]+)[\"']?\s*$")
ITEM_TYPE = re.compile(r"^(?:- |  )type:\s*[\"']?([A-Za-z0-9_]+)[\"']?\s*$")
SLOT_CATEGORY = re.compile(r"^  slot_category:")
GENDER = re.compile(r"^  gender:\s*[\"']?([A-Za-z0-9_-]+)[\"']?\s*$")


class LayoutError(Exception):
    """The icon folders and item catalogs cannot be reconciled automatically."""


@dataclass
class Item:
    id: str
    type: str
    gender: str | None
    path: Path
    start: int
    end: int


def read_items(document: str, path: Path) -> list[Item]:
    """Finds each top-level list entry in an item catalog without a YAML dependency."""
    lines = document.splitlines(keepends=True)
    starts = [index for index, line in enumerate(lines) if ITEM_START.match(line)]
    items = []
    for position, start in enumerate(starts):
        end = starts[position + 1] if position + 1 < len(starts) else len(lines)
        block = lines[start:end]
        item_id = next((m[1] for m in map(ITEM_ID.match, block) if m), None)
        item_type = next((m[1] for m in map(ITEM_TYPE.match, block) if m), None)
        if item_id is None or item_type is None:
            raise LayoutError(f"{path}:{start + 1}: item entry has no plain id and type")
        gender = next((m[1] for m in map(GENDER.match, block) if m), None)
        items.append(Item(item_id, item_type, gender, path, start, end))
    return items


def apply_gender(document: str, item: Item, gender: str | None) -> str:
    """Sets or removes one item's `gender:` line, leaving every other byte alone."""
    lines = document.splitlines(keepends=True)
    block = lines[item.start : item.end]
    kept = [line for line in block if not GENDER.match(line)]
    if gender is not None:
        at = next((i for i, line in enumerate(kept) if SLOT_CATEGORY.match(line)), None)
        if at is None:
            raise LayoutError(f"{item.path}: `{item.id}` has no slot_category to follow")
        kept.insert(at + 1, f"  gender: {gender}\n")
    return "".join(lines[: item.start] + kept + lines[item.end :])


def scan_icons(icon_root: Path) -> dict[str, Path]:
    """Maps each item id to its icon, which must have a unique file stem."""
    icons: dict[str, Path] = {}
    for path in sorted(icon_root.rglob("*.png")):
        if path.stem in icons:
            raise LayoutError(
                f"{path.stem}: two icons share this id: {icons[path.stem]} and {path}"
            )
        icons[path.stem] = path
    return icons


def plan_scenario(scenario: Path) -> tuple[dict[Path, str], dict[str, Path], Counter[str], list[str]]:
    """Validates one scenario; returns new catalog text, icons, folder counts and errors."""
    icon_root = scenario / ICONS
    errors: list[str] = []
    icons = scan_icons(icon_root)
    documents = {
        path: path.read_text(encoding="utf-8")
        for path in sorted((scenario / ITEMS).glob("*.yaml"))
        if path.name != NOT_AN_ITEM_CATALOG
    }
    items: dict[str, Item] = {}
    for path, document in documents.items():
        for item in read_items(document, path):
            items[item.id] = item

    counts: Counter[str] = Counter()
    changed = dict(documents)
    # Bottom-up so an edit never shifts the line range of an entry not yet visited.
    for item in sorted(items.values(), key=lambda value: (value.path, -value.start)):
        icon = icons.get(item.id)
        if icon is None:
            errors.append(f"{item.id}: no icon under {icon_root}")
            continue
        folder = icon.parent.relative_to(icon_root).as_posix()
        counts[folder] += 1
        if item.type in EQUIPMENT_TYPES:
            if folder not in EQUIPMENT_FOLDERS:
                errors.append(
                    f"{icon}: {item.type} item `{item.id}` belongs in equipment/"
                    " (or equipment/male, equipment/female for body armor)"
                )
                continue
            wanted = EQUIPMENT_FOLDERS[folder]
            if wanted is not None and item.type != "body":
                errors.append(
                    f"{icon}: only body armor can be gender-specific, `{item.id}` is {item.type}"
                )
                continue
            if item.type == "body" and item.gender != wanted:
                changed[item.path] = apply_gender(changed[item.path], item, wanted)
            elif item.type != "body" and item.gender is not None:
                errors.append(f"{item.path}: `{item.id}` has gender but is not body armor")
        elif folder != NON_EQUIPMENT_FOLDER:
            errors.append(
                f"{icon}: {item.type} item `{item.id}` belongs in {NON_EQUIPMENT_FOLDER}/"
            )
    for item_id, icon in icons.items():
        if item_id not in items:
            errors.append(f"{icon}: no item has the id `{item_id}`")
    return {p: t for p, t in changed.items() if t != documents[p]}, icons, counts, errors


def rewrite_ledger(ledger: str, repo_root: Path, moved: dict[str, tuple[Path, dict[str, Path]]]) -> tuple[str, list[tuple[str, str]]]:
    """Repoints ledger rows at each icon's current path; returns the text and (old, new) moves."""
    moves: list[tuple[str, str]] = []

    def repoint(match: re.Match[str]) -> str:
        path = match[1]
        known = moved.get(path.split("/media/", 1)[0])
        if known is None:
            return match[0]
        icon = known[1].get(Path(path).stem)
        if icon is None:
            return match[0]
        current = icon.relative_to(repo_root).as_posix()
        if current != path:
            moves.append((path, current))
        return f"`{current}`"

    pattern = re.compile(rf"`({SCENARIOS.as_posix()}/[^/`]+/{ICONS.as_posix()}/[^`]+\.png)`")
    return pattern.sub(repoint, ledger), moves


def publish(path: Path, text: str) -> None:
    descriptor, temp_name = tempfile.mkstemp(dir=path.parent, prefix=f".{path.name}.")
    temp_path = Path(temp_name)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8", newline="") as handle:
            handle.write(text)
        os.chmod(temp_path, path.stat().st_mode & 0o7777)
        os.replace(temp_path, path)
    except BaseException:
        temp_path.unlink(missing_ok=True)
        raise


def stage_moves(repo_root: Path, moves: Sequence[tuple[str, str]]) -> None:
    """Stages only the icons this run saw move; a path git already knows is left as it is."""
    for old, new in moves:
        subprocess.run(
            ["git", "rm", "--cached", "--quiet", "--ignore-unmatch", "--", old],
            cwd=repo_root,
            check=True,
        )
        subprocess.run(["git", "add", "--", new], cwd=repo_root, check=True)


def run(repo_root: Path, ledger_path: Path, write: bool, lock_timeout: float) -> int:
    scenarios_root = repo_root / SCENARIOS
    scenarios = (
        [p for p in sorted(scenarios_root.iterdir()) if (p / ICONS).is_dir()]
        if scenarios_root.is_dir()
        else []
    )
    if not scenarios:
        return 0

    lock = acquire_lock(repo_root, lock_timeout) if write else None
    try:
        catalog_changes: dict[Path, str] = {}
        icons_by_scenario: dict[str, tuple[Path, dict[str, Path]]] = {}
        errors: list[str] = []
        for scenario in scenarios:
            changes, icons, counts, scenario_errors = plan_scenario(scenario)
            catalog_changes.update(changes)
            errors.extend(scenario_errors)
            icons_by_scenario[scenario.relative_to(repo_root).as_posix()] = (scenario, icons)
            summary = ", ".join(f"{folder}={count}" for folder, count in sorted(counts.items()))
            print(f"{scenario.name}: {summary}")
        if errors:
            for error in errors:
                print(f"error: {error}", file=sys.stderr)
            return 2

        ledger_file = repo_root / ledger_path
        ledger = ledger_file.read_text(encoding="utf-8") if ledger_file.is_file() else ""
        new_ledger, moves = rewrite_ledger(ledger, repo_root, icons_by_scenario)
        for path in catalog_changes:
            print(f"{'gender sync' if write else 'stale gender'}: {path.relative_to(repo_root)}")
        for old, new in moves:
            print(f"{'ledger path' if write else 'stale ledger path'}: {old} -> {new}")
        if not write:
            if catalog_changes or moves:
                print("Run scripts/sync_item_icons.py --write", file=sys.stderr)
                return 1
            return 0

        for path, text in catalog_changes.items():
            publish(path, text)
        if moves:
            publish(ledger_file, new_ledger)
            stage_moves(repo_root, moves)
        return 0
    finally:
        if lock is not None:
            lock.close()


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true", help="update YAML, ledger and index")
    mode.add_argument("--check", action="store_true", help="fail if anything is out of date")
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument("--ledger", type=Path, default=DEFAULT_INVENTORY)
    parser.add_argument("--lock-timeout", type=float, default=DEFAULT_LOCK_TIMEOUT)
    return parser.parse_args(argv)


def main(argv: Sequence[str] = ()) -> int:
    args = parse_args(argv)
    try:
        return run(args.repo_root.resolve(), args.ledger, args.write, args.lock_timeout)
    except (LayoutError, ConflictError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
