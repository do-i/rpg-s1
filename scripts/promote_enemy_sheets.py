#!/usr/bin/env python3
"""Promote packed enemy sheets into the shipped v2 folder and add their ledger rows.

    python3 -I scripts/promote_enemy_sheets.py ID SHEETS_DIR [--dry-run] [--force]

SHEETS_DIR is the folder scripts/pack_enemy_sheets.py wrote (ID-status.png etc.; imggen: STAGE/ID/sheets/).
Copies ID-status.{png,tsx} and ID-attack.{png,tsx} into
assets/scenarios/rusted_kingdoms/media/sprites/enemies/v2/ and appends four rows
(next free ALI id, SHA-256) to the end of the `Home-grown assets (MIT)` table in
docs/asset-license-inventory.md. Refuses to overwrite existing files unless --force.
Review the diff, then run scripts/check_asset_rights.py --check yourself. Does not commit.
"""
import argparse, hashlib, re, shutil, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
V2 = ROOT / "assets/scenarios/rusted_kingdoms/media/sprites/enemies/v2"
LEDGER = ROOT / "docs/asset-license-inventory.md"
ORIGIN_PNG = "AI-generated (Grok), author-directed"
ORIGIN_TSX = "author / AI agents"

def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("id"); ap.add_argument("sheets_dir")
    ap.add_argument("--dry-run", action="store_true"); ap.add_argument("--force", action="store_true")
    a = ap.parse_args()
    src = Path(a.sheets_dir)
    names = [f"{a.id}-{k}.{e}" for k in ("status", "attack") for e in ("png", "tsx")]
    missing = [n for n in names if not (src / n).is_file()]
    if missing: sys.exit(f"missing in {src}: {', '.join(missing)}")
    clash = [n for n in names if (V2 / n).exists()]
    if clash and not a.force: sys.exit(f"already in v2 (use --force): {', '.join(clash)}")

    text = LEDGER.read_text()
    lines = text.split("\n")
    start = next(i for i, l in enumerate(lines) if l.startswith("## Home-grown assets (MIT)"))
    end = next(i for i in range(start + 1, len(lines)) if lines[i].startswith("## "))
    last = max(i for i in range(start, end) if lines[i].startswith("| "))
    ids = [int(m.group(1)) for m in re.finditer(r"^\| ALI-(\d+) ", text, re.M)]
    nxt = max(ids) + 1
    rel = V2.relative_to(ROOT).as_posix()
    rows = []
    for n in names:
        key = f"{rel}/{n}"
        if any(f"`{key}`" in l for l in lines):
            if not a.force: sys.exit(f"ledger already has a row for {key}")
            continue
        h = sha(src / n)
        kind, ext = n[len(a.id) + 1:].split(".")
        if ext == "png":
            dim = "1024x1024 transparent PNG, 4x4 of 256px pose tiles" if kind == "status" else "512x256 transparent PNG, 2 attack tiles of 256px"
            desc = f"enemy battler sheet / {a.id} v2 {kind} poses; {dim}"
            origin = ORIGIN_PNG
        else:
            desc = f"Tiled tileset / {a.id} v2 {kind} sheet"
            origin = ORIGIN_TSX
        rows.append(f"| ALI-{nxt} | {origin} | `{key}` | `{h}` | {desc} |"); nxt += 1
    print("copy:", ", ".join(names)); print("\n".join(rows))
    if a.dry_run: print("(dry run, nothing written)"); return
    for n in names: shutil.copy(src / n, V2 / n)
    lines[last + 1:last + 1] = rows
    LEDGER.write_text("\n".join(lines))
    print(f"promoted {a.id}: {len(names)} files, {len(rows)} ledger rows")

main()
