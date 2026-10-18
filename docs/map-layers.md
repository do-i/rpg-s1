# Map layers

The shrine established this convention. Ardel, Millhaven, and their roof/tree
corrections have passed visual review. The remaining 44 maps now use the
same separation and await visual review. All 58 playable maps have hidden
collision layers; encounter spawn markers are also hidden and remain active.

Use the single 32×32 opaque red (`#FF0000`) tile from `collision.tsx` /
`collision.png` to paint blocked cells. Show the `collision` layer in Tiled to
inspect or edit it; hide the art layers above it or use Tiled's layer isolation
to see the mask clearly. Keep `visible="0"` when saving for gameplay. The game
uses nonzero tile IDs for blocking, regardless of the tile's color or visibility.
Global IDs vary by map; select the tile from the `collision` tileset rather than
assuming it has GID 1. No additional layer is needed.

Usual layer order, **top to bottom in Tiled** (`terrain` is optional):

| Layer | Rendering | Movement |
| --- | --- | --- |
| `foreground` | Overhead art; complete trees sort at their authored base | Does not block |
| `decoration` | Below characters; candles, signs, low objects, and object bases | Does not block |
| `architecture` | Below decoration; walls, buildings, and fixed supporting surfaces | Does not block |
| `terrain` | Above ground; paths and terrain overlays, where needed | Does not block |
| `ground` | Base floor or landscape | Does not block |
| `collision` | Hidden (`visible="0"`); solid footprints only | Every nonzero tile blocks movement |
| `portals` | Object group for transition triggers | Defines map connections |

Use only the established tile-layer names: `ground`, `terrain`, `architecture`,
`decoration`, `foreground`, `collision`, and `spawn_tile`. Each name appears at
most once per map. **New layer names require the owner's explicit permission.**
Discuss a layout that cannot fit these layers before adding another layer.
Keep existing `portals` and `boss_enemy` object groups for gameplay metadata.
Hidden `spawn_tile` layers retain encounter data beside the collision mask.
Signs are visible decoration; their tileset IDs identify interactions.

The shrine omits `terrain`. Keep the existing plural name `portals` for the
object group; the runtime uses that name to find map transitions. TMX stores
these layers in the reverse order, from bottom to top.
Objects and object groups may use Tiled's `visible="0"` to hide their editor
markers. This does not disable portal triggers or other gameplay objects.

Place a tall object's upper tiles in `foreground`, its bottom/base tiles in
`decoration`, and its solid footprint in `collision`. For a three-tile-tall
column, the upper two tiles can cover a character while their floor cells
remain walkable. The bottom tile is drawn below characters and its cell is
blocked. A character standing below that base then appears in front of it.

Use the upper/base split for tall props and walls. Buildings keep collision
throughout their roof footprint, including cells whose art is in `foreground`.
Set collision independently of the visible art. Fallen logs and other low
objects belong in `decoration`.
Keep wall tiles in `architecture` and candles or signs in `decoration`.
Their transparent pixels reveal the wall beneath. For example, an inn sign
can occupy the same cell as its building wall without replacing that wall.
Keep floor and terrain artwork in their own layers.

Fixed surfaces can also support decoration in the same cell: for example,
a kitchen counter belongs in `architecture` when separate cookware occupies
`decoration` above it. Freestanding low furniture stays in `decoration`.

Put the uppermost terrain artwork in `terrain` and its opaque backing in
`ground`. When transparent terrain needs a separate supporting surface, use
`architecture` beneath `terrain`, retaining the authored appearance. Seven
outdoor maps use this order: Starting Forest, Marshland, Sunken Cave, Mountain
Foothills 01, and Mountain Pass 01–03. Their cliff or terrain backing uses
`architecture`; no additional layer is needed. Covered tiles that contribute
no pixels can be omitted. Collision and encounter data remain independent.

The Ardel migration keeps map sizes, portals, NPC metadata, and prop positions.
Roofs, complete trees, and tall furniture tops use `foreground`. Roof footprints
and tree bases remain blocked, while cells behind the tree canopy are walkable.
The town's existing northern boundary stays blocked. Wall-backed furniture retains the
wall's collision even where its upper artwork is in `foreground`. Interior
floor tiles continue beneath separated furniture wherever needed to avoid
transparent gaps. The epilogue retains its original sanctuary layout and
pew direction while adopting the same upper/base split.

Millhaven uses the same separation for its town, inn, shop, and mill. Paths
and the tilled field have their own `terrain` layer above the grass base.
Complete trees, the scarecrow's upper tile, roofs, and tall furniture tops use
`foreground`; roof footprints and solid object bases remain blocked. Wall-backed shop shelves
keep the wall collision underneath. Map sizes, portal objects, NPC metadata,
and artwork positions are preserved.

`foreground` is the exact, case-sensitive name of the overhead layer.
Keep it above the other visible layers in Tiled. The engine
places ordinary foreground tiles above players, NPCs, enemies, and world objects,
independently of their position in the TMX. Ordinary visible layers retain their
authored order below characters. A hidden foreground layer is not rendered.

For trees, keep all four tiles in `foreground`, including the lower canopy
pixels that share tiles with the trunk. The TSX integer property `sort_base_y`
sets the sorting baseline in map pixels below each tile's top edge: `64` for
the upper row and `32` for the lower row. Both rows then sort at the same root
position. Characters behind the tree are covered by its complete opaque
silhouette; characters in front are drawn above it. Transparent outer corners
keep their original shape. The property affects only `foreground` tiles, so
legacy maps retain their existing rendering. Tiles without this property
remain fixed overhead art.

The Starting Forest and Corrupted Forest also contain taller trees assembled
with repeated connecting crown rows. Keep all those tiles in the same
`foreground` layer. Its string property `sort_base_y_overrides` contains
whitespace-separated `column,row:pixel_offset` entries, using zero-based map
coordinates. For example, `13,4:96 13,5:64 13,6:32` places three rows at one
root baseline. An entry overrides the TSX baseline only at its occupied cell;
other tiles retain their TSX baseline or fixed overhead behavior. The parser
ignores entries on empty cells so erasing tiles in Tiled does not prevent map
loading. It rejects duplicate coordinates, out-of-bounds coordinates, and
negative or malformed offsets. Connected forest barriers retain their collision
and use per-cell baselines for their visible edges. Connected forest crowns
sort at their actual root or bottom edge, including repeated crown rows;
do not give their upper tiles a one-row (`32`) baseline. A walkable upper
crown must still cover a player behind it. A base drawn in `decoration`
also counts when determining the foreground crown's baseline.

The remaining-map migration preserves map dimensions, tile positions, portal
objects, NPC metadata, and spawn data. Roofs, cliffs, water barriers, and map
boundaries remain solid. Only identified upper tree, furniture, and standalone
column cells become walkable. Cave voids retain their transparency. A few
source maps reused tile-layer IDs or had stale `nextlayerid` values; the new
layers use unique IDs while portal and boss object groups retain theirs.

The runtime still honors visible collision layers in older external maps.
Move their artwork into visible layers before hiding `collision`.

Editor-only map templates live in `media/map_templates`, outside the
playable `media/maps` directory. They retain placeholder portal properties
for authors to replace and are excluded from release payloads.
