---
name: enemy-battler-art
description: >
  Paint one review sample of a Chronicles of the Lost Flame battle enemy:
  a new facing-the-viewer character in the item-icon paint style. The first
  pass is one ready pose for approval, assembled into a labeled sheet under temp/.
  Shipped battlers stay untouched. Use when the user runs /enemy-battler-art,
  or asks to generate, paint, or sample the next enemy sprite, battler, or
  enemy character art.
---

# Enemy battler art

Produce one review sheet for one catalog enemy. The accepted quality bar is the shipped Minotaur sheet, `assets/scenarios/rusted_kingdoms/media/sprites/enemies/v2/minotaur_brute_base-status.png`. Match its paint, not its bull.

Read the `imagine` skill and `game-assets` `assets.md` before the first image call. This skill uses separate still poses. Do not harvest a video cycle unless the user asks for one.

## Leave shipped art alone

Do not edit, replace, or delete anything under `assets/scenarios/rusted_kingdoms/media/sprites/enemies/`. Do not edit `docs/asset-license-inventory.md`. Do not commit. The battle UI already draws `{sprite_id}_battle.tsx`. This sample is not an engine drop-in.

## Output

Create `temp/YYYYMMDD-HHMM-<id>/` with the local date and time (`date +%Y%m%d-%H%M`). `temp/` is gitignored. Write only there:

- `<id>-sample-sheet.png` — one labeled ready figure
- `motion-ready.png` — the transparent cutout
- `REVIEW.md`

This pass is the ready pose only. Do not paint a strike, a hurt pose, or status and attack sheets until the user approves this ready pose.

## Sizes

- **Cutout: 1024×1024.** Generate every pose at 1:1 and check it with `magick identify`. If a render is not 1024×1024, resize the source image to `1024x1024!` before cutting. `cutout.py`'s thresholds assume that size. Keep `motion-ready.png` at 1024×1024: no crop, trim, or resize.
- **Game sheets: 256×256 tiles.** Shipped v2 battlers live in `assets/scenarios/rusted_kingdoms/media/sprites/enemies/v2/`. `<sprite_id>-status.png` is 4×4 tiles (1024×1024), and `<sprite_id>-attack.png` is 2×1 tiles (512×256). The pose order is in `docs/enemy-battler-tiles.md`. Each pose is downscaled from its 1024 cutout into one 256 tile. This skill does not build those sheets; it only produces the 1024 cutout they are made from.
- **Review sheet: 640×780.** See *Cut out and assemble* below.

## Resolve the enemy

Search `assets/scenarios/rusted_kingdoms/data/enemies/`. A name or id the user gives wins. If they say "next" and name nobody, pick one enemy that has no `temp/*-<id>/` sample yet, say the id and display name, and continue. Do not ask which enemy first.

Read that entry's `id`, `name`, `type`, `rank`, `size`, `sprite_scale`, and moves. Find the encounter file under `data/encount/` that lists the id, and use that zone for materials. Design a new creature from the name and the zone. Do not copy the LPC battler, its anatomy, its palette, or its pixel style.

## Paint

Seed the canonical image with item icons, as image references:

- Always `assets/scenarios/rusted_kingdoms/media/images/icons/items/iron_sword.png` (dark contour, brushed metal).
- Always `.../icons/items/potion.png` (saturated color, soft highlight).
- One more item icon whose material matches this enemy and its zone. Use `flamebrand.png` for fire or volcanic enemies. Do not use it for a cold or marsh enemy.

Do not use `icons/spells`, `icons/menu`, `icons/status`, `icons/stats`, `icons/categories`, or `icons/slots`. Do not pass the Minotaur sheet, or any existing enemy sprite, as a generation reference. That copies the bull.

The accepted paint is a hand-painted full-body figure: a dark rim, saturated material color, brushed highlights on metal and hide, facing the viewer, on a flat solid magenta field. No floor, cast shadow, or scenery. The rim on the Minotaur is softer than the icon ink. That is accepted. Do not turn the figure back into pixel art to thicken the line.

The dark rim is the figure's own 1-3px ink line. It is not a border around the figure. Image models like to add three things that ship as a ring around the battler. Forbid all three in every prompt:

