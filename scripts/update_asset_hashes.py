#!/usr/bin/env python3
"""Refresh stale SHA-256 values of approved home-grown entries in the asset ledger.

Only the hash cell of an existing row in the `Home-grown assets (MIT)` table is
ever rewritten. Third-party hashes, approvals, and every other byte of the
ledger are left alone, and the whole input is validated before anything is
written. Run with exactly one of --write, --check, or --dry-run.

Exit status: 0 success; 1 --check found stale hashes; 2 invalid input;
3 the ledger or an asset changed underneath the update, or the lock was busy.
"""

from __future__ import annotations

import argparse
import errno
import fcntl
import os
import stat
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import Sequence

try:
    from scripts.check_asset_rights import (
        LedgerEntry,
        file_sha256,
        parse_ledger,
        table_cells,
    )
    from scripts.release_assets import release_asset_paths
except ModuleNotFoundError:  # Direct execution adds scripts/, not the repo root.
    from check_asset_rights import (  # type: ignore[no-redef]
        LedgerEntry,
        file_sha256,
        parse_ledger,
        table_cells,
    )
    from release_assets import release_asset_paths  # type: ignore[no-redef]


DEFAULT_INVENTORY = Path("docs/asset-license-inventory.md")
LOCK_PATH = Path("temp/update-asset-hashes.lock")
DEFAULT_LOCK_TIMEOUT = 30.0
HOME_GROWN_SECTION = "Home-grown assets (MIT)"
HOME_GROWN_HEADER = ["ID", "Origin", "Destination path", "SHA-256", "Kind/name"]
# A compact row is `| ID | Origin | Destination | SHA-256 | Kind |`; splitting the
# raw line on "|" puts the hash in this part (part 0 is before the leading pipe).
HASH_PART = 4
LFS_POINTER_PREFIX = b"version https://git-lfs.github.com/spec/v1"

EXIT_OK = 0
EXIT_STALE = 1
EXIT_INVALID = 2
EXIT_CONFLICT = 3


class UpdateError(Exception):
    """The input is invalid; nothing was written."""


class ConflictError(Exception):
    """Something changed underneath the update, or the lock stayed busy."""


@dataclass(frozen=True)
class Replacement:
    entry: LedgerEntry
    actual_sha256: str


@dataclass(frozen=True)
class FileSnapshot:
    device: int
    inode: int
    size: int
    mtime_ns: int

    @classmethod
    def of(cls, path: Path) -> FileSnapshot:
        info = path.lstat()
        return cls(info.st_dev, info.st_ino, info.st_size, info.st_mtime_ns)


@dataclass(frozen=True)
class Plan:
    """Everything an update would change, and what it was computed from."""

    ledger_bytes: bytes
    replacements: tuple[Replacement, ...]
    asset_snapshots: dict[Path, FileSnapshot]


def is_home_grown_row(entry: LedgerEntry, lines: Sequence[str]) -> bool:
    """Whether `entry` is a compact row of the approved home-grown table."""
    if entry.section != HOME_GROWN_SECTION or entry.status != "approved":
        return False
    if not 0 <= entry.line < len(lines):
        return False
    header = entry.line
    while header > 0 and lines[header - 1].startswith("|"):
        header -= 1
    return table_cells(lines[header]) == HOME_GROWN_HEADER


def safe_destination(repo_root: Path, destination: str) -> Path:
    pure = PurePosixPath(destination)
    if (
        pure.is_absolute()
        or destination != pure.as_posix()
        or any(part in {"", ".", ".."} for part in pure.parts)
        or len(pure.parts) < 2
        or pure.parts[0] != "assets"
    ):
        raise UpdateError(f"unsafe asset path: {destination!r}")
    path = repo_root / destination
    real_root = os.path.realpath(repo_root)
    if os.path.commonpath([real_root, os.path.realpath(path.parent)]) != real_root:
        raise UpdateError(f"asset path leaves the repository: {destination}")
    return path


