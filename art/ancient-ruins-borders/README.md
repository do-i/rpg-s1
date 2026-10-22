# Ancient ruins perimeter

The approved fourth border concept surrounds the original ruin with weathered
wood, two southern mossy stone wall sections, and northern house remnants,
dead trees, broken fences, and a pool. The map is 72×55 tiles. Its former 64×44
interior moves four tiles east and six tiles south, preserving every original
tile, the owner's gate edits, and existing layer IDs and order.

The palisade uses a due-south orthographic camera pitched down 45 degrees,
without yaw or perspective taper. Gray-brown cracked timber, rusty fastenings,
rope and restrained moss match the abandoned buildings.

The production atlas is
`assets/scenarios/rusted_kingdoms/media/tilesets/ancient_ruins_borders.png`,
with minimal image-grid metadata in `ancient_ruins_borders.tsx`. It is 128×32,
with four 32×32 slots:

| Local ID | Object |
| --- | --- |
| 0 | Northern cap |
| 1 | Repeating body A |
| 2 | Repeating body B |
| 3 | Southern cap |

Each body scanline is exactly 18 opaque pixels wide, at columns 7–24. Cap
silhouettes fit the same width. Eight permitted cap/body and body/body joins
have identical RGBA connection rows. Alpha is binary. Caps occur only at the
ends of each vertical run, avoiding repeated bulges. The east side is mirrored.
[tile-validation.json](tile-validation.json) records the deterministic checks.

Rebuild the atlas and metadata from the repository root:

```sh
python3 art/ancient-ruins-borders/build_tileset.py
```

The script uses ImageMagick to trim and normalize the generated north/body/south
sections, construct two body variants, align connection rows, and reduce the
palette to 96 colors. Temporary exports go under `temp/`.

The two `generated-*.png` files retain the original timber reference and the
unmodified modular revision generated with the built-in OpenAI image generation
tool on 2026-10-22. [prompts.json](prompts.json) records the edit prompt and both
source hashes. The accepted atlas pixels match the reviewed fourth concept.

[layout.json](layout.json) records arrivals, NPC positions, the two 12×5-tile
southern stone sections, and 39 additional boundary prop placements. The
original 41 yard props remain documented in the separate prop manifest.
The building atlas and owner-regenerated building TSX are unchanged. No new
map layers were added. Lower-zone collision conventions remain in use.
