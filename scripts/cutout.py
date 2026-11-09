#!/usr/bin/env python3
"""Cut one enemy pose out of its flat magenta field, cleanly.

    python3 -I scripts/cutout.py INPUT.jpg OUTPUT.png

Needs only numpy and ImageMagick (`magick`). No PIL, no scipy.

A plain `magick -fuzz 16% -transparent <corner>` key fails on these renders
in three ways, and this script fixes each one:

1. Ring / fringe. The image model's "magenta" is a muddy raspberry around
   (154,46,96). The blend between the figure's dark rim and that field is kept
   as an opaque plum ring. Some renders also paint a maroon "sticker" halo or a
   flat dark die-cut outline (6-10px at 1024) around the whole figure.
   -> background grows through magenta-hued halo pixels; edge pixels are
      un-mixed against the locally sampled field; a flat dark outline ring is
      measured and peeled to its own width.
2. Holes. The key colour sits close to red skin, dark leather, rust and dark
   fur shadow, so a fuzz key punches transparent holes in the body and weapon.
   -> the coarse mask is a flood fill from the border, so only field-connected
      pixels can go transparent; enclosed see-through pixels with a body colour
      are restored; notches on the outer edge are grown back.
3. Real gaps (between legs, arm and torso) must stay see-through.
   -> pure key-colour pixels anywhere, and magenta pockets near the edge, seed
      the background.

Exit status 0 = clean. Exit status 2 = the render itself is unusable (figure
touches the image edge, vignette/scene at the border); regenerate the image
rather than patching it. The report lists what was removed and restored.
"""
import subprocess
import sys

import numpy as np

BAND = 3          # px edge band that gets soft alpha (at ~1024px source)
HALO_T = 14       # min(r,b)-g above this = magenta-hued
REF_RATIO = 0.56   # blue/red of the key colour the hue thresholds below were tuned at
KEY = [None]       # key colour of the image being processed, shared with verify()
OUTLINE_MAX = 14  # max painted-outline width to peel
PINK_MAX = 100    # leftover magenta px allowed (~6px at the 256px tile; real fringe is thousands)
PLUM_MAX = 400     # opaque plum px at the rim allowed (real fringe is thousands; ember and membrane edges give a few hundred)
EDGE_BAND = 6     # px from transparency where verify() looks for fringe / shadow leftovers
POCKET_REACH = 14  # px a pure-field seed spreads through an enclosed pocket before the rest counts as art
HOLES_MAX = 150   # enclosed see-through body-colour px allowed (open ribcages give ~100; real holes give hundreds to thousands)
STRAY_REACH = 8   # px from a gap within which leftover field colour is cleared
HUE_TOL = 0.07    # max |b/r - key b/r| for an enclosed pixel to count as field


# ---------------------------------------------------------------- image io
def load(p):
    w, h = map(int, subprocess.check_output(["magick", "identify", "-format", "%w %h", p]).split())
    raw = subprocess.check_output(["magick", p, "-alpha", "set", "-depth", "8", "RGBA:-"])
    return np.frombuffer(raw, np.uint8).reshape(h, w, 4).astype(np.float64)


def save(a, p):
    h, w = a.shape[:2]
    subprocess.run(["magick", "-size", f"{w}x{h}", "-depth", "8", "RGBA:-", "PNG32:" + p],
                   input=np.clip(a + 0.5, 0, 255).astype(np.uint8).tobytes(), check=True)


# ---------------------------------------------------------- morphology
def shift(m, dy, dx):
    out = np.zeros_like(m)
    h, w = m.shape[:2]
    ys, yd = (slice(0, h - dy), slice(dy, h)) if dy >= 0 else (slice(-dy, h), slice(0, h + dy))
    xs, xd = (slice(0, w - dx), slice(dx, w)) if dx >= 0 else (slice(-dx, w), slice(0, w + dx))
    out[yd, xd] = m[ys, xs]
    return out


def dilate(m, n):
    for _ in range(n):
        acc = m.copy()
        for dy in (-1, 0, 1):
            for dx in (-1, 0, 1):
                acc |= shift(m, dy, dx)
        m = acc
    return m


def outside(mask):
    """Pixels of `mask` 8-connected to the image border (flood fill)."""
    h, w = mask.shape
    pad = np.ones((h + 2, w + 2), bool)
    pad[1:-1, 1:-1] = mask
    out = subprocess.run(["magick", "-size", f"{w + 2}x{h + 2}", "-depth", "8", "gray:-",
                          "-fill", "gray(128)", "-draw", "color 0,0 floodfill", "-depth", "8", "gray:-"],
                         input=(pad * 255).astype(np.uint8).tobytes(), capture_output=True, check=True).stdout
    return (np.frombuffer(out, np.uint8).reshape(h + 2, w + 2) == 128)[1:-1, 1:-1]


