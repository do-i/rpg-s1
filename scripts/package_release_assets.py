#!/usr/bin/env python3
"""Copy the exact, rights-audited asset set into a release staging directory."""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path
from typing import Sequence

try:
    from scripts.release_assets import copy_release_assets, release_asset_paths
except ModuleNotFoundError:  # Direct execution adds scripts/, not the repo root.
    from release_assets import copy_release_assets, release_asset_paths


def repository_root() -> Path:
    result = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        check=True,
        capture_output=True,
        text=True,
    )
    return Path(result.stdout.strip())


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "destination",
        type=Path,
        help="new directory that will become the payload's assets/ directory",
    )
    return parser.parse_args(argv)


def main(argv: Sequence[str] = ()) -> int:
    args = parse_args(argv)
    repo_root = repository_root()
    destination = args.destination
    if not destination.is_absolute():
        destination = Path.cwd() / destination

    payload, selection_errors = release_asset_paths(repo_root)
    errors = selection_errors or copy_release_assets(repo_root, destination, payload)
    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1

    print(f"Copied {len(payload)} release assets to {destination}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
