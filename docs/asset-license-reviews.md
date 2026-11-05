# Asset license reviews

Why the assets in [`asset-license-inventory.md`](asset-license-inventory.md) are approved
to ship. The inventory remains the source of truth for each file's status.

| Assets | License | Attribution ships in |
| --- | --- | --- |
| [Sound effects](#sound-effects) | Leohpaz publisher terms | `media/audio/sfx/CREDIT` |
| [LPC sprites](#lpc-sprites) | CC BY-SA 3.0, OGA-BY 3.0, and other per-layer licenses | `credits/` |
| [Fonts](#fonts) | SIL OFL 1.1 | `credits/Philosopher-OFL.txt`, `media/fonts/Philosopher-OFL.txt` |
| [Floor tileset](#floor-tileset) | MIT | `media/tilesets/CREDITS-floor.txt` |

## Sound effects

The release ships 62 MP3s selected by `scripts/release_assets.py`, encoded from two free
official packs.

| Pack | Sounds | Game SFX folders | Archive SHA-256 |
| --- | --- | --- | --- |
| [RPG Essentials SFX - Free!](https://leohpaz.itch.io/rpg-essentials-sfx-free) | 35 | `atk_magic`, `battle`, `buffs_heals`, `player_movement`, `ui_menu` | `68cca9ea2abf49e100ac3081c7068eef88c02662fb44d2f77296b1966316d234` |
| [Minifantasy - Dungeon Audio Pack](https://leohpaz.itch.io/minifantasy-dungeon-sfx-pack) | 27 | `dungeon` | `95165fcf13c4929e00510e74eff250989c381fd9cb803e0ad5601131c4beb92c` |

**Terms.** Both packs allow personal and commercial project use, with optional credit, and
prohibit redistributing or selling the pack itself. The creator's page comments also confirm
commercial use of RPG Essentials and say open-source use of Minifantasy Dungeon should be fine.
These are not MIT, CC0, or unrestricted terms, so do not distribute the sounds as a library
or standalone pack.

**Owner decision, 2026-10-29.** Neither pack page addresses loose files in a public source
repository. Joji Doi accepts that ambiguity for the 35 RPG Essentials sounds and approves the
release, for an open-source, noncommercial game with the Leohpaz credit kept and no original
pack ZIP committed. This is the owner's interpretation, not extra permission from Leohpaz. If
clarification makes replacement necessary, the owner plans to use AI-generated sounds.

**Limits.** Filenames, durations, and waveform correlation (0.934574 to 1.000000) tie each MP3
to its official WAV, but do not prove the download date or encoder command. Neither page grants
a general derivative-work permission, so this does not cover redistributing edited sounds as
assets.

## LPC sprites

The ledger identifies 257 shipped sprite PNGs as third-party LPC artwork.

**Terms.** The [generator's licensing guidance](https://github.com/LiberatedPixelCup/Universal-LPC-Spritesheet-Character-Generator#licensing-and-attribution-credits)
permits use under each layer's open license, including in public repositories and commercial
games. [CC BY-SA 3.0](https://creativecommons.org/licenses/by-sa/3.0/) requires attribution,
license identification, and ShareAlike for adapted art.
[OGA-BY 3.0](https://static.opengameart.org/OGA-BY-3.0.txt) requires attribution and
identification of changes. Do not treat this art as MIT or assume every layer offers the same
choices.

**Attribution.** `credits/` holds a credit file for each party member and these two notices:

- `LPC-CREDITS.csv` is the generator's unchanged `CREDITS.csv` at revision
  `86160e7507d6841259cff624a51a63c1d4847ee3`. The generator permits shipping this file in place
  of a composed attribution list.
- `LPC-BATTLE-MODIFICATIONS.txt` records the 102 LPC-derived `*_battle.png` sheets, enlarged
  from 64×64 to 256×256 per frame with smoothing. They keep their original licenses, including
  ShareAlike, because enlarging does not make them MIT artwork. Cinder Marshal's sprites are
  aliases of `fallen_angel_ash_wing` and are covered by the same notice. The Titch pair is
  project-generated and is not included.

**Limits.** The current generator list may differ from the version that made these sprites, so
the exact layers and license choice of each PNG are not independently established. The smoothing
filter is also unspecified. The owner has approved release with these limits.

## Fonts

The release ships one font binary, `media/fonts/Philosopher-Regular.ttf`. Both copies of its
notice match the [official text](https://github.com/google/fonts/blob/main/ofl/philosopher/OFL.txt)
after whitespace normalization. The Quintessential OFL notice also ships, but there is no
Quintessential font binary.

**Terms.** The [OFL 1.1](https://openfontlicense.org/open-font-license-official-text/) and its
[FAQ, sections 1.2–1.4](https://openfontlicense.org/ofl-faq/) permit bundling and redistribution
with software and in online repositories. Keep the copyright and notice, keep the font under OFL,
do not sell it by itself, and observe any Reserved Font Name restrictions for modified versions.
The game code does not need to adopt OFL.

## Floor tileset

`media/tilesets/floor.png` (`ALI-1109`) is project-authored artwork under the root MIT license.
The repository owner confirmed on 2026-10-29 that its tiles were AI-generated at the author's
direction and that no third-party artwork was downloaded for it. The layout is in the
[authoring guide](authoring.md#floor-tileset).