def read_asset(path: Path, destination: str) -> tuple[str, FileSnapshot]:
    """Hashes one payload file, refusing anything but a resolved regular file."""
    try:
        info = path.lstat()
    except FileNotFoundError:
        raise UpdateError(f"tracked asset is missing: {destination}") from None
    if stat.S_ISLNK(info.st_mode):
        raise UpdateError(f"tracked asset is a symlink: {destination}")
    if not stat.S_ISREG(info.st_mode):
        raise UpdateError(f"tracked asset is not a regular file: {destination}")
    before = FileSnapshot.of(path)
    with path.open("rb") as handle:
        if handle.read(len(LFS_POINTER_PREFIX)) == LFS_POINTER_PREFIX:
            raise UpdateError(
                f"tracked asset is an unresolved Git LFS pointer: {destination} "
                "(run `git lfs pull`)"
            )
    digest = file_sha256(path)
    if FileSnapshot.of(path) != before:
        raise ConflictError(f"asset changed while it was being hashed: {destination}")
    return digest, before


def plan_update(repo_root: Path, ledger_path: Path) -> Plan:
    """Validates the ledger and payload and computes every eligible replacement."""
    if ledger_path.is_symlink() or not ledger_path.is_file():
        raise UpdateError(f"ledger is not a regular file: {ledger_path}")
    ledger_bytes = ledger_path.read_bytes()
    try:
        text = ledger_bytes.decode("utf-8")
    except UnicodeDecodeError as error:
        raise UpdateError(f"ledger is not UTF-8: {error}") from None

    entries, parse_errors = parse_ledger(text)
    payload, selection_errors = release_asset_paths(repo_root)
    errors = [*parse_errors, *selection_errors]
    by_destination = {entry.destination: entry for entry in entries}
    errors.extend(
        f"tracked asset has no ledger entry; record its provenance row first: {path}"
        for path in payload
        if path not in by_destination
    )
    if errors:
        raise UpdateError("\n".join(errors))

    lines = text.splitlines()
    replacements: list[Replacement] = []
    snapshots: dict[Path, FileSnapshot] = {}
    for destination in payload:
        entry = by_destination[destination]
        path = safe_destination(repo_root, destination)
        actual, snapshot = read_asset(path, destination)
        snapshots[path] = snapshot
        if actual == entry.destination_sha256:
            continue
        if not is_home_grown_row(entry, lines):
            errors.append(
                f"{entry.entry_id} is not an approved home-grown row and its hash "
                f"is stale; update it by hand and review it: {destination} "
                f"(ledger={entry.destination_sha256}, actual={actual})"
            )
            continue
        replacements.append(Replacement(entry, actual))
    if errors:
        raise UpdateError("\n".join(errors))
    return Plan(ledger_bytes, tuple(replacements), snapshots)


def apply_replacements(
    ledger_bytes: bytes, replacements: Sequence[Replacement]
) -> bytes:
    """Rewrites exactly the hash cell of each replaced row, keeping every other byte."""
    lines = ledger_bytes.decode("utf-8").splitlines(keepends=True)
    for replacement in replacements:
        entry = replacement.entry
        line = lines[entry.line]
        body = line.rstrip("\r\n")
        parts = body.split("|")
        old_cell = f"`{entry.destination_sha256}`"
        if len(parts) <= HASH_PART or parts[HASH_PART].strip() != old_cell:
            raise UpdateError(f"{entry.entry_id}: hash cell not found where parsed")
        parts[HASH_PART] = parts[HASH_PART].replace(
            old_cell, f"`{replacement.actual_sha256}`"
        )
        lines[entry.line] = "|".join(parts) + line[len(body) :]
    return "".join(lines).encode("utf-8")


def acquire_lock(repo_root: Path, timeout: float):
    """Takes the per-worktree updater lock, waiting at most `timeout` seconds."""
    lock_path = repo_root / LOCK_PATH
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    handle = lock_path.open("a+b")
    deadline = time.monotonic() + timeout
    while True:
        try:
            fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
            return handle
        except OSError as error:
            if error.errno not in (errno.EAGAIN, errno.EACCES):
                handle.close()
                raise
            if time.monotonic() >= deadline:
                handle.close()
                raise ConflictError(
                    f"another updater held {LOCK_PATH} for {timeout:g}s; try again"
                ) from None
            time.sleep(0.05)


def verify_unchanged(ledger_path: Path, plan: Plan) -> None:
    if ledger_path.read_bytes() != plan.ledger_bytes:
        raise ConflictError("the ledger changed during the update; nothing was written")
    for path, snapshot in plan.asset_snapshots.items():
        if FileSnapshot.of(path) != snapshot:
            raise ConflictError(
                f"an asset changed during the update; nothing was written: {path}"
            )


