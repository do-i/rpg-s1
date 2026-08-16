# Ardel new-game screenshot oracle

Run from the repository root:

```bash
scripts/check-ardel-screenshot.sh
```

The default rendered artifact is `target/ardel-new-game.png`. Pass another
output path as the first argument when a temporary or CI artifact is desired.
The command requires Tiled's `tmxrasterizer`, ImageMagick 7's `magick`, and
`sha256sum`; Qt is forced to its offscreen platform unless the caller already
selected another platform.

The composition contract is the fixed 1280x766 gameplay canvas with the
30x20, 32-pixel Ardel map centered over the shared `(10, 10, 30)` clear color.
It preserves every authored visible layer in TMX source order, including the
`collision` layer whose tiles also provide Ardel's building and fence artwork,
then overlays TSX tile 18 (Aric's down-idle frame) with its feet-aligned collision
rectangle centered at new-game tile `[14, 5]`. The oracle hashes the
decoded 8-bit RGBA pixels rather than PNG file bytes, avoiding differences in
compression or metadata.

`rgba8.sha256` is an intentional visual-regression boundary. Update it only
after inspecting the generated PNG and confirming that a map, atlas, spawn,
canvas, or layer-order change is expected.
