# LPC sprite and font license review

Reviewed 2026-10-29. These are the two remaining rows after the floor and SFX
reviews. Existing owner release approval is retained; this document records
what was independently checked and the remaining provenance limits.

## LPC sprites

The ledger identifies 257 shipped sprite PNGs as third-party LPC artwork.
The official [generator licensing and attribution guidance](https://github.com/LiberatedPixelCup/Universal-LPC-Spritesheet-Character-Generator#licensing-and-attribution-credits)
permits use under the applicable per-layer open licenses. These licenses allow
redistribution, including public source repositories and commercial games,
subject to their conditions. [CC BY-SA 3.0](https://creativecommons.org/licenses/by-sa/3.0/)
requires attribution, license identification and ShareAlike for adapted art;
[OGA-BY 3.0](https://static.opengameart.org/OGA-BY-3.0.txt) also requires attribution
and identification of changes. Do not treat this art as MIT or assume every
layer offers the same license choices.

Only Aric's detailed party credit was present locally. Restored the original
Elise, Reiya, Jep and Kael credit files byte-for-byte from `do-i/agentic-rpg`
commit `08970359d6cb03586948625d29b0d3351dbbf785`. They now sit beside Aric's notice
in `assets/scenarios/rusted_kingdoms/credits/`.

The generator expressly permits distributing its entire `CREDITS.csv` as an
alternative to a composed attribution list. Added that file, unchanged, as
`credits/LPC-CREDITS.csv`, pinned to generator revision `86160e7507d6841259cff624a51a63c1d4847ee3`.
It retains author names, source URLs and license alternatives. The existing
in-game credits direct users to the shipped `credits/` directory.

This improves attribution coverage and restores omitted source records. It
does not independently establish the historical layer composition or license
choice of all 257 PNGs; the current generator list might also differ from the
historical generation version. The owner's prior approval remains recorded
with that limitation. Exact original party notices are preserved alongside
the broader current generator list. No sprite image was modified.

### Enemy battle sprite modifications

The owner clarified on 2026-10-29 that the original enemy sheets were enlarged
from 64x64 to 256x256 per animation frame, with smoothing. The original review covered 105 LPC-derived `*_battle.png` sheets in the
third-party ledger section; 102 distinct battle PNGs remain after retiring the
three duplicate enemy variants and their complete sprite pairs. Each has
a corresponding original PNG and TSX metadata confirming the 9x12 frame grid:
576x768 original sheets and 2304x3072 battle sheets. Source/derivative paths
and hashes are listed in the shipped `credits/LPC-BATTLE-MODIFICATIONS.txt`.
The actual smoothing filter and historical processing date remain unspecified.

The Cinder Marshal uses aliases: `cinder_marshal.tsx` refers to
`fallen_angel_ash_wing.png`, and `cinder_marshal_battle.tsx` refers to
`fallen_angel_ash_wing_battle.png`. That pair is covered by the same notice.
The separate Titch original/battle pair is recorded as project-generated
artwork, so it is not included in this LPC-specific notice.

The variants retain the applicable original layer licenses and attribution,
including ShareAlike for CC BY-SA components. Enlarging or smoothing does
not convert LPC-derived artwork into wholly original MIT artwork. The new
notice identifies the owner-described changes; no PNG was edited here.

### Enemy replacement and duplicate consolidation, 2026-10-29

Retired three ordinary encounter enemy variants and all four corresponding
PNG/TSX files for each variant. Encounter entries now use the existing retained
enemy IDs below, with their existing stats, AI and drops. Formation sizes,
weights and boss definitions are unchanged. In the ruins, the former mixed
zombie-guard formation is now two `zombie_guard_base` enemies.

| Retired enemy ID | Existing replacement ID | Reserved retired inventory IDs |
| --- | --- | --- |
| `lizard_dragon_guard_trident_guard` | `lizard_monster_winged_lizard` | ALI-0457, ALI-0458, ALI-0459, ALI-0460 |
| `skeleton_archer_greatbow_bones` | `skeleton_archer_base` | ALI-0621, ALI-0622, ALI-0623, ALI-0624 |
| `zombie_guard_wounded_guard` | `zombie_guard_base` | ALI-0733, ALI-0734, ALI-0735, ALI-0736 |

None of these retired enemies was a boss or referenced by dialogue or quests.
They are removed from the three enemy catalogs and five encounter files.
No TSX aliases were introduced; the retained replacements each use their own
same-named ordinary and battle PNG/TSX pair. Retired inventory IDs remain
reserved and must not be reused. The modification notice now covers only the
102 remaining LPC battle PNGs. No surviving PNG was modified.

## Fonts

The selected release payload contains exactly one font binary:
`media/fonts/Philosopher-Regular.ttf`. Its existing ledger hash matches the
original imported font. Both `credits/Philosopher-OFL.txt` and
`media/fonts/Philosopher-OFL.txt` ship with it. Their copyright and license text
match the [official Philosopher notice](https://github.com/google/fonts/blob/main/ofl/philosopher/OFL.txt)
after whitespace normalization; one local copy uses LF and the other CRLF.
Neither notice was modified by this review.

The [official OFL 1.1 terms](https://openfontlicense.org/open-font-license-official-text/)
and [OFL FAQ, sections 1.2–1.4](https://openfontlicense.org/ofl-faq/)
permit bundling and redistribution with software and in online repositories.
Keep the copyright and OFL notice, keep the font under OFL, do not sell it by
itself, and observe any Reserved Font Name restrictions for modified versions.
These font conditions do not require the game code to adopt OFL.

The Quintessential OFL notice is still shipped, but there is no Quintessential
font binary in the payload. Its presence is a retained notice, not evidence
of another shipped font. The font row passes the current redistribution and
notice review.

## Evidence and scope

Original party notices, the pinned full LPC credits, upstream generator README,
generator revision metadata and official Philosopher notice are retained in
`temp/2026-10-29-004-lpc-font-review/`. The six restored/added notices each
have a current SHA-256 and approved inventory entry. The new files must be
committed before the tracked-payload selection automatically includes them.
