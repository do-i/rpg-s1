# Map layers

The shrine established this convention. All `town_01*` maps now use it and
have passed visual review. The four `town_02*` maps and the roof/tree
corrections have also passed visual review. Other maps retain their existing
layers until their own migration is reviewed.

Layer order, **top to bottom in Tiled** (`terrain` is optional):

| Layer | Rendering | Movement |
| --- | --- | --- |
| `foreground` | Overhead art; complete trees sort at their authored base | Does not block |
| `decoration` | Below characters; candles, signs, low objects, and object bases | Does not block |
| `architecture` | Below decoration; walls, buildings, and fixed supporting surfaces | Does not block |
| `terrain` | Above ground; paths and terrain overlays, where needed | Does not block |
| `ground` | Base floor or landscape | Does not block |
| `collision` | Hidden (`visible="0"`); solid footprints only | Every nonzero tile blocks movement |
| `portals` | Object group for transition triggers | Defines map connections |

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

`foreground` is an exact, case-sensitive tile-layer name. Keep it above the
other visible layers in Tiled so its preview matches the runtime. The engine
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
unmigrated maps retain their existing rendering. Tiles without this property
remain fixed overhead art.

For older maps, the runtime still honors visible collision layers because
many currently contain building and wall artwork. During migration, move
that artwork into visible layers before hiding `collision`.

Editor-only map templates live in `media/map_templates`, outside the
playable `media/maps` directory. They retain placeholder portal properties
for authors to replace and are excluded from release payloads.