def push(vals, known, n):
    """Extend known colours outward n pixels by neighbour averaging."""
    vals = vals * known[..., None]
    known = known.copy()
    for _ in range(n):
        s = np.zeros_like(vals)
        c = np.zeros(known.shape)
        for dy in (-1, 0, 1):
            for dx in (-1, 0, 1):
                s += shift(vals, dy, dx)
                c += shift(known.astype(float), dy, dx)
        new = (~known) & (c > 0)
        vals[new] = s[new] / c[new][:, None]
        known = known | new
    return vals, known


def local_std(x, k=2):
    def box(v):
        pad = np.pad(v, k, mode="edge")
        c = np.pad(np.cumsum(np.cumsum(pad, 0), 1), ((1, 0), (1, 0)))
        n = 2 * k + 1
        return (c[n:, n:] - c[:-n, n:] - c[n:, :-n] + c[:-n, :-n]) / (n * n)
    m = box(x)
    return np.sqrt(np.maximum(box(x * x) - m * m, 0))


def spill(c):
    """Magenta excess: how far red+blue sit above green."""
    return (c[..., 0] + c[..., 2]) / 2 - c[..., 1]


# ------------------------------------------------------------- the cut
def peel_outline(src, bg, report):
    """Strip a painted die-cut outline: a flat dark ring hugging the figure."""
    lum = src.mean(-1)
    sd = local_std(lum)
    out = outside(bg)
    ring = dilate(out, 4) & ~dilate(out, 1) & ~bg
    if not ring.any():
        return bg
    flat = ring & (sd < 8)
    if flat.sum() < 0.5 * ring.sum():
        report.append("outline: none")
        return bg
    ocol = np.median(src[flat], 0)
    if ocol.mean() > 85:
        report.append(f"outline: none (edge ring is light {ocol.astype(int).tolist()})")
        return bg
    dist = np.linalg.norm(src - ocol, axis=-1)
    prev, width = out, 0
    for d in range(1, OUTLINE_MAX + 1):
        ring_d = dilate(prev, 1) & ~prev & ~bg
        if not ring_d.any() or (dist[ring_d] < 30).mean() < 0.6:
            if d > 2:
                break
        else:
            width = d
        prev = prev | ring_d
    reach, peeled = out.copy(), 0
    for d in range(1, width + 2):
        ok = (dist < 35) if d <= 2 else ((dist < 30) & (sd < 10))
        nb = dilate(reach, 1) & ~reach
        nb_fg = nb & ~bg & ok
        if not nb_fg.any():
            break
        peeled += int(nb_fg.sum())
        bg = bg | nb_fg
        reach = reach | nb_fg | (nb & bg)
    report.append(f"outline: painted ring {ocol.astype(int).tolist()} width {width}px, peeled {peeled}px")
    return bg


def hue_scale(key):
    """Scale for the blue/red thresholds: the model's magenta varies from run to run (b/r 0.48 to 0.58).
    Only ever relaxes them (redder keys); keys at or above the reference keep the original thresholds."""
    return min(1.0, (key[2] / max(key[0], 1.0)) / REF_RATIO)


def hue_cap(key):
    """Upper blue/red ratio of anything that can be field, halo or fringe. Violet and indigo art
    (night-fiend wings, mages) has b/r well above 1; fringe is magenta-to-maroon, b/r below the key's + margin."""
    return key[2] / max(key[0], 1.0) + 0.15


def shadowish(r, g, b, key):
    """Dark, key-hued pixel: a shaded patch of the magenta field."""
    ratio = b / np.maximum(r, 1.0)
    f = hue_scale(key)
    return ((np.minimum(r, b) - g > 14) & (ratio > 0.38 * f) & (ratio < max(0.80 * f, key[2] / max(key[0], 1.0) + 0.15))
            & (g < 0.35 * b) & (r < 0.85 * key[0]))


