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

The ledger identifies 155 shipped sprite PNGs as third-party LPC artwork.

**Terms.** The [generator's licensing guidance](https://github.com/LiberatedPixelCup/Universal-LPC-Spritesheet-Character-Generator#licensing-and-attribution-credits)
permits use under each layer's open license, including in public repositories and commercial
games. [CC BY-SA 3.0](https://creativecommons.org/licenses/by-sa/3.0/) requires attribution,
license identification, and ShareAlike for adapted art.
[OGA-BY 3.0](https://static.opengameart.org/OGA-BY-3.0.txt) requires attribution and
identification of changes. Do not treat this art as MIT or assume every layer offers the same
choices.

**Attribution.** `credits/` holds a credit file for each party member and this notice:

- `LPC-CREDITS.csv` is the generator's unchanged `CREDITS.csv` at revision
  `86160e7507d6841259cff624a51a63c1d4847ee3`. The generator permits shipping this file in place
  of a composed attribution list.

**Limits.** The current generator list may differ from the version that made these sprites, so
the exact layers and license choice of each PNG are not independently established. The owner has approved release with these limits.

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

## Battle spell-group background — 2026-11-08

`ALI-1634`: generated at the repository owner's direction with the built-in OpenAI imagegen tool on 2026-11-08. The owner explicitly approved the final frame design and instructed Codex to apply it to the code and create a commit. Home-grown AI-generated artwork follows this repository's MIT policy; no third-party image was used as an input.

Destination: `assets/scenarios/rusted_kingdoms/media/images/ui/battle_spell_groups.png`. The 1030×1527 opaque PNG is copied unmodified from the accepted generator output.

Generation prompt:

Use case: stylized-concept
Asset type: one background illustration for a narrow vertical fantasy RPG spell-group selection frame, displayed at 190 x 282 pixels.
Primary request: a single coherent dark fantasy elemental tapestry representing seven disciplines: Fire, Water, Wind, Earth, Holy, Heal, and Utility. This is backdrop art behind translucent UI buttons, not a screenshot or a diagram.
Composition: portrait 2:3 aspect ratio, edge-to-edge continuous painted composition. Seven softly interwoven elemental motifs arranged loosely downward in a subtle zigzag: upper-left tiny ember and curling flame (Fire), upper-right swirling cool water (Water), left airy wind ribbons (Wind), middle-right ancient rock and roots (Earth), lower-left soft sacred golden light (Holy), lower-right jade healing aura around a small botanical sprout (Heal), bottom-left violet arcane wisps (Utility). The motifs should be legible at very small size without looking like seven separate icons. Blend them organically into one continuous scene.
Style: polished painterly fantasy game art, restrained texture, elegant atmospheric forms, dark charcoal and midnight slate foundation with muted elemental accents. Match an understated gold-trimmed dark RPG menu.
Lighting: dark, low contrast and low saturation throughout, sparse soft highlights at the edges of the motifs. The center band stays quiet so overlaid cream spell-group labels remain readable.
Constraints: one background only; no letters, no text, no labels, no runes resembling text, no buttons, no UI, no diamonds, no borders, no gold frame, no panels, no characters, no watermark. Opaque background. Keep all seven themes visible within the portrait crop.

## Zone 2 cave battle background — 2026-11-15

Generated at the repository owner's direction with the built-in OpenAI imagegen tool on 2026-11-15 to give both Open Plains cave maps their own battle image. No third-party image was supplied to the generator. Home-grown AI-generated artwork follows this repository's MIT policy.

Destination: `assets/scenarios/rusted_kingdoms/media/images/battle_bg/zone2-cave-bg-1280x468.webp` (`ALI-1839`). The generated opaque PNG was resized to 1280×468 and encoded as WebP at quality 92 with ImageMagick. The ground rectangle is restricted to the open foreground floor. Cave 01 references this background; Cave 02 has its own cliff-ravine background documented below.

Generation prompt:

```text
Use case: stylized-concept
Asset type: one ultrawide fantasy RPG battle background for the underground Open Plains caves in Zone 2 of Rusted Kingdoms.
Primary request: a dedicated dry natural cave interior with a broad unobstructed earthen and stone battle floor.
Style/medium: polished painterly fantasy game environment, realistic rock forms with softly painted detail, matching traditional illustrated RPG battle backdrops.
Composition/framing: panoramic 1280:468 aspect ratio (about 2.735:1), edge-to-edge opaque artwork. Camera at standing eye level looking slightly downward into a broad low-ceilinged cavern. Layered weathered brown-gray limestone walls, a dark winding tunnel receding in the rear, a few small stalactites overhead. The lower 40 percent is a continuous flat dusty tan-gray rocky floor, clearly visible and evenly lit across the central 85 percent of the frame, with enough empty floor for a group of enemy sprites. Keep boulders and stalagmites confined to the extreme sides and distant wall; no pillars obstructing the arena.
Lighting/mood: subdued underground atmosphere, soft reflected daylight filtering from an unseen entrance, warm earthy rock with cooler gray shadows, floor bright enough to read silhouettes. Distant tunnel stays dark, foreground floor stays readable.
Constraints: no characters, no creatures, no items, no treasure, no structures, no water, no shipwreck, no glowing crystals, no text, no UI, no borders, no watermark. One landscape image only. Fully underground; no visible sky or outdoor vista.
```

## Zone 2 Cave 02 battle background — 2026-11-15

Generated at the repository owner's direction with the built-in OpenAI imagegen tool on 2026-11-15 for Cave 02 in Zone 2. No third-party image was supplied as input. Home-grown AI-generated artwork follows this repository's MIT policy.

Destination: `assets/scenarios/rusted_kingdoms/media/images/battle_bg/zone2-cave-cliffs-bg-1280x468.webp` (`ALI-1840`). The opaque generated PNG was resized to 1280×468 and encoded as WebP at quality 92 with ImageMagick. Cave 02's battle ground rectangle covers the visible foreground floor, and its encounter document references this background.

Generation prompt:

```text
Use case: stylized-concept
Asset type: dedicated panoramic fantasy RPG battle background for the second Zone 2 cave in Rusted Kingdoms.
Primary request: a very dark subterranean rocky chasm battle arena, with sheer cliff faces instead of enclosed cave walls and a high shadowed ceiling.
Style/medium: polished painterly fantasy game environment, detailed natural stone, consistent with the approved Zone 2 cave battle backdrop.
Composition/framing: 1280:468 panoramic landscape, edge-to-edge opaque image. Ground-level camera looking slightly down across a broad clear rocky floor. Massive steep layered cliffs rise on both sides and taper into deep perspective, forming a narrow ravine-like cavern with a very high roof lost in shadow. Keep the distant view short: darkness and atmospheric haze close off the ravine within the middle distance, with no far vista. The upper half feels vertically vast and almost black; only faint charcoal rock contours reveal the high overhead darkness. Maintain enough soft local light on the central foreground floor so enemy silhouettes and feet remain clearly visible. Leave the center open for several enemy sprites; keep cliff debris tight to the side edges.
Lighting/mood: considerably darker and more enclosed than a sunlit ravine, deep charcoal and muted cool brown stone shadows, very sparse faint reflected light on the floor, a short pool of readable visibility around the battle area, distant background quickly disappearing into darkness.
Constraints: no ordinary vertical cave walls, no obvious low ceiling, no sky, no visible exterior, no characters or creatures, no objects, no ruins, no water, no crystals, no torches, no text, no UI, no border, no watermark. One opaque landscape image only.
```
