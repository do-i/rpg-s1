#!/usr/bin/env python3
"""Verify that every tracked release asset has an approved ledger entry."""

from __future__ import annotations

import argparse
import hashlib
import re
import subprocess
import sys
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Sequence


ENTRY_HEADING = re.compile(r"^### Asset entry: `(?P<id>ALI-\d{4})`")
COMPACT_ID = re.compile(r"^ALI-\d{4}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")
SHARED_REVIEW = re.compile(r"^- review: `(?P<status>[a-z-]+)`")
KNOWN_STATUSES = {
    "draft",
    "needs-evidence",
    "needs-review",
    "blocked",
    "approved",
    "superseded",
}


@dataclass(frozen=True)
class LedgerEntry:
    entry_id: str
    destination: str
    destination_sha256: str
    status: str


@dataclass(frozen=True)
class AuditResult:
    payload_count: int
    status_counts: Counter[str]
    missing_entries: tuple[str, ...]
    nonapproved_entries: tuple[LedgerEntry, ...]
    hash_mismatches: tuple[tuple[LedgerEntry, str], ...]
    ledger_only_entries: tuple[LedgerEntry, ...]
    errors: tuple[str, ...]

    @property
    def approved(self) -> bool:
        return not (
            self.missing_entries
            or self.nonapproved_entries
            or self.hash_mismatches
            or self.errors
        )


def clean_cell(value: str) -> str:
    value = value.strip()
    if len(value) >= 2 and value.startswith("`") and value.endswith("`"):
        value = value[1:-1]
    return value.strip()


def table_cells(line: str) -> list[str]:
    if not line.startswith("|") or not line.rstrip().endswith("|"):
        return []
    return [clean_cell(cell) for cell in line.strip().strip("|").split("|")]


def parse_ledger(text: str) -> tuple[list[LedgerEntry], list[str]]:
    """Parse both the expanded and compact ledger entry formats."""
    lines = text.splitlines()
    entries: list[LedgerEntry] = []
    errors: list[str] = []
    shared_status: str | None = None
    index = 0

    while index < len(lines):
        line = lines[index]
        if line.startswith("## "):
            shared_status = None

        shared_match = SHARED_REVIEW.match(line)
        if shared_match:
            shared_status = shared_match.group("status")

        heading_match = ENTRY_HEADING.match(line)
        if heading_match:
            entry_id = heading_match.group("id")
            fields: dict[str, str] = {}
            index += 1
            while index < len(lines) and not lines[index].startswith("##"):
                cells = table_cells(lines[index])
                if len(cells) == 2 and cells[0] not in {"Field", "---"}:
                    fields[cells[0]] = cells[1]
                index += 1

            destination = fields.get("Destination path", "")
            destination_sha256 = fields.get("Destination SHA-256", "")
            status = fields.get("Review status", "")
            entries.append(
                LedgerEntry(entry_id, destination, destination_sha256, status)
            )
            continue

        cells = table_cells(line)
        if len(cells) >= 5 and COMPACT_ID.fullmatch(cells[0]):
            if shared_status is None:
                errors.append(f"{cells[0]}: compact entry has no shared review status")
            entries.append(
                LedgerEntry(cells[0], cells[2], cells[3], shared_status or "")
            )
        index += 1

    seen_ids: dict[str, LedgerEntry] = {}
    seen_destinations: dict[str, LedgerEntry] = {}
    for entry in entries:
        if entry.entry_id in seen_ids:
            errors.append(f"{entry.entry_id}: duplicate stable entry ID")
        seen_ids[entry.entry_id] = entry

        if not entry.destination.startswith("assets/"):
            errors.append(
                f"{entry.entry_id}: invalid destination {entry.destination!r}; "
                "expected an assets/ path"
            )
        elif entry.destination in seen_destinations:
            other = seen_destinations[entry.destination]
            errors.append(
                f"{entry.entry_id}: destination duplicates {other.entry_id}: "
                f"{entry.destination}"
            )
        seen_destinations[entry.destination] = entry

        if not SHA256.fullmatch(entry.destination_sha256):
            errors.append(
                f"{entry.entry_id}: invalid destination SHA-256 "
                f"{entry.destination_sha256!r}"
            )
        if entry.status not in KNOWN_STATUSES:
            errors.append(f"{entry.entry_id}: invalid review status {entry.status!r}")

    return entries, errors


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


