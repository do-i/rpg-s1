# Asset License Inventory

Status: active; two title-screen assets have completed evidence audits and are
blocked from release pending the proof recorded below.

This ledger records the provenance and release rights of every asset copied
into, or shipped from, this repository. It supports the Rusted Kingdoms port
and the temporary title-screen assets alike. It is an evidence record, not a
substitute for a license, permission, or legal review.

The canonical destination for a migrated Rusted Kingdoms source file is
`assets/scenarios/rusted_kingdoms/<source-relative-path>`. This destination
rule describes layout only; it does not authorize a copy.

## Shipping rule

An asset may be copied into a release payload or shipped only when its ledger
entry has review status `approved`. Every copied or shipped asset needs its
own entry, including source files that support another asset (for example,
TSX, font-license, and tile-image files). Do not infer an entry's evidence,
rightsholder, license, or permissions from a similarly named asset.

`unknown`, `needs-evidence`, and `blocked` are release blockers. When an
evidence field is unknown, record it literally as `unknown` and set the review
status to `needs-evidence` or `blocked`; never guess or substitute a likely
license. A `not-applicable` value is allowed only when the notes explain why.

## Review statuses

| Status | Meaning | May copy/ship? |
| --- | --- | --- |
| `draft` | Entry created; required facts have not yet been fully collected. | No |
| `needs-evidence` | One or more provenance or rights facts lack reliable evidence. | No |
| `needs-review` | Evidence is recorded and awaits an authorized reviewer. | No |
| `blocked` | Evidence or terms prohibit the intended use, or a known issue remains unresolved. | No |
| `approved` | Reviewer has confirmed that the recorded evidence permits the intended copy and shipment, including required notice and attribution. | Yes |
| `superseded` | The asset is no longer the release candidate; the entry is retained for history. | No |

Only a named reviewer may set an entry to `approved`. A later source, hash,
destination, modification, license, or permission change invalidates approval;
set the entry back to `needs-review` (or `needs-evidence` if proof is missing).

## How to add an entry

1. Assign a stable, never-reused ID in the form `ALI-0001`, incrementing the
   numeric part. Keep the ID when the entry is revised or superseded.
2. Record source and destination paths relative to their repository or package
   roots. Do not use machine-specific absolute paths.
3. Calculate and record the SHA-256 hash of the exact source file to be copied.
   If the destination differs after a permitted modification, record its hash
   and the modification in the entry notes.
4. Link primary source evidence, such as an official asset page, repository
   license, license file, permission statement, or a preserved local evidence
   file. Give a relative local-evidence path when evidence cannot be linked.
5. State the actual license identifier or name, and point to the license text
   or notice that will accompany the asset. Record required attribution exactly
   enough to reproduce it in a release notice.
6. Record each intended-use permission as `yes`, `no`, `conditional`, or
   `unknown`, with conditions in notes. Review the entry and choose a status.
7. Before packaging, compare every shipped asset against approved entries by
   destination path and content hash. Resolve mismatches before shipment.

## Entry fields

Use one level-three `Asset entry` section per file below. This scales without a
wide table, keeps evidence readable, and can be searched by ID, path, or
status. The required fields are intentionally repeated in each entry.

### Asset entry: `ALI-NNNN` — `<title or name>`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-NNNN` |
| Source path | `<relative source path>` |
| Destination path | `<relative repository destination path>` |
| Source SHA-256 | `<64 lowercase hexadecimal characters>` |
| Destination SHA-256 | `<64 lowercase hexadecimal characters, or not-applicable before copy>` |
| Asset kind | `<audio, image, font, tileset image, TSX, TMX, license text, other>` |
| Title/name | `<asset title or descriptive name>` |
| Creator/rightsholder | `<name, organization, or unknown>` |
| Source/evidence | `<URL and/or relative local evidence path; unknown is a blocker>` |
| License identifier/name | `<SPDX identifier if applicable, otherwise stated license name; unknown is a blocker>` |
| License text/notice location | `<relative path or source URL; unknown is a blocker>` |
| Required attribution | `<exact required credit/notice, none, or unknown>` |
| Modification status/details | `<unmodified, modified: details, or unknown>` |
| Redistribution permission | `<yes, no, conditional: details, or unknown>` |
| Commercial-use permission | `<yes, no, conditional: details, or unknown>` |
| Derivative-work permission | `<yes, no, conditional: details, or unknown>` |
| Review status | `<draft, needs-evidence, needs-review, blocked, approved, or superseded>` |
| Reviewer/date | `<reviewer and YYYY-MM-DD, or not yet reviewed>` |
| Related port task/wave | `<for example M0.10, M0.11, or M12.<wave/task>>` |
| Notes/blocker | `<conditions, uncertainty, remediation, or none>` |

## Reviewed asset entries

