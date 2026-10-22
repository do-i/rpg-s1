# Ancient Ruins Gate

`zone_04_ancient_ruins_01_gate` is a 64×44 outdoor ruin (2048×1408 pixels),
expanded from 40×28. A massive, worn defensive gatehouse and an abandoned
residential quarter replace the former flat ornamental ruins.

- The central gatehouse is an 18×14-tile object with deep stone towers, arrow
  slits, a partly fallen portcullis, chipped battlements, moss, and a collapsed
  right tower. Two ruined curtain-wall segments connect to its flanks.
- Six old stone homes use two damage variants: one has half its tiled roof
  missing and exposed upper rooms; the other has mostly collapsed roofing,
  broken beams, and low remnants of its front wall. Some instances are mirrored.
- The southwest drowned colonnade has a stone crossing, a small treasure
  island, and a southern ruined home reached along the longer loop.
- Broken, incomplete paving follows the abandoned lanes. The southeast homes
  and western quarter offer guarded caches and routes around the fallen walls.
- The northeast has a small surviving devotional stone and a cache. Adran waits
  beside the northern road with advice and supplies.
- Forty-one abandoned props fill former yards and supply areas: six scorched
  haystacks, four broken wheelbarrows, seven crate clusters, seven damaged
  barrels, seven dead trees, four charred stumps, three wrecked wagons, and
  three broken fences. Every sprite uses a straight-front orthographic camera
  pitched down 45 degrees, showing top and front surfaces without isometric yaw.

Six independent chest instances use the shared contents catalog. The two
original instance names remain intact for save compatibility; two of the new
caches are trapped. Each chest's center tile is blocked and has an open
standing tile beside it.

Sixteen encounter markers cover the gate, old lanes, houses, and optional loops.
Existing undead formations now pursue within two to four tiles and reactivate
on a 35-second interval. The excavation camp and arrival tiles have no markers.
The existing barrier-enemy item requirement remains in effect; the gate adds
no boss or new progression requirement.

The portal graph remains Marshland ↔ Gate ↔ Courtyard, with Gate ↔ Ruinwatch.
Arrival tiles in the gate are `[62,23]` from Marshland, `[1,23]` from the
Courtyard, and `[49,1]` from Ruinwatch. The neighboring maps change only their
return coordinates. All outgoing destinations retain their existing values.

The new `ancient_ruins.png`/`.tsx` object atlas is 640×896 pixels, containing
560 slots on a 32×32 grid, grouped into gatehouse, defensive-wall, and residential
bands. Houses are grouped from their actual map footprints, separated by a
transparent tile column, with both foundations aligned on atlas row 27.
The northwest, northeast, and southeast half-roof houses use a camera
pitched down 45 degrees facing their front facade, with no exterior side wall
visible. Generated sources, prompts, normalization details,
and object rectangles are retained in [the art sources](../art/ancient-ruins-objects/README.md).
Gate towers and house roof footprints are solid. The gate keeps a two-tile-wide
north/south passage at columns 32–33. Upper silhouettes use `foreground`;
foundations and low rubble use `decoration`. The owner-regenerated TSX contains
image-grid metadata without per-tile sorting offsets.

The separate `ancient_ruins_props.png`/`.tsx` atlas is 320×416 pixels, with
10 columns and 130 slots. Complete props are grouped into vegetation/hay,
supplies, and wreckage bands with transparent gutters. Its TSX uses minimal
image-grid metadata. Per-placement depth offsets live on the existing
`foreground` layer, following the lower-zone Marshland pattern. Object bases
use `decoration`; solid bases have collision, while dead-tree branches remain
walkable underneath. The original shortest exit/NPC approaches, both gate
passage columns, all encounter markers, all chest standing tiles, and all
arrival points remain reachable. Source art, prompts, tile rectangles, exact
placements, and collision footprints are recorded in
[the prop sources](../art/ancient-ruins-props/README.md).

Collision and layer patterns follow Starting Forest, Open Plains, and Marshland.
Higher-zone placeholder maps were not visual or structural references.
Existing layer IDs and order are preserved. In TMX order the map contains
`portals`, `collision`, `spawn_tile`, `ground`, `terrain`, `architecture`,
`decoration`, `item_boxes`, and `foreground`. Collision and spawn markers are
visible in Tiled. No layers were added.
