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
