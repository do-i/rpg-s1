#!/usr/bin/env python3
"""Fail if docs/ contains a file that is not on the allowlist."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path
from typing import Iterable

# Only the repository owner may extend this list, and only by explicit request.
ALLOWED_DOCS = frozenset(
    {
        "docs/DO_NOT_ADD_FILES_HERE.md",
        "docs/asset-license-inventory.md",
        "docs/asset-license-reviews.md",
        "docs/authoring.md",
        "docs/screenshots/ardel-overworld.png",
        "docs/screenshots/field-menu.png",
        "docs/screenshots/title-screen.png",
        "docs/screenshots/volcanic-battle.png",
    }
)


def unexpected_docs(paths: Iterable[str]) -> list[str]:
    return sorted(set(paths) - ALLOWED_DOCS)


def docs_files(repo_root: Path) -> list[str]:
    """List tracked files, plus untracked ones so local runs catch them too."""
    result = subprocess.run(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "--", "docs"],
        cwd=repo_root,
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout.splitlines()


def main() -> int:
    repo_root = Path(__file__).resolve().parent.parent
    unexpected = unexpected_docs(docs_files(repo_root))
    if not unexpected:
        print("docs/ allowlist: OK")
        return 0

    print("docs/ is closed to new files. Remove or move:", file=sys.stderr)
    for path in unexpected:
        print(f"  {path}", file=sys.stderr)
    print(
        "\nSee docs/DO_NOT_ADD_FILES_HERE.md. Short-lived work goes in temp/, lasting "
        "reference goes in an existing document, and follow-up work becomes a GitHub issue. "
        "Do not extend the allowlist unless the repository owner asked for it.",
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    sys.exit(main())
