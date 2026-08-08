# ADR 0002: Load scenario authoring formats directly

- Status: Accepted
- Date: 2026-08-07
- Decision owner: M0.06
- Source snapshot: `../agentic-rpg` at
  `08970359d6cb03586948625d29b0d3351dbbf785`

## Context

Rusted Kingdoms is authored as YAML plus Tiled TMX/TSX, with ordinary image,
font, and audio assets reached through relative paths. The Rust port must
choose whether these files remain runtime inputs or become inputs to a separate
conversion pipeline.

The pinned source establishes the following behavior:

- `engine/io/yaml_loader.py` parses scenario YAML with PyYAML's safe loader,
  supports both single documents and multi-document enemy streams, and caches
  immutable results until a scenario reload.
- `manifest.yaml` is the scenario entry point. Its `refs` mapping names the
  YAML roots, including the source spelling `data/encount/`.
- `engine/world/tile_map.py` gives a TMX file directly to `pytmx`, which follows
  external TSX references and image paths. It then pre-renders visible layers
  in memory; it does not create a different map file on disk.
- TMX map identity is its filename stem. The Python runtime derives
  `assets/maps/<map-id>.tmx` and optionally reads
  `data/maps/<map-id>.yaml` rather than storing another map-path field.
- The existing Pygame and web map editors read and write the same TMX files.
  Portal edits preserve the Tiled property names `target_map`,
  `target_position_x`, and `target_position_y`.
- The pinned corpus contains 193 YAML files, 47 normal TMX maps, and 283 TSX
  files. YAML roots include mappings, lists, and multi-document streams. The
  Tiled corpus uses external TSX/image references and contains both runtime
  structures and editor metadata.
- The pinned Python validator passes this corpus, but it primarily checks
  references and flags. It does not provide a complete structural schema and
  does not catch every missing asset, including the manifest cursor mismatch
  reserved for M0.12.

The port's parity checklist also requires content authors to change YAML or a
TMX map, relaunch the same binary without rebuilding it, and observe the edit.
Both existing map editors must continue to produce maps the Rust runtime can
load.

## Decision

The Rust validator and runtime will read the scenario's **YAML, TMX, and TSX
authoring files directly**. These authoring files are the canonical shipped
scenario representation. There will be no required YAML-to-Rust, TMX-to-Rust,
or TSX-to-Rust preprocessing step and no compiled data form that replaces them
in a release package.

The exact Rust YAML library and the exact TMX/TSX parser or crate are
implementation choices. M0.07 will decide the TMX approach and its supported
feature matrix. This ADR requires that the chosen implementation accept every
game-relevant construct in the pinned corpus; it does not choose that
implementation. M0.08 will choose the final in-repository scenario root; this
ADR only requires preservation of scenario-internal relative paths.

## Meaning of unchanged

"Unchanged" is a compatibility promise about authored syntax, schema, names,
and path semantics, not a promise about bytes in memory:

- A pinned source file placed beneath the eventual scenario root, with its
  internal directory relationships preserved, must not need a schema rewrite,
  field rename, filename rewrite, generated sidecar, or engine rebuild before
  Rust can consume it.
- Existing source spellings and conventions remain valid. In particular,
  `refs.encount`, the `data/encount/` directory, TMX layer/property names, and
  TSX references are not "corrected" only for Rust aesthetics.
- YAML comments, mapping order, quote style, and XML whitespace/attribute order
  need not survive parsing because the game does not write these files. Tiled
  or another editor may serialize semantically equivalent XML differently.
- Parsing into typed Rust values, resolving references, uploading textures,
  building render data, and keeping in-memory caches are runtime work, not
  format conversion.
- The source checkout is not a runtime location. M0.08 and packaging may move
  the whole scenario tree, provided references still resolve with the same
  scenario-relative or containing-file-relative rules.
- An intentional content repair is still a content change. The missing cursor
  asset reference must be resolved explicitly by M0.12; this ADR neither picks
  a replacement filename nor permits the loader to guess one.

The promise is limited to the pinned corpus and the documented schema it
exercises. It is not a promise to accept every document PyYAML's safe loader or
every Tiled release can parse.