### Asset entry: `ALI-0001` — `Chronicles of the Lost Flame title artwork`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0001` |
| Source path | `rusted_kingdoms/assets/images/title_bg/title_lost_flame.webp` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/images/title_lost_flame.webp` |
| Source SHA-256 | `c36fc2defc4ddee6ba18e53a61c40b840713219f196775618d46ac344723a9bb` |
| Destination SHA-256 | `c36fc2defc4ddee6ba18e53a61c40b840713219f196775618d46ac344723a9bb` |
| Asset kind | image |
| Title/name | Chronicles of the Lost Flame title artwork |
| Creator/rightsholder | unknown |
| Source/evidence | Exact source file above; source LFS object declaration for the same SHA-256; source commits `1c501973580bbea290f6275741a986dfdc7ea83e` (adds the unlettered `title_image.webp`) and `92031d91dcef7a71b0805cabe9c58c2e70f124d9` (adds this file with message `Combine title image and text into a webp`); source `rusted_kingdoms/manifest.yaml`, `docs/design/scenario.md`, and `README.md` (references the title image and states that third-party assets retain their own terms, but does not identify terms for this file); inspected 2026-08-07 |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified during this port: source and destination are byte-for-byte identical. Source history identifies the upstream file as a derivative combining the previously committed `rusted_kingdoms/assets/images/title_bg/title_image.webp` with title text; the complete derivation recipe and rights for its inputs are unknown. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-07 |
| Related port task/wave | M0.10 |
| Notes/blocker | **Release blocker.** File possession, an LFS hash, and commit authorship do not establish copyright ownership or a redistribution license. Unblock with primary provenance for the base image and every nontrivial overlay input, the creator/rightsholder identity, and the applicable license or written permission explicitly allowing redistribution, commercial use, and derivative works. If that evidence cannot be obtained, replace this file with independently sourced artwork and create a new ledger entry for the replacement. |

### Asset entry: `ALI-0002` — `Embers of a Lost Flame title music`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0002` |
| Source path | `rusted_kingdoms/assets/audio/bgm/Chronicles_of_the_Lost_Flame_Title.mp3` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/audio/title_theme.mp3` |
| Source SHA-256 | `09716ca60887e053c9cd006c7541e7b9df868348239f099b668d33c1d8e72b5e` |
| Destination SHA-256 | `09716ca60887e053c9cd006c7541e7b9df868348239f099b668d33c1d8e72b5e` |
| Asset kind | audio |
| Title/name | Embers of a Lost Flame (source filename: `Chronicles_of_the_Lost_Flame_Title.mp3`) |
| Creator/rightsholder | unknown; embedded artist tag is `CeruleanPianoPipa788368`, but the tag does not establish identity or ownership |
| Source/evidence | Exact source file above; source LFS object declaration for the same SHA-256; source commit `d4be1ace21b23f4f1df63f0f32b5693c95687647` (`Add title bgm`); embedded ID3 tags (`title=Embers of a Lost Flame`, `album=Embers of a Lost Flame`, `artist=CeruleanPianoPipa788368`, `comment=Generated by Audjust`); source `rusted_kingdoms/data/audio/bgm_index.yaml`, `rusted_kingdoms/assets/audio/README-audio.md` (contains no entry for this title track), and `README.md` (does not identify terms for this file); [Audjust Terms of Service](https://www.audjust.com/legal) and [Audjust pricing](https://www.audjust.com/pricing), accessed 2026-08-07 |
| License identifier/name | unknown; no asset-specific license or permission record was found |
| License text/notice location | unknown; current Audjust terms are contextual evidence, not an asset-specific license |
| Required attribution | unknown |
| Modification status/details | Unmodified during this port: source and destination are byte-for-byte identical. The embedded `Lavf61.4.100`/`Lavc61.9.` encoder tags establish an export encoding path but not whether the underlying audio was generated from scratch or derived from submitted audio. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown; current Audjust terms limit free-plan output to personal, non-commercial use and make paid-plan commercial permission conditional on holding applicable rights to the submitted original |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-07 |
| Related port task/wave | M0.10 |
| Notes/blocker | **Release blocker.** Exact-title, artist-tag, and provider-tag web searches did not locate an authoritative public track page or asset-specific grant; search-result absence was not treated as a rights conclusion. Unblock with the Audjust generation/export record tied to this exact file and account, proof of the plan in effect when it was created, the versioned terms accepted at generation, and proof of rights to any submitted source audio. The collected grant must explicitly cover redistribution in the game, commercial use, derivatives, and any notice/attribution. If those records cannot be obtained, replace the track with independently licensed music and create a new ledger entry for it. |

### Asset entry: `ALI-0003` — `001_Hover_01` menu sound effect

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0003` |
| Source path | `rusted_kingdoms/assets/audio/sfx/ui_menu/001_Hover_01.mp3` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/audio/menu_hover.mp3` |
| Source SHA-256 | `2458f348156391f55587d16f1185fa3e2452392730168a9d839bffd4242a3e13` |
| Destination SHA-256 | `2458f348156391f55587d16f1185fa3e2452392730168a9d839bffd4242a3e13` |
| Asset kind | audio |
| Title/name | `001_Hover_01` (menu hover effect) |
| Creator/rightsholder | Leohpaz is identified as creator by the source `rusted_kingdoms/assets/audio/sfx/CREDIT`; rightsholder is not expressly identified. |
| Source/evidence | Exact source file above; source LFS object for the same SHA-256; source import commit `33261e5133f7b0d6614ed1e6b799ee7c7a1a926c` (`Add Leohpaz SFX files`) and its `rusted_kingdoms/assets/audio/sfx/CREDIT`; source `rusted_kingdoms/data/audio/sfx_index.yaml` maps `ui.hover` to this exact path; [Retro RPG 100 UI Sound Effects](https://leohpaz.itch.io/100-retro-rpg-ui-sound-effects) and [Leohpaz profile](https://leohpaz.itch.io/), accessed 2026-08-07. The store page describes a ten-hover pack and one hover demo sample, but publishes neither file hashes nor filenames, so it is contextual store evidence rather than exact-file provenance. |
| License identifier/name | Stated itch.io pack terms; no SPDX identifier and no license file tied to this exact hash were found. |
| License text/notice location | [Retro RPG 100 UI Sound Effects licensing section](https://leohpaz.itch.io/100-retro-rpg-ui-sound-effects); it permits use in projects, prohibits selling or freely distributing the pack, and says credits are not mandatory. Those public terms are not proof that this exact imported file came from the demo or a licensed purchase. |
| Required attribution | No mandatory credit stated on the candidate store page; retain voluntary credit `Sound effect created by Leohpaz — https://leohpaz.itch.io` if later approved. |
| Modification status/details | Unmodified during this port: source and destination are byte-for-byte identical. The source import is an MP3 in the `ui_menu` directory; no conversion or edit record was found. |
| Redistribution permission | unknown; the candidate pack terms prohibit distributing the asset pack, and the source does not prove whether this exact file is a licensed demo/purchase file or how those terms apply to this game's bundled copy. |
| Commercial-use permission | unknown; the candidate page permits project use but there is no exact-file provenance or acquisition record. |
| Derivative-work permission | unknown; the candidate page does not state a derivative-work grant, and the source contains no asset-specific terms. |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-07 |
| Related port task/wave | M0.11 |
| Notes/blocker | **Release blocker.** The creator credit and filename category make the referenced store page plausible, not conclusive: its free demo offers one unnamed hover sample while the full pack contains ten, and neither download is hash-addressed publicly. Unblock with the original package/download record that maps this SHA-256 to a named file, proof that the source acquisition complied with the applicable demo or purchase terms, and written clarification from Leohpaz that embedding this exact file in a distributable game is permitted (including commercial releases, if intended). Record any required notice before approval; otherwise replace it with an independently licensed effect. |