def cutout(src, report):
    r, g, b = src[..., 0], src[..., 1], src[..., 2]
    mag = (np.minimum(r, b) - g > 30) & (b >= 0.4 * r)
    frame = np.zeros(mag.shape, bool)
    frame[:32, :] = frame[-32:, :] = frame[:, :32] = frame[:, -32:] = True
    pool = src[mag & frame] if (mag & frame).sum() > 500 else src[mag]
    if len(pool) < 500:
        raise SystemExit("no magenta field found; the render must sit on flat magenta")
    key = np.median(pool, 0)
    KEY[0] = key
    report.append(f"key colour: {key.astype(int).tolist()}")

    dkey = np.linalg.norm(src - key, axis=-1)
    cap = hue_cap(key)
    halo = (np.minimum(r, b) - g > HALO_T) & (b >= 0.4 * r) & (b < cap * r)
    f = hue_scale(key)
    plum = (b - g > 18) & (r - g > 30) & (b >= 0.55 * f * r) & (b < cap * r)
    plum_loose = (b - g > 12) & (r - g > 25) & (b >= 0.5 * f * r) & (b < cap * r)
    shadow = shadowish(r, g, b, key)
    body = ~plum_loose & ~shadow & (src.sum(-1) > 30)
    # Hue guard: enclosed pixels only count as field if their blue/red ratio matches the key's.
    # Saturated red/orange art (flames, embers, lava) sits within colour distance of the key
    # but has a lower blue/red ratio (~0.46 vs ~0.56), so it must never be cleared as a "gap".
    kratio = key[2] / max(key[0], 1.0)
    keyish = np.abs(b / np.maximum(r, 1.0) - kratio) < HUE_TOL

    # 1. coarse field: flood from the border through key-like pixels only
    bg = outside((dkey < 45) | halo)
    # 2. enclosed gaps: pure key anywhere, magenta pockets near the edge
    pocket = halo & ((np.minimum(r, b) - g > 40) | (dkey < 40) | plum) & dilate(bg, 80) & keyish
    bg |= pocket | ((dkey < 25) & keyish)
    # 3. grow through the maroon sticker halo
    for _ in range(24):
        nb = dilate(bg, 1) & halo & ~bg
        if not nb.any():
            break
        bg |= nb
    # 3b. contact shadows: darker versions of the field colour (same hue, much lower value)
    # that the model paints under feet, weapons and ears. Grow the field through them until it
    # stops; flames and other bright red art are excluded by the value test, brown and green
    # art by the hue test.
    for _ in range(200):
        nb = dilate(bg, 1) & shadow & ~bg
        if not nb.any():
            break
        bg |= nb
    # 4. restore body pixels: enclosed holes, then edge notches
    enclosed = bg & ~outside(bg)
    holes = enclosed & body & ((dkey >= 45) | ~keyish)
    # An enclosed pocket is a real gap only if pure field colour sits inside it (or within a halo's
    # width of it). Mauve membrane or ember pixels that were cleared by hue alone have none: restore them.
    reach = enclosed & (dkey < 25)
    for _ in range(POCKET_REACH):
        reach |= dilate(reach, 1) & enclosed
    orphan = enclosed & ~reach & (dkey >= 45) & (src.sum(-1) > 30)
    holes |= orphan
    bg &= ~holes
    notches = 0
    for _ in range(12):
        nb = dilate(~bg, 1) & bg & body & (dkey >= 50)
        if not nb.any():
            break
        notches += int(nb.sum())
        bg &= ~nb
    report.append(f"restored: {int(holes.sum())} hole px, {notches} edge-notch px")
    # 5. painted dark outline
    bg = peel_outline(src, bg, report)

    # 6. soft edge: un-mix a thin band against the local field colour
    fg = ~bg
    near_bg = dilate(bg, BAND)
    interior = fg & ~near_bg
    band = (fg & near_bg) | (bg & dilate(fg, 1))
    F, _ = push(src, interior, BAND + 2)
    B, bk = push(src, bg & ~dilate(fg, BAND), BAND + 3)
    B[~bk] = key
    sC, sF, sB = spill(src), spill(F), spill(B)
    den = sF - sB
    ok = np.abs(den) > 25
    d = F - B
    dd = (d * d).sum(-1)
    proj_ok = ~ok & (dd > 40 * 40)
    proj = ((src - B) * d).sum(-1) / np.where(proj_ok, dd, 1)
    a = np.where(ok, (sC - sB) / np.where(ok, den, 1), np.where(proj_ok, proj, fg.astype(float)))
    a = np.clip(a, 0, 1)
    alpha = np.where(interior, 1.0, np.where(band, a, 0.0))
    U = np.clip((src - (1 - alpha[..., None]) * B) / np.maximum(alpha, 1e-3)[..., None], 0, 255)
    w = np.clip((alpha - 0.15) / 0.45, 0, 1)[..., None]
    col = np.where(band[..., None], w * U + (1 - w) * F, src)

    # 7. final guard: no enclosed see-through pixel may carry a body colour
    clear = alpha < 0.5
    fix = clear & ~outside(clear) & body & ((dkey >= 45) | ~keyish)
    alpha = np.where(fix, 1.0, alpha)
    col = np.where(fix[..., None], src, col)
    # 8. stray field: opaque pixels that verify() would call visible magenta, within STRAY_REACH of any
    # transparency (an inner gap or the outer silhouette), are field seen through bone, rib or strap
    # gaps that the hue guard above kept as art. Magenta-ish art further inside is left alone.
    ro, go, bo = col[..., 0], col[..., 1], col[..., 2]
    near_gap = dilate(alpha < 0.5, STRAY_REACH)
    pink_out = (ro - go > 60) & (bo - go > 60) & (np.abs(ro - bo) < 90) & (bo < cap * ro)
    stray = (alpha > 0) & near_gap & pink_out
    report.append(f"stray field: {int(stray.sum())} px cleared")
    alpha = np.where(stray, 0.0, alpha)
    col = np.where((alpha > 0)[..., None], col, 0)
    return np.dstack([col, alpha * 255])