## Compatibility boundary

### YAML syntax and schema

Rust must accept the YAML forms present in the pinned scenario: UTF-8 text,
comments, plain and quoted scalars, booleans, nulls, integers including digit
separators, floats, block and flow collections, mapping-root files, list-root
files, and the enemy catalogs' multi-document streams. The pinned corpus uses
no custom tags or anchor/merge-key schema and has no duplicate mapping keys.

Custom YAML tags, non-string mapping keys, duplicate mapping keys, and values
outside the supported scalar/list/mapping model are rejected. Anchors and
aliases are not part of the compatibility promise even if a selected library
can parse them. This keeps the Rust contract tied to the authored corpus rather
than to PyYAML-specific behavior.

Known gameplay mappings are closed schemas. An unknown YAML field is an error,
not something silently dropped. A field that exists in the pinned source but
is not yet acted on by an early milestone remains a **known** field and must be
parsed and validated so later work cannot mistake a typo for forward
compatibility. There is no general extension-key escape hatch in the pinned
schema.

Required fields, types, enum values, ranges, identities, and collection root
shapes are strict. Rust must not broadly coerce strings to numbers or malformed
values to empty collections merely because a Python call site once used
`int()`, `float()`, or `dict.get()`.

Defaults are allowed only when they reproduce an observed source rule and are
named in the Rust schema and a focused test. They must not arise accidentally
from a blanket Serde default. Source rules that must remain expressible
include:

- same-stem map metadata is optional and absence produces no NPC, item-box,
  map-BGM, or encounter override data;
- a missing map metadata `id` uses the filename stem as effective identity;
- dialogue utility files and boss move-set files may use their referenced
  filename/path as identity and do not gain a required document `id`;
- genuinely optional gameplay fields retain their pinned defaults, such as an
  ordinary enemy not being a boss when `boss` is absent; and
- missing required data remains an error even if a Python consumer would fail
  only when that feature was reached.

M2 owns the complete per-field inventory and tests. If later inspection finds
another permissive Python fallback, it becomes a documented compatibility
default or a deliberate rejection; it must not become an unreviewed generic
default.

### TMX and TSX

TMX and TSX remain standard XML authoring files, including their external TSX
and image references. Runtime behavior is based on XML meaning, not formatting
or exact bytes. All game-relevant constructs observed in the pinned corpus,
including Tiled flip bits in tile GIDs, must survive the M0.07 support review.
Editor-only data present in the source, such as Wang-set authoring metadata,
may be ignored at runtime when it cannot affect the already-authored map, but
it must not require removing or rewriting the source file.

Gameplay-reserved structures are strict. Misspelled required layer names,
portal properties with the wrong Tiled type, invalid object geometry, broken
GIDs, and missing external TSX/image references are validation errors. General
Tiled metadata can be ignored only when M0.07 records that it has no runtime
semantics. M0.07, not this ADR, will enumerate supported and unsupported Tiled
features and decide how unknown XML elements or attributes are classified.

Only `.tmx` files are map inputs. The two `.tmx.bak` files are editor backups,
not maps. The two `sample_*.tmx` files are not members of the portal-reachable
campaign merely because they share the map directory, though tools may load
them explicitly as authoring fixtures.

### Relative paths

YAML paths are resolved relative to the selected scenario root unless their
specific source schema defines a narrower root. Existing audio-index entries,
for example, resolve below `assets/audio/`. A TMX `source` is resolved relative
to its containing TMX file, and a TSX image `source` is resolved relative to
its containing TSX file, matching Tiled. Thus source references such as
`../tilesets/...` remain valid when their normalized destination stays inside
one scenario package.

Absolute paths and any normalized or canonicalized path that escapes the
scenario root are rejected. Symlink resolution must not allow containment to
be bypassed. Diagnostics and persisted provenance use scenario-relative paths,
not developer-machine paths. The contract uses the source's case-sensitive,
forward-slash names; the loader does not probe alternate spellings or silently
substitute a same-looking file.

