# Aric sprite alternatives

Three original character designs. Armored is now Aric's default in the game.
Open **Character**, select Aric, and use **Left/Right** to preview Armored,
Classic or Traveler. **Enter** applies the sprite and character selection;
**Escape** cancels the draft. The selected sprite is stored in the adventure's
save. Saves without a sprite choice use Armored.

The runtime PNG/TSX copies live in
`assets/scenarios/rusted_kingdoms/media/sprites/party/01_aric_{armored,classic,traveler}_walk.*`.

To try a version in the game, run one of these from the repository root:

```sh
python3 scripts/preview-aric.py classic
python3 scripts/preview-aric.py traveler
python3 scripts/preview-aric.py armored
python3 scripts/preview-aric.py original
```

The launcher uses `target/debug/rpg-s1` (build with `scripts/build.sh` first if
needed), copies the current scenario into a disposable preview package, and
starts Aric directly in Harborgate at `[7,17]`. Each window title identifies its
variant. Saves and settings are confined to that preview session; closing the
game removes the copy and its session data. Source assets and the normal
campaign remain intact. Preview audio is muted.

Use `--check` to validate a variant and load all preview maps without opening a
window, or `--binary target/release/rpg-s1` to use an existing release build.

| Variant | Sheet | Animated walk preview | Tiled metadata |
| --- | --- | --- | --- |
| Classic blue-and-gold swordsman | [01-classic.png](01-classic.png) | [Walk](01-classic-walk.gif) | [TSX](01-classic.tsx) |
| Traveling hero with blue scarf | [02-traveler.png](02-traveler.png) | [Walk](02-traveler-walk.gif) | [TSX](02-traveler.tsx) |
| Armored adventurer | [03-armored.png](03-armored.png) | [Walk](03-armored-walk.gif) | [TSX](03-armored.tsx) |

Every sheet is **576×256 RGBA**, with **9 columns × 4 rows of 64×64 cells**.
Rows are **up, left, down, right**. Column zero is standing; columns one
through eight form the walk loop. TSX animation owners are tiles 0, 9, 18 and
27, each referencing the following eight frames at 100 ms per frame, matching
the current character-atlas contract. Each GIF shows the four directions from
left to right at 3× nearest-neighbor scale and the same 100 ms cadence.

The built-in OpenAI imagegen tool generated the artwork. The existing Aric
sheet supplied pose guidance during the animation revision. The final export
extracts each pose, applies one uniform scale per variant, aligns the head
center and foot baseline, and packs the poses into exact cells. Each sheet uses
a shared palette of at most 64 colors and binary alpha. This directory preserves
the authoring sheets, previews and generation prompts for the runtime copies.

The armored south-facing cycle alternates the leading leg between the two
halves of the walk. Its [focused south-facing preview](03-armored-south-walk.gif)
shows the eight walking frames at 4× scale. The correction replaces only the
lower legs in the final four south-facing walking frames, preserving the
standing frame, upper bodies, other directions and animation timing.

## Generation prompts

### Classic swordsman

> Create a production pixel-art RPG CHARACTER WALK SPRITE SHEET, a NEW original design. The character is Aric, a 17-year-old male hero, tousled spiky copper-red hair, youthful uncovered face, lean athletic build. Classic polished 16-bit top-down RPG art matching 32px map tiles with 64x64 character cells; restrained warm pixel palette, crisp deliberate pixels, dark outlines, readable silhouette, no painterly blur. CRITICAL OUTPUT CONTRACT: EXACTLY NINE COLUMNS and FOUR ROWS, 36 separate full-body poses total. Canvas aspect ratio 9:4, transparent background, equal square cells, no spacing or margins between cells. Every cell contains exactly ONE whole character, centered on the SAME vertical centerline with feet at the SAME ground baseline. All sprites same scale, approximately 48 pixels tall within intended 64px cell, full head/feet/gear visible and padded. ROW ORDER: row1 facing UP/north, back visible with NO face; row2 facing LEFT/west, strict left side profile; row3 facing DOWN/south, front face visible; row4 facing RIGHT/east, strict right side profile. All nine sprites in each row face ONLY that row's direction. COLUMN ORDER: column1 is a relaxed STANDING STILL pose with both feet together; columns2-9 are eight genuinely DIFFERENT sequential phases of one smooth looping WALK: left foot forward contact, left loading/recoil, right passing under body, right lifting/high point, right foot forward contact, right loading/recoil, left passing under body, left lifting/high point, then loop smoothly to column2. Walk must have alternating legs, coherent opposite arm swing, small 1px torso bob and hair follow-through. This is a WALK, never running, never attacking, never turning. Keep head size/hair/face/clothing/weapon identical across ALL36 frames; no costume or body morphs, no random shifting. Stable registration, fixed root and consistent ground plane; no horizontal jitter, no bouncing weapon. Show both complete 8-frame gait cycles as mirrored stride halves, with distinctive leg silhouettes at contact/passing frames. No duplicate walking poses, no 3-frame ping-pong substitute. No labels, text, numbers, frames, grid marks, shadows, floor, scenery, background or extra accessories. Genuine alpha transparency outside every character.
> Outfit variant 1: refined blue-and-gold swordsman. Dark navy thigh-length sleeveless tabard with simple gold piping and small gold chest clasp over charcoal fitted clothes; steel shoulder guards and forearm bracers; brown leather belt and dark brown boots; a short iron sword sheathed securely at his left hip, hilt stays fixed to belt. Copper-red hair and blue tabard are the primary readable color blocks. Compact confident classic RPG proportions, subtle fabric motion. Balanced practical outfit.

