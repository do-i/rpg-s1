#!/usr/bin/env python3
"""Select the tracked asset files that belong in a release payload."""

from __future__ import annotations

import subprocess
from pathlib import Path, PurePosixPath
from typing import Sequence


DEFAULT_EXCLUSION_LIST = Path("release-assets-exclude.txt")


def tracked_asset_paths(repo_root: Path) -> list[str]:
    result = subprocess.run(
        ["git", "ls-files", "-z", "--", "assets"],
        cwd=repo_root,
        check=True,
        capture_output=True,
    )
    return sorted(
        path.decode("utf-8")
        for path in result.stdout.split(b"\0")
        if path
    )


def parse_exclusion_list(text: str) -> tuple[tuple[str, ...], tuple[str, ...]]:
    """Parse exact repository-relative asset paths from the release exclusion list."""
    paths: list[str] = []
    errors: list[str] = []
    seen: set[str] = set()

    for line_number, raw_line in enumerate(text.splitlines(), start=1):
        path = raw_line.strip()
        if not path or path.startswith("#"):
            continue

        pure = PurePosixPath(path)
        if (
            pure.is_absolute()
            or path != pure.as_posix()
            or any(part in {".", ".."} for part in pure.parts)
            or len(pure.parts) < 2
            or pure.parts[0] != "assets"
        ):
            errors.append(
                f"line {line_number}: exclusion must be a normalized assets/ path: {path!r}"
            )
            continue
        if path in seen:
            errors.append(f"line {line_number}: duplicate exclusion: {path}")
            continue

        seen.add(path)
        paths.append(path)

    return tuple(paths), tuple(errors)


def release_asset_paths(
    repo_root: Path,
    exclusion_list: Path = DEFAULT_EXCLUSION_LIST,
) -> tuple[list[str], tuple[str, ...]]:
    """Return tracked release assets after validating and applying exact exclusions."""
    if not exclusion_list.is_absolute():
        exclusion_list = repo_root / exclusion_list
    if not exclusion_list.is_file():
        return [], (f"release exclusion list is not readable: {exclusion_list}",)

    exclusions, parse_errors = parse_exclusion_list(
        exclusion_list.read_text(encoding="utf-8")
    )
    tracked = set(tracked_asset_paths(repo_root))
    errors = list(parse_errors)
    for path in exclusions:
        if path not in tracked:
            errors.append(f"release exclusion is not a tracked asset: {path}")

    return sorted(tracked.difference(exclusions)), tuple(errors)


def copy_release_assets(
    repo_root: Path,
    destination: Path,
    payload_paths: Sequence[str],
) -> tuple[str, ...]:
    """Copy an already-selected payload, preserving its layout below assets/."""
    import shutil

    errors = tuple(
        f"release asset is not a readable regular file: {path}"
        for path in payload_paths
        if not (repo_root / path).is_file()
    )
    if errors:
        return errors
    if destination.exists():
        return (f"release asset destination already exists: {destination}",)

    for path in payload_paths:
        relative = Path(path).relative_to("assets")
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(repo_root / path, target)
    return ()