### Asset entry: `ALI-0004` — `013_Confirm_03` menu sound effect

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0004` |
| Source path | `rusted_kingdoms/assets/audio/sfx/ui_menu/013_Confirm_03.mp3` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/audio/menu_confirm.mp3` |
| Source SHA-256 | `81aabb9231ec1c3e2e2459f82bfc9878edfa7713890e6b4161e4ebba4d708c31` |
| Destination SHA-256 | `81aabb9231ec1c3e2e2459f82bfc9878edfa7713890e6b4161e4ebba4d708c31` |
| Asset kind | audio |
| Title/name | `013_Confirm_03` (menu confirmation effect) |
| Creator/rightsholder | Leohpaz is identified as creator by the source `rusted_kingdoms/assets/audio/sfx/CREDIT`; rightsholder is not expressly identified. |
| Source/evidence | Exact source file above; source LFS object for the same SHA-256; source import commit `33261e5133f7b0d6614ed1e6b799ee7c7a1a926c` (`Add Leohpaz SFX files`) and its `rusted_kingdoms/assets/audio/sfx/CREDIT`; source `rusted_kingdoms/data/audio/sfx_index.yaml` maps `ui.confirm` to this exact path; [Retro RPG 100 UI Sound Effects](https://leohpaz.itch.io/100-retro-rpg-ui-sound-effects) and [Leohpaz profile](https://leohpaz.itch.io/), accessed 2026-08-07. The store page describes a ten-confirm pack and one confirm demo sample, but publishes neither file hashes nor filenames, so it is contextual store evidence rather than exact-file provenance. |
| License identifier/name | Stated itch.io pack terms; no SPDX identifier and no license file tied to this exact hash were found. |
| License text/notice location | [Retro RPG 100 UI Sound Effects licensing section](https://leohpaz.itch.io/100-retro-rpg-ui-sound-effects); it permits use in projects, prohibits selling or freely distributing the pack, and says credits are not mandatory. Those public terms are not proof that this exact imported file came from the demo or a licensed purchase. |
| Required attribution | No mandatory credit stated on the candidate store page; retain voluntary credit `Sound effect created by Leohpaz — https://leohpaz.itch.io` if later approved. |
| Modification status/details | Unmodified during this port: source and destination are byte-for-byte identical. The source import is an MP3 in the `ui_menu` directory; no conversion or edit record was found. |
| Redistribution permission | unknown; the candidate pack terms prohibit distributing the asset pack, and the source does not prove whether this exact file is a licensed demo/purchase file or how those terms apply to this game's bundled copy. |
| Commercial-use permission | unknown; the candidate page permits project use but there is no exact-file provenance or acquisition record. |
| Derivative-work permission | unknown; the candidate page does not state a derivative-work grant, and the source contains no asset-specific terms. |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-07 |
| Related port task/wave | M0.11 |
| Notes/blocker | **Release blocker.** The creator credit and filename category make the referenced store page plausible, not conclusive: its free demo offers one unnamed confirm sample while the full pack contains ten, and neither download is hash-addressed publicly. Unblock with the original package/download record that maps this SHA-256 to a named file, proof that the source acquisition complied with the applicable demo or purchase terms, and written clarification from Leohpaz that embedding this exact file in a distributable game is permitted (including commercial releases, if intended). Record any required notice before approval; otherwise replace it with an independently licensed effect. |

### Asset entry: `ALI-0005` — Aric walk sprite atlas image

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0005` |
| Source path | `rusted_kingdoms/assets/sprites/party/01_aric_walk.png` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/assets/sprites/party/01_aric_walk.png` |
| Source SHA-256 | `bf728f9c5f37acbd8818ea3a9966cc261de90f2a0dd489866f816795d8500ef1` |
| Destination SHA-256 | `bf728f9c5f37acbd8818ea3a9966cc261de90f2a0dd489866f816795d8500ef1` |
| Asset kind | image |
| Title/name | Aric four-direction walk sprite atlas |
| Creator/rightsholder | bluecarrot16; Evert; TheraHedwig; Benjamin K. Smith (BenCreating); MuffinElZangano; Durrani; Pierre Vigier (pvigier); Eliza Wyatt (ElizaWy); Matthew Krohn (makrohn); Johannes Sjölund (wulax); Stephen Challener (Redshrike); JaidynReiman; Nila122; Fabzy; Michael Whitlock (bigbeargames), as recorded by the pinned source's per-character credit |
| Source/evidence | Pinned source file and its LFS declaration for the same SHA-256; pinned `credits/01_aric_credits.txt`; source `README.md` Credits and Attribution section; [official Universal LPC generator licensing and attribution guidance](https://github.com/LiberatedPixelCup/Universal-LPC-Spritesheet-Character-Generator#licensing-and-attribution-credits); source image commits `050cbbfa9cb3ea912cf74a702858e57308a926b7`, `1af64f33a9f95e5a194fb2c55130d4328d2907ff`, and `54f9b27ba734bf9e843104e37cdd5aba5b95eadc`; inspected 2026-08-10 |
| License identifier/name | Creative Commons Attribution-ShareAlike 3.0 Unported (`CC-BY-SA-3.0`) and OpenGameArt.org Attribution 3.0 (`OGA-BY-3.0`) for their respective component layers. The preserved per-layer record identifies the available choices; CC BY-SA 3.0 is selected where offered and OGA-BY 3.0 is selected for the two layers that offer only OGA-BY 3.0. |
| License text/notice location | [CC BY-SA 3.0](https://creativecommons.org/licenses/by-sa/3.0/) and [OGA-BY 3.0](https://static.opengameart.org/OGA-BY-3.0.txt); required local attribution is preserved at `assets/scenarios/rusted_kingdoms/credits/01_aric_credits.txt` |
| Required attribution | Preserve the complete creator, layer, license-choice, and source-link record in `assets/scenarios/rusted_kingdoms/credits/01_aric_credits.txt`; identify the sprite as Liberated Pixel Cup artwork under CC BY-SA 3.0 and OGA-BY 3.0 for their respective components; keep that attribution reasonably discoverable in a shipped game. |
| Modification status/details | Unmodified during this port: source and destination are byte-for-byte identical. In the source repository, the original generator export was replaced by commit `1af64f3` (`improve sprite quality`) and then normalized to a 576 by 256, four-row walk sheet by commit `54f9b27`; the pinned source continues to associate its per-character layer and contributor record with the resulting atlas. |
| Redistribution permission | yes, conditional on the respective CC BY-SA 3.0 and OGA-BY 3.0 notices and attribution requirements, plus ShareAlike for the CC BY-SA components |
| Commercial-use permission | yes, conditional on compliance with both selected licenses |
| Derivative-work permission | yes, conditional on the respective selected-license terms, including ShareAlike for adaptations of CC BY-SA components and change identification under OGA-BY 3.0 |
| Review status | `approved` |
| Reviewer/date | Codex asset-license review, 2026-08-10 |
| Related port task/wave | M4.15 |
| Notes/blocker | Approved for the byte-identical M4.15 copy with the preserved local credit and both selected-license notices. Do not remove or obscure the attribution, apply technical restrictions to the CC BY-SA components in conflict with that license, or distribute a modified version without satisfying each component's applicable terms. A different PNG hash requires a fresh evidence and attribution review. |

### Asset entry: `ALI-0006` — Aric walk Tiled tileset metadata

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0006` |
| Source path | `rusted_kingdoms/assets/sprites/party/01_aric_walk.tsx` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/assets/sprites/party/01_aric_walk.tsx` |
| Source SHA-256 | `6349f6d0253ef916fff749fe03d712736725685b807fc3c605225453e7f0654a` |
| Destination SHA-256 | `6349f6d0253ef916fff749fe03d712736725685b807fc3c605225453e7f0654a` |
| Asset kind | TSX |
| Title/name | Aric walk Tiled tileset metadata |
| Creator/rightsholder | Ninja (source commit author) and the credited LPC sprite contributors for the referenced atlas |
| Source/evidence | Exact pinned source file; source commit `050cbbfa9cb3ea912cf74a702858e57308a926b7` (`Add party sprites`); pinned `credits/01_aric_credits.txt`; source `README.md` Credits and Attribution section; inspected 2026-08-10 |
| License identifier/name | Creative Commons Attribution-ShareAlike 3.0 Unported (`CC-BY-SA-3.0`) and OpenGameArt.org Attribution 3.0 (`OGA-BY-3.0`) for distribution with the respective components of the Aric atlas |
| License text/notice location | [CC BY-SA 3.0](https://creativecommons.org/licenses/by-sa/3.0/) and [OGA-BY 3.0](https://static.opengameart.org/OGA-BY-3.0.txt); required local attribution is preserved at `assets/scenarios/rusted_kingdoms/credits/01_aric_credits.txt` |
| Required attribution | Preserve the companion Aric credit plus both selected-license notices with this metadata and its referenced image. |
| Modification status/details | Unmodified: source and destination are byte-for-byte identical. The TSX retains the sibling image source `01_aric_walk.png`, 64 by 64 tile size, 9 columns, 36 tiles, and four ordered walk animations. |
| Redistribution permission | yes, conditional on the companion attribution and both selected licenses' terms |
| Commercial-use permission | yes, conditional on compliance with both selected licenses |
| Derivative-work permission | yes, conditional on the respective selected-license terms |
| Review status | `approved` |
| Reviewer/date | Codex asset-license review, 2026-08-10 |
| Related port task/wave | M4.15 |
| Notes/blocker | Approved only as the unmodified companion metadata for `ALI-0005`. Its relative image reference is part of the approved registration and must continue to resolve inside the scenario package. |

### Asset entry: `ALI-0007` — Aric sprite attribution record

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0007` |
| Source path | `credits/01_aric_credits.txt` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/credits/01_aric_credits.txt` |
| Source SHA-256 | `fe2262356929bbf363480599ebf19194a7c1e31b7f46e3b265b444ca20eff20d` |
| Destination SHA-256 | `fe2262356929bbf363480599ebf19194a7c1e31b7f46e3b265b444ca20eff20d` |
| Asset kind | license text |
| Title/name | Aric sprite layer attribution record |
| Creator/rightsholder | not-applicable; this generated factual notice identifies the creators and license choices for the component artwork |
| Source/evidence | Exact pinned source file; source commit `58b7e62fe55651a2aeb44446e4bcc562996a63a7` (`Add credits for sprites`); official Universal LPC generator guidance says generated sprites must ship a composed credit list or the complete generator credits; inspected 2026-08-10 |
| License identifier/name | not-applicable; retained as the required attribution and license-choice record for `ALI-0005` and `ALI-0006`, not as independently used artwork |
| License text/notice location | [CC BY-SA 3.0](https://creativecommons.org/licenses/by-sa/3.0/) and [OGA-BY 3.0](https://static.opengameart.org/OGA-BY-3.0.txt) apply to their respective components of the associated Aric sprite; this file is their local attribution notice |
| Required attribution | Preserve this file unmodified and make it reasonably discoverable in a shipped game. |
| Modification status/details | Unmodified: source and destination are byte-for-byte identical. |
| Redistribution permission | yes; redistribution is required to preserve the associated asset's attribution |
| Commercial-use permission | yes; this is a required factual attribution record, not independently exploited artwork |
| Derivative-work permission | not-applicable; preserve the factual attribution unmodified |
| Review status | `approved` |
| Reviewer/date | Codex asset-license review, 2026-08-10 |
| Related port task/wave | M4.15 |
| Notes/blocker | The not-applicable fields are intentional because this file is a generated factual attribution notice. Its creators, layer paths, license choices, and source links are necessary evidence for the associated sprite and must not be removed. |

### Asset entry: `ALI-0008` — Ardel town Tiled map

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0008` |
| Source path | `rusted_kingdoms/assets/maps/town_01_ardel.tmx` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/assets/maps/town_01_ardel.tmx` |
| Source SHA-256 | `a90184da5454cec1edc7bcca3088b94edeb3958f94196f7b04e8abe6d44605b5` |
| Destination SHA-256 | `a90184da5454cec1edc7bcca3088b94edeb3958f94196f7b04e8abe6d44605b5` |
| Asset kind | TMX |
| Title/name | Ardel town map (`town_01_ardel`) |
| Creator/rightsholder | Ninja is the source commit author; copyright ownership and any additional contributors are not expressly documented. |
| Source/evidence | Exact pinned source file; source creation commit `9a85c1a6b142bdd38cd23df9f368e51febd83a55` (`Use real tilesets`) and subsequent file history through `0897035`; source README states that engine source is MIT and bundled third-party assets retain their own terms, but does not state a license for project-authored scenario maps; inspected 2026-08-10 |
| License identifier/name | unknown; the source README's MIT statement is expressly limited to engine source |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified during this port: source and destination are byte-for-byte identical. M4.11-M4.13 render its ground, terrain, and decoration layers from exact copied dependencies while reserving `collision` as non-visual data. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M4.11-M4.13 |
| Notes/blocker | **Release blocker.** The task authorizes this working-copy migration, but repository ownership and commit authorship do not establish a public redistribution license. Before release, obtain an explicit grant from the map rightsholder covering redistribution, commercial use, and derivatives, and record any attribution requirement. The referenced third-party terrain atlas is reviewed separately below. |

### Asset entry: `ALI-0009` — LPC terrain atlas image

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0009` |
| Source path | `rusted_kingdoms/assets/tilesets/ground/terrain-v7.png` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/assets/tilesets/ground/terrain-v7.png` |
| Source SHA-256 | `d098d23fbe6bb51b53f5d719d05a8e620d393f9d831bb14d2ed201b650163b7b` |
| Destination SHA-256 | `d098d23fbe6bb51b53f5d719d05a8e620d393f9d831bb14d2ed201b650163b7b` |
| Asset kind | tileset image |
| Title/name | `[LPC] Terrains` (`terrain-v7`) |
| Creator/rightsholder | bluecarrot16; Lanea Zimmerman (Sharm); Daniel Eddeland (Daneeklu); Richard Kettering (Jetrel); Zachariah Husiar (Zabin); Hyptosis; Casper Nilsson; Buko Studios; Nushio; ZaPaper; billknye; William Thompson; caeles; Redshrike; Bertram; Rayane Félix (RayaneFLX), as recorded by the pinned terrain credit |
| Source/evidence | Exact pinned source file and source LFS declaration; source import commit `cb425944cff27b5b25b632aa264be9c726d0c147` (`Add terrain packs`); pinned `rusted_kingdoms/assets/tilesets/ground/CREDITS-terrain.txt`, which identifies every component collection, creator, license choice, and OpenGameArt source URL; source README Credits and Attribution section; inspected 2026-08-10 |
| License identifier/name | Creative Commons Attribution-ShareAlike 3.0 Unported (`CC-BY-SA-3.0`). The preserved source notice offers CC BY-SA 3.0 for every ShareAlike component and CC BY 3.0 for the two attribution-only components; CC BY-SA 3.0 is the selected distribution license for the combined atlas. |
| License text/notice location | [CC BY-SA 3.0](https://creativecommons.org/licenses/by-sa/3.0/); complete local attribution at `assets/scenarios/rusted_kingdoms/assets/tilesets/ground/CREDITS-terrain.txt` |
| Required attribution | Preserve the complete creator, component-title, license, and source-link record in `assets/scenarios/rusted_kingdoms/assets/tilesets/ground/CREDITS-terrain.txt`; identify the combined terrain atlas as LPC artwork distributed under CC BY-SA 3.0; keep the notice reasonably discoverable in a shipped game. |
| Modification status/details | Unmodified during this port: source and destination are byte-for-byte identical. |
| Redistribution permission | yes, conditional on attribution, the CC BY-SA 3.0 notice, and ShareAlike terms |
| Commercial-use permission | yes, conditional on CC BY-SA 3.0 compliance |
| Derivative-work permission | yes, conditional on attribution, change identification, and ShareAlike terms |
| Review status | `approved` |
| Reviewer/date | Codex asset-license review, 2026-08-10 |
| Related port task/wave | M4.11 |
| Notes/blocker | Approved for this byte-identical copy with the complete local credit and selected CC BY-SA 3.0 notice. A changed image hash or removed attribution requires a fresh review. Git LFS covers the destination PNG. |

### Asset entry: `ALI-0010` — LPC terrain Tiled metadata

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0010` |
| Source path | `rusted_kingdoms/assets/tilesets/ground/terrain-v7.tsx` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/assets/tilesets/ground/terrain-v7.tsx` |
| Source SHA-256 | `285a0342a68b7e61c5e2aeb8fa92775d2dc851cd671705fb39ed6bbf915e8ed6` |
| Destination SHA-256 | `285a0342a68b7e61c5e2aeb8fa92775d2dc851cd671705fb39ed6bbf915e8ed6` |
| Asset kind | TSX |
| Title/name | `terrain-v7` Tiled tileset metadata |
| Creator/rightsholder | Ninja is the source import author; the credited LPC terrain contributors are the rightsholders of the referenced atlas. |
| Source/evidence | Exact pinned source file; source import commit `cb425944cff27b5b25b632aa264be9c726d0c147` (`Add terrain packs`); companion pinned terrain image and `CREDITS-terrain.txt`; inspected 2026-08-10 |
| License identifier/name | Creative Commons Attribution-ShareAlike 3.0 Unported (`CC-BY-SA-3.0`) for distribution as companion metadata to the reviewed terrain atlas |
| License text/notice location | [CC BY-SA 3.0](https://creativecommons.org/licenses/by-sa/3.0/); local attribution at `assets/scenarios/rusted_kingdoms/assets/tilesets/ground/CREDITS-terrain.txt` |
| Required attribution | Preserve the companion terrain credit and CC BY-SA 3.0 notice with this metadata and its referenced image. |
| Modification status/details | Unmodified: source and destination are byte-for-byte identical. The TSX retains its sibling `terrain-v7.png` reference, 32-pixel tiles, 32 columns, 2,048 tiles, and Wang metadata. |
| Redistribution permission | yes, conditional on the companion attribution and CC BY-SA 3.0 terms |
| Commercial-use permission | yes, conditional on CC BY-SA 3.0 compliance |
| Derivative-work permission | yes, conditional on CC BY-SA 3.0 terms |
| Review status | `approved` |
| Reviewer/date | Codex asset-license review, 2026-08-10 |
| Related port task/wave | M4.11 |
| Notes/blocker | Approved only as unmodified companion metadata for `ALI-0009`; its relative image reference must continue to resolve inside the scenario package. |

### Asset entry: `ALI-0011` — LPC terrain attribution record

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0011` |
| Source path | `rusted_kingdoms/assets/tilesets/ground/CREDITS-terrain.txt` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/assets/tilesets/ground/CREDITS-terrain.txt` |
| Source SHA-256 | `79c07c5b15b57a08bcf7c7fde6f56341c8055c587eb470468010ec04b2b38c7c` |
| Destination SHA-256 | `79c07c5b15b57a08bcf7c7fde6f56341c8055c587eb470468010ec04b2b38c7c` |
| Asset kind | license text |
| Title/name | `[LPC] Terrains` component attribution record |
| Creator/rightsholder | not-applicable; this factual notice identifies the creators and licenses for the associated terrain artwork |
| Source/evidence | Exact pinned source notice imported with the terrain pack by commit `cb425944cff27b5b25b632aa264be9c726d0c147`; its component-specific OpenGameArt links and license choices; inspected 2026-08-10 |
| License identifier/name | not-applicable; retained as the required attribution and license-choice record for `ALI-0009` and `ALI-0010` |
| License text/notice location | [CC BY-SA 3.0](https://creativecommons.org/licenses/by-sa/3.0/) applies to the combined associated atlas; this file is its local attribution notice |
| Required attribution | Preserve this file unmodified and keep it reasonably discoverable in a shipped game. |
| Modification status/details | Unmodified: source and destination are byte-for-byte identical. |
| Redistribution permission | yes; redistribution preserves the associated asset's required attribution |
| Commercial-use permission | yes; this is a required factual attribution record, not independently exploited artwork |
| Derivative-work permission | not-applicable; preserve the factual attribution unmodified |
| Review status | `approved` |
| Reviewer/date | Codex asset-license review, 2026-08-10 |
| Related port task/wave | M4.11 |
| Notes/blocker | The not-applicable fields are intentional because this file is a factual attribution notice. It must remain alongside the associated terrain atlas. |

### Asset entry: `ALI-0012` — `grass_cave_walls_24x14` atlas image

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0012` |
| Source path | `rusted_kingdoms/assets/tilesets/grass_cave_walls_24x14.png` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/assets/tilesets/grass_cave_walls_24x14.png` |
| Source SHA-256 | `60a58b6c8d9b1f99370b987a10ce0b4a76b97bd766d538b66af86c2fbd117520` |
| Destination SHA-256 | `60a58b6c8d9b1f99370b987a10ce0b4a76b97bd766d538b66af86c2fbd117520` |
| Asset kind | tileset image |
| Title/name | `grass_cave_walls_24x14` atlas |
| Creator/rightsholder | unknown |
| Source/evidence | Exact pinned source file and source import commit `9a85c1a6b142bdd38cd23df9f368e51febd83a55` (`Use real tilesets`); no creator, upstream package, license file, or public exact-filename result was found; inspected 2026-08-10 |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified during this port: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M4.12 |
| Notes/blocker | **Release blocker.** The task authorizes this exact working-copy migration so Ardel can be rendered locally, but possession and source commit history do not establish public redistribution rights. Obtain the original package, creator identity, exact-file provenance, and applicable license or replace the atlas before release. |

### Asset entry: `ALI-0013` — `grass_cave_walls_24x14` Tiled metadata

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0013` |
| Source path | `rusted_kingdoms/assets/tilesets/grass_cave_walls_24x14.tsx` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/assets/tilesets/grass_cave_walls_24x14.tsx` |
| Source SHA-256 | `b70c47ed5c00aaf644392422524935e9e6a81bd5eec7bc238dfe4564910bb85d` |
| Destination SHA-256 | `b70c47ed5c00aaf644392422524935e9e6a81bd5eec7bc238dfe4564910bb85d` |
| Asset kind | TSX |
| Title/name | `grass_cave_walls_24x14` Tiled metadata |
| Creator/rightsholder | Ninja is the source import author; ownership of the referenced atlas and metadata is not documented. |
| Source/evidence | Exact pinned source file and source import commit `9a85c1a6b142bdd38cd23df9f368e51febd83a55`; companion image `ALI-0012`; inspected 2026-08-10 |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified: the metadata retains its sibling PNG reference, 32-pixel tiles, 24 columns, and 336 tiles. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M4.12 |
| Notes/blocker | **Release blocker.** Resolve the companion image's provenance and identify the metadata's applicable terms before public redistribution. |

### Asset entry: `ALI-0014` — `icon_table_stage_14x9` atlas image

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0014` |
| Source path | `rusted_kingdoms/assets/tilesets/icon_table_stage_14x9.png` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/assets/tilesets/icon_table_stage_14x9.png` |
| Source SHA-256 | `c3327a0edcb2cf85bb42b269cc239a5661212e8fb711991c362ec4ea963d8c15` |
| Destination SHA-256 | `c3327a0edcb2cf85bb42b269cc239a5661212e8fb711991c362ec4ea963d8c15` |
| Asset kind | tileset image |
| Title/name | `icon_table_stage_14x9` atlas |
| Creator/rightsholder | unknown |
| Source/evidence | Exact pinned source file and source import commit `167f349f16cd5af934fa4339d4d51ead4b27c202` (`add more tiles and collision tests`); no creator, upstream package, license file, or public exact-filename result was found; inspected 2026-08-10 |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified during this port: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M4.12 |
| Notes/blocker | **Release blocker.** The task authorizes this exact working-copy migration so Ardel can be rendered locally, but source history does not establish rights. Obtain original-package provenance and a redistribution grant or replace the atlas before release. |

### Asset entry: `ALI-0015` — `icon_table_stage_14x9` Tiled metadata

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0015` |
| Source path | `rusted_kingdoms/assets/tilesets/icon_table_stage_14x9.tsx` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/assets/tilesets/icon_table_stage_14x9.tsx` |
| Source SHA-256 | `128b99b09af9dd41b9345e32eb113f740f2b9e0d72a3ec9cab7e15580cf218b7` |
| Destination SHA-256 | `128b99b09af9dd41b9345e32eb113f740f2b9e0d72a3ec9cab7e15580cf218b7` |
| Asset kind | TSX |
| Title/name | `icon_table_stage_14x9` Tiled metadata |
| Creator/rightsholder | Ninja is the source import author; ownership of the referenced atlas and metadata is not documented. |
| Source/evidence | Exact pinned source file and source import commit `167f349f16cd5af934fa4339d4d51ead4b27c202`; companion image `ALI-0014`; inspected 2026-08-10 |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified: the metadata retains its sibling PNG reference, 32-pixel tiles, 14 columns, and 126 tiles. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M4.12 |
| Notes/blocker | **Release blocker.** Resolve the companion image's provenance and identify the metadata's applicable terms before public redistribution. |

### Asset entry: `ALI-0016` — Astral Pixels `finestre` atlas image

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0016` |
| Source path | `rusted_kingdoms/assets/tilesets/astralpixels/finestre.png` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/assets/tilesets/astralpixels/finestre.png` |
| Source SHA-256 | `d09a5200065c60a9d749612d7bf9c92586e026206bb63831781e4ca814734867` |
| Destination SHA-256 | `d09a5200065c60a9d749612d7bf9c92586e026206bb63831781e4ca814734867` |
| Asset kind | tileset image |
| Title/name | `finestre` window atlas from RPG Interior Tileset 32x32 |
| Creator/rightsholder | AstralPixels is identified by the pinned local credit and candidate official asset page. |
| Source/evidence | Exact pinned source file; source import commit `582e7487602ea629e8a62eb3fb4ad57a992854c2` (`Add house map in ardel`); pinned `astralpixels/credit.txt`; [official RPG Interior Tileset 32x32 asset page](https://astralpixels.itch.io/rpg-interior-tileset-32x32-furniture-house-pack), accessed 2026-08-10 |
| License identifier/name | Candidate official asset-page terms; exact-file acquisition provenance is incomplete. |
| License text/notice location | Official asset page linked above permits personal and commercial project use and modification, prohibits redistributing/reselling/repackaging the assets, and does not require credit. |
| Required attribution | None stated by the candidate asset page; preserve the pinned voluntary credit while provenance is resolved. |
| Modification status/details | Unmodified during this port: source and destination are byte-for-byte identical. The 160-by-160 file appears to be a project-specific extracted window atlas, but the derivation recipe is not documented. |
| Redistribution permission | unknown; project embedding appears contemplated by the candidate terms, but the source does not prove exact-file acquisition or how the extracted atlas was produced. |
| Commercial-use permission | unknown pending exact-file provenance; the candidate terms permit commercial project use. |
| Derivative-work permission | unknown pending exact-file provenance; the candidate terms permit modification. |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M4.12 |
| Notes/blocker | **Release blocker.** Obtain the original `interior1.zip` acquisition record and preserve a reproducible mapping from that package to this extracted exact hash. Confirm that embedding the extracted subset in a distributable game complies with the no-repackaging term before release. |

### Asset entry: `ALI-0017` — Astral Pixels `finestre` Tiled metadata

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0017` |
| Source path | `rusted_kingdoms/assets/tilesets/astralpixels/finestre.tsx` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/assets/tilesets/astralpixels/finestre.tsx` |
| Source SHA-256 | `091787c2ce0e1361835b4698555cc011189011285c27accaea0c4f4732095f77` |
| Destination SHA-256 | `091787c2ce0e1361835b4698555cc011189011285c27accaea0c4f4732095f77` |
| Asset kind | TSX |
| Title/name | Astral Pixels `finestre` Tiled metadata |
| Creator/rightsholder | Ninja is the source import author; AstralPixels is the identified creator of the referenced artwork. |
| Source/evidence | Exact pinned source file and source import commit `582e7487602ea629e8a62eb3fb4ad57a992854c2`; companion image `ALI-0016`; pinned `astralpixels/credit.txt`; inspected 2026-08-10 |
| License identifier/name | unknown for the project-authored metadata; companion artwork has candidate official asset-page terms. |
| License text/notice location | `assets/scenarios/rusted_kingdoms/assets/tilesets/astralpixels/credit.txt` and the official page recorded by `ALI-0016` |
| Required attribution | Preserve the companion voluntary credit while provenance is unresolved. |
| Modification status/details | Unmodified: the metadata retains its sibling PNG reference, 32-pixel tiles, five columns, and 25 tiles. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M4.12 |
| Notes/blocker | **Release blocker.** Resolve the companion exact-file provenance and metadata terms before public redistribution. |

### Asset entry: `ALI-0018` — Astral Pixels source credit

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0018` |
| Source path | `rusted_kingdoms/assets/tilesets/astralpixels/credit.txt` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/assets/tilesets/astralpixels/credit.txt` |
| Source SHA-256 | `575258fbc7761b51ebec5f9041c50e31b5d06e5390a200e3d21ecad73eefe0a1` |
| Destination SHA-256 | `575258fbc7761b51ebec5f9041c50e31b5d06e5390a200e3d21ecad73eefe0a1` |
| Asset kind | attribution/source notice |
| Title/name | Astral Pixels source credit |
| Creator/rightsholder | not-applicable; this factual notice identifies the associated artwork's candidate official source. |
| Source/evidence | Exact pinned source notice and source import commit `582e7487602ea629e8a62eb3fb4ad57a992854c2`; official page linked by the notice; inspected 2026-08-10 |
| License identifier/name | not-applicable; retained as source evidence for the associated artwork. |
| License text/notice location | Official Astral Pixels asset page named in the file. |
| Required attribution | Preserve unmodified while the associated asset remains in the tree. |
| Modification status/details | Unmodified: source and destination are byte-for-byte identical. |
| Redistribution permission | yes; this factual source URL is preserved to support rights review. |
| Commercial-use permission | yes; this factual notice is not independently exploited artwork. |
| Derivative-work permission | not-applicable; preserve the factual notice unmodified. |
| Review status | `approved` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M4.12 |
| Notes/blocker | This notice does not itself prove exact-file provenance or remove the `ALI-0016` and `ALI-0017` blockers. |

### Asset entry: `ALI-0019` — Rusted Kingdoms scenario manifest

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0019` |
| Source path | `rusted_kingdoms/manifest.yaml` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/manifest.yaml` |
| Source SHA-256 | `93bd1d549152437237c37e398f9e9bc9cc7dcb6cb934cb013857a0a5c7340ec9` |
| Destination SHA-256 | `93bd1d549152437237c37e398f9e9bc9cc7dcb6cb934cb013857a0a5c7340ec9` |
| Asset kind | scenario YAML |
| Title/name | Rusted Kingdoms scenario manifest |
| Creator/rightsholder | unknown; Ninja authored commits in the file history, but commit authorship does not establish sole ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file; latest pinned change `1bcbdf4bb6bd2c7cee05b30c83240a13fbd722f0`; source README license section, which limits MIT to engine source and leaves bundled content under its own terms; inspected 2026-08-10 |
| License identifier/name | unknown; the source repository's MIT statement does not cover scenario content. |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified working-copy parity inclusion: source and destination are byte-for-byte identical. This manifest selects the production new-game inputs, scenario font, starting map, and protagonist sprite. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M3.13-M4.25 runtime closure |
| Notes/blocker | **Release blocker.** The user authorized local parity inclusion from their source tree, not public redistribution. Obtain an explicit grant from the scenario rightsholder covering redistribution, commercial use, derivatives, and attribution before release. |

### Asset entry: `ALI-0020` — Rusted Kingdoms party catalog

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0020` |
| Source path | `rusted_kingdoms/data/party.yaml` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/data/party.yaml` |
| Source SHA-256 | `24e95813f5d500f771f6fd9c3d37e46aba3c1b170832471edf18ab4760a6e04e` |
| Destination SHA-256 | `24e95813f5d500f771f6fd9c3d37e46aba3c1b170832471edf18ab4760a6e04e` |
| Asset kind | scenario YAML |
| Title/name | Rusted Kingdoms party catalog |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file; creation commit `3f5a7252041bf52ea0af20e7dde09271447db7d2` and latest pinned change `06ced7291f978b0c065697545dc17d6320cb44cb`; source README license boundary; inspected 2026-08-10 |
| License identifier/name | unknown; the source repository's MIT statement does not cover scenario content. |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified working-copy parity inclusion: source and destination are byte-for-byte identical. The production new-game builder reads the Aric record from this catalog. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M3.13-M4.25 runtime closure |
| Notes/blocker | **Release blocker.** Local parity inclusion is authorized for this task, but public redistribution requires an explicit scenario-content grant. |

### Asset entry: `ALI-0021` — Rusted Kingdoms balance data

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0021` |
| Source path | `rusted_kingdoms/data/balance.yaml` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/data/balance.yaml` |
| Source SHA-256 | `06e38b48fd5bed575fb8f6ca3990f8864ce2d891bb3d8c57602264913f7fad53` |
| Destination SHA-256 | `06e38b48fd5bed575fb8f6ca3990f8864ce2d891bb3d8c57602264913f7fad53` |
| Asset kind | scenario YAML |
| Title/name | Rusted Kingdoms balance data |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and latest pinned change `bb292536b530559d3be8cd615f64035a71017c3f`; source README license boundary; inspected 2026-08-10 |
| License identifier/name | unknown; the source repository's MIT statement does not cover scenario content. |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified working-copy parity inclusion: source and destination are byte-for-byte identical. The production new-game builder consumes its progression and economy caps. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M3.13-M4.25 runtime closure |
| Notes/blocker | **Release blocker.** Local parity inclusion is authorized for this task, but public redistribution requires an explicit scenario-content grant. |

### Asset entry: `ALI-0022` — Rusted Kingdoms intro cutscene

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0022` |
| Source path | `rusted_kingdoms/data/dialogue/intro_cutscene.yaml` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/data/dialogue/intro_cutscene.yaml` |
| Source SHA-256 | `f0ceb9f3b1d9cd13673dacd63f021b69686c2adfe9b85792f8fb23609b6e6d3a` |
| Destination SHA-256 | `f0ceb9f3b1d9cd13673dacd63f021b69686c2adfe9b85792f8fb23609b6e6d3a` |
| Asset kind | scenario dialogue YAML |
| Title/name | Rusted Kingdoms opening narration |
| Creator/rightsholder | unknown; source commit authorship does not establish rights to the authored narrative. |
| Source/evidence | Exact pinned source file; creation/history includes `a118974daf66c44ef30eb8a50c84d967d1e61417` and latest pinned path change `b9ca66094b4fb6b004fd5ec74be8f1ff7122dc44`; source README license boundary; inspected 2026-08-10 |
| License identifier/name | unknown; the source repository's MIT statement does not cover scenario narrative. |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified working-copy parity inclusion: source and destination are byte-for-byte identical. It drives the production Dialogue-to-World transition. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M3.14-M4.25 runtime closure |
| Notes/blocker | **Release blocker.** Obtain an explicit grant from the narrative rightsholder before public distribution; local parity inclusion alone is not rights evidence. |

### Asset entry: `ALI-0023` — Philosopher Regular scenario font

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0023` |
| Source path | `rusted_kingdoms/assets/fonts/Philosopher-Regular.ttf` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/assets/fonts/Philosopher-Regular.ttf` |
| Source SHA-256 | `9b9dced466f89eedbe6e9c6900e6bbcde6ad9bf3042b6e7715cd9ebed1125bd9` |
| Destination SHA-256 | `9b9dced466f89eedbe6e9c6900e6bbcde6ad9bf3042b6e7715cd9ebed1125bd9` |
| Asset kind | font |
| Title/name | Philosopher Regular |
| Creator/rightsholder | The Philosopher Project Authors, copyright 2011, as stated by the companion OFL notice. |
| Source/evidence | Exact pinned source font and import commit `69f9dc104b1dc448b0dbf13325c34532050eb32d`; exact companion `Philosopher-OFL.txt`; inspected 2026-08-10 |
| License identifier/name | SIL Open Font License 1.1 (`OFL-1.1`) |
| License text/notice location | `assets/scenarios/rusted_kingdoms/assets/fonts/Philosopher-OFL.txt` |
| Required attribution | Distribute the copyright notice and OFL-1.1 license with the font; do not sell the font by itself or use a Reserved Font Name for a modified version without permission. |
| Modification status/details | Unmodified: source and destination are byte-for-byte identical. The scenario-scoped copy is intentional because the manifest resolves fonts relative to its package root. |
| Redistribution permission | yes, conditional on OFL-1.1 |
| Commercial-use permission | yes, conditional on OFL-1.1 and not selling the font by itself |
| Derivative-work permission | yes, conditional on OFL-1.1, its license-continuity rule, and Reserved Font Name restriction |
| Review status | `approved` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M3.14-M4.25 runtime closure |
| Notes/blocker | Approved for this exact hash with the exact companion OFL notice. Git LFS covers the destination TTF. |

### Asset entry: `ALI-0024` — Philosopher OFL notice

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0024` |
| Source path | `rusted_kingdoms/assets/fonts/Philosopher-OFL.txt` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/assets/fonts/Philosopher-OFL.txt` |
| Source SHA-256 | `a812c3a94bd45b79bb8eb94a349f72a66b859f08dbdbb49777e9e9e81b8c9575` |
| Destination SHA-256 | `a812c3a94bd45b79bb8eb94a349f72a66b859f08dbdbb49777e9e9e81b8c9575` |
| Asset kind | license text |
| Title/name | Philosopher SIL Open Font License notice |
| Creator/rightsholder | not-applicable; this is the copyright and license notice supplied for the companion font. |
| Source/evidence | Exact pinned source notice and import commit `69f9dc104b1dc448b0dbf13325c34532050eb32d`; inspected 2026-08-10 |
| License identifier/name | SIL Open Font License 1.1 (`OFL-1.1`) |
| License text/notice location | This destination file is the complete local notice. |
| Required attribution | Preserve the file with the companion font. |
| Modification status/details | Unmodified, including its source CRLF line endings: source and destination are byte-for-byte identical. |
| Redistribution permission | yes; preservation satisfies the companion font's notice requirement |
| Commercial-use permission | yes; this required notice is not independently exploited artwork |
| Derivative-work permission | not-applicable; preserve the complete notice unmodified |
| Review status | `approved` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M3.14-M4.25 runtime closure |
| Notes/blocker | Approved as the exact companion notice for `ALI-0023`. |

### Asset entry: `ALI-0025` — Ardel map metadata

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0025` |
| Source path | `rusted_kingdoms/data/maps/town_01_ardel.yaml` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/data/maps/town_01_ardel.yaml` |
| Source SHA-256 | `06e8333dad57fc6139d2cffab14c1214a2be619469f704d354a31898bf20c8d8` |
| Destination SHA-256 | `06e8333dad57fc6139d2cffab14c1214a2be619469f704d354a31898bf20c8d8` |
| Asset kind | scenario map YAML |
| Title/name | Ardel Village metadata |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file; creation/history includes `9a121085c79bd52c4a0da30cf7bd68567f3bc03a` and latest pinned change `1bcbdf4bb6bd2c7cee05b30c83240a13fbd722f0`; source README license boundary; inspected 2026-08-10 |
| License identifier/name | unknown; the source repository's MIT statement does not cover scenario content. |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified working-copy parity inclusion: source and destination are byte-for-byte identical. Its `bgm: town.default` field is the production Ardel BGM selection. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M4.24-M4.25 |
| Notes/blocker | **Release blocker.** Obtain an explicit scenario-content grant before public redistribution. This task authorizes only the local parity copy. |

### Asset entry: `ALI-0026` — Rusted Kingdoms BGM index

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0026` |
| Source path | `rusted_kingdoms/data/audio/bgm_index.yaml` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/data/audio/bgm_index.yaml` |
| Source SHA-256 | `ae2c01bbaf243f3fcc6858a9beefd5c552f65edf7a05fc20b60433d17ac6aaa3` |
| Destination SHA-256 | `ae2c01bbaf243f3fcc6858a9beefd5c552f65edf7a05fc20b60433d17ac6aaa3` |
| Asset kind | scenario audio-index YAML |
| Title/name | Rusted Kingdoms BGM index |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file; creation commit `994eb9a63b7b6e41fc2c85fe3b07fe4578f52182` and latest pinned change `d4be1ace21b23f4f1df63f0f32b5693c95687647`; source README license boundary; inspected 2026-08-10 |
| License identifier/name | unknown; the source repository's MIT statement does not cover scenario content. |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified working-copy parity inclusion: source and destination are byte-for-byte identical. It maps `town.default` to `bgm/Whiteveil_Streets.mp3`; its other entries intentionally remain unresolved until later map waves copy their tracks. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M4.24-M4.25 |
| Notes/blocker | **Release blocker.** Local runtime parity does not establish permission to redistribute this scenario index or any track it references. |

### Asset entry: `ALI-0027` — Whiteveil Streets BGM

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0027` |
| Source path | `rusted_kingdoms/assets/audio/bgm/Whiteveil_Streets.mp3` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/assets/audio/bgm/Whiteveil_Streets.mp3` |
| Source SHA-256 | `1c2411d98b665d011b216c8f96c6ceb4321892afd25a4a355c526f9245ef3584` |
| Destination SHA-256 | `1c2411d98b665d011b216c8f96c6ceb4321892afd25a4a355c526f9245ef3584` |
| Asset kind | audio |
| Title/name | Whiteveil Streets |
| Creator/rightsholder | unknown |
| Source/evidence | Exact pinned source file; source import commit `78ce00d184a2b4768a63e25be81cc578ec76aec3` and later path history through `33261e5133f7b0d6614ed1e6b799ee7c7a1a926c`; companion `rusted_kingdoms/assets/audio/README-audio.md`, which records a generation prompt and `https://www.youtube.com/watch?v=kDwZaYTKr9I` but no creator identity, generation provider, acquisition record, or license; MP3 inspection shows a 194.56-second, 64-kbps, 48-kHz stereo file with only `encoder=Lavf62.3.100`; inspected 2026-08-10 |
| License identifier/name | unknown |
| License text/notice location | unknown; the companion README is provenance context, not a redistribution license. |
| Required attribution | unknown |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M4.24-M4.25 |
| Notes/blocker | **Release blocker.** The user authorized intentional local inclusion from their source tree, but possession, an encoder tag, a prompt, and a YouTube link do not prove ownership or permission. Before release, obtain the exact generation/acquisition record, creator/rightsholder identity, applicable versioned terms or written grant, and proof that redistribution in a commercial game and derivative use are permitted. Otherwise replace the track with independently licensed music and add a new ledger entry. Git LFS covers this MP3. |

### Asset entry: `ALI-0028` — Rusted Kingdoms audio provenance README

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0028` |
| Source path | `rusted_kingdoms/assets/audio/README-audio.md` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/assets/audio/README-audio.md` |
| Source SHA-256 | `e6ac9366ee7c63097dd607c31c40866cfc98d90915864e6dd73557100fbd805f` |
| Destination SHA-256 | `e6ac9366ee7c63097dd607c31c40866cfc98d90915864e6dd73557100fbd805f` |
| Asset kind | provenance record |
| Title/name | Rusted Kingdoms BGM prompt and source notes |
| Creator/rightsholder | unknown; the file does not identify who authored its prompts or responses. |
| Source/evidence | Exact pinned source file; creation commit `78ce00d184a2b4768a63e25be81cc578ec76aec3` and latest pinned path change `b9ca66094b4fb6b004fd5ec74be8f1ff7122dc44`; inspected 2026-08-10 |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified local evidence copy: source and destination are byte-for-byte identical. It preserves the only source-tree context currently associated with Whiteveil Streets. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M4.24-M4.25 |
| Notes/blocker | **Release blocker.** Retain this file as local provenance evidence while `ALI-0027` is unresolved, but do not ship it or infer music rights from it without an explicit grant covering the record itself and the associated track. |

## Compact blank entry

Copy this block for each new file. Retain every field; use `unknown` rather
than omitting a fact, and use `not-applicable` only with an explanation.

```text
ID: ALI-NNNN
Source: <relative path>
Destination: <relative path>
Source SHA-256: <hash>
Destination SHA-256: <hash or not-applicable>
Kind: <kind>
Name: <name>
Creator/rightsholder: <name or unknown>
Evidence: <URL and/or relative local path>
License: <identifier/name>
License text/notice: <relative path or URL>
Required attribution: <text, none, or unknown>
Modification: <unmodified/modified details/unknown>
Redistribution: <yes/no/conditional/unknown>
Commercial use: <yes/no/conditional/unknown>
Derivatives: <yes/no/conditional/unknown>
Review: <status>
Reviewer/date: <name and YYYY-MM-DD, or not yet reviewed>
Port task/wave: <task>
Notes/blocker: <text or none>
```
