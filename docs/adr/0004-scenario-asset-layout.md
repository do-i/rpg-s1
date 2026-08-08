# ADR 0004: Store scenarios under `assets/scenarios/`

- Status: Accepted
- Date: 2026-08-07
- Decision owner: M0.08
- Source snapshot: `../agentic-rpg` at
  `08970359d6cb03586948625d29b0d3351dbbf785`
- Target campaign package key: `rusted_kingdoms`
- Canonical target campaign root: `assets/scenarios/rusted_kingdoms/`

## Context

The pinned source keeps one scenario in `../agentic-rpg/rusted_kingdoms/`.
Its `manifest.yaml` is the package entry point and its references assume that
the scenario's `data/` and `assets/` directories remain siblings beneath that
entry point. YAML asset references are scenario-root-relative, while Tiled
references are relative to their containing TMX or TSX file.

ADR 0002 requires the Rust runtime and validator to read those authoring files
directly without changing their schema or internal path semantics. ADR 0003
requires Bevy runtime loads and the filesystem validator to enforce the same
scenario containment boundary. Neither decision chose where the complete
package belongs in this repository or in a release.

The current prototype is different from a scenario package. Bevy uses
`assets/` as its default asset-source directory, and `src/title_screen.rs`
loads flat paths such as `images/title_lost_flame.webp` and
`audio/title_theme.mp3`. Those files were copied only for the title-screen
evaluation and `assets/README.md` still records unresolved redistribution
questions. They must not accidentally become the permanent scenario layout.

The final release must also support direct authoring, more than one scenario,
headless validation, and execution without the Python source checkout or an
assumption about the process working directory.

## Decision

`assets/scenarios/` is the repository's scenario-package collection. Each
immediate child is one independently selectable scenario package, and that
child directory is the containment root for all files in the package.

The canonical root for the ported campaign is exactly:

```text
assets/scenarios/rusted_kingdoms/
```

After migration, its preserved layout is:

```text
assets/
├── scenarios/
│   └── rusted_kingdoms/
│       ├── manifest.yaml
│       ├── data/
│       │   ├── audio/
│       │   ├── classes/
│       │   ├── dialogue/
│       │   ├── encount/
│       │   ├── enemies/
│       │   ├── items/
│       │   ├── maps/
│       │   ├── recipe/
│       │   ├── balance.yaml
│       │   ├── battle_backgrounds.yaml
│       │   ├── party.yaml
│       │   └── quests.yaml
│       └── assets/
│           ├── audio/
│           ├── fonts/
│           ├── images/
│           ├── maps/
│           ├── sprites/
│           └── tilesets/
├── audio/                         # temporary title prototype namespace
├── fonts/                         # temporary title prototype namespace
└── images/                        # temporary title prototype namespace
```

The source-relative destination rule is mechanical: a source authoring file
at `../agentic-rpg/rusted_kingdoms/<relative-path>` is destined for
`assets/scenarios/rusted_kingdoms/<relative-path>`. This rule defines layout;
it does not authorize copying any file or settle its license status.

No extra `content/` or `game/` directory is inserted inside a package. In
particular, `manifest.yaml`, `data/`, `assets/`, the `data/encount/` spelling,
and the relationship between `assets/maps/` and `assets/tilesets/` remain as
authored.

## Package key, manifest identity, and selection

The immediate directory name is a **package key** used to locate a scenario.
The default package key is `rusted_kingdoms`. A production selector accepts a
single portable key, not a path; a key consists of lowercase ASCII letters,
digits, `_`, or `-`, begins with a letter or digit, and contains no separator
or dot component. The selected package entry point is therefore always:

```text
scenarios/<package-key>/manifest.yaml
```

That path is relative to Bevy's `assets/` asset source. The repository path
for the default manifest is consequently
`assets/scenarios/rusted_kingdoms/manifest.yaml`, while the `AssetServer` path
is `scenarios/rusted_kingdoms/manifest.yaml`.

The package key is location, not content identity. After the manifest loads,
its `id` and `version` fields identify content for saves, caches, recordings,
and compatibility checks. For the pinned source they remain `my_rpg_story`
and `1.0.0`; the directory is not renamed to match `id`, and the manifest is
not rewritten to match the directory. A save is checked against the active
manifest identity and version rather than against a developer filesystem
path. Renaming a package key alone does not create a new scenario identity.

Only one package is active in a game session. A scenario change is an explicit
selection followed by complete validation and transactional publication; it
clears scenario-owned indexes and asset state as required by ADRs 0002 and
0003. Multiple sibling packages can ship or be used in authoring without a
Rust rebuild:

