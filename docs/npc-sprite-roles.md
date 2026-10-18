# NPC sprite roles

Keep shopkeeper appearances in their designated shop maps. Other town NPCs
should use different sprites, with unarmed civilians preferred. All NPCs on
`zone_*` maps outside towns must carry weapons and wear armor, including those
in caves, ruins, and the final story areas. Replacements must have visible faces;
do not use sprites with full-face helmets. Prefer a sprite that is not already
used by another NPC on the same map.

The zone NPCs use the sword fighters, the axe fighter, `knight_01`, and the
open-face plate knights (`plate_knight_mace_knight` and
`plate_knight_open_helmet`). Their armor ranges from leather to plate. The
`female_sword_fighter_armored_katana` sheet has no visible weapon, so its name
alone does not make it suitable. Hearth Descent memories use these armed
appearances as well; playable party sprites are unchanged.

Sprite paths below are relative to `assets/scenarios/rusted_kingdoms/`.

| Reserved sprite | Shop role |
| --- | --- |
| `media/sprites/npc/merchant_base.tsx` | Item shopkeeper |
| `media/sprites/npc/merchant_wealthy_bowler.tsx` | Magic Core shopkeeper |
| `media/sprites/npc/male_blond_01.tsx` | Weapon shopkeeper |
| `media/sprites/npc/farmer_male_base.tsx` | Armor shopkeeper |
| `media/sprites/npc/female_wiz_01.tsx` | Apothecary |
| `media/sprites/npc/merchant_traveling_merchant.tsx` | Reserved merchant appearance |

The town-house couple uses `village_female_person_apron_keeper.tsx` and
`village_male_person_plain_dark_hair.tsx`, which are civilian appearances.
