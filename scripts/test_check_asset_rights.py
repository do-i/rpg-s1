import hashlib
import tempfile
import unittest
from pathlib import Path

from scripts.check_asset_rights import LedgerEntry, audit, parse_ledger


class ParseLedgerTests(unittest.TestCase):
    def test_parses_expanded_and_compact_entries(self):
        digest = "a" * 64
        entries, errors = parse_ledger(
            f"""
### Asset entry: `ALI-0001` — expanded

| Field | Value |
| --- | --- |
| Destination path | `assets/expanded.png` |
| Destination SHA-256 | `{digest}` |
| Review status | `approved` |

## Compact entries

- review: `needs-evidence`, reviewer, 2026-09-06;

| ID | Source path | Destination path | SHA-256 | Kind/name |
| --- | --- | --- | --- | --- |
| ALI-0002 | `source.png` | `assets/compact.png` | `{digest}` | image |
"""
        )

        self.assertEqual(errors, [])
        self.assertEqual(
            entries,
            [
                LedgerEntry("ALI-0001", "assets/expanded.png", digest, "approved"),
                LedgerEntry(
                    "ALI-0002", "assets/compact.png", digest, "needs-evidence"
                ),
            ],
        )

    def test_reports_duplicate_destination_and_invalid_fields(self):
        entries, errors = parse_ledger(
            """
### Asset entry: `ALI-0001` — first
| Destination path | `assets/shared.png` |
| Destination SHA-256 | `bad` |
| Review status | `invented` |
### Asset entry: `ALI-0002` — second
| Destination path | `assets/shared.png` |
| Destination SHA-256 | `bad` |
| Review status | `approved` |
"""
        )

        self.assertEqual(len(entries), 2)
        self.assertTrue(any("destination duplicates" in error for error in errors))
        self.assertTrue(any("invalid destination SHA-256" in error for error in errors))
        self.assertTrue(any("invalid review status" in error for error in errors))


class AuditTests(unittest.TestCase):
    def test_requires_approved_matching_entries_for_every_payload_file(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            (root / "assets").mkdir()
            (root / "assets/approved.txt").write_text("approved", encoding="utf-8")
            (root / "assets/pending.txt").write_text("pending", encoding="utf-8")
            (root / "assets/changed.txt").write_text("changed", encoding="utf-8")
            approved_hash = hashlib.sha256(b"approved").hexdigest()
            pending_hash = hashlib.sha256(b"pending").hexdigest()
            entries = [
                LedgerEntry(
                    "ALI-0001", "assets/approved.txt", approved_hash, "approved"
                ),
                LedgerEntry(
                    "ALI-0002", "assets/pending.txt", pending_hash, "needs-review"
                ),
                LedgerEntry(
                    "ALI-0003", "assets/changed.txt", "0" * 64, "approved"
                ),
            ]

            result = audit(
                root,
                entries,
                [
                    "assets/approved.txt",
                    "assets/pending.txt",
                    "assets/changed.txt",
                    "assets/missing.txt",
                ],
            )

            self.assertFalse(result.approved)
            self.assertEqual(result.missing_entries, ("assets/missing.txt",))
            self.assertEqual(
                [entry.entry_id for entry in result.nonapproved_entries],
                ["ALI-0002"],
            )
            self.assertEqual(
                [entry.entry_id for entry, _actual in result.hash_mismatches],
                ["ALI-0003"],
            )


if __name__ == "__main__":
    unittest.main()