Engine implementation configuration is outside this boundary. In particular,
Python's `engine/settings/settings.yaml`, Python saves, and Python input-record
files are not scenario authoring data. Native save compatibility is governed by
ADR 0001.

## Known source anomalies

The following are compatibility cases, not reasons to transpile the package:

- Map identity remains the TMX filename stem, and metadata remains an optional
  same-stem YAML lookup. Therefore
  `zone_02_open_plains_cave_01`, `zone_02_open_plains_cave_02`, and
  `zone_05_mountain_foothills_01` load without map metadata, matching Python.
- `data/maps/zone_05_mountain_foothills.yaml` is not silently attached to the
  `_01` TMX. The validator reports the unmatched file as a source-compatibility
  warning until a later, explicit content decision changes it.
- Sixteen map files without `id`, the two dialogue files without `id`, and the
  id-less boss move sets use the filename/reference defaults above. They are
  not bulk-rewritten to add ids.
- Enemy rank files remain YAML multi-document streams. They are not converted
  to list-root documents.
- `encount` remains the manifest key and directory spelling. No hidden alias to
  `encounter` is introduced.
- The manifest's nonexistent cursor filename is an error under strict path
  validation. M0.12 must record and test the chosen content repair; runtime
  fallback by filename similarity is rejected.

Warnings for known anomalies must be stable and identifiable so Gate 2 can
distinguish accepted source behavior from newly introduced bad data. A warning
must never turn a required missing asset or malformed reference into a usable
placeholder during validation.

## Validation and runtime responsibilities

The standalone validator and game runtime must share the same Rust parsing,
typed-schema, path-resolution, and reference-validation code. The validator is
the comprehensive authoring interface: it aggregates diagnostics, checks the
whole reachable package, and exits unsuccessfully on errors without modifying
content.

The runtime must not assume that the validator was run. At scenario selection
or startup it validates the manifest and required catalogs before publishing a
live scenario resource. Lazily loaded YAML, TMX, TSX, and assets are validated
through the same code before a map or scene becomes live. A failure leaves the
previous application state intact and cannot create a partially initialized
game session.

Errors include a scenario-relative file, YAML document number and field path
or XML element/property where available, and the cause. Reference errors also
name the owning record and target. Syntax errors should include line/column
information supplied by the parser. Multiple validator errors may be
aggregated; runtime presentation may stop at the failing load boundary but
must retain the same contextual diagnostic.

CI and release packaging validate the exact shipped authoring tree. Passing an
older generated cache or an independently maintained compiled copy is not
evidence that the shipped YAML/TMX/TSX is valid.

## Editor interoperability

The existing Tiled-based, Pygame, and web authoring flows continue to target
the same TMX/TSX/YAML tree the game reads. Saving a supported edit and
relaunching the game must expose that edit without compiling the binary or
running a conversion command. The Rust runtime does not need to preserve XML
formatting because it does not own map writes; editor round-trip preservation
is assessed against the editor, source control diff, validator, and game load.

M13.13 may reuse, adapt, or replace editor frontends, but it cannot introduce a
different canonical runtime map format without superseding this ADR. Editor
backup files, caches, and previews remain derived tooling artifacts.

## Snapshot and schema evolution

This compatibility adapter is identified by the full source commit above. The
manifest `version: 1.0.0` is scenario content identity; it is not treated as a
complete schema-version declaration that authorizes unknown fields.

Conforming edits within the accepted schema load without a Rust rebuild. Moving
the source pin, adding a new field or Tiled feature, or changing an existing
field's meaning requires a dedicated snapshot update, corpus diff, updated
fixtures, and an explicit compatibility review. Rust must not silently broaden
the adapter because a newer source checkout happens to be nearby. A later
schema-version mechanism can be added by a separate decision without rewriting
the meaning of this pinned adapter.

## Performance and caching

Direct loading does not require reparsing every file every frame. YAML catalogs
may be parsed and indexed once per scenario load, and TMX/TSX render data and
textures may be cached on first use. Scenario change or explicit authoring
reload clears the affected caches.

