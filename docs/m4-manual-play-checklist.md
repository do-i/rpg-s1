# M4 manual play evidence

Date: 2026-08-10  
Runtime revision: `317db1f` plus the committed M4 runtime assets and screenshot oracle  
Pinned source revision: `08970359d6cb03586948625d29b0d3351dbbf785`

This check exercises the real Bevy window and production scenario package. It
complements the headless production-package test and the deterministic screenshot
oracle; it is not a claim that later portal, NPC, save, or combat milestones work.

## Environment

- Linux/X11 with Openbox
- Bevy 0.19 Vulkan backend
- Mesa llvmpipe software renderer
- 1280x766 game client area
- `cargo run` from the repository root

## Results

| Check | Result | Evidence |
| --- | --- | --- |
| Title boot | PASS | The production window opened as `Chronicles of the Lost Flame` with title art, menu, and New Game selected. Capture: `/tmp/rpg-s1-m4-title.png`, SHA-256 `697d16b482002755e6dce96a27084f318299650c5e8c95d89f0374eefb9ddcc7`. |
| New Game to name entry | PASS | A real Return key event replaced the title with the production `Enter your name` prompt and default `Aric` value. Capture: `/tmp/rpg-s1-m4-name.png`, SHA-256 `a2c240a24766837382c7bd306d30e887284eac330d55d04cf2cde5dc28e13485`. |
| Name confirmation to intro | PASS | A fresh Return confirmed `Aric` and rendered line 1/13 from the production intro. Capture: `/tmp/rpg-s1-m4-after-name.png`, SHA-256 `630dad250dc65314242b847276f601e1d2567c204bc3278a18f7c33a65cf6ef6`. |
| Supported intro Back path to Ardel | PASS | Escape used the tested Back behavior and entered Ardel at manifest tile `[14, 5]`. The complete map, real Aric sprite, centered small-map camera, and clear-color bars rendered with no asset errors. Capture: `/tmp/rpg-s1-m4-world-spawn.png`, SHA-256 `96d4bc37c6c7dfe4c740e6e113f5c90cb7335e0b89d5bd11f5f0f0d08a8ddfde`. |
| Cardinal walk | PASS | One Right action moved Aric from `[14, 5]` to `[15, 5]`, exactly 32 pixels, and settled to right-facing idle. Capture: `/tmp/rpg-s1-m4-world-right.png`, SHA-256 `b806ed6d7251e2c4b954807b5ecdc3d86508fa37874a9b3daa834b5034371a25`. |
| Diagonal walk | PASS | One simultaneous Right+Down action moved Aric from `[15, 5]` to `[16, 6]`, exactly one tile on each open axis, and settled to the documented vertical-priority down idle. Capture: `/tmp/rpg-s1-m4-world-diagonal.png`, SHA-256 `6a9bac46293d9c283ab87d71fc543f9c7d16669971d9e8df19fa11a9a109be73`. |
| Runtime diagnostics | PASS | Startup selected llvmpipe/Vulkan and emitted no missing-asset, loader, audio, or panic diagnostics through both movement checks. The process then exited cleanly via Ctrl-C. |

## Repeatable automated companion

Run:

```sh
scripts/check-ardel-screenshot.sh
```

It must produce a 1280x766 composition whose decoded RGBA8 bytes hash to
`122b47cda515c384cb40a531fb3e86666d28a71676e944639525c8fd4924934c`.
The script uses the copied TMX/TSX/PNG assets and real Aric atlas, excludes the
collision layer, and applies the M4 canvas and Y-order contract.