```text
assets/scenarios/rusted_kingdoms/manifest.yaml
assets/scenarios/<another-package-key>/manifest.yaml
```

The first parity release may package only `rusted_kingdoms`, but code and
validation must not bake that key into reusable loaders. The default belongs
in startup configuration.

## Runtime addressing and containment

The runtime has two related roots:

1. The Bevy asset-source base is the physical `assets/` directory.
2. The active scenario root is the logical AssetServer prefix
   `scenarios/<package-key>/`.

Every scenario request is formed by joining a validated scenario-relative
path to the active logical prefix. For example:

```text
manifest reference:  assets/maps/town_01_ardel.tmx
AssetServer request: scenarios/rusted_kingdoms/assets/maps/town_01_ardel.tmx

TMX owner:           assets/maps/town_01_ardel.tmx
TMX TSX source:      ../tilesets/grass_cave_walls_24x14.tsx
normalized target:  assets/tilesets/grass_cave_walls_24x14.tsx
AssetServer request: scenarios/rusted_kingdoms/assets/tilesets/grass_cave_walls_24x14.tsx
```

Manifest and ordinary YAML references use the scenario-relative or narrower
schema root fixed by ADR 0002. TMX `source` values are resolved relative to the
containing TMX, and TSX image `source` values relative to the containing TSX.
Normalization occurs before the active scenario prefix is added.

The project-owned scenario path type rejects absolute paths, URI-like paths,
backslashes, empty components, and any normalization that leaves the selected
package. The containing-file resolver consumes `.` and `..` lexically and
emits a normalized path containing neither, so valid Tiled references such as
`../tilesets/` remain accepted only when they stay inside the package. Runtime
code must not use an unapproved-path override to reach outside Bevy's asset
source, and the broader ability of `AssetServer` to read another file under
`assets/` does not weaken the active scenario boundary.

The Bevy loaders from ADR 0003 receive and retain the owning scenario root.
Their dependency requests use normalized AssetServer paths under that same
prefix. They never open scenario files through `std::fs` and never construct a
path into `../agentic-rpg`.

Diagnostics and persisted provenance use the package key plus a path relative
to the selected package, such as
`rusted_kingdoms:assets/maps/town_01_ardel.tmx`. They do not expose the
repository location, installed package location, or process working directory.

## Headless validator and test fixtures

The headless validator selects a production scenario by the same package key
and applies the same join, normalization, and containment rules as the Bevy
runtime. Its filesystem frontend maps the logical collection to the physical
`assets/scenarios/` directory, canonicalizes the selected root and existing
targets, and rejects symlink escapes. It reads directly from the selected
package; it does not copy content to a validator-only layout.

Production code shares the scenario selector, scenario-relative path type,
parsers, and diagnostics with the validator. Only the final I/O frontend
differs. The validator's normal default is `rusted_kingdoms`, and an explicit
package key selects a sibling package. A production command does not accept an
arbitrary manifest path that could bypass the collection boundary.

Small synthetic packages used by unit and negative tests live outside shipped
assets under:

```text
tests/fixtures/scenarios/<fixture-key>/
├── manifest.yaml
├── data/
└── assets/
```

Tests inject `tests/fixtures/scenarios/` as their collection root through the
same bounded resource abstraction. They do not add fixture-specific search
rules to production selection. Corpus and integration tests use the canonical
`assets/scenarios/rusted_kingdoms/` tree once the needed files have been
migrated. Test fixtures, editor caches, previews, and `.tmx.bak` files are not
release scenarios merely because tooling can inspect them.

## Prototype transition

This ADR does not move or copy assets. The existing flat `assets/images/`,
`assets/audio/`, and `assets/fonts/` files remain the title prototype's inputs
for now, and the existing AssetServer paths remain valid during baseline
hardening. They are outside every scenario containment root and must not be
treated as implicit fallback files for missing scenario references.

A later content task may place an approved source file at its preserved path
under `assets/scenarios/rusted_kingdoms/` only after the applicable origin,
license, and attribution work is recorded. Runtime callers switch from flat
prototype paths to scenario-qualified paths only when the referenced scenario
files exist and validation covers the change. Missing scenario assets never
fall back to same-named prototype assets.

M14.06 decides which flat prototype copies are then unused and removes them
only after reference and runtime sweeps. This decision makes no redistribution
finding for the title art, title music, menu SFX, font, or wider source corpus;
M0.09-M0.11 and the per-asset M12 work retain that ownership.

## Repository, package, and working-directory behavior

