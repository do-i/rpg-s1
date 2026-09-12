# Asset License Inventory

Status: active. The 2026-09-12 backfill closed every *mechanical* gap: the audit
now reports 1,012 selected release assets, 1,016 ledger entries, **0 payload
files without an entry**, **0 destination-hash mismatches**, and 0 ledger
errors. What remains is evidence, not bookkeeping — 9 approved, 1 needs-review,
and 1,006 needs-evidence — so the release is still blocked, but the blockers are
now decisions rather than unknowns. Run
`python3 scripts/check_asset_rights.py --report-only` for the current report;
this dated snapshot is not a substitute for rerunning it from a committed tree.

The remaining work sorts into three piles, in increasing order of difficulty:

1. **88 project-authored files** have no third-party rights question at all.
   They need one explicit redistribution grant from the owner, which would clear
   the whole pile in a single decision. See the project-authored backfill
   section.
2. **1 entry awaits re-review** — `ALI-0024`, whose approval lapsed when its
   line endings were normalized. The notice text is intact.
3. **~915 migrated files** carry genuinely unknown third-party provenance and
   cannot be cleared from inside this repository. Each needs upstream evidence
   or replacement. This is the real cost of the release, and no amount of
   ledger work shrinks it.

Note on paths: entries written before 2026-09-12 cite the pinned source tree as
`../agentic-rpg`. That checkout is now `../zzz-agentic-rpg`; the pinned commit
`08970359d6cb03586948625d29b0d3351dbbf785` is unchanged and remains its HEAD, so
the evidence those entries cite is still reachable under the new directory name.

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

`scripts/check_asset_rights.py` performs that comparison against the exact set
of tracked files selected by the release workflow after applying
`release-assets-exclude.txt`. The exclusion list is for repository files kept
as authoring/source material and is not a substitute for rights approval when
a file is selected for shipment. The checker is a strict local and GitHub
release gate. During ongoing evidence work, `--report-only` prints the same
findings without treating unresolved rights as a command failure; ledger syntax
and structural errors still fail in either mode.

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
| Destination path | `assets/scenarios/rusted_kingdoms/media/sprites/party/01_aric_walk.png` |
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
| Destination path | `assets/scenarios/rusted_kingdoms/media/sprites/party/01_aric_walk.tsx` |
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
| Destination path | `assets/scenarios/rusted_kingdoms/media/maps/town_01_ardel.tmx` |
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
| Destination path | `assets/scenarios/rusted_kingdoms/media/tilesets/ground/terrain-v7.png` |
| Source SHA-256 | `d098d23fbe6bb51b53f5d719d05a8e620d393f9d831bb14d2ed201b650163b7b` |
| Destination SHA-256 | `d098d23fbe6bb51b53f5d719d05a8e620d393f9d831bb14d2ed201b650163b7b` |
| Asset kind | tileset image |
| Title/name | `[LPC] Terrains` (`terrain-v7`) |
| Creator/rightsholder | bluecarrot16; Lanea Zimmerman (Sharm); Daniel Eddeland (Daneeklu); Richard Kettering (Jetrel); Zachariah Husiar (Zabin); Hyptosis; Casper Nilsson; Buko Studios; Nushio; ZaPaper; billknye; William Thompson; caeles; Redshrike; Bertram; Rayane Félix (RayaneFLX), as recorded by the pinned terrain credit |
| Source/evidence | Exact pinned source file and source LFS declaration; source import commit `cb425944cff27b5b25b632aa264be9c726d0c147` (`Add terrain packs`); pinned `rusted_kingdoms/assets/tilesets/ground/CREDITS-terrain.txt`, which identifies every component collection, creator, license choice, and OpenGameArt source URL; source README Credits and Attribution section; inspected 2026-08-10 |
| License identifier/name | Creative Commons Attribution-ShareAlike 3.0 Unported (`CC-BY-SA-3.0`). The preserved source notice offers CC BY-SA 3.0 for every ShareAlike component and CC BY 3.0 for the two attribution-only components; CC BY-SA 3.0 is the selected distribution license for the combined atlas. |
| License text/notice location | [CC BY-SA 3.0](https://creativecommons.org/licenses/by-sa/3.0/); complete local attribution at `assets/scenarios/rusted_kingdoms/media/tilesets/ground/CREDITS-terrain.txt` |
| Required attribution | Preserve the complete creator, component-title, license, and source-link record in `assets/scenarios/rusted_kingdoms/media/tilesets/ground/CREDITS-terrain.txt`; identify the combined terrain atlas as LPC artwork distributed under CC BY-SA 3.0; keep the notice reasonably discoverable in a shipped game. |
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
| Destination path | `assets/scenarios/rusted_kingdoms/media/tilesets/ground/terrain-v7.tsx` |
| Source SHA-256 | `285a0342a68b7e61c5e2aeb8fa92775d2dc851cd671705fb39ed6bbf915e8ed6` |
| Destination SHA-256 | `285a0342a68b7e61c5e2aeb8fa92775d2dc851cd671705fb39ed6bbf915e8ed6` |
| Asset kind | TSX |
| Title/name | `terrain-v7` Tiled tileset metadata |
| Creator/rightsholder | Ninja is the source import author; the credited LPC terrain contributors are the rightsholders of the referenced atlas. |
| Source/evidence | Exact pinned source file; source import commit `cb425944cff27b5b25b632aa264be9c726d0c147` (`Add terrain packs`); companion pinned terrain image and `CREDITS-terrain.txt`; inspected 2026-08-10 |
| License identifier/name | Creative Commons Attribution-ShareAlike 3.0 Unported (`CC-BY-SA-3.0`) for distribution as companion metadata to the reviewed terrain atlas |
| License text/notice location | [CC BY-SA 3.0](https://creativecommons.org/licenses/by-sa/3.0/); local attribution at `assets/scenarios/rusted_kingdoms/media/tilesets/ground/CREDITS-terrain.txt` |
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
| Destination path | `assets/scenarios/rusted_kingdoms/media/tilesets/ground/CREDITS-terrain.txt` |
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
| Destination path | `assets/scenarios/rusted_kingdoms/media/tilesets/grass_cave_walls_24x14.png` |
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
| Destination path | `assets/scenarios/rusted_kingdoms/media/tilesets/grass_cave_walls_24x14.tsx` |
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
| Destination path | `assets/scenarios/rusted_kingdoms/media/tilesets/icon_table_stage_14x9.png` |
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
| Destination path | `assets/scenarios/rusted_kingdoms/media/tilesets/icon_table_stage_14x9.tsx` |
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
| Destination path | `assets/scenarios/rusted_kingdoms/media/tilesets/astralpixels/finestre.png` |
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
| Destination path | `assets/scenarios/rusted_kingdoms/media/tilesets/astralpixels/finestre.tsx` |
| Source SHA-256 | `091787c2ce0e1361835b4698555cc011189011285c27accaea0c4f4732095f77` |
| Destination SHA-256 | `091787c2ce0e1361835b4698555cc011189011285c27accaea0c4f4732095f77` |
| Asset kind | TSX |
| Title/name | Astral Pixels `finestre` Tiled metadata |
| Creator/rightsholder | Ninja is the source import author; AstralPixels is the identified creator of the referenced artwork. |
| Source/evidence | Exact pinned source file and source import commit `582e7487602ea629e8a62eb3fb4ad57a992854c2`; companion image `ALI-0016`; pinned `astralpixels/credit.txt`; inspected 2026-08-10 |
| License identifier/name | unknown for the project-authored metadata; companion artwork has candidate official asset-page terms. |
| License text/notice location | `assets/scenarios/rusted_kingdoms/media/tilesets/astralpixels/credit.txt` and the official page recorded by `ALI-0016` |
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
| Destination path | `assets/scenarios/rusted_kingdoms/media/tilesets/astralpixels/credit.txt` |
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
| Destination SHA-256 | `0c35147fa6457d1063ba573de641a3544518a737d0bf8b96577f66f2b625d242` |
| Asset kind | scenario YAML |
| Title/name | Rusted Kingdoms scenario manifest |
| Creator/rightsholder | unknown; Ninja authored commits in the file history, but commit authorship does not establish sole ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file; latest pinned change `1bcbdf4bb6bd2c7cee05b30c83240a13fbd722f0`; source README license section, which limits MIT to engine source and leaves bundled content under its own terms; inspected 2026-08-10 |
| License identifier/name | unknown; the source repository's MIT statement does not cover scenario content. |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | **Modified by this port after copying.** The destination SHA-256 above is the current committed file and is authoritative for what would ship; the source SHA-256 is retained so the derivation stays traceable. This entry previously asserted byte-for-byte identity, which later content work made false; corrected 2026-09-12. Being a derivative does not clear the original's rights, which remain unknown. This manifest selects the production new-game inputs, scenario font, starting map, and protagonist sprite. |
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
| Destination SHA-256 | `52aaeb79ebdb55ff8d8d3a9a2fe89be02f42443423a944428ac98af51023d954` |
| Asset kind | scenario YAML |
| Title/name | Rusted Kingdoms party catalog |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file; creation commit `3f5a7252041bf52ea0af20e7dde09271447db7d2` and latest pinned change `06ced7291f978b0c065697545dc17d6320cb44cb`; source README license boundary; inspected 2026-08-10 |
| License identifier/name | unknown; the source repository's MIT statement does not cover scenario content. |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | **Modified by this port after copying.** The destination SHA-256 above is the current committed file and is authoritative for what would ship; the source SHA-256 is retained so the derivation stays traceable. This entry previously asserted byte-for-byte identity, which later content work made false; corrected 2026-09-12. Being a derivative does not clear the original's rights, which remain unknown. The production new-game builder reads the Aric record from this catalog. |
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
| Destination SHA-256 | `6aba77a84c85aac323af3ceea7c7340d2ddcc8395e112b32953f4d2fb5a4fe80` |
| Asset kind | scenario YAML |
| Title/name | Rusted Kingdoms balance data |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and latest pinned change `bb292536b530559d3be8cd615f64035a71017c3f`; source README license boundary; inspected 2026-08-10 |
| License identifier/name | unknown; the source repository's MIT statement does not cover scenario content. |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | **Modified by this port after copying.** The destination SHA-256 above is the current committed file and is authoritative for what would ship; the source SHA-256 is retained so the derivation stays traceable. This entry previously asserted byte-for-byte identity, which later content work made false; corrected 2026-09-12. Being a derivative does not clear the original's rights, which remain unknown. The production new-game builder consumes its progression and economy caps. |
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
| Destination path | `assets/scenarios/rusted_kingdoms/media/fonts/Philosopher-Regular.ttf` |
| Source SHA-256 | `9b9dced466f89eedbe6e9c6900e6bbcde6ad9bf3042b6e7715cd9ebed1125bd9` |
| Destination SHA-256 | `9b9dced466f89eedbe6e9c6900e6bbcde6ad9bf3042b6e7715cd9ebed1125bd9` |
| Asset kind | font |
| Title/name | Philosopher Regular |
| Creator/rightsholder | The Philosopher Project Authors, copyright 2011, as stated by the companion OFL notice. |
| Source/evidence | Exact pinned source font and import commit `69f9dc104b1dc448b0dbf13325c34532050eb32d`; exact companion `Philosopher-OFL.txt`; inspected 2026-08-10 |
| License identifier/name | SIL Open Font License 1.1 (`OFL-1.1`) |
| License text/notice location | `assets/scenarios/rusted_kingdoms/credits/Philosopher-OFL.txt` |
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
| Destination path | `assets/scenarios/rusted_kingdoms/credits/Philosopher-OFL.txt` |
| Source SHA-256 | `a812c3a94bd45b79bb8eb94a349f72a66b859f08dbdbb49777e9e9e81b8c9575` |
| Destination SHA-256 | `bc884dc1c5a18df27e250d7d4c1e5574234215017161290a1010bfcdba880b9a` |
| Asset kind | license text |
| Title/name | Philosopher SIL Open Font License notice |
| Creator/rightsholder | not-applicable; this is the copyright and license notice supplied for the companion font. |
| Source/evidence | Exact pinned source notice and import commit `69f9dc104b1dc448b0dbf13325c34532050eb32d`; inspected 2026-08-10 |
| License identifier/name | SIL Open Font License 1.1 (`OFL-1.1`) |
| License text/notice location | This destination file is the complete local notice. |
| Required attribution | Preserve the file with the companion font. |
| Modification status/details | **Line endings normalized from CRLF to LF.** The source is CRLF; the committed destination is LF, so the two are no longer byte-for-byte identical. The notice text itself is unchanged — only the line terminators differ — so the OFL-1.1 requirement to preserve the copyright and license notice is still met in substance. Recorded because the previous entry asserted byte-for-byte identity, which was false. |
| Redistribution permission | yes; preservation satisfies the companion font's notice requirement |
| Commercial-use permission | yes; this required notice is not independently exploited artwork |
| Derivative-work permission | not-applicable; preserve the complete notice unmodified |
| Review status | `needs-review` |
| Reviewer/date | Automated backfill audit, 2026-09-12; awaiting an authorized reviewer |
| Related port task/wave | M3.14-M4.25 runtime closure; M14.05 |
| Notes/blocker | Was `approved` on the basis of byte-for-byte identity with the source notice. That basis no longer holds: the destination is LF where the source is CRLF, so the recorded hash was stale. Per this ledger's own rule a hash change invalidates approval, so the entry returns to `needs-review`. A reviewer should confirm that line-ending normalization is acceptable for the companion notice of `ALI-0023` and, if so, re-approve against the hash now recorded. |

### Asset entry: `ALI-0025` — Ardel map metadata

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0025` |
| Source path | `rusted_kingdoms/data/maps/town_01_ardel.yaml` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/data/maps/town_01_ardel.yaml` |
| Source SHA-256 | `06e8333dad57fc6139d2cffab14c1214a2be619469f704d354a31898bf20c8d8` |
| Destination SHA-256 | `cfab7e92199ae1be40ffebf52d1ed102cca9a30ece93384fc7d5701921dd0f20` |
| Asset kind | scenario map YAML |
| Title/name | Ardel Village metadata |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file; creation/history includes `9a121085c79bd52c4a0da30cf7bd68567f3bc03a` and latest pinned change `1bcbdf4bb6bd2c7cee05b30c83240a13fbd722f0`; source README license boundary; inspected 2026-08-10 |
| License identifier/name | unknown; the source repository's MIT statement does not cover scenario content. |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | **Modified by this port after copying.** The destination SHA-256 above is the current committed file and is authoritative for what would ship; the source SHA-256 is retained so the derivation stays traceable. This entry previously asserted byte-for-byte identity, which later content work made false; corrected 2026-09-12. Being a derivative does not clear the original's rights, which remain unknown. Its `bgm: town.default` field is the production Ardel BGM selection. |
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
| Destination SHA-256 | `13ef5861ab5ed1298d5eed7cb6490fea883e1c3cf11dcdd99d13bc7e846da572` |
| Asset kind | scenario audio-index YAML |
| Title/name | Rusted Kingdoms BGM index |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file; creation commit `994eb9a63b7b6e41fc2c85fe3b07fe4578f52182` and latest pinned change `d4be1ace21b23f4f1df63f0f32b5693c95687647`; source README license boundary; inspected 2026-08-10 |
| License identifier/name | unknown; the source repository's MIT statement does not cover scenario content. |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | **Modified by this port after copying.** The destination SHA-256 above is the current committed file and is authoritative for what would ship; the source SHA-256 is retained so the derivation stays traceable. This entry previously asserted byte-for-byte identity, which later content work made false; corrected 2026-09-12. Being a derivative does not clear the original's rights, which remain unknown. It maps `town.default` to `bgm/Whiteveil_Streets.mp3`; its other entries intentionally remain unresolved until later map waves copy their tracks. |
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
| Destination path | `assets/scenarios/rusted_kingdoms/media/audio/bgm/Whiteveil_Streets.mp3` |
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
| Destination path | `assets/scenarios/rusted_kingdoms/media/audio/README-audio.md` |
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

### Asset entry: `ALI-0029` — `Storming_the_Citadel.mp3`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0029` |
| Source path | `rusted_kingdoms/assets/audio/bgm/Storming_the_Citadel.mp3` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/audio/bgm/Storming_the_Citadel.mp3` |
| Source SHA-256 | `d9e1b4d0147ecec36af88c8fda49fccad6c52a452d243af9b607872b7fed7761` |
| Destination SHA-256 | `d9e1b4d0147ecec36af88c8fda49fccad6c52a452d243af9b607872b7fed7761` |
| Asset kind | audio |
| Title/name | `Storming_the_Citadel.mp3` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.05-M5.06 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0030` — `CREDIT`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0030` |
| Source path | `rusted_kingdoms/assets/audio/sfx/CREDIT` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/audio/sfx/CREDIT` |
| Source SHA-256 | `69fa0da996ecd8f098f35b087f570a06e84d3723e5c5c5b80d1223544800dae0` |
| Destination SHA-256 | `69fa0da996ecd8f098f35b087f570a06e84d3723e5c5c5b80d1223544800dae0` |
| Asset kind | attribution notice |
| Title/name | `CREDIT` |
| Creator/rightsholder | not-applicable; this factual notice identifies the associated sound-effect creator. |
| Source/evidence | Exact pinned source notice; import commit `33261e5133f7b0d6614ed1e6b799ee7c7a1a926c`; inspected 2026-08-10. |
| License identifier/name | not-applicable; retained as provenance evidence. |
| License text/notice location | This destination file is the local creator notice. |
| Required attribution | Preserve while associated effects remain in the tree. |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | yes; preserve with the associated files |
| Commercial-use permission | yes; the factual notice is not independently exploited artwork |
| Derivative-work permission | not-applicable; preserve unmodified |
| Review status | `approved` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.26 |
| Notes/blocker | This notice supports, but does not resolve, the exact-file acquisition and redistribution blockers on the associated effects. |

### Asset entry: `ALI-0031` — `013_Confirm_03.mp3`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0031` |
| Source path | `rusted_kingdoms/assets/audio/sfx/ui_menu/013_Confirm_03.mp3` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/audio/sfx/ui_menu/013_Confirm_03.mp3` |
| Source SHA-256 | `81aabb9231ec1c3e2e2459f82bfc9878edfa7713890e6b4161e4ebba4d708c31` |
| Destination SHA-256 | `81aabb9231ec1c3e2e2459f82bfc9878edfa7713890e6b4161e4ebba4d708c31` |
| Asset kind | audio |
| Title/name | `013_Confirm_03.mp3` |
| Creator/rightsholder | Leohpaz is identified as creator by the pinned SFX credit; exact-file acquisition is not independently proven. |
| Source/evidence | Exact pinned source file and LFS object; import commit `33261e5133f7b0d6614ed1e6b799ee7c7a1a926c`; pinned SFX credit and candidate official pack terms recorded by `ALI-0003` and `ALI-0004`; inspected 2026-08-10. |
| License identifier/name | Stated itch.io pack terms; exact-file acquisition provenance is incomplete. |
| License text/notice location | `assets/scenarios/rusted_kingdoms/media/audio/sfx/CREDIT`; candidate store terms are recorded by `ALI-0003`. |
| Required attribution | Retain voluntary credit `Sound effect created by Leohpaz — https://leohpaz.itch.io` if later approved. |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.26 |
| Notes/blocker | **Release blocker.** Obtain the original package/download record for this exact hash and confirmation that embedding it in a distributable game complies with the pack terms; otherwise replace it. |

### Asset entry: `ALI-0032` — `029_Decline_09.mp3`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0032` |
| Source path | `rusted_kingdoms/assets/audio/sfx/ui_menu/029_Decline_09.mp3` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/audio/sfx/ui_menu/029_Decline_09.mp3` |
| Source SHA-256 | `2aec75f16066739d5784eb6e5cdbff881899b89e76b18085797978c706a1b9dc` |
| Destination SHA-256 | `2aec75f16066739d5784eb6e5cdbff881899b89e76b18085797978c706a1b9dc` |
| Asset kind | audio |
| Title/name | `029_Decline_09.mp3` |
| Creator/rightsholder | Leohpaz is identified as creator by the pinned SFX credit; exact-file acquisition is not independently proven. |
| Source/evidence | Exact pinned source file and LFS object; import commit `33261e5133f7b0d6614ed1e6b799ee7c7a1a926c`; pinned SFX credit and candidate official pack terms recorded by `ALI-0003` and `ALI-0004`; inspected 2026-08-10. |
| License identifier/name | Stated itch.io pack terms; exact-file acquisition provenance is incomplete. |
| License text/notice location | `assets/scenarios/rusted_kingdoms/media/audio/sfx/CREDIT`; candidate store terms are recorded by `ALI-0003`. |
| Required attribution | Retain voluntary credit `Sound effect created by Leohpaz — https://leohpaz.itch.io` if later approved. |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.26 |
| Notes/blocker | **Release blocker.** Obtain the original package/download record for this exact hash and confirmation that embedding it in a distributable game complies with the pack terms; otherwise replace it. |

### Asset entry: `ALI-0033` — `033_Denied_03.mp3`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0033` |
| Source path | `rusted_kingdoms/assets/audio/sfx/ui_menu/033_Denied_03.mp3` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/audio/sfx/ui_menu/033_Denied_03.mp3` |
| Source SHA-256 | `0e10087d9bacf942fdb66ee0cb3fec88401208e55e6f67a2e41c21724e21bf2d` |
| Destination SHA-256 | `0e10087d9bacf942fdb66ee0cb3fec88401208e55e6f67a2e41c21724e21bf2d` |
| Asset kind | audio |
| Title/name | `033_Denied_03.mp3` |
| Creator/rightsholder | Leohpaz is identified as creator by the pinned SFX credit; exact-file acquisition is not independently proven. |
| Source/evidence | Exact pinned source file and LFS object; import commit `33261e5133f7b0d6614ed1e6b799ee7c7a1a926c`; pinned SFX credit and candidate official pack terms recorded by `ALI-0003` and `ALI-0004`; inspected 2026-08-10. |
| License identifier/name | Stated itch.io pack terms; exact-file acquisition provenance is incomplete. |
| License text/notice location | `assets/scenarios/rusted_kingdoms/media/audio/sfx/CREDIT`; candidate store terms are recorded by `ALI-0003`. |
| Required attribution | Retain voluntary credit `Sound effect created by Leohpaz — https://leohpaz.itch.io` if later approved. |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.26 |
| Notes/blocker | **Release blocker.** Obtain the original package/download record for this exact hash and confirmation that embedding it in a distributable game complies with the pack terms; otherwise replace it. |

### Asset entry: `ALI-0034` — `051_use_item_01.mp3`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0034` |
| Source path | `rusted_kingdoms/assets/audio/sfx/ui_menu/051_use_item_01.mp3` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/audio/sfx/ui_menu/051_use_item_01.mp3` |
| Source SHA-256 | `647c787d41bcdd38a31fb40b557f221f95015fe2cf68fc4ec679184dd3c500fd` |
| Destination SHA-256 | `647c787d41bcdd38a31fb40b557f221f95015fe2cf68fc4ec679184dd3c500fd` |
| Asset kind | audio |
| Title/name | `051_use_item_01.mp3` |
| Creator/rightsholder | Leohpaz is identified as creator by the pinned SFX credit; exact-file acquisition is not independently proven. |
| Source/evidence | Exact pinned source file and LFS object; import commit `33261e5133f7b0d6614ed1e6b799ee7c7a1a926c`; pinned SFX credit and candidate official pack terms recorded by `ALI-0003` and `ALI-0004`; inspected 2026-08-10. |
| License identifier/name | Stated itch.io pack terms; exact-file acquisition provenance is incomplete. |
| License text/notice location | `assets/scenarios/rusted_kingdoms/media/audio/sfx/CREDIT`; candidate store terms are recorded by `ALI-0003`. |
| Required attribution | Retain voluntary credit `Sound effect created by Leohpaz — https://leohpaz.itch.io` if later approved. |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.26 |
| Notes/blocker | **Release blocker.** Obtain the original package/download record for this exact hash and confirmation that embedding it in a distributable game complies with the pack terms; otherwise replace it. |

### Asset entry: `ALI-0035` — `town_01_ardel_house_01.tmx`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0035` |
| Source path | `rusted_kingdoms/assets/maps/town_01_ardel_house_01.tmx` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/maps/town_01_ardel_house_01.tmx` |
| Source SHA-256 | `b1c7821f81c40e067dd5c59b394d12beaf7eb072d80076b2af033c2771ee28fd` |
| Destination SHA-256 | `b1c7821f81c40e067dd5c59b394d12beaf7eb072d80076b2af033c2771ee28fd` |
| Asset kind | Tiled map |
| Title/name | `town_01_ardel_house_01.tmx` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.01-M5.07 and M5.23-M5.25 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0036` — `zone_01_starting_forest.tmx`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0036` |
| Source path | `rusted_kingdoms/assets/maps/zone_01_starting_forest.tmx` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/maps/zone_01_starting_forest.tmx` |
| Source SHA-256 | `c842ea10c830fa17b638f0dbd09faaba911848d4bd4d3d6142a2541f9d746322` |
| Destination SHA-256 | `eff9b31c2da8e8a1703dc90a3acbe604d392643d1d58d120f193e78c3b340f12` |
| Asset kind | Tiled map |
| Title/name | `zone_01_starting_forest.tmx` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Modified locally on 2026-08-16: the gameplay-only `spawn_tile` layer is explicitly hidden so its five editor markers are not rendered in play. Tile data and encounter spawn semantics are unchanged. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.01-M5.07 and M5.23-M5.25 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0037` — `knight_01.png`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0037` |
| Source path | `rusted_kingdoms/assets/sprites/npc/knight_01.png` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/sprites/npc/knight_01.png` |
| Source SHA-256 | `6369914f762d144ee58b5335fee83216a12fc2ff8bc60df2551adf9b7ae647e6` |
| Destination SHA-256 | `6369914f762d144ee58b5335fee83216a12fc2ff8bc60df2551adf9b7ae647e6` |
| Asset kind | image |
| Title/name | `knight_01.png` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.08-M5.20 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0038` — `knight_01.tsx`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0038` |
| Source path | `rusted_kingdoms/assets/sprites/npc/knight_01.tsx` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/sprites/npc/knight_01.tsx` |
| Source SHA-256 | `7c74a85552fa791256c08dc44e0e8a5d232da5c88d05352080bad3e9d77da90c` |
| Destination SHA-256 | `7c74a85552fa791256c08dc44e0e8a5d232da5c88d05352080bad3e9d77da90c` |
| Asset kind | Tiled TSX metadata |
| Title/name | `knight_01.tsx` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.08-M5.20 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0039` — `male_old_02.png`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0039` |
| Source path | `rusted_kingdoms/assets/sprites/npc/male_old_02.png` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/sprites/npc/male_old_02.png` |
| Source SHA-256 | `d454f4a3e85f34a9b1cde126965bf433e0dba8a41423e5f48f4bc8920581fee0` |
| Destination SHA-256 | `d454f4a3e85f34a9b1cde126965bf433e0dba8a41423e5f48f4bc8920581fee0` |
| Asset kind | image |
| Title/name | `male_old_02.png` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.08-M5.20 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0040` — `male_old_02.tsx`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0040` |
| Source path | `rusted_kingdoms/assets/sprites/npc/male_old_02.tsx` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/sprites/npc/male_old_02.tsx` |
| Source SHA-256 | `4e2b4285ffced6b53d32e8b1050e24a963151f3664a8c574778359957a0ba61b` |
| Destination SHA-256 | `4e2b4285ffced6b53d32e8b1050e24a963151f3664a8c574778359957a0ba61b` |
| Asset kind | Tiled TSX metadata |
| Title/name | `male_old_02.tsx` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.08-M5.20 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0041` — `male_sword_fighter_axe_fighter.png`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0041` |
| Source path | `rusted_kingdoms/assets/sprites/npc/male_sword_fighter_axe_fighter.png` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/sprites/npc/male_sword_fighter_axe_fighter.png` |
| Source SHA-256 | `bede99be95ba589244f21539ed4ea1ec1e7353fe9e0bcc9c2214a79d09caec1d` |
| Destination SHA-256 | `bede99be95ba589244f21539ed4ea1ec1e7353fe9e0bcc9c2214a79d09caec1d` |
| Asset kind | image |
| Title/name | `male_sword_fighter_axe_fighter.png` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.08-M5.20 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0042` — `male_sword_fighter_axe_fighter.tsx`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0042` |
| Source path | `rusted_kingdoms/assets/sprites/npc/male_sword_fighter_axe_fighter.tsx` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/sprites/npc/male_sword_fighter_axe_fighter.tsx` |
| Source SHA-256 | `41d6594e437433d6e5df6c5f8303304b1d8cc8d06210d9b6909f4b9d63469312` |
| Destination SHA-256 | `41d6594e437433d6e5df6c5f8303304b1d8cc8d06210d9b6909f4b9d63469312` |
| Asset kind | Tiled TSX metadata |
| Title/name | `male_sword_fighter_axe_fighter.tsx` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.08-M5.20 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0043` — `teen_halfmessy_01.png`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0043` |
| Source path | `rusted_kingdoms/assets/sprites/npc/teen_halfmessy_01.png` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/sprites/npc/teen_halfmessy_01.png` |
| Source SHA-256 | `72afc55330c9ffc6a3e936035cfb87523eafc2461ab759d02873eff662f7d6ec` |
| Destination SHA-256 | `72afc55330c9ffc6a3e936035cfb87523eafc2461ab759d02873eff662f7d6ec` |
| Asset kind | image |
| Title/name | `teen_halfmessy_01.png` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.08-M5.20 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0044` — `teen_halfmessy_01.tsx`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0044` |
| Source path | `rusted_kingdoms/assets/sprites/npc/teen_halfmessy_01.tsx` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/sprites/npc/teen_halfmessy_01.tsx` |
| Source SHA-256 | `52116e9c6c5eec8d4d73c546517a36778ba966cb4854d96f1a828b2ccbdb80bd` |
| Destination SHA-256 | `52116e9c6c5eec8d4d73c546517a36778ba966cb4854d96f1a828b2ccbdb80bd` |
| Asset kind | Tiled TSX metadata |
| Title/name | `teen_halfmessy_01.tsx` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.08-M5.20 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0045` — `teen_wiz_01.png`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0045` |
| Source path | `rusted_kingdoms/assets/sprites/npc/teen_wiz_01.png` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/sprites/npc/teen_wiz_01.png` |
| Source SHA-256 | `5ad63590c3a76b022cf586c041d28c3099b1f7d842a6f6d314f74a75a79d32cd` |
| Destination SHA-256 | `5ad63590c3a76b022cf586c041d28c3099b1f7d842a6f6d314f74a75a79d32cd` |
| Asset kind | image |
| Title/name | `teen_wiz_01.png` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.08-M5.20 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0046` — `teen_wiz_01.tsx`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0046` |
| Source path | `rusted_kingdoms/assets/sprites/npc/teen_wiz_01.tsx` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/sprites/npc/teen_wiz_01.tsx` |
| Source SHA-256 | `4ba455e94a22b5d25a1539f3397cb16025d3b5925993c6a3e27a4c8fbbc84879` |
| Destination SHA-256 | `4ba455e94a22b5d25a1539f3397cb16025d3b5925993c6a3e27a4c8fbbc84879` |
| Asset kind | Tiled TSX metadata |
| Title/name | `teen_wiz_01.tsx` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.08-M5.20 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0047` — `village_female_person_basket_carrier.png`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0047` |
| Source path | `rusted_kingdoms/assets/sprites/npc/village_female_person_basket_carrier.png` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/sprites/npc/village_female_person_basket_carrier.png` |
| Source SHA-256 | `f73f271a7aec85a7550e42a26e7a4dd61099500f7c3032f21c73076dcb1167a9` |
| Destination SHA-256 | `f73f271a7aec85a7550e42a26e7a4dd61099500f7c3032f21c73076dcb1167a9` |
| Asset kind | image |
| Title/name | `village_female_person_basket_carrier.png` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.08-M5.20 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0048` — `village_female_person_basket_carrier.tsx`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0048` |
| Source path | `rusted_kingdoms/assets/sprites/npc/village_female_person_basket_carrier.tsx` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/sprites/npc/village_female_person_basket_carrier.tsx` |
| Source SHA-256 | `2718f2e22fbc223cd9de7a22ddf0672c31acf320fbabe80811a6aef978f4d3a3` |
| Destination SHA-256 | `2718f2e22fbc223cd9de7a22ddf0672c31acf320fbabe80811a6aef978f4d3a3` |
| Asset kind | Tiled TSX metadata |
| Title/name | `village_female_person_basket_carrier.tsx` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.08-M5.20 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0049` — `item_box.png`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0049` |
| Source path | `rusted_kingdoms/assets/sprites/objects/item_box.png` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/sprites/objects/item_box.png` |
| Source SHA-256 | `b2e0c5bb594fcf9ff8fe427922fe22aaf4b4ac85928a84be1d032040beb31ca1` |
| Destination SHA-256 | `b2e0c5bb594fcf9ff8fe427922fe22aaf4b4ac85928a84be1d032040beb31ca1` |
| Asset kind | image |
| Title/name | `item_box.png` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.23-M5.25 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0050` — `item_box.tsx`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0050` |
| Source path | `rusted_kingdoms/assets/sprites/objects/item_box.tsx` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/sprites/objects/item_box.tsx` |
| Source SHA-256 | `d038d160a110c8e1ceeb4a0909775bde713c4777dffffcd265a454dd43077ed5` |
| Destination SHA-256 | `d038d160a110c8e1ceeb4a0909775bde713c4777dffffcd265a454dd43077ed5` |
| Asset kind | Tiled TSX metadata |
| Title/name | `item_box.tsx` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.23-M5.25 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0051` — `02_elise_walk.png`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0051` |
| Source path | `rusted_kingdoms/assets/sprites/party/02_elise_walk.png` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/sprites/party/02_elise_walk.png` |
| Source SHA-256 | `d54dab0913dd5a31714aa0ffea31dd95c19cf65124f4be5263d69a3cacefef9c` |
| Destination SHA-256 | `d54dab0913dd5a31714aa0ffea31dd95c19cf65124f4be5263d69a3cacefef9c` |
| Asset kind | image |
| Title/name | `02_elise_walk.png` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.08-M5.20 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0052` — `02_elise_walk.tsx`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0052` |
| Source path | `rusted_kingdoms/assets/sprites/party/02_elise_walk.tsx` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/sprites/party/02_elise_walk.tsx` |
| Source SHA-256 | `c70c101ff81ac9bec876037f278704f830908787028b32e768dfc5a0afa0d0d4` |
| Destination SHA-256 | `c70c101ff81ac9bec876037f278704f830908787028b32e768dfc5a0afa0d0d4` |
| Asset kind | Tiled TSX metadata |
| Title/name | `02_elise_walk.tsx` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.08-M5.20 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0053` — `altro.png`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0053` |
| Source path | `rusted_kingdoms/assets/tilesets/astralpixels/altro.png` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/tilesets/astralpixels/altro.png` |
| Source SHA-256 | `3ea4e8cb3bcf46b18f174d4b790747e916006c566da5fc4dd146ea2a89fc887f` |
| Destination SHA-256 | `3ea4e8cb3bcf46b18f174d4b790747e916006c566da5fc4dd146ea2a89fc887f` |
| Asset kind | image |
| Title/name | `altro.png` |
| Creator/rightsholder | AstralPixels is identified as the artwork creator by the pinned credit; the source import author created the TSX metadata. |
| Source/evidence | Exact pinned source file; import commit `582e7487602ea629e8a62eb3fb4ad57a992854c2`; pinned AstralPixels credit and official page recorded by `ALI-0016` through `ALI-0018`; inspected 2026-08-10. |
| License identifier/name | Candidate official asset-page terms; exact-file acquisition provenance is incomplete. |
| License text/notice location | `assets/scenarios/rusted_kingdoms/media/tilesets/astralpixels/credit.txt` and the official page recorded by `ALI-0016`. |
| Required attribution | Preserve the pinned voluntary AstralPixels credit while provenance is unresolved. |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.05-M5.06 |
| Notes/blocker | **Release blocker.** Obtain the original package acquisition record, map it reproducibly to this exact extracted hash, and confirm game embedding complies with the no-repackaging term. |

### Asset entry: `ALI-0054` — `altro.tsx`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0054` |
| Source path | `rusted_kingdoms/assets/tilesets/astralpixels/altro.tsx` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/tilesets/astralpixels/altro.tsx` |
| Source SHA-256 | `55872b7a10c85d183e6ff1332e1791d68d148ed0867c14951adb6f69e035b23b` |
| Destination SHA-256 | `55872b7a10c85d183e6ff1332e1791d68d148ed0867c14951adb6f69e035b23b` |
| Asset kind | Tiled TSX metadata |
| Title/name | `altro.tsx` |
| Creator/rightsholder | AstralPixels is identified as the artwork creator by the pinned credit; the source import author created the TSX metadata. |
| Source/evidence | Exact pinned source file; import commit `582e7487602ea629e8a62eb3fb4ad57a992854c2`; pinned AstralPixels credit and official page recorded by `ALI-0016` through `ALI-0018`; inspected 2026-08-10. |
| License identifier/name | Candidate official asset-page terms; exact-file acquisition provenance is incomplete. |
| License text/notice location | `assets/scenarios/rusted_kingdoms/media/tilesets/astralpixels/credit.txt` and the official page recorded by `ALI-0016`. |
| Required attribution | Preserve the pinned voluntary AstralPixels credit while provenance is unresolved. |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.05-M5.06 |
| Notes/blocker | **Release blocker.** Obtain the original package acquisition record, map it reproducibly to this exact extracted hash, and confirm game embedding complies with the no-repackaging term. |

### Asset entry: `ALI-0055` — `cucina.png`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0055` |
| Source path | `rusted_kingdoms/assets/tilesets/astralpixels/cucina.png` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/tilesets/astralpixels/cucina.png` |
| Source SHA-256 | `46700310f1cd1c7f0c5ccf10cb1e5af89ae675f103480811427d710d82ded7f6` |
| Destination SHA-256 | `46700310f1cd1c7f0c5ccf10cb1e5af89ae675f103480811427d710d82ded7f6` |
| Asset kind | image |
| Title/name | `cucina.png` |
| Creator/rightsholder | AstralPixels is identified as the artwork creator by the pinned credit; the source import author created the TSX metadata. |
| Source/evidence | Exact pinned source file; import commit `582e7487602ea629e8a62eb3fb4ad57a992854c2`; pinned AstralPixels credit and official page recorded by `ALI-0016` through `ALI-0018`; inspected 2026-08-10. |
| License identifier/name | Candidate official asset-page terms; exact-file acquisition provenance is incomplete. |
| License text/notice location | `assets/scenarios/rusted_kingdoms/media/tilesets/astralpixels/credit.txt` and the official page recorded by `ALI-0016`. |
| Required attribution | Preserve the pinned voluntary AstralPixels credit while provenance is unresolved. |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.05-M5.06 |
| Notes/blocker | **Release blocker.** Obtain the original package acquisition record, map it reproducibly to this exact extracted hash, and confirm game embedding complies with the no-repackaging term. |

### Asset entry: `ALI-0056` — `cucina.tsx`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0056` |
| Source path | `rusted_kingdoms/assets/tilesets/astralpixels/cucina.tsx` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/tilesets/astralpixels/cucina.tsx` |
| Source SHA-256 | `1247bbf165a038b2ad45236674038d2496b1f6b6d5cb0917f56c323584de98ae` |
| Destination SHA-256 | `1247bbf165a038b2ad45236674038d2496b1f6b6d5cb0917f56c323584de98ae` |
| Asset kind | Tiled TSX metadata |
| Title/name | `cucina.tsx` |
| Creator/rightsholder | AstralPixels is identified as the artwork creator by the pinned credit; the source import author created the TSX metadata. |
| Source/evidence | Exact pinned source file; import commit `582e7487602ea629e8a62eb3fb4ad57a992854c2`; pinned AstralPixels credit and official page recorded by `ALI-0016` through `ALI-0018`; inspected 2026-08-10. |
| License identifier/name | Candidate official asset-page terms; exact-file acquisition provenance is incomplete. |
| License text/notice location | `assets/scenarios/rusted_kingdoms/media/tilesets/astralpixels/credit.txt` and the official page recorded by `ALI-0016`. |
| Required attribution | Preserve the pinned voluntary AstralPixels credit while provenance is unresolved. |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.05-M5.06 |
| Notes/blocker | **Release blocker.** Obtain the original package acquisition record, map it reproducibly to this exact extracted hash, and confirm game embedding complies with the no-repackaging term. |

### Asset entry: `ALI-0057` — `mensole.png`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0057` |
| Source path | `rusted_kingdoms/assets/tilesets/astralpixels/mensole.png` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/tilesets/astralpixels/mensole.png` |
| Source SHA-256 | `253d47b1489f59dfc7d309c50927944a99aa6233d1049249f7902a1f41c3757b` |
| Destination SHA-256 | `253d47b1489f59dfc7d309c50927944a99aa6233d1049249f7902a1f41c3757b` |
| Asset kind | image |
| Title/name | `mensole.png` |
| Creator/rightsholder | AstralPixels is identified as the artwork creator by the pinned credit; the source import author created the TSX metadata. |
| Source/evidence | Exact pinned source file; import commit `582e7487602ea629e8a62eb3fb4ad57a992854c2`; pinned AstralPixels credit and official page recorded by `ALI-0016` through `ALI-0018`; inspected 2026-08-10. |
| License identifier/name | Candidate official asset-page terms; exact-file acquisition provenance is incomplete. |
| License text/notice location | `assets/scenarios/rusted_kingdoms/media/tilesets/astralpixels/credit.txt` and the official page recorded by `ALI-0016`. |
| Required attribution | Preserve the pinned voluntary AstralPixels credit while provenance is unresolved. |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.05-M5.06 |
| Notes/blocker | **Release blocker.** Obtain the original package acquisition record, map it reproducibly to this exact extracted hash, and confirm game embedding complies with the no-repackaging term. |

### Asset entry: `ALI-0058` — `mensole.tsx`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0058` |
| Source path | `rusted_kingdoms/assets/tilesets/astralpixels/mensole.tsx` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/tilesets/astralpixels/mensole.tsx` |
| Source SHA-256 | `5cf8fe6900c738366a3dd818d789b18bac3cfaa1884b73a5f966866d2420abae` |
| Destination SHA-256 | `5cf8fe6900c738366a3dd818d789b18bac3cfaa1884b73a5f966866d2420abae` |
| Asset kind | Tiled TSX metadata |
| Title/name | `mensole.tsx` |
| Creator/rightsholder | AstralPixels is identified as the artwork creator by the pinned credit; the source import author created the TSX metadata. |
| Source/evidence | Exact pinned source file; import commit `582e7487602ea629e8a62eb3fb4ad57a992854c2`; pinned AstralPixels credit and official page recorded by `ALI-0016` through `ALI-0018`; inspected 2026-08-10. |
| License identifier/name | Candidate official asset-page terms; exact-file acquisition provenance is incomplete. |
| License text/notice location | `assets/scenarios/rusted_kingdoms/media/tilesets/astralpixels/credit.txt` and the official page recorded by `ALI-0016`. |
| Required attribution | Preserve the pinned voluntary AstralPixels credit while provenance is unresolved. |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.05-M5.06 |
| Notes/blocker | **Release blocker.** Obtain the original package acquisition record, map it reproducibly to this exact extracted hash, and confirm game embedding complies with the no-repackaging term. |

### Asset entry: `ALI-0059` — `mobili.png`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0059` |
| Source path | `rusted_kingdoms/assets/tilesets/astralpixels/mobili.png` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/tilesets/astralpixels/mobili.png` |
| Source SHA-256 | `467e550f0c66e24505dff542684547e14e41b771164167c5124c81acf297f59d` |
| Destination SHA-256 | `467e550f0c66e24505dff542684547e14e41b771164167c5124c81acf297f59d` |
| Asset kind | image |
| Title/name | `mobili.png` |
| Creator/rightsholder | AstralPixels is identified as the artwork creator by the pinned credit; the source import author created the TSX metadata. |
| Source/evidence | Exact pinned source file; import commit `582e7487602ea629e8a62eb3fb4ad57a992854c2`; pinned AstralPixels credit and official page recorded by `ALI-0016` through `ALI-0018`; inspected 2026-08-10. |
| License identifier/name | Candidate official asset-page terms; exact-file acquisition provenance is incomplete. |
| License text/notice location | `assets/scenarios/rusted_kingdoms/media/tilesets/astralpixels/credit.txt` and the official page recorded by `ALI-0016`. |
| Required attribution | Preserve the pinned voluntary AstralPixels credit while provenance is unresolved. |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.05-M5.06 |
| Notes/blocker | **Release blocker.** Obtain the original package acquisition record, map it reproducibly to this exact extracted hash, and confirm game embedding complies with the no-repackaging term. |

### Asset entry: `ALI-0060` — `mobili.tsx`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0060` |
| Source path | `rusted_kingdoms/assets/tilesets/astralpixels/mobili.tsx` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/tilesets/astralpixels/mobili.tsx` |
| Source SHA-256 | `2da6294c0b742cf87366205a76fe65cfeb0754d9abea938932614d963ebe1a5d` |
| Destination SHA-256 | `2da6294c0b742cf87366205a76fe65cfeb0754d9abea938932614d963ebe1a5d` |
| Asset kind | Tiled TSX metadata |
| Title/name | `mobili.tsx` |
| Creator/rightsholder | AstralPixels is identified as the artwork creator by the pinned credit; the source import author created the TSX metadata. |
| Source/evidence | Exact pinned source file; import commit `582e7487602ea629e8a62eb3fb4ad57a992854c2`; pinned AstralPixels credit and official page recorded by `ALI-0016` through `ALI-0018`; inspected 2026-08-10. |
| License identifier/name | Candidate official asset-page terms; exact-file acquisition provenance is incomplete. |
| License text/notice location | `assets/scenarios/rusted_kingdoms/media/tilesets/astralpixels/credit.txt` and the official page recorded by `ALI-0016`. |
| Required attribution | Preserve the pinned voluntary AstralPixels credit while provenance is unresolved. |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.05-M5.06 |
| Notes/blocker | **Release blocker.** Obtain the original package acquisition record, map it reproducibly to this exact extracted hash, and confirm game embedding complies with the no-repackaging term. |

### Asset entry: `ALI-0061` — `muro_tileset.png`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0061` |
| Source path | `rusted_kingdoms/assets/tilesets/astralpixels/muro_tileset.png` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/tilesets/astralpixels/muro_tileset.png` |
| Source SHA-256 | `772acd218e50f724959ef15d850971d6ba417a9a29c45a244ed2fd5d43465dea` |
| Destination SHA-256 | `772acd218e50f724959ef15d850971d6ba417a9a29c45a244ed2fd5d43465dea` |
| Asset kind | image |
| Title/name | `muro_tileset.png` |
| Creator/rightsholder | AstralPixels is identified as the artwork creator by the pinned credit; the source import author created the TSX metadata. |
| Source/evidence | Exact pinned source file; import commit `582e7487602ea629e8a62eb3fb4ad57a992854c2`; pinned AstralPixels credit and official page recorded by `ALI-0016` through `ALI-0018`; inspected 2026-08-10. |
| License identifier/name | Candidate official asset-page terms; exact-file acquisition provenance is incomplete. |
| License text/notice location | `assets/scenarios/rusted_kingdoms/media/tilesets/astralpixels/credit.txt` and the official page recorded by `ALI-0016`. |
| Required attribution | Preserve the pinned voluntary AstralPixels credit while provenance is unresolved. |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.05-M5.06 |
| Notes/blocker | **Release blocker.** Obtain the original package acquisition record, map it reproducibly to this exact extracted hash, and confirm game embedding complies with the no-repackaging term. |

### Asset entry: `ALI-0062` — `muro_tileset_wall.tsx`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0062` |
| Source path | `rusted_kingdoms/assets/tilesets/astralpixels/muro_tileset_wall.tsx` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/tilesets/astralpixels/muro_tileset_wall.tsx` |
| Source SHA-256 | `904cfb5af6321f9721f292d359d369b5f871257d7c07c11a0e9a3ad4a0f97839` |
| Destination SHA-256 | `904cfb5af6321f9721f292d359d369b5f871257d7c07c11a0e9a3ad4a0f97839` |
| Asset kind | Tiled TSX metadata |
| Title/name | `muro_tileset_wall.tsx` |
| Creator/rightsholder | AstralPixels is identified as the artwork creator by the pinned credit; the source import author created the TSX metadata. |
| Source/evidence | Exact pinned source file; import commit `582e7487602ea629e8a62eb3fb4ad57a992854c2`; pinned AstralPixels credit and official page recorded by `ALI-0016` through `ALI-0018`; inspected 2026-08-10. |
| License identifier/name | Candidate official asset-page terms; exact-file acquisition provenance is incomplete. |
| License text/notice location | `assets/scenarios/rusted_kingdoms/media/tilesets/astralpixels/credit.txt` and the official page recorded by `ALI-0016`. |
| Required attribution | Preserve the pinned voluntary AstralPixels credit while provenance is unresolved. |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.05-M5.06 |
| Notes/blocker | **Release blocker.** Obtain the original package acquisition record, map it reproducibly to this exact extracted hash, and confirm game embedding complies with the no-repackaging term. |

### Asset entry: `ALI-0063` — `scale.png`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0063` |
| Source path | `rusted_kingdoms/assets/tilesets/astralpixels/scale.png` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/tilesets/astralpixels/scale.png` |
| Source SHA-256 | `de88f8ed9297a3592ad8f5197e4fefdb21a2e3150a4e699fd0f9b371b612f151` |
| Destination SHA-256 | `de88f8ed9297a3592ad8f5197e4fefdb21a2e3150a4e699fd0f9b371b612f151` |
| Asset kind | image |
| Title/name | `scale.png` |
| Creator/rightsholder | AstralPixels is identified as the artwork creator by the pinned credit; the source import author created the TSX metadata. |
| Source/evidence | Exact pinned source file; import commit `582e7487602ea629e8a62eb3fb4ad57a992854c2`; pinned AstralPixels credit and official page recorded by `ALI-0016` through `ALI-0018`; inspected 2026-08-10. |
| License identifier/name | Candidate official asset-page terms; exact-file acquisition provenance is incomplete. |
| License text/notice location | `assets/scenarios/rusted_kingdoms/media/tilesets/astralpixels/credit.txt` and the official page recorded by `ALI-0016`. |
| Required attribution | Preserve the pinned voluntary AstralPixels credit while provenance is unresolved. |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.05-M5.06 |
| Notes/blocker | **Release blocker.** Obtain the original package acquisition record, map it reproducibly to this exact extracted hash, and confirm game embedding complies with the no-repackaging term. |

### Asset entry: `ALI-0064` — `scale.tsx`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0064` |
| Source path | `rusted_kingdoms/assets/tilesets/astralpixels/scale.tsx` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/tilesets/astralpixels/scale.tsx` |
| Source SHA-256 | `6af8fb4d85efb9cbf620bf9e5e22237c13198f51302cc1453887f965b8d91539` |
| Destination SHA-256 | `6af8fb4d85efb9cbf620bf9e5e22237c13198f51302cc1453887f965b8d91539` |
| Asset kind | Tiled TSX metadata |
| Title/name | `scale.tsx` |
| Creator/rightsholder | AstralPixels is identified as the artwork creator by the pinned credit; the source import author created the TSX metadata. |
| Source/evidence | Exact pinned source file; import commit `582e7487602ea629e8a62eb3fb4ad57a992854c2`; pinned AstralPixels credit and official page recorded by `ALI-0016` through `ALI-0018`; inspected 2026-08-10. |
| License identifier/name | Candidate official asset-page terms; exact-file acquisition provenance is incomplete. |
| License text/notice location | `assets/scenarios/rusted_kingdoms/media/tilesets/astralpixels/credit.txt` and the official page recorded by `ALI-0016`. |
| Required attribution | Preserve the pinned voluntary AstralPixels credit while provenance is unresolved. |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.05-M5.06 |
| Notes/blocker | **Release blocker.** Obtain the original package acquisition record, map it reproducibly to this exact extracted hash, and confirm game embedding complies with the no-repackaging term. |

### Asset entry: `ALI-0065` — `terreno.png`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0065` |
| Source path | `rusted_kingdoms/assets/tilesets/astralpixels/terreno.png` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/tilesets/astralpixels/terreno.png` |
| Source SHA-256 | `880e9099e0e3d2d433c6079db99747f32b7ac555120e953f325fb0814f0dde6e` |
| Destination SHA-256 | `880e9099e0e3d2d433c6079db99747f32b7ac555120e953f325fb0814f0dde6e` |
| Asset kind | image |
| Title/name | `terreno.png` |
| Creator/rightsholder | AstralPixels is identified as the artwork creator by the pinned credit; the source import author created the TSX metadata. |
| Source/evidence | Exact pinned source file; import commit `582e7487602ea629e8a62eb3fb4ad57a992854c2`; pinned AstralPixels credit and official page recorded by `ALI-0016` through `ALI-0018`; inspected 2026-08-10. |
| License identifier/name | Candidate official asset-page terms; exact-file acquisition provenance is incomplete. |
| License text/notice location | `assets/scenarios/rusted_kingdoms/media/tilesets/astralpixels/credit.txt` and the official page recorded by `ALI-0016`. |
| Required attribution | Preserve the pinned voluntary AstralPixels credit while provenance is unresolved. |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.05-M5.06 |
| Notes/blocker | **Release blocker.** Obtain the original package acquisition record, map it reproducibly to this exact extracted hash, and confirm game embedding complies with the no-repackaging term. |

### Asset entry: `ALI-0066` — `terreno.tsx`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0066` |
| Source path | `rusted_kingdoms/assets/tilesets/astralpixels/terreno.tsx` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/tilesets/astralpixels/terreno.tsx` |
| Source SHA-256 | `9e93464720694cd682e9bbf3d2c9aad4c6521b01b73673e613e8e92b5008d0e2` |
| Destination SHA-256 | `9e93464720694cd682e9bbf3d2c9aad4c6521b01b73673e613e8e92b5008d0e2` |
| Asset kind | Tiled TSX metadata |
| Title/name | `terreno.tsx` |
| Creator/rightsholder | AstralPixels is identified as the artwork creator by the pinned credit; the source import author created the TSX metadata. |
| Source/evidence | Exact pinned source file; import commit `582e7487602ea629e8a62eb3fb4ad57a992854c2`; pinned AstralPixels credit and official page recorded by `ALI-0016` through `ALI-0018`; inspected 2026-08-10. |
| License identifier/name | Candidate official asset-page terms; exact-file acquisition provenance is incomplete. |
| License text/notice location | `assets/scenarios/rusted_kingdoms/media/tilesets/astralpixels/credit.txt` and the official page recorded by `ALI-0016`. |
| Required attribution | Preserve the pinned voluntary AstralPixels credit while provenance is unresolved. |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.05-M5.06 |
| Notes/blocker | **Release blocker.** Obtain the original package acquisition record, map it reproducibly to this exact extracted hash, and confirm game embedding complies with the no-repackaging term. |

### Asset entry: `ALI-0067` — `stone_tile_stares_16x16.png`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0067` |
| Source path | `rusted_kingdoms/assets/tilesets/stone_tile_stares_16x16.png` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/tilesets/stone_tile_stares_16x16.png` |
| Source SHA-256 | `8a83562ad82134ebf566fe7b8ef1f41b3bbc16d29da3a536acb46a20c6125333` |
| Destination SHA-256 | `8a83562ad82134ebf566fe7b8ef1f41b3bbc16d29da3a536acb46a20c6125333` |
| Asset kind | image |
| Title/name | `stone_tile_stares_16x16.png` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.21-M5.22 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0068` — `stone_tile_stares_16x16.tsx`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0068` |
| Source path | `rusted_kingdoms/assets/tilesets/stone_tile_stares_16x16.tsx` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/media/tilesets/stone_tile_stares_16x16.tsx` |
| Source SHA-256 | `3fcf618edc5c55180fa42ed6f9c648682bfb35f7736b915ca5efe896f627bc21` |
| Destination SHA-256 | `3fcf618edc5c55180fa42ed6f9c648682bfb35f7736b915ca5efe896f627bc21` |
| Asset kind | Tiled TSX metadata |
| Title/name | `stone_tile_stares_16x16.tsx` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.21-M5.22 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0069` — `sfx_index.yaml`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0069` |
| Source path | `rusted_kingdoms/data/audio/sfx_index.yaml` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/data/audio/sfx_index.yaml` |
| Source SHA-256 | `c85d9b352c7c2e10c8b0c72a903336fca501ab6dc8a1d43601d97e6679183b66` |
| Destination SHA-256 | `4eec9fa7a1cbab42d6911d5d2e419fafeb381ec50dfb9e6789e97d783b8349d5` |
| Asset kind | scenario YAML |
| Title/name | `sfx_index.yaml` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | **Modified by this port after copying.** The destination SHA-256 above is the current committed file and is authoritative for what would ship; the source SHA-256 is retained so the derivation stays traceable. This entry previously asserted byte-for-byte identity, which later content work made false; corrected 2026-09-12. Being a derivative does not clear the original's rights, which remain unknown. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.26 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0070` — `ardel_apprentice.yaml`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0070` |
| Source path | `rusted_kingdoms/data/dialogue/ardel_apprentice.yaml` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/data/dialogue/ardel_apprentice.yaml` |
| Source SHA-256 | `3773c67f880fb3ec1d6f59bfea81ff0035553e419dc9ea134c38bc6609dd81f5` |
| Destination SHA-256 | `3773c67f880fb3ec1d6f59bfea81ff0035553e419dc9ea134c38bc6609dd81f5` |
| Asset kind | scenario YAML |
| Title/name | `ardel_apprentice.yaml` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.12-M5.18 and M5.22 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0071` — `ardel_child.yaml`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0071` |
| Source path | `rusted_kingdoms/data/dialogue/ardel_child.yaml` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/data/dialogue/ardel_child.yaml` |
| Source SHA-256 | `b0d884d8e68d85c4e3f336a254790b1e8d3db264794c3d0408c870dc701ebc2e` |
| Destination SHA-256 | `b0d884d8e68d85c4e3f336a254790b1e8d3db264794c3d0408c870dc701ebc2e` |
| Asset kind | scenario YAML |
| Title/name | `ardel_child.yaml` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.12-M5.18 and M5.22 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0072` — `ardel_fisherman.yaml`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0072` |
| Source path | `rusted_kingdoms/data/dialogue/ardel_fisherman.yaml` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/data/dialogue/ardel_fisherman.yaml` |
| Source SHA-256 | `6b2318a67c5d0e52c5d8c6567289b2dcee8d082a781b939ab29073f9a23fdd0a` |
| Destination SHA-256 | `6b2318a67c5d0e52c5d8c6567289b2dcee8d082a781b939ab29073f9a23fdd0a` |
| Asset kind | scenario YAML |
| Title/name | `ardel_fisherman.yaml` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.12-M5.18 and M5.22 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0073` — `ardel_smith.yaml`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0073` |
| Source path | `rusted_kingdoms/data/dialogue/ardel_smith.yaml` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/data/dialogue/ardel_smith.yaml` |
| Source SHA-256 | `de600ee9a884189d65b24ac0791f5777db8070855e3ed64edcf327cd2532e389` |
| Destination SHA-256 | `de600ee9a884189d65b24ac0791f5777db8070855e3ed64edcf327cd2532e389` |
| Asset kind | scenario YAML |
| Title/name | `ardel_smith.yaml` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.12-M5.18 and M5.22 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0074` — `bridge_guard_zone5.yaml`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0074` |
| Source path | `rusted_kingdoms/data/dialogue/bridge_guard_zone5.yaml` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/data/dialogue/bridge_guard_zone5.yaml` |
| Source SHA-256 | `66ffc539b1f59a7fa5420fef63fe32b18354431387fcf57aecafde8a8e5bf329` |
| Destination SHA-256 | `f1bce4a36f1cfac248c6d704ef3c7155225ff757c9560cdb2297fa5b16d53f38` |
| Asset kind | scenario YAML |
| Title/name | `bridge_guard_zone5.yaml` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | **Modified by this port after copying.** The destination SHA-256 above is the current committed file and is authoritative for what would ship; the source SHA-256 is retained so the derivation stays traceable. This entry previously asserted byte-for-byte identity, which later content work made false; corrected 2026-09-12. Being a derivative does not clear the original's rights, which remain unknown. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.12-M5.18 and M5.22 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0075` — `elder_intro.yaml`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0075` |
| Source path | `rusted_kingdoms/data/dialogue/elder_intro.yaml` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/data/dialogue/elder_intro.yaml` |
| Source SHA-256 | `1f6e9f269495dd31fd422f8a753154a257bc87b9ff368fd47c436bb2fbc4dffb` |
| Destination SHA-256 | `1f6e9f269495dd31fd422f8a753154a257bc87b9ff368fd47c436bb2fbc4dffb` |
| Asset kind | scenario YAML |
| Title/name | `elder_intro.yaml` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.12-M5.18 and M5.22 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0076` — `elise_join.yaml`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0076` |
| Source path | `rusted_kingdoms/data/dialogue/elise_join.yaml` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/data/dialogue/elise_join.yaml` |
| Source SHA-256 | `6462df82dc276b9342835663dcb50773715cc2cdbbc0b31351d064f6f542b5fa` |
| Destination SHA-256 | `6462df82dc276b9342835663dcb50773715cc2cdbbc0b31351d064f6f542b5fa` |
| Asset kind | scenario YAML |
| Title/name | `elise_join.yaml` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.12-M5.18 and M5.22 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0077` — `guide_ardel.yaml`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0077` |
| Source path | `rusted_kingdoms/data/dialogue/guide_ardel.yaml` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/data/dialogue/guide_ardel.yaml` |
| Source SHA-256 | `1930ab11d1c83f48e44ec8957ad2facdc0f8ac6501280c0c88c6782bd0b15362` |
| Destination SHA-256 | `1930ab11d1c83f48e44ec8957ad2facdc0f8ac6501280c0c88c6782bd0b15362` |
| Asset kind | scenario YAML |
| Title/name | `guide_ardel.yaml` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.12-M5.18 and M5.22 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0078` — `guide_excuses.yaml`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0078` |
| Source path | `rusted_kingdoms/data/dialogue/guide_excuses.yaml` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/data/dialogue/guide_excuses.yaml` |
| Source SHA-256 | `d7193882dc326cf0a88f28e3db3580b3db4ca3ce08cd24d199fc8ed59affa2b6` |
| Destination SHA-256 | `d7193882dc326cf0a88f28e3db3580b3db4ca3ce08cd24d199fc8ed59affa2b6` |
| Asset kind | scenario YAML |
| Title/name | `guide_excuses.yaml` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.12-M5.18 and M5.22 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0079` — `sign_town_01_ardel.yaml`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0079` |
| Source path | `rusted_kingdoms/data/dialogue/sign_town_01_ardel.yaml` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/data/dialogue/sign_town_01_ardel.yaml` |
| Source SHA-256 | `1eb180ad4b07a8583db57ef6973f8d4b5179dca48f2367c795c028097283e233` |
| Destination SHA-256 | `1eb180ad4b07a8583db57ef6973f8d4b5179dca48f2367c795c028097283e233` |
| Asset kind | scenario YAML |
| Title/name | `sign_town_01_ardel.yaml` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.12-M5.18 and M5.22 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0080` — `sign_zone_01_starting_forest.yaml`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0080` |
| Source path | `rusted_kingdoms/data/dialogue/sign_zone_01_starting_forest.yaml` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/data/dialogue/sign_zone_01_starting_forest.yaml` |
| Source SHA-256 | `8a844f6e7065b48f868f4cb305ef3b8076bf465c690783e80eea96c206964b52` |
| Destination SHA-256 | `8a844f6e7065b48f868f4cb305ef3b8076bf465c690783e80eea96c206964b52` |
| Asset kind | scenario YAML |
| Title/name | `sign_zone_01_starting_forest.yaml` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | Unmodified local parity inclusion: source and destination are byte-for-byte identical. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.12-M5.18 and M5.22 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0081` — `stronghold_gate_guard.yaml`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0081` |
| Source path | `rusted_kingdoms/data/dialogue/stronghold_gate_guard.yaml` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/data/dialogue/stronghold_gate_guard.yaml` |
| Source SHA-256 | `97896d523e701884dc387aa945359fe1957f7e0302476ff5ea8d61c7054ef999` |
| Destination SHA-256 | `7e7a281130adff33fdec771bc1bddb9072dfed78cee4fa7b1d5f8e3ad76e5dce` |
| Asset kind | scenario YAML |
| Title/name | `stronghold_gate_guard.yaml` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | **Modified by this port after copying.** The destination SHA-256 above is the current committed file and is authoritative for what would ship; the source SHA-256 is retained so the derivation stays traceable. This entry previously asserted byte-for-byte identity, which later content work made false; corrected 2026-09-12. Being a derivative does not clear the original's rights, which remain unknown. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.12-M5.18 and M5.22 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0082` — `town_01_ardel_house_01.yaml`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0082` |
| Source path | `rusted_kingdoms/data/maps/town_01_ardel_house_01.yaml` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/data/maps/town_01_ardel_house_01.yaml` |
| Source SHA-256 | `d30a15189a9b0236fc6defbf994aa738175f082bbc5ea51d15ad8d5971f02ead` |
| Destination SHA-256 | `e32977e1a03ea2229f510c43b1995ab5c2854613ce770a24b14d45058bd09579` |
| Asset kind | scenario YAML |
| Title/name | `town_01_ardel_house_01.yaml` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | **Modified by this port after copying.** The destination SHA-256 above is the current committed file and is authoritative for what would ship; the source SHA-256 is retained so the derivation stays traceable. This entry previously asserted byte-for-byte identity, which later content work made false; corrected 2026-09-12. Being a derivative does not clear the original's rights, which remain unknown. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.01-M5.07 and M5.23-M5.25 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |

### Asset entry: `ALI-0083` — `zone_01_starting_forest.yaml`

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0083` |
| Source path | `rusted_kingdoms/data/maps/zone_01_starting_forest.yaml` in `../agentic-rpg` at `0897035` |
| Destination path | `assets/scenarios/rusted_kingdoms/data/maps/zone_01_starting_forest.yaml` |
| Source SHA-256 | `b1e34f9ba943023b668681723698119117c33e596c1d40155270ed074db64dbe` |
| Destination SHA-256 | `7460cdb28109ae4819b29be544a9f5773ad7beb99690153853bd565ffa63f5e6` |
| Asset kind | scenario YAML |
| Title/name | `zone_01_starting_forest.yaml` |
| Creator/rightsholder | unknown; source commit authorship does not establish complete ownership or a redistribution grant. |
| Source/evidence | Exact pinned source file and history through revision `08970359d6cb03586948625d29b0d3351dbbf785`; source README license boundary; inspected 2026-08-10. |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | **Modified by this port after copying.** The destination SHA-256 above is the current committed file and is authoritative for what would ship; the source SHA-256 is retained so the derivation stays traceable. This entry previously asserted byte-for-byte identity, which later content work made false; corrected 2026-09-12. Being a derivative does not clear the original's rights, which remain unknown. |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-08-10 |
| Related port task/wave | M5.01-M5.07 and M5.23-M5.25 |
| Notes/blocker | **Release blocker.** Establish exact creator/rightsholder identity, provenance, license, attribution, and redistribution permission for this hash before public release; otherwise replace it. |


## M6 class, item, and field-menu backdrop entries

The following byte-identical files were copied from the pinned source revision
for M6. Each row is a distinct ledger entry. Shared review fields for every row:

- creator/rightsholder: unknown; commit authorship does not establish ownership;
- source/evidence: exact pinned source file and history through `08970359d6cb03586948625d29b0d3351dbbf785`;
- license, notice, and required attribution: unknown;
- modification: unmodified, with identical source and destination SHA-256;
- redistribution, commercial use, and derivatives: unknown;
- review: `needs-evidence`, Codex evidence audit, 2026-08-12;
- related work: M6.05-M6.24 / Gate 6; and
- blocker: **Release blocker.** Establish exact ownership, provenance, license,
  attribution, and redistribution permission for each hash before public
  release; otherwise replace it.

| ID | Source path in `../agentic-rpg` | Destination path | SHA-256 | Kind/name |
| --- | --- | --- | --- | --- |
| ALI-0084 | `rusted_kingdoms/data/classes/cleric.yaml` | `assets/scenarios/rusted_kingdoms/data/classes/cleric.yaml` | `8168a37e564be1a6151956b2b49b3733dd91532dbe27588eba518f919a06a650` | scenario YAML / cleric class |
| ALI-0086 | `rusted_kingdoms/data/classes/rogue.yaml` | `assets/scenarios/rusted_kingdoms/data/classes/rogue.yaml` | `1a2d96bf1cb417414d3896b7e21e96421ed6d6f263c0dc04c5806cdfbe18ae4a` | scenario YAML / rogue class |
| ALI-0087 | `rusted_kingdoms/data/classes/sorcerer.yaml` | `assets/scenarios/rusted_kingdoms/data/classes/sorcerer.yaml` | `c7774723c8f3ac11719ef7c5c57269b330552e05486bcf897fa1b1b855f5f723` | scenario YAML / sorcerer class |
| ALI-0088 | `rusted_kingdoms/data/classes/warrior.yaml` | `assets/scenarios/rusted_kingdoms/data/classes/warrior.yaml` | `0d6cca291dd59ec6bef33f645a5e43a643bf266e4c1337883fdb13291a905b33` | scenario YAML / warrior class |
| ALI-0091 | `rusted_kingdoms/data/items/consumables_battle_throw.yaml` | `assets/scenarios/rusted_kingdoms/data/items/consumables_battle_throw.yaml` | `dbc9bac385109f6e5869597e2b70f967a52932681b86e32aaacae506a788131f` | scenario YAML / battle consumables |
| ALI-0092 | `rusted_kingdoms/data/items/consumables_field.yaml` | `assets/scenarios/rusted_kingdoms/data/items/consumables_field.yaml` | `c559775d03ce8139e0747b6505e82d531b165a3ddc23c3bc188c04d3cfa94461` | scenario YAML / field consumables |
| ALI-0094 | `rusted_kingdoms/data/items/consumables_status_cure.yaml` | `assets/scenarios/rusted_kingdoms/data/items/consumables_status_cure.yaml` | `047bc51c0ef6314f46c6c29796df3b79cb87af858dbcb75b2b1d3a4c0b408a4d` | scenario YAML / status consumables |
| ALI-0097 | `rusted_kingdoms/data/items/key_items.yaml` | `assets/scenarios/rusted_kingdoms/data/items/key_items.yaml` | `cf0b9e790c0ff077f0b2ff2b643fdecd7885f1c07751de96173af80344b04cdb` | scenario YAML / key items |
| ALI-0098 | `rusted_kingdoms/data/items/magic_cores.yaml` | `assets/scenarios/rusted_kingdoms/data/items/magic_cores.yaml` | `3d6eee8482b1a2f51e2004b468733dfa1260d258e9bcd6fb85279c9a973fa409` | scenario YAML / magic cores |
| ALI-0099 | `rusted_kingdoms/data/items/materials.yaml` | `assets/scenarios/rusted_kingdoms/data/items/materials.yaml` | `f2075beb9f90e328aa8665023c471ac09c1f063cb96da5e077cec4dc813a0423` | scenario YAML / materials |
| ALI-0102 | `rusted_kingdoms/assets/images/battle_bg/zone4-sanctum-bg-1280x468.webp` | `assets/scenarios/rusted_kingdoms/media/images/battle_bg/zone4-sanctum-bg-1280x468.webp` | `9eee6b2ff027e44069abe52de82d6195a3e56d8ee0c2baa373be0a18b5c38f3c` | image / field-menu backdrop |

Target-authored Milestone 12 compatibility data (not copied third-party
content):

| ID | Source path | Destination path | SHA-256 | Kind/name |
| --- | --- | --- | --- | --- |
| M12-0001 | N/A - project-authored repair for dangling pinned enemy-drop ids | `assets/scenarios/rusted_kingdoms/data/items/migration_zone1_drops.yaml` | `c239b2db128f9a14068e456267343dc0809159b43c578b337d08702109160c01` | scenario YAML / Zone 1 drop metadata |

## M8 encounter, enemy, and battle-presentation entries

The following byte-identical files were copied from the pinned source revision
for M8. Each row is a distinct ledger entry. Shared review fields for every row
unless a more specific fact is recorded below:

- creator/rightsholder: unknown; commit authorship does not establish ownership;
- source/evidence: exact pinned source file and history through
  `08970359d6cb03586948625d29b0d3351dbbf785`;
- license, notice, and required attribution: unknown;
- modification: unmodified, with identical source and destination SHA-256;
- redistribution, commercial use, and derivatives: unknown;
- review: `needs-evidence`, Codex evidence audit, 2026-08-15;
- related work: M8.01-M8.12 / Gate 8; and
- blocker: **Release blocker.** Establish exact ownership, provenance, license,
  attribution, and redistribution permission for each hash before public
  release; otherwise replace it.

| ID | Source path in `../agentic-rpg` | Destination path | SHA-256 | Kind/name |
| --- | --- | --- | --- | --- |
| ALI-0103 | `rusted_kingdoms/data/encount/zone_01_starting_forest.yaml` | `assets/scenarios/rusted_kingdoms/data/encount/zone_01_starting_forest.yaml` | `aab41973505ce465ee920edf250f3e9da9c5c14d7cbf52ff4580d0b3a672c4f5` | scenario YAML / Starting Forest encounter zone |
| ALI-0111 | `rusted_kingdoms/data/enemies/enemies_rank_8_F.yaml` | `assets/scenarios/rusted_kingdoms/data/enemies/enemies_rank_8_F.yaml` | `09d277f1d69e1cde6dfc27d693bf729fccc063087b2c902f588e42be4b65c483` | scenario YAML / rank F enemies |
| ALI-0112 | `rusted_kingdoms/data/battle_backgrounds.yaml` | `assets/scenarios/rusted_kingdoms/data/battle_backgrounds.yaml` | `483aa5150c74fc5c3acc060b4a7622d012ad1018e6407c346bad186e564ffdee` | scenario YAML / battle-background catalog |
| ALI-0113 | `rusted_kingdoms/assets/images/battle_bg/zone1-bg-1280x468.webp` | `assets/scenarios/rusted_kingdoms/media/images/battle_bg/zone1-bg-1280x468.webp` | `b388cb588279b1e6bcdcafd18b850dc8f9baefc383c84dc5ea00a905c35bc80e` | image / zone-one battle background |
| ALI-0114 | `rusted_kingdoms/assets/audio/bgm/Pixelated_Crusade.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/bgm/Pixelated_Crusade.mp3` | `6a6c1d757c052e0b23720b5b7f0ca7255265ab74c3d216fee93d5e7f0ef109e4` | audio / normal battle BGM |
| ALI-0115 | `rusted_kingdoms/assets/audio/bgm/Crimson_Storm_s_Echo.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/bgm/Crimson_Storm_s_Echo.mp3` | `ecccf83691b31031aaa47dc6ad9ed505d1cf382d0a38caf81824ac14e73d2fd6` | audio / boss battle BGM |
| ALI-0116 | `rusted_kingdoms/assets/audio/sfx/battle/55_Encounter_02.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/battle/55_Encounter_02.mp3` | `8d6f7e5b52ba81b82e08d10c97fa24e58fa445fef79e24ad88eaecdd619625bd` | audio / encounter SFX |
| ALI-0117 | `rusted_kingdoms/assets/sprites/enemies/goblin.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin.png` | `13a7183f5d5372d5f2ed7db2b2d511feee68af070dd9c6178c964511c21c6f14` | image / Goblin world sprite |
| ALI-0118 | `rusted_kingdoms/assets/sprites/enemies/goblin.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin.tsx` | `7fde2ca3abc890679e901089afa359ded6ca2bcae30135a8247b04c53a1f8303` | Tiled TSX / Goblin world sprite |
| ALI-0119 | `rusted_kingdoms/assets/sprites/enemies/goblin_scout_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_scout_base.png` | `590b5b2d6890087e092ad29616f6917ff07773bdca56e4fa8045bf265cd0f40b` | image / Goblin Scout world sprite |
| ALI-0120 | `rusted_kingdoms/assets/sprites/enemies/goblin_scout_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_scout_base.tsx` | `c291515bc5d0a4e104a048375d040cbf3ee2a38967ff73aa5c38e4c82fdd5039` | Tiled TSX / Goblin Scout world sprite |
| ALI-0121 | `rusted_kingdoms/assets/sprites/enemies/goblin_scout_hooded_goblin.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_scout_hooded_goblin.png` | `8f3fa3e878c35a23728122621d83d0941338ff35d75442296454036272fc5953` | image / Hooded Goblin world sprite |
| ALI-0122 | `rusted_kingdoms/assets/sprites/enemies/goblin_scout_hooded_goblin.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_scout_hooded_goblin.tsx` | `29e14c2d53ff71d0576ac475d923414c1cf15db7646610da481d2f2caa953649` | Tiled TSX / Hooded Goblin world sprite |
| ALI-0123 | `rusted_kingdoms/assets/sprites/enemies/goblin_scout_sling_scout.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_scout_sling_scout.png` | `c7010905c160222b0ad571528a50095c8f0b0e5d762fdf29a175c4dbb228293c` | image / Sling Scout world sprite |
| ALI-0124 | `rusted_kingdoms/assets/sprites/enemies/goblin_scout_sling_scout.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_scout_sling_scout.tsx` | `e4778f7bb41264d35830badaf06e4b75bcd25e35e41e795968c9c472bb08ff65` | Tiled TSX / Sling Scout world sprite |
| ALI-0125 | `rusted_kingdoms/assets/sprites/enemies/goblin_warrior.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_warrior.png` | `c4d573e3aba47317471d954d77616e4d64726dc2549cf3ff678e0e9f72d575b0` | image / Goblin Warrior world sprite |
| ALI-0126 | `rusted_kingdoms/assets/sprites/enemies/goblin_warrior.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_warrior.tsx` | `80c1a9800368465335f1ae30b814512c21dd76beb0082c7da3653f37e485c64d` | Tiled TSX / Goblin Warrior world sprite |
| ALI-0127 | `rusted_kingdoms/assets/sprites/enemies/grik_the_grin.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/grik_the_grin.png` | `8fc2dc90937967a824d0a18552d05cf987e388d6da6ff0c2f47bf757db17bdf0` | image / Grik the Grin boss sprite |
| ALI-0128 | `rusted_kingdoms/assets/sprites/enemies/grik_the_grin.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/grik_the_grin.tsx` | `b6540c7deb52b649ea3863fa47d497807a528f1f9b552a7a6639e3e54cb9fe65` | Tiled TSX / Grik the Grin boss sprite |

Additional evidence boundaries:

- `ALI-0114` and `ALI-0115`: the pinned
  `rusted_kingdoms/assets/audio/README-audio.md` records titles and generation
  prompts, but no creator identity or redistribution license for these exact
  MP3 hashes.
- `ALI-0116`: the pinned `rusted_kingdoms/assets/audio/sfx/CREDIT` identifies
  Leohpaz as creator of the source sound-effect collection, but it does not
  establish the exact acquisition record or redistribution terms for this
  copied file. Preserve that credit while the file remains in the local parity
  package.
- `ALI-0117` through `ALI-0128`: the source README describes character sprites
  generally as LPC generator assets and links a global credits file, but the
  pinned tree has no per-file generator credits for these exact images. Exact
  component creators, license choices, and required attribution remain unknown.

### Asset entry: `ALI-0129` — Ember Atlas parchment base map

| Field | Value |
| --- | --- |
| Stable entry ID | `ALI-0129` |
| Source path | not-applicable — generated directly for B4.1; no pre-existing source asset was copied |
| Destination path | `assets/scenarios/rusted_kingdoms/media/images/ui/ember_atlas.png` |
| Source SHA-256 | `fedc104155fd24406064ef471417fbaf828070c968f8d6d83fa6d481a54aa659` |
| Destination SHA-256 | `fedc104155fd24406064ef471417fbaf828070c968f8d6d83fa6d481a54aa659` |
| Asset kind | image |
| Title/name | Ember Atlas parchment base map |
| Creator/rightsholder | unknown pending project rights review |
| Source/evidence | OpenAI image-generation output created for B4.1 on 2026-09-01; prompt: `wide hand-painted fantasy parchment world map, burned irregular edges, muted terrain silhouettes, no text, labels, pins, routes, UI, compass, or watermark` |
| License identifier/name | unknown |
| License text/notice location | unknown |
| Required attribution | unknown |
| Modification status/details | unmodified generated PNG; runtime code supplies all labels, pins, routes, and highlights |
| Redistribution permission | unknown |
| Commercial-use permission | unknown |
| Derivative-work permission | unknown |
| Review status | `needs-evidence` |
| Reviewer/date | Codex evidence audit, 2026-09-07; approval still requires a named authorized reviewer |

Notes/blocker: preserve the exact generated hash and establish which OpenAI product and account
terms governed the generation, retain the applicable dated output-rights evidence, and determine
any required disclosure or notice before release approval. The current official
[OpenAI API data-control documentation](https://developers.openai.com/api/docs/guides/your-data)
confirms image-generation handling but does not identify the account/product terms that applied to
this tool-created file. This entry records provenance only and does not treat generation as
automatic redistribution clearance.

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

## M14.05 backfill: byte-identical migrated payload files

Payload files that had no ledger entry before the 2026-09-12 backfill and are
byte-for-byte identical to their counterpart in the pinned source tree at
`08970359d6cb03586948625d29b0d3351dbbf785`. Each row is a distinct ledger entry. Shared review fields for every
row:

- creator/rightsholder: unknown; commit authorship does not establish ownership;
- source/evidence: the exact pinned source file at `08970359d6cb03586948625d29b0d3351dbbf785`, compared by
  SHA-256 on 2026-09-12;
- license, notice, and required attribution: unknown;
- modification: unmodified, with identical source and destination SHA-256;
- redistribution, commercial use, and derivatives: unknown;
- review: `needs-evidence`, automated backfill audit, 2026-09-12;
- related work: M14.05; and
- blocker: **Release blocker.** Establish exact ownership, provenance, license,
  attribution, and redistribution permission for each hash before public
  release; otherwise replace it.

These rows record provenance only. Recording that a file came from the pinned
source does not establish who owns it or whether it may be redistributed, and
no row here may be shipped while its status is `needs-evidence`.

| ID | Source path in the pinned source tree | Destination path | SHA-256 | Kind/name |
| --- | --- | --- | --- | --- |
| ALI-0130 | `rusted_kingdoms/data/dialogue/apothecary_ardel.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/apothecary_ardel.yaml` | `4564f243347ebb6d822105c9eba377b7dbb44dc4ae6e4a21d977b754657c11b0` | scenario YAML / apothecary_ardel.yaml |
| ALI-0131 | `rusted_kingdoms/data/dialogue/armor_shop_ardel.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/armor_shop_ardel.yaml` | `3fa99c6742ce65edbd20c83d64999b141795cb2a90facabcd6a09fd6a4ada163` | scenario YAML / armor_shop_ardel.yaml |
| ALI-0132 | `rusted_kingdoms/data/dialogue/armor_shop_ashenveil.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/armor_shop_ashenveil.yaml` | `920aa2eba7e928093c09380638046e84e8cf8a964f96ef127c37bc5e2e545688` | scenario YAML / armor_shop_ashenveil.yaml |
| ALI-0133 | `rusted_kingdoms/data/dialogue/armor_shop_frostholm.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/armor_shop_frostholm.yaml` | `1613013aea192ec9a08e2f4946509ba341d3bc59e17d208cd5149bddccbdd077` | scenario YAML / armor_shop_frostholm.yaml |
| ALI-0134 | `rusted_kingdoms/data/dialogue/armor_shop_harborgate.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/armor_shop_harborgate.yaml` | `160bd819d9effe2d780edff8d95b9622620774ff5f17592480633e7151f744e7` | scenario YAML / armor_shop_harborgate.yaml |
| ALI-0135 | `rusted_kingdoms/data/dialogue/armor_shop_millhaven.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/armor_shop_millhaven.yaml` | `1b6f81f727271ca10851d4efe07ce0801fe16da1e64453dd2de94a073c6ef8a7` | scenario YAML / armor_shop_millhaven.yaml |
| ALI-0136 | `rusted_kingdoms/data/dialogue/armor_shop_ruinwatch.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/armor_shop_ruinwatch.yaml` | `ade853704ba482abf24eb970b7666c3fdcb6a94a9544cba500d312efd438b20a` | scenario YAML / armor_shop_ruinwatch.yaml |
| ALI-0137 | `rusted_kingdoms/data/dialogue/ashenveil_acolyte.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/ashenveil_acolyte.yaml` | `a2b215c1afa235383d44dcbd54b25a50586edfa69d2c111ab3a7e40c5158b614` | scenario YAML / ashenveil_acolyte.yaml |
| ALI-0138 | `rusted_kingdoms/data/dialogue/ashenveil_ashgatherer.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/ashenveil_ashgatherer.yaml` | `8c4887c70859c61858baff9d81fedd8d4884e792cfcc80a40b7a22a2854af388` | scenario YAML / ashenveil_ashgatherer.yaml |
| ALI-0139 | `rusted_kingdoms/data/dialogue/ashenveil_keeper.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/ashenveil_keeper.yaml` | `54846871280a4dc7044e93d060fec261e7695703b75b58952abe79bdcf0753ef` | scenario YAML / ashenveil_keeper.yaml |
| ALI-0140 | `rusted_kingdoms/data/dialogue/ashenveil_mourner.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/ashenveil_mourner.yaml` | `862098fde20821b5cd964716a7ae87a202857865ed72258c7463f306352f0dcf` | scenario YAML / ashenveil_mourner.yaml |
| ALI-0141 | `rusted_kingdoms/data/dialogue/ashenveil_widow.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/ashenveil_widow.yaml` | `6bc16dfc51dac21803bcb2c8202e6188dc9f144606b2f6212d00b9523c2ee858` | scenario YAML / ashenveil_widow.yaml |
| ALI-0142 | `rusted_kingdoms/data/dialogue/frostholm_beggar.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/frostholm_beggar.yaml` | `b2394be62bee64b4cdb5ad439da68278a9b7097f14e4e76134fb820a7aaa012e` | scenario YAML / frostholm_beggar.yaml |
| ALI-0143 | `rusted_kingdoms/data/dialogue/frostholm_captain_hint.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/frostholm_captain_hint.yaml` | `eb8012464a4deb3b4a2679330b586b805adbe7dc94a928f113260949c164eb64` | scenario YAML / frostholm_captain_hint.yaml |
| ALI-0144 | `rusted_kingdoms/data/dialogue/frostholm_king.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/frostholm_king.yaml` | `30410b1d378b8c415d0ecd2034f791e7c1c4028bae3f12a3083e0985e70ebfc0` | scenario YAML / frostholm_king.yaml |
| ALI-0145 | `rusted_kingdoms/data/dialogue/frostholm_quartermaster.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/frostholm_quartermaster.yaml` | `5db86d74918cd5b3778633ad585d5b77b68b54f365f8a30bc9dfcb64b9fd0665` | scenario YAML / frostholm_quartermaster.yaml |
| ALI-0146 | `rusted_kingdoms/data/dialogue/frostholm_sentry.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/frostholm_sentry.yaml` | `7337ea4af375a582136ce407e9db7fce073166687580d79478679afc9ac2c267` | scenario YAML / frostholm_sentry.yaml |
| ALI-0147 | `rusted_kingdoms/data/dialogue/harborgate_clerk.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/harborgate_clerk.yaml` | `9f0544de09e2fca292262426eb27d0e0bf759dcaa5a994f5fa9c9261abe3fbb7` | scenario YAML / harborgate_clerk.yaml |
| ALI-0148 | `rusted_kingdoms/data/dialogue/harborgate_dockhand.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/harborgate_dockhand.yaml` | `7067dbe651452f4c6b5fc24ff871726c2896c5461aa139b9b88e553231081307` | scenario YAML / harborgate_dockhand.yaml |
| ALI-0149 | `rusted_kingdoms/data/dialogue/harborgate_fishwife.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/harborgate_fishwife.yaml` | `3fef1dd73d023b1ad2a408966ffcee0548bf25c1e8c31536b975999075ebf3e6` | scenario YAML / harborgate_fishwife.yaml |
| ALI-0150 | `rusted_kingdoms/data/dialogue/harborgate_patient.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/harborgate_patient.yaml` | `30d6cf3f339a0261be9adb3553a3a09eaf3d778a1bf6b4ebf913e7da84d0a3f0` | scenario YAML / harborgate_patient.yaml |
| ALI-0151 | `rusted_kingdoms/data/dialogue/harborgate_priestess.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/harborgate_priestess.yaml` | `de1a183567aa3d8b4f8701a632585936a7bde4440d80bf660a2948a6f9418e1a` | scenario YAML / harborgate_priestess.yaml |
| ALI-0152 | `rusted_kingdoms/data/dialogue/harborgate_sailor.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/harborgate_sailor.yaml` | `4dc0c3d95a4848bba932e3baddcc3bff0fea1a49053a9ebb44c13414dbdc9425` | scenario YAML / harborgate_sailor.yaml |
| ALI-0153 | `rusted_kingdoms/data/dialogue/harborgate_stevedore.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/harborgate_stevedore.yaml` | `dd94a90490e9f218eaed08d55d0598b9a1002d1967da53b7dc3c842f412cd582` | scenario YAML / harborgate_stevedore.yaml |
| ALI-0154 | `rusted_kingdoms/data/dialogue/inn_ardel.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/inn_ardel.yaml` | `66757170fb04f28007f854879d2d701d7277b05733214b5039edda4d89649442` | scenario YAML / inn_ardel.yaml |
| ALI-0155 | `rusted_kingdoms/data/dialogue/inn_ashenveil.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/inn_ashenveil.yaml` | `b664233e1bb64746f950c6b0f4d14e4fdc83c779d773ddc492adfc4e5ab9c590` | scenario YAML / inn_ashenveil.yaml |
| ALI-0156 | `rusted_kingdoms/data/dialogue/inn_frostholm.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/inn_frostholm.yaml` | `06b0d8b6a8c0370ecace2d42aab7164262fac74e4cc595f7730a3bc894802015` | scenario YAML / inn_frostholm.yaml |
| ALI-0157 | `rusted_kingdoms/data/dialogue/inn_harborgate.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/inn_harborgate.yaml` | `0c30aae0a428fdb2b1ba569125165ef113a728749c05240dade5253778b9e724` | scenario YAML / inn_harborgate.yaml |
| ALI-0158 | `rusted_kingdoms/data/dialogue/inn_millhaven.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/inn_millhaven.yaml` | `ec32c8d155dba203f71db4b8c92a7e2836daed17879f04b91322ee234db7e192` | scenario YAML / inn_millhaven.yaml |
| ALI-0159 | `rusted_kingdoms/data/dialogue/inn_ruinwatch.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/inn_ruinwatch.yaml` | `33e2f1ba133a198ef91dc29d7258247e103c10bd317b5c2cffd9dcc1b5a9d302` | scenario YAML / inn_ruinwatch.yaml |
| ALI-0160 | `rusted_kingdoms/data/dialogue/item_shop_ardel.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/item_shop_ardel.yaml` | `16e357263d597f255ea578e39031a730c0ad6221e0134cd321cc9223beb26db7` | scenario YAML / item_shop_ardel.yaml |
| ALI-0161 | `rusted_kingdoms/data/dialogue/item_shop_ashenveil.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/item_shop_ashenveil.yaml` | `dd0c287180aa8c40f50f7e7adc3fa4c8360500c932d3e755455ab54536ad8f7f` | scenario YAML / item_shop_ashenveil.yaml |
| ALI-0162 | `rusted_kingdoms/data/dialogue/item_shop_frostholm.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/item_shop_frostholm.yaml` | `9276f457896b4a27a6b11e0bdfa62d4940b710f5c28ecd4adec35f8505b5456a` | scenario YAML / item_shop_frostholm.yaml |
| ALI-0163 | `rusted_kingdoms/data/dialogue/item_shop_harborgate.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/item_shop_harborgate.yaml` | `d108167a42b415a30ccf1eb2fbcca0e86bb87fd9b494e36f934526fe2257673c` | scenario YAML / item_shop_harborgate.yaml |
| ALI-0164 | `rusted_kingdoms/data/dialogue/item_shop_millhaven.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/item_shop_millhaven.yaml` | `3bbcf712d28e5e89500a10f02335295488e8aa0ba6a98e53ab3d7175262fb635` | scenario YAML / item_shop_millhaven.yaml |
| ALI-0165 | `rusted_kingdoms/data/dialogue/item_shop_ruinwatch.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/item_shop_ruinwatch.yaml` | `5208e3879e8b47a4ca043213183e714ed81bee0e207a76490dd11392670c795d` | scenario YAML / item_shop_ruinwatch.yaml |
| ALI-0166 | `rusted_kingdoms/data/dialogue/jep_join.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/jep_join.yaml` | `8a8615b7e8914d20a3a5ee158cf64f2ba6d7c7075691829ecc12a957e8955c61` | scenario YAML / jep_join.yaml |
| ALI-0167 | `rusted_kingdoms/data/dialogue/kael_join.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/kael_join.yaml` | `609ed2307ce322b9eb10e742c9c34583761cd4a12adff062b17a5ce676737c57` | scenario YAML / kael_join.yaml |
| ALI-0168 | `rusted_kingdoms/data/dialogue/millhaven_baker.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/millhaven_baker.yaml` | `ca416b25359dc208d4fdb777d079bbf174cd94cfd1eb10ba9f6aa7fda9815dbc` | scenario YAML / millhaven_baker.yaml |
| ALI-0169 | `rusted_kingdoms/data/dialogue/millhaven_carter.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/millhaven_carter.yaml` | `1212f9c4f94f5a86cec6896e08cb669be4a9c9307c2fee2418a7ac5502107f39` | scenario YAML / millhaven_carter.yaml |
| ALI-0170 | `rusted_kingdoms/data/dialogue/millhaven_elder_hint.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/millhaven_elder_hint.yaml` | `15bb9fd2957014bc1bf523d293ea7f5318857e3283290ff5306b1aebd35504c2` | scenario YAML / millhaven_elder_hint.yaml |
| ALI-0171 | `rusted_kingdoms/data/dialogue/millhaven_gossip.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/millhaven_gossip.yaml` | `be3d8c39eb932a7a1ba3bca0290abd7c6a9f43c198a76e08584d247bf41a64a0` | scenario YAML / millhaven_gossip.yaml |
| ALI-0172 | `rusted_kingdoms/data/dialogue/millhaven_granary.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/millhaven_granary.yaml` | `44b137aafbca8b938b2e85281c9687d16ce27e249e81753a89cb2b63abb1080c` | scenario YAML / millhaven_granary.yaml |
| ALI-0173 | `rusted_kingdoms/data/dialogue/millhaven_miller.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/millhaven_miller.yaml` | `29fa79803f01e562cd8a44b820eee93b049cd4913a8c5dc2b75c4a1809b26d70` | scenario YAML / millhaven_miller.yaml |
| ALI-0174 | `rusted_kingdoms/data/dialogue/reiya_join.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/reiya_join.yaml` | `48d623d52a60c3da9d171b8ae214c48df67d2350adca55bafd4b9d5e76ad798b` | scenario YAML / reiya_join.yaml |
| ALI-0175 | `rusted_kingdoms/data/dialogue/ruinwatch_archivist.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/ruinwatch_archivist.yaml` | `ba1c28d333246779f75a9ad37ce07ec866be2cce57e43587381d220d484a2235` | scenario YAML / ruinwatch_archivist.yaml |
| ALI-0176 | `rusted_kingdoms/data/dialogue/ruinwatch_digger.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/ruinwatch_digger.yaml` | `55df6a617667572ba68e1b761888f46a40650266261bafb57832dc3917451a7f` | scenario YAML / ruinwatch_digger.yaml |
| ALI-0177 | `rusted_kingdoms/data/dialogue/ruinwatch_mason.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/ruinwatch_mason.yaml` | `fb2ea6d991d9a064948ebbd569b83f31a31ea3067dd7ed477b88de00e06192a6` | scenario YAML / ruinwatch_mason.yaml |
| ALI-0178 | `rusted_kingdoms/data/dialogue/ruinwatch_pilgrim.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/ruinwatch_pilgrim.yaml` | `08de9bc08c38bec6bdd803a2645593eda38ea3a7801ffd855ef2ec7e886f50fc` | scenario YAML / ruinwatch_pilgrim.yaml |
| ALI-0179 | `rusted_kingdoms/data/dialogue/ruinwatch_scholar_hint.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/ruinwatch_scholar_hint.yaml` | `9c94aba79150cd534aebbba03a8b41b339fe2d382b22c56a04f96ebf4668487d` | scenario YAML / ruinwatch_scholar_hint.yaml |
| ALI-0180 | `rusted_kingdoms/data/dialogue/sign_town_02_millhaven.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/sign_town_02_millhaven.yaml` | `dee2ce5637e699b29d5a8597f19579eb1ac98e2975ad129dbd5c5725e30b4219` | scenario YAML / sign_town_02_millhaven.yaml |
| ALI-0181 | `rusted_kingdoms/data/dialogue/sign_town_03_ruinwatch.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/sign_town_03_ruinwatch.yaml` | `7de8ec4a2bc06d5f26037afc4981a192fdde5988d6b414453aa73ddbae1cefda` | scenario YAML / sign_town_03_ruinwatch.yaml |
| ALI-0182 | `rusted_kingdoms/data/dialogue/sign_town_04_frostholm.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/sign_town_04_frostholm.yaml` | `1986fdcdeeec177e2895b2bb055c0a54505f2e89711d980cd4e36e5ef0468d1e` | scenario YAML / sign_town_04_frostholm.yaml |
| ALI-0183 | `rusted_kingdoms/data/dialogue/sign_town_05_ashenveil.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/sign_town_05_ashenveil.yaml` | `e5c2576f49ffc2a2eef8c8aba4a41eba8939a1ebbacb9b3ba57fb3002ee9527c` | scenario YAML / sign_town_05_ashenveil.yaml |
| ALI-0184 | `rusted_kingdoms/data/dialogue/sign_zone_02_open_plains.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/sign_zone_02_open_plains.yaml` | `ebaee2ace6abfed216322dde0d9484298fa33b905ba9b371e0d97ebcda646242` | scenario YAML / sign_zone_02_open_plains.yaml |
| ALI-0185 | `rusted_kingdoms/data/dialogue/sign_zone_05_mountain_foothills_02.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/sign_zone_05_mountain_foothills_02.yaml` | `5b5a3cf5af2934014144eea0db7ad00a38eeb659852aafb85ba363758e597f0e` | scenario YAML / sign_zone_05_mountain_foothills_02.yaml |
| ALI-0186 | `rusted_kingdoms/data/dialogue/sign_zone_05_mountain_foothills_03.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/sign_zone_05_mountain_foothills_03.yaml` | `994e1553c068730f68606164ad3823d98a2f817f46ed30fcfea9f2decfea3847` | scenario YAML / sign_zone_05_mountain_foothills_03.yaml |
| ALI-0187 | `rusted_kingdoms/data/dialogue/sign_zone_06_mountain_pass.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/sign_zone_06_mountain_pass.yaml` | `4993d1551a2f9b684b25473fcfb2eedf8bcfa3adb176264406a3a5d4d5849802` | scenario YAML / sign_zone_06_mountain_pass.yaml |
| ALI-0188 | `rusted_kingdoms/data/dialogue/sign_zone_06_mountain_pass_02.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/sign_zone_06_mountain_pass_02.yaml` | `64d0029584ac82cf15a93c83a45a4e64d69179d3565f6cec9c113ebf0136a4e2` | scenario YAML / sign_zone_06_mountain_pass_02.yaml |
| ALI-0189 | `rusted_kingdoms/data/dialogue/sign_zone_06_mountain_pass_03.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/sign_zone_06_mountain_pass_03.yaml` | `8288e2a88fcd82fae084b245ae6c24d2b65d766ea5d93c08ccb953b730e03cdd` | scenario YAML / sign_zone_06_mountain_pass_03.yaml |
| ALI-0190 | `rusted_kingdoms/data/dialogue/sign_zone_07_sunken_cave.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/sign_zone_07_sunken_cave.yaml` | `47f6e61ee38647d779d5a20eb27c4f432c06e4a00b229d87273ce731f926e982` | scenario YAML / sign_zone_07_sunken_cave.yaml |
| ALI-0191 | `rusted_kingdoms/data/dialogue/sign_zone_08_corrupted_forest.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/sign_zone_08_corrupted_forest.yaml` | `3b4b501c610bb30fa167876bfb6fcd83918b5070165dbd10ec6e546b4d8f294a` | scenario YAML / sign_zone_08_corrupted_forest.yaml |
| ALI-0192 | `rusted_kingdoms/data/dialogue/sign_zone_09_volcanic_region.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/sign_zone_09_volcanic_region.yaml` | `cd7632944de78321dc30acdeb427ab00dd1c11f8493e1957ba834c1198c4f893` | scenario YAML / sign_zone_09_volcanic_region.yaml |
| ALI-0193 | `rusted_kingdoms/data/dialogue/sign_zone_10_final_stronghold.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/sign_zone_10_final_stronghold.yaml` | `48fab54c0afeefd03b0d30e37a3e780b731b76c0685cac3f5c31024ac681d65d` | scenario YAML / sign_zone_10_final_stronghold.yaml |
| ALI-0194 | `rusted_kingdoms/data/dialogue/weapon_shop_ardel.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/weapon_shop_ardel.yaml` | `9e227ebb9b3de4f5318cb7623955211cd22586fd016158b0951bcf3c7a5bf7e9` | scenario YAML / weapon_shop_ardel.yaml |
| ALI-0195 | `rusted_kingdoms/data/dialogue/weapon_shop_ashenveil.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/weapon_shop_ashenveil.yaml` | `4412032b6bfb0a9b028780100f69238e977050b9a274ee8c20f813b7bb92f3a8` | scenario YAML / weapon_shop_ashenveil.yaml |
| ALI-0196 | `rusted_kingdoms/data/dialogue/weapon_shop_frostholm.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/weapon_shop_frostholm.yaml` | `9c5788166b981e0095b72726f144cdd404b0537bd603c962d3d3850962762ced` | scenario YAML / weapon_shop_frostholm.yaml |
| ALI-0197 | `rusted_kingdoms/data/dialogue/weapon_shop_harborgate.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/weapon_shop_harborgate.yaml` | `22cdc6c99808177e10508c65d1c92be9798abd9c8dd3ea01b5d4115d14f8b669` | scenario YAML / weapon_shop_harborgate.yaml |
| ALI-0198 | `rusted_kingdoms/data/dialogue/weapon_shop_millhaven.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/weapon_shop_millhaven.yaml` | `488d814abc1f0a18690b4a70b08164f957eb026743845d84d20e5eb9294a51bd` | scenario YAML / weapon_shop_millhaven.yaml |
| ALI-0199 | `rusted_kingdoms/data/dialogue/weapon_shop_ruinwatch.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/weapon_shop_ruinwatch.yaml` | `bc4755ad57aa917e0de9fffbfae9bff8b179b41934ea42fa14c9338f2f0b47f1` | scenario YAML / weapon_shop_ruinwatch.yaml |
| ALI-0200 | `rusted_kingdoms/data/encount/zone_02_open_plains.yaml` | `assets/scenarios/rusted_kingdoms/data/encount/zone_02_open_plains.yaml` | `e0fa8962d5203746d89c22f503c0ec1147aa47091e07a7303dc7097004b08132` | scenario YAML / zone_02_open_plains.yaml |
| ALI-0201 | `rusted_kingdoms/data/encount/zone_03_marshland.yaml` | `assets/scenarios/rusted_kingdoms/data/encount/zone_03_marshland.yaml` | `5b74d6e0726ec2dde72f76b6c45b3ca119df1848e50b7beb6585247b3a72a26e` | scenario YAML / zone_03_marshland.yaml |
| ALI-0202 | `rusted_kingdoms/data/encount/zone_04_ancient_ruins_01_gate.yaml` | `assets/scenarios/rusted_kingdoms/data/encount/zone_04_ancient_ruins_01_gate.yaml` | `67ea70b57abb2f9cd31e7d48a377903d1d340ab6f13b5d1249a28c1b78769058` | scenario YAML / zone_04_ancient_ruins_01_gate.yaml |
| ALI-0203 | `rusted_kingdoms/data/encount/zone_04_ancient_ruins_02_courtyard.yaml` | `assets/scenarios/rusted_kingdoms/data/encount/zone_04_ancient_ruins_02_courtyard.yaml` | `fc26cb6d58f031c16690926551847faab57ac12382baddec65ec6f73c47942d9` | scenario YAML / zone_04_ancient_ruins_02_courtyard.yaml |
| ALI-0204 | `rusted_kingdoms/data/encount/zone_04_ancient_ruins_03_sanctum.yaml` | `assets/scenarios/rusted_kingdoms/data/encount/zone_04_ancient_ruins_03_sanctum.yaml` | `9e2a01da23ac00e598212f7ade79e71f146bceeded49a8126ab766e8a46cc10d` | scenario YAML / zone_04_ancient_ruins_03_sanctum.yaml |
| ALI-0205 | `rusted_kingdoms/data/encount/zone_05_mountain_foothills_01.yaml` | `assets/scenarios/rusted_kingdoms/data/encount/zone_05_mountain_foothills_01.yaml` | `81b3169b7129d2843e4ebb484b8737c2f52e4e9652b879a8b7dee63a164d2517` | scenario YAML / zone_05_mountain_foothills_01.yaml |
| ALI-0206 | `rusted_kingdoms/data/encount/zone_05_mountain_foothills_02.yaml` | `assets/scenarios/rusted_kingdoms/data/encount/zone_05_mountain_foothills_02.yaml` | `c661e471e8419838808cde4f20ceb90ea59c5d8f9ca37c58daccd6e850900cdf` | scenario YAML / zone_05_mountain_foothills_02.yaml |
| ALI-0207 | `rusted_kingdoms/data/encount/zone_06_mountain_pass_01.yaml` | `assets/scenarios/rusted_kingdoms/data/encount/zone_06_mountain_pass_01.yaml` | `3ab092a8362b21c7822f9a26336cab154d94a2bd8204e33fc2cfde30934c59fb` | scenario YAML / zone_06_mountain_pass_01.yaml |
| ALI-0208 | `rusted_kingdoms/data/encount/zone_06_mountain_pass_02.yaml` | `assets/scenarios/rusted_kingdoms/data/encount/zone_06_mountain_pass_02.yaml` | `6d86ac4ee7b4f53796dddb12c4cfc8b494eefeba84ede316aa0e724aaec1c20b` | scenario YAML / zone_06_mountain_pass_02.yaml |
| ALI-0209 | `rusted_kingdoms/data/encount/zone_06_mountain_pass_03.yaml` | `assets/scenarios/rusted_kingdoms/data/encount/zone_06_mountain_pass_03.yaml` | `e3438031d7ca1203e8436d5706668e7fca36589ea2ea4faf8730e94c15b9cbc0` | scenario YAML / zone_06_mountain_pass_03.yaml |
| ALI-0210 | `rusted_kingdoms/data/encount/zone_07_sunken_cave.yaml` | `assets/scenarios/rusted_kingdoms/data/encount/zone_07_sunken_cave.yaml` | `907f400557b8cf2257e790d3383504c1ec2bc9a4a2034ab95c5d39c57acd4666` | scenario YAML / zone_07_sunken_cave.yaml |
| ALI-0211 | `rusted_kingdoms/data/encount/zone_08_corrupted_forest.yaml` | `assets/scenarios/rusted_kingdoms/data/encount/zone_08_corrupted_forest.yaml` | `10807357e6ee958e1a90f654f3a46aead94a0b53bf271ad1f29485c3c9856bdb` | scenario YAML / zone_08_corrupted_forest.yaml |
| ALI-0212 | `rusted_kingdoms/data/encount/zone_09_volcanic_region.yaml` | `assets/scenarios/rusted_kingdoms/data/encount/zone_09_volcanic_region.yaml` | `5718980328a291b1384dccc775d77f428d8db9bc66548015c033ff8bf15e7d2f` | scenario YAML / zone_09_volcanic_region.yaml |
| ALI-0213 | `rusted_kingdoms/data/encount/zone_10_final_stronghold.yaml` | `assets/scenarios/rusted_kingdoms/data/encount/zone_10_final_stronghold.yaml` | `de43ed4c424eceee6c5570250c42da38d4c6b9912805f97f3e1cfcc599ef03db` | scenario YAML / zone_10_final_stronghold.yaml |
| ALI-0214 | `rusted_kingdoms/data/enemies/boss_move_sets/fallen_angel_red_judicator.yaml` | `assets/scenarios/rusted_kingdoms/data/enemies/boss_move_sets/fallen_angel_red_judicator.yaml` | `1338c012323335bc20ae0fef58b3eb443cb4d9d2a9d8045bfbd17f5a98cf54d4` | scenario YAML / fallen_angel_red_judicator.yaml |
| ALI-0215 | `rusted_kingdoms/data/enemies/boss_move_sets/orc_shaman_red_hood_shaman.yaml` | `assets/scenarios/rusted_kingdoms/data/enemies/boss_move_sets/orc_shaman_red_hood_shaman.yaml` | `b30ff115078bdd42bed7fe0dfbda91374fc1e4b03c54ec0b2a3a3689d7be77a7` | scenario YAML / orc_shaman_red_hood_shaman.yaml |
| ALI-0216 | `rusted_kingdoms/data/enemies/boss_move_sets/pirate_captain_eyepatch_captain.yaml` | `assets/scenarios/rusted_kingdoms/data/enemies/boss_move_sets/pirate_captain_eyepatch_captain.yaml` | `7dcaac3ffc6fefe7e94e5a5a5aa2ab5b7a32014eecb1639ae846c1bfde937d0c` | scenario YAML / pirate_captain_eyepatch_captain.yaml |
| ALI-0217 | `rusted_kingdoms/data/enemies/boss_move_sets/ratkin_plague_doctor_black_mask_doctor.yaml` | `assets/scenarios/rusted_kingdoms/data/enemies/boss_move_sets/ratkin_plague_doctor_black_mask_doctor.yaml` | `ab4e79e8d93e441fc595be0c0e241445a86de380ccefa698b17212ca4665cf66` | scenario YAML / ratkin_plague_doctor_black_mask_doctor.yaml |
| ALI-0218 | `rusted_kingdoms/data/enemies/boss_move_sets/skeleton_knight_base.yaml` | `assets/scenarios/rusted_kingdoms/data/enemies/boss_move_sets/skeleton_knight_base.yaml` | `0870f7fde8f6cf8ba0f81c99ce75de980bd798d2733db573c9d3bfe1e6cdea1f` | scenario YAML / skeleton_knight_base.yaml |
| ALI-0219 | `rusted_kingdoms/data/enemies/boss_move_sets/troll_shaman_base.yaml` | `assets/scenarios/rusted_kingdoms/data/enemies/boss_move_sets/troll_shaman_base.yaml` | `e078c112094c22f1daf971f503d67320999ddc578c93c73dc48b49c942fc9987` | scenario YAML / troll_shaman_base.yaml |
| ALI-0220 | `rusted_kingdoms/data/enemies/boss_move_sets/wartotaur_warlord_blackhorn_chief.yaml` | `assets/scenarios/rusted_kingdoms/data/enemies/boss_move_sets/wartotaur_warlord_blackhorn_chief.yaml` | `6d551b6dc0e8e2562e2028679c7f925843c41bcdb16dddf2732ca3ca2fc05e1f` | scenario YAML / wartotaur_warlord_blackhorn_chief.yaml |
| ALI-0221 | `rusted_kingdoms/data/enemies/boss_move_sets/wolf_beast_black_fur.yaml` | `assets/scenarios/rusted_kingdoms/data/enemies/boss_move_sets/wolf_beast_black_fur.yaml` | `399df63a8df588b7de28e951841ce9ba455ebafb0eff79b5886fb48065fd622c` | scenario YAML / wolf_beast_black_fur.yaml |
| ALI-0222 | `rusted_kingdoms/data/maps/zone_05_mountain_foothills_02.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/zone_05_mountain_foothills_02.yaml` | `afa5d197734e9c07c7effed8ef8f31519894a190be027b4bc6177cc102af7f7e` | scenario YAML / zone_05_mountain_foothills_02.yaml |
| ALI-0223 | `rusted_kingdoms/data/maps/zone_05_mountain_foothills_03.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/zone_05_mountain_foothills_03.yaml` | `671cf4856f07cb5f814ed6ce462894254edcbc1896bb975d5f89635eac9b57ef` | scenario YAML / zone_05_mountain_foothills_03.yaml |
| ALI-0224 | `rusted_kingdoms/data/maps/zone_06_mountain_pass_02.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/zone_06_mountain_pass_02.yaml` | `c819f6136202f8817d1940e501f26572458480f74981a194f9cd62a361c9384b` | scenario YAML / zone_06_mountain_pass_02.yaml |
| ALI-0225 | `rusted_kingdoms/data/maps/zone_06_mountain_pass_03.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/zone_06_mountain_pass_03.yaml` | `e5dbd989b0043f27496be24c7a4aca67f906748fb52756219041a7820d7feb69` | scenario YAML / zone_06_mountain_pass_03.yaml |
| ALI-0226 | `rusted_kingdoms/assets/audio/bgm/Across_the_Sundered_Peaks_64.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/bgm/Across_the_Sundered_Peaks_64.mp3` | `5910a7b194a72afc272835403ca75220db5ec28c7cdc49125f622548c6449909` | audio / Across_the_Sundered_Peaks_64.mp3 |
| ALI-0227 | `rusted_kingdoms/assets/audio/bgm/Golden_Hour_Haze.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/bgm/Golden_Hour_Haze.mp3` | `1e3d52cd11f14f26826b1ef4f522c359c8ae72512906b0c617bfb5f05a0be5c4` | audio / Golden_Hour_Haze.mp3 |
| ALI-0228 | `rusted_kingdoms/assets/audio/bgm/Neon_Samurai_s_Lament.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/bgm/Neon_Samurai_s_Lament.mp3` | `f44404dd9b445b9f851f90c18a0055fba61a11976ac362c9aba97fe6831bcbeb` | audio / Neon_Samurai_s_Lament.mp3 |
| ALI-0229 | `rusted_kingdoms/assets/audio/bgm/Sun_Kissed_Horizon.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/bgm/Sun_Kissed_Horizon.mp3` | `4112aa18af0b16e4b7798f983db1bb2972911017fd727034d1db6eb1197c4221` | audio / Sun_Kissed_Horizon.mp3 |
| ALI-0230 | `rusted_kingdoms/assets/audio/bgm/The_Horizon_s_Call.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/bgm/The_Horizon_s_Call.mp3` | `b4a6ccb8d2db9b76e2acb8c3cd92a6cc3047eff6e7b26ed2c9f2fbfa025905c3` | audio / The_Horizon_s_Call.mp3 |
| ALI-0231 | `rusted_kingdoms/assets/audio/bgm/Whispers_in_the_Unfolding_Dark.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/bgm/Whispers_in_the_Unfolding_Dark.mp3` | `4bf9658cec7509e1b6fe9a79db35d4d3cc64a77b5a921c94bc10008adafabb7f` | audio / Whispers_in_the_Unfolding_Dark.mp3 |
| ALI-0232 | `rusted_kingdoms/assets/audio/bgm/Whispers_of_the_Alpine_Stream.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/bgm/Whispers_of_the_Alpine_Stream.mp3` | `829ceecab1f73d46bc46cb722eb5d05d9d288cfb02df0f61b8cc68adf74cebb4` | audio / Whispers_of_the_Alpine_Stream.mp3 |
| ALI-0233 | `rusted_kingdoms/assets/audio/bgm/Whispers_of_the_Bamboo_Grove.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/bgm/Whispers_of_the_Bamboo_Grove.mp3` | `251065efbda61e32af30c81bcd2419331d01d1341b50b0128ba1d9bdd2a29181` | audio / Whispers_of_the_Bamboo_Grove.mp3 |
| ALI-0234 | `rusted_kingdoms/assets/audio/sfx/atk_magic/04_Fire_explosion_04_medium.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/atk_magic/04_Fire_explosion_04_medium.mp3` | `fbae582c030681a895e8333953c004cab2a8031177a6b49e0c1e15869dc35a80` | audio / 04_Fire_explosion_04_medium.mp3 |
| ALI-0235 | `rusted_kingdoms/assets/audio/sfx/atk_magic/13_Ice_explosion_01.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/atk_magic/13_Ice_explosion_01.mp3` | `bd102f4a8b800a08466e74d8b493babdd873238cc30be6ba892732cbbc52dde1` | audio / 13_Ice_explosion_01.mp3 |
| ALI-0236 | `rusted_kingdoms/assets/audio/sfx/atk_magic/18_Thunder_02.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/atk_magic/18_Thunder_02.mp3` | `0cf53e836c1b4f1e238cb6f40c919bc66bd559bdea7e8c1671c0403566bff893` | audio / 18_Thunder_02.mp3 |
| ALI-0237 | `rusted_kingdoms/assets/audio/sfx/atk_magic/22_Water_02.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/atk_magic/22_Water_02.mp3` | `c9e031634fb363552da34233060f2c6fe0bbb87a9c629d7847b95e6980f45c79` | audio / 22_Water_02.mp3 |
| ALI-0238 | `rusted_kingdoms/assets/audio/sfx/atk_magic/25_Wind_01.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/atk_magic/25_Wind_01.mp3` | `cae371da25ec8c4f04996d0411780c05146972a3854809ec450ae6a37aff3808` | audio / 25_Wind_01.mp3 |
| ALI-0239 | `rusted_kingdoms/assets/audio/sfx/atk_magic/30_Earth_02.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/atk_magic/30_Earth_02.mp3` | `af229936e9dd6f25d65a83cd2543aa24e3aa74d43f4316dba8878a870bb649d1` | audio / 30_Earth_02.mp3 |
| ALI-0240 | `rusted_kingdoms/assets/audio/sfx/atk_magic/46_Poison_01.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/atk_magic/46_Poison_01.mp3` | `3922eb85608c1fafacf078a6bf9b7f5c953f528a0b8ca46ef64e138e78f9d86d` | audio / 46_Poison_01.mp3 |
| ALI-0241 | `rusted_kingdoms/assets/audio/sfx/battle/03_Claw_03.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/battle/03_Claw_03.mp3` | `6ec9fe9a64ecbb4db291c854eb849d78d8258a4be971649d357a28569b6d541b` | audio / 03_Claw_03.mp3 |
| ALI-0242 | `rusted_kingdoms/assets/audio/sfx/battle/08_Bite_04.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/battle/08_Bite_04.mp3` | `90099c18b407baeced8a503a83bfb65f5148f684a0e1c23ece8972cde5ebbdad` | audio / 08_Bite_04.mp3 |
| ALI-0243 | `rusted_kingdoms/assets/audio/sfx/battle/15_Impact_flesh_02.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/battle/15_Impact_flesh_02.mp3` | `7f35630b67ac517d1573dd5fa42abab114c93a9abb3843faf1b41faab1682d7c` | audio / 15_Impact_flesh_02.mp3 |
| ALI-0244 | `rusted_kingdoms/assets/audio/sfx/battle/22_Slash_04.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/battle/22_Slash_04.mp3` | `fd7f1af1cc34219648c80d5e03831b79c3918b9f41833ecca349af465f62e405` | audio / 22_Slash_04.mp3 |
| ALI-0245 | `rusted_kingdoms/assets/audio/sfx/battle/35_Miss_Evade_02.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/battle/35_Miss_Evade_02.mp3` | `eb8a4b4eedd96712471ea03bd8222905a768bcf4f02674bcddf3594331d179e8` | audio / 35_Miss_Evade_02.mp3 |
| ALI-0246 | `rusted_kingdoms/assets/audio/sfx/battle/39_Block_03.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/battle/39_Block_03.mp3` | `629b454fbcd9d2424aad3ba332e5197ff3afc5263098502032fee295ba9f7570` | audio / 39_Block_03.mp3 |
| ALI-0247 | `rusted_kingdoms/assets/audio/sfx/battle/51_Flee_02.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/battle/51_Flee_02.mp3` | `10cc2a2a5c03898f4a8b18f09fb070a1a872848035edbb6f27d0f0801f273e99` | audio / 51_Flee_02.mp3 |
| ALI-0248 | `rusted_kingdoms/assets/audio/sfx/battle/69_Enemy_death_01.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/battle/69_Enemy_death_01.mp3` | `39cef4a713a0b678018a4550af76a82c5c80c14fed0de84803238ceee6399823` | audio / 69_Enemy_death_01.mp3 |
| ALI-0249 | `rusted_kingdoms/assets/audio/sfx/battle/77_flesh_02.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/battle/77_flesh_02.mp3` | `73eb8a5c05dc4dfe14e0b140612e3889fb3f83f1e2ca2d142a1d5061e407f4a6` | audio / 77_flesh_02.mp3 |
| ALI-0250 | `rusted_kingdoms/assets/audio/sfx/buffs_heals/02_Heal_02.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/buffs_heals/02_Heal_02.mp3` | `66c468b6b254efa438841dbfb0ecde1eb57fb6bbf4d94f8bdfdd593ae8c82ee4` | audio / 02_Heal_02.mp3 |
| ALI-0251 | `rusted_kingdoms/assets/audio/sfx/buffs_heals/16_Atk_buff_04.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/buffs_heals/16_Atk_buff_04.mp3` | `4f38c04f4b7e088808ef945732667ee10a9a0207ef5b01feaf1416149e0e316e` | audio / 16_Atk_buff_04.mp3 |
| ALI-0252 | `rusted_kingdoms/assets/audio/sfx/buffs_heals/17_Def_buff_01.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/buffs_heals/17_Def_buff_01.mp3` | `8d7523596ed3330c797ecc17b5034d444c4924b738d0ee9ece70ffc2be269bd6` | audio / 17_Def_buff_01.mp3 |
| ALI-0253 | `rusted_kingdoms/assets/audio/sfx/buffs_heals/21_Debuff_01.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/buffs_heals/21_Debuff_01.mp3` | `03a870a1954a0c5c9fb31a53d236421cd46594a5429cabd98561438663aa7d82` | audio / 21_Debuff_01.mp3 |
| ALI-0254 | `rusted_kingdoms/assets/audio/sfx/buffs_heals/30_Revive_03.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/buffs_heals/30_Revive_03.mp3` | `a0300428e213779890a906c2cde46bade51bdda494ec01ddbde676da14c6a7b2` | audio / 30_Revive_03.mp3 |
| ALI-0255 | `rusted_kingdoms/assets/audio/sfx/buffs_heals/44_Sleep_01.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/buffs_heals/44_Sleep_01.mp3` | `45adeb6b80c0c87dacbac347af219b95fe5599741b39dae497a94e797d952b9b` | audio / 44_Sleep_01.mp3 |
| ALI-0256 | `rusted_kingdoms/assets/audio/sfx/buffs_heals/48_Speed_up_02.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/buffs_heals/48_Speed_up_02.mp3` | `59fb6bf1492446af6e7499e10513ba194e4741002acdad36206be03b782f226c` | audio / 48_Speed_up_02.mp3 |
| ALI-0257 | `rusted_kingdoms/assets/audio/sfx/dungeon/01_chest_open_1.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/dungeon/01_chest_open_1.mp3` | `ab13264b97f43582eccc0d484d9083071fe096c91eb03a35a395b56227914b36` | audio / 01_chest_open_1.mp3 |
| ALI-0258 | `rusted_kingdoms/assets/audio/sfx/dungeon/01_chest_open_2.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/dungeon/01_chest_open_2.mp3` | `b011b51585a92de4f937085b31e970e1d2e0ac9ab53309636fe3acb6965e5a24` | audio / 01_chest_open_2.mp3 |
| ALI-0259 | `rusted_kingdoms/assets/audio/sfx/dungeon/01_chest_open_3.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/dungeon/01_chest_open_3.mp3` | `ef34ff83a9d6dc85407c590eebeb89a8fd41feb71d87d6a4457d64ca53ee10ff` | audio / 01_chest_open_3.mp3 |
| ALI-0260 | `rusted_kingdoms/assets/audio/sfx/dungeon/01_chest_open_4.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/dungeon/01_chest_open_4.mp3` | `781a686850a19c7a844fa14934564a543b7befcf57e8231aa0837b92017f51f1` | audio / 01_chest_open_4.mp3 |
| ALI-0261 | `rusted_kingdoms/assets/audio/sfx/dungeon/02_chest_close_1.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/dungeon/02_chest_close_1.mp3` | `d59f1ecaff2d457fecbff25fd588eddbc108c0915a7061fa46b68d802d3e39a5` | audio / 02_chest_close_1.mp3 |
| ALI-0262 | `rusted_kingdoms/assets/audio/sfx/dungeon/02_chest_close_2.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/dungeon/02_chest_close_2.mp3` | `1d7a349b33b3646a0744c39be68717c2e150c091c1c2a70a9da0a0b73d7b6e1c` | audio / 02_chest_close_2.mp3 |
| ALI-0263 | `rusted_kingdoms/assets/audio/sfx/dungeon/02_chest_close_3.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/dungeon/02_chest_close_3.mp3` | `ed4320d48ec1a85605f2b1ff81f490821a69a2348d7b834813650766134789cf` | audio / 02_chest_close_3.mp3 |
| ALI-0264 | `rusted_kingdoms/assets/audio/sfx/dungeon/05_door_open_1.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/dungeon/05_door_open_1.mp3` | `8a335e37100fcd24e28a05eb1572589e20191595128a188f4fe00279876dfce3` | audio / 05_door_open_1.mp3 |
| ALI-0265 | `rusted_kingdoms/assets/audio/sfx/dungeon/05_door_open_2.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/dungeon/05_door_open_2.mp3` | `eea73ad3f8488a02fcbade1fde7e14b377ffa72e9772668e80b1bfd498e5d5c2` | audio / 05_door_open_2.mp3 |
| ALI-0266 | `rusted_kingdoms/assets/audio/sfx/dungeon/07_human_atk_sword_1.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/dungeon/07_human_atk_sword_1.mp3` | `b643f9219f75ff04909b08fc95fe023518a0dc15fb2898a7508ebd2855023c02` | audio / 07_human_atk_sword_1.mp3 |
| ALI-0267 | `rusted_kingdoms/assets/audio/sfx/dungeon/07_human_atk_sword_2.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/dungeon/07_human_atk_sword_2.mp3` | `a8fba2bd3d159b434667794fa5803cf2e41f7a8801a4182fbd6129a57bedff0e` | audio / 07_human_atk_sword_2.mp3 |
| ALI-0268 | `rusted_kingdoms/assets/audio/sfx/dungeon/07_human_atk_sword_3.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/dungeon/07_human_atk_sword_3.mp3` | `73d303a02c33641bbf9f2ac95a5b883d233c0d0616079c53b318bd06ae5143b1` | audio / 07_human_atk_sword_3.mp3 |
| ALI-0269 | `rusted_kingdoms/assets/audio/sfx/dungeon/11_human_damage_1.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/dungeon/11_human_damage_1.mp3` | `85edcfca332b07c3f80f48352487f56d8779a36a815836870b1d47d1f6689cfe` | audio / 11_human_damage_1.mp3 |
| ALI-0270 | `rusted_kingdoms/assets/audio/sfx/dungeon/11_human_damage_2.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/dungeon/11_human_damage_2.mp3` | `783435aaae473a975b405a2139a5b5b3770beeca625d21df647cd9decf64057b` | audio / 11_human_damage_2.mp3 |
| ALI-0271 | `rusted_kingdoms/assets/audio/sfx/dungeon/11_human_damage_3.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/dungeon/11_human_damage_3.mp3` | `acd8d8d3d6bbdbc431a4e7789efd4cab7a009392acc49c7881c9d0a0d3e99f9c` | audio / 11_human_damage_3.mp3 |
| ALI-0272 | `rusted_kingdoms/assets/audio/sfx/dungeon/17_orc_atk_sword_1.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/dungeon/17_orc_atk_sword_1.mp3` | `fa582c00248d38920e056e03cff683d636500f56d4c967035dd8da344e0f39fa` | audio / 17_orc_atk_sword_1.mp3 |
| ALI-0273 | `rusted_kingdoms/assets/audio/sfx/dungeon/17_orc_atk_sword_2.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/dungeon/17_orc_atk_sword_2.mp3` | `9da550beb72ce3dfc36aad65ede530f96fceb9699134bc9136113e71d9a70d28` | audio / 17_orc_atk_sword_2.mp3 |
| ALI-0274 | `rusted_kingdoms/assets/audio/sfx/dungeon/17_orc_atk_sword_3.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/dungeon/17_orc_atk_sword_3.mp3` | `050015f67bdda5fa80d7bc9bddaabcb1b03a65978a2623031de1873dad543dc0` | audio / 17_orc_atk_sword_3.mp3 |
| ALI-0275 | `rusted_kingdoms/assets/audio/sfx/dungeon/21_orc_damage_1.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/dungeon/21_orc_damage_1.mp3` | `d5ad0859db519f8082a6febfae034ddca157c0953023d2b2abf1e46ffbfad6d5` | audio / 21_orc_damage_1.mp3 |
| ALI-0276 | `rusted_kingdoms/assets/audio/sfx/dungeon/21_orc_damage_2.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/dungeon/21_orc_damage_2.mp3` | `3d78620a8abbfa8c1793112974c7c50d6886c7d9353a15675daf0044aa6c6a38` | audio / 21_orc_damage_2.mp3 |
| ALI-0277 | `rusted_kingdoms/assets/audio/sfx/dungeon/21_orc_damage_3.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/dungeon/21_orc_damage_3.mp3` | `cedd6ccf19717317e62f24ed613e62fd9a8f016846024c7c40f163e548b531da` | audio / 21_orc_damage_3.mp3 |
| ALI-0278 | `rusted_kingdoms/assets/audio/sfx/dungeon/26_sword_hit_1.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/dungeon/26_sword_hit_1.mp3` | `796d8e12c52e425afc05bf92924ab6fc8cdaf87c63f0ceafa33a31c90d87aca6` | audio / 26_sword_hit_1.mp3 |
| ALI-0279 | `rusted_kingdoms/assets/audio/sfx/dungeon/26_sword_hit_2.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/dungeon/26_sword_hit_2.mp3` | `a47961f90a829147d717679b69ef4546e3aecd32db0128f042ec53590b19297f` | audio / 26_sword_hit_2.mp3 |
| ALI-0280 | `rusted_kingdoms/assets/audio/sfx/dungeon/26_sword_hit_3.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/dungeon/26_sword_hit_3.mp3` | `7a03a1322dd2372e566714c457c87ca65559e59b682267e52fefa128de9a7501` | audio / 26_sword_hit_3.mp3 |
| ALI-0281 | `rusted_kingdoms/assets/audio/sfx/dungeon/27_sword_miss_1.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/dungeon/27_sword_miss_1.mp3` | `4ff1929707c0a74f4fb4921f9bad0580a01302064fafcb67298f5e9852c4069d` | audio / 27_sword_miss_1.mp3 |
| ALI-0282 | `rusted_kingdoms/assets/audio/sfx/dungeon/27_sword_miss_2.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/dungeon/27_sword_miss_2.mp3` | `8a9eab6d678e10c2f68a693626007d0d515702bf2d065d3af80ad10786e60b3c` | audio / 27_sword_miss_2.mp3 |
| ALI-0283 | `rusted_kingdoms/assets/audio/sfx/dungeon/27_sword_miss_3.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/dungeon/27_sword_miss_3.mp3` | `941b656fa523c69a97205e5af95b0c2f57c5b6acfc1cd9904781f9b6f73bbfe0` | audio / 27_sword_miss_3.mp3 |
| ALI-0284 | `rusted_kingdoms/assets/audio/sfx/player_movement/88_Teleport_02.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/player_movement/88_Teleport_02.mp3` | `c378c2e7e5c88eafc48f2160fb0eaeeb209170eb66d590ec2dfb6f11220001db` | audio / 88_Teleport_02.mp3 |
| ALI-0285 | `rusted_kingdoms/assets/audio/sfx/ui_menu/001_Hover_01.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/ui_menu/001_Hover_01.mp3` | `2458f348156391f55587d16f1185fa3e2452392730168a9d839bffd4242a3e13` | audio / 001_Hover_01.mp3 |
| ALI-0286 | `rusted_kingdoms/assets/audio/sfx/ui_menu/070_Equip_10.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/ui_menu/070_Equip_10.mp3` | `b19354000d238b29bb534cae07fdd9c6a22688470ed3d520aceceb28a6f601bf` | audio / 070_Equip_10.mp3 |
| ALI-0287 | `rusted_kingdoms/assets/audio/sfx/ui_menu/071_Unequip_01.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/ui_menu/071_Unequip_01.mp3` | `7a3a6d9cf0da74a8e0505ab1935aad69a515112a60b99540ba94cd9ba9372b01` | audio / 071_Unequip_01.mp3 |
| ALI-0288 | `rusted_kingdoms/assets/audio/sfx/ui_menu/079_Buy_sell_01.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/ui_menu/079_Buy_sell_01.mp3` | `c5d7d4a95da9b9210e8599b1ccbe765ee71721f689deeb6129a88328d2c61f4a` | audio / 079_Buy_sell_01.mp3 |
| ALI-0289 | `rusted_kingdoms/assets/audio/sfx/ui_menu/092_Pause_04.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/ui_menu/092_Pause_04.mp3` | `b44d11dbd4884a79e17f715f9fd2f11e6c7e270324907df9a69cc36489adcfb0` | audio / 092_Pause_04.mp3 |
| ALI-0290 | `rusted_kingdoms/assets/audio/sfx/ui_menu/098_Unpause_04.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/sfx/ui_menu/098_Unpause_04.mp3` | `13ec3ea44ce366c3f60fd8eaf4442cf4a6ac54d03306e11e866bfa25f749f422` | audio / 098_Unpause_04.mp3 |
| ALI-0291 | `rusted_kingdoms/assets/fonts/Philosopher-OFL.txt` | `assets/scenarios/rusted_kingdoms/media/fonts/Philosopher-OFL.txt` | `a812c3a94bd45b79bb8eb94a349f72a66b859f08dbdbb49777e9e9e81b8c9575` | text / Philosopher-OFL.txt |
| ALI-0292 | `rusted_kingdoms/assets/images/aric_profile.png` | `assets/scenarios/rusted_kingdoms/media/images/aric_profile.png` | `c0038e75a56e3d533ca527f2a835c9c6f04cebc1a1335e81c22d88e3d04d15ef` | image / aric_profile.png |
| ALI-0293 | `rusted_kingdoms/assets/images/battle_bg/zone10-bg-1280x468.webp` | `assets/scenarios/rusted_kingdoms/media/images/battle_bg/zone10-bg-1280x468.webp` | `e2e09aa1e3a208b4ed76c64e02b47249885565781d75e78df946a0b3c722579e` | image / zone10-bg-1280x468.webp |
| ALI-0294 | `rusted_kingdoms/assets/images/battle_bg/zone2-bg-1280x468.webp` | `assets/scenarios/rusted_kingdoms/media/images/battle_bg/zone2-bg-1280x468.webp` | `7dbd4c54173d7a9e5e0d945473bbd4d2d400aaec868136a77acc9e80656af9fc` | image / zone2-bg-1280x468.webp |
| ALI-0295 | `rusted_kingdoms/assets/images/battle_bg/zone3-bg-1280x468.webp` | `assets/scenarios/rusted_kingdoms/media/images/battle_bg/zone3-bg-1280x468.webp` | `7ce97bd987b3755b281968891cb3ad5ea9090534a2fa18dbb2a834405a5c0b0e` | image / zone3-bg-1280x468.webp |
| ALI-0296 | `rusted_kingdoms/assets/images/battle_bg/zone4-bg-1280x468.webp` | `assets/scenarios/rusted_kingdoms/media/images/battle_bg/zone4-bg-1280x468.webp` | `e63b4cfcf163ba43638346531b018f9b2f919785c7336263e36c1c12a20d9237` | image / zone4-bg-1280x468.webp |
| ALI-0297 | `rusted_kingdoms/assets/images/battle_bg/zone4-courtyard-bg-1280x468.webp` | `assets/scenarios/rusted_kingdoms/media/images/battle_bg/zone4-courtyard-bg-1280x468.webp` | `a1164c57d7929b326d142e00c0fb88fa6cf9c9ff576f84af2b2c453bfd02cad7` | image / zone4-courtyard-bg-1280x468.webp |
| ALI-0298 | `rusted_kingdoms/assets/images/battle_bg/zone4-gate-bg-1280x468.webp` | `assets/scenarios/rusted_kingdoms/media/images/battle_bg/zone4-gate-bg-1280x468.webp` | `74724c6a32dfa828f1d0dfc75fac99777ffbafdd788f140ec5c7b8e55da956f6` | image / zone4-gate-bg-1280x468.webp |
| ALI-0299 | `rusted_kingdoms/assets/images/battle_bg/zone5-bg-1280x468.webp` | `assets/scenarios/rusted_kingdoms/media/images/battle_bg/zone5-bg-1280x468.webp` | `b718825729c2652cc6261ae56bedb239efba92f0d2d99a7de9c3ca48e107e33e` | image / zone5-bg-1280x468.webp |
| ALI-0300 | `rusted_kingdoms/assets/images/battle_bg/zone6-bg-1280x468.webp` | `assets/scenarios/rusted_kingdoms/media/images/battle_bg/zone6-bg-1280x468.webp` | `aaaad0bf6f6a81b5c4f987799c2e12704464adc683932eaf72e40356f23e62bc` | image / zone6-bg-1280x468.webp |
| ALI-0301 | `rusted_kingdoms/assets/images/battle_bg/zone7-bg-1280x468.webp` | `assets/scenarios/rusted_kingdoms/media/images/battle_bg/zone7-bg-1280x468.webp` | `3a7daf03a7ceddbaaa8ecfc2681f24b1b5c40f5d78a321a2f9cd1f9e6bd374bc` | image / zone7-bg-1280x468.webp |
| ALI-0302 | `rusted_kingdoms/assets/images/battle_bg/zone8-bg-1280x468.webp` | `assets/scenarios/rusted_kingdoms/media/images/battle_bg/zone8-bg-1280x468.webp` | `bbe33bb8441644ba52ec83a3939159ed79b626fcb451e364b74147f6ce6d2577` | image / zone8-bg-1280x468.webp |
| ALI-0303 | `rusted_kingdoms/assets/images/battle_bg/zone9-bg-1280x468.webp` | `assets/scenarios/rusted_kingdoms/media/images/battle_bg/zone9-bg-1280x468.webp` | `a8088b90c44ed25d2bbe297472b5f5de11431907b4895b0cb49fef702c89ae6b` | image / zone9-bg-1280x468.webp |
| ALI-0304 | `rusted_kingdoms/assets/images/elise_profile.png` | `assets/scenarios/rusted_kingdoms/media/images/elise_profile.png` | `78952d98cf772a1c15fc0f72259418e46490efa737fa4f378081514f3a89f9e2` | image / elise_profile.png |
| ALI-0305 | `rusted_kingdoms/assets/images/icons/arrow-head-right.webp` | `assets/scenarios/rusted_kingdoms/media/images/icons/arrow-head-right.webp` | `0792fe370ece7ccd57d7ce4694f39cc892fd5235c589c485c5ca76f3b1fad901` | image / arrow-head-right.webp |
| ALI-0306 | `rusted_kingdoms/assets/images/icons/lock-locked-red-small.webp` | `assets/scenarios/rusted_kingdoms/media/images/icons/lock-locked-red-small.webp` | `4fd8734c6b02dac562edc68d5b4b98303490b5b65343d4c29fad35fe00f7a6db` | image / lock-locked-red-small.webp |
| ALI-0307 | `rusted_kingdoms/assets/images/icons/lock-unlocked-green-small.webp` | `assets/scenarios/rusted_kingdoms/media/images/icons/lock-unlocked-green-small.webp` | `15f90477f872524b2f27da0a3cac2c38e723f948b363dd3138c6633f6d5bab35` | image / lock-unlocked-green-small.webp |
| ALI-0308 | `rusted_kingdoms/assets/images/icons/lock-unlocked-yellow-small.webp` | `assets/scenarios/rusted_kingdoms/media/images/icons/lock-unlocked-yellow-small.webp` | `b6eec690619ca0e9b7b401238db0263c458e2e773eb599c511fff924a588d679` | image / lock-unlocked-yellow-small.webp |
| ALI-0309 | `rusted_kingdoms/assets/images/jep_profile.png` | `assets/scenarios/rusted_kingdoms/media/images/jep_profile.png` | `4a2ded072e603f748e3ed82171ecee88b148df8299049040b57cde65d651c849` | image / jep_profile.png |
| ALI-0310 | `rusted_kingdoms/assets/images/kael_profile.png` | `assets/scenarios/rusted_kingdoms/media/images/kael_profile.png` | `89228d35082ff5c0a4ac7e53cf58d95f835e255161e3af4836bff37938e344af` | image / kael_profile.png |
| ALI-0311 | `rusted_kingdoms/assets/images/party_portraits_large/aric_status_portrait.webp` | `assets/scenarios/rusted_kingdoms/media/images/party_portraits_large/aric_status_portrait.webp` | `24fda4e388bf96e467fde0ace40246ed0547c29e85f8860cad4a2de2d0e64979` | image / aric_status_portrait.webp |
| ALI-0312 | `rusted_kingdoms/assets/images/party_portraits_large/elise_status_portrait.webp` | `assets/scenarios/rusted_kingdoms/media/images/party_portraits_large/elise_status_portrait.webp` | `ae5f16ebcc7f585f6adec7e963b21e08b9fa05e3408eebe89796e0617c08075c` | image / elise_status_portrait.webp |
| ALI-0313 | `rusted_kingdoms/assets/images/party_portraits_large/jep_status_portrait.webp` | `assets/scenarios/rusted_kingdoms/media/images/party_portraits_large/jep_status_portrait.webp` | `286f5663c5486fba9a6b6e48306054318f87d86c28d07d77d2859092783a8bd0` | image / jep_status_portrait.webp |
| ALI-0314 | `rusted_kingdoms/assets/images/party_portraits_large/kael_status_portrait.webp` | `assets/scenarios/rusted_kingdoms/media/images/party_portraits_large/kael_status_portrait.webp` | `52461cf0b5ff3750f3931f4f6fde411d1fec507c4ab6ee64991eebd2f36f643a` | image / kael_status_portrait.webp |
| ALI-0315 | `rusted_kingdoms/assets/images/party_portraits_large/reiya_status_portrait.webp` | `assets/scenarios/rusted_kingdoms/media/images/party_portraits_large/reiya_status_portrait.webp` | `cda93192163de879ea0c2da0847321a8e8bb55a9f63c9675c040307178f2f0d2` | image / reiya_status_portrait.webp |
| ALI-0316 | `rusted_kingdoms/assets/images/reiya_profile.png` | `assets/scenarios/rusted_kingdoms/media/images/reiya_profile.png` | `4823944bd43db768fe3169c9315b63749b61d4826ebbcec648d31804dbac6fe2` | image / reiya_profile.png |
| ALI-0317 | `rusted_kingdoms/assets/images/title_bg/title_lost_flame.webp` | `assets/scenarios/rusted_kingdoms/media/images/title_bg/title_lost_flame.webp` | `c36fc2defc4ddee6ba18e53a61c40b840713219f196775618d46ac344723a9bb` | image / title_lost_flame.webp |
| ALI-0318 | `rusted_kingdoms/assets/maps/rusted_kingdoms.tiled-project` | `assets/scenarios/rusted_kingdoms/media/maps/rusted_kingdoms.tiled-project` | `7ff120b8a32974ec9a29c77a2972e85e5a6e8e9bd624b55287e634c8831d219f` | tiled project / rusted_kingdoms.tiled-project |
| ALI-0319 | `rusted_kingdoms/assets/maps/town_01_ardel_inn_01.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/town_01_ardel_inn_01.tmx` | `e0c3c7b4419937b53fd3d0075e1182334a4dfe06e61e1e2bf2d89b29449ba13f` | map XML / town_01_ardel_inn_01.tmx |
| ALI-0320 | `rusted_kingdoms/assets/maps/town_01_ardel_shop_01.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/town_01_ardel_shop_01.tmx` | `eb0562a4dfc4e41f5464a796bf0e083a9798cd0e25faab1a2200fe6586bb4ee3` | map XML / town_01_ardel_shop_01.tmx |
| ALI-0321 | `rusted_kingdoms/assets/maps/town_01_ardel_shrine.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/town_01_ardel_shrine.tmx` | `de6cbaa6b6b1673711d34b39096d42308d6a97b36061e27dc1755f5cd8aaa268` | map XML / town_01_ardel_shrine.tmx |
| ALI-0322 | `rusted_kingdoms/assets/maps/town_02_millhaven.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/town_02_millhaven.tmx` | `4fb19608257767c305b6172bb801113183580725e85495351a3a5e142d3dcdeb` | map XML / town_02_millhaven.tmx |
| ALI-0323 | `rusted_kingdoms/assets/maps/town_02_millhaven_inn.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/town_02_millhaven_inn.tmx` | `e2c889bbd2c83f99066ef581802b2798e7bf656b5decf32c828ef0dee6670e95` | map XML / town_02_millhaven_inn.tmx |
| ALI-0324 | `rusted_kingdoms/assets/maps/town_02_millhaven_mill.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/town_02_millhaven_mill.tmx` | `27a76d06488ad7691e263ee8dafdd714b053e81f0e844bf464bf081832903387` | map XML / town_02_millhaven_mill.tmx |
| ALI-0325 | `rusted_kingdoms/assets/maps/town_02_millhaven_shop.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/town_02_millhaven_shop.tmx` | `ecb65bbf97bbf105a5c19ee87c73bf85d408ced03fa03e965597810ee48765c6` | map XML / town_02_millhaven_shop.tmx |
| ALI-0326 | `rusted_kingdoms/assets/maps/town_03_ruinwatch_inn.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/town_03_ruinwatch_inn.tmx` | `a35155a33c5fe704683e73d60bc46707a520df8525e9dfffb3b279f877e87389` | map XML / town_03_ruinwatch_inn.tmx |
| ALI-0327 | `rusted_kingdoms/assets/maps/town_03_ruinwatch_shop.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/town_03_ruinwatch_shop.tmx` | `6ae626b2630f8ea0746b4bc8ebf4f954fceb05a499008ee17e5c49272775053e` | map XML / town_03_ruinwatch_shop.tmx |
| ALI-0328 | `rusted_kingdoms/assets/maps/town_04_frostholm_shop.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/town_04_frostholm_shop.tmx` | `1bd6b1e56bd71418048f8ede2444d110bc8278753c0fffda9b9ed14f40285169` | map XML / town_04_frostholm_shop.tmx |
| ALI-0329 | `rusted_kingdoms/assets/maps/town_05_ashenveil_inn.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/town_05_ashenveil_inn.tmx` | `0ea3c869439f7025612318e7dd442f6c4a9b5810e1f7015709df82aae274ff07` | map XML / town_05_ashenveil_inn.tmx |
| ALI-0330 | `rusted_kingdoms/assets/maps/town_05_ashenveil_shop.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/town_05_ashenveil_shop.tmx` | `c339a21450af40256ef4d442d61cd1797fce134b795dd41ba325cfaa433e63a8` | map XML / town_05_ashenveil_shop.tmx |
| ALI-0331 | `rusted_kingdoms/assets/maps/zone_02_open_plains.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/zone_02_open_plains.tmx` | `1b0f4b7391f8b15ab82bd289e12dea81ff1926625e6e39d8e62ac5868f11e884` | map XML / zone_02_open_plains.tmx |
| ALI-0332 | `rusted_kingdoms/assets/maps/zone_02_open_plains_cave_01.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/zone_02_open_plains_cave_01.tmx` | `6463f0a289c70384799326808d24ccdb3a3a71e991cd187586bb88e1c6172e21` | map XML / zone_02_open_plains_cave_01.tmx |
| ALI-0333 | `rusted_kingdoms/assets/maps/zone_02_open_plains_cave_02.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/zone_02_open_plains_cave_02.tmx` | `6c7530446fe4b51f7ba4b92657b72e7a25daac532702355c8385508d9ec1825b` | map XML / zone_02_open_plains_cave_02.tmx |
| ALI-0334 | `rusted_kingdoms/assets/maps/zone_04_ancient_ruins_01_gate.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/zone_04_ancient_ruins_01_gate.tmx` | `61f721b201e9b23c84e1e33da56a70c10c9788d5b0a274a5acd8cbab703fbb43` | map XML / zone_04_ancient_ruins_01_gate.tmx |
| ALI-0335 | `rusted_kingdoms/assets/maps/zone_04_ancient_ruins_02_courtyard.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/zone_04_ancient_ruins_02_courtyard.tmx` | `56f8398e4e0858f80c6601378d2840ddfd63e7ad1cf7324c891f539c0cbf06aa` | map XML / zone_04_ancient_ruins_02_courtyard.tmx |
| ALI-0336 | `rusted_kingdoms/assets/maps/zone_05_mountain_foothills_02.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/zone_05_mountain_foothills_02.tmx` | `2a06eff71226b82844a72bfec9e893670415912708241fabed8b5989ad278bd1` | map XML / zone_05_mountain_foothills_02.tmx |
| ALI-0337 | `rusted_kingdoms/assets/maps/zone_06_mountain_pass_01.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/zone_06_mountain_pass_01.tmx` | `9662cf773403c3d7c76d8e9c8aa1dd687f8894f1be432b618aa84e8cc95d0bdc` | map XML / zone_06_mountain_pass_01.tmx |
| ALI-0338 | `rusted_kingdoms/assets/maps/zone_06_mountain_pass_02.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/zone_06_mountain_pass_02.tmx` | `38b38f293daa1f5ddfb7f1c45e41e59bdc8b9c84ed9d67fa9711eb0aafd366c7` | map XML / zone_06_mountain_pass_02.tmx |
| ALI-0339 | `rusted_kingdoms/assets/maps/zone_07_sunken_cave.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/zone_07_sunken_cave.tmx` | `da4103d15ebc4127bfbe2732c73ff6154272c8737b5cdadea3d43a5381cc4abf` | map XML / zone_07_sunken_cave.tmx |
| ALI-0340 | `rusted_kingdoms/assets/maps/zone_08_corrupted_forest.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/zone_08_corrupted_forest.tmx` | `9463045822c1b1ad304683e06df8285f567c4556b5d1253f962a276893bfba6f` | map XML / zone_08_corrupted_forest.tmx |
| ALI-0341 | `rusted_kingdoms/assets/sprites/enemies/alien_invader_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/alien_invader_base.png` | `d7518622d5f8ddd30b26d48e3eda9ca7a2bf60d28f0edf4552314b6fe0a1826b` | image / alien_invader_base.png |
| ALI-0342 | `rusted_kingdoms/assets/sprites/enemies/alien_invader_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/alien_invader_base.tsx` | `f7c23b2227f8918e40f6d4a012972085daaa68bc264af7007c8641365727bc8b` | tileset XML / alien_invader_base.tsx |
| ALI-0343 | `rusted_kingdoms/assets/sprites/enemies/alien_invader_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/alien_invader_base_battle.png` | `51f2e59a6aff4418f5b6cccd6640d7a80afb2863f5ff37efe55d24fe8708fc6c` | image / alien_invader_base_battle.png |
| ALI-0344 | `rusted_kingdoms/assets/sprites/enemies/alien_invader_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/alien_invader_base_battle.tsx` | `bfedf24cca000ef178603947df29ab16b1d82cbad7352c585199d750fea9d450` | tileset XML / alien_invader_base_battle.tsx |
| ALI-0345 | `rusted_kingdoms/assets/sprites/enemies/alien_invader_night_alien.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/alien_invader_night_alien.png` | `ad4e9611b76f2b740571d11c7ad31265869b756d4e75ecf7e9b3860227e60bb7` | image / alien_invader_night_alien.png |
| ALI-0346 | `rusted_kingdoms/assets/sprites/enemies/alien_invader_night_alien.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/alien_invader_night_alien.tsx` | `42a6259c74deaf5df2f42daacaa9e0454ecafd3c6f65b111d2221666a7571980` | tileset XML / alien_invader_night_alien.tsx |
| ALI-0347 | `rusted_kingdoms/assets/sprites/enemies/alien_invader_night_alien_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/alien_invader_night_alien_battle.png` | `674ddf65551497c463cbc48eeca2cd9502762aa42e39ebf531a930024e8df195` | image / alien_invader_night_alien_battle.png |
| ALI-0348 | `rusted_kingdoms/assets/sprites/enemies/alien_invader_night_alien_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/alien_invader_night_alien_battle.tsx` | `1aea7bcfa22a54b42964ed6b274cd343ae0dc2e4646448a85ca22377a44e0806` | tileset XML / alien_invader_night_alien_battle.tsx |
| ALI-0349 | `rusted_kingdoms/assets/sprites/enemies/alien_invader_red_crystal_invader.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/alien_invader_red_crystal_invader.png` | `5f848f10a71fe09dc2988ccc319e6bf14428d1515e4831798dd137e8bd5803f0` | image / alien_invader_red_crystal_invader.png |
| ALI-0350 | `rusted_kingdoms/assets/sprites/enemies/alien_invader_red_crystal_invader.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/alien_invader_red_crystal_invader.tsx` | `f1379985d4dbda691d80c15581fedd05626874e67a34208ee7dc0528a4b33169` | tileset XML / alien_invader_red_crystal_invader.tsx |
| ALI-0351 | `rusted_kingdoms/assets/sprites/enemies/alien_invader_red_crystal_invader_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/alien_invader_red_crystal_invader_battle.png` | `ee004e9914cc8b2cc6d13cc9ba64097d52b4d7693c9c6d5bf0fdb70197cea2a8` | image / alien_invader_red_crystal_invader_battle.png |
| ALI-0352 | `rusted_kingdoms/assets/sprites/enemies/alien_invader_red_crystal_invader_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/alien_invader_red_crystal_invader_battle.tsx` | `81f1e4bcb9ac5ca00c02c4c98b369d97b28b71d119bbd2024372edb73f641229` | tileset XML / alien_invader_red_crystal_invader_battle.tsx |
| ALI-0353 | `rusted_kingdoms/assets/sprites/enemies/bat_demon_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/bat_demon_base.png` | `1c09853223cb14661141e1e1073b5b9072abbf1bbe231475ec2fef5057b91e20` | image / bat_demon_base.png |
| ALI-0354 | `rusted_kingdoms/assets/sprites/enemies/bat_demon_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/bat_demon_base.tsx` | `597ef5b1fd7cf15257e4c5708f86fdaa42fe6e52d0a23227182375f357f22edd` | tileset XML / bat_demon_base.tsx |
| ALI-0355 | `rusted_kingdoms/assets/sprites/enemies/bat_demon_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/bat_demon_base_battle.png` | `a86307d64f8052448b3cc8a961cd0ce2a13b43023e6299db47a01541fa6e0c1f` | image / bat_demon_base_battle.png |
| ALI-0356 | `rusted_kingdoms/assets/sprites/enemies/bat_demon_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/bat_demon_base_battle.tsx` | `5126802728c1370f2fc628aff6ed5d6a1c16ddbd53bd592ec07d23304c95ed90` | tileset XML / bat_demon_base_battle.tsx |
| ALI-0357 | `rusted_kingdoms/assets/sprites/enemies/bat_demon_night_fiend.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/bat_demon_night_fiend.png` | `22b7321c9557cf0bbcdf3b71d51ddf7af25950e8bad21e319c68b9a8d97550b9` | image / bat_demon_night_fiend.png |
| ALI-0358 | `rusted_kingdoms/assets/sprites/enemies/bat_demon_night_fiend.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/bat_demon_night_fiend.tsx` | `3e8e11e0b316d123c692fb4c3d6b7d1549133396ea01e96893e5ee885488267b` | tileset XML / bat_demon_night_fiend.tsx |
| ALI-0359 | `rusted_kingdoms/assets/sprites/enemies/bat_demon_night_fiend_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/bat_demon_night_fiend_battle.png` | `4afae9e13e01ffdfae8dfcaad1aa55012b65a63d788297a1a1cd550550d46f27` | image / bat_demon_night_fiend_battle.png |
| ALI-0360 | `rusted_kingdoms/assets/sprites/enemies/bat_demon_night_fiend_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/bat_demon_night_fiend_battle.tsx` | `f7130b5085bd8022a5f076d0ad4a5c5a10a3e91999e768091e2dcd38350707a9` | tileset XML / bat_demon_night_fiend_battle.tsx |
| ALI-0361 | `rusted_kingdoms/assets/sprites/enemies/bat_demon_red_wing_fiend.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/bat_demon_red_wing_fiend.png` | `2d4640f321369b55216daeadb2764be7adc95eeeffb96daee9dfacd07b784605` | image / bat_demon_red_wing_fiend.png |
| ALI-0362 | `rusted_kingdoms/assets/sprites/enemies/bat_demon_red_wing_fiend.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/bat_demon_red_wing_fiend.tsx` | `e23087970f6c3812c0119f6853e35027113d99d39322277d0408d4389be2c424` | tileset XML / bat_demon_red_wing_fiend.tsx |
| ALI-0363 | `rusted_kingdoms/assets/sprites/enemies/bat_demon_red_wing_fiend_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/bat_demon_red_wing_fiend_battle.png` | `4edb8f954513f7cab859de3f9b1daf987e643ce82849abb88d319857e4a443cf` | image / bat_demon_red_wing_fiend_battle.png |
| ALI-0364 | `rusted_kingdoms/assets/sprites/enemies/bat_demon_red_wing_fiend_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/bat_demon_red_wing_fiend_battle.tsx` | `ef970f1bf7c1e549a56181a69b9463482ea7fa092b13bd1af204b694a766e172` | tileset XML / bat_demon_red_wing_fiend_battle.tsx |
| ALI-0365 | `rusted_kingdoms/assets/sprites/enemies/boarman_berserker_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/boarman_berserker_base.png` | `9c4483aa8393f87fadf60ab8479118b76f7bbb0a2a868adf678d01078655c1be` | image / boarman_berserker_base.png |
| ALI-0366 | `rusted_kingdoms/assets/sprites/enemies/boarman_berserker_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/boarman_berserker_base.tsx` | `68e400a79db5b9b3b7a018b27313ae8c8eb8f3844d12431071f77cab5c4ef597` | tileset XML / boarman_berserker_base.tsx |
| ALI-0367 | `rusted_kingdoms/assets/sprites/enemies/boarman_berserker_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/boarman_berserker_base_battle.png` | `145409c90daa07e4f48867904ebff6898260cb44bd8b2f29e8ea6f5663ba21dc` | image / boarman_berserker_base_battle.png |
| ALI-0368 | `rusted_kingdoms/assets/sprites/enemies/boarman_berserker_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/boarman_berserker_base_battle.tsx` | `a64ed7b55c8257b817badd9372d1631149f0085403ed392e114a6cf7a1379ed4` | tileset XML / boarman_berserker_base_battle.tsx |
| ALI-0369 | `rusted_kingdoms/assets/sprites/enemies/boarman_berserker_blackhide_raider.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/boarman_berserker_blackhide_raider.png` | `b2217490751c2f9733bbc605e62c71acd6e2bc5cf22e09cf43af250eb4915969` | image / boarman_berserker_blackhide_raider.png |
| ALI-0370 | `rusted_kingdoms/assets/sprites/enemies/boarman_berserker_blackhide_raider.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/boarman_berserker_blackhide_raider.tsx` | `d80ea0731a369986c0f11180133a69f8fecdc3f76bddf1c69a09309402d24392` | tileset XML / boarman_berserker_blackhide_raider.tsx |
| ALI-0371 | `rusted_kingdoms/assets/sprites/enemies/boarman_berserker_blackhide_raider_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/boarman_berserker_blackhide_raider_battle.png` | `8cbec136b5074bffca708f0576d0aa14add46e1ec2bb77f883647c70a9ed0c07` | image / boarman_berserker_blackhide_raider_battle.png |
| ALI-0372 | `rusted_kingdoms/assets/sprites/enemies/boarman_berserker_blackhide_raider_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/boarman_berserker_blackhide_raider_battle.tsx` | `296305f64db03a3fee63478de93cbb02fd1d45635242f11a29d0ff5a71102db2` | tileset XML / boarman_berserker_blackhide_raider_battle.tsx |
| ALI-0373 | `rusted_kingdoms/assets/sprites/enemies/boarman_berserker_tusk_charger.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/boarman_berserker_tusk_charger.png` | `c88a0763f51ea7afd95a0ca6222be1b9a05a148e7552e2435ce2eb5d211de4ec` | image / boarman_berserker_tusk_charger.png |
| ALI-0374 | `rusted_kingdoms/assets/sprites/enemies/boarman_berserker_tusk_charger.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/boarman_berserker_tusk_charger.tsx` | `d0137044d7be9fb56e4fc7f7c2f94072170976dbed6cd6e47b6df6c802ce9b35` | tileset XML / boarman_berserker_tusk_charger.tsx |
| ALI-0375 | `rusted_kingdoms/assets/sprites/enemies/boarman_berserker_tusk_charger_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/boarman_berserker_tusk_charger_battle.png` | `63e396b8cb371d8a38137a294331ca87f00f0b23578212af947b3acde3e1527f` | image / boarman_berserker_tusk_charger_battle.png |
| ALI-0376 | `rusted_kingdoms/assets/sprites/enemies/boarman_berserker_tusk_charger_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/boarman_berserker_tusk_charger_battle.tsx` | `669dca2f54eff23355c0af820c8e7ccfe2493ed28d3bb8a2864a6f4522cac077` | tileset XML / boarman_berserker_tusk_charger_battle.tsx |
| ALI-0377 | `rusted_kingdoms/assets/sprites/enemies/dark_fairy_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/dark_fairy_base.png` | `223d638dc5e968e1e3758fbb7607dcdf92d90f387552addb75bfddb622d6dd22` | image / dark_fairy_base.png |
| ALI-0378 | `rusted_kingdoms/assets/sprites/enemies/dark_fairy_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/dark_fairy_base.tsx` | `cd50eb74e92ea8b2bf75646cdc1906a456991357f98e3aace65f9b0271aa6551` | tileset XML / dark_fairy_base.tsx |
| ALI-0379 | `rusted_kingdoms/assets/sprites/enemies/dark_fairy_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/dark_fairy_base_battle.png` | `b940c6c7238b26078d975cf75ec2e04cbebef86a3d883718eaab2225c61cb6f8` | image / dark_fairy_base_battle.png |
| ALI-0380 | `rusted_kingdoms/assets/sprites/enemies/dark_fairy_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/dark_fairy_base_battle.tsx` | `7ce3d721f291065cb87ca50d034479d757e9d96c1aa7ceeae3a0ab0c29188cbd` | tileset XML / dark_fairy_base_battle.tsx |
| ALI-0381 | `rusted_kingdoms/assets/sprites/enemies/dark_fairy_purple_hex_fairy.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/dark_fairy_purple_hex_fairy.png` | `58faaaecc6708f6f9c0a503e06a7a084ab2b08864ef601367274953e63a38be2` | image / dark_fairy_purple_hex_fairy.png |
| ALI-0382 | `rusted_kingdoms/assets/sprites/enemies/dark_fairy_purple_hex_fairy.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/dark_fairy_purple_hex_fairy.tsx` | `e0d3addce62c5ba34f81db4e28e6280c2dd53cfd8ba741a5dfdde2d9407faac3` | tileset XML / dark_fairy_purple_hex_fairy.tsx |
| ALI-0383 | `rusted_kingdoms/assets/sprites/enemies/dark_fairy_purple_hex_fairy_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/dark_fairy_purple_hex_fairy_battle.png` | `8752a98be8611e9a6dc1804553449e165e50be6dc3765430a2dd0c15ccbb1837` | image / dark_fairy_purple_hex_fairy_battle.png |
| ALI-0384 | `rusted_kingdoms/assets/sprites/enemies/dark_fairy_purple_hex_fairy_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/dark_fairy_purple_hex_fairy_battle.tsx` | `c09f6a936c2366fb324f35e681cc7beeb451cdd989faabd6ff5be4420c2c4ae3` | tileset XML / dark_fairy_purple_hex_fairy_battle.tsx |
| ALI-0385 | `rusted_kingdoms/assets/sprites/enemies/dark_fairy_redcap_fairy.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/dark_fairy_redcap_fairy.png` | `985bf18f678cf09fdfb016dfc16ec29b3d1ce16d60e92cd83410472e824ae820` | image / dark_fairy_redcap_fairy.png |
| ALI-0386 | `rusted_kingdoms/assets/sprites/enemies/dark_fairy_redcap_fairy.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/dark_fairy_redcap_fairy.tsx` | `091e16ce7dea831a60880441a9e32879877e1d9b69dbe2cebff6042e61655c45` | tileset XML / dark_fairy_redcap_fairy.tsx |
| ALI-0387 | `rusted_kingdoms/assets/sprites/enemies/dark_fairy_redcap_fairy_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/dark_fairy_redcap_fairy_battle.png` | `686f41fbc9d4e35b4086b00a2bdd2296a8b1851d13036c97899561bfa846cb88` | image / dark_fairy_redcap_fairy_battle.png |
| ALI-0388 | `rusted_kingdoms/assets/sprites/enemies/dark_fairy_redcap_fairy_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/dark_fairy_redcap_fairy_battle.tsx` | `15b5882f3ffa5665c0d07a62e3404589f521d43ba801cf24b007dc58989dcbb1` | tileset XML / dark_fairy_redcap_fairy_battle.tsx |
| ALI-0389 | `rusted_kingdoms/assets/sprites/enemies/fallen_angel_ash_wing.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/fallen_angel_ash_wing.png` | `6589ea3fce52cc630f129231f623fddb5bc20a614895b7d4aa5d2760183edd65` | image / fallen_angel_ash_wing.png |
| ALI-0390 | `rusted_kingdoms/assets/sprites/enemies/fallen_angel_ash_wing.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/fallen_angel_ash_wing.tsx` | `8df770258cbd80aa2fa1fa0dffa42be052ea81f7f23aeade7323afccbd64b686` | tileset XML / fallen_angel_ash_wing.tsx |
| ALI-0391 | `rusted_kingdoms/assets/sprites/enemies/fallen_angel_ash_wing_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/fallen_angel_ash_wing_battle.png` | `da2c6c5472c81a3458227191a05729582f5bc64f20a06e165e193e09197797c2` | image / fallen_angel_ash_wing_battle.png |
| ALI-0392 | `rusted_kingdoms/assets/sprites/enemies/fallen_angel_ash_wing_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/fallen_angel_ash_wing_battle.tsx` | `4e1de359ff8bed3f8285190409c515ede1266ec9d95c5bef36ef91b69e436255` | tileset XML / fallen_angel_ash_wing_battle.tsx |
| ALI-0393 | `rusted_kingdoms/assets/sprites/enemies/fallen_angel_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/fallen_angel_base.png` | `58dff74191a8c82b9385b7e04c8aa40fe34143a6fcd80bcef122a33262835d34` | image / fallen_angel_base.png |
| ALI-0394 | `rusted_kingdoms/assets/sprites/enemies/fallen_angel_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/fallen_angel_base.tsx` | `c7505747cb9961fc2098a8aeee7edc90ee76b92b02ffff8affabbb9e2e166d11` | tileset XML / fallen_angel_base.tsx |
| ALI-0395 | `rusted_kingdoms/assets/sprites/enemies/fallen_angel_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/fallen_angel_base_battle.png` | `e5246c8855d76838b9c0c599f148869cb92f1780884cbe6a4686cac98cf88112` | image / fallen_angel_base_battle.png |
| ALI-0396 | `rusted_kingdoms/assets/sprites/enemies/fallen_angel_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/fallen_angel_base_battle.tsx` | `5c1652002b22cba89c1624a1034aecec4d9ac25fc33fef9df42313c926b61b7d` | tileset XML / fallen_angel_base_battle.tsx |
| ALI-0397 | `rusted_kingdoms/assets/sprites/enemies/fallen_angel_red_judicator.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/fallen_angel_red_judicator.png` | `165cf2aa091e4fc2b47a081a8642722efb21adfed6874c0eaf030a27341807e3` | image / fallen_angel_red_judicator.png |
| ALI-0398 | `rusted_kingdoms/assets/sprites/enemies/fallen_angel_red_judicator.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/fallen_angel_red_judicator.tsx` | `d2a2076bb9041c1a72fcb8afa0cbfaedf94f48cbd51bbf07a278526324c54757` | tileset XML / fallen_angel_red_judicator.tsx |
| ALI-0399 | `rusted_kingdoms/assets/sprites/enemies/fallen_angel_red_judicator_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/fallen_angel_red_judicator_battle.png` | `7598fdfa2e2f4c54c91240022c1fece1eeeca33777c9d407a8410d775ecad626` | image / fallen_angel_red_judicator_battle.png |
| ALI-0400 | `rusted_kingdoms/assets/sprites/enemies/fallen_angel_red_judicator_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/fallen_angel_red_judicator_battle.tsx` | `cb7836557070d7294c9cf956bc3360e1704217e2d4e9d110d1b4b4bac1c0477e` | tileset XML / fallen_angel_red_judicator_battle.tsx |
| ALI-0401 | `rusted_kingdoms/assets/sprites/enemies/frankenstein_brawler_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/frankenstein_brawler_base.png` | `c78e408d9dea7795f8aa11c52e7276d234e8d87175b96e916a0aac444abb9f7a` | image / frankenstein_brawler_base.png |
| ALI-0402 | `rusted_kingdoms/assets/sprites/enemies/frankenstein_brawler_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/frankenstein_brawler_base.tsx` | `ee76b47902bd13ef5ccc63068d268352574ab18a9c99b523e23904f19b10b82c` | tileset XML / frankenstein_brawler_base.tsx |
| ALI-0403 | `rusted_kingdoms/assets/sprites/enemies/frankenstein_brawler_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/frankenstein_brawler_base_battle.png` | `05107f9a162de3dd38fe2a3a3446b63e734124fa47ec20f6e634764122b7cb98` | image / frankenstein_brawler_base_battle.png |
| ALI-0404 | `rusted_kingdoms/assets/sprites/enemies/frankenstein_brawler_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/frankenstein_brawler_base_battle.tsx` | `e6429c9b118dab57eb59ba35b538606f71e8c76ad1b5e7081f252aae06bfc207` | tileset XML / frankenstein_brawler_base_battle.tsx |
| ALI-0405 | `rusted_kingdoms/assets/sprites/enemies/frankenstein_brawler_graveyard_smasher.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/frankenstein_brawler_graveyard_smasher.png` | `b9406a302689efec617900c468d6f2571d70eb8968d10cba795279d9f028b928` | image / frankenstein_brawler_graveyard_smasher.png |
| ALI-0406 | `rusted_kingdoms/assets/sprites/enemies/frankenstein_brawler_graveyard_smasher.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/frankenstein_brawler_graveyard_smasher.tsx` | `664a1a0447cda1ba04079a3d7a5fa9b34665def102874555dbbc3b9b36bff17e` | tileset XML / frankenstein_brawler_graveyard_smasher.tsx |
| ALI-0407 | `rusted_kingdoms/assets/sprites/enemies/frankenstein_brawler_graveyard_smasher_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/frankenstein_brawler_graveyard_smasher_battle.png` | `a71df00a91ea8a676eb0b11fb5fca739f8b913d8c3aa2dff4cb65ff4668d5c2d` | image / frankenstein_brawler_graveyard_smasher_battle.png |
| ALI-0408 | `rusted_kingdoms/assets/sprites/enemies/frankenstein_brawler_graveyard_smasher_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/frankenstein_brawler_graveyard_smasher_battle.tsx` | `1b3942cebc28617006a1c145a213899e7b3362e986294f6f52d79887d2f35105` | tileset XML / frankenstein_brawler_graveyard_smasher_battle.tsx |
| ALI-0409 | `rusted_kingdoms/assets/sprites/enemies/frankenstein_brawler_scarred_construct.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/frankenstein_brawler_scarred_construct.png` | `637c2d85517a922c77117c48ed51609baf5a2faf267c3401f2f950cd90d281bb` | image / frankenstein_brawler_scarred_construct.png |
| ALI-0410 | `rusted_kingdoms/assets/sprites/enemies/frankenstein_brawler_scarred_construct.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/frankenstein_brawler_scarred_construct.tsx` | `b7dd30e823641717cf4ecbe0ac583d3cd525ffbac3b53a1cdd3d169bcb6ef0d2` | tileset XML / frankenstein_brawler_scarred_construct.tsx |
| ALI-0411 | `rusted_kingdoms/assets/sprites/enemies/frankenstein_brawler_scarred_construct_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/frankenstein_brawler_scarred_construct_battle.png` | `1e41da8f90d0976220392b411dc49dbee2b0c8908cbf9a57c84df46b9422cccd` | image / frankenstein_brawler_scarred_construct_battle.png |
| ALI-0412 | `rusted_kingdoms/assets/sprites/enemies/frankenstein_brawler_scarred_construct_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/frankenstein_brawler_scarred_construct_battle.tsx` | `699735c8a04ef5074b74bd8882b6b897f871081a245736912bcc4709aa8ea104` | tileset XML / frankenstein_brawler_scarred_construct_battle.tsx |
| ALI-0413 | `rusted_kingdoms/assets/sprites/enemies/goblin_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_battle.png` | `ade22e31e715437ed637320bda7fe9fb00ead3d193b43bbb5e0d217258097d76` | image / goblin_battle.png |
| ALI-0414 | `rusted_kingdoms/assets/sprites/enemies/goblin_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_battle.tsx` | `7da8c09f85319f9f91591de4a587d1e277fde1442aa9c8c50d7f4a9783942899` | tileset XML / goblin_battle.tsx |
| ALI-0415 | `rusted_kingdoms/assets/sprites/enemies/goblin_bomber_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_bomber_base.png` | `723f63e7c1ab93d072f83d7e13023523e90fdc8cde228ee4c5d34ef7ddcdb23c` | image / goblin_bomber_base.png |
| ALI-0416 | `rusted_kingdoms/assets/sprites/enemies/goblin_bomber_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_bomber_base.tsx` | `50b419feb8eea5c48b1fbd27135f431f18583bafa54da304c9ad3b4722691a64` | tileset XML / goblin_bomber_base.tsx |
| ALI-0417 | `rusted_kingdoms/assets/sprites/enemies/goblin_bomber_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_bomber_base_battle.png` | `a40c170181dc92a4854294bb61b3853178901ad44411465196fd96e79e3d1974` | image / goblin_bomber_base_battle.png |
| ALI-0418 | `rusted_kingdoms/assets/sprites/enemies/goblin_bomber_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_bomber_base_battle.tsx` | `6bf3df99153246019026310b45f2867cd2093b38e040aecc3ca0bd624b314f5f` | tileset XML / goblin_bomber_base_battle.tsx |
| ALI-0419 | `rusted_kingdoms/assets/sprites/enemies/goblin_bomber_club_sapper.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_bomber_club_sapper.png` | `b4af6b06db8518a183be4a29b46a6d7906892b205e47fea77cb5b4cd1f1fc58f` | image / goblin_bomber_club_sapper.png |
| ALI-0420 | `rusted_kingdoms/assets/sprites/enemies/goblin_bomber_club_sapper.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_bomber_club_sapper.tsx` | `ade9eac4b4ec4824ddc6c1ad715deebb548ba8abc2fb412991bf01cb168ee7a3` | tileset XML / goblin_bomber_club_sapper.tsx |
| ALI-0421 | `rusted_kingdoms/assets/sprites/enemies/goblin_bomber_club_sapper_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_bomber_club_sapper_battle.png` | `11e557713d1911d983e395813a79ab67959269197fcc83b298c050f97f600703` | image / goblin_bomber_club_sapper_battle.png |
| ALI-0422 | `rusted_kingdoms/assets/sprites/enemies/goblin_bomber_club_sapper_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_bomber_club_sapper_battle.tsx` | `3d174447ca4ab7a623cedf96723df581affc4520e2bd49ba245aa4647a4c1b69` | tileset XML / goblin_bomber_club_sapper_battle.tsx |
| ALI-0423 | `rusted_kingdoms/assets/sprites/enemies/goblin_bomber_redcap_bomber.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_bomber_redcap_bomber.png` | `e93ac49bcd7581bb409b96589966705de15f4d332bec07e83a2a562eb57ffe3b` | image / goblin_bomber_redcap_bomber.png |
| ALI-0424 | `rusted_kingdoms/assets/sprites/enemies/goblin_bomber_redcap_bomber.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_bomber_redcap_bomber.tsx` | `296f30f9b86e7d893176ac545ae7afba38d303dd71a818f4f42a84b5a027cd2e` | tileset XML / goblin_bomber_redcap_bomber.tsx |
| ALI-0425 | `rusted_kingdoms/assets/sprites/enemies/goblin_bomber_redcap_bomber_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_bomber_redcap_bomber_battle.png` | `04ab5f8ca59ea7f2ce03a26e2e52b0876097e83b8e0b6f3b0da72a05e8dddb4a` | image / goblin_bomber_redcap_bomber_battle.png |
| ALI-0426 | `rusted_kingdoms/assets/sprites/enemies/goblin_bomber_redcap_bomber_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_bomber_redcap_bomber_battle.tsx` | `0d5b89063efc86f81225b1fe98b129e2f6cb657d474030f6a334b088a6c3f696` | tileset XML / goblin_bomber_redcap_bomber_battle.tsx |
| ALI-0427 | `rusted_kingdoms/assets/sprites/enemies/goblin_crossbowman_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_crossbowman_base.png` | `ae3e01d7b7c5987aea7529d8e5229d58b5602e6bf77ba87b722ce857752def5c` | image / goblin_crossbowman_base.png |
| ALI-0428 | `rusted_kingdoms/assets/sprites/enemies/goblin_crossbowman_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_crossbowman_base.tsx` | `31189f259dea17550ad9a38c46a71bee09670412951ee454477753e62a10c24f` | tileset XML / goblin_crossbowman_base.tsx |
| ALI-0429 | `rusted_kingdoms/assets/sprites/enemies/goblin_crossbowman_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_crossbowman_base_battle.png` | `7ad1101c60625b778178ba311371cc2585b3780fa9354fda8f05a2421105be08` | image / goblin_crossbowman_base_battle.png |
| ALI-0430 | `rusted_kingdoms/assets/sprites/enemies/goblin_crossbowman_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_crossbowman_base_battle.tsx` | `69abe20d551364b29ed1dcf8355b00210da47f5bec5d6afc63e41a8484615283` | tileset XML / goblin_crossbowman_base_battle.tsx |
| ALI-0431 | `rusted_kingdoms/assets/sprites/enemies/goblin_crossbowman_hooded_marksman.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_crossbowman_hooded_marksman.png` | `592deb5d812aad2d882860976451420e939a4f04dea25316cc7627ba9d0ef2f0` | image / goblin_crossbowman_hooded_marksman.png |
| ALI-0432 | `rusted_kingdoms/assets/sprites/enemies/goblin_crossbowman_hooded_marksman.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_crossbowman_hooded_marksman.tsx` | `3d6d6f1c7496b4901f44235b357c3642a0983aa367533983d30ba131f3a2fc4f` | tileset XML / goblin_crossbowman_hooded_marksman.tsx |
| ALI-0433 | `rusted_kingdoms/assets/sprites/enemies/goblin_crossbowman_hooded_marksman_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_crossbowman_hooded_marksman_battle.png` | `21e83283e691444d5133c356e4e8ae19a6bb0fad19d882b22e135341667df05d` | image / goblin_crossbowman_hooded_marksman_battle.png |
| ALI-0434 | `rusted_kingdoms/assets/sprites/enemies/goblin_crossbowman_hooded_marksman_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_crossbowman_hooded_marksman_battle.tsx` | `430532329212c9640edf952170bd0ae0ead4a1fdeef5deee6ffd9cf79ce67912` | tileset XML / goblin_crossbowman_hooded_marksman_battle.tsx |
| ALI-0435 | `rusted_kingdoms/assets/sprites/enemies/goblin_crossbowman_knife_backup.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_crossbowman_knife_backup.png` | `aeb215087766d9e1a6e75b9081a2b17eeb6d5b740813b2030d68c423bc7ce58c` | image / goblin_crossbowman_knife_backup.png |
| ALI-0436 | `rusted_kingdoms/assets/sprites/enemies/goblin_crossbowman_knife_backup.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_crossbowman_knife_backup.tsx` | `aa66c73261fc5faeb1f6fb7fa815b139652ed3cd18931772992bd2c0d7cf24ee` | tileset XML / goblin_crossbowman_knife_backup.tsx |
| ALI-0437 | `rusted_kingdoms/assets/sprites/enemies/goblin_crossbowman_knife_backup_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_crossbowman_knife_backup_battle.png` | `f1a94993ab3209583a502e350e8763b9581b6457d5ea33a24c39ae8af281b8c7` | image / goblin_crossbowman_knife_backup_battle.png |
| ALI-0438 | `rusted_kingdoms/assets/sprites/enemies/goblin_crossbowman_knife_backup_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_crossbowman_knife_backup_battle.tsx` | `337875a41e9c95b5da057ce90f521359015f90cbc4fa05ae9dd48d3f85a501fc` | tileset XML / goblin_crossbowman_knife_backup_battle.tsx |
| ALI-0439 | `rusted_kingdoms/assets/sprites/enemies/goblin_scout_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_scout_base_battle.png` | `69ea7f308986e40ec479e6c289eb9b1b43cf754358f0bfb38073a66836436e9b` | image / goblin_scout_base_battle.png |
| ALI-0440 | `rusted_kingdoms/assets/sprites/enemies/goblin_scout_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_scout_base_battle.tsx` | `befaad14e46ebc467bb3f16c1f3d429bf61ecd8aa44138edfd4ca76197565ea7` | tileset XML / goblin_scout_base_battle.tsx |
| ALI-0441 | `rusted_kingdoms/assets/sprites/enemies/goblin_scout_hooded_goblin_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_scout_hooded_goblin_battle.png` | `e367b5a36c8c1f40b903c46a8f1856d7aba09139cd21d403b5a41f194df80450` | image / goblin_scout_hooded_goblin_battle.png |
| ALI-0442 | `rusted_kingdoms/assets/sprites/enemies/goblin_scout_hooded_goblin_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_scout_hooded_goblin_battle.tsx` | `4389af7d96b05fe85755af07ebc4cf668a0071ae0b23efb18324dc17999a2a0e` | tileset XML / goblin_scout_hooded_goblin_battle.tsx |
| ALI-0443 | `rusted_kingdoms/assets/sprites/enemies/goblin_scout_sling_scout_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_scout_sling_scout_battle.png` | `e2e285a2a0a2659756cb6cdcf669ff65a01108a7594d8e6e9a1158721fe8585c` | image / goblin_scout_sling_scout_battle.png |
| ALI-0444 | `rusted_kingdoms/assets/sprites/enemies/goblin_scout_sling_scout_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_scout_sling_scout_battle.tsx` | `4c5c7fa909771a1e677ffe40602d5b80a915c62e68acdae8b9bc1a4fbebdfb10` | tileset XML / goblin_scout_sling_scout_battle.tsx |
| ALI-0445 | `rusted_kingdoms/assets/sprites/enemies/goblin_warrior_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_warrior_battle.png` | `edeb2909df19fd0adb0de49b73d67d24b025e09c3cec814658783033b77c2b00` | image / goblin_warrior_battle.png |
| ALI-0446 | `rusted_kingdoms/assets/sprites/enemies/goblin_warrior_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/goblin_warrior_battle.tsx` | `37c0fc4abf91bb597e1c2620e4565c60fb866785314f88ce31c7e29a99b087fb` | tileset XML / goblin_warrior_battle.tsx |
| ALI-0447 | `rusted_kingdoms/assets/sprites/enemies/grik_the_grin_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/grik_the_grin_battle.png` | `0b9332bef962f487be43aecb8e1330f215c76720ae9de50d94538233ee7e009e` | image / grik_the_grin_battle.png |
| ALI-0448 | `rusted_kingdoms/assets/sprites/enemies/grik_the_grin_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/grik_the_grin_battle.tsx` | `93f1ccf5a18a7cdda3a4bcdbb5e043bc057eabc1ec8afa15edec03edc1bfb353` | tileset XML / grik_the_grin_battle.tsx |
| ALI-0449 | `rusted_kingdoms/assets/sprites/enemies/lizard_dragon_guard_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/lizard_dragon_guard_base.png` | `664f6d10f101799fdd0bd62e08f3cd06b2fe0faa58e805ca2109a057b9083abe` | image / lizard_dragon_guard_base.png |
| ALI-0450 | `rusted_kingdoms/assets/sprites/enemies/lizard_dragon_guard_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/lizard_dragon_guard_base.tsx` | `c2a288ee063209cf1a9d1443f3e4a646350a47cec9f080e20c228a77d2676b6a` | tileset XML / lizard_dragon_guard_base.tsx |
| ALI-0451 | `rusted_kingdoms/assets/sprites/enemies/lizard_dragon_guard_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/lizard_dragon_guard_base_battle.png` | `3f6d4e8b7de31e8645a278d91132ba8c512c367b6b8dbd97254fe4fad9fefd3e` | image / lizard_dragon_guard_base_battle.png |
| ALI-0452 | `rusted_kingdoms/assets/sprites/enemies/lizard_dragon_guard_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/lizard_dragon_guard_base_battle.tsx` | `00b9ccfe5a6dd97f1ab2afa508a835df2bc69f38194a8cd9ee3e07c6f7b8ab46` | tileset XML / lizard_dragon_guard_base_battle.tsx |
| ALI-0453 | `rusted_kingdoms/assets/sprites/enemies/lizard_dragon_guard_blue_drake_guard.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/lizard_dragon_guard_blue_drake_guard.png` | `20018ffed89a513acca83211c8c8d6cb06920888e0b8a511d66c0c174f5368d4` | image / lizard_dragon_guard_blue_drake_guard.png |
| ALI-0454 | `rusted_kingdoms/assets/sprites/enemies/lizard_dragon_guard_blue_drake_guard.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/lizard_dragon_guard_blue_drake_guard.tsx` | `b2b7af2592bd20ff6162175658a6e3f200b3d812eae79747e0937c469713972d` | tileset XML / lizard_dragon_guard_blue_drake_guard.tsx |
| ALI-0455 | `rusted_kingdoms/assets/sprites/enemies/lizard_dragon_guard_blue_drake_guard_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/lizard_dragon_guard_blue_drake_guard_battle.png` | `6b573f985d8c4c2d5f0310a7b344c75f0b5aed785c1d7e302c800a08c716eeb3` | image / lizard_dragon_guard_blue_drake_guard_battle.png |
| ALI-0456 | `rusted_kingdoms/assets/sprites/enemies/lizard_dragon_guard_blue_drake_guard_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/lizard_dragon_guard_blue_drake_guard_battle.tsx` | `1a30f243122482e4f22c45de778e25785bfbc875695a4033890ac798254632a1` | tileset XML / lizard_dragon_guard_blue_drake_guard_battle.tsx |
| ALI-0457 | `rusted_kingdoms/assets/sprites/enemies/lizard_dragon_guard_trident_guard.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/lizard_dragon_guard_trident_guard.png` | `94c075b6cdd8c590fb431cecfd860ebf528e56470ed9ddd21875b72f2d114fdc` | image / lizard_dragon_guard_trident_guard.png |
| ALI-0458 | `rusted_kingdoms/assets/sprites/enemies/lizard_dragon_guard_trident_guard.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/lizard_dragon_guard_trident_guard.tsx` | `b104c1d94f4a2ceb1d26ced9e3adabc4379555ad2517390b04ff5fa4e348a1de` | tileset XML / lizard_dragon_guard_trident_guard.tsx |
| ALI-0459 | `rusted_kingdoms/assets/sprites/enemies/lizard_dragon_guard_trident_guard_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/lizard_dragon_guard_trident_guard_battle.png` | `c37b2d7fbac4b27552f0a982c25357a951bc3b9e5f30ce87bd0bc7a05fd5f407` | image / lizard_dragon_guard_trident_guard_battle.png |
| ALI-0460 | `rusted_kingdoms/assets/sprites/enemies/lizard_dragon_guard_trident_guard_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/lizard_dragon_guard_trident_guard_battle.tsx` | `a5de191b0d72dee2eed5d41d7f00b6ea9d640cfbab79b13e0356bc03f5f89408` | tileset XML / lizard_dragon_guard_trident_guard_battle.tsx |
| ALI-0461 | `rusted_kingdoms/assets/sprites/enemies/lizard_monster_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/lizard_monster_base.png` | `60ede1f775e86e3ddbd34ea72db2486fff0cc2d666c3248065f1a5a096a00315` | image / lizard_monster_base.png |
| ALI-0462 | `rusted_kingdoms/assets/sprites/enemies/lizard_monster_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/lizard_monster_base.tsx` | `92c6baca8dc87f1de7a99d65300453bd9e37e3140a004dfc28439a772f61f347` | tileset XML / lizard_monster_base.tsx |
| ALI-0463 | `rusted_kingdoms/assets/sprites/enemies/lizard_monster_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/lizard_monster_base_battle.png` | `ce578dc611e8d2ee85c09de83dddbd1ccbd4f2814742ec78adaa8c5a1280f7bf` | image / lizard_monster_base_battle.png |
| ALI-0464 | `rusted_kingdoms/assets/sprites/enemies/lizard_monster_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/lizard_monster_base_battle.tsx` | `0e91a0f38998a3276f13c1e44f33ae5330d68abae694ccfebf386e13fe699c51` | tileset XML / lizard_monster_base_battle.tsx |
| ALI-0465 | `rusted_kingdoms/assets/sprites/enemies/lizard_monster_blue_scale_lizard.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/lizard_monster_blue_scale_lizard.png` | `93fe071ae790de779169c4e3925f6bb599ab7771c477c86bba2fa9f10abc05d5` | image / lizard_monster_blue_scale_lizard.png |
| ALI-0466 | `rusted_kingdoms/assets/sprites/enemies/lizard_monster_blue_scale_lizard.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/lizard_monster_blue_scale_lizard.tsx` | `a61a834d9151a6434cfbdcd571158d5d28a1fddf1cec1df10d05343996a8a604` | tileset XML / lizard_monster_blue_scale_lizard.tsx |
| ALI-0467 | `rusted_kingdoms/assets/sprites/enemies/lizard_monster_blue_scale_lizard_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/lizard_monster_blue_scale_lizard_battle.png` | `d3ceb9bd41e9deab090d818ecfdad627b938c6d695b051eb25149ed3f2994bb9` | image / lizard_monster_blue_scale_lizard_battle.png |
| ALI-0468 | `rusted_kingdoms/assets/sprites/enemies/lizard_monster_blue_scale_lizard_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/lizard_monster_blue_scale_lizard_battle.tsx` | `fb103e2dcb953e71136c3241d03ed5220beb87e370bff1b69fa674b7910f8eb2` | tileset XML / lizard_monster_blue_scale_lizard_battle.tsx |
| ALI-0469 | `rusted_kingdoms/assets/sprites/enemies/lizard_monster_winged_lizard.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/lizard_monster_winged_lizard.png` | `188f97d2befc9b038841062ef936456a8750d8e39756f66a91c5a2d8a0c5e2f5` | image / lizard_monster_winged_lizard.png |
| ALI-0470 | `rusted_kingdoms/assets/sprites/enemies/lizard_monster_winged_lizard.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/lizard_monster_winged_lizard.tsx` | `ebb60d548ce77c30bbb46436019dfb62f48fbbce954c57d8e891b1bc5346b3c6` | tileset XML / lizard_monster_winged_lizard.tsx |
| ALI-0471 | `rusted_kingdoms/assets/sprites/enemies/lizard_monster_winged_lizard_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/lizard_monster_winged_lizard_battle.png` | `4b0312f376fa03b687ce0f8dba571f32b05d42fb6173e091655b3df92791fb0d` | image / lizard_monster_winged_lizard_battle.png |
| ALI-0472 | `rusted_kingdoms/assets/sprites/enemies/lizard_monster_winged_lizard_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/lizard_monster_winged_lizard_battle.tsx` | `18f5f470a63f3ee54eff100119b77508ce423665e2a945f40569032139aa3e68` | tileset XML / lizard_monster_winged_lizard_battle.tsx |
| ALI-0473 | `rusted_kingdoms/assets/sprites/enemies/minotaur_brute_armored_minotaur.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/minotaur_brute_armored_minotaur.png` | `de2360915d3fa6775874abaec0d4963d943391aef8ce1a4afff58602bc2fe692` | image / minotaur_brute_armored_minotaur.png |
| ALI-0474 | `rusted_kingdoms/assets/sprites/enemies/minotaur_brute_armored_minotaur.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/minotaur_brute_armored_minotaur.tsx` | `70d311fa56cd47f1645c2d1e848f9340570a3a133a92be1c8b87631a5c1cc204` | tileset XML / minotaur_brute_armored_minotaur.tsx |
| ALI-0475 | `rusted_kingdoms/assets/sprites/enemies/minotaur_brute_armored_minotaur_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/minotaur_brute_armored_minotaur_battle.png` | `c0099c6af18a30e9933fa8dc562c77e144e083e2569445838d4fb3955f768c68` | image / minotaur_brute_armored_minotaur_battle.png |
| ALI-0476 | `rusted_kingdoms/assets/sprites/enemies/minotaur_brute_armored_minotaur_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/minotaur_brute_armored_minotaur_battle.tsx` | `b2e928b132f43dc1712d2c1d03105e9d1157e1844165fb955362c16564386f4b` | tileset XML / minotaur_brute_armored_minotaur_battle.tsx |
| ALI-0477 | `rusted_kingdoms/assets/sprites/enemies/minotaur_brute_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/minotaur_brute_base.png` | `cead06a21331b95d4a56fd37a37690985ecc6ca95c47524f4218259e7e2fbdff` | image / minotaur_brute_base.png |
| ALI-0478 | `rusted_kingdoms/assets/sprites/enemies/minotaur_brute_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/minotaur_brute_base.tsx` | `a1d254f618eba2187323ff3a131772f55efaa4fc8801323f84fa6d0750b1de7d` | tileset XML / minotaur_brute_base.tsx |
| ALI-0479 | `rusted_kingdoms/assets/sprites/enemies/minotaur_brute_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/minotaur_brute_base_battle.png` | `dc39d1bcc78537ad71e2c98d826fb299b544c64a2962b807b4625f6b17ef2738` | image / minotaur_brute_base_battle.png |
| ALI-0480 | `rusted_kingdoms/assets/sprites/enemies/minotaur_brute_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/minotaur_brute_base_battle.tsx` | `b8ac959eb001456887c55367d62467e1fc3b94606e27bf9eb974e67df5935b12` | tileset XML / minotaur_brute_base_battle.tsx |
| ALI-0481 | `rusted_kingdoms/assets/sprites/enemies/minotaur_brute_club_brute.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/minotaur_brute_club_brute.png` | `a8847ceb12c9a67c3f3a1ba40840edad30fcce0d3ae9e57ad50e53a110392880` | image / minotaur_brute_club_brute.png |
| ALI-0482 | `rusted_kingdoms/assets/sprites/enemies/minotaur_brute_club_brute.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/minotaur_brute_club_brute.tsx` | `a5d93969c298fa841a2ba2637d27f6964319a3907f451fe327c54d79b3d807ab` | tileset XML / minotaur_brute_club_brute.tsx |
| ALI-0483 | `rusted_kingdoms/assets/sprites/enemies/minotaur_brute_club_brute_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/minotaur_brute_club_brute_battle.png` | `6ab0f1a8b368e3e97c9fc223f0656591acca318eee58304fb0525e5fba110f41` | image / minotaur_brute_club_brute_battle.png |
| ALI-0484 | `rusted_kingdoms/assets/sprites/enemies/minotaur_brute_club_brute_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/minotaur_brute_club_brute_battle.tsx` | `922ff18441a7bd1db72c2f0bb87d5a9159eb1ad44690c3c74e4dd1836ad1827c` | tileset XML / minotaur_brute_club_brute_battle.tsx |
| ALI-0485 | `rusted_kingdoms/assets/sprites/enemies/mouse_thief_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/mouse_thief_base.png` | `37df0126a56da1ca800d3aa6128109fd75fe9bfe037c9afda20aaa19ffb54055` | image / mouse_thief_base.png |
| ALI-0486 | `rusted_kingdoms/assets/sprites/enemies/mouse_thief_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/mouse_thief_base.tsx` | `2956f9336082c813cc18098c3fe0d3b07cb19ef36bf224785977e109d5e117c1` | tileset XML / mouse_thief_base.tsx |
| ALI-0487 | `rusted_kingdoms/assets/sprites/enemies/mouse_thief_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/mouse_thief_base_battle.png` | `cc166ae793969cce9f4cd8ae34723271f78693ca1c3041463f510ebee5f62575` | image / mouse_thief_base_battle.png |
| ALI-0488 | `rusted_kingdoms/assets/sprites/enemies/mouse_thief_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/mouse_thief_base_battle.tsx` | `dbf581df7276ddb2fb6a34b251c11d6e52888dbc834faa579dfd008ff7a95a4e` | tileset XML / mouse_thief_base_battle.tsx |
| ALI-0489 | `rusted_kingdoms/assets/sprites/enemies/mouse_thief_mouse_archer.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/mouse_thief_mouse_archer.png` | `b10fa2a2dd6f6941a1ec65524a7906fae56d7111c8a805735ac85bdec4d4f92a` | image / mouse_thief_mouse_archer.png |
| ALI-0490 | `rusted_kingdoms/assets/sprites/enemies/mouse_thief_mouse_archer.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/mouse_thief_mouse_archer.tsx` | `e702813fcbb518cd0ef239f9154f88cb650b85bf8f92e29d29b4d61ecff41371` | tileset XML / mouse_thief_mouse_archer.tsx |
| ALI-0491 | `rusted_kingdoms/assets/sprites/enemies/mouse_thief_mouse_archer_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/mouse_thief_mouse_archer_battle.png` | `249614d46f2c46a18438391da5cc21d77bd8d595e3ca15a8835a088283554e5d` | image / mouse_thief_mouse_archer_battle.png |
| ALI-0492 | `rusted_kingdoms/assets/sprites/enemies/mouse_thief_mouse_archer_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/mouse_thief_mouse_archer_battle.tsx` | `13e4930dd408cf54b1ede7e6d1b4df5e4f270d093914abfdf97aef413e81decf` | tileset XML / mouse_thief_mouse_archer_battle.tsx |
| ALI-0493 | `rusted_kingdoms/assets/sprites/enemies/mouse_thief_white_mouse.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/mouse_thief_white_mouse.png` | `d36139eec03bcb0fcb0bfb73c64f055f1c5cc5fbbb982abc6c50831c24c24a86` | image / mouse_thief_white_mouse.png |
| ALI-0494 | `rusted_kingdoms/assets/sprites/enemies/mouse_thief_white_mouse.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/mouse_thief_white_mouse.tsx` | `082e99c60602c02809a0a87cf2302c798010542d2316e0b895a5e5acdb038b01` | tileset XML / mouse_thief_white_mouse.tsx |
| ALI-0495 | `rusted_kingdoms/assets/sprites/enemies/mouse_thief_white_mouse_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/mouse_thief_white_mouse_battle.png` | `9cf2f6b02c9c1353b82611276432fea97754d3c80019436ef474436849508018` | image / mouse_thief_white_mouse_battle.png |
| ALI-0496 | `rusted_kingdoms/assets/sprites/enemies/mouse_thief_white_mouse_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/mouse_thief_white_mouse_battle.tsx` | `a9c9a999c4f9375a17fca773660f53cf1217c6660217db66b0d3e8dd447bb60b` | tileset XML / mouse_thief_white_mouse_battle.tsx |
| ALI-0497 | `rusted_kingdoms/assets/sprites/enemies/orc_archer_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_archer_base.png` | `d1f62086b0efb6413338018d8c95a011cae909f64c8f87a1d235e6ad9f47c6ef` | image / orc_archer_base.png |
| ALI-0498 | `rusted_kingdoms/assets/sprites/enemies/orc_archer_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_archer_base.tsx` | `d720a2583e417ec1b1814d58287b7d9b6c37078d190590f4d3dffee2a2e0cf87` | tileset XML / orc_archer_base.tsx |
| ALI-0499 | `rusted_kingdoms/assets/sprites/enemies/orc_archer_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_archer_base_battle.png` | `5a7ccfdc44212ac190a439fa93a85be054b829812166242d7794448ea56989d0` | image / orc_archer_base_battle.png |
| ALI-0500 | `rusted_kingdoms/assets/sprites/enemies/orc_archer_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_archer_base_battle.tsx` | `0f8074d97e62ff4fa6d16678a593bc1bdf07ae5f6e8e09dc558143e9e0878c13` | tileset XML / orc_archer_base_battle.tsx |
| ALI-0501 | `rusted_kingdoms/assets/sprites/enemies/orc_archer_forest_bowman.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_archer_forest_bowman.png` | `2d0734056b22c4681ec56acd29d6036f3658cabd3d3a2ba4efaf26c65b3919cf` | image / orc_archer_forest_bowman.png |
| ALI-0502 | `rusted_kingdoms/assets/sprites/enemies/orc_archer_forest_bowman.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_archer_forest_bowman.tsx` | `acbb2718ac66bc75039ee3fd55e3cfa94242c5b3bebd540845cc9e25aa4132c4` | tileset XML / orc_archer_forest_bowman.tsx |
| ALI-0503 | `rusted_kingdoms/assets/sprites/enemies/orc_archer_forest_bowman_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_archer_forest_bowman_battle.png` | `67e592303cb473c5950eb01ce07222f14c8679c72af24829bbd7792d06b457f4` | image / orc_archer_forest_bowman_battle.png |
| ALI-0504 | `rusted_kingdoms/assets/sprites/enemies/orc_archer_forest_bowman_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_archer_forest_bowman_battle.tsx` | `b55a90ab6d39cf061cb2894f4d54daf821fe69cde435a2261f72d6871775089f` | tileset XML / orc_archer_forest_bowman_battle.tsx |
| ALI-0505 | `rusted_kingdoms/assets/sprites/enemies/orc_archer_greatbow_raider.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_archer_greatbow_raider.png` | `60cbc736f4254a91ff8a91a9da7daeb3bd4d2570dddb5bd046771a71fd3a6b97` | image / orc_archer_greatbow_raider.png |
| ALI-0506 | `rusted_kingdoms/assets/sprites/enemies/orc_archer_greatbow_raider.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_archer_greatbow_raider.tsx` | `900b81e9ffca12b62fbff3023bd459ce70489653cff982e5925b1b1e3cb7d9d8` | tileset XML / orc_archer_greatbow_raider.tsx |
| ALI-0507 | `rusted_kingdoms/assets/sprites/enemies/orc_archer_greatbow_raider_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_archer_greatbow_raider_battle.png` | `d1d23fc5e6b97a898a85a2d86fe10222c1969301c94ac4701f936187f5ea7416` | image / orc_archer_greatbow_raider_battle.png |
| ALI-0508 | `rusted_kingdoms/assets/sprites/enemies/orc_archer_greatbow_raider_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_archer_greatbow_raider_battle.tsx` | `200570f258a451b132f156f7d78fdcbd812e018ef945f1c3f71fbcf105b21cc0` | tileset XML / orc_archer_greatbow_raider_battle.tsx |
| ALI-0509 | `rusted_kingdoms/assets/sprites/enemies/orc_raider_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_raider_base.png` | `17189a082c8ea02b4a05b50686b5e4028d566679b859802fe91d366a6e6d43f8` | image / orc_raider_base.png |
| ALI-0510 | `rusted_kingdoms/assets/sprites/enemies/orc_raider_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_raider_base.tsx` | `05ed0b3bd262880cc2db987e4d05113dda01150cfa90287b1e9337509bf52f01` | tileset XML / orc_raider_base.tsx |
| ALI-0511 | `rusted_kingdoms/assets/sprites/enemies/orc_raider_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_raider_base_battle.png` | `92cab68745b59fb43ed2b235654314d4dd2e6ed0e41d991389d91fd25d5854a7` | image / orc_raider_base_battle.png |
| ALI-0512 | `rusted_kingdoms/assets/sprites/enemies/orc_raider_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_raider_base_battle.tsx` | `4a957e9ddeef97f1dabd3e530838f7aeaff3930c1f808adf3da198a31c667ddb` | tileset XML / orc_raider_base_battle.tsx |
| ALI-0513 | `rusted_kingdoms/assets/sprites/enemies/orc_raider_shield_bruiser.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_raider_shield_bruiser.png` | `f687f80b0db3e693b0e74513f03645ebdcebf78e941248eba7924bcfacf413bb` | image / orc_raider_shield_bruiser.png |
| ALI-0514 | `rusted_kingdoms/assets/sprites/enemies/orc_raider_shield_bruiser.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_raider_shield_bruiser.tsx` | `a4748c5bc38cfa5262987eaefb642435509f4acd68ddcba7b076232881b53a82` | tileset XML / orc_raider_shield_bruiser.tsx |
| ALI-0515 | `rusted_kingdoms/assets/sprites/enemies/orc_raider_shield_bruiser_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_raider_shield_bruiser_battle.png` | `1f0dc55b818f2a83ad86d6ddeac7e5885b4b9d5d7a700c242cf10240ede97f2f` | image / orc_raider_shield_bruiser_battle.png |
| ALI-0516 | `rusted_kingdoms/assets/sprites/enemies/orc_raider_shield_bruiser_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_raider_shield_bruiser_battle.tsx` | `3aa408f8282b202bbdac33af46ee09a251d9dfe67fd06e38e63c2cbde5ad0322` | tileset XML / orc_raider_shield_bruiser_battle.tsx |
| ALI-0517 | `rusted_kingdoms/assets/sprites/enemies/orc_raider_spear_raider.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_raider_spear_raider.png` | `74f15304057ba4c7401c3f9ff5da799a1dc9fbf87bf2c8e84fa96e8f47e1c88d` | image / orc_raider_spear_raider.png |
| ALI-0518 | `rusted_kingdoms/assets/sprites/enemies/orc_raider_spear_raider.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_raider_spear_raider.tsx` | `120716c3492263186bd6a2bcf24281f71497a858d367dac40e9b29dfd894da1d` | tileset XML / orc_raider_spear_raider.tsx |
| ALI-0519 | `rusted_kingdoms/assets/sprites/enemies/orc_raider_spear_raider_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_raider_spear_raider_battle.png` | `5c63c182b6c57ec6d9c6761ce5f384f90444294d0318f7c04dfda9dba4b9b17d` | image / orc_raider_spear_raider_battle.png |
| ALI-0520 | `rusted_kingdoms/assets/sprites/enemies/orc_raider_spear_raider_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_raider_spear_raider_battle.tsx` | `fbf148d5c6272426bafe2578bd59630f1d0fceaca743de1e8a43338a249c6972` | tileset XML / orc_raider_spear_raider_battle.tsx |
| ALI-0521 | `rusted_kingdoms/assets/sprites/enemies/orc_shaman_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_shaman_base.png` | `cea156d40472a9fcc8265721535bef9c41bc04cd30a8960d08934573543ef6b3` | image / orc_shaman_base.png |
| ALI-0522 | `rusted_kingdoms/assets/sprites/enemies/orc_shaman_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_shaman_base.tsx` | `7a8c799e87926a303ff5801d16c6c0da737a1f09f1fe2c2c19ca287181f1e75a` | tileset XML / orc_shaman_base.tsx |
| ALI-0523 | `rusted_kingdoms/assets/sprites/enemies/orc_shaman_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_shaman_base_battle.png` | `7082d4b5e13b969d9160325b1aae171097fa41613215923115efb20d37e79ab6` | image / orc_shaman_base_battle.png |
| ALI-0524 | `rusted_kingdoms/assets/sprites/enemies/orc_shaman_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_shaman_base_battle.tsx` | `048a229b09e69bc40417eb3409e2b58973aec4dafb818f183c0d0a2bb8d55d4f` | tileset XML / orc_shaman_base_battle.tsx |
| ALI-0525 | `rusted_kingdoms/assets/sprites/enemies/orc_shaman_bone_caller.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_shaman_bone_caller.png` | `e7e3fa8e9b32803ff26cbc6c96e4094def08de5d2495a94517eac859b5f20ed1` | image / orc_shaman_bone_caller.png |
| ALI-0526 | `rusted_kingdoms/assets/sprites/enemies/orc_shaman_bone_caller.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_shaman_bone_caller.tsx` | `53c942d5893d222d95903d582db361302f968093453bf4d44d78673a3191d1cb` | tileset XML / orc_shaman_bone_caller.tsx |
| ALI-0527 | `rusted_kingdoms/assets/sprites/enemies/orc_shaman_bone_caller_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_shaman_bone_caller_battle.png` | `0559862b61c66ed2ebb7d9d16dafd0ad77a82476752446a310ab37d41bb450c8` | image / orc_shaman_bone_caller_battle.png |
| ALI-0528 | `rusted_kingdoms/assets/sprites/enemies/orc_shaman_bone_caller_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_shaman_bone_caller_battle.tsx` | `b02da4cac877e3e87c6ed700d3166a580ca6f2688c41ac66698d1271b7752e99` | tileset XML / orc_shaman_bone_caller_battle.tsx |
| ALI-0529 | `rusted_kingdoms/assets/sprites/enemies/orc_shaman_red_hood_shaman.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_shaman_red_hood_shaman.png` | `c6bd2fc0b8a417c28bba2f9c576e8a6388ab17f0a1fb216cdcdb653687462ecd` | image / orc_shaman_red_hood_shaman.png |
| ALI-0530 | `rusted_kingdoms/assets/sprites/enemies/orc_shaman_red_hood_shaman.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_shaman_red_hood_shaman.tsx` | `69a63c4a5eccbe7d737dd1e9ebd7fbfe8983ea3f7b939dcf76a004ffb2b60cde` | tileset XML / orc_shaman_red_hood_shaman.tsx |
| ALI-0531 | `rusted_kingdoms/assets/sprites/enemies/orc_shaman_red_hood_shaman_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_shaman_red_hood_shaman_battle.png` | `c09dc5689bb19f750314bc465d95fc87d431ab639d9750602f5aaa4ceb3be1ea` | image / orc_shaman_red_hood_shaman_battle.png |
| ALI-0532 | `rusted_kingdoms/assets/sprites/enemies/orc_shaman_red_hood_shaman_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/orc_shaman_red_hood_shaman_battle.tsx` | `ec6e2cafed2f9958161fabd1845158ec50c37443edec8617c898c48eab2acedb` | tileset XML / orc_shaman_red_hood_shaman_battle.tsx |
| ALI-0533 | `rusted_kingdoms/assets/sprites/enemies/pigman_guard_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pigman_guard_base.png` | `ad00d38d4bd80431c8e8e15c5ca1319102ca0246a20d93a9d492f0eb02432e72` | image / pigman_guard_base.png |
| ALI-0534 | `rusted_kingdoms/assets/sprites/enemies/pigman_guard_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pigman_guard_base.tsx` | `e926b181ba27af5f68484b7b85221a3d7c58f5b2e21ad701ced9c719bbe292c3` | tileset XML / pigman_guard_base.tsx |
| ALI-0535 | `rusted_kingdoms/assets/sprites/enemies/pigman_guard_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pigman_guard_base_battle.png` | `1a3ec0aa498b39105ab95c6a678396ce8989906ee44835e6263e39a50a216ac2` | image / pigman_guard_base_battle.png |
| ALI-0536 | `rusted_kingdoms/assets/sprites/enemies/pigman_guard_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pigman_guard_base_battle.tsx` | `43a943a1edc1a8c37894af2a021977f46be5b930befa965f3aff5dd7417d61b9` | tileset XML / pigman_guard_base_battle.tsx |
| ALI-0537 | `rusted_kingdoms/assets/sprites/enemies/pigman_guard_boar_guard.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pigman_guard_boar_guard.png` | `9b6845b011dc6dcedeed7aa79777884286c6f64e370c54e064bda6404281d348` | image / pigman_guard_boar_guard.png |
| ALI-0538 | `rusted_kingdoms/assets/sprites/enemies/pigman_guard_boar_guard.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pigman_guard_boar_guard.tsx` | `8ae5951d879e2c4e465a0abbae875469760cbc69a54f83088d9824f876c2a27d` | tileset XML / pigman_guard_boar_guard.tsx |
| ALI-0539 | `rusted_kingdoms/assets/sprites/enemies/pigman_guard_boar_guard_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pigman_guard_boar_guard_battle.png` | `ef7c7ab115486f0e981d3feecec6087438b3098cebb2a623a3ce69ffa46ffd61` | image / pigman_guard_boar_guard_battle.png |
| ALI-0540 | `rusted_kingdoms/assets/sprites/enemies/pigman_guard_boar_guard_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pigman_guard_boar_guard_battle.tsx` | `10ed7d205652eda06f22aa4cc106175a271e0a337465564cbc9d1e34c924dac0` | tileset XML / pigman_guard_boar_guard_battle.tsx |
| ALI-0541 | `rusted_kingdoms/assets/sprites/enemies/pigman_guard_halberd_guard.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pigman_guard_halberd_guard.png` | `ba7f0e37b53bbaab39835f2c4fd6086c0315c31d4bd92ca95c6186979ef5ebaf` | image / pigman_guard_halberd_guard.png |
| ALI-0542 | `rusted_kingdoms/assets/sprites/enemies/pigman_guard_halberd_guard.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pigman_guard_halberd_guard.tsx` | `19b4511d68fdee30828790c6d350461ffb22335760eb58266df93e78a74890e7` | tileset XML / pigman_guard_halberd_guard.tsx |
| ALI-0543 | `rusted_kingdoms/assets/sprites/enemies/pigman_guard_halberd_guard_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pigman_guard_halberd_guard_battle.png` | `61b3bc3deee13a371135632494d578908bce99754d184dba4b4a6dfc86fd7548` | image / pigman_guard_halberd_guard_battle.png |
| ALI-0544 | `rusted_kingdoms/assets/sprites/enemies/pigman_guard_halberd_guard_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pigman_guard_halberd_guard_battle.tsx` | `d1656a00c00c11a190ce48cda9d342e5a6edefca9f98d7b13782e5de07b563e0` | tileset XML / pigman_guard_halberd_guard_battle.tsx |
| ALI-0545 | `rusted_kingdoms/assets/sprites/enemies/pirate_captain_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pirate_captain_base.png` | `e6cc0d2fd9677802d28c2bfdfb4817bfa557cf4997d9c0fbc442e190a6b0cccc` | image / pirate_captain_base.png |
| ALI-0546 | `rusted_kingdoms/assets/sprites/enemies/pirate_captain_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pirate_captain_base.tsx` | `e463e47febabf19574d7c62d6974419538ba239b1e10c9010e2f320db0c7bdb7` | tileset XML / pirate_captain_base.tsx |
| ALI-0547 | `rusted_kingdoms/assets/sprites/enemies/pirate_captain_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pirate_captain_base_battle.png` | `635ae21e08743e26c460337ed1782e132a042e99ddb45bedb049cb635e4dfc98` | image / pirate_captain_base_battle.png |
| ALI-0548 | `rusted_kingdoms/assets/sprites/enemies/pirate_captain_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pirate_captain_base_battle.tsx` | `3487afcd40a439c397b8a4a81ebca2a9fc6fa96d362df7310264ed22f3a10cf3` | tileset XML / pirate_captain_base_battle.tsx |
| ALI-0549 | `rusted_kingdoms/assets/sprites/enemies/pirate_captain_cutlass_raider.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pirate_captain_cutlass_raider.png` | `3b0a56ace1b8cdb178b47d54cc1ab68128ab7cfe699e4925209bf37f3287b15f` | image / pirate_captain_cutlass_raider.png |
| ALI-0550 | `rusted_kingdoms/assets/sprites/enemies/pirate_captain_cutlass_raider.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pirate_captain_cutlass_raider.tsx` | `d9adc4b38f03c620150f36aef8ca5a5fbaa0525091a663f07f8b4e9acbb32a79` | tileset XML / pirate_captain_cutlass_raider.tsx |
| ALI-0551 | `rusted_kingdoms/assets/sprites/enemies/pirate_captain_cutlass_raider_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pirate_captain_cutlass_raider_battle.png` | `518be3d81cbebd4d830d0f622c853cf9fb0eb6f39769ad185830a0b5baa99cc1` | image / pirate_captain_cutlass_raider_battle.png |
| ALI-0552 | `rusted_kingdoms/assets/sprites/enemies/pirate_captain_cutlass_raider_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pirate_captain_cutlass_raider_battle.tsx` | `5d9af39a870069730988d337128ff7288a00106a0c15f69a5b7285603c84a3fc` | tileset XML / pirate_captain_cutlass_raider_battle.tsx |
| ALI-0553 | `rusted_kingdoms/assets/sprites/enemies/pirate_captain_eyepatch_captain.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pirate_captain_eyepatch_captain.png` | `6be454a4b0b798625abeb0f42ae1fb7b7594bb8ddb88e23510846f367d1d3ee1` | image / pirate_captain_eyepatch_captain.png |
| ALI-0554 | `rusted_kingdoms/assets/sprites/enemies/pirate_captain_eyepatch_captain.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pirate_captain_eyepatch_captain.tsx` | `e31974623a12e8df45657e51a894858cc6ee7f899f9211b6ae94ee17a8dfcd68` | tileset XML / pirate_captain_eyepatch_captain.tsx |
| ALI-0555 | `rusted_kingdoms/assets/sprites/enemies/pirate_captain_eyepatch_captain_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pirate_captain_eyepatch_captain_battle.png` | `7986c1e8ea8a702ecff96e17c7ba0370007c9c0d693fa7ada9614a9eab4c7a30` | image / pirate_captain_eyepatch_captain_battle.png |
| ALI-0556 | `rusted_kingdoms/assets/sprites/enemies/pirate_captain_eyepatch_captain_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pirate_captain_eyepatch_captain_battle.tsx` | `3940dfa4fa5005e3dbf02ff0e53ca1dcf56119a9cc258e35cf700812090c036e` | tileset XML / pirate_captain_eyepatch_captain_battle.tsx |
| ALI-0557 | `rusted_kingdoms/assets/sprites/enemies/pumpkin_jack_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pumpkin_jack_base.png` | `8ffbc10c64446b4f79c786b63ff80d9366beb3f0f2893444706687fd6443e4a4` | image / pumpkin_jack_base.png |
| ALI-0558 | `rusted_kingdoms/assets/sprites/enemies/pumpkin_jack_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pumpkin_jack_base.tsx` | `850b7fe41f1f1bda8a1311a975d7d3311c7fa451d2b25ab496ae3a80c85b1ea1` | tileset XML / pumpkin_jack_base.tsx |
| ALI-0559 | `rusted_kingdoms/assets/sprites/enemies/pumpkin_jack_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pumpkin_jack_base_battle.png` | `8a2ab92df13976a4b234ceb769f079404b84c113ad9f0c6443b4bec639a19927` | image / pumpkin_jack_base_battle.png |
| ALI-0560 | `rusted_kingdoms/assets/sprites/enemies/pumpkin_jack_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pumpkin_jack_base_battle.tsx` | `71e31cc5db3f14b416eab156eda062b4a75c47ab7dbccefb51e04b6d9d63afee` | tileset XML / pumpkin_jack_base_battle.tsx |
| ALI-0561 | `rusted_kingdoms/assets/sprites/enemies/pumpkin_jack_harvest_reaper.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pumpkin_jack_harvest_reaper.png` | `048642bd721655beb03a1a4f1107cc9a84289c4ebe0c419ddb99cf24c5657d02` | image / pumpkin_jack_harvest_reaper.png |
| ALI-0562 | `rusted_kingdoms/assets/sprites/enemies/pumpkin_jack_harvest_reaper.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pumpkin_jack_harvest_reaper.tsx` | `900657448be1f2a2827e2c5d26ba20a209a89b79a19f2754fac89cd0998fb563` | tileset XML / pumpkin_jack_harvest_reaper.tsx |
| ALI-0563 | `rusted_kingdoms/assets/sprites/enemies/pumpkin_jack_harvest_reaper_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pumpkin_jack_harvest_reaper_battle.png` | `f9c41d26ebb41e6d7ac791c82a36573aa05660d45dcfa94f1ec50d094f5e0cbd` | image / pumpkin_jack_harvest_reaper_battle.png |
| ALI-0564 | `rusted_kingdoms/assets/sprites/enemies/pumpkin_jack_harvest_reaper_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pumpkin_jack_harvest_reaper_battle.tsx` | `aa78cb5a466313f1fda05cb1b823edff948a4b8daac4ad16fee02d6be9a22d28` | tileset XML / pumpkin_jack_harvest_reaper_battle.tsx |
| ALI-0565 | `rusted_kingdoms/assets/sprites/enemies/pumpkin_jack_lantern_duelist.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pumpkin_jack_lantern_duelist.png` | `1dcf27e798f6b21868483d55c8e6b5d1773049828a96b308ef71602b7279191e` | image / pumpkin_jack_lantern_duelist.png |
| ALI-0566 | `rusted_kingdoms/assets/sprites/enemies/pumpkin_jack_lantern_duelist.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pumpkin_jack_lantern_duelist.tsx` | `6c90e4eb27c314b250f98d76b43e150ce894649428eea5d1f864df641e596ae9` | tileset XML / pumpkin_jack_lantern_duelist.tsx |
| ALI-0567 | `rusted_kingdoms/assets/sprites/enemies/pumpkin_jack_lantern_duelist_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pumpkin_jack_lantern_duelist_battle.png` | `529473747ed2f3aeea5edcce4f25a278e762d6adfac46c7ade4f9fd5ca7e5b63` | image / pumpkin_jack_lantern_duelist_battle.png |
| ALI-0568 | `rusted_kingdoms/assets/sprites/enemies/pumpkin_jack_lantern_duelist_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/pumpkin_jack_lantern_duelist_battle.tsx` | `6687a6eb708c536ac51c5d1e714fb552e145d73d627dc4de8f57f4e2eedaefe6` | tileset XML / pumpkin_jack_lantern_duelist_battle.tsx |
| ALI-0569 | `rusted_kingdoms/assets/sprites/enemies/rabbit_bandit_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/rabbit_bandit_base.png` | `4e11f067c8d2f23140098ce05701f79414067b3cb4a2ebbcebb37f295db0d2be` | image / rabbit_bandit_base.png |
| ALI-0570 | `rusted_kingdoms/assets/sprites/enemies/rabbit_bandit_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/rabbit_bandit_base.tsx` | `b9bdded5a5db87069600f0628416b0b6a77f506e7cb79cf639599e75af1f1a53` | tileset XML / rabbit_bandit_base.tsx |
| ALI-0571 | `rusted_kingdoms/assets/sprites/enemies/rabbit_bandit_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/rabbit_bandit_base_battle.png` | `4623a3ba6fb02227cb96354a490b5165d572568d41f9fd6ca0b7606003229312` | image / rabbit_bandit_base_battle.png |
| ALI-0572 | `rusted_kingdoms/assets/sprites/enemies/rabbit_bandit_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/rabbit_bandit_base_battle.tsx` | `37f419a291ae006e981a0738612ff045b878ccc728c82dd50dc88373e37c6664` | tileset XML / rabbit_bandit_base_battle.tsx |
| ALI-0573 | `rusted_kingdoms/assets/sprites/enemies/rabbit_bandit_brown_rabbit.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/rabbit_bandit_brown_rabbit.png` | `fefd16951b6c7de3d5920f37eae26bae836b73a248f7fe33483c4c05131f930b` | image / rabbit_bandit_brown_rabbit.png |
| ALI-0574 | `rusted_kingdoms/assets/sprites/enemies/rabbit_bandit_brown_rabbit.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/rabbit_bandit_brown_rabbit.tsx` | `5946406028b540eefa192258ab768ba4dc5e28dd1f2daf44676af76b984cd574` | tileset XML / rabbit_bandit_brown_rabbit.tsx |
| ALI-0575 | `rusted_kingdoms/assets/sprites/enemies/rabbit_bandit_brown_rabbit_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/rabbit_bandit_brown_rabbit_battle.png` | `2ceae3ac15be743c02ba8e96b30810f9ff38e96ef224956523fa3a6609879b34` | image / rabbit_bandit_brown_rabbit_battle.png |
| ALI-0576 | `rusted_kingdoms/assets/sprites/enemies/rabbit_bandit_brown_rabbit_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/rabbit_bandit_brown_rabbit_battle.tsx` | `3b11d12947d08dd3c0e648008b0bf413edb2fb9137665c14f7d8e99a9f11122a` | tileset XML / rabbit_bandit_brown_rabbit_battle.tsx |
| ALI-0577 | `rusted_kingdoms/assets/sprites/enemies/rabbit_bandit_knife_bandit.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/rabbit_bandit_knife_bandit.png` | `d4362d5d9236c36d90046b90ead0623fd17d69723e23a58f109eccaa5e11b94a` | image / rabbit_bandit_knife_bandit.png |
| ALI-0578 | `rusted_kingdoms/assets/sprites/enemies/rabbit_bandit_knife_bandit.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/rabbit_bandit_knife_bandit.tsx` | `6427222b90b746428141b028c7b838c0e4e08c2bc0b47e6ef69376a5ebe56e85` | tileset XML / rabbit_bandit_knife_bandit.tsx |
| ALI-0579 | `rusted_kingdoms/assets/sprites/enemies/rabbit_bandit_knife_bandit_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/rabbit_bandit_knife_bandit_battle.png` | `d046b2c9668a1bb4f44fea01a70f12bbc9f5c2c9f8bbdc59c53c8531863883e7` | image / rabbit_bandit_knife_bandit_battle.png |
| ALI-0580 | `rusted_kingdoms/assets/sprites/enemies/rabbit_bandit_knife_bandit_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/rabbit_bandit_knife_bandit_battle.tsx` | `faae82339a7fbed668a07c75253789f496f9d8115cf66ac97fd65e5dec11ba54` | tileset XML / rabbit_bandit_knife_bandit_battle.tsx |
| ALI-0581 | `rusted_kingdoms/assets/sprites/enemies/ratkin_cutpurse_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/ratkin_cutpurse_base.png` | `50d775b57a5e71f1c81d6b706d1502db86cdc4228e08ff16cf0a2a08ac03a4ef` | image / ratkin_cutpurse_base.png |
| ALI-0582 | `rusted_kingdoms/assets/sprites/enemies/ratkin_cutpurse_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/ratkin_cutpurse_base.tsx` | `2d812afcff00322d310bbfe3e172e590415c4616c6dfa9452a4be6e8488f400d` | tileset XML / ratkin_cutpurse_base.tsx |
| ALI-0583 | `rusted_kingdoms/assets/sprites/enemies/ratkin_cutpurse_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/ratkin_cutpurse_base_battle.png` | `e25e2e81b98caf53204432df7e0844727c66e8b9162667d94f63ff1f9004391b` | image / ratkin_cutpurse_base_battle.png |
| ALI-0584 | `rusted_kingdoms/assets/sprites/enemies/ratkin_cutpurse_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/ratkin_cutpurse_base_battle.tsx` | `ba9e0c3aac915c44f7ea6d464f7ba0beb9c9c668c051789232fe69d9e3524633` | tileset XML / ratkin_cutpurse_base_battle.tsx |
| ALI-0585 | `rusted_kingdoms/assets/sprites/enemies/ratkin_cutpurse_masked_scavenger.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/ratkin_cutpurse_masked_scavenger.png` | `98f7690ee79310a7cfc35a9a9fe1748cb99dea191e4aafadedf0a09940c8d121` | image / ratkin_cutpurse_masked_scavenger.png |
| ALI-0586 | `rusted_kingdoms/assets/sprites/enemies/ratkin_cutpurse_masked_scavenger.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/ratkin_cutpurse_masked_scavenger.tsx` | `615d7dcf94bd1e99c3dfba3feb70336d5980b05e9047a421c6fe09f8e8ac4ef5` | tileset XML / ratkin_cutpurse_masked_scavenger.tsx |
| ALI-0587 | `rusted_kingdoms/assets/sprites/enemies/ratkin_cutpurse_masked_scavenger_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/ratkin_cutpurse_masked_scavenger_battle.png` | `7e52d2603796c4c4e586e286cb863b863431ad7fec7d504e1c97b9dc3cbe5957` | image / ratkin_cutpurse_masked_scavenger_battle.png |
| ALI-0588 | `rusted_kingdoms/assets/sprites/enemies/ratkin_cutpurse_masked_scavenger_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/ratkin_cutpurse_masked_scavenger_battle.tsx` | `bef1fc5b04affc5daf67941137da4f5e32521d8f40adabc889869f64d7a89ace` | tileset XML / ratkin_cutpurse_masked_scavenger_battle.tsx |
| ALI-0589 | `rusted_kingdoms/assets/sprites/enemies/ratkin_cutpurse_sewer_archer.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/ratkin_cutpurse_sewer_archer.png` | `a834d0f719b257e0b845295fa026981fce40b3c41ea00140cf4ea2faff9b655b` | image / ratkin_cutpurse_sewer_archer.png |
| ALI-0590 | `rusted_kingdoms/assets/sprites/enemies/ratkin_cutpurse_sewer_archer.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/ratkin_cutpurse_sewer_archer.tsx` | `30648767a35a3b82d76610f0b94a52ec79a67f37e76e6baed94af0351c6e13b7` | tileset XML / ratkin_cutpurse_sewer_archer.tsx |
| ALI-0591 | `rusted_kingdoms/assets/sprites/enemies/ratkin_cutpurse_sewer_archer_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/ratkin_cutpurse_sewer_archer_battle.png` | `2d7f2e4de7f7f2ab5e6cb9a6e25cc4f8acb7dc62b0133dd2193a5ae3518520da` | image / ratkin_cutpurse_sewer_archer_battle.png |
| ALI-0592 | `rusted_kingdoms/assets/sprites/enemies/ratkin_cutpurse_sewer_archer_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/ratkin_cutpurse_sewer_archer_battle.tsx` | `418cbf0d86f3e9c13ff38706c651d53e0dffc4682b51498348685def5f981b96` | tileset XML / ratkin_cutpurse_sewer_archer_battle.tsx |
| ALI-0593 | `rusted_kingdoms/assets/sprites/enemies/ratkin_plague_doctor_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/ratkin_plague_doctor_base.png` | `ef0a1f7792af747f2bfd00cdba66d19018b77a1d8a3d5e51ce5726a638429298` | image / ratkin_plague_doctor_base.png |
| ALI-0594 | `rusted_kingdoms/assets/sprites/enemies/ratkin_plague_doctor_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/ratkin_plague_doctor_base.tsx` | `e4a7632333a3faf5595b6a05357bc981f87991231cfddecbeba04b667b50f664` | tileset XML / ratkin_plague_doctor_base.tsx |
| ALI-0595 | `rusted_kingdoms/assets/sprites/enemies/ratkin_plague_doctor_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/ratkin_plague_doctor_base_battle.png` | `db697bce6481e5f243efb15a918d73e4d13fb712ca9baefb3e5793fd7c237aae` | image / ratkin_plague_doctor_base_battle.png |
| ALI-0596 | `rusted_kingdoms/assets/sprites/enemies/ratkin_plague_doctor_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/ratkin_plague_doctor_base_battle.tsx` | `530fbad18f685246dd214d63418974067bb7fa65c5c1feaf2196688d7e6d8063` | tileset XML / ratkin_plague_doctor_base_battle.tsx |
| ALI-0597 | `rusted_kingdoms/assets/sprites/enemies/ratkin_plague_doctor_black_mask_doctor.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/ratkin_plague_doctor_black_mask_doctor.png` | `17c78b562ea08e5b9a45d2454c5a8a20034bb62aacd396a9acdc8b776da44373` | image / ratkin_plague_doctor_black_mask_doctor.png |
| ALI-0598 | `rusted_kingdoms/assets/sprites/enemies/ratkin_plague_doctor_black_mask_doctor.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/ratkin_plague_doctor_black_mask_doctor.tsx` | `cbbaa62385c34bf6c92b6fb17b9c235a306d6ec1640e3c40b85c0cb549213a12` | tileset XML / ratkin_plague_doctor_black_mask_doctor.tsx |
| ALI-0599 | `rusted_kingdoms/assets/sprites/enemies/ratkin_plague_doctor_black_mask_doctor_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/ratkin_plague_doctor_black_mask_doctor_battle.png` | `2bf6d0ace6cbdae412dc54a0fa40c119ce8046f4e0e8024ad0447684ef62c551` | image / ratkin_plague_doctor_black_mask_doctor_battle.png |
| ALI-0600 | `rusted_kingdoms/assets/sprites/enemies/ratkin_plague_doctor_black_mask_doctor_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/ratkin_plague_doctor_black_mask_doctor_battle.tsx` | `0b4275542ac0edf4459bfd3b9b6b16d31a0fa2c0edcb5ee45087f576d35e5787` | tileset XML / ratkin_plague_doctor_black_mask_doctor_battle.tsx |
| ALI-0601 | `rusted_kingdoms/assets/sprites/enemies/ratkin_plague_doctor_sewer_physician.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/ratkin_plague_doctor_sewer_physician.png` | `81da2ad83cd29198e7678a21ccf94eb9c078ed09595fb7134f28522946a17377` | image / ratkin_plague_doctor_sewer_physician.png |
| ALI-0602 | `rusted_kingdoms/assets/sprites/enemies/ratkin_plague_doctor_sewer_physician.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/ratkin_plague_doctor_sewer_physician.tsx` | `266275d7ffc61209a19047ef122ba7363b56f92ad12927d2c872faf639c414f7` | tileset XML / ratkin_plague_doctor_sewer_physician.tsx |
| ALI-0603 | `rusted_kingdoms/assets/sprites/enemies/ratkin_plague_doctor_sewer_physician_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/ratkin_plague_doctor_sewer_physician_battle.png` | `0b422f2fe03befdc81aae2d1345cd41de12fd0817b4514eab5b2c3593c59652e` | image / ratkin_plague_doctor_sewer_physician_battle.png |
| ALI-0604 | `rusted_kingdoms/assets/sprites/enemies/ratkin_plague_doctor_sewer_physician_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/ratkin_plague_doctor_sewer_physician_battle.tsx` | `65e8832870e32a4b0244ec758ac2882c4fd1232ffaef7c4b1649b933a2942d97` | tileset XML / ratkin_plague_doctor_sewer_physician_battle.tsx |
| ALI-0605 | `rusted_kingdoms/assets/sprites/enemies/sheep_cultist_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/sheep_cultist_base.png` | `c3734ef2b24bc16ff5a14190c310e342251032cdd28ce41ddb11db40c1387d78` | image / sheep_cultist_base.png |
| ALI-0606 | `rusted_kingdoms/assets/sprites/enemies/sheep_cultist_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/sheep_cultist_base.tsx` | `cf6d7fcf3446bb8e84714d0385fdc84651dcd839d016475d363483b3140b8555` | tileset XML / sheep_cultist_base.tsx |
| ALI-0607 | `rusted_kingdoms/assets/sprites/enemies/sheep_cultist_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/sheep_cultist_base_battle.png` | `c5ec9729cfd07fcd6a9354ef57fdec9b9dbe103840dc5fca115bca577281d027` | image / sheep_cultist_base_battle.png |
| ALI-0608 | `rusted_kingdoms/assets/sprites/enemies/sheep_cultist_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/sheep_cultist_base_battle.tsx` | `f07d41942a22afc5ef0fcfe0e3d4266109618d5e6c24c3f9b2f74cd2443118ba` | tileset XML / sheep_cultist_base_battle.tsx |
| ALI-0609 | `rusted_kingdoms/assets/sprites/enemies/sheep_cultist_crystal_cultist.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/sheep_cultist_crystal_cultist.png` | `599e51a9c1e9996cd049ba49384626ce76d669243f01daf14e7e595d00c070c2` | image / sheep_cultist_crystal_cultist.png |
| ALI-0610 | `rusted_kingdoms/assets/sprites/enemies/sheep_cultist_crystal_cultist.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/sheep_cultist_crystal_cultist.tsx` | `6a6cb0440ac1e66b5cdf26a82f5a0c8808ef786db0335199d64e90bb493f2843` | tileset XML / sheep_cultist_crystal_cultist.tsx |
| ALI-0611 | `rusted_kingdoms/assets/sprites/enemies/sheep_cultist_crystal_cultist_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/sheep_cultist_crystal_cultist_battle.png` | `aad58cc1b7d9e62510302d5bfe02b5a6c01fa2ec30e1c8725ad8e0033f8ca078` | image / sheep_cultist_crystal_cultist_battle.png |
| ALI-0612 | `rusted_kingdoms/assets/sprites/enemies/sheep_cultist_crystal_cultist_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/sheep_cultist_crystal_cultist_battle.tsx` | `a02b17334e0a02b0a53ed5eb8b8e2bbf37fdcbd41034a420c572fdf4c127c2b5` | tileset XML / sheep_cultist_crystal_cultist_battle.tsx |
| ALI-0613 | `rusted_kingdoms/assets/sprites/enemies/sheep_cultist_white_robed_cultist.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/sheep_cultist_white_robed_cultist.png` | `5c45ea89bbfcef5eff11137453d6814545fa0f21b2b402c7cecb0cac605cb230` | image / sheep_cultist_white_robed_cultist.png |
| ALI-0614 | `rusted_kingdoms/assets/sprites/enemies/sheep_cultist_white_robed_cultist.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/sheep_cultist_white_robed_cultist.tsx` | `37a04528e15fe8d8c84bcdea086ad746930aa9848763d2c343b55a01ee72dbd7` | tileset XML / sheep_cultist_white_robed_cultist.tsx |
| ALI-0615 | `rusted_kingdoms/assets/sprites/enemies/sheep_cultist_white_robed_cultist_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/sheep_cultist_white_robed_cultist_battle.png` | `3014adb8fa70b67c71d37a16c2ca632612c679ef3a55ea99e0ae9c1d5b652f32` | image / sheep_cultist_white_robed_cultist_battle.png |
| ALI-0616 | `rusted_kingdoms/assets/sprites/enemies/sheep_cultist_white_robed_cultist_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/sheep_cultist_white_robed_cultist_battle.tsx` | `7f2783b4aebd9fbb4bd80b3e84deeb81b804643a4bb9fca3276e1efa7f2bc200` | tileset XML / sheep_cultist_white_robed_cultist_battle.tsx |
| ALI-0617 | `rusted_kingdoms/assets/sprites/enemies/skeleton_archer_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_archer_base.png` | `28a92f4b0625642933402f85645d7f50ae48cfb0e25e7e4d7c0328fd4c433bf9` | image / skeleton_archer_base.png |
| ALI-0618 | `rusted_kingdoms/assets/sprites/enemies/skeleton_archer_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_archer_base.tsx` | `ffb7fb59404f3a795b5a9e5a852ea59caca4b0450dd6bdb9952b33bdd27e3b74` | tileset XML / skeleton_archer_base.tsx |
| ALI-0619 | `rusted_kingdoms/assets/sprites/enemies/skeleton_archer_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_archer_base_battle.png` | `0d7037c85c3a91f51a9fc71e1f4a034788cb18f690d13a4d4498451e7da12b23` | image / skeleton_archer_base_battle.png |
| ALI-0620 | `rusted_kingdoms/assets/sprites/enemies/skeleton_archer_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_archer_base_battle.tsx` | `b4e814259da214cbfa218343e88e0b565ca8e24617833f9553a8121f4a8e7134` | tileset XML / skeleton_archer_base_battle.tsx |
| ALI-0621 | `rusted_kingdoms/assets/sprites/enemies/skeleton_archer_greatbow_bones.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_archer_greatbow_bones.png` | `0604b8637b0ebd82b85146d26c6175031550964f647ee37e9a3e5165d705b6e4` | image / skeleton_archer_greatbow_bones.png |
| ALI-0622 | `rusted_kingdoms/assets/sprites/enemies/skeleton_archer_greatbow_bones.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_archer_greatbow_bones.tsx` | `088cf51f7c091ad58c1afd7cc251bb352b7f3eae008cbf196d82344e48d9c80c` | tileset XML / skeleton_archer_greatbow_bones.tsx |
| ALI-0623 | `rusted_kingdoms/assets/sprites/enemies/skeleton_archer_greatbow_bones_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_archer_greatbow_bones_battle.png` | `dae6d055075c10c65b371f607fa588633bc6409eb042b0e914a77124f458c008` | image / skeleton_archer_greatbow_bones_battle.png |
| ALI-0624 | `rusted_kingdoms/assets/sprites/enemies/skeleton_archer_greatbow_bones_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_archer_greatbow_bones_battle.tsx` | `ad908de003caaa678ab51dfc56a27952266e4f8f4b3da12c77f259aadbc92e0c` | tileset XML / skeleton_archer_greatbow_bones_battle.tsx |
| ALI-0625 | `rusted_kingdoms/assets/sprites/enemies/skeleton_archer_hooded_archer.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_archer_hooded_archer.png` | `0fce1d6cb3d9a444c27b2da811bdece5c37d8a302ccb2eb7aec72a9d85e7d2aa` | image / skeleton_archer_hooded_archer.png |
| ALI-0626 | `rusted_kingdoms/assets/sprites/enemies/skeleton_archer_hooded_archer.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_archer_hooded_archer.tsx` | `d984d8a8e7eeee92f5349b5905966cd8394ebce8c0ea8defe8ec7cb209a7ac1f` | tileset XML / skeleton_archer_hooded_archer.tsx |
| ALI-0627 | `rusted_kingdoms/assets/sprites/enemies/skeleton_archer_hooded_archer_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_archer_hooded_archer_battle.png` | `e9907b1e1e933a3dfbe8fd91df5c93d93900c82044a017964733ca33a5855222` | image / skeleton_archer_hooded_archer_battle.png |
| ALI-0628 | `rusted_kingdoms/assets/sprites/enemies/skeleton_archer_hooded_archer_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_archer_hooded_archer_battle.tsx` | `5840c49049360cea87c28123a12f65875ec553b632ac30ae4032809ccd2c9ddd` | tileset XML / skeleton_archer_hooded_archer_battle.tsx |
| ALI-0629 | `rusted_kingdoms/assets/sprites/enemies/skeleton_knight_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_knight_base.png` | `3847557f48d902e6c47dc76f2941f31d0ac7825bcec8b27621291f87ad524865` | image / skeleton_knight_base.png |
| ALI-0630 | `rusted_kingdoms/assets/sprites/enemies/skeleton_knight_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_knight_base.tsx` | `804436161cf3a81253a50c2490a6630c69c0e4d29be60f4ff4ea8398a686e07d` | tileset XML / skeleton_knight_base.tsx |
| ALI-0631 | `rusted_kingdoms/assets/sprites/enemies/skeleton_knight_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_knight_base_battle.png` | `d4c87da15144bb31c9ae9db003c2e6c7a8a6942d4452bf9da7717881b460f549` | image / skeleton_knight_base_battle.png |
| ALI-0632 | `rusted_kingdoms/assets/sprites/enemies/skeleton_knight_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_knight_base_battle.tsx` | `95f664cd0cdf176b769c924defe1e322ccd983bcdad00bc3c71c8e506652a874` | tileset XML / skeleton_knight_base_battle.tsx |
| ALI-0633 | `rusted_kingdoms/assets/sprites/enemies/skeleton_knight_black_knight.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_knight_black_knight.png` | `75a597edc879efed61396f7c865a2ade641d82aeb69821f73630c5b1e543ba9f` | image / skeleton_knight_black_knight.png |
| ALI-0634 | `rusted_kingdoms/assets/sprites/enemies/skeleton_knight_black_knight.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_knight_black_knight.tsx` | `63bfa0237e6052c9787d8a9b9aa5a0f851d4377f93596850f9be2573a47c0b32` | tileset XML / skeleton_knight_black_knight.tsx |
| ALI-0635 | `rusted_kingdoms/assets/sprites/enemies/skeleton_knight_black_knight_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_knight_black_knight_battle.png` | `49608659fa5666e6a79c35b55cde8abac96c404e2289495f2e53e75d8e595609` | image / skeleton_knight_black_knight_battle.png |
| ALI-0636 | `rusted_kingdoms/assets/sprites/enemies/skeleton_knight_black_knight_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_knight_black_knight_battle.tsx` | `03e44b0fd4ea066c8321bb9d24490ec8444953e18e7e32af4918ae37defd2d8a` | tileset XML / skeleton_knight_black_knight_battle.tsx |
| ALI-0637 | `rusted_kingdoms/assets/sprites/enemies/skeleton_knight_mace_guard.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_knight_mace_guard.png` | `b6e606fb267c79d9fd6699fecb7c3b50aad15eee994134574a6c6052018a5f13` | image / skeleton_knight_mace_guard.png |
| ALI-0638 | `rusted_kingdoms/assets/sprites/enemies/skeleton_knight_mace_guard.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_knight_mace_guard.tsx` | `6ce1298948315354ecc1fefb0c2343b0f05978b84a92d763b228f3038ecce8fb` | tileset XML / skeleton_knight_mace_guard.tsx |
| ALI-0639 | `rusted_kingdoms/assets/sprites/enemies/skeleton_knight_mace_guard_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_knight_mace_guard_battle.png` | `a5ed033540b57aa593ed84c8108edd42c04cd931de56c5d1fa4851a6655a7c9b` | image / skeleton_knight_mace_guard_battle.png |
| ALI-0640 | `rusted_kingdoms/assets/sprites/enemies/skeleton_knight_mace_guard_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_knight_mace_guard_battle.tsx` | `54b14819013366d3f114b59f9ec31ba0437ba6c5ab9ec32b130f522c3fd9ac71` | tileset XML / skeleton_knight_mace_guard_battle.tsx |
| ALI-0641 | `rusted_kingdoms/assets/sprites/enemies/skeleton_monster_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_monster_base.png` | `bdc44050ed18f2f363e337c0d1f99f201760c6a419801d97919bf95fb053a473` | image / skeleton_monster_base.png |
| ALI-0642 | `rusted_kingdoms/assets/sprites/enemies/skeleton_monster_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_monster_base.tsx` | `3b439058668d2f5912b60d09a09fa9b3b5b148b6a312bb1dad861a7ac0670940` | tileset XML / skeleton_monster_base.tsx |
| ALI-0643 | `rusted_kingdoms/assets/sprites/enemies/skeleton_monster_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_monster_base_battle.png` | `283aa3c2d5779e6f7f41399e60e2ceeb33b1a7bbc2ffd3b070ce238903c31e21` | image / skeleton_monster_base_battle.png |
| ALI-0644 | `rusted_kingdoms/assets/sprites/enemies/skeleton_monster_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_monster_base_battle.tsx` | `ca83877d9dad1966d481932f23e7412076faa6c472aaae3bce17f5ee9b3694c4` | tileset XML / skeleton_monster_base_battle.tsx |
| ALI-0645 | `rusted_kingdoms/assets/sprites/enemies/skeleton_monster_hooded_bones.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_monster_hooded_bones.png` | `ec76d189b6b3ed909d93026c66080c65175a3cd0d53a466546b1a4b4ba0d3cc1` | image / skeleton_monster_hooded_bones.png |
| ALI-0646 | `rusted_kingdoms/assets/sprites/enemies/skeleton_monster_hooded_bones.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_monster_hooded_bones.tsx` | `2e58598940f4a495c7c05e9ed116f751b11d257218bc7ca1f9f46c589f61c884` | tileset XML / skeleton_monster_hooded_bones.tsx |
| ALI-0647 | `rusted_kingdoms/assets/sprites/enemies/skeleton_monster_hooded_bones_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_monster_hooded_bones_battle.png` | `6b47a70150033cfb0c3754cd6109e7da7e502a451917e0e8fdb44b18378dea2e` | image / skeleton_monster_hooded_bones_battle.png |
| ALI-0648 | `rusted_kingdoms/assets/sprites/enemies/skeleton_monster_hooded_bones_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_monster_hooded_bones_battle.tsx` | `c70a51ca3b603aef6c8681db0c3b8783fe49aea04b2f666ffb9c2623cf70a5de` | tileset XML / skeleton_monster_hooded_bones_battle.tsx |
| ALI-0649 | `rusted_kingdoms/assets/sprites/enemies/skeleton_monster_rusted_sword.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_monster_rusted_sword.png` | `bfa4bee3118818acf8b6c62ff26297fdbd4cbf5a9c076600a5053874264dbc01` | image / skeleton_monster_rusted_sword.png |
| ALI-0650 | `rusted_kingdoms/assets/sprites/enemies/skeleton_monster_rusted_sword.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_monster_rusted_sword.tsx` | `db8df90faad347a3cf4325eacf7c699b337bb6f68a6978d126cf9cc487f8b393` | tileset XML / skeleton_monster_rusted_sword.tsx |
| ALI-0651 | `rusted_kingdoms/assets/sprites/enemies/skeleton_monster_rusted_sword_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_monster_rusted_sword_battle.png` | `a2befd1c74f6f90ddec1a01d9efa5634d944009a15239b10378bec042d463962` | image / skeleton_monster_rusted_sword_battle.png |
| ALI-0652 | `rusted_kingdoms/assets/sprites/enemies/skeleton_monster_rusted_sword_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/skeleton_monster_rusted_sword_battle.tsx` | `2207d23d05ebea87636c6ee21797b780966dea24f9a4eb6a4dea7096fc5efc36` | tileset XML / skeleton_monster_rusted_sword_battle.tsx |
| ALI-0653 | `rusted_kingdoms/assets/sprites/enemies/troll_berserker_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/troll_berserker_base.png` | `2053ad1c7d2aac7cce6d9fd2d2385bdab795931dbe1d72a39120d65e08ad2f2e` | image / troll_berserker_base.png |
| ALI-0654 | `rusted_kingdoms/assets/sprites/enemies/troll_berserker_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/troll_berserker_base.tsx` | `03dd81b81f1331e5f14a0885fcb4369633bc05a448b1e6518d09ebe4821a3869` | tileset XML / troll_berserker_base.tsx |
| ALI-0655 | `rusted_kingdoms/assets/sprites/enemies/troll_berserker_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/troll_berserker_base_battle.png` | `6e493e35a78aed4610d26b595e816e14c2f20a7cddf56ba8b7b0254197b5802b` | image / troll_berserker_base_battle.png |
| ALI-0656 | `rusted_kingdoms/assets/sprites/enemies/troll_berserker_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/troll_berserker_base_battle.tsx` | `30bdf2d8656be7d12696f848200a5f35c791d1d68b0ccae51380a7b8a767567b` | tileset XML / troll_berserker_base_battle.tsx |
| ALI-0657 | `rusted_kingdoms/assets/sprites/enemies/troll_berserker_stone_troll.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/troll_berserker_stone_troll.png` | `f9e0ae7218f54feb2e0048792f807e6a20bd797715cc079a0a0d0f9a45b6d5d2` | image / troll_berserker_stone_troll.png |
| ALI-0658 | `rusted_kingdoms/assets/sprites/enemies/troll_berserker_stone_troll.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/troll_berserker_stone_troll.tsx` | `b01c5b03cd9fd3c9223462700f182e14265e3514e4a5779d1b52c4706594a3fe` | tileset XML / troll_berserker_stone_troll.tsx |
| ALI-0659 | `rusted_kingdoms/assets/sprites/enemies/troll_berserker_stone_troll_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/troll_berserker_stone_troll_battle.png` | `4809f2d6be4dbc687e89c99ee29f2dae1ebe9e7a7efe64258913e72286e27c7d` | image / troll_berserker_stone_troll_battle.png |
| ALI-0660 | `rusted_kingdoms/assets/sprites/enemies/troll_berserker_stone_troll_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/troll_berserker_stone_troll_battle.tsx` | `f59dac865072919b3195dcf894567ea31e89a8894df4146dd19b900b1104df69` | tileset XML / troll_berserker_stone_troll_battle.tsx |
| ALI-0661 | `rusted_kingdoms/assets/sprites/enemies/troll_berserker_swamp_troll.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/troll_berserker_swamp_troll.png` | `d1289ca5abf708446021f06961c4f5a0e3e1d9658fae5d2b6fb9a79d8035beb5` | image / troll_berserker_swamp_troll.png |
| ALI-0662 | `rusted_kingdoms/assets/sprites/enemies/troll_berserker_swamp_troll.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/troll_berserker_swamp_troll.tsx` | `717d9318261d6675000c6acdd90adf3ef30365a6b78d440207b8a5a23c23acf4` | tileset XML / troll_berserker_swamp_troll.tsx |
| ALI-0663 | `rusted_kingdoms/assets/sprites/enemies/troll_berserker_swamp_troll_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/troll_berserker_swamp_troll_battle.png` | `62184fb00fb2a07d28f89df0e59f8dfeba42c1ed6e9d22cdf4325fa60577eb29` | image / troll_berserker_swamp_troll_battle.png |
| ALI-0664 | `rusted_kingdoms/assets/sprites/enemies/troll_berserker_swamp_troll_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/troll_berserker_swamp_troll_battle.tsx` | `b411d691c569709ea0372f8bb99aaa5a486020858e3fbe5c35007670ea5f0be5` | tileset XML / troll_berserker_swamp_troll_battle.tsx |
| ALI-0665 | `rusted_kingdoms/assets/sprites/enemies/troll_shaman_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/troll_shaman_base.png` | `63a88c13bfb8368c5cd1a944613b26f8fed6fe62ef77cd9364480a559f3684fa` | image / troll_shaman_base.png |
| ALI-0666 | `rusted_kingdoms/assets/sprites/enemies/troll_shaman_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/troll_shaman_base.tsx` | `fa4986b1f7fba65a9a760ee3d6917ca524df719deca0829ced1333f336661a6e` | tileset XML / troll_shaman_base.tsx |
| ALI-0667 | `rusted_kingdoms/assets/sprites/enemies/troll_shaman_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/troll_shaman_base_battle.png` | `9cc492384aaba85df4dce18721e49666f0bb286cf758ccf82bd0058c5e8e0c84` | image / troll_shaman_base_battle.png |
| ALI-0668 | `rusted_kingdoms/assets/sprites/enemies/troll_shaman_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/troll_shaman_base_battle.tsx` | `ed5cacd1cd5f94bebe9dfb8a31cb1663575507fa4caf03c0c5517a21c51623f8` | tileset XML / troll_shaman_base_battle.tsx |
| ALI-0669 | `rusted_kingdoms/assets/sprites/enemies/troll_shaman_crystal_sage.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/troll_shaman_crystal_sage.png` | `6a7f72eb67c0560842c8485fb87bfeb48f93265113515af12f44dfb7f70e81d4` | image / troll_shaman_crystal_sage.png |
| ALI-0670 | `rusted_kingdoms/assets/sprites/enemies/troll_shaman_crystal_sage.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/troll_shaman_crystal_sage.tsx` | `fde2d3616d4d10c021114e51354cc7edf1f1e7bb02cdf20d8b9a89e3b12245f9` | tileset XML / troll_shaman_crystal_sage.tsx |
| ALI-0671 | `rusted_kingdoms/assets/sprites/enemies/troll_shaman_crystal_sage_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/troll_shaman_crystal_sage_battle.png` | `fd27ee27a37ae2e440876fe221f0a0c48268f067e77a9c3e5aee929d8b88406d` | image / troll_shaman_crystal_sage_battle.png |
| ALI-0672 | `rusted_kingdoms/assets/sprites/enemies/troll_shaman_crystal_sage_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/troll_shaman_crystal_sage_battle.tsx` | `d30c4d54878300f4a2b49c58d756a4725a07d2e66c79a42b40fa059a8acdbc0f` | tileset XML / troll_shaman_crystal_sage_battle.tsx |
| ALI-0673 | `rusted_kingdoms/assets/sprites/enemies/troll_shaman_swamp_hexer.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/troll_shaman_swamp_hexer.png` | `d2a48d745de21a7c7dec96c825224cd1eddf3d62c56d6a0c08c14278ee72b8bb` | image / troll_shaman_swamp_hexer.png |
| ALI-0674 | `rusted_kingdoms/assets/sprites/enemies/troll_shaman_swamp_hexer.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/troll_shaman_swamp_hexer.tsx` | `71a78331019d28fa2290d561842b7c1fd0a1c1edd2518dc5f0e2044c4c2f2c68` | tileset XML / troll_shaman_swamp_hexer.tsx |
| ALI-0675 | `rusted_kingdoms/assets/sprites/enemies/troll_shaman_swamp_hexer_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/troll_shaman_swamp_hexer_battle.png` | `b3977016e00a81ea7b22e109820e67588afa3da122061cdfef3a229ac830d677` | image / troll_shaman_swamp_hexer_battle.png |
| ALI-0676 | `rusted_kingdoms/assets/sprites/enemies/troll_shaman_swamp_hexer_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/troll_shaman_swamp_hexer_battle.tsx` | `c0d7229e843e3b665a0f6077f025189348db12e9fcc597c84f1344ab86abb29d` | tileset XML / troll_shaman_swamp_hexer_battle.tsx |
| ALI-0677 | `rusted_kingdoms/assets/sprites/enemies/vampire_bat_lord_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/vampire_bat_lord_base.png` | `bba009537504d8c551b39309050955134a51417dce88d2c010007d7a7a196339` | image / vampire_bat_lord_base.png |
| ALI-0678 | `rusted_kingdoms/assets/sprites/enemies/vampire_bat_lord_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/vampire_bat_lord_base.tsx` | `5b9f1597f66f73f2e83ad6e25a4c3180635f35f5c0d6cbbb4e4383026d3ee2b5` | tileset XML / vampire_bat_lord_base.tsx |
| ALI-0679 | `rusted_kingdoms/assets/sprites/enemies/vampire_bat_lord_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/vampire_bat_lord_base_battle.png` | `894ba59f4671d4045127a1ba07026fad099dab8611d2bdd1c657a2715672d6f4` | image / vampire_bat_lord_base_battle.png |
| ALI-0680 | `rusted_kingdoms/assets/sprites/enemies/vampire_bat_lord_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/vampire_bat_lord_base_battle.tsx` | `c69665280db8e834ce1ce1d03dcdfe49047333135eb011c3273b86427f7b8b39` | tileset XML / vampire_bat_lord_base_battle.tsx |
| ALI-0681 | `rusted_kingdoms/assets/sprites/enemies/vampire_bat_lord_blood_saber_lord.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/vampire_bat_lord_blood_saber_lord.png` | `e839a5f42a7409bdede85f051edcbd869a9e5ed3b9fbbb15003ffbc066ec784c` | image / vampire_bat_lord_blood_saber_lord.png |
| ALI-0682 | `rusted_kingdoms/assets/sprites/enemies/vampire_bat_lord_blood_saber_lord.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/vampire_bat_lord_blood_saber_lord.tsx` | `d4da894f87f16290ba79e3fb801d16b4728cbb504b4b2771f325c96f19aec013` | tileset XML / vampire_bat_lord_blood_saber_lord.tsx |
| ALI-0683 | `rusted_kingdoms/assets/sprites/enemies/vampire_bat_lord_blood_saber_lord_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/vampire_bat_lord_blood_saber_lord_battle.png` | `0eaba6740ac4fa706a295a0d93d8a16551add30225566f940b55e99aa51f3106` | image / vampire_bat_lord_blood_saber_lord_battle.png |
| ALI-0684 | `rusted_kingdoms/assets/sprites/enemies/vampire_bat_lord_blood_saber_lord_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/vampire_bat_lord_blood_saber_lord_battle.tsx` | `9c5b2b025b3cf63e13823ad9f94c45be944c34f822e319123d92c359cc7432e9` | tileset XML / vampire_bat_lord_blood_saber_lord_battle.tsx |
| ALI-0685 | `rusted_kingdoms/assets/sprites/enemies/vampire_bat_lord_white_wing_lord.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/vampire_bat_lord_white_wing_lord.png` | `6038c90733b7f4f272af71ba803c00dc7fce5484993f48ca833cf3da363cea49` | image / vampire_bat_lord_white_wing_lord.png |
| ALI-0686 | `rusted_kingdoms/assets/sprites/enemies/vampire_bat_lord_white_wing_lord.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/vampire_bat_lord_white_wing_lord.tsx` | `79b70eb46311b038e03689e72f0d8c388630b8d0e063973415d40fb9938cd0db` | tileset XML / vampire_bat_lord_white_wing_lord.tsx |
| ALI-0687 | `rusted_kingdoms/assets/sprites/enemies/vampire_bat_lord_white_wing_lord_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/vampire_bat_lord_white_wing_lord_battle.png` | `f424056ba15cd5c9786d97ba12b808eec5105555b5394d3e6106ecbfa36513ae` | image / vampire_bat_lord_white_wing_lord_battle.png |
| ALI-0688 | `rusted_kingdoms/assets/sprites/enemies/vampire_bat_lord_white_wing_lord_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/vampire_bat_lord_white_wing_lord_battle.tsx` | `206d00deb128053314459458107a5f58af626c0ef24bbaf08fb53aa1cfd456db` | tileset XML / vampire_bat_lord_white_wing_lord_battle.tsx |
| ALI-0689 | `rusted_kingdoms/assets/sprites/enemies/vampire_noble_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/vampire_noble_base.png` | `9092d5be1a6f743fa5fd7e82c8c8172a015a08d52d8d451e77ca5b27d6c33e2b` | image / vampire_noble_base.png |
| ALI-0690 | `rusted_kingdoms/assets/sprites/enemies/vampire_noble_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/vampire_noble_base.tsx` | `88c7524c663bf1d5fd9c6ebf22de83bafe1085e013c59bb8d61e508b3737dc4b` | tileset XML / vampire_noble_base.tsx |
| ALI-0691 | `rusted_kingdoms/assets/sprites/enemies/vampire_noble_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/vampire_noble_base_battle.png` | `bc411ca4e2a70d462cc0776282a609e7f55f3077b8450e5fa8be90f365e2322f` | image / vampire_noble_base_battle.png |
| ALI-0692 | `rusted_kingdoms/assets/sprites/enemies/vampire_noble_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/vampire_noble_base_battle.tsx` | `52939b2357a84662a5c5fe315e5bd9ec49ef188b94e355663f0de2277d18a03b` | tileset XML / vampire_noble_base_battle.tsx |
| ALI-0693 | `rusted_kingdoms/assets/sprites/enemies/vampire_noble_blood_court_duelist.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/vampire_noble_blood_court_duelist.png` | `d65b18faff7e4746e206af6c411ef0bff7f5ee187e9beff217f838aabb4faa02` | image / vampire_noble_blood_court_duelist.png |
| ALI-0694 | `rusted_kingdoms/assets/sprites/enemies/vampire_noble_blood_court_duelist.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/vampire_noble_blood_court_duelist.tsx` | `5b81faab18b80a549e17c73a9736f48e0b2df26a525b51acfd096a7a0e2cd6a0` | tileset XML / vampire_noble_blood_court_duelist.tsx |
| ALI-0695 | `rusted_kingdoms/assets/sprites/enemies/vampire_noble_blood_court_duelist_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/vampire_noble_blood_court_duelist_battle.png` | `695b4a86adddcd546ae79807ff9f9f267962e4e500dc87aa095018afaf1aa63d` | image / vampire_noble_blood_court_duelist_battle.png |
| ALI-0696 | `rusted_kingdoms/assets/sprites/enemies/vampire_noble_blood_court_duelist_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/vampire_noble_blood_court_duelist_battle.tsx` | `dc4beffe7fc895a9286abd42f8b732f969da2da3d1e50494bd43bf97c5cd7560` | tileset XML / vampire_noble_blood_court_duelist_battle.tsx |
| ALI-0697 | `rusted_kingdoms/assets/sprites/enemies/vampire_noble_masked_vampire.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/vampire_noble_masked_vampire.png` | `d88d2b022c198648b1fdd3abcf055859f68e8adc12550d7467996687d2f24e49` | image / vampire_noble_masked_vampire.png |
| ALI-0698 | `rusted_kingdoms/assets/sprites/enemies/vampire_noble_masked_vampire.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/vampire_noble_masked_vampire.tsx` | `4b61cc46279b42ff63f3fa58306f3d9f0fa133d29861f46b1759fa4f1aae7f79` | tileset XML / vampire_noble_masked_vampire.tsx |
| ALI-0699 | `rusted_kingdoms/assets/sprites/enemies/vampire_noble_masked_vampire_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/vampire_noble_masked_vampire_battle.png` | `31c5db39261bd4d374a647943b5e91567d089116f7917bcd0b8d95f99bd0f44d` | image / vampire_noble_masked_vampire_battle.png |
| ALI-0700 | `rusted_kingdoms/assets/sprites/enemies/vampire_noble_masked_vampire_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/vampire_noble_masked_vampire_battle.tsx` | `fdcfa3681e8eaaaa98729dd17d5239b2d60209a10a435d1f921e8d09f8794de5` | tileset XML / vampire_noble_masked_vampire_battle.tsx |
| ALI-0701 | `rusted_kingdoms/assets/sprites/enemies/wartotaur_warlord_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/wartotaur_warlord_base.png` | `8d6397ae9dc16e0efad2ddde89e3c44a095b374ee7a3487b997a938fa4d32934` | image / wartotaur_warlord_base.png |
| ALI-0702 | `rusted_kingdoms/assets/sprites/enemies/wartotaur_warlord_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/wartotaur_warlord_base.tsx` | `1438261d74546587aac9b573b62dbe48e0c08d6b32a7d79e2b943ba71b9f47a7` | tileset XML / wartotaur_warlord_base.tsx |
| ALI-0703 | `rusted_kingdoms/assets/sprites/enemies/wartotaur_warlord_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/wartotaur_warlord_base_battle.png` | `18535792e67e78410688cd91818a192526db2392bbacf554473926ac15c983b3` | image / wartotaur_warlord_base_battle.png |
| ALI-0704 | `rusted_kingdoms/assets/sprites/enemies/wartotaur_warlord_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/wartotaur_warlord_base_battle.tsx` | `7d82fd92c40ef99bf9341dee2643719c236b002cfd93b2420225f828c0a4eb0a` | tileset XML / wartotaur_warlord_base_battle.tsx |
| ALI-0705 | `rusted_kingdoms/assets/sprites/enemies/wartotaur_warlord_blackhorn_chief.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/wartotaur_warlord_blackhorn_chief.png` | `7dcaadbb49177d6ad1e8541e49d7d6de4ecb8cf3ada74d02e0dedddc24d3fe8a` | image / wartotaur_warlord_blackhorn_chief.png |
| ALI-0706 | `rusted_kingdoms/assets/sprites/enemies/wartotaur_warlord_blackhorn_chief.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/wartotaur_warlord_blackhorn_chief.tsx` | `3d8767103a3c0de7dcc68609f23d9b0d84250222642b2c287b27c6e552a19360` | tileset XML / wartotaur_warlord_blackhorn_chief.tsx |
| ALI-0707 | `rusted_kingdoms/assets/sprites/enemies/wartotaur_warlord_blackhorn_chief_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/wartotaur_warlord_blackhorn_chief_battle.png` | `56d054f7b53cbd1259c18ff8f5e56c4dcd54af595e676ed44337c24a66c5813d` | image / wartotaur_warlord_blackhorn_chief_battle.png |
| ALI-0708 | `rusted_kingdoms/assets/sprites/enemies/wartotaur_warlord_blackhorn_chief_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/wartotaur_warlord_blackhorn_chief_battle.tsx` | `c57a8e83f4ffc80b487b4d82e5db24ec7c2d2d7f553c8bb95410be0390c52a03` | tileset XML / wartotaur_warlord_blackhorn_chief_battle.tsx |
| ALI-0709 | `rusted_kingdoms/assets/sprites/enemies/wartotaur_warlord_bronze_halberdier.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/wartotaur_warlord_bronze_halberdier.png` | `f2ef31e154c116ba922bd6270ddf97b69728e04e67a919f3029c51fc8afaaab4` | image / wartotaur_warlord_bronze_halberdier.png |
| ALI-0710 | `rusted_kingdoms/assets/sprites/enemies/wartotaur_warlord_bronze_halberdier.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/wartotaur_warlord_bronze_halberdier.tsx` | `5d1831dd7690ee14e9899f987f044279b98cd271297085b4579a5664cfaa0a9f` | tileset XML / wartotaur_warlord_bronze_halberdier.tsx |
| ALI-0711 | `rusted_kingdoms/assets/sprites/enemies/wartotaur_warlord_bronze_halberdier_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/wartotaur_warlord_bronze_halberdier_battle.png` | `30743e57beb2eeed198f44a279e5654b650f40687920d608bbcadd80b88024ab` | image / wartotaur_warlord_bronze_halberdier_battle.png |
| ALI-0712 | `rusted_kingdoms/assets/sprites/enemies/wartotaur_warlord_bronze_halberdier_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/wartotaur_warlord_bronze_halberdier_battle.tsx` | `b34cd1ce9e2827fece34d5c13643d3e53a9fd08d85d25743f1a6b07a6967b923` | tileset XML / wartotaur_warlord_bronze_halberdier_battle.tsx |
| ALI-0713 | `rusted_kingdoms/assets/sprites/enemies/wolf_beast_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/wolf_beast_base.png` | `2d4f8c2e0f53cd6d96ab31492ed1a8b89db05f166a536effd7d2da58d73d7b0f` | image / wolf_beast_base.png |
| ALI-0714 | `rusted_kingdoms/assets/sprites/enemies/wolf_beast_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/wolf_beast_base.tsx` | `1f0469b2e7645732cb0084dd9fcd20dc642a7e45bce024dc91cf8730045fe13e` | tileset XML / wolf_beast_base.tsx |
| ALI-0715 | `rusted_kingdoms/assets/sprites/enemies/wolf_beast_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/wolf_beast_base_battle.png` | `e495d132d5988d385b2dc7a5c58bacde359f955ed46aa1199955f3769d81a908` | image / wolf_beast_base_battle.png |
| ALI-0716 | `rusted_kingdoms/assets/sprites/enemies/wolf_beast_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/wolf_beast_base_battle.tsx` | `40aece308ba72df12e2e24c1cd5e7d1bf326e753850b13c1e3f6e931cc8caf76` | tileset XML / wolf_beast_base_battle.tsx |
| ALI-0717 | `rusted_kingdoms/assets/sprites/enemies/wolf_beast_black_fur.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/wolf_beast_black_fur.png` | `8b5e983d8cec782771a5c528ab8a1678fd7a52f7e37b23f16368d5234d03640e` | image / wolf_beast_black_fur.png |
| ALI-0718 | `rusted_kingdoms/assets/sprites/enemies/wolf_beast_black_fur.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/wolf_beast_black_fur.tsx` | `7121a8e2828042d82991d2cab23d4d3edda2daae0892d4fd1c91dc31ec13623e` | tileset XML / wolf_beast_black_fur.tsx |
| ALI-0719 | `rusted_kingdoms/assets/sprites/enemies/wolf_beast_black_fur_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/wolf_beast_black_fur_battle.png` | `de92ecc7e82ab1090086d9aa163ca0ac79c5f1dfb34cfbcdfb399d7d2321303c` | image / wolf_beast_black_fur_battle.png |
| ALI-0720 | `rusted_kingdoms/assets/sprites/enemies/wolf_beast_black_fur_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/wolf_beast_black_fur_battle.tsx` | `0a23a564d330d29abebcab1279f3075df409cc334c51367a8a1d10ce25b63067` | tileset XML / wolf_beast_black_fur_battle.tsx |
| ALI-0721 | `rusted_kingdoms/assets/sprites/enemies/wolf_beast_spear_beast.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/wolf_beast_spear_beast.png` | `4074788820bba67516727e162dd3b64a5f8075cf093e66eb19b60f253f05afd0` | image / wolf_beast_spear_beast.png |
| ALI-0722 | `rusted_kingdoms/assets/sprites/enemies/wolf_beast_spear_beast.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/wolf_beast_spear_beast.tsx` | `5f7ab59a7662c4d09b27d140e50881eb777bbe2c1a4d43f112de1ef228e3c29b` | tileset XML / wolf_beast_spear_beast.tsx |
| ALI-0723 | `rusted_kingdoms/assets/sprites/enemies/wolf_beast_spear_beast_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/wolf_beast_spear_beast_battle.png` | `048c020e5c2a519c91ca3523754adb897d0b5e3aa3208eb42df56b4f3ee7beaa` | image / wolf_beast_spear_beast_battle.png |
| ALI-0724 | `rusted_kingdoms/assets/sprites/enemies/wolf_beast_spear_beast_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/wolf_beast_spear_beast_battle.tsx` | `b3c9fa4ed40e91decf26e2e0aaaaff6a3180f0ffcb6daeb7fb5043f369959f87` | tileset XML / wolf_beast_spear_beast_battle.tsx |
| ALI-0725 | `rusted_kingdoms/assets/sprites/enemies/zombie_guard_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/zombie_guard_base.png` | `de6f1f1b6ebe50a39d103021f9b9cf419a68ab4c765e77f8b16c182424e97926` | image / zombie_guard_base.png |
| ALI-0726 | `rusted_kingdoms/assets/sprites/enemies/zombie_guard_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/zombie_guard_base.tsx` | `ce75f6ab6530a4945b4befcaabf6f75a0622380029763f36404875fbf2d9076a` | tileset XML / zombie_guard_base.tsx |
| ALI-0727 | `rusted_kingdoms/assets/sprites/enemies/zombie_guard_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/zombie_guard_base_battle.png` | `d1c88dda02ab3c15b0a3a0b49efdd80cbe29989b8ff096160e08eb712455b115` | image / zombie_guard_base_battle.png |
| ALI-0728 | `rusted_kingdoms/assets/sprites/enemies/zombie_guard_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/zombie_guard_base_battle.tsx` | `e876f2857d41161287fce782190beefc64b2d11742954869a9ab1b2f33f09635` | tileset XML / zombie_guard_base_battle.tsx |
| ALI-0729 | `rusted_kingdoms/assets/sprites/enemies/zombie_guard_rusted_sentry.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/zombie_guard_rusted_sentry.png` | `6e322dc451bfb18111a8a20bac53f15a1b675605b385c61281e08524e98f4311` | image / zombie_guard_rusted_sentry.png |
| ALI-0730 | `rusted_kingdoms/assets/sprites/enemies/zombie_guard_rusted_sentry.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/zombie_guard_rusted_sentry.tsx` | `8830d36abafd5deb0a2e51563c5986e8835b6e798a35b16e9672c9fa20605368` | tileset XML / zombie_guard_rusted_sentry.tsx |
| ALI-0731 | `rusted_kingdoms/assets/sprites/enemies/zombie_guard_rusted_sentry_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/zombie_guard_rusted_sentry_battle.png` | `4dc2889b6b58f3a4ca418d1029d3ee4363cf58edc88b8b857ed84881d1cfc3bf` | image / zombie_guard_rusted_sentry_battle.png |
| ALI-0732 | `rusted_kingdoms/assets/sprites/enemies/zombie_guard_rusted_sentry_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/zombie_guard_rusted_sentry_battle.tsx` | `e4ca2613f69f535d093fcab9aeb125cc13ad488d6776a8296b42bfa6aa0b994e` | tileset XML / zombie_guard_rusted_sentry_battle.tsx |
| ALI-0733 | `rusted_kingdoms/assets/sprites/enemies/zombie_guard_wounded_guard.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/zombie_guard_wounded_guard.png` | `c70a33b85b447f75addba092f2cb9a96eb67c7abda2d5e36e49cf9edeb75f26b` | image / zombie_guard_wounded_guard.png |
| ALI-0734 | `rusted_kingdoms/assets/sprites/enemies/zombie_guard_wounded_guard.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/zombie_guard_wounded_guard.tsx` | `6029eb0d8a86a2f8c562dbb0361ad5c7572f65a5fc24af3993b34ec369ed5e10` | tileset XML / zombie_guard_wounded_guard.tsx |
| ALI-0735 | `rusted_kingdoms/assets/sprites/enemies/zombie_guard_wounded_guard_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/zombie_guard_wounded_guard_battle.png` | `d1c88dda02ab3c15b0a3a0b49efdd80cbe29989b8ff096160e08eb712455b115` | image / zombie_guard_wounded_guard_battle.png |
| ALI-0736 | `rusted_kingdoms/assets/sprites/enemies/zombie_guard_wounded_guard_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/zombie_guard_wounded_guard_battle.tsx` | `67c8a6e9ff65f102cafa8db20420f7f8fac2af9c48a507d0646e50a4ed8cd9f7` | tileset XML / zombie_guard_wounded_guard_battle.tsx |
| ALI-0737 | `rusted_kingdoms/assets/sprites/enemies/zombie_monster_armed_zombie.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/zombie_monster_armed_zombie.png` | `8dd2d98548b924475df0ddd28a1d8cd5ca6a78d60c4e287e78700192960a9bed` | image / zombie_monster_armed_zombie.png |
| ALI-0738 | `rusted_kingdoms/assets/sprites/enemies/zombie_monster_armed_zombie.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/zombie_monster_armed_zombie.tsx` | `c11e4395bed26d2773550a0159f45a2ecb918a2d97373a9a64aef0c83a6b7adf` | tileset XML / zombie_monster_armed_zombie.tsx |
| ALI-0739 | `rusted_kingdoms/assets/sprites/enemies/zombie_monster_armed_zombie_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/zombie_monster_armed_zombie_battle.png` | `be7ef1cc5d4875eb87a8c5b889cea71fb823365d4879d35fe7bf3f50fbbe9ab4` | image / zombie_monster_armed_zombie_battle.png |
| ALI-0740 | `rusted_kingdoms/assets/sprites/enemies/zombie_monster_armed_zombie_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/zombie_monster_armed_zombie_battle.tsx` | `67d072e99a0307dfad6825f50ad51a38d834af029786d110adb937090e21a80e` | tileset XML / zombie_monster_armed_zombie_battle.tsx |
| ALI-0741 | `rusted_kingdoms/assets/sprites/enemies/zombie_monster_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/zombie_monster_base.png` | `a96f586ef784ce9e655275acf85b96bf226534b324e9c099cb6114f8fcde7d1b` | image / zombie_monster_base.png |
| ALI-0742 | `rusted_kingdoms/assets/sprites/enemies/zombie_monster_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/zombie_monster_base.tsx` | `7c800cb135ba8fa81aba0d96f2c66a7bfd45075f2f38044c299c1ac42c1eed25` | tileset XML / zombie_monster_base.tsx |
| ALI-0743 | `rusted_kingdoms/assets/sprites/enemies/zombie_monster_base_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/zombie_monster_base_battle.png` | `dfba2b382e7395da68c7b5d26d490b4ecf3b41551155bf0fbd81f19068aa0161` | image / zombie_monster_base_battle.png |
| ALI-0744 | `rusted_kingdoms/assets/sprites/enemies/zombie_monster_base_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/zombie_monster_base_battle.tsx` | `2bec00aa0cb06c4a51c187010fc06c8159f07bc9d8ef7c228a320b42a8f1fd55` | tileset XML / zombie_monster_base_battle.tsx |
| ALI-0745 | `rusted_kingdoms/assets/sprites/enemies/zombie_monster_ragged_worker.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/zombie_monster_ragged_worker.png` | `7f58d60c1474873d1b10f9494a733e28cdf7bb5398ba26800d73d8dd22ffcf2e` | image / zombie_monster_ragged_worker.png |
| ALI-0746 | `rusted_kingdoms/assets/sprites/enemies/zombie_monster_ragged_worker.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/zombie_monster_ragged_worker.tsx` | `c21cdb0f063667ecf0544a7314058f56aeb6d4cdcf4570a48affe833b2cea984` | tileset XML / zombie_monster_ragged_worker.tsx |
| ALI-0747 | `rusted_kingdoms/assets/sprites/enemies/zombie_monster_ragged_worker_battle.png` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/zombie_monster_ragged_worker_battle.png` | `81eac81de1ed9bc2982d48d660715c4ae5cde95d6a70c1b2611311c891cb18b9` | image / zombie_monster_ragged_worker_battle.png |
| ALI-0748 | `rusted_kingdoms/assets/sprites/enemies/zombie_monster_ragged_worker_battle.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/zombie_monster_ragged_worker_battle.tsx` | `4500ad0465d73fcf380e4892412b50a72cf6c5249c9e8eec0a75e22839fdfee7` | tileset XML / zombie_monster_ragged_worker_battle.tsx |
| ALI-0749 | `rusted_kingdoms/assets/sprites/npc/archer_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/archer_base.png` | `9fb8d1d8abb9f10fa09c98e9ad07d78aa254cbedd6a6f45eeb3c6e770eb33007` | image / archer_base.png |
| ALI-0750 | `rusted_kingdoms/assets/sprites/npc/archer_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/archer_base.tsx` | `836b51de7921a023a3954e2b2808a76bb044c47962732a6bbe489ab6718bde4a` | tileset XML / archer_base.tsx |
| ALI-0751 | `rusted_kingdoms/assets/sprites/npc/archer_crossbow_hunter.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/archer_crossbow_hunter.png` | `326534f93b74df77de392758fb33a77497436063f9ac2299600587b493e07c0f` | image / archer_crossbow_hunter.png |
| ALI-0752 | `rusted_kingdoms/assets/sprites/npc/archer_crossbow_hunter.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/archer_crossbow_hunter.tsx` | `28c870191656d2ec6fb97662be66f9a370124305902daea45e687d95a7713da8` | tileset XML / archer_crossbow_hunter.tsx |
| ALI-0753 | `rusted_kingdoms/assets/sprites/npc/archer_recurve_scout.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/archer_recurve_scout.png` | `6fd8dc7e7b07500f3a6e3f51549a182695ac0236ccceae78d259a66b4912d25f` | image / archer_recurve_scout.png |
| ALI-0754 | `rusted_kingdoms/assets/sprites/npc/archer_recurve_scout.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/archer_recurve_scout.tsx` | `b3b4e7d47cb684578db7bf8a3362dfe444bb3d1d5ef0d1756e650e819ac92b26` | tileset XML / archer_recurve_scout.tsx |
| ALI-0755 | `rusted_kingdoms/assets/sprites/npc/farmer_male_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/farmer_male_base.png` | `5132fc861496b432763fc04f02cc6012ce0c4a3d6147fec30757d5d6e7e8192c` | image / farmer_male_base.png |
| ALI-0756 | `rusted_kingdoms/assets/sprites/npc/farmer_male_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/farmer_male_base.tsx` | `9191d9905e5aa224667389904310e36d173d91e74e843d3884c961892d117869` | tileset XML / farmer_male_base.tsx |
| ALI-0757 | `rusted_kingdoms/assets/sprites/npc/farmer_male_older_farmer.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/farmer_male_older_farmer.png` | `126228bbb868bec7c0140bb7580bbab6ac193bbbc52d5d2cc266e970c1bf8261` | image / farmer_male_older_farmer.png |
| ALI-0758 | `rusted_kingdoms/assets/sprites/npc/farmer_male_older_farmer.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/farmer_male_older_farmer.tsx` | `b756c007478db5bdf2c5f69af5f655ee74fb3a53b916e13bd12ea159ae387f05` | tileset XML / farmer_male_older_farmer.tsx |
| ALI-0759 | `rusted_kingdoms/assets/sprites/npc/farmer_male_straw_hat_worker.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/farmer_male_straw_hat_worker.png` | `0b212239fe5be8f94134af7191e8bd8f0752d9ccd1fc7e06e6ab45891deed513` | image / farmer_male_straw_hat_worker.png |
| ALI-0760 | `rusted_kingdoms/assets/sprites/npc/farmer_male_straw_hat_worker.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/farmer_male_straw_hat_worker.tsx` | `b79d9fba688c6076ed2fd9617860ae6bd53561c965ecf33604fac5816e5c7e41` | tileset XML / farmer_male_straw_hat_worker.tsx |
| ALI-0761 | `rusted_kingdoms/assets/sprites/npc/female_blue_01.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/female_blue_01.png` | `5248994feecb9122ab2b9429377b360a3b4bdeb1c766bca330daff40c00840bb` | image / female_blue_01.png |
| ALI-0762 | `rusted_kingdoms/assets/sprites/npc/female_blue_01.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/female_blue_01.tsx` | `2884d3ca0c09f0cd6fc2de2eaae9ea21910f3649ccc2e25a933256700fb14f82` | tileset XML / female_blue_01.tsx |
| ALI-0763 | `rusted_kingdoms/assets/sprites/npc/female_sword_fighter_armored_katana.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/female_sword_fighter_armored_katana.png` | `6e56457870753e1c3697e309b76204a957d6696c019eae9787cc2c73653eb0fb` | image / female_sword_fighter_armored_katana.png |
| ALI-0764 | `rusted_kingdoms/assets/sprites/npc/female_sword_fighter_armored_katana.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/female_sword_fighter_armored_katana.tsx` | `c9a1b047c01fde5592c0b5f93bebb89d4cccbe6c6fad9c637ce58dc0c353f09d` | tileset XML / female_sword_fighter_armored_katana.tsx |
| ALI-0765 | `rusted_kingdoms/assets/sprites/npc/female_sword_fighter_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/female_sword_fighter_base.png` | `91ab0596ed4448b09b734f03124c1aca43bdb89bf20d7f388b4d1aa505aeffee` | image / female_sword_fighter_base.png |
| ALI-0766 | `rusted_kingdoms/assets/sprites/npc/female_sword_fighter_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/female_sword_fighter_base.tsx` | `f3001c2db8836a2b2ce49d6ca258dee40ded56f93cfced2aaeb04fb00618d22c` | tileset XML / female_sword_fighter_base.tsx |
| ALI-0767 | `rusted_kingdoms/assets/sprites/npc/female_sword_fighter_rapier_duelist.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/female_sword_fighter_rapier_duelist.png` | `5a9413458df648589bb10c0dff9992895ea86322e7898b35a4fcdc9e0cb5aeaa` | image / female_sword_fighter_rapier_duelist.png |
| ALI-0768 | `rusted_kingdoms/assets/sprites/npc/female_sword_fighter_rapier_duelist.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/female_sword_fighter_rapier_duelist.tsx` | `31bb35a6739a34001d4e38984f7ddaa17e5bcc4aba2a4e74d597d37d157cb676` | tileset XML / female_sword_fighter_rapier_duelist.tsx |
| ALI-0769 | `rusted_kingdoms/assets/sprites/npc/female_wiz_01.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/female_wiz_01.png` | `1913de902c70228e71c95a8b4b4968956f743257fe8d988192075dcd2742be46` | image / female_wiz_01.png |
| ALI-0770 | `rusted_kingdoms/assets/sprites/npc/female_wiz_01.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/female_wiz_01.tsx` | `b5c096bec5dc4c6ed54708ed76fcb4933c946baae0104243bd185942cceb12ee` | tileset XML / female_wiz_01.tsx |
| ALI-0771 | `rusted_kingdoms/assets/sprites/npc/male_blond_01.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/male_blond_01.png` | `6290f754b1b1ef56d447c148e491be1544e54f25d54743176c18cd73d47a60b9` | image / male_blond_01.png |
| ALI-0772 | `rusted_kingdoms/assets/sprites/npc/male_blond_01.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/male_blond_01.tsx` | `c242525975ba62a4e2add9cfb9b408760bd7ffd0cbbf01216afff1dbc8c74627` | tileset XML / male_blond_01.tsx |
| ALI-0773 | `rusted_kingdoms/assets/sprites/npc/male_sword_fighter_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/male_sword_fighter_base.png` | `9c510c91bac3ebf7c33b2fafa4d0e836ed2dd0178fa141f602ea0bed30efd578` | image / male_sword_fighter_base.png |
| ALI-0774 | `rusted_kingdoms/assets/sprites/npc/male_sword_fighter_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/male_sword_fighter_base.tsx` | `e7702df6b3cae48f15b22cfbee0167d57b12f16ae38cc366f3bd0d7be53e99f8` | tileset XML / male_sword_fighter_base.tsx |
| ALI-0775 | `rusted_kingdoms/assets/sprites/npc/male_sword_fighter_longsword_guard.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/male_sword_fighter_longsword_guard.png` | `e12c5591a2f0a53c48d384c51c82eee8edc2051b01e211b9a6a50a830ebf584a` | image / male_sword_fighter_longsword_guard.png |
| ALI-0776 | `rusted_kingdoms/assets/sprites/npc/male_sword_fighter_longsword_guard.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/male_sword_fighter_longsword_guard.tsx` | `7b1c2eb3f6d5f30b820da08a35e14c1f6c396b0cc622bf5c65f0a455371e1f76` | tileset XML / male_sword_fighter_longsword_guard.tsx |
| ALI-0777 | `rusted_kingdoms/assets/sprites/npc/man_01.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/man_01.png` | `999e83cb1852e5531850b610f0ed685d1e7a63bc23426c68974da307617046bd` | image / man_01.png |
| ALI-0778 | `rusted_kingdoms/assets/sprites/npc/man_01.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/man_01.tsx` | `7b4b0b206d230b232c9d3470300668c2acebf7bf980a61cd26fc5ec833f937b7` | tileset XML / man_01.tsx |
| ALI-0779 | `rusted_kingdoms/assets/sprites/npc/merchant_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/merchant_base.png` | `d925f0580a9ac76c2d66c77129817ba8a71150bb19b7b83bac6905c96d3b4b04` | image / merchant_base.png |
| ALI-0780 | `rusted_kingdoms/assets/sprites/npc/merchant_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/merchant_base.tsx` | `261d38d4940688388a0b65f3dde6a507e9de9bf2a8a180a0e854c4ff6c8eb271` | tileset XML / merchant_base.tsx |
| ALI-0781 | `rusted_kingdoms/assets/sprites/npc/merchant_traveling_merchant.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/merchant_traveling_merchant.png` | `bd5b07cff3365d9e206eeb64b0584740c0cc6379f9d2e2df50b6f77c2675f6b8` | image / merchant_traveling_merchant.png |
| ALI-0782 | `rusted_kingdoms/assets/sprites/npc/merchant_traveling_merchant.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/merchant_traveling_merchant.tsx` | `f581d98ebf3c7e70c42321c305239b94539a968c42a5fb0e4f135fc7e852a381` | tileset XML / merchant_traveling_merchant.tsx |
| ALI-0783 | `rusted_kingdoms/assets/sprites/npc/merchant_wealthy_bowler.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/merchant_wealthy_bowler.png` | `79ffc3d034951e850a66e1ce485f33cfe3eab00444e0970ea738426b7fa66ce3` | image / merchant_wealthy_bowler.png |
| ALI-0784 | `rusted_kingdoms/assets/sprites/npc/merchant_wealthy_bowler.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/merchant_wealthy_bowler.tsx` | `8122561617d6918f49ea3b2e756c46f8996a2c359639d398d57303e5313d2409` | tileset XML / merchant_wealthy_bowler.tsx |
| ALI-0785 | `rusted_kingdoms/assets/sprites/npc/old_man_01.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/old_man_01.png` | `d93a1cf742e824eb44e86123b1330b4ba5c5b917d9acea959f39960ec38755b9` | image / old_man_01.png |
| ALI-0786 | `rusted_kingdoms/assets/sprites/npc/old_man_01.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/old_man_01.tsx` | `e5a123ca281afea8c6c9924fc1966644d810c1900a4b5ed6d4b2c4b7bfd737eb` | tileset XML / old_man_01.tsx |
| ALI-0787 | `rusted_kingdoms/assets/sprites/npc/plate_knight_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/plate_knight_base.png` | `d48b441a69faa174a54488036d6a030005396e74302f621d01cce757d0fbae8c` | image / plate_knight_base.png |
| ALI-0788 | `rusted_kingdoms/assets/sprites/npc/plate_knight_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/plate_knight_base.tsx` | `9a75a537cd19aa27c73a893165155f8a7c588a2a09b8d03bf4a0c76d52d752dc` | tileset XML / plate_knight_base.tsx |
| ALI-0789 | `rusted_kingdoms/assets/sprites/npc/plate_knight_mace_knight.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/plate_knight_mace_knight.png` | `3e276b7b95a1c7139c25a001ea1cd5d746f5ef5a59e985788a595584b03d20f3` | image / plate_knight_mace_knight.png |
| ALI-0790 | `rusted_kingdoms/assets/sprites/npc/plate_knight_mace_knight.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/plate_knight_mace_knight.tsx` | `a2ae062b6228a873317193e5da74160e0afb78ef19320e46d43fa2ef45ca4e45` | tileset XML / plate_knight_mace_knight.tsx |
| ALI-0791 | `rusted_kingdoms/assets/sprites/npc/plate_knight_open_helmet.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/plate_knight_open_helmet.png` | `51db92efa4dffc7ddc9b8f4e7b0d78f0e7f671d89da11bd9fe0439b10de448c9` | image / plate_knight_open_helmet.png |
| ALI-0792 | `rusted_kingdoms/assets/sprites/npc/plate_knight_open_helmet.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/plate_knight_open_helmet.tsx` | `a372e436739349ec0437d394b2f3b7cec00f63464d202cb61c4f9e4766e14659` | tileset XML / plate_knight_open_helmet.tsx |
| ALI-0793 | `rusted_kingdoms/assets/sprites/npc/rogue_bandit_saber.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/rogue_bandit_saber.png` | `871a367978f332ff367661cefcea9f769b1b65355ed34c5e65cc21057821d8b8` | image / rogue_bandit_saber.png |
| ALI-0794 | `rusted_kingdoms/assets/sprites/npc/rogue_bandit_saber.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/rogue_bandit_saber.tsx` | `f3d7fcfbd6de886cca65eb78b1e5e927bc5ee6664e7b7fbf3369bffc6deb4b3f` | tileset XML / rogue_bandit_saber.tsx |
| ALI-0795 | `rusted_kingdoms/assets/sprites/npc/rogue_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/rogue_base.png` | `43ce3e182364fbae289b734784f4c2cf156d08ea11e53792ce75a488c4822cb6` | image / rogue_base.png |
| ALI-0796 | `rusted_kingdoms/assets/sprites/npc/rogue_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/rogue_base.tsx` | `673b124ce953e580b2f6257bad9eac83f390c17a2cc43b8c5a6cf13888ccc953` | tileset XML / rogue_base.tsx |
| ALI-0797 | `rusted_kingdoms/assets/sprites/npc/rogue_hooded_rogue.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/rogue_hooded_rogue.png` | `da94888b43a2b0d35214b347ab49456235347a38dffc95fea318441341f3bcab` | image / rogue_hooded_rogue.png |
| ALI-0798 | `rusted_kingdoms/assets/sprites/npc/rogue_hooded_rogue.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/rogue_hooded_rogue.tsx` | `fae1b2b03b03c20dc4ad9dd1d5c53b653b480b4c16d05033d9266a885d1987ec` | tileset XML / rogue_hooded_rogue.tsx |
| ALI-0799 | `rusted_kingdoms/assets/sprites/npc/village_female_person_apron_keeper.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/village_female_person_apron_keeper.png` | `0b63c45b2836ade26b00efd7e62b86334a10f4d72d269f9a629e59b5ee5c04a0` | image / village_female_person_apron_keeper.png |
| ALI-0800 | `rusted_kingdoms/assets/sprites/npc/village_female_person_apron_keeper.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/village_female_person_apron_keeper.tsx` | `48b4a0e6b10e785e16f7a92e52e43146aca4c6c2dc64c4aac32de1a68ca7f2de` | tileset XML / village_female_person_apron_keeper.tsx |
| ALI-0801 | `rusted_kingdoms/assets/sprites/npc/village_female_person_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/village_female_person_base.png` | `12e0c635966118c6850b1774c6b94a329b7093ce889c2911091dc48ba61132dc` | image / village_female_person_base.png |
| ALI-0802 | `rusted_kingdoms/assets/sprites/npc/village_female_person_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/village_female_person_base.tsx` | `b98b8141ae8f2402a7ddcebbcc95d712f84c7020b707ebf1ece341c2bb2d9b5d` | tileset XML / village_female_person_base.tsx |
| ALI-0803 | `rusted_kingdoms/assets/sprites/npc/village_female_person_elder_matron.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/village_female_person_elder_matron.png` | `645f17a8a94a6be06bffc382b01717c61987612c6b5547706eddc566158b870c` | image / village_female_person_elder_matron.png |
| ALI-0804 | `rusted_kingdoms/assets/sprites/npc/village_female_person_elder_matron.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/village_female_person_elder_matron.tsx` | `c836f67ace46396017cb5af5285f1d92f82b6a002be20ea96f97081e5cd262d1` | tileset XML / village_female_person_elder_matron.tsx |
| ALI-0805 | `rusted_kingdoms/assets/sprites/npc/village_female_person_headscarf_neighbor.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/village_female_person_headscarf_neighbor.png` | `c2fa5807e3069f2fb915b34828946c90409fd9e5768f0fdc05f5e7734948481a` | image / village_female_person_headscarf_neighbor.png |
| ALI-0806 | `rusted_kingdoms/assets/sprites/npc/village_female_person_headscarf_neighbor.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/village_female_person_headscarf_neighbor.tsx` | `2f7464a0df31a7e9355856cac51e7ec859048d533b5f6fd0a75aa34d3a91a70d` | tileset XML / village_female_person_headscarf_neighbor.tsx |
| ALI-0807 | `rusted_kingdoms/assets/sprites/npc/village_female_person_long_hair_green_skirt.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/village_female_person_long_hair_green_skirt.png` | `65d6921a8aeb85110f76a97fa220223e680e3fa885a0deb1231ceb3d02db9a03` | image / village_female_person_long_hair_green_skirt.png |
| ALI-0808 | `rusted_kingdoms/assets/sprites/npc/village_female_person_long_hair_green_skirt.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/village_female_person_long_hair_green_skirt.tsx` | `3f99bc48b10c08de47fd76ac557c784cebe31d626b0ec9eb1536d5b8cc0fd1c2` | tileset XML / village_female_person_long_hair_green_skirt.tsx |
| ALI-0809 | `rusted_kingdoms/assets/sprites/npc/village_female_person_practical_pants.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/village_female_person_practical_pants.png` | `6029be77c6b5e898a3760783e47bb2f5241217f8fb5bdbe8eb9c3560561b8a54` | image / village_female_person_practical_pants.png |
| ALI-0810 | `rusted_kingdoms/assets/sprites/npc/village_female_person_practical_pants.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/village_female_person_practical_pants.tsx` | `b74f0fe70554127521bd57189588d7abbe7eed3d503f43270b276e8b6b27a290` | tileset XML / village_female_person_practical_pants.tsx |
| ALI-0811 | `rusted_kingdoms/assets/sprites/npc/village_male_person_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/village_male_person_base.png` | `e2bcdf9e617e54ba0327dbc352db6ffe4f5ba1ac3d64c3819a04518979cefa67` | image / village_male_person_base.png |
| ALI-0812 | `rusted_kingdoms/assets/sprites/npc/village_male_person_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/village_male_person_base.tsx` | `c7a051b75b68a2f3cdda6fdcae8b88216b4bbf3c0fe99ba145340f4a8cc46f12` | tileset XML / village_male_person_base.tsx |
| ALI-0813 | `rusted_kingdoms/assets/sprites/npc/village_male_person_festival_local.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/village_male_person_festival_local.png` | `d404ce2b37fa9e213e669c84f608f08eb1d9b7a49ad6514850e069851f62bc27` | image / village_male_person_festival_local.png |
| ALI-0814 | `rusted_kingdoms/assets/sprites/npc/village_male_person_festival_local.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/village_male_person_festival_local.tsx` | `12976c4e457cd06e3168a50aefcfb63ad11142b044c1e67170226ce831df278f` | tileset XML / village_male_person_festival_local.tsx |
| ALI-0815 | `rusted_kingdoms/assets/sprites/npc/village_male_person_field_hand.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/village_male_person_field_hand.png` | `e2310a36014d1c3a28322d4393db50af250210eee55d701d9a737fffd44bedfd` | image / village_male_person_field_hand.png |
| ALI-0816 | `rusted_kingdoms/assets/sprites/npc/village_male_person_field_hand.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/village_male_person_field_hand.tsx` | `7ab957141dc4089b66a53064faa62ddf538d3ec88e85794c27f992362ae4ffdc` | tileset XML / village_male_person_field_hand.tsx |
| ALI-0817 | `rusted_kingdoms/assets/sprites/npc/village_male_person_market_runner.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/village_male_person_market_runner.png` | `1bbc52ccd650c3fea0aba07cc2fb8fb658a8bcb4bd9b8b98a36fe081d83ef2d5` | image / village_male_person_market_runner.png |
| ALI-0818 | `rusted_kingdoms/assets/sprites/npc/village_male_person_market_runner.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/village_male_person_market_runner.tsx` | `c4a89cb30da0c4f0b965fcdd1f9ffd31387fc064f3f294baaf2a0ec1ff801962` | tileset XML / village_male_person_market_runner.tsx |
| ALI-0819 | `rusted_kingdoms/assets/sprites/npc/village_male_person_older_villager.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/village_male_person_older_villager.png` | `b3e37ead9658d13202e151e8606755300ff16496499cd8ab8dae6671c1ca59ca` | image / village_male_person_older_villager.png |
| ALI-0820 | `rusted_kingdoms/assets/sprites/npc/village_male_person_older_villager.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/village_male_person_older_villager.tsx` | `3f5a734f304c2b2ba21e3ec733db805bfebc6cbf5566b5d8a50f5a700ccc4714` | tileset XML / village_male_person_older_villager.tsx |
| ALI-0821 | `rusted_kingdoms/assets/sprites/npc/village_male_person_plain_dark_hair.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/village_male_person_plain_dark_hair.png` | `ec4fcf98968799acf50493eb087300cfaf8610f99bafa53ddb4caef26cb331ed` | image / village_male_person_plain_dark_hair.png |
| ALI-0822 | `rusted_kingdoms/assets/sprites/npc/village_male_person_plain_dark_hair.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/village_male_person_plain_dark_hair.tsx` | `7e6f024f7f56091d55495cf1a713e1fdc4a9914a756b462b1c363567c380c515` | tileset XML / village_male_person_plain_dark_hair.tsx |
| ALI-0823 | `rusted_kingdoms/assets/sprites/npc/village_male_person_quiet_neighbor.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/village_male_person_quiet_neighbor.png` | `1c0c3327e9e416d577b337599215baefaf21dcea60fc44a48d225d7414bfcd6a` | image / village_male_person_quiet_neighbor.png |
| ALI-0824 | `rusted_kingdoms/assets/sprites/npc/village_male_person_quiet_neighbor.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/village_male_person_quiet_neighbor.tsx` | `4cc8066711840c1e3459629c56ce59ad24b948c436dad6a1761f3a49f923060b` | tileset XML / village_male_person_quiet_neighbor.tsx |
| ALI-0825 | `rusted_kingdoms/assets/sprites/npc/wizard_base.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/wizard_base.png` | `6c944db07bac016e59ba3d97887054e1bf46c09e4fcd6eddc5ba72a3c7fc4f39` | image / wizard_base.png |
| ALI-0826 | `rusted_kingdoms/assets/sprites/npc/wizard_base.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/wizard_base.tsx` | `2a75f0552dcf0e6c884c04536243da88f762af1ace4746ed5bf13b4f612b49bf` | tileset XML / wizard_base.tsx |
| ALI-0827 | `rusted_kingdoms/assets/sprites/npc/wizard_blue_robed_mage.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/wizard_blue_robed_mage.png` | `fdc4586438a8b1d83140f56fd6a82c57be30d848c062689b1eb7938fa6cc1f41` | image / wizard_blue_robed_mage.png |
| ALI-0828 | `rusted_kingdoms/assets/sprites/npc/wizard_blue_robed_mage.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/wizard_blue_robed_mage.tsx` | `ba362d7d889bba57f344f55882e3fffa3a12a9fbd1a134b6cffc3021dcbfb9c1` | tileset XML / wizard_blue_robed_mage.tsx |
| ALI-0829 | `rusted_kingdoms/assets/sprites/npc/wizard_wand_caster.png` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/wizard_wand_caster.png` | `1e6468307c01bccb5e672c1ab661468d08e191b0d5e0ed4b6f297afdfcc037db` | image / wizard_wand_caster.png |
| ALI-0830 | `rusted_kingdoms/assets/sprites/npc/wizard_wand_caster.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/npc/wizard_wand_caster.tsx` | `620e6387fe8739834a44321364a47d51ca07a22def1c52d72f48a26e883cfdbe` | tileset XML / wizard_wand_caster.tsx |
| ALI-0831 | `rusted_kingdoms/assets/sprites/party/03_reiya_walk.png` | `assets/scenarios/rusted_kingdoms/media/sprites/party/03_reiya_walk.png` | `c97c531443f6c3ca93574a1ae9e8f70c090cbbfda21a1715030d0eb78b11ea40` | image / 03_reiya_walk.png |
| ALI-0832 | `rusted_kingdoms/assets/sprites/party/03_reiya_walk.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/party/03_reiya_walk.tsx` | `54f41742a54af7984b17a146bc8b031aac2c0b2d0a20d8ba0a0222c135535263` | tileset XML / 03_reiya_walk.tsx |
| ALI-0833 | `rusted_kingdoms/assets/sprites/party/04_jep_walk.png` | `assets/scenarios/rusted_kingdoms/media/sprites/party/04_jep_walk.png` | `3ebaeb33124208a537fb19518243e68c2b2d49edd3b215baabc9ade462836da9` | image / 04_jep_walk.png |
| ALI-0834 | `rusted_kingdoms/assets/sprites/party/04_jep_walk.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/party/04_jep_walk.tsx` | `bc6ba5170a7191e858c3052eab7ea43a6bcbdabd2cb902971202a18f610e97c9` | tileset XML / 04_jep_walk.tsx |
| ALI-0835 | `rusted_kingdoms/assets/sprites/party/05_kael_walk.png` | `assets/scenarios/rusted_kingdoms/media/sprites/party/05_kael_walk.png` | `45f8eea41154f1c4d0ccd99279f1339591e55bf88685e9557d40071cb8ce5f06` | image / 05_kael_walk.png |
| ALI-0836 | `rusted_kingdoms/assets/sprites/party/05_kael_walk.tsx` | `assets/scenarios/rusted_kingdoms/media/sprites/party/05_kael_walk.tsx` | `ffe2f7c526c72198bc883e440d79957008d15740a40be3062779024b12c3dc8f` | tileset XML / 05_kael_walk.tsx |
| ALI-0837 | `rusted_kingdoms/assets/tilesets/ground/terrain-v7.md` | `assets/scenarios/rusted_kingdoms/media/tilesets/ground/terrain-v7.md` | `fd2c0d5a9c254af8bd6ba8214a1d5d027919e7e517778fc0eaaf69f635e280fe` | markdown / terrain-v7.md |
| ALI-0838 | `rusted_kingdoms/assets/tilesets/sample_01.png` | `assets/scenarios/rusted_kingdoms/media/tilesets/sample_01.png` | `1d72dcc86be5249ff778b6f8dee11630963e980429d908f55887d64049524d80` | image / sample_01.png |
| ALI-0839 | `rusted_kingdoms/assets/tilesets/sample_01.tsx` | `assets/scenarios/rusted_kingdoms/media/tilesets/sample_01.tsx` | `a8a2ca383dccd1e6c796651decdf928a670d9c2ce504e5227bc779c8b16e8093` | tileset XML / sample_01.tsx |
| ALI-0840 | `rusted_kingdoms/assets/tilesets/schwarnhild/tiles-all-32x32.png` | `assets/scenarios/rusted_kingdoms/media/tilesets/schwarnhild/tiles-all-32x32.png` | `39a38cb4281083563c77538cd6e57785bdba3fe00cad6376bee332b7bd9ccd88` | image / tiles-all-32x32.png |
| ALI-0841 | `rusted_kingdoms/assets/tilesets/schwarnhild/tiles-all-32x32.tsx` | `assets/scenarios/rusted_kingdoms/media/tilesets/schwarnhild/tiles-all-32x32.tsx` | `8eb8412852d05090e31e4d01de2a0bd5a800fb1044f6e505ae4d77d2260e8b7c` | tileset XML / tiles-all-32x32.tsx |
| ALI-0842 | `rusted_kingdoms/assets/tilesets/stamps/17.stamp` | `assets/scenarios/rusted_kingdoms/media/tilesets/stamps/17.stamp` | `37f916f3217cf2cad6eb979f0f6cee4bef21f69da82144c24209047e92fdd51c` | tiled stamp / 17.stamp |
| ALI-0843 | `rusted_kingdoms/assets/tilesets/stamps/ardel_house.stamp` | `assets/scenarios/rusted_kingdoms/media/tilesets/stamps/ardel_house.stamp` | `3134e01789c568e9e1cff5807ba228da7c53a1f11eb0aaa27e575cc9c6070db1` | tiled stamp / ardel_house.stamp |
| ALI-0844 | `rusted_kingdoms/assets/tilesets/stamps/bridge_long.stamp` | `assets/scenarios/rusted_kingdoms/media/tilesets/stamps/bridge_long.stamp` | `1596c30f43824312bfe2053ee40c71e717726fc8664aaadae5cc0022fb04dcf8` | tiled stamp / bridge_long.stamp |
| ALI-0845 | `rusted_kingdoms/assets/tilesets/stamps/bridge_short.stamp` | `assets/scenarios/rusted_kingdoms/media/tilesets/stamps/bridge_short.stamp` | `b39df7df4af5fda2716e52b4e96b75f6efde504b5089496fffdb8d570fd0ab07` | tiled stamp / bridge_short.stamp |
| ALI-0846 | `rusted_kingdoms/assets/tilesets/stamps/building_blacksmith_01.stamp` | `assets/scenarios/rusted_kingdoms/media/tilesets/stamps/building_blacksmith_01.stamp` | `2653c86c0143d6963f6524719565fc5125522a678cf37feb3379a1467a05e43a` | tiled stamp / building_blacksmith_01.stamp |
| ALI-0847 | `rusted_kingdoms/assets/tilesets/stamps/building_house_01.stamp` | `assets/scenarios/rusted_kingdoms/media/tilesets/stamps/building_house_01.stamp` | `0e01ee8ea76a7a42e1ca03eaef0c713cf9fbf65260da2cfef8305940d6d9039b` | tiled stamp / building_house_01.stamp |
| ALI-0848 | `rusted_kingdoms/assets/tilesets/stamps/building_house_02.stamp` | `assets/scenarios/rusted_kingdoms/media/tilesets/stamps/building_house_02.stamp` | `9300e13bb7433f61f67d3467b2ff5d5b959914adb083ca853365b2cca7931929` | tiled stamp / building_house_02.stamp |
| ALI-0849 | `rusted_kingdoms/assets/tilesets/stamps/building_house_03.stamp` | `assets/scenarios/rusted_kingdoms/media/tilesets/stamps/building_house_03.stamp` | `6d35bf8265242afab8ab536c1655bc7d3bafe58a1b67d8ea35a5a5f26a071321` | tiled stamp / building_house_03.stamp |
| ALI-0850 | `rusted_kingdoms/assets/tilesets/stamps/building_house_04.stamp` | `assets/scenarios/rusted_kingdoms/media/tilesets/stamps/building_house_04.stamp` | `ac9136b67f1632581e9fb4fc33478cf80c3eaea7e0fdcca760215ef22a25a4e5` | tiled stamp / building_house_04.stamp |
| ALI-0851 | `rusted_kingdoms/assets/tilesets/stamps/building_house_05.stamp` | `assets/scenarios/rusted_kingdoms/media/tilesets/stamps/building_house_05.stamp` | `156675ed6ee2c444f6e5beaa9482ea48a213fe559dec2ed16978fad656877242` | tiled stamp / building_house_05.stamp |
| ALI-0852 | `rusted_kingdoms/assets/tilesets/stamps/building_inn_01.stamp` | `assets/scenarios/rusted_kingdoms/media/tilesets/stamps/building_inn_01.stamp` | `b2ca64a1e0e3455f6941be71db30f7214fd1f7486f09bf996076d43831dc64ee` | tiled stamp / building_inn_01.stamp |
| ALI-0853 | `rusted_kingdoms/assets/tilesets/stamps/cave_entrance.stamp` | `assets/scenarios/rusted_kingdoms/media/tilesets/stamps/cave_entrance.stamp` | `e7fbff0384661afd8571240ab76179c199ca3ded48d941d8105dcdd4912f430c` | tiled stamp / cave_entrance.stamp |
| ALI-0854 | `rusted_kingdoms/assets/tilesets/stamps/door_wood_1.stamp` | `assets/scenarios/rusted_kingdoms/media/tilesets/stamps/door_wood_1.stamp` | `007f86fe1aca0447624f37fa43c32c8202aa7730590fdc30984022237fdf6a0d` | tiled stamp / door_wood_1.stamp |
| ALI-0855 | `rusted_kingdoms/assets/tilesets/stamps/door_wood_2.stamp` | `assets/scenarios/rusted_kingdoms/media/tilesets/stamps/door_wood_2.stamp` | `dc3317018adbcb8855a7b63a4bed3be6dfd1bf4406711fc750cc41741822ba0a` | tiled stamp / door_wood_2.stamp |
| ALI-0856 | `rusted_kingdoms/assets/tilesets/stamps/forest.stamp` | `assets/scenarios/rusted_kingdoms/media/tilesets/stamps/forest.stamp` | `10f2a4932a18d9ef0f0332769717d29254120d287348052038189d8c66677bef` | tiled stamp / forest.stamp |
| ALI-0857 | `rusted_kingdoms/assets/tilesets/stamps/log_horizontal.stamp` | `assets/scenarios/rusted_kingdoms/media/tilesets/stamps/log_horizontal.stamp` | `29a468a673521c48af3ccef00392219bef207d427f55d21a02650ac0086cc7b6` | tiled stamp / log_horizontal.stamp |
| ALI-0858 | `rusted_kingdoms/assets/tilesets/stamps/log_vertical.stamp` | `assets/scenarios/rusted_kingdoms/media/tilesets/stamps/log_vertical.stamp` | `2d4fb1e370d6452be888f6cfece21325f3563ff6bda2fb0f4af38adcc47d3d70` | tiled stamp / log_vertical.stamp |
| ALI-0859 | `rusted_kingdoms/assets/tilesets/stamps/object_broken_stone_column_01.stamp` | `assets/scenarios/rusted_kingdoms/media/tilesets/stamps/object_broken_stone_column_01.stamp` | `120d7f19fe73d466914b5a4c6f41960cc0c5f3a952271d9586cc16e408b8dc71` | tiled stamp / object_broken_stone_column_01.stamp |
| ALI-0860 | `rusted_kingdoms/assets/tilesets/stamps/object_stone_column_02.stamp` | `assets/scenarios/rusted_kingdoms/media/tilesets/stamps/object_stone_column_02.stamp` | `85892bf82956f5e4e791498d2cea7a496475ad515799452cd4179ad91c0e0d6d` | tiled stamp / object_stone_column_02.stamp |
| ALI-0861 | `rusted_kingdoms/assets/tilesets/stamps/scare_crow.stamp` | `assets/scenarios/rusted_kingdoms/media/tilesets/stamps/scare_crow.stamp` | `47aabf39812502bfcd1604381b15f6e3ea3a329ab1b91592756c8dda583ebfaa` | tiled stamp / scare_crow.stamp |
| ALI-0862 | `rusted_kingdoms/assets/tilesets/stamps/tree.stamp` | `assets/scenarios/rusted_kingdoms/media/tilesets/stamps/tree.stamp` | `157103e7f32b34c08903f6e2419de658d313510394478e2c278c7e0fac6ac930` | tiled stamp / tree.stamp |
| ALI-0863 | `rusted_kingdoms/assets/tilesets/stamps/tree_dark.stamp` | `assets/scenarios/rusted_kingdoms/media/tilesets/stamps/tree_dark.stamp` | `484281890c175ae8d283cc69e593a72c5cfa102f0421ae8d70f774a351cc9050` | tiled stamp / tree_dark.stamp |
| ALI-0864 | `rusted_kingdoms/assets/tilesets/stamps/tree_dead.stamp` | `assets/scenarios/rusted_kingdoms/media/tilesets/stamps/tree_dead.stamp` | `928cd6268a3ca443bbbbc18873b412309f972771d6260affedb61f6a72da136d` | tiled stamp / tree_dead.stamp |
| ALI-0865 | `rusted_kingdoms/assets/tilesets/stamps/tree_double.stamp` | `assets/scenarios/rusted_kingdoms/media/tilesets/stamps/tree_double.stamp` | `ff46ba175d28f57e7b1fa540f0fd05e55dc529aee9454f4aa3bd90b30500d02d` | tiled stamp / tree_double.stamp |
| ALI-0866 | `rusted_kingdoms/assets/tilesets/stamps/tree_orange.stamp` | `assets/scenarios/rusted_kingdoms/media/tilesets/stamps/tree_orange.stamp` | `bbdd4c55005c90f17f9514c2251c993698eaa3049ec4aa10b73bb661bf307cfe` | tiled stamp / tree_orange.stamp |
| ALI-0867 | `rusted_kingdoms/assets/tilesets/stamps/tree_triple.stamp` | `assets/scenarios/rusted_kingdoms/media/tilesets/stamps/tree_triple.stamp` | `326482e6ba151066d6db176da6f291d7cdd816f5f9050c117f984f8a7048cc2d` | tiled stamp / tree_triple.stamp |
| ALI-0868 | `rusted_kingdoms/assets/tilesets/walls_02.png` | `assets/scenarios/rusted_kingdoms/media/tilesets/walls_02.png` | `3dbcddb3de6a647d4d1a04cd9bab46f41b3ed2e36c2a53ad46f444ad274503a2` | image / walls_02.png |
| ALI-0869 | `rusted_kingdoms/assets/tilesets/walls_02.tsx` | `assets/scenarios/rusted_kingdoms/media/tilesets/walls_02.tsx` | `b68b5121e161b22cf8f2262014ee84faacebee56fb23aca6e3799103dbe9e34f` | tileset XML / walls_02.tsx |
| ALI-0870 | `rusted_kingdoms/assets/tilesets/window_8x6.png` | `assets/scenarios/rusted_kingdoms/media/tilesets/window_8x6.png` | `ca5435323827a31f927978b4f31b4ffc918f42db356f27749006b32cba10a0ee` | image / window_8x6.png |
| ALI-0871 | `rusted_kingdoms/assets/tilesets/window_8x6.tsx` | `assets/scenarios/rusted_kingdoms/media/tilesets/window_8x6.tsx` | `2f54addf4aeabfac61f110d33cd01bb9a60dae4dd687fefc6d27b767b36f645e` | tileset XML / window_8x6.tsx |

## M14.05 backfill: port-modified migrated payload files

Payload files derived from a counterpart in the pinned source tree at
`08970359d6cb03586948625d29b0d3351dbbf785` and then changed by this port. The SHA-256 column is the current
destination hash, which deliberately differs from the source hash. Each row is
a distinct ledger entry. Shared review fields for every row:

- creator/rightsholder: unknown for the underlying original; the port's own
  changes do not transfer ownership of what they modify;
- source/evidence: the exact pinned source file at `08970359d6cb03586948625d29b0d3351dbbf785`, compared by
  SHA-256 on 2026-09-12, which established that the destination differs;
- license, notice, and required attribution: unknown;
- modification: **modified by this port after copying**; the destination hash
  below is authoritative for what would ship;
- redistribution, commercial use, and derivatives: unknown;
- review: `needs-evidence`, automated backfill audit, 2026-09-12;
- related work: M14.05; and
- blocker: **Release blocker.** A derivative of an asset whose own rights are
  unknown cannot be cleared by the derivation. Establish the original's
  ownership, license, and derivative-work permission before public release, or
  replace the asset.

Rows carrying an ID below `ALI-0130` were already in this ledger and recorded a
stale hash; they are restated here with their original ID, their current
destination hash, and the modification fact their previous section denied.

| ID | Source path in the pinned source tree | Destination path | SHA-256 | Kind/name |
| --- | --- | --- | --- | --- |
| ALI-0085 | `rusted_kingdoms/data/classes/hero.yaml` | `assets/scenarios/rusted_kingdoms/data/classes/hero.yaml` | `bbb45da4d8efd0dcd5f608fafaff52b501e6cbc19d4927074440641971cfb7ae` | scenario YAML / hero.yaml |
| ALI-0089 | `rusted_kingdoms/data/items/accessories.yaml` | `assets/scenarios/rusted_kingdoms/data/items/accessories.yaml` | `e58a37d3baa1e068bcb57655281f5696efa00a63531452233fb0403707283525` | scenario YAML / accessories.yaml |
| ALI-0090 | `rusted_kingdoms/data/items/body.yaml` | `assets/scenarios/rusted_kingdoms/data/items/body.yaml` | `46992bd5a810771265312c032f3b18e5242e94ab25b9a17ba657f6c56497c166` | scenario YAML / body.yaml |
| ALI-0093 | `rusted_kingdoms/data/items/consumables_recovery.yaml` | `assets/scenarios/rusted_kingdoms/data/items/consumables_recovery.yaml` | `d4599ecf0f21e871812b94af9b6b881a875dac7857c1509fe9497ed3808d5668` | scenario YAML / consumables_recovery.yaml |
| ALI-0095 | `rusted_kingdoms/data/items/field_use.yaml` | `assets/scenarios/rusted_kingdoms/data/items/field_use.yaml` | `d6459e73510bc82d3e99e62f344056a9161a4eb6de3e26b05b0f5b4eaac6d1b7` | scenario YAML / field_use.yaml |
| ALI-0096 | `rusted_kingdoms/data/items/helmets.yaml` | `assets/scenarios/rusted_kingdoms/data/items/helmets.yaml` | `2a186f5fcec573d4f4c6a8f9df1aa4b3b7ab0843b4b87500c315933bf8153607` | scenario YAML / helmets.yaml |
| ALI-0100 | `rusted_kingdoms/data/items/shields.yaml` | `assets/scenarios/rusted_kingdoms/data/items/shields.yaml` | `2c00efe3de852f226abefeddfd52613ae2e0691b9a702264a37bf9a07932d4ec` | scenario YAML / shields.yaml |
| ALI-0101 | `rusted_kingdoms/data/items/weapons.yaml` | `assets/scenarios/rusted_kingdoms/data/items/weapons.yaml` | `e41b8449f84f06400fc60691f10eaa118cb93e0f7de03017c1f4e346ca10f936` | scenario YAML / weapons.yaml |
| ALI-0104 | `rusted_kingdoms/data/enemies/enemies_rank_1_SS.yaml` | `assets/scenarios/rusted_kingdoms/data/enemies/enemies_rank_1_SS.yaml` | `a15871a85f500c852d406b34864afbf70f0d2b990cf73f42985d72fe8ce2d8a7` | scenario YAML / enemies_rank_1_SS.yaml |
| ALI-0105 | `rusted_kingdoms/data/enemies/enemies_rank_2_S.yaml` | `assets/scenarios/rusted_kingdoms/data/enemies/enemies_rank_2_S.yaml` | `e142f72e9eb93071852b6983a7b4889035cb5175de396ab316e914d2bf03529a` | scenario YAML / enemies_rank_2_S.yaml |
| ALI-0106 | `rusted_kingdoms/data/enemies/enemies_rank_3_A.yaml` | `assets/scenarios/rusted_kingdoms/data/enemies/enemies_rank_3_A.yaml` | `5a8bcb25dbc2a4de0e292df3b4988fedd4d8b6fb6fafefc90b09fbcae0e52280` | scenario YAML / enemies_rank_3_A.yaml |
| ALI-0107 | `rusted_kingdoms/data/enemies/enemies_rank_4_B.yaml` | `assets/scenarios/rusted_kingdoms/data/enemies/enemies_rank_4_B.yaml` | `71958f4c79623f0c204f9e8aa14c94a4fb599a01fcaf2343e43fed8c10177068` | scenario YAML / enemies_rank_4_B.yaml |
| ALI-0108 | `rusted_kingdoms/data/enemies/enemies_rank_5_C.yaml` | `assets/scenarios/rusted_kingdoms/data/enemies/enemies_rank_5_C.yaml` | `7cdea20500d1de233aa176baf2641056e670947272dcd41460905a42320c9008` | scenario YAML / enemies_rank_5_C.yaml |
| ALI-0109 | `rusted_kingdoms/data/enemies/enemies_rank_6_D.yaml` | `assets/scenarios/rusted_kingdoms/data/enemies/enemies_rank_6_D.yaml` | `996813151f95dc2bf760a547e87cc25649a7000a17937be999081f55f00356f1` | scenario YAML / enemies_rank_6_D.yaml |
| ALI-0110 | `rusted_kingdoms/data/enemies/enemies_rank_7_E.yaml` | `assets/scenarios/rusted_kingdoms/data/enemies/enemies_rank_7_E.yaml` | `db08fd9880ca69551f75a88f5548535d4d8fd1544b9b8e5818c350e84ad3e2d6` | scenario YAML / enemies_rank_7_E.yaml |
| ALI-0872 | `rusted_kingdoms/data/dialogue/ardel_shrine_keeper.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/ardel_shrine_keeper.yaml` | `aad9817d223d0004d8c1f727bfd9c40a07ea3a4e35c6653169bf079e4477dbbf` | scenario YAML / ardel_shrine_keeper.yaml |
| ALI-0873 | `rusted_kingdoms/data/dialogue/ashenveil_oracle_hint.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/ashenveil_oracle_hint.yaml` | `a299fb90962cf3db45aa9b9490272c029ba4eb2cf9be88eb344377e5dda2bc5b` | scenario YAML / ashenveil_oracle_hint.yaml |
| ALI-0874 | `rusted_kingdoms/data/dialogue/frostholm_courtier.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/frostholm_courtier.yaml` | `c37742bd4c14d49e4dfb58307dbca4f3393e76a417ed250538474fd335b14dca` | scenario YAML / frostholm_courtier.yaml |
| ALI-0875 | `rusted_kingdoms/data/dialogue/frostholm_vault_warden.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/frostholm_vault_warden.yaml` | `04cfd0faf8db9f04aafcf65ac57ef600f5a298dc05f8c0a88e1d84efa9dc705b` | scenario YAML / frostholm_vault_warden.yaml |
| ALI-0876 | `rusted_kingdoms/data/dialogue/mc_shop_intro.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/mc_shop_intro.yaml` | `46f2e2df21f96f46dbcbe70613b09a8bf663d96d884f1e4b2ebf18e2460feb09` | scenario YAML / mc_shop_intro.yaml |
| ALI-0877 | `rusted_kingdoms/data/dialogue/port_master_intro.yaml` | `assets/scenarios/rusted_kingdoms/data/dialogue/port_master_intro.yaml` | `f5d175435918b5130f8c9595bb45aa9720d919b90826329d7505e8b20c984e87` | scenario YAML / port_master_intro.yaml |
| ALI-0878 | `rusted_kingdoms/data/encount/zone_05_mountain_foothills_03.yaml` | `assets/scenarios/rusted_kingdoms/data/encount/zone_05_mountain_foothills_03.yaml` | `cd75f53d964651ee39a4ec0c5934ad51214fc62e4f982bd02f7236303f171ae4` | scenario YAML / zone_05_mountain_foothills_03.yaml |
| ALI-0879 | `rusted_kingdoms/data/maps/town_01_ardel_inn_01.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/town_01_ardel_inn_01.yaml` | `0cca6ba227cf92037c0acdaad515ffbd48fd7f05ced6f32368710dabd32caa8d` | scenario YAML / town_01_ardel_inn_01.yaml |
| ALI-0880 | `rusted_kingdoms/data/maps/town_01_ardel_shop_01.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/town_01_ardel_shop_01.yaml` | `9c82caff119f24cb52f0f0681b8c9760a2adda9e9d6c0c52a6c6b0b8a7a87e97` | scenario YAML / town_01_ardel_shop_01.yaml |
| ALI-0881 | `rusted_kingdoms/data/maps/town_01_ardel_shrine.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/town_01_ardel_shrine.yaml` | `8a8b2a6a7929d56294fb376ec68ca8da0fe08ab0e7dd6ca993b53a2b3121304e` | scenario YAML / town_01_ardel_shrine.yaml |
| ALI-0882 | `rusted_kingdoms/data/maps/town_02_millhaven.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/town_02_millhaven.yaml` | `2cd6b7c48859884dcaee56f2a959acfc1b957e1d919cd06885569f8617cdd1c4` | scenario YAML / town_02_millhaven.yaml |
| ALI-0883 | `rusted_kingdoms/data/maps/town_02_millhaven_inn.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/town_02_millhaven_inn.yaml` | `893ccc3d08549b7c4b5aedfc1fe17e9002ca5d0c01c75fe6f9e142a31d29d009` | scenario YAML / town_02_millhaven_inn.yaml |
| ALI-0884 | `rusted_kingdoms/data/maps/town_02_millhaven_mill.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/town_02_millhaven_mill.yaml` | `997d79e5a3f42d444c9f81348c262a098fa0a014b62d25cf49b4cb6a0218e410` | scenario YAML / town_02_millhaven_mill.yaml |
| ALI-0885 | `rusted_kingdoms/data/maps/town_02_millhaven_shop.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/town_02_millhaven_shop.yaml` | `4c525dafeeff61caf68064ee7575004d4a32b8da2a9cb4ba67f4523ebf7a0aad` | scenario YAML / town_02_millhaven_shop.yaml |
| ALI-0886 | `rusted_kingdoms/data/maps/town_03_ruinwatch.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/town_03_ruinwatch.yaml` | `2865769e1c89213bd4f2d5b6a290f4cddf0132e736fcbcadb28f03f017060065` | scenario YAML / town_03_ruinwatch.yaml |
| ALI-0887 | `rusted_kingdoms/data/maps/town_03_ruinwatch_inn.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/town_03_ruinwatch_inn.yaml` | `74802ec30f06b6a3f2ced8fa06c5f5b2b31ec5f768802da0bc2839cd40c737aa` | scenario YAML / town_03_ruinwatch_inn.yaml |
| ALI-0888 | `rusted_kingdoms/data/maps/town_03_ruinwatch_monastery_vaults.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/town_03_ruinwatch_monastery_vaults.yaml` | `0822746fa356c6718c46732f6feff4c1b12895f04f685b6618885bc7f82efed3` | scenario YAML / town_03_ruinwatch_monastery_vaults.yaml |
| ALI-0889 | `rusted_kingdoms/data/maps/town_03_ruinwatch_shop.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/town_03_ruinwatch_shop.yaml` | `486dafd599557df3fa0c131902e2944339b90266856fa932646b3404c2e8787f` | scenario YAML / town_03_ruinwatch_shop.yaml |
| ALI-0890 | `rusted_kingdoms/data/maps/town_04_frostholm.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/town_04_frostholm.yaml` | `09890f9bc1f9f2d54b39d321927a48104bc7e03c968f95b7f33039b31537d0cd` | scenario YAML / town_04_frostholm.yaml |
| ALI-0891 | `rusted_kingdoms/data/maps/town_04_frostholm_inn.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/town_04_frostholm_inn.yaml` | `6538397056fa81488712f45f0107ef4890a5ad012f54aa6edb54548ec265a087` | scenario YAML / town_04_frostholm_inn.yaml |
| ALI-0892 | `rusted_kingdoms/data/maps/town_04_frostholm_palace.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/town_04_frostholm_palace.yaml` | `39eaa21bbb919faa15a9b2ac07d2460f735c38c352ccb1d28adf38d4ddf1d540` | scenario YAML / town_04_frostholm_palace.yaml |
| ALI-0893 | `rusted_kingdoms/data/maps/town_04_frostholm_shop.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/town_04_frostholm_shop.yaml` | `95180f808cedac5cce64e3f33e2f1833121376131787d847fe17f53c8e31a8f3` | scenario YAML / town_04_frostholm_shop.yaml |
| ALI-0894 | `rusted_kingdoms/data/maps/town_04_frostholm_vault.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/town_04_frostholm_vault.yaml` | `4dacc7b3ffde43e0556e16026e9f243ef3adfec9390a835748fc7b67c7ade3c3` | scenario YAML / town_04_frostholm_vault.yaml |
| ALI-0895 | `rusted_kingdoms/data/maps/town_05_ashenveil.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/town_05_ashenveil.yaml` | `1ce22cc262307927aa57d48ee95650112d28ed6f6cd37b70cdb4b17c972db947` | scenario YAML / town_05_ashenveil.yaml |
| ALI-0896 | `rusted_kingdoms/data/maps/town_05_ashenveil_inn.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/town_05_ashenveil_inn.yaml` | `3d340582429fa3b67ba9a1b2507ceb0f299fe1f2e9cca9ab099ae70f17a612e0` | scenario YAML / town_05_ashenveil_inn.yaml |
| ALI-0897 | `rusted_kingdoms/data/maps/town_05_ashenveil_oracle_sanctum.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/town_05_ashenveil_oracle_sanctum.yaml` | `5c3b4b1204311876050b534b389984aca60e28b371f165764a4cf5f4ffa77d2c` | scenario YAML / town_05_ashenveil_oracle_sanctum.yaml |
| ALI-0898 | `rusted_kingdoms/data/maps/town_05_ashenveil_shop.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/town_05_ashenveil_shop.yaml` | `525ca74d20859da7292960c214da2c9ba32df26c33cb7ff6c23562d912626470` | scenario YAML / town_05_ashenveil_shop.yaml |
| ALI-0899 | `rusted_kingdoms/data/maps/zone_02_open_plains.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/zone_02_open_plains.yaml` | `b4a948d0cf59a0b6b86d1084ed137a3cf82b9fd0d0c4f5ee737defc1f7ee11b4` | scenario YAML / zone_02_open_plains.yaml |
| ALI-0900 | `rusted_kingdoms/data/maps/zone_03_marshland.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/zone_03_marshland.yaml` | `70c7d0ddfc767cfd6832d23ec2f9f1fccf4c5988442009aac071c268c0669a68` | scenario YAML / zone_03_marshland.yaml |
| ALI-0901 | `rusted_kingdoms/data/maps/zone_04_ancient_ruins_01_gate.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/zone_04_ancient_ruins_01_gate.yaml` | `6e67a85e2d022a399ca2769688750516d4e2996c77db5d1cf99bffb3a8f99f02` | scenario YAML / zone_04_ancient_ruins_01_gate.yaml |
| ALI-0902 | `rusted_kingdoms/data/maps/zone_04_ancient_ruins_02_courtyard.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/zone_04_ancient_ruins_02_courtyard.yaml` | `fe7630b612a5ff75cdd79b9bdef14a2ed5b735d090cb1ad22046124ceebc6ada` | scenario YAML / zone_04_ancient_ruins_02_courtyard.yaml |
| ALI-0903 | `rusted_kingdoms/data/maps/zone_04_ancient_ruins_03_sanctum.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/zone_04_ancient_ruins_03_sanctum.yaml` | `dc10f2389ca73f91becf92b240aa3411d378e52466a98a1c6101c7bd44eac3cf` | scenario YAML / zone_04_ancient_ruins_03_sanctum.yaml |
| ALI-0904 | `rusted_kingdoms/data/maps/zone_06_mountain_pass_01.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/zone_06_mountain_pass_01.yaml` | `ed0910140400ecdbd497f72e300893f7507c7b243fcb29d4f28d3e8b31b7660c` | scenario YAML / zone_06_mountain_pass_01.yaml |
| ALI-0905 | `rusted_kingdoms/data/maps/zone_07_sunken_cave.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/zone_07_sunken_cave.yaml` | `1b72e0a51271c1184af3f3dd52731340cb086754bb7ff6de430c2d90c7705c20` | scenario YAML / zone_07_sunken_cave.yaml |
| ALI-0906 | `rusted_kingdoms/data/maps/zone_08_corrupted_forest.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/zone_08_corrupted_forest.yaml` | `cfe5a44352a61b68821559d0ffb262c786bcf929d60b4224307048ada07ed3c5` | scenario YAML / zone_08_corrupted_forest.yaml |
| ALI-0907 | `rusted_kingdoms/data/maps/zone_09_volcanic_region.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/zone_09_volcanic_region.yaml` | `9d258165f3abb51c3029243904410e77e30a3ad9c63e6b91eed2c0d7edf7bd97` | scenario YAML / zone_09_volcanic_region.yaml |
| ALI-0908 | `rusted_kingdoms/data/maps/zone_10_final_stronghold.yaml` | `assets/scenarios/rusted_kingdoms/data/maps/zone_10_final_stronghold.yaml` | `a307ca0d9c1ddcd26a8bc1d7d49c3dbd58e56cf624105a3df6a94e6329fc734e` | scenario YAML / zone_10_final_stronghold.yaml |
| ALI-0909 | `rusted_kingdoms/data/quests.yaml` | `assets/scenarios/rusted_kingdoms/data/quests.yaml` | `342f002b69dff4df8dac3d2ed45346d415e7d0f98407a2e97ab5c571792e98a4` | scenario YAML / quests.yaml |
| ALI-0910 | `rusted_kingdoms/data/recipe/all_recipe.yaml` | `assets/scenarios/rusted_kingdoms/data/recipe/all_recipe.yaml` | `58312a9df3d0fda6b6deec906b2f3cdf7ad1e3ef7ef4d8d53b850eb20bfb9d72` | scenario YAML / all_recipe.yaml |
| ALI-0911 | `rusted_kingdoms/assets/audio/bgm/Chronicles_of_the_Lost_Flame_Title.mp3` | `assets/scenarios/rusted_kingdoms/media/audio/bgm/Chronicles_of_the_Lost_Flame_Title.mp3` | `f2e777876ea1ebd7d6442e4e4311a8328be390bb36153d664df4088f846b1afc` | audio / Chronicles_of_the_Lost_Flame_Title.mp3 |
| ALI-0912 | `rusted_kingdoms/assets/fonts/Quintessential-Regular-OFL.txt` | `assets/scenarios/rusted_kingdoms/media/fonts/Quintessential-Regular-OFL.txt` | `3b022dc192aa6a748fe7302f479a66377f19f3851be46b3233600c0226dc99f5` | text / Quintessential-Regular-OFL.txt |
| ALI-0913 | `rusted_kingdoms/assets/maps/town_03_ruinwatch.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/town_03_ruinwatch.tmx` | `519be2eefd183969456c9bba007eb168edcc633af07de5b1c9b73ae3a973085a` | map XML / town_03_ruinwatch.tmx |
| ALI-0914 | `rusted_kingdoms/assets/maps/town_03_ruinwatch_monastery_vaults.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/town_03_ruinwatch_monastery_vaults.tmx` | `ebd55e9404198bb758a946d512ae80cf44f9003f68324fe35625372c5e02b72e` | map XML / town_03_ruinwatch_monastery_vaults.tmx |
| ALI-0915 | `rusted_kingdoms/assets/maps/town_04_frostholm.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/town_04_frostholm.tmx` | `5e72565ccb2d467aa09abfff317ffed42aa3f9ef0a989cbf4a77e9bb7c85da13` | map XML / town_04_frostholm.tmx |
| ALI-0916 | `rusted_kingdoms/assets/maps/town_04_frostholm_inn.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/town_04_frostholm_inn.tmx` | `aa2f7bc5f208f17843915a0facb1b3f5c502e305c63f569166810a3f8d3af95c` | map XML / town_04_frostholm_inn.tmx |
| ALI-0917 | `rusted_kingdoms/assets/maps/town_04_frostholm_palace.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/town_04_frostholm_palace.tmx` | `e36a47db51f488a3806ed8bc4ab57c94d92e9ed0a7bc0234c6af807f3395725f` | map XML / town_04_frostholm_palace.tmx |
| ALI-0918 | `rusted_kingdoms/assets/maps/town_04_frostholm_vault.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/town_04_frostholm_vault.tmx` | `e37b1c0c7e487b49949bb2b7143f9ea8a11dbdca9902224fe16d94bf7ca1c1cc` | map XML / town_04_frostholm_vault.tmx |
| ALI-0919 | `rusted_kingdoms/assets/maps/town_05_ashenveil.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/town_05_ashenveil.tmx` | `dc26a27ab5779bdb0107a0c21359ea578bdae456a478585985840e51b274e620` | map XML / town_05_ashenveil.tmx |
| ALI-0920 | `rusted_kingdoms/assets/maps/town_05_ashenveil_oracle_sanctum.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/town_05_ashenveil_oracle_sanctum.tmx` | `8e452ae2cea319e03ee49820ca0e8aa1ceebad70843155a7564004d58fa0a780` | map XML / town_05_ashenveil_oracle_sanctum.tmx |
| ALI-0921 | `rusted_kingdoms/assets/maps/zone_03_marshland.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/zone_03_marshland.tmx` | `0d48311def608fecc6399d4922a36387fe544af95fc37db48afc962f3c8af700` | map XML / zone_03_marshland.tmx |
| ALI-0922 | `rusted_kingdoms/assets/maps/zone_04_ancient_ruins_03_sanctum.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/zone_04_ancient_ruins_03_sanctum.tmx` | `0487c11c2e7d0f4a5121f6a7dd622b52dae6a67bbfbe38c39fd6e3e45ce49ad3` | map XML / zone_04_ancient_ruins_03_sanctum.tmx |
| ALI-0923 | `rusted_kingdoms/assets/maps/zone_05_mountain_foothills_01.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/zone_05_mountain_foothills_01.tmx` | `1fcc33caf3be875f11406665a67118883be1cda269858d82382198dba89f0133` | map XML / zone_05_mountain_foothills_01.tmx |
| ALI-0924 | `rusted_kingdoms/assets/maps/zone_05_mountain_foothills_03.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/zone_05_mountain_foothills_03.tmx` | `76404a6bb3f141ee5f7992f91eafdad37832a35d7c7685f0deccb235678de6c5` | map XML / zone_05_mountain_foothills_03.tmx |
| ALI-0925 | `rusted_kingdoms/assets/maps/zone_06_mountain_pass_03.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/zone_06_mountain_pass_03.tmx` | `3e6acef6caf1604b8b451ec2f0a4681dcd84bbf2fc501a51ad84eb7c27f9b123` | map XML / zone_06_mountain_pass_03.tmx |
| ALI-0926 | `rusted_kingdoms/assets/maps/zone_09_volcanic_region.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/zone_09_volcanic_region.tmx` | `1ace33b3d452b18a5d53dee2f0238fd59d40e99f45e8c6ed32f73a7531c5cf4f` | map XML / zone_09_volcanic_region.tmx |
| ALI-0927 | `rusted_kingdoms/assets/maps/zone_10_final_stronghold.tmx` | `assets/scenarios/rusted_kingdoms/media/maps/zone_10_final_stronghold.tmx` | `db506bbaecfd489c47688a876d48d1e1a2b33d836c34d3ea8f73cb88ae0d1385` | map XML / zone_10_final_stronghold.tmx |
| ALI-0928 | `rusted_kingdoms/assets/tilesets/schwarnhild/license.txt` | `assets/scenarios/rusted_kingdoms/media/tilesets/schwarnhild/license.txt` | `ff08257768903641b6fa741b46ca344978c75aad4dd7a0c4547795537e3b1aff` | text / license.txt |
| ALI-0929 | `engine/settings/settings.yaml` | `assets/settings.yaml` | `d6de41269817a10c1d34ef922ae96a9cbfa81b7251cdc4c8f3421261ef482612` | scenario YAML / settings.yaml |

## M14.05 backfill: project-authored payload files

Payload files with no counterpart in the pinned source tree at `08970359d6cb03586948625d29b0d3351dbbf785`.
These were authored in this repository. Each row is a distinct ledger entry.
Shared review fields for every row:

- creator/rightsholder: this project;
- source/evidence: no pinned-source counterpart exists; searched by the
  destination-to-source path rule on 2026-09-12;
- license, notice, and required attribution: **unknown — the project has not
  yet stated the terms under which it publishes its own content**;
- modification: not-applicable; authored here rather than copied;
- redistribution, commercial use, and derivatives: unknown, pending that grant;
- review: `needs-evidence`, automated backfill audit, 2026-09-12;
- related work: M14.05; and
- blocker: **Release blocker, but the one blocker the project can clear by
  itself.** These files need an explicit redistribution grant from the owner,
  not third-party evidence. A single owner decision covers every row here.

Authorship in this repository is recorded as a fact about provenance. It is not
a license: an owner grant is still required before these may ship, which is why
every row is `needs-evidence` rather than `approved`.

| ID | Source path in the pinned source tree | Destination path | SHA-256 | Kind/name |
| --- | --- | --- | --- | --- |
| ALI-0930 | N/A - project-authored in this repository | `assets/README.md` | `f34ea09502871ba7db7430533e1315e6ac51948791a51f18a15462d717a3949b` | markdown / README.md |
| ALI-0931 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/apothecary_ashenveil.yaml` | `c833426ed56b7b684e41fcdd7040af436ce3bd6cab58ace677ec6b13492f78af` | scenario YAML / apothecary_ashenveil.yaml |
| ALI-0932 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/apothecary_frostholm.yaml` | `1f1505a3f62e6f5d98c0330e23c2c07fd531bf2b626f98819f37a9f7183e6c92` | scenario YAML / apothecary_frostholm.yaml |
| ALI-0933 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/apothecary_ruinwatch.yaml` | `e66a7c3fd4e4283dbfe7f019f4b220c57d9c252759fac6d4603859e2989d3637` | scenario YAML / apothecary_ruinwatch.yaml |
| ALI-0934 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/ardel_epilogue.yaml` | `5f6adce1f6b78b215984560f428e05e7efe8aa5a3db1148c5bcfae0cafcf4bb3` | scenario YAML / ardel_epilogue.yaml |
| ALI-0935 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/ardel_epilogue_elise.yaml` | `b5d6504cfafbe55f44ec56d7a51ec5db50ec7b4ced071839ba86ee8ba9fdbc47` | scenario YAML / ardel_epilogue_elise.yaml |
| ALI-0936 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/ardel_epilogue_jep.yaml` | `e2af83edff1f7240b01a64fd55626ebe948f2431796f0fd588f9e6c6084bdc72` | scenario YAML / ardel_epilogue_jep.yaml |
| ALI-0937 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/ardel_epilogue_kael.yaml` | `3ffeef5852619787f270ebd65bf81e85871de03610bd20ca1e7e84704e0014b5` | scenario YAML / ardel_epilogue_kael.yaml |
| ALI-0938 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/ardel_epilogue_reiya.yaml` | `0875eb5519d8ebb6db505652fa06793cd2d9cfd0ed1f77917eccc072207fd61c` | scenario YAML / ardel_epilogue_reiya.yaml |
| ALI-0939 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/cave_delver.yaml` | `3b6638769a726d0ebd10af8584e17f13fa3d9618030f28ac8ffcaf49c3f8ce8e` | scenario YAML / cave_delver.yaml |
| ALI-0940 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/cinder_marshal_aftermath.yaml` | `744dd7645a06fefd3b3245d4976b0e1f9cac63e4eb5e9257c65ebccc2acdd7b4` | scenario YAML / cinder_marshal_aftermath.yaml |
| ALI-0941 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/cinder_marshal_duel.yaml` | `85cd54b546c9b928942de0346fedb99b4079fd8af267ed48f94c68c90c32f0c9` | scenario YAML / cinder_marshal_duel.yaml |
| ALI-0942 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/cinder_marshal_grave.yaml` | `cbb2582d5f63b4658c7ae6a507b3743c10d36116f40da770a5ef9cbb3d501fcf` | scenario YAML / cinder_marshal_grave.yaml |
| ALI-0943 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/corrupted_forest_warden.yaml` | `7d12873e0015c016ad999d0c2c72a9e9954da332b618c6b2dfa00a7863dae3c3` | scenario YAML / corrupted_forest_warden.yaml |
| ALI-0944 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/corrupted_herbalist.yaml` | `1b10eb98b5949d05acb78c3fc1b46206bd6b5b50813258c3241eb7f7ee5ebf62` | scenario YAML / corrupted_herbalist.yaml |
| ALI-0945 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/foothills_trailhand.yaml` | `b5951cd55d3a38bdc412260261db0d5ff407961c3b3f486bbdf1e8b4ae5c22fb` | scenario YAML / foothills_trailhand.yaml |
| ALI-0946 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/forest_ranger.yaml` | `171a9f2c0a1b43513bc09058047e2b2264a45aa6f8e0a30659e89cd2188a1702` | scenario YAML / forest_ranger.yaml |
| ALI-0947 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/hearth_final_choice.yaml` | `3924b94f1ab8bdc702eaeb8359d6bd44592dee9f32bcd0b7b3618a77ecdf5328` | scenario YAML / hearth_final_choice.yaml |
| ALI-0948 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/hearth_memory_aric.yaml` | `9a6753d59f72ffbe2cc70553838adb0232d9aaeee21ab41bec9a8506948b36a2` | scenario YAML / hearth_memory_aric.yaml |
| ALI-0949 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/hearth_memory_elise.yaml` | `4b99c2e5168f2eced83bce7f37e66a77d94a94ceac9371e40c8dbff97c309249` | scenario YAML / hearth_memory_elise.yaml |
| ALI-0950 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/hearth_memory_jep.yaml` | `2685a6fb28093d4bf0e877c74cc4ab8e0fb097eeae0c7466dcc2fac7b7a18e00` | scenario YAML / hearth_memory_jep.yaml |
| ALI-0951 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/hearth_memory_kael.yaml` | `10b3881fbf90e2fda43dd701d5497792d801e77a33e4070111d27bf02302447c` | scenario YAML / hearth_memory_kael.yaml |
| ALI-0952 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/hearth_memory_reiya.yaml` | `84b5058b76f4b309b8d6f3194eb5ff94286ee4a3002d53b80261ad17f4f611a3` | scenario YAML / hearth_memory_reiya.yaml |
| ALI-0953 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/hearth_threshold.yaml` | `caea89f000d328dbd09c7c2b97642a02c82542b650f7fb8b1426291ad59efe84` | scenario YAML / hearth_threshold.yaml |
| ALI-0954 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/hearth_warden.yaml` | `0db3d4e5f5ad525157a1f983fbd0e87acfc63c8b19eb4418ea7906fc53441b6b` | scenario YAML / hearth_warden.yaml |
| ALI-0955 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/marsh_farmer.yaml` | `c14bceae5001e0b90b24075a786d5c2b85ac2ae6e987cf1e3fadd60caedb5635` | scenario YAML / marsh_farmer.yaml |
| ALI-0956 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/marshland_reedcutter.yaml` | `b96cdd41981bc0ab4470497434a03785402f8b5c1ecdb361df77dcd2978f2450` | scenario YAML / marshland_reedcutter.yaml |
| ALI-0957 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/mountain_pilgrim.yaml` | `7eb25958fb601adbe3050124c55faaf2b4e6ef1f5a15ae52193413f0b7e14133` | scenario YAML / mountain_pilgrim.yaml |
| ALI-0958 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/plains_scout.yaml` | `860a26b53e2e2647ab7f880d660170aba37e0142660948436c0d8a7b9d07e811` | scenario YAML / plains_scout.yaml |
| ALI-0959 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/ruins_duelist.yaml` | `f18f664dfdc623689d4ab634b62f16e7d7a91b35be794ac2fa1f4bcf4b6760f1` | scenario YAML / ruins_duelist.yaml |
| ALI-0960 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/ruins_gate_surveyor.yaml` | `5c17935f7142cb7a70d980074fc961263a2fbbe0feffb80e97570cfc681793ed` | scenario YAML / ruins_gate_surveyor.yaml |
| ALI-0961 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/ruins_sanctum_penitent.yaml` | `d21e0a5cf35493928976a528db176baca66457f89bed4b9e50b9ee775c6072f0` | scenario YAML / ruins_sanctum_penitent.yaml |
| ALI-0962 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/sign_town_03_harborgate.yaml` | `df7a7c55917dd97bc9e15e6d9d09961114770cc91c7d477e0071e319b1124cc6` | scenario YAML / sign_town_03_harborgate.yaml |
| ALI-0963 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/sign_zone_03_marshland.yaml` | `c273e296a04ad2fa60ac2b619952f86a89a8e885e52b785caa95972f68778b72` | scenario YAML / sign_zone_03_marshland.yaml |
| ALI-0964 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/sign_zone_05_mountain_foothills_01.yaml` | `cb59c9302dc2086645c5ca84479933a92920fa2b4ba9f2a3a1ac23a781d4622d` | scenario YAML / sign_zone_05_mountain_foothills_01.yaml |
| ALI-0965 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/sign_zone_06_mountain_pass_01.yaml` | `9de960c48414bd0e690cae37812eb7864b1214cb841dfefec3f14af8ed3b82b0` | scenario YAML / sign_zone_06_mountain_pass_01.yaml |
| ALI-0966 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/sign_zone_09_marshal_camp.yaml` | `9622e2dfd01bb3627f354d21cd2d16c6bf063ca5a3175c6457699c248d961edd` | scenario YAML / sign_zone_09_marshal_camp.yaml |
| ALI-0967 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/stronghold_deserter.yaml` | `4909ef8b1f46f337c00c33bf276f757f8ca8bbfd1e5780548ebe3db346ee647b` | scenario YAML / stronghold_deserter.yaml |
| ALI-0968 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/sunken_cave_lampkeeper.yaml` | `64ffb591a3434e897683ae9db33d21f953744ebd8c5a946ca3b8261ebc5d8d38` | scenario YAML / sunken_cave_lampkeeper.yaml |
| ALI-0969 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/dialogue/volcanic_refugee.yaml` | `66d5e2005be8ccec4ba9b448f97998144b752f05a9616c862a241bf9b4c72277` | scenario YAML / volcanic_refugee.yaml |
| ALI-0970 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/encount/town_01_ardel_epilogue.yaml` | `24b4ebb1adebbfe499eec79795a74c8879d434bb223fd031951dfafcb57b6bbd` | scenario YAML / town_01_ardel_epilogue.yaml |
| ALI-0971 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/encount/zone_09_marshal_camp.yaml` | `fd18e365f8cbbdf7e1cbc9af6855ada4cba59a5eed51fd6b57c2496ea0b32a33` | scenario YAML / zone_09_marshal_camp.yaml |
| ALI-0972 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/encount/zone_10_hearth_core.yaml` | `a444cec8767129930ad317c9d8fe4d579d33c302e58f09851a818f4fc0af9283` | scenario YAML / zone_10_hearth_core.yaml |
| ALI-0973 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/enemies/boss_move_sets/cinder_marshal.yaml` | `7b56eef0362382978272491cde4bd4a4f873b7d8913815baaf2a95e1f26080be` | scenario YAML / cinder_marshal.yaml |
| ALI-0974 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/enemies/boss_move_sets/hearth_effigy_ashen_crown.yaml` | `139e6f89fa049b5c9465e7ebd53297757a7f026b0967166bdc506974969a131d` | scenario YAML / hearth_effigy_ashen_crown.yaml |
| ALI-0975 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/enemies/boss_move_sets/titch_the_ticker.yaml` | `1c3b33a20b3ef73d1591c9748689685eb681849124d6c2de2308ec45d631a360` | scenario YAML / titch_the_ticker.yaml |
| ALI-0976 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/items/migration_endgame_drops.yaml` | `bf82c70f08b5cc9da240ff6ebe78a0062ed176bb6360fa13571f3e8806ea1a10` | scenario YAML / migration_endgame_drops.yaml |
| ALI-0977 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/items/migration_zone1_drops.yaml` | `c239b2db128f9a14068e456267343dc0809159b43c578b337d08702109160c01` | scenario YAML / migration_zone1_drops.yaml |
| ALI-0978 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/maps/town_01_ardel_epilogue.yaml` | `cfa5b51b6cb7f256e0187c10880be601beac9bc7afcbea2cb49931c0ef2fc711` | scenario YAML / town_01_ardel_epilogue.yaml |
| ALI-0979 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/maps/town_03_harborgate.yaml` | `b5c96b423500fbd4737926615cc841b2638264869dc31987ee414f54c807a19c` | scenario YAML / town_03_harborgate.yaml |
| ALI-0980 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/maps/town_03_harborgate_harbormaster.yaml` | `89f3db0c3b783236fdb36e14ab2f1b6d245383948b5f9de59b996eb42b596013` | scenario YAML / town_03_harborgate_harbormaster.yaml |
| ALI-0981 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/maps/town_03_harborgate_inn.yaml` | `9918d5507b80fcbac8e8ee90e6e9c5b4de8940d05327153e364ece033f5c5ee1` | scenario YAML / town_03_harborgate_inn.yaml |
| ALI-0982 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/maps/town_03_harborgate_quarantine.yaml` | `44afedd650154df136db3571ad6dba86369b6bd90c98da9fee290a503df0a18b` | scenario YAML / town_03_harborgate_quarantine.yaml |
| ALI-0983 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/maps/town_03_harborgate_shop.yaml` | `ac7fed30861bec736e710a53132a9fd424109b7efd463d5fc9270bb52f756c33` | scenario YAML / town_03_harborgate_shop.yaml |
| ALI-0984 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/maps/zone_05_mountain_foothills_01.yaml` | `20bbf75d6d8321b896c34a713eb14555e8ff3a0bfcb7e8764be0d4b40dee9863` | scenario YAML / zone_05_mountain_foothills_01.yaml |
| ALI-0985 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/maps/zone_09_marshal_camp.yaml` | `363efec809779bd8b69f9020a5942b385b5eb28758f534b964fddad24e97ff61` | scenario YAML / zone_09_marshal_camp.yaml |
| ALI-0986 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/maps/zone_10_hearth_core.yaml` | `c7024d6d1e1939c27e23abeba37cf9c1bc6f2c6538b30812500531a33d354083` | scenario YAML / zone_10_hearth_core.yaml |
| ALI-0987 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/maps/zone_10_hearth_descent_01.yaml` | `bf67b8b2e606660c0ddeec0ef9e60cfb3a0b70a2deaa57338fee51a246089878` | scenario YAML / zone_10_hearth_descent_01.yaml |
| ALI-0988 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/maps/zone_10_hearth_descent_02.yaml` | `a9421ee01d467cd30ff620ef6925a448b4bac15cd7203f9b4857be9162c78122` | scenario YAML / zone_10_hearth_descent_02.yaml |
| ALI-0989 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/maps/zone_10_hearth_descent_03.yaml` | `382670c1a54000f078e8b31d8f1264c1aeea7211799c72b00a559a16ec0bb788` | scenario YAML / zone_10_hearth_descent_03.yaml |
| ALI-0990 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/maps/zone_10_hearth_descent_04.yaml` | `e57bb55b9e70342ef869ef8d44fd44bfae6d1847cf5725ce6d48698c46640402` | scenario YAML / zone_10_hearth_descent_04.yaml |
| ALI-0991 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/maps/zone_10_hearth_descent_05.yaml` | `b2d0ec0eba12dd7395d60e5c748b76b3b37f2af845b6390da000ac54e325b656` | scenario YAML / zone_10_hearth_descent_05.yaml |
| ALI-0992 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/maps/zone_10_hearth_descent_06.yaml` | `3b2916f3179baacd3299ecb5e37c37c15bdea3a87f7727204c179bd6e44eb6b0` | scenario YAML / zone_10_hearth_descent_06.yaml |
| ALI-0993 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/data/transport.yaml` | `a648a742737a7c7745596e4dff0bca0ea137423fe67f308ad9a8c4e157f1e488` | scenario YAML / transport.yaml |
| ALI-0994 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/media/maps/town_01_ardel_epilogue.tmx` | `15954e6c77fb6126f2e9aed88fd2d03413563e2ec0092043439e0e4926a36ecc` | map XML / town_01_ardel_epilogue.tmx |
| ALI-0995 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/media/maps/town_03_harborgate.tmx` | `c9ec5fae141df76c4cb86f78aecd901bfbbbaa6949964f838616a04ca9101656` | map XML / town_03_harborgate.tmx |
| ALI-0996 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/media/maps/town_03_harborgate_harbormaster.tmx` | `acb108d97731f01df50f8b7f89c72b5878e7105ae5c1e2fe23c644b70aa123e5` | map XML / town_03_harborgate_harbormaster.tmx |
| ALI-0997 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/media/maps/town_03_harborgate_inn.tmx` | `028b45befc49cbfe97a8565b3af49c3d4af4dc33a9081494354a77680de33936` | map XML / town_03_harborgate_inn.tmx |
| ALI-0998 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/media/maps/town_03_harborgate_quarantine.tmx` | `be4028c40c4a773330a25fbfa036f676d7930aa08fa7c39c43961c832b2e2bdb` | map XML / town_03_harborgate_quarantine.tmx |
| ALI-0999 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/media/maps/town_03_harborgate_shop.tmx` | `2ce65ce563e4219998300a5c9ceb3916fe51a1c14bc7d3b8612b023a57c55b63` | map XML / town_03_harborgate_shop.tmx |
| ALI-1000 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/media/maps/zone_09_marshal_camp.tmx` | `7e5a008cfba23aee967eff5daf14cab0fea5c02464c25b0494e3e0cba6aca13e` | map XML / zone_09_marshal_camp.tmx |
| ALI-1001 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/media/maps/zone_10_hearth_core.tmx` | `ca80683f4792c7037376206f15c1011f3654dcb71f0c67e747df08fbcc7fe078` | map XML / zone_10_hearth_core.tmx |
| ALI-1002 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/media/maps/zone_10_hearth_descent_01.tmx` | `0b5f5c4813c20c821eee54363cb3d07c41401485de945ff5d77a4ea4ea1bd4c3` | map XML / zone_10_hearth_descent_01.tmx |
| ALI-1003 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/media/maps/zone_10_hearth_descent_02.tmx` | `f6aa4df69a3f6b0155612cd7a96f1f2f4d819a8add7afbf6555e052be3df7b31` | map XML / zone_10_hearth_descent_02.tmx |
| ALI-1004 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/media/maps/zone_10_hearth_descent_03.tmx` | `94de36957a155d5b5abcca56224d5f4a4c766ed415e450cc8b0968f172f31a7a` | map XML / zone_10_hearth_descent_03.tmx |
| ALI-1005 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/media/maps/zone_10_hearth_descent_04.tmx` | `c0e095c79fcb566f005176258e44552e8afdb3bb593707621d08ba21d0a5be74` | map XML / zone_10_hearth_descent_04.tmx |
| ALI-1006 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/media/maps/zone_10_hearth_descent_05.tmx` | `5020d82f301d3859aa3c443ce9eb5b707f46a1586f613723db948c5dbef764fe` | map XML / zone_10_hearth_descent_05.tmx |
| ALI-1007 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/media/maps/zone_10_hearth_descent_06.tmx` | `4c7bace51dd54bbd0b71be9cb395c5804d6ca8c4adf7e87f492916ed8460440c` | map XML / zone_10_hearth_descent_06.tmx |
| ALI-1008 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/cinder_marshal.tsx` | `141ea8a46a581f71613da39112a05ca079b37b3ab8ac9d25b16becc648055e63` | tileset XML / cinder_marshal.tsx |
| ALI-1009 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/cinder_marshal_battle.tsx` | `8f716b1bf1bcda2104e2da46b91c922177e0031649bd11b825002b5886961cbb` | tileset XML / cinder_marshal_battle.tsx |
| ALI-1010 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/hearth_effigy_ashen_crown.tsx` | `a25c42fd72d5af1cd37068aa2506bc91381838b276a1780fe025403014e75d1b` | tileset XML / hearth_effigy_ashen_crown.tsx |
| ALI-1011 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/hearth_effigy_ashen_crown_battle.tsx` | `f4c32c62a509031af0641bda55c00ecb25ec571d4bddf532d117b4513ca125f4` | tileset XML / hearth_effigy_ashen_crown_battle.tsx |
| ALI-1012 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/titch_the_ticker.png` | `1303ef7132549769b9d0591faec42873a7fcd0805bc478e5c9e123fc673a5535` | image / titch_the_ticker.png |
| ALI-1013 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/titch_the_ticker.tsx` | `88e931e0ef60bc33659b7a2e0ce499e9648db2046d5f89f82e5df4043d09db10` | tileset XML / titch_the_ticker.tsx |
| ALI-1014 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/titch_the_ticker_battle.png` | `1e07180a794c3a0819e34496c6df7768ac54eb065201089f55f99f96ad9146c8` | image / titch_the_ticker_battle.png |
| ALI-1015 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/media/sprites/enemies/titch_the_ticker_battle.tsx` | `1d0df997dd2ae1d23a20f759150ab3a34990529ccee7bc8383854c9526d5b250` | tileset XML / titch_the_ticker_battle.tsx |
| ALI-1016 | N/A - project-authored in this repository | `assets/scenarios/rusted_kingdoms/validation-baseline.txt` | `2d1149e82d37c4447b3984b8ccb0da3f62c6d947d4e621987711f270b74409a8` | text / validation-baseline.txt |