### Traveling hero

> Create a production pixel-art RPG CHARACTER WALK SPRITE SHEET, a NEW original design. The character is Aric, a 17-year-old male hero, tousled spiky copper-red hair, youthful uncovered face, lean athletic build. Classic polished 16-bit top-down RPG art matching 32px map tiles with 64x64 character cells; restrained warm pixel palette, crisp deliberate pixels, dark outlines, readable silhouette, no painterly blur. CRITICAL OUTPUT CONTRACT: EXACTLY NINE COLUMNS and FOUR ROWS, 36 separate full-body poses total. Canvas aspect ratio 9:4, transparent background, equal square cells, no spacing or margins between cells. Every cell contains exactly ONE whole character, centered on the SAME vertical centerline with feet at the SAME ground baseline. All sprites same scale, approximately 48 pixels tall within intended 64px cell, full head/feet/gear visible and padded. ROW ORDER: row1 facing UP/north, back visible with NO face; row2 facing LEFT/west, strict left side profile; row3 facing DOWN/south, front face visible; row4 facing RIGHT/east, strict right side profile. All nine sprites in each row face ONLY that row's direction. COLUMN ORDER: column1 is a relaxed STANDING STILL pose with both feet together; columns2-9 are eight genuinely DIFFERENT sequential phases of one smooth looping WALK: left foot forward contact, left loading/recoil, right passing under body, right lifting/high point, right foot forward contact, right loading/recoil, left passing under body, left lifting/high point, then loop smoothly to column2. Walk must have alternating legs, coherent opposite arm swing, small 1px torso bob and hair follow-through. This is a WALK, never running, never attacking, never turning. Keep head size/hair/face/clothing/weapon identical across ALL36 frames; no costume or body morphs, no random shifting. Stable registration, fixed root and consistent ground plane; no horizontal jitter, no bouncing weapon. Show both complete 8-frame gait cycles as mirrored stride halves, with distinctive leg silhouettes at contact/passing frames. No duplicate walking poses, no 3-frame ping-pong substitute. No labels, text, numbers, frames, grid marks, shadows, floor, scenery, background or extra accessories. Genuine alpha transparency outside every character.
> Outfit variant 2: lighter traveling hero. Fitted warm brown leather vest over a dark indigo tunic, short cobalt-blue shoulder scarf draped close to back with a small loose end, tan leather fingerless gloves and belt, charcoal trousers, brown calf boots, small steel bracers. Copper-red spiky hair, youthful face. Short iron sword sheathed at left hip, hilt fixed to belt, no backpack or shield. Nimble natural walk, readable knees and hands. The scarf has subtle secondary motion but never changes length.

### Armored adventurer

