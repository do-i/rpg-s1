import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from scripts.test_update_asset_hashes import MAP_V1, MAP_V2, FixtureRepo, sha

SCRIPTS = Path(__file__).resolve().parent
WRAPPED = (
    "build.sh",
    "run.sh",
    "update_asset_hashes.py",
    "check_asset_rights.py",
    "release_assets.py",
)

# Records what Cargo would have seen: its arguments one per line, its working
# directory, and the ledger as it stood when Cargo started.
FAKE_CARGO = """#!/usr/bin/env bash
log="$FAKE_CARGO_LOG"
printf '%s\\n' "$@" > "$log.args"
pwd > "$log.cwd"
cp docs/asset-license-inventory.md "$log.ledger"
exit "${FAKE_CARGO_EXIT:-0}"
"""


class WrapperTestCase(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        base = Path(temp.name)
        self.repo = FixtureRepo(base / "repo")
        (self.repo.root / "scripts").mkdir()
        for name in WRAPPED:
            shutil.copy2(SCRIPTS / name, self.repo.root / "scripts" / name)

        bin_dir = base / "bin"
        bin_dir.mkdir()
        cargo = bin_dir / "cargo"
        cargo.write_text(FAKE_CARGO)
        cargo.chmod(0o755)
        self.log = base / "cargo"
        self.outside = base / "elsewhere"
        self.outside.mkdir()
        self.env = {
            **os.environ,
            "PATH": f"{bin_dir}{os.pathsep}{os.environ['PATH']}",
            "FAKE_CARGO_LOG": str(self.log),
        }

    def wrap(self, script: str, *args: str, cargo_exit: int = 0):
        return subprocess.run(
            [str(self.repo.root / "scripts" / script), *args],
            cwd=self.outside,
            env={**self.env, "FAKE_CARGO_EXIT": str(cargo_exit)},
            capture_output=True,
            text=True,
        )

    def cargo_ran(self) -> bool:
        return Path(f"{self.log}.args").exists()

    def cargo_args(self) -> list[str]:
        return Path(f"{self.log}.args").read_text().splitlines()


class BuildWrapperTests(WrapperTestCase):
    def test_edited_map_is_refreshed_before_cargo_starts_from_outside_the_repo(self):
        self.repo.write("assets/map.tmx", MAP_V2)

        result = self.wrap("build.sh")

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.cargo_args(), ["build"])
        self.assertEqual(
            Path(f"{self.log}.cwd").read_text().strip(), str(self.repo.root)
        )
        seen = Path(f"{self.log}.ledger").read_text()
        self.assertIn(sha(MAP_V2), seen)
        self.assertNotIn(f"`{sha(MAP_V1)}` | TMX", seen)
        self.assertIn("ALI-0004", result.stdout)

    def test_refresh_runs_even_when_nothing_else_changed(self):
        # A warm build: Cargo would have nothing to do, but the map still moved on.
        self.assertEqual(self.wrap("build.sh").returncode, 0)
        self.repo.write("assets/map.tmx", MAP_V2)

        self.assertEqual(self.wrap("build.sh").returncode, 0)

        self.assertIn(sha(MAP_V2), self.repo.ledger.read_text())

    def test_arguments_reach_cargo_unchanged(self):
        args = ["--release", "--locked", "--workspace", "--target", "x86 64", ""]

        result = self.wrap("build.sh", *args)

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.cargo_args(), ["build", *args])

    def test_cargo_exit_status_reaches_the_caller(self):
        self.assertEqual(self.wrap("build.sh", cargo_exit=7).returncode, 7)

    def test_failed_update_keeps_cargo_from_starting(self):
        before = self.repo.ledger.read_bytes()
        self.repo.write("assets/sprite.png", b"edited third-party sprite")

        result = self.wrap("build.sh")

        self.assertEqual(result.returncode, 2)
        self.assertIn("ALI-0002", result.stderr)
        self.assertFalse(self.cargo_ran())
        self.assertEqual(self.repo.ledger.read_bytes(), before)

    def test_refresh_is_never_staged(self):
        self.repo.write("assets/map.tmx", MAP_V2)

        self.wrap("build.sh")

        staged = subprocess.run(
            ["git", "diff", "--cached", "--name-only", "--", "docs"],
            cwd=self.repo.root,
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertEqual(staged.stdout, "")


class RunWrapperTests(WrapperTestCase):
    def test_run_refreshes_then_forwards_to_cargo_run(self):
        self.repo.write("assets/map.tmx", MAP_V2)

        result = self.wrap("run.sh", "--", "play", "rusted_kingdoms", "--seed", "1")

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            self.cargo_args(),
            ["run", "--", "play", "rusted_kingdoms", "--seed", "1"],
        )
        self.assertIn(sha(MAP_V2), Path(f"{self.log}.ledger").read_text())

    def test_failed_update_keeps_the_game_from_launching(self):
        self.repo.write("assets/font.ttf", b"edited third-party font")

        self.assertEqual(self.wrap("run.sh").returncode, 2)
        self.assertFalse(self.cargo_ran())


if __name__ == "__main__":
    unittest.main()
