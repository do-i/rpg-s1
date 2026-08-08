# Fixed gameplay canvas policy

The application renders gameplay through a fixed **1280x766 logical canvas**.
World positions and gameplay UI layout use those logical coordinates regardless
of the window size or display pixel density.

For each nonzero physical window size, the shell chooses the largest centered
viewport that fits while preserving the canvas aspect ratio. It uses one
uniform target scale and never crops the logical canvas. The non-limiting
physical edge is rounded down to a whole pixel so the viewport cannot exceed
the window. This can introduce less than one physical pixel of unavoidable
aspect quantization; it is not an intentional non-uniform resize. If centering
leaves an odd pixel, the extra pixel is placed on the right or bottom. The
existing dark clear color fills unused space, producing letterbox bars above
and below or pillarbox bars on the sides.

The calculation uses the window's physical pixel dimensions. Bevy's UI scale is
adjusted for the window scale factor, so a HiDPI display does not change the
logical layout. Resizing recomputes the viewport and uniform UI scale. A zero
width or height, as can occur while minimized, deactivates canvas cameras and
does not construct an invalid zero-sized viewport; rendering resumes when a
nonzero size returns.

Future pointer input must respect the same boundary. Convert the pointer to
physical pixels, reject positions in the bars, subtract the viewport's physical
origin, and divide by the uniform physical scale to obtain canvas coordinates.
Camera world/viewport conversion helpers should be preferred where they apply.
The final row or column can contain a subpixel rounding remainder, so converted
coordinates should be clamped to the logical canvas bounds.

## M1.14 title rendering verification

M1.14 passed on 2026-08-08 at commit
`a88e9ff299e6a1cf998f29b8c70a6ecff4f3698f`. The preflight `cargo fmt
--check`, `cargo test` (55 tests), `cargo clippy --all-targets -- -D warnings`,
and `cargo build` commands all exited successfully. A real `cargo run` on X11
used Vulkan llvmpipe (`device_type: Cpu`, Mesa 26.1.6), which is the accepted
renderer on this no-GPU development machine. Live terminal output showed only
the known XSETTINGS reload and software-rendering performance warnings before
the capture run was interrupted; no panic or fatal asset, audio, or rendering
error was observed.

| Client size | Expected viewport | Observed result | Evidence |
| --- | --- | --- | --- |
| 1280x766 | 1280x766 at (0, 0), no bars | PASS: the image fills the client; title artwork and the complete bottom-centered menu are correctly framed and readable. | `/tmp/rpg-s1-m1-14-baseline-1280x766.png` (SHA-256 `697d16b482002755e6dce96a27084f318299650c5e8c95d89f0374eefb9ddcc7`) |
| 900x600 | 900x538 at (0, 31) | PASS: rows 0-30 and 569-599 are each a single color, `srgb(10,10,30)`; rendered content occupies rows 31-568 with unchanged composition and readable UI. | `/tmp/rpg-s1-m1-14-small-900x600.png` (SHA-256 `854833fbe56ab22db0ee270d624bc0c08d168d00ddf49e9830a31894f406a0ae`) |
| 1600x900 | 1503x900 at (48, 0), with the odd pixel on the right | PASS: the 48-pixel left bar and 49-pixel right bar are each uniformly `srgb(10,10,30)`; content remains centered, correctly framed, and readable. | `/tmp/rpg-s1-m1-14-wide-1600x900.png` (SHA-256 `71b104765a3856dc718a11d059bb0aa10d6c77cc3752e135146d70740a53b9cc`) |

The smaller and wider viewports were cropped and normalized back to 1280x766
for structural comparison with the baseline. ImageMagick RMSE was
`904.256 (0.0137981)` for the smaller capture and `519.131 (0.00792144)` for
the wider capture; visual inspection confirmed that the differences are
resampling, not cropping, layout drift, or aspect loss.

This evidence set verifies rendering and resize responsiveness only. The
capture process was interrupted during image analysis before its planned
navigation and graceful-Quit phase, so it supplies no new audio or clean-exit
claim. Graceful Quit remains covered separately by M0.17c and the successful
post-M1.07 llvmpipe/X11 interaction smoke; those results are not inferred from
these images.
