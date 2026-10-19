# Ruinwatch shop badges

Created with the built-in OpenAI image generation tool, using the existing
weapon badge as the visual reference. `generated.png` retains the original
generated sheet. `badges.png` contains four normalized 32×32 tiles in order:
weapon, item, Magic Core, apothecary.

Each generated quadrant was cropped to its alpha bounds, reduced to 32×32 with
nearest-neighbor sampling, and given binary alpha. The four tiles replace only
shop atlas IDs 0, 7, 6 and 63, respectively. The atlas stays 256×256 with 64 tiles;
tile 63 was unused. Every pixel outside those four tiles is preserved exactly.
The other towns using this shared atlas also receive the clearer badge art.

The Ruinwatch red-roof northern building now houses Sedge's dedicated apothecary.
Its exterior and interior use tile 63. The armor showrooms use the shield badge
instead of the former sword badge.

## Generation prompt

```text
Use case: precise-object-edit / RPG pixel-art tile sheet.
Use the supplied existing 32x32 wooden shop badge (shown enlarged) only as style reference. Create ONE coherent sprite sheet of FOUR replacement shop badges arranged in a precise 2 by 2 grid. Output a square transparent PNG. Each quadrant contains exactly one square badge of identical size, centered, with ample transparent separation. All four use the same dark desaturated navy blue inset, reddish-brown wooden rectangular frame and small gold corner studs as the reference, with the same small top hanging bracket. Crisp chunky 32x32 pixel-art appearance, flat frontal view, limited palette, no antialiasing, no perspective, no words or letters. The full tile, including the hanger, must fit in each quadrant.
Top left: weapon shop: an unmistakable steel SWORD tilted 45 degrees, gold hilt at lower left, silver blade pointing to upper right. Long silver blade, short brown grip and clear small gold crossguard; no upright cross.
Top right: item/general supply shop: a tan/brown leather SUPPLY BAG with flap and gold buckle, a clearly visible carrying strap, simple readable silhouette.
Bottom left: magic core shop: a large brilliant cyan FACETED CRYSTAL with a purple shadow face and one tiny sparkle, clearly a magical gemstone. No moon, bottle or bag.
Bottom right: apothecary shop: a pale stone MORTAR AND PESTLE with two vivid green HERB LEAVES poking from the bowl, simple and easy to distinguish from the other signs. No potion bottle, moon or cross.
The icons must be visually readable after reduction to a real 32x32 tile. Keep the four frames consistent, do not add anything beyond these four badges. Transparent background outside the frames.
```
