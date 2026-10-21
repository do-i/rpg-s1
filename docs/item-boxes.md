# Treasure boxes

Chest images, positions and instance names are authored as tile objects in a
Tiled object layer named `item_boxes`. Place this layer before `foreground`
so tree canopies and building roofs can cover the chests. Maps without chests
do not need an empty object layer.

Each object has a unique **Name** within its map, plus a string custom property
`item_box_id`. The name identifies the physical chest for save persistence;
`item_box_id` selects a reusable contents definition. Multiple objects, even
across maps, may share a definition while opening independently.

```yaml
# data/item_boxes.yaml, selected by manifest refs.item_boxes
item_boxes:
  supplies:
    loot:
      items:
        - id: potion
          qty: 2
```

To add a chest in Tiled:

1. Select the `item_boxes` object layer.
2. Select the closed tile in the `item_box` tileset and press **T** (Insert Tile).
3. Place a 32×32 tile object and give it a unique Name.
4. Add the string property `item_box_id` with a key from the contents catalog.
5. Paint a tile from `collision.tsx` in the `collision` layer at the tile
   containing the chest image's center. Keep an adjacent standing tile open
   so the player can interact, and save the collision layer hidden.

Positions belong only to TMX. Tile objects use Tiled's default bottom-left
anchor; the engine retains fractional pixel coordinates and uses the tile
containing the image center for interaction. Use the map's tile size without scaling or
rotation. The tile immediately after the selected closed image is its opened
frame. The engine draws one runtime sprite and changes that frame when opened;
the object image is not also drawn as a static map tile.

Collision uses that same center tile, including for fractional placements and
object-group offsets. For a default 32×32 tile object, its center is at
`(x + offsetx + 16, y + offsety - 16)` in map pixels. Opening the chest keeps
its collision tile in place.

Definitions contain loot, optional traps and optional `present` flag
conditions. The catalog is cached by the asset loader and resolved when a map
loads. Interacting with a chest uses the resolved runtime data, without file
reads. Editing a shared definition affects every placement referring to it.

Existing chest names remain unchanged during migration, preserving native and
imported Python opened-box keys. Renaming an instance makes it a new chest for
save purposes. Changing only `item_box_id` preserves the instance's opened state.

Packages without `refs.item_boxes` retain the previous per-map YAML chest
format for compatibility. Rusted Kingdoms uses the TMX/catalog format throughout.