An optional on-disk cache may be considered later only as a disposable
acceleration. It must be keyed by source content, schema-adapter identity, and
relevant engine version; a miss or corrupt entry falls back to the canonical
authoring files. A release may not omit those files or require a cache-building
tool. Performance evidence in M13/M14 determines whether such a cache is worth
adding.

## Consequences

Benefits:

- source content can be migrated without a parallel schema rewrite;
- Tiled and both existing map editors remain useful;
- author changes are visible without rebuilding Rust;
- validation and runtime exercise exactly what is shipped; and
- Python is unnecessary even though its data formats remain compatible.

Costs:

- Rust must support the pinned YAML shapes and the Tiled feature subset rather
  than choosing only the easiest native representation;
- strict typed validation requires modeling fields Python dictionaries once
  ignored or failed on late;
- source anomalies need explicit defaults, warnings, or tracked content fixes;
  and
- startup/on-demand parsing has a measurable cost that caching must manage.

## Rejected alternatives

### Compile YAML/TMX/TSX into a Rust-native release format

Rejected because the compiled output would become a second contract, complicate
editor use and diagnostics, and violate the no-rebuild authoring checks. It can
also conceal invalid shipped source behind stale generated output.

### Redesign and normalize the source schema during the port

Rejected because renaming fields or directories, adding ids mechanically, or
converting enemy streams and maps would combine engine migration with a large
content migration. It would make parity disagreements harder to attribute.

### Reproduce Python's permissive dictionary behavior

Rejected because silent unknown fields, late failures, broad coercion, and
implicit empty defaults hide authoring mistakes. Compatibility is provided by
explicit observed defaults, not by retaining every accidental permissive path.

### Keep Python or Python-generated output in the runtime pipeline

Rejected because the release must be self-contained and Python-independent.
Direct format compatibility requires Rust parsers and validators, not Python
execution.

### Maintain separate editor and runtime map formats

Rejected because conversion drift would make an editor save insufficient proof
of game behavior and would add another artifact to version and package.

## Effect on later plan tasks

- **M0.07:** choose the Rust TMX/TSX loading implementation and record the
  pinned Tiled support matrix. It may not require source conversion.
- **M0.08:** choose a canonical scenario root while preserving the internal
  relative-path semantics defined here.
- **M0.12:** repair the cursor reference explicitly; do not add a fuzzy runtime
  fallback.
- **M2.01-M2.23:** define typed schemas that deserialize the original YAML
  shapes, roots, multi-document streams, and explicit source defaults.
- **M2.02, M2.08-M2.09:** enforce root containment and contextual required-field
  and path errors without rejecting valid containing-file-relative Tiled
  references.
- **M2.24-M2.27:** share strict schema/reference validation with runtime, add
  original pinned positive fixtures plus focused rejected cases, and record
  known source warnings. Gate 2 does not pass until M0.12's cursor error is
  resolved.
- **M2.26-M2.28:** validate the direct authoring tree; the command does not
  compile, normalize, or rewrite it.
- **M4:** load and render the original TMX/TSX maps, layers, GIDs, properties,
  images, and same-stem optional metadata through the M0.07 implementation.
  Map activation remains transactional on load failure.
- **M12:** migrate campaign waves by preserving source formats and relative
  references. Any content repair or accepted behavior change is a reviewable
  source diff, not a hidden converter rule; each wave validates and plays from
  the same files.
- **M13.10-M13.14:** sweep every direct map/data input, keep editors targeting
  those files, and document edit-validate-run/replay without a conversion
  command. Profiling determines whether disposable caches are justified.
- **M14.01-M14.04:** parity and replay evidence comes from the shipped direct
  authoring data, including all accepted anomaly behavior.
- **M14.07-M14.09:** measure parsing/cache costs and package the canonical
  YAML/TMX/TSX plus assets in a self-contained layout selected by M0.08.
- **M14.10-M14.11:** document authoring/validation and archive the source hash,
  compatibility decisions, known warnings, and final parity report.
- **M14.12:** prove that direct YAML/TMX/TSX loading uses no Python runtime,
  module, generated Python output, or source checkout.
