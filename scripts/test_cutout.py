"""Synthetic-image tests for scripts/cutout.py (needs numpy and ImageMagick; skipped without them)."""
import os
import shutil
import subprocess
import sys
import tempfile
import unittest

try:
    import numpy as np
    from scripts import cutout
except ImportError:  # numpy missing
    np = None

FIELD = "rgb(227,22,136)"
HAVE = np is not None and shutil.which("magick") is not None
SCRIPT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "cutout.py")


def render(path, *draws, size=1024):
    subprocess.run(["magick", "-size", f"{size}x{size}", f"xc:{FIELD}", *draws, "-quality", "95", path], check=True)


def run(src, dst):
    return subprocess.run([sys.executable, "-I", SCRIPT, src, dst], capture_output=True, text=True)


@unittest.skipUnless(HAVE, "numpy and ImageMagick required")
class CutoutTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.tmp, True)

    def alpha(self, png):
        return cutout.load(png)[..., 3]

    def test_violet_body_survives_and_real_gap_is_cleared(self):
        src, dst = f"{self.tmp}/a.jpg", f"{self.tmp}/a.png"
        render(src,
               "-fill", "rgb(60,50,140)", "-draw", "ellipse 512,512 300,380 0,360",   # indigo body
               "-fill", "rgb(120,40,170)", "-draw", "ellipse 400,400 90,120 0,360",    # violet membrane patch
               "-fill", FIELD, "-draw", "ellipse 620,620 40,40 0,360")                  # real gap through the body
        res = run(src, dst)
        self.assertEqual(res.returncode, 0, res.stdout)
        a = self.alpha(dst)
        self.assertEqual(a[512, 512], 255)    # body
        self.assertEqual(a[400, 400], 255)    # violet patch is art, not field
        self.assertEqual(a[620, 620], 0)      # the gap stays see-through
        self.assertEqual(a[20, 20], 0)        # field

    def test_flame_tinted_field_gap_is_not_a_hole(self):
        # field seen through a gap and tinted redder by flame glow (b/r a hair under the key's) stays
        # see-through and must not trip the enclosed-hole check, however large the gap is
        src, dst = f"{self.tmp}/g.jpg", f"{self.tmp}/g.png"
        subprocess.run(["magick", "-size", "1024x1024", "xc:rgb(244,13,120)",
                        "-fill", "rgb(60,120,60)", "-draw", "ellipse 512,512 300,380 0,360",
                        "-fill", "rgb(238,25,103)", "-draw", "rectangle 480,440 560,560",
                        "-quality", "95", src], check=True)
        r = run(src, dst)
        self.assertEqual(r.returncode, 0, r.stdout)
        self.assertEqual(self.alpha(dst)[500, 520], 0)

    def _shade_fails(self, draw):
        key = np.array([244.0, 0.0, 119.0])
        cutout.KEY[0] = key
        n = 1024
        out = np.zeros((n, n, 4))
        out[300:700, 300:700] = [60, 120, 60, 255]
        draw(out)
        src = np.where(out[..., 3:4] > 0, out[..., :3], key)
        return [f for f in cutout.verify(src, out, []) if "shadow-hued" in f]

    def test_crimson_cloth_speckle_on_the_rim_is_not_a_shadow(self):
        # dark red hems share the shadow hue but only speckle the edge: no failure
        def speckle(out):
            for i in range(300, 700, 2):
                for x, y in ((i, 300), (i, 699), (300, i), (699, i)):
                    out[y, x] = [91, 5, 36, 255]
        self.assertEqual(self._shade_fails(speckle), [])

    def test_solid_shadow_band_on_the_rim_still_fails(self):
        def band(out):
            out[690:700, 300:700] = [91, 5, 36, 255]
        self.assertEqual(len(self._shade_fails(band)), 1)

    def test_enclosed_violet_pocket_is_restored(self):
        """A mauve pocket with no pure field colour in it is art, even though its hue matches the key's.

        b/r 0.64 sits inside the key's hue band (key 0.60, cap 0.75) and the pocket is within 80px of
        the silhouette, so the hue rules clear it as a magenta pocket; only the pure-field-seed test
        (POCKET_REACH) puts it back."""
        src, dst = f"{self.tmp}/b.jpg", f"{self.tmp}/b.png"
        render(src,
               "-fill", "rgb(50,45,120)", "-draw", "ellipse 512,512 300,380 0,360",
               "-fill", "rgb(140,45,90)", "-draw", "ellipse 262,512 22,22 0,360")
        res = run(src, dst)
        self.assertEqual(res.returncode, 0, res.stdout)
        self.assertEqual(self.alpha(dst)[512, 262], 255)

    def test_figure_touching_the_edge_asks_for_regeneration(self):
        src, dst = f"{self.tmp}/c.jpg", f"{self.tmp}/c.png"
        render(src, "-fill", "rgb(60,50,140)", "-draw", "rectangle 0,300 400,700")
        res = run(src, dst)
        self.assertEqual(res.returncode, 2)
        self.assertIn("REGENERATE", res.stdout)

    def test_truncated_jpeg_fails_cleanly(self):
        src, dst = f"{self.tmp}/d.jpg", f"{self.tmp}/d.png"
        with open(src, "wb") as fh:
            fh.write(b"\xff\xd8\xff\xe0 not a real jpeg")
        res = run(src, dst)
        self.assertEqual(res.returncode, 2)
        self.assertIn("REGENERATE", res.stdout)
        self.assertNotIn("Traceback", res.stderr)


if __name__ == "__main__":
    unittest.main()