- a die-cut or sticker outline: a flat dark or maroon band 5-10px wide around the whole silhouette
- a drop shadow, glow, or aura that bleeds into the field
- a vignette or darkened frame at the image edges

The field must be one flat magenta from edge to edge, touching the figure directly, with clear field between the figure and every image edge.

## Generate

1. Canonical ready pose with `image_edit`, aspect `1:1`, seeded with the three item icons. Full body in frame, space around extremities, both feet planted, facing the viewer, flat solid magenta field. Prompts stay a few sentences. State the creature. Do not ask for a grid or labels. End every prompt with this sentence: "Flat pure magenta background to every edge; no outline, border, or sticker edge around the figure; no shadow, glow, or vignette."
2. Read the image back. Redo once if it is the wrong creature, pixel art, a baked scene, clipped, or still a bull because the Minotaur leaked in. Also redo once if a dark or maroon band runs around the whole silhouette, or the corners darken. Keep the better of the two and say what is wrong.
3. Stop. Show the ready pose and wait. Later poses, when the user asks for them, are `image_edit`s from this ready image only. Repeat the background sentence from step 1. Edits drift toward adding an outline, and a KO or lying pose tends to add a vignette.

## Cut out and assemble

Cut each pose out with `cutout.py` from this skill folder. Run it on the full-size generated image, before any resize:

```bash
python3 -I .grok/skills/enemy-battler-art/cutout.py input.jpg out.png
```

Do not key with `magick -fuzz N% -transparent`. The model's "magenta" is a muddy raspberry, about (154,46,96), close to red skin, dark leather, rust, and fur shadow. A fuzz key leaves a plum ring around the figure and punches see-through holes in the body and weapon. Raising or lowering the fuzz trades one defect for the other. `cutout.py` does what a fuzz key cannot:

- **Field mask.** It flood-fills the field from the image border, so only field-connected pixels can go clear.
- **Real gaps.** Pockets between legs and arms clear only where they hold actual key colour.
- **Halo and fringe.** It grows the field through a maroon sticker halo, and un-mixes the edge pixels against the local field colour, so no plum fringe is left.
- **Holes.** It restores body-coloured pixels that ended up clear, both enclosed holes and nicks on the outer edge.
- **Painted outline.** It measures the width of a flat dark ring and peels exactly that much. Textured boots and leather stop the peel.
- **Report.** It prints what it removed and restored.

Exit 0 means clean. Exit 2 prints `FAIL:` lines:

- `REGENERATE: … opaque px on the image border`: the render has a vignette, scenery, or a clipped figure. Regenerate that pose with the background sentence. Do not crop or patch it.
- `opaque plum px`, `see-through pixels inside the body`, or `dark outline ring still present`: regenerate once. If it fails again, show the cutout and the report to the user instead of hand-painting.

Never "clean" a cutout afterwards. Do not fill holes with flood fills, posterize, median, or morphology passes. That is what turned the orc's red face into flat blotches. Fix the source image or the cut, not the result.

Composite the cutout on `#2a2428` and read it. Then composite it on bright green (`#00ff40`) and read that too. A hole or nick shows green, and a ring shows as a dark or pink band on green. Then build the sheet in ImageMagick. Set `-gravity NorthWest` before every `-composite` and before the labels. Gravity left at North stacks the figure and walks it off the canvas.

The sheet is a flat `#2a2428` field, 640×780. Title is the display name. The subtitle line is `rusted_kingdoms · <id> · ready pose, for approval`. Place the 500px figure at `+70+108`. Label it, in DejaVu Sans, Motion / Ready. Do not ask an image model to draw the frame or the words.

Scale the cutout with `-resize` on the RGBA PNG, never on a flattened image, so the soft edge alpha survives the downscale.

Read the finished sheet. The figure is the ready pose, the label matches, and the field is flat. The figure has no pink, plum, or dark band around it, and no see-through spot.

## Review note

Write `REVIEW.md` beside the sheet. Include the catalog id, display name, type, rank, size, zone, and moves. Name the three item-icon files and say how the outline, shade, and color compare with them. Say so if the rim is softer than the icon ink. Paste the ready pose's `cutout.py` report, and say if it was regenerated and why.

Show the sheet in the reply and stop. The sample path is the `temp/` directory. Wait for the user to approve the ready pose.
