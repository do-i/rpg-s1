import contextlib
import fcntl
import hashlib
import io
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from scripts import update_asset_hashes as updater
from scripts.check_asset_rights import audit, parse_ledger
from scripts.release_assets import release_asset_paths


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


MAP_V1 = b"<map version='1'/>\n"
MAP_V2 = b"<map version='2'/>\n"
README = b"# assets\n"
SPRITE = b"\x89PNG third-party sprite"
FONT = b"third-party font"

LEDGER = f"""# Asset License Inventory

## Third-party: individually reviewed entries

### Asset entry: `ALI-0001` — font

| Field | Value |
| --- | --- |
| Destination path | `assets/font.ttf` |
| Destination SHA-256 | `{sha(FONT)}` |
| Review status | `approved` |
| Notes/blocker | mentions `assets/map.tmx` and {sha(MAP_V1)} on purpose |

## Third-party: LPC character sprites

- review: `needs-evidence`, unchanged; and

| ID | Source path | Destination path | SHA-256 | Kind/name |
| --- | --- | --- | --- | --- |
| ALI-0002 | src/sprite.png | `assets/sprite.png` | `{sha(SPRITE)}` | image / sprite |

## Home-grown assets (MIT)

- review: `approved`, Owner, 2026-10-11; and

| ID | Origin | Destination path | SHA-256 | Kind/name |
| --- | --- | --- | --- | --- |
| ALI-0003 | author | `assets/README.md` | `{sha(README)}` | markdown / README.md |
| ALI-0004 | author (Tiled) | `assets/map.tmx` | `{sha(MAP_V1)}` | TMX / map |
"""


class FixtureRepo:
    """A throwaway git repository holding a small ledger and payload."""

    def __init__(self, root: Path, ledger: str = LEDGER, newline: str = "\n"):
        self.root = root
        subprocess.run(["git", "init", "-q", str(root)], check=True)
        (root / "assets").mkdir()
        (root / "docs").mkdir()
        self.write("assets/README.md", README)
        self.write("assets/map.tmx", MAP_V1)
        self.write("assets/sprite.png", SPRITE)
        self.write("assets/font.ttf", FONT)
        (root / "release-assets-exclude.txt").write_text("# none\n")
        self.ledger.write_bytes(ledger.replace("\n", newline).encode("utf-8"))
        self.track()

    @property
    def ledger(self) -> Path:
        return self.root / "docs/asset-license-inventory.md"

    def write(self, path: str, data: bytes) -> None:
        (self.root / path).write_bytes(data)

    def track(self) -> None:
        subprocess.run(["git", "add", "assets"], cwd=self.root, check=True)

    def run(self, *args: str) -> tuple[int, str, str]:
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = updater.main(["--repo-root", str(self.root), *args])
        return code, out.getvalue(), err.getvalue()

    def stray_temp_files(self) -> list[Path]:
        return [path for path in self.ledger.parent.iterdir() if path.suffix == ".tmp"]


