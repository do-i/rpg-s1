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
