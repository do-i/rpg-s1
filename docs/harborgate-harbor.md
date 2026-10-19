# Harborgate shops, houses and harbor

Harborgate keeps its authored 41×50 exterior. Four shops beside the southern
waterfront copy the corrected Millhaven layouts and NPC anchors, while keeping
Harborgate's merchants, dialogue and stock.

Coordinates below are zero-based TMX floor tiles. All doors have reciprocal
portals, and arrival points are clear of their return triggers.

| Exterior doorway | Interior map | Size | Interior arrival | Town return |
| --- | --- | --- | --- | --- |
| `[6,36]` | `town_03_harborgate_shop_weapon` | 15×12 | `[7,10]` | `[6,37]` |
| `[15,36]` | `town_03_harborgate_shop_armor` | 15×12 | `[7,10]` | `[15,37]` |
| `[24,36]` | `town_03_harborgate_shop_item` | 15×12 | `[7,10]` | `[24,37]` |
| `[33,36]` | `town_03_harborgate_shop_mc` | 15×12 | `[7,10]` | `[33,37]` |
| `[20,5]` | `town_03_harborgate_shop` | 16×11 | `[7,9]` | `[20,6]` |
| `[36,5]` | `town_03_harborgate_house_02` | 10×9 | `[4,7]` | `[36,6]` |
| `[7,15]` | `town_03_harborgate_storage` | 15×13 | `[7,11]` | `[7,16]` |

The northwest timber building opens into a barn-like storehouse. Cargo crates,
barrels, grain sacks, hay bales, rope and cut timber fill four storage bays
around a broad central aisle. Cargo and walls block movement; the entrance,
aisle and passages between the bays remain clear. The room has no shop,
encounters or NPCs.

The former shop retains its map ID for compatibility with saves and the atlas,
but now contains a furnished village house without shop services. Its exterior
weapon sign has been removed. The northeast room contains a bed, cabinet,
table and chair. Its resident, Niko, uses the existing dark-skinned, unarmed
`village_male_person_market_runner` civilian sprite. His authored sprite anchor
is `[4,3]`; his collision center occupies floor tile `[5,4]`.

## Dock tiles

`assets/scenarios/rusted_kingdoms/media/tilesets/harbor_dock.png` is a 96×64 RGBA
sheet containing six 32×32 tiles. Its companion TSX provides three columns
(left, repeating center, right) and two rows (deck and end fascia with mooring
posts). The sheet uses hard alpha and a 48-color palette.

The dock occupies columns 29–36, rows 45–46, on the exterior's existing
`decoration` layer. Row 45 meets the land, and row 46 extends exactly one tile
into the water. Both end mooring posts on row 46 are blocked; the deck between
them is walkable. Water and shoreline fragments around the dock are blocked,
as is the southern map boundary. The existing underlying shore art is retained.

The built-in OpenAI imagegen tool generated the new dock using `town.png` as a
style reference. The result was trimmed, scaled to 96×64 with nearest-neighbor
sampling, given a hard alpha mask and reduced to 48 colors before integration.
No existing tilesheet or tile IDs were replaced.

## Storehouse props

`assets/scenarios/rusted_kingdoms/media/tilesets/storage_props.png` and
`storage_props.tsx` contain six 32×32 cargo props in a 96×64 atlas. The built-in
OpenAI imagegen tool generated the sheet; it was reduced with nearest-neighbor
sampling, hard alpha and a 48-color palette. The existing tilesheets are reused
for the timber floor, walls, doorway and lanterns. Rendered previews are written
to `temp/harborgate-storage/interior.png` and `exterior.png`.

Final storage-prop generation prompt:

> Use case: stylized-concept. Asset type: a tiny classic 16-bit top-down RPG storage/barn tileset with real alpha transparency. Generate a NEW single sprite atlas of exactly 3 columns and 2 rows, intended to be normalized to 96x64 pixels with six individual 32x32 cells. Each object is fully contained and centered in its own square with transparent padding, no overlap. Top row left to right: closed wooden shipping crate with diagonal brace; upright wooden barrel with iron hoops and visible oval top; tied burlap grain sack. Bottom row left to right: rectangular golden straw hay bale tied with rope; small coil of dock rope; short horizontal stack of cut timber. Orthographic RPG view, slight view of tops, fronts facing south, warm muted browns and golden ochre, strong dark pixel outlines, hard pixel edges, limited palette, simple readable shapes, no shadows outside cells. Each object fills 80 percent of its cell. All six cells have IDENTICAL square dimensions in an exact 3x2 grid, equal margins. No grid marks, labels, text, floor, ground, background, weapons, people, scenery or perspective. Transparent background.

Final generation prompt:

> Create a NEW project game asset, a modular wooden harbor docking platform sprite sheet, using the attached existing town tilesheet ONLY as a style and palette reference. Do not alter the existing tilesheet. Classic 32x32 RPG pixel art, orthographic top-down with a very slight front edge matching the reference's wooden bridge and buildings. Output a single rectangular wooden dock exactly 3 square tile columns across and 2 square tile rows tall, intended to be normalized to a 96x64 PNG. No margins, no labels, no grid lines, no background. Genuine alpha transparency only outside the dock perimeter. Horizontal weathered medium brown timber planks with restrained gold-brown highlights, dark seams, a few nail heads, left/right end rim boards, round thick mooring posts at the two bottom corners with a small coil of pale rope on one post. The center column must repeat horizontally seamlessly: its two outer vertical edges line up perfectly in both rows. The upper edge is a flat walk-on connection from land. Lower edge shows a narrow dark timber fascia and tiny transparent corners around the end posts, implying a pier above water. NO water, NO land, NO boats, NO people, NO perspective/isometric angle, NO side rail across walking surface. Hard pixel edges, limited palette, simple readable wood grain, match the warm rustic 16-bit style of the reference. Occupy the full canvas width and height with the rectangular dock except very small transparent corner cutouts.
