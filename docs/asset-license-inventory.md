# Asset License Inventory

Status: active template; no asset entries have been reviewed by this document.

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