class UpdaterTestCase(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.repo = FixtureRepo(Path(temp.name))

    def assertLedgerUnchanged(self, before: bytes):
        self.assertEqual(self.repo.ledger.read_bytes(), before)
        self.assertEqual(self.repo.stray_temp_files(), [])


class RefreshTests(UpdaterTestCase):
    def test_changed_home_grown_map_updates_exactly_its_hash_cell(self):
        before = self.repo.ledger.read_bytes()
        self.repo.write("assets/map.tmx", MAP_V2)

        code, out, _ = self.repo.run("--write")

        self.assertEqual(code, updater.EXIT_OK)
        self.assertIn("ALI-0004 assets/map.tmx", out)
        after = self.repo.ledger.read_bytes()
        old_row = f"| `{sha(MAP_V1)}` | TMX / map |".encode()
        new_row = f"| `{sha(MAP_V2)}` | TMX / map |".encode()
        self.assertEqual(after, before.replace(old_row, new_row))
        # The expanded third-party note that quotes the old hash is untouched.
        self.assertEqual(after.count(sha(MAP_V1).encode()), 1)

    def test_crlf_newlines_survive_a_refresh(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        repo = FixtureRepo(Path(temp.name), newline="\r\n")
        repo.write("assets/map.tmx", MAP_V2)

        self.assertEqual(repo.run("--write")[0], updater.EXIT_OK)

        after = repo.ledger.read_bytes()
        self.assertNotIn(b"\n", after.replace(b"\r\n", b""))
        self.assertIn(sha(MAP_V2).encode(), after)

    def test_current_hashes_cause_no_write_and_a_second_update_is_stable(self):
        self.repo.write("assets/map.tmx", MAP_V2)
        self.assertEqual(self.repo.run("--write")[0], updater.EXIT_OK)
        refreshed = self.repo.ledger.read_bytes()
        os.utime(self.repo.ledger, ns=(1_000_000_000, 1_000_000_000))

        code, out, _ = self.repo.run("--write")

        self.assertEqual(code, updater.EXIT_OK)
        self.assertIn("current", out)
        self.assertEqual(self.repo.ledger.read_bytes(), refreshed)
        self.assertEqual(self.repo.ledger.stat().st_mtime_ns, 1_000_000_000)

    def test_refresh_preserves_file_mode(self):
        os.chmod(self.repo.ledger, 0o640)
        self.repo.write("assets/map.tmx", MAP_V2)

        self.assertEqual(self.repo.run("--write")[0], updater.EXIT_OK)

        self.assertEqual(self.repo.ledger.stat().st_mode & 0o777, 0o640)

    def test_checker_agrees_with_refreshed_hashes(self):
        self.repo.write("assets/map.tmx", MAP_V2)
        self.repo.run("--write")

        entries, errors = parse_ledger(self.repo.ledger.read_text(encoding="utf-8"))
        payload, selection_errors = release_asset_paths(self.repo.root)
        result = audit(self.repo.root, entries, payload, (*errors, *selection_errors))

        self.assertEqual(result.errors, ())
        self.assertEqual(result.hash_mismatches, ())


class ReadOnlyModeTests(UpdaterTestCase):
    def test_check_fails_on_stale_hashes_without_writing(self):
        before = self.repo.ledger.read_bytes()
        mtime = self.repo.ledger.stat().st_mtime_ns
        self.repo.write("assets/map.tmx", MAP_V2)

        code, out, _ = self.repo.run("--check")

        self.assertEqual(code, updater.EXIT_STALE)
        self.assertIn("ALI-0004", out)
        self.assertLedgerUnchanged(before)
        self.assertEqual(self.repo.ledger.stat().st_mtime_ns, mtime)

    def test_check_passes_when_current(self):
        self.assertEqual(self.repo.run("--check")[0], updater.EXIT_OK)

    def test_dry_run_previews_exactly_what_write_does(self):
        before = self.repo.ledger.read_bytes()
        self.repo.write("assets/map.tmx", MAP_V2)

        code, preview, _ = self.repo.run("--dry-run")
        self.assertEqual(code, updater.EXIT_OK)
        self.assertLedgerUnchanged(before)

        _, written, _ = self.repo.run("--write")
        def changes(out):
            return [line for line in out.splitlines() if line.startswith("  - ")]

        self.assertEqual(changes(preview), changes(written))
        self.assertEqual(len(changes(preview)), 1)

    def test_mode_is_required(self):
        with self.assertRaises(SystemExit), contextlib.redirect_stderr(io.StringIO()):
            updater.parse_args([])


class InvalidInputTests(UpdaterTestCase):
    def assertRefused(self, fragment: str, mode: str = "--write"):
        before = self.repo.ledger.read_bytes()
        code, _, err = self.repo.run(mode)
        self.assertEqual(code, updater.EXIT_INVALID, err)
        self.assertIn(fragment, err)
        self.assertLedgerUnchanged(before)

    def test_third_party_hash_mismatch_is_refused_even_alongside_a_valid_refresh(self):
        self.repo.write("assets/map.tmx", MAP_V2)
        self.repo.write("assets/sprite.png", b"edited sprite")
        self.assertRefused("ALI-0002 is not an approved home-grown row")

    def test_expanded_entry_mismatch_is_refused(self):
        self.repo.write("assets/font.ttf", b"edited font")
        self.assertRefused("ALI-0001 is not an approved home-grown row", "--check")

    def test_tracked_asset_without_a_row_is_refused(self):
        self.repo.write("assets/new.tmx", b"<map/>")
        self.repo.track()
        self.assertRefused("record its provenance row first: assets/new.tmx")

    def test_malformed_ledger_is_refused(self):
        text = self.repo.ledger.read_text(encoding="utf-8")
        self.repo.ledger.write_text(
            text.replace("| ALI-0004 |", "| ALI-0003 |"), encoding="utf-8"
        )
        self.assertRefused("duplicate stable entry ID")

    def test_unresolved_lfs_pointer_is_refused(self):
        self.repo.write(
            "assets/sprite.png",
            b"version https://git-lfs.github.com/spec/v1\noid sha256:abc\nsize 3\n",
        )
        self.assertRefused("unresolved Git LFS pointer")

    def test_symlinked_asset_is_refused(self):
        (self.repo.root / "assets/map.tmx").unlink()
        (self.repo.root / "assets/map.tmx").symlink_to(
            self.repo.root / "assets/README.md"
        )
        self.assertRefused("is a symlink")

    def test_missing_tracked_asset_is_refused(self):
        (self.repo.root / "assets/map.tmx").unlink()
        self.assertRefused("tracked asset is missing: assets/map.tmx")

    def test_home_grown_section_with_an_unapproved_status_is_not_eligible(self):
        text = self.repo.ledger.read_text(encoding="utf-8")
        self.repo.ledger.write_text(
            text.replace("review: `approved`, Owner", "review: `needs-review`, Owner"),
            encoding="utf-8",
        )
        self.repo.write("assets/map.tmx", MAP_V2)
        self.assertRefused("ALI-0004 is not an approved home-grown row")

    def test_home_grown_table_with_another_schema_is_not_eligible(self):
        text = self.repo.ledger.read_text(encoding="utf-8")
        self.repo.ledger.write_text(
            text.replace("| ID | Origin |", "| ID | Source path |"), encoding="utf-8"
        )
        self.repo.write("assets/map.tmx", MAP_V2)
        self.assertRefused("ALI-0004 is not an approved home-grown row")


class ConcurrencyTests(UpdaterTestCase):
    def test_lock_contention_times_out_without_writing(self):
        before = self.repo.ledger.read_bytes()
        self.repo.write("assets/map.tmx", MAP_V2)
        lock_path = self.repo.root / updater.LOCK_PATH
        lock_path.parent.mkdir(parents=True, exist_ok=True)
        with lock_path.open("a+b") as held:
            fcntl.flock(held, fcntl.LOCK_EX)
            code, _, err = self.repo.run("--write", "--lock-timeout", "0.1")

        self.assertEqual(code, updater.EXIT_CONFLICT)
        self.assertIn("another updater", err)
        self.assertLedgerUnchanged(before)

    def test_read_only_modes_do_not_wait_for_the_lock(self):
        lock_path = self.repo.root / updater.LOCK_PATH
        lock_path.parent.mkdir(parents=True, exist_ok=True)
        with lock_path.open("a+b") as held:
            fcntl.flock(held, fcntl.LOCK_EX)
            self.assertEqual(self.repo.run("--check", "--lock-timeout", "0")[0], 0)

    def test_concurrent_ledger_edit_aborts_the_publish(self):
        self.repo.write("assets/map.tmx", MAP_V2)
        real_apply = updater.apply_replacements

        def apply_then_edit(ledger_bytes, replacements):
            # An editor saves the ledger after the plan was computed.
            self.repo.ledger.write_bytes(ledger_bytes + b"\nedited elsewhere\n")
            return real_apply(ledger_bytes, replacements)

        with mock.patch.object(updater, "apply_replacements", apply_then_edit):
            code, _, err = self.repo.run("--write")

        self.assertEqual(code, updater.EXIT_CONFLICT)
        self.assertIn("ledger changed", err)
        self.assertTrue(self.repo.ledger.read_bytes().endswith(b"edited elsewhere\n"))
        self.assertNotIn(sha(MAP_V2).encode(), self.repo.ledger.read_bytes())
        self.assertEqual(self.repo.stray_temp_files(), [])

    def test_concurrent_asset_edit_aborts_the_publish(self):
        before = self.repo.ledger.read_bytes()
        self.repo.write("assets/map.tmx", MAP_V2)
        real_apply = updater.apply_replacements

        def apply_then_edit(ledger_bytes, replacements):
            self.repo.write("assets/map.tmx", b"<map version='3' longer='yes'/>\n")
            return real_apply(ledger_bytes, replacements)

        with mock.patch.object(updater, "apply_replacements", apply_then_edit):
            code, _, err = self.repo.run("--write")

        self.assertEqual(code, updater.EXIT_CONFLICT)
        self.assertIn("asset changed", err)
        self.assertLedgerUnchanged(before)

    def test_failure_before_replacement_keeps_the_original(self):
        before = self.repo.ledger.read_bytes()
        self.repo.write("assets/map.tmx", MAP_V2)

        with mock.patch.object(updater.os, "replace", side_effect=OSError("disk full")):
            code, _, err = self.repo.run("--write")

        self.assertEqual(code, updater.EXIT_INVALID)
        self.assertIn("could not write the ledger: disk full", err)
        self.assertIn("not changed", err)
        self.assertLedgerUnchanged(before)

    def test_durability_failure_after_replacement_counts_as_published(self):
        self.repo.write("assets/map.tmx", MAP_V2)

        with mock.patch.object(
            updater, "fsync_directory", side_effect=OSError("sync failed")
        ):
            code, out, err = self.repo.run("--write")

        self.assertEqual(code, updater.EXIT_OK)
        self.assertIn("Refreshed 1", out)
        self.assertIn("warning: ledger was replaced", err)
        self.assertIn(sha(MAP_V2).encode(), self.repo.ledger.read_bytes())
        self.assertEqual(self.repo.stray_temp_files(), [])


if __name__ == "__main__":
    unittest.main()