Development and release use the same logical AssetServer paths. What changes
is only the physical base that supplies `assets/`:

- non-packaged development builds derive the repository asset base from the
  build manifest directory captured by Cargo, not `current_dir()`;
- tests inject a temporary or fixture asset base explicitly; and
- an installed release derives its asset base from the executable's package
  directory and expects `assets/` beside the executable.

The release layout is:

```text
rusted-kingdoms/
├── rpg-s1
└── assets/
    └── scenarios/
        └── rusted_kingdoms/
            ├── manifest.yaml
            ├── data/
            └── assets/
```

The executable and validator must therefore behave the same when launched
from the package directory, a parent directory, or an unrelated directory.
The clean-package test unsets development asset-root overrides. It also proves
that no runtime read reaches the source checkout, Python engine, repository
documentation, test fixtures, or build output.

Release construction preserves every scenario-internal relative path. It may
exclude unreferenced source artifacts only after M12/M14 validation and
licensing decisions show they are not required authoring or runtime inputs.
Canonical YAML/TMX/TSX files are shipped directly rather than replaced by a
compiled content form.

## Consequences for later milestones

- **M1:** M1.09 represents the selected package key, logical scenario prefix,
  and validated manifest AssetServer path as a resource. M1.10 reports
  package-qualified, scenario-relative errors. Startup asset-base selection
  must not depend on `current_dir()`.
- **M2:** M2.02 implements the scenario-relative path and containment rules.
  Manifest/catalog loading begins at
  `scenarios/<package-key>/manifest.yaml`; runtime and M2.26 validation share
  selection and resolution. M2 fixtures use the test collection rather than
  the release asset tree unless they intentionally exercise the migrated
  corpus.
- **M4:** TMX/TSX owners and dependencies remain under the active logical
  prefix. Valid `../tilesets/` and containing-TSX image paths normalize within
  that package before Bevy loads them. The filesystem validator additionally
  enforces canonical containment.
- **M12:** Every content task preserves its path below the source scenario
  root when placing it below `assets/scenarios/rusted_kingdoms/`. A task does
  not copy a dependency merely because this ADR names its destination;
  licensing, validation, and per-wave acceptance still apply.
- **M14:** License and reference sweeps operate on the exact package payload.
  M14.06 separates removable flat prototype assets from canonical scenario
  inputs. M14.08 packages the binary beside `assets/`; M14.09 launches that
  package from unrelated working directories; M14.12 proves no Python or
  source-checkout dependency remains.

## Rejected alternatives

### Put the scenario at the repository root

`rusted_kingdoms/` would sit outside Bevy's default approved asset source.
Runtime loaders would need a second filesystem path or an unapproved-path
escape, making dependency tracking, packaging, and containment harder to keep
identical.

### Flatten scenario content into `assets/`

Placing `manifest.yaml`, `data/`, maps, and sprites directly under the global
Bevy asset root would collide with prototype assets and prevent clean sibling
scenario packages. It would also destroy the source package boundary that
scenario-relative references and validation rely on.

### Use `assets/rusted_kingdoms/` without a collection directory

This can hold one package but gives global engine assets and additional
scenario packages no durable namespace. `assets/scenarios/<package-key>/`
makes selection and containment explicit with only one extra stable segment.

### Name the directory after manifest `id`

The source package is already known as `rusted_kingdoms` while its manifest id
is `my_rpg_story`. Forcing the two to match would turn a location decision into
a source-content rewrite and would make package renames change save identity.

### Embed or compile the campaign into the binary

This would break direct authoring and edit-relaunch parity, duplicate the
canonical authoring representation, and conflict with ADR 0002. A release
ships the package tree beside the binary instead.

## Required verification

Later implementation is conforming only when automated checks prove:

- default selection produces
  `scenarios/rusted_kingdoms/manifest.yaml` as an AssetServer path;
- a second valid package key selects a sibling package without recompilation;
- manifest id/version, not package key or filesystem path, provide content
  identity;
- source-root-relative YAML paths and containing-file-relative TMX/TSX paths
  normalize to the expected scenario-relative and AssetServer paths;
- absolute, separator-bearing package keys, lexical escapes, wrong-case
  references, symlink escapes, and cross-package references fail;
- runtime and validator diagnostics contain package-qualified relative paths
  and no machine path;
- fixture validation cannot escape `tests/fixtures/scenarios/<fixture-key>/`;
- no missing scenario asset resolves through a flat prototype fallback; and
- a release package validates and reaches playable content when started from
  at least one unrelated working directory with the source checkout absent.