> Create a production pixel-art RPG CHARACTER WALK SPRITE SHEET, a NEW original design. The character is Aric, a 17-year-old male hero, tousled spiky copper-red hair, youthful uncovered face, lean athletic build. Classic polished 16-bit top-down RPG art matching 32px map tiles with 64x64 character cells; restrained warm pixel palette, crisp deliberate pixels, dark outlines, readable silhouette, no painterly blur. CRITICAL OUTPUT CONTRACT: EXACTLY NINE COLUMNS and FOUR ROWS, 36 separate full-body poses total. Canvas aspect ratio 9:4, transparent background, equal square cells, no spacing or margins between cells. Every cell contains exactly ONE whole character, centered on the SAME vertical centerline with feet at the SAME ground baseline. All sprites same scale, approximately 48 pixels tall within intended 64px cell, full head/feet/gear visible and padded. ROW ORDER: row1 facing UP/north, back visible with NO face; row2 facing LEFT/west, strict left side profile; row3 facing DOWN/south, front face visible; row4 facing RIGHT/east, strict right side profile. All nine sprites in each row face ONLY that row's direction. COLUMN ORDER: column1 is a relaxed STANDING STILL pose with both feet together; columns2-9 are eight genuinely DIFFERENT sequential phases of one smooth looping WALK: left foot forward contact, left loading/recoil, right passing under body, right lifting/high point, right foot forward contact, right loading/recoil, left passing under body, left lifting/high point, then loop smoothly to column2. Walk must have alternating legs, coherent opposite arm swing, small 1px torso bob and hair follow-through. This is a WALK, never running, never attacking, never turning. Keep head size/hair/face/clothing/weapon identical across ALL36 frames; no costume or body morphs, no random shifting. Stable registration, fixed root and consistent ground plane; no horizontal jitter, no bouncing weapon. Show both complete 8-frame gait cycles as mirrored stride halves, with distinctive leg silhouettes at contact/passing frames. No duplicate walking poses, no 3-frame ping-pong substitute. No labels, text, numbers, frames, grid marks, shadows, floor, scenery, background or extra accessories. Genuine alpha transparency outside every character.
> Outfit variant 3: polished armored adventurer. Restrained silver steel breastplate and matching compact shoulder guards over a royal-blue padded tunic, muted gold fasteners, brown belt, charcoal trousers with steel knee guards and dark leather boots. No helmet; copper-red spiky hair and young face remain exposed. Short iron sword sheathed securely at left hip with fixed hilt. No cape and no shield. Sturdy but lean silhouette, grounded relaxed stride, clear leg articulation. Avoid bulky oversized armor.

### Armored south-facing leg correction

The built-in OpenAI imagegen tool edited a 2×2 reference of the first four
south-facing leg poses to produce their opposite phases. The four resulting
leg pairs were fitted to the existing lower-leg cells, mapped to the original
palette, and composited into the second half of the south-facing walk. Pixel
comparison verifies that changes stay within those four lower-leg patches.

> Use case: precise-object-edit. The image is a 2x2 reference sheet of FOUR cropped pairs of pixel-art armored legs ONLY, no upper bodies. For EACH of the four cells, reverse the leg stance by horizontally MIRRORING THAT INDIVIDUAL PAIR OF LEGS in place. Do NOT reverse the order of cells. Top-left stays top-left, top-right stays top-right, bottom-left stays bottom-left, bottom-right stays bottom-right. The long straight leg with its LOW boot that is currently on the LEFT side of each cell must be on the RIGHT side after your edit. The short bent/receding leg currently on the RIGHT must be on the LEFT after your edit. The boot on the RIGHT must visibly reach lower in ALL FOUR cells. Same boots, knee plates, pants and shading, same scale, same four leg-pose shapes, just reverse left/right within each cell. Preserve the blocky pixel art exactly, no smoothing, no extra detail, no redrawing outfit. Only legs and boots. No head, no torso, no arms, no whole character. Maintain a clean 2x2 grid of isolated leg pairs with generous equal transparent padding around every cell, no labels or grid lines. Fully transparent background. This supplies the OPPOSITE four walking phases for a south-facing sprite, so the side reversal is the entire purpose of the edit.

### Animation correction

> Edit IMAGE 1, the sprite-sheet target. IMAGE 2 is ONLY the exact walking-pose and layout reference. Fix the animation to use the exact 36 corresponding poses from image2, retaining image1's character face, hair, proportions, colors, armor/outfit and sheathed sword. OUTPUT exactly 9 columns x4 rows, transparent background, 36 equally-spaced sprites, aspect9:4. Rows UP(back), LEFT(profile), DOWN(front), RIGHT(profile). Column1 standing; columns2-9 eight sequential walk phases. VERY IMPORTANT: current image1 side views wrongly have extended legs in nearly every column. Replace those repeats. In columns4 and8 the knees pass each other and the silhouette is NARROW: the planted leg is straight DOWN under pelvis, the other leg BENT with raised heel, feet nearly touching horizontally. In columns5 and9 the advancing knee lifts, trailing foot is behind, narrow-to-medium stance. Columns2 and6 are opposite wide-stride contacts; columns3 and7 are compressed recoil with bent planted knee. The leading leg in col2 must be the TRAILING leg in col6, and vice versa. Arms counter-swing continuously. Copy the corresponding leg and arm pose from image2, not a newly invented sequence. Front/back rows must alternate left/right feet with the same phases. Idle is NOT repeated in walk. Preserve outfit and identity absolutely; stable head size/position, tiny 1px bob only, no clothing morph, no horizontal jitter. Show full whole bodies with transparent padding, no clipping or labels. Keep the existing high quality pixel art design from image1. Only pose/animation correctness is being changed. Do not return the original unchanged. Render true alpha transparency, no colored background.
