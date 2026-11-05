import subprocess
import tempfile
import unittest
from pathlib import Path

from scripts.check_docs_allowlist import ALLOWED_DOCS, docs_files, unexpected_docs


class UnexpectedDocsTest(unittest.TestCase):
    def test_allowed_files_pass(self):
        self.assertEqual(unexpected_docs(ALLOWED_DOCS), [])

    def test_new_top_level_file_fails(self):
        self.assertEqual(unexpected_docs(["docs/plan.md"]), ["docs/plan.md"])

    def test_new_screenshot_fails(self):
        path = "docs/screenshots/new.png"
        self.assertEqual(unexpected_docs([path]), [path])

    def test_old_readme_name_fails(self):
        self.assertEqual(unexpected_docs(["docs/README.md"]), ["docs/README.md"])


class DocsFilesTest(unittest.TestCase):
    def test_lists_tracked_and_untracked_but_not_ignored_files(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            subprocess.run(["git", "init", "--quiet"], cwd=root, check=True)
            (root / "docs").mkdir()
            (root / "docs" / "tracked.md").write_text("tracked\n")
            subprocess.run(["git", "add", "docs/tracked.md"], cwd=root, check=True)
            (root / "docs" / "untracked.md").write_text("untracked\n")
            (root / "docs" / "ignored.md").write_text("ignored\n")
            (root / ".gitignore").write_text("docs/ignored.md\n")
            (root / "outside.md").write_text("outside\n")

            self.assertEqual(
                sorted(docs_files(root)),
                ["docs/tracked.md", "docs/untracked.md"],
            )


if __name__ == "__main__":
    unittest.main()
