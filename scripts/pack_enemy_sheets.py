#!/usr/bin/env python3
"""Pack approved enemy pose cutouts into production v2 battler sheets.

    python3 -I scripts/pack_enemy_sheets.py ID POSES_DIR --ready READY.jpg [--out DIR] [--tile 256]

POSES_DIR holds ID-NN-<pose>.jpg; scripts/imggen step 2 cuts them out into POSES_DIR/cutouts/ and calls this
READY is the canonical image (jpg is cut out first; a transparent png is used as is).

Writes OUT/ID/ (default OUT = POSES_DIR/export):
  ID-status.png/.tsx   4x4 tiles, index order from docs/authoring.md, tile 15 empty
  ID-attack.png/.tsx   2x1 tiles (thrust, lunge)
  1k/<pose>.png        the full-size cutouts, renamed after the pose
Layout matches shipped sheets: one shared scale (widest pose fits), feet 4px above the
cell bottom, centered on each pose's bounding box, transparent, no labels. Needs ImageMagick.
Does not touch assets/ or the ledger.
"""
import argparse, os, re, shutil, subprocess, sys, glob

STATUS = ["ready","poison","sleep","stun","silence","burn","freeze","knockback",
          "taunt","guard","cover","haste","slow","regen","ko"]
ATTACK = ["attack-thrust","attack-lunge"]
HERE = os.path.dirname(os.path.abspath(__file__))
CUTOUT = os.path.join(HERE, "cutout.py")

def run(*a):
    return subprocess.run(a, check=True, capture_output=True, text=True).stdout

def bbox(p):
    m = re.match(r"(\d+)x(\d+)\+(\d+)\+(\d+)", run("magick", p, "-format", "%@", "info:"))
    w, h, x, y = map(int, m.groups()); return x, y, w, h

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("id"); ap.add_argument("poses_dir")
    ap.add_argument("--ready", required=True)
    ap.add_argument("--out"); ap.add_argument("--tile", type=int, default=256)
    a = ap.parse_args()
    T = a.tile
    out = os.path.join(a.out or os.path.join(a.poses_dir, "export"), a.id)
    k1 = os.path.join(out, "1k"); os.makedirs(k1, exist_ok=True)

    # 1k cutouts renamed after the pose
    src = {}
    for f in glob.glob(os.path.join(a.poses_dir, "cutouts", "*.png")):
        m = re.search(r"-\d\d-(.+)\.png$", f)
        if m: src[m.group(1)] = f
    ready = os.path.join(k1, "ready.png")
    if a.ready.lower().endswith(".png"):
        shutil.copy(a.ready, ready)
    else:
        subprocess.run([sys.executable, "-I", CUTOUT, a.ready, ready], check=True)
    src["ready"] = ready
    miss = [p for p in STATUS + ATTACK if p not in src]
    if miss: sys.exit("missing poses: " + ", ".join(miss))
    for p in STATUS + ATTACK:
        dst = os.path.join(k1, p.replace("-", "_") + ".png")
        if os.path.abspath(src[p]) != os.path.abspath(dst): shutil.copy(src[p], dst)
        src[p] = dst

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
    print("1k poses:", k1)

main()
