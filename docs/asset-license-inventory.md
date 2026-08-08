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
