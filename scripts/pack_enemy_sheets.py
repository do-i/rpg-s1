#!/usr/bin/env python3
"""Pack approved enemy pose cutouts into production v2 battler sheets.

    python3 -I scripts/pack_enemy_sheets.py ID CUTOUTS_DIR OUT_DIR [--tile 256]

CUTOUTS_DIR holds the accepted 1k cutouts: ready.png and ID-NN-<pose>.png (scripts/imggen step 2
keeps them in STAGE/ID/ID/). They are read in place, never copied.

Writes into OUT_DIR:
  ID-status.png/.tsx   4x4 tiles, index order from docs/authoring.md, tile 15 empty
  ID-attack.png/.tsx   2x1 tiles (thrust, lunge)
Layout matches shipped sheets: one shared scale (widest pose fits), feet 4px above the
cell bottom, centered on each pose's bounding box, transparent, no labels. Needs ImageMagick.
Does not touch assets/ or the ledger.
"""
import argparse, os, re, subprocess, sys, glob

STATUS = ["ready","poison","sleep","stun","silence","burn","freeze","knockback",
          "taunt","guard","cover","haste","slow","regen","ko"]
ATTACK = ["attack-thrust","attack-lunge"]

def run(*a):
    return subprocess.run(a, check=True, capture_output=True, text=True).stdout

def bbox(p):
    m = re.match(r"(\d+)x(\d+)\+(\d+)\+(\d+)", run("magick", p, "-format", "%@", "info:"))
    w, h, x, y = map(int, m.groups()); return x, y, w, h

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("id"); ap.add_argument("cutouts_dir"); ap.add_argument("out_dir")
    ap.add_argument("--tile", type=int, default=256)
    a = ap.parse_args()
    T = a.tile
    out = a.out_dir; os.makedirs(out, exist_ok=True)

    src = {}
    for f in glob.glob(os.path.join(a.cutouts_dir, f"{glob.escape(a.id)}-[0-9][0-9]-*.png")):
        m = re.search(r"-\d\d-(.+)\.png$", f)
        if m: src[m.group(1)] = f
    ready = os.path.join(a.cutouts_dir, "ready.png")
    if os.path.isfile(ready): src["ready"] = ready
    miss = [p for p in STATUS + ATTACK if p not in src]
    if miss: sys.exit("missing poses: " + ", ".join(miss))

    # one shared scale across every pose; widest (or tallest) pose just fits
    bb = {p: bbox(src[p]) for p in STATUS + ATTACK}
    scale = min((T - 10) / max(b[2] for b in bb.values()),
                (T - 8) / max(b[3] for b in bb.values()))
    print("shared scale %.4f" % scale)

    def tile_args(p, col, row):
        x, y, w, h = bb[p]
        tw, th = max(1, round(w * scale)), max(1, round(h * scale))
        px = col * T + (T - tw) // 2
        py = row * T + (T - 4 - th)
        return ["(", src[p], "-crop", f"{w}x{h}+{x}+{y}", "+repage",
                "-filter", "Lanczos", "-resize", f"{tw}x{th}!", ")",
                "-geometry", f"+{px}+{py}", "-composite"]

    def sheet(name, poses, cols, rows):
        png = os.path.join(out, f"{a.id}-{name}.png")
        cmd = ["magick", "-size", f"{cols*T}x{rows*T}", "xc:none", "-gravity", "NorthWest"]
        for i, p in enumerate(poses): cmd += tile_args(p, i % cols, i // cols)
        subprocess.run(cmd + ["-depth", "8", png], check=True)
        tsx = os.path.join(out, f"{a.id}-{name}.tsx")
        open(tsx, "w").write(
            '<?xml version="1.0" encoding="UTF-8"?>\n'
            f'<tileset version="1.10" tiledversion="1.12.1" name="{a.id}-{name}" tilewidth="{T}" tileheight="{T}" tilecount="{cols*rows}" columns="{cols}">\n'
            f' <image source="{a.id}-{name}.png" width="{cols*T}" height="{rows*T}"/>\n</tileset>\n')
        print("wrote", png)

    sheet("status", STATUS, 4, 4)
    sheet("attack", ATTACK, 2, 1)

main()
