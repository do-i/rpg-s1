import contextlib
import io
import subprocess
import tempfile
import unittest
from pathlib import Path

from scripts.sync_item_icons import main

SCENARIO = "assets/scenarios/demo"
ICONS = f"{SCENARIO}/media/images/icons/items"
BODY = """- id: tunic
  name: Tunic
  type: body
  slot_category: leather
  stats:
    con: 1
  description: "A tunic."

- id: gown
  name: Gown
  type: body
  slot_category: robe
  stats:
    con: 1
  description: "A gown."
"""
WEAPONS = """- id: blade
  type: weapon
  slot_category: sword
"""
MATERIALS = """- id: pelt
  type: material
"""


class SyncItemIconsTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        self.git("init", "--quiet")
        self.write(f"{SCENARIO}/data/items/body.yaml", BODY)
        self.write(f"{SCENARIO}/data/items/weapons.yaml", WEAPONS)
        self.write(f"{SCENARIO}/data/items/materials.yaml", MATERIALS)
        self.write(f"{SCENARIO}/data/items/field_use.yaml", "- item: tunic\n  effect: none\n")
        icons = {
            "equipment/tunic.png",
            "equipment/gown.png",
            "equipment/blade.png",
            "non_equipment/pelt.png",
        }
        rows = "".join(f"| ALI | `{ICONS}/{icon}` | hash |\n" for icon in sorted(icons))
        for icon in icons:
            self.write(f"{ICONS}/{icon}", "png")
        self.write("docs/asset-license-inventory.md", rows)
        self.git("add", ".")

    def git(self, *args: str) -> str:
        return subprocess.run(
            ["git", *args], cwd=self.root, check=True, capture_output=True, text=True
        ).stdout

    def write(self, relative: str, text: str) -> None:
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

    def read(self, relative: str) -> str:
        return (self.root / relative).read_text(encoding="utf-8")

    def move(self, old: str, new: str) -> None:
        target = self.root / ICONS / new
        target.parent.mkdir(parents=True, exist_ok=True)
        (self.root / ICONS / old).rename(target)

    def sync(self, mode: str) -> tuple[int, str]:
        err = io.StringIO()
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(err):
            status = main([mode, "--repo-root", str(self.root)])
        return status, err.getvalue()

    def test_unchanged_layout_is_clean_and_writes_nothing(self):
        self.assertEqual(self.sync("--check")[0], 0)
        self.assertEqual(self.sync("--write")[0], 0)
        self.assertEqual(self.read(f"{SCENARIO}/data/items/body.yaml"), BODY)

    def test_moving_an_icon_labels_only_that_item(self):
        self.move("equipment/gown.png", "equipment/female/gown.png")
        self.assertEqual(self.sync("--check")[0], 1)
        self.assertEqual(self.sync("--write")[0], 0)
        self.assertEqual(
            self.read(f"{SCENARIO}/data/items/body.yaml"),
            BODY.replace("slot_category: robe\n", "slot_category: robe\n  gender: female\n"),
        )
        self.assertEqual(self.sync("--check")[0], 0)

    def test_moving_an_icon_back_to_equipment_removes_the_label(self):
        self.move("equipment/gown.png", "equipment/male/gown.png")
        self.sync("--write")
        self.move("equipment/male/gown.png", "equipment/gown.png")
        self.assertEqual(self.sync("--write")[0], 0)
        self.assertEqual(self.read(f"{SCENARIO}/data/items/body.yaml"), BODY)

    def test_labels_two_items_in_one_catalog_without_shifting_either(self):
        self.move("equipment/tunic.png", "equipment/male/tunic.png")
        self.move("equipment/gown.png", "equipment/female/gown.png")
        self.assertEqual(self.sync("--write")[0], 0)
        self.assertEqual(
            self.read(f"{SCENARIO}/data/items/body.yaml"),
            BODY.replace("slot_category: leather\n", "slot_category: leather\n  gender: male\n")
            .replace("slot_category: robe\n", "slot_category: robe\n  gender: female\n"),
        )

    def test_move_repoints_the_ledger_and_stages_only_the_icon(self):
        self.git("commit", "--quiet", "-m", "base", "--no-gpg-sign",
                 "--author", "t <t@example.invalid>")
        self.move("equipment/gown.png", "equipment/female/gown.png")
        self.assertEqual(self.sync("--write")[0], 0)
        self.assertIn(
            f"`{ICONS}/equipment/female/gown.png`",
            self.read("docs/asset-license-inventory.md"),
        )
        self.assertNotIn(
            f"`{ICONS}/equipment/gown.png`", self.read("docs/asset-license-inventory.md")
        )
        tracked = self.git("ls-files", ICONS).splitlines()
        self.assertIn(f"{ICONS}/equipment/female/gown.png", tracked)
        self.assertNotIn(f"{ICONS}/equipment/gown.png", tracked)
        staged = self.git("diff", "--cached", "--name-only", "--no-renames").splitlines()
        self.assertEqual(
            sorted(staged),
            [f"{ICONS}/equipment/female/gown.png", f"{ICONS}/equipment/gown.png"],
        )

    def assert_rejected(self, expected: str) -> None:
        before = self.read(f"{SCENARIO}/data/items/body.yaml")
        status, errors = self.sync("--write")
        self.assertEqual(status, 2, errors)
        self.assertIn(expected, errors)
        self.assertEqual(self.read(f"{SCENARIO}/data/items/body.yaml"), before)

    def test_rejects_gender_folder_for_anything_but_body_armor(self):
        self.move("equipment/blade.png", "equipment/male/blade.png")
        self.assert_rejected("only body armor can be gender-specific")

    def test_rejects_equipment_outside_the_equipment_folder(self):
        self.move("equipment/gown.png", "non_equipment/gown.png")
        self.assert_rejected("belongs in equipment/")

    def test_rejects_non_equipment_in_a_gender_folder(self):
        self.move("non_equipment/pelt.png", "equipment/female/pelt.png")
        self.assert_rejected("belongs in non_equipment/")

    def test_rejects_an_unknown_folder(self):
        self.move("equipment/gown.png", "equipment/unisex/gown.png")
        self.assert_rejected("belongs in equipment/")

    def test_rejects_duplicate_and_orphan_icons(self):
        self.write(f"{ICONS}/non_equipment/gown.png", "png")
        self.assert_rejected("two icons share this id")
        (self.root / ICONS / "non_equipment/gown.png").unlink()
        self.write(f"{ICONS}/non_equipment/ghost.png", "png")
        self.assert_rejected("no item has the id `ghost`")

    def test_rejects_an_item_without_an_icon(self):
        (self.root / ICONS / "equipment/gown.png").unlink()
        self.assert_rejected("gown: no icon")

    def test_a_repository_without_item_icons_is_a_no_op(self):
        with tempfile.TemporaryDirectory() as empty:
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(main(["--write", "--repo-root", empty]), 0)


if __name__ == "__main__":
    unittest.main()