def fsync_directory(directory: Path) -> None:
    descriptor = os.open(directory, os.O_RDONLY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def publish(ledger_path: Path, plan: Plan, new_bytes: bytes) -> list[str]:
    """Atomically replaces the ledger, returning warnings raised after replacement.

    Up to the rename, any failure removes the temporary sibling and leaves the
    original ledger untouched. Once the rename has happened the update is
    published; a failure to make it durable is reported, not undone.
    """
    mode = stat.S_IMODE(ledger_path.stat().st_mode)
    descriptor, temp_name = tempfile.mkstemp(
        prefix=f".{ledger_path.name}.", suffix=".tmp", dir=ledger_path.parent
    )
    temp_path = Path(temp_name)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(new_bytes)
            handle.flush()
            os.fsync(handle.fileno())
        os.chmod(temp_path, mode)
        verify_unchanged(ledger_path, plan)
        os.replace(temp_path, ledger_path)
    except OSError as error:
        temp_path.unlink(missing_ok=True)
        raise UpdateError(f"could not write the ledger: {error}") from error
    except BaseException:
        temp_path.unlink(missing_ok=True)
        raise
    try:
        fsync_directory(ledger_path.parent)
    except OSError as error:
        return [f"ledger was replaced but its directory could not be synced: {error}"]
    return []


def describe(replacements: Sequence[Replacement]) -> list[str]:
    return [
        f"{replacement.entry.entry_id} {replacement.entry.destination}: "
        f"{replacement.entry.destination_sha256} -> {replacement.actual_sha256}"
        for replacement in replacements
    ]


def run(repo_root: Path, ledger_path: Path, mode: str, lock_timeout: float) -> int:
    lock = acquire_lock(repo_root, lock_timeout) if mode == "write" else None
    try:
        # Planned after the lock is held, so a waiting updater re-reads whatever
        # the previous holder published.
        plan = plan_update(repo_root, ledger_path)
        changes = describe(plan.replacements)
        if not changes:
            print("Asset hashes are current.")
            return EXIT_OK
        if mode == "check":
            print(f"Stale home-grown asset hashes ({len(changes)}):")
            for change in changes:
                print(f"  - {change}")
            print("Run this script with --write and commit the ledger.")
            return EXIT_STALE
        if mode == "dry-run":
            print(f"Would refresh {len(changes)} home-grown asset hash(es):")
            for change in changes:
                print(f"  - {change}")
            return EXIT_OK

        new_bytes = apply_replacements(plan.ledger_bytes, plan.replacements)
        warnings = publish(ledger_path, plan, new_bytes)
        print(f"Refreshed {len(changes)} home-grown asset hash(es):")
        for change in changes:
            print(f"  - {change}")
        for warning in warnings:
            print(f"warning: {warning}", file=sys.stderr)
        return EXIT_OK
    finally:
        if lock is not None:
            lock.close()


def repository_root() -> Path:
    result = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        cwd=Path(__file__).resolve().parent,
        check=True,
        capture_output=True,
        text=True,
    )
    return Path(result.stdout.strip())


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    mode = parser.add_mutually_exclusive_group(required=True)
    for flag, help_text in (
        ("--write", "refresh stale eligible hashes in place"),
        ("--check", "read-only; exit 1 if any eligible hash is stale"),
        ("--dry-run", "read-only; print the replacements --write would make"),
    ):
        mode.add_argument(
            flag,
            dest="mode",
            action="store_const",
            const=flag.removeprefix("--"),
            help=help_text,
        )
    parser.add_argument(
        "--repo-root",
        type=Path,
        help="repository to update (default: the one holding this script)",
    )
    parser.add_argument(
        "--inventory",
        type=Path,
        default=DEFAULT_INVENTORY,
        help="ledger path relative to the repository root",
    )
    parser.add_argument(
        "--lock-timeout",
        type=float,
        default=DEFAULT_LOCK_TIMEOUT,
        help="seconds to wait for another updater (default: %(default)s)",
    )
    return parser.parse_args(argv)


def main(argv: Sequence[str] = ()) -> int:
    args = parse_args(argv)
    repo_root = (args.repo_root or repository_root()).resolve()
    ledger_path = args.inventory
    if not ledger_path.is_absolute():
        ledger_path = repo_root / ledger_path
    try:
        return run(repo_root, ledger_path, args.mode, args.lock_timeout)
    except UpdateError as error:
        print(f"error: {error}", file=sys.stderr)
        print("The ledger was not changed.", file=sys.stderr)
        return EXIT_INVALID
    except ConflictError as error:
        print(f"error: {error}", file=sys.stderr)
        return EXIT_CONFLICT


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