# ----------------------------------------------------------- verify
def verify(src, out, report):
    """Return a list of failures. Empty list = clean."""
    fails = []
    al = out[..., 3]
    r, g, b = out[..., 0], out[..., 1], out[..., 2]
    vis = al > 0
    border = np.zeros(al.shape, bool)
    border[:2, :] = border[-2:, :] = border[:, :2] = border[:, -2:] = True
    edge_px = int((border & (al > 128)).sum())
    if edge_px:
        fails.append(f"REGENERATE: {edge_px} opaque px on the image border "
                     "(figure clipped, vignette, or scenery; the field must be flat to every edge)")
    cap = hue_cap(KEY[0])
    # Fringe and halo only exist at the silhouette: mauve tongues, violet membranes and dark-red
    # embers inside the figure are art, so only count pixels within EDGE_BAND of transparency.
    rim = dilate(al < 128, EDGE_BAND)
    pink = int((vis & (r - g > 60) & (b - g > 60) & (abs(r - b) < 90) & (b < cap * r)).sum())
    plum = int((rim & (al > 128) & (b - g > 18) & (r - g > 30) & (b >= 0.55 * hue_scale(KEY[0]) * r) & (b < cap * r) & (r < 140)).sum())
    report.append(f"check: {pink} bright-magenta px, {plum} opaque plum px")
    if pink > PINK_MAX:
        fails.append(f"{pink} visible magenta pixels remain")
    if plum > PLUM_MAX:
        fails.append(f"{plum} opaque plum px: a magenta fringe or sticker halo is still on the figure")
    shade = int((rim & (al > 128) & shadowish(r, g, b, KEY[0])).sum()) if KEY else 0
    report.append(f"check: {shade} opaque shadow-hued px")
    if shade > 200:
        fails.append(f"{shade} opaque shadow-hued px: a contact shadow of the field is still on the figure")
    clear = al < 128
    sr, sg, sb = src[..., 0], src[..., 1], src[..., 2]
    body = (~((sb - sg > 12) & (sr - sg > 25) & (sb >= 0.5 * hue_scale(KEY[0]) * sr) & (sb < cap * sr)) & (src.sum(-1) > 30)
            & ~shadowish(sr, sg, sb, KEY[0]))
    holes = int((clear & ~outside(clear) & body).sum())
    report.append(f"check: {holes} enclosed body-colour holes")
    if holes > HOLES_MAX:
        fails.append(f"{holes} see-through pixels inside the body")
    # dark flat ring still hugging the silhouette?
    lum = out[..., :3].mean(-1)
    sd = local_std(lum)
    o = outside(clear)
    ring = dilate(o, 4) & ~dilate(o, 1) & ~clear
    if ring.any():
        flat_dark = (ring & (sd < 6) & (lum < 60)).sum() / ring.sum()
        report.append(f"check: edge ring {flat_dark:.0%} flat-dark")
        if flat_dark > 0.35:
            fails.append(f"dark outline ring still present ({flat_dark:.0%} of the edge)")
    return fails


def main():
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    try:
        src = load(sys.argv[1])[..., :3]
    except (subprocess.CalledProcessError, ValueError):
        print("FAIL: REGENERATE: image is unreadable or truncated")
        sys.exit(2)
    report = []
    out = cutout(src, report)
    save(out, sys.argv[2])
    fails = verify(src, out, report)
    for line in report:
        print(line)
    if fails:
        for f in fails:
            print("FAIL:", f)
        sys.exit(2)
    print("OK")


if __name__ == "__main__":
    main()
