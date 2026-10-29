# Leohpaz sound-effect license review

Reviewed 2026-10-29. Scope: the 62 MP3 sound effects selected by
`scripts/release_assets.py` for the current game release. Existing owner
approval is retained; this review replaces the earlier assumption that these
files necessarily require proof of a paid-pack purchase.

## Official sources and terms

| Official pack | Shipped sounds | Terms checked |
| --- | --- | --- |
| [RPG Essentials SFX - Free!](https://leohpaz.itch.io/rpg-essentials-sfx-free) | 35 | Free project use; the creator also confirms commercial use in the page comments. Credits are optional. Redistribution or sale of the asset pack itself is prohibited. |
| [Minifantasy - Dungeon Audio Pack](https://leohpaz.itch.io/minifantasy-dungeon-sfx-pack) | 27 | Personal and commercial project use is allowed. Credits are optional. Redistribution or sale of the asset pack itself is prohibited. The creator also says use of this free pack in an open-source game should be fine in the page comments. |

Both official downloads are available for zero payment through the publisher's
name-your-price download flow. A paid-pack purchase is therefore not required
for the files mapped to these free archives. These are custom publisher terms,
not MIT, CC0 or an unrestricted sound-redistribution license.

Using the mapped sounds as game effects in the release is consistent with the
publisher's project-use permission. That is the interpretation used for this
review; neither page separately specifies this project's loose-file packaging.
Keep the sound effects under the publisher's terms and avoid distributing or
selling them as a sound library or standalone asset pack. Existing creator
credit at `assets/scenarios/rusted_kingdoms/media/audio/sfx/CREDIT` remains
unchanged, although the publisher makes credit optional.

## Owner release decision, 2026-10-29

Joji Doi explicitly accepts the remaining ambiguity about including the 35
RPG Essentials Free sounds as loose files in this public GitHub game repository
and approves proceeding with the release. The owner's intended use is an
open-source, noncommercial game, with the existing Leohpaz credit preserved
and no original sound-pack ZIP committed or published with the game. The
27 Minifantasy Dungeon sounds additionally have the creator's published
open-source-project clarification noted above.

This is the owner's documented interpretation and acceptance of uncertainty,
not an additional permission obtained from Leohpaz. The sounds retain their
publisher terms. If clarification later makes replacement necessary, the owner
plans to replace the affected effects with AI-generated sounds.

## File mapping and audio evidence

Downloaded the official `RPG_Essentials_Free.zip` (48 WAV files) and
`Minifantasy_Dungeon_SFX.zip` (62 WAV files), rather than relying on filenames
from third-party repositories. Every selected MP3 has exactly one matching
filename stem in these two archives.

| Game SFX folder | Official pack | Selected files |
| --- | --- | --- |
| `atk_magic` | RPG Essentials Free | 7 |
| `battle` | RPG Essentials Free | 10 |
| `buffs_heals` | RPG Essentials Free | 7 |
| `player_movement` | RPG Essentials Free | 1 |
| `ui_menu` | RPG Essentials Free | 10 |
| `dungeon` | Minifantasy Dungeon | 27 |

All 62 source/destination pairs have equal decoded duration. Comparing mono
PCM at 8000 Hz yields cosine correlations from 0.934574 to 1.000000 (full
precision is in the report). These waveform comparisons support the inference
that the MP3s are encodings of the corresponding official WAVs. WAV and MP3
bytes differ, and this does not recover the original encoder command or prove
the historical download date. The previously imported MP3 hashes are retained
in the inventory; no sound file was changed by this review.

The pages do not separately spell out a general derivative-work permission.
This review covers the existing game effects and their WAV-to-MP3 encoding;
it does not infer a broader right to sublicense or redistribute edited sounds
as assets. The 48 tracked MP3s excluded from the release are outside the audio
comparison scope of this review.

## Evidence

Official page snapshots, free-download metadata, both ZIPs, extracted source
WAVs, the comparison script, per-file source/destination hashes and waveform
results are in `temp/2026-10-29-003-sfx-license-review/`.

Archive SHA-256 values:

- `essentials.zip`: `68cca9ea2abf49e100ac3081c7068eef88c02662fb44d2f77296b1966316d234`
- `dungeon.zip`: `95165fcf13c4929e00510e74eff250989c381fd9cb803e0ad5601131c4beb92c`