def file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def audit(
    repo_root: Path,
    entries: Sequence[LedgerEntry],
    payload_paths: Iterable[str],
    parse_errors: Iterable[str] = (),
) -> AuditResult:
    payload = set(payload_paths)
    by_destination = {entry.destination: entry for entry in entries}
    missing = tuple(sorted(payload - by_destination.keys()))
    ledger_only = tuple(
        sorted(
            (entry for entry in entries if entry.destination not in payload),
            key=lambda entry: entry.destination,
        )
    )
    nonapproved = tuple(
        sorted(
            (
                by_destination[path]
                for path in payload & by_destination.keys()
                if by_destination[path].status != "approved"
            ),
            key=lambda entry: entry.destination,
        )
    )

    mismatches: list[tuple[LedgerEntry, str]] = []
    errors = list(parse_errors)
    for path in sorted(payload & by_destination.keys()):
        entry = by_destination[path]
        asset_path = repo_root / path
        if not asset_path.is_file():
            errors.append(f"tracked asset is not a readable regular file: {path}")
            continue
        actual = file_sha256(asset_path)
        if actual != entry.destination_sha256:
            mismatches.append((entry, actual))

    return AuditResult(
        payload_count=len(payload),
        status_counts=Counter(entry.status for entry in entries),
        missing_entries=missing,
        nonapproved_entries=nonapproved,
        hash_mismatches=tuple(mismatches),
        ledger_only_entries=ledger_only,
        errors=tuple(errors),
    )


def print_items(label: str, items: Sequence[str], limit: int) -> None:
    if not items:
        return
    print(f"\n{label} ({len(items)}):")
    for item in items[:limit]:
        print(f"  - {item}")
    if len(items) > limit:
        print(f"  ... {len(items) - limit} more (use --show-all)")


def print_report(result: AuditResult, entry_count: int, limit: int) -> None:
    statuses = ", ".join(
        f"{status}={count}" for status, count in sorted(result.status_counts.items())
    )
    print("Asset rights audit")
    print(f"  tracked release assets: {result.payload_count}")
    print(f"  ledger entries: {entry_count} ({statuses})")
    print(f"  missing ledger entries: {len(result.missing_entries)}")
    print(f"  non-approved payload entries: {len(result.nonapproved_entries)}")
    print(f"  hash mismatches: {len(result.hash_mismatches)}")
    print(f"  ledger entries outside payload: {len(result.ledger_only_entries)}")
    print(f"  ledger errors: {len(result.errors)}")

    print_items("Missing ledger entries", result.missing_entries, limit)
    print_items(
        "Non-approved payload entries",
        tuple(
            f"{entry.entry_id} [{entry.status}] {entry.destination}"
            for entry in result.nonapproved_entries
        ),
        limit,
    )
    print_items(
        "Hash mismatches",
        tuple(
            f"{entry.entry_id} {entry.destination}: "
            f"ledger={entry.destination_sha256}, actual={actual}"
            for entry, actual in result.hash_mismatches
        ),
        limit,
    )
    print_items(
        "Ledger entries outside the tracked payload",
        tuple(
            f"{entry.entry_id} [{entry.status}] {entry.destination}"
            for entry in result.ledger_only_entries
        ),
        limit,
    )
    print_items("Ledger errors", result.errors, limit)

    if result.approved:
        print("\nPASS: every tracked release asset is approved at its current hash.")
    else:
        print("\nFAIL: release payload contains unresolved asset-rights blockers.")


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--inventory",
        type=Path,
        default=Path("docs/asset-license-inventory.md"),
        help="ledger path relative to the repository root",
    )
    parser.add_argument(
        "--report-only",
        action="store_true",
        help="print blockers but return success (for ongoing inventory work)",
    )
    parser.add_argument(
        "--show-all",
        action="store_true",
        help="print every blocker instead of the first 20 in each category",
    )
    return parser.parse_args(argv)


def repository_root() -> Path:
    result = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        check=True,
        capture_output=True,
        text=True,
    )
    return Path(result.stdout.strip())


def main(argv: Sequence[str] = ()) -> int:
    args = parse_args(argv)
    repo_root = repository_root()
    inventory_path = args.inventory
    if not inventory_path.is_absolute():
        inventory_path = repo_root / inventory_path
    entries, errors = parse_ledger(inventory_path.read_text(encoding="utf-8"))
    result = audit(repo_root, entries, tracked_asset_paths(repo_root), errors)
    print_report(result, len(entries), sys.maxsize if args.show_all else 20)
    if result.errors:
        return 2
    return 0 if result.approved or args.report_only else 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
