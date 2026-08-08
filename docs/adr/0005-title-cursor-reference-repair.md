# ADR 0005: Repair the title cursor reference during migration

- Status: Accepted
- Date: 2026-08-07
- Decision owner: M0.12
- Source snapshot: `../agentic-rpg` at
  `08970359d6cb03586948625d29b0d3351dbbf785`

## Context

The pinned source manifest sets `title.cursor_icon` to
`assets/images/icons/arrow-head-red-right-01.webp`. That file does not exist.
The source title scene treats a missing cursor file as optional, silently
falling back to a text-space-width cursor. This is a player-visible regression,
not a valid compatibility behavior for the port.

The source history explains the mismatch. Commit `e75d1ce` initially selected
`arrow-head-orange-right.webp`. Commit `582e748` removed the orange and blue
arrow assets, introduced `arrow-head-left.webp`, `arrow-head-right-01.webp`,
and `arrow-head-right.webp`, then changed the manifest to the nonexistent
`arrow-head-red-right-01.webp` name.

At the pinned snapshot the viable right-facing candidates are:

| File | Dimensions | SHA-256 | Assessment |
| --- | ---: | --- | --- |
| `assets/images/icons/arrow-head-right-01.webp` | 202 x 153 | `9a5e521b06515e674bbc7c530fa1105772fb68b7610b6611cdc2e9bd68335d1b` | Small teal arrow; does not match the requested red cursor. |
| `assets/images/icons/arrow-head-right.webp` | 434 x 293 | `0792fe370ece7ccd57d7ce4694f39cc892fd5235c589c485c5ca76f3b1fad901` | Full-sized red right-facing arrow; matches the broken reference's semantic intent. |

`arrow-head-left.webp` is red but faces left and therefore is not a candidate.

## Decision

When the source manifest is brought into the target scenario package, replace
only this broken reference:

```text
assets/images/icons/arrow-head-red-right-01.webp
```

with this existing source-relative path:

```text
assets/images/icons/arrow-head-right.webp
```

This is a one-entry migration compatibility repair. Preserve all other
manifest values verbatim. Do not change the Python source checkout, add a
runtime fallback for the broken filename, or copy the asset before its
provenance and redistribution status has been approved in the asset-license
inventory. When the asset is approved and migrated, its destination is defined
by ADR 0004 as
`assets/scenarios/rusted_kingdoms/assets/images/icons/arrow-head-right.webp`.

The Rust manifest loader and validator must require the repaired reference to
resolve within the selected scenario package. A missing cursor image is an
error at validation time; it must not silently degrade to an empty/text cursor.

## Reproducible evidence

The following disposable fixture was used; it leaves the source checkout
unchanged. It has a copied manifest and symlinks its `assets/` and `data/`
directories to the pinned source. The direct existence check passed, then the
fixture manifest was changed only at `title.cursor_icon` and validated:

```bash
fixture_dir="$(mktemp -d /tmp/rusted-kingdoms-m0-12.XXXXXX)"
mkdir "$fixture_dir/scenario"
ln -s "$(pwd)/../agentic-rpg/rusted_kingdoms/assets" "$fixture_dir/scenario/assets"
ln -s "$(pwd)/../agentic-rpg/rusted_kingdoms/data" "$fixture_dir/scenario/data"
cp ../agentic-rpg/rusted_kingdoms/manifest.yaml "$fixture_dir/scenario/manifest.yaml"
test -f "$fixture_dir/scenario/assets/images/icons/arrow-head-right.webp"
perl -0pi -e 's#assets/images/icons/arrow-head-red-right-01\\.webp#assets/images/icons/arrow-head-right.webp#' "$fixture_dir/scenario/manifest.yaml"
(cd ../agentic-rpg && .venv/bin/python tools/validate.py --root "$fixture_dir/scenario")
rm -rf "$fixture_dir"
```

The validator reported `RESULT: PASS`, with its pre-existing warning that
`aric_teleport_unlocked` is defined but unconsumed. The source validator does
**not** inspect `title.cursor_icon`; the direct `test -f` is therefore the
evidence that this repair resolves the cursor path. M2's Rust validator must
add the missing manifest-asset check rather than treating this Python pass as
coverage for it.
