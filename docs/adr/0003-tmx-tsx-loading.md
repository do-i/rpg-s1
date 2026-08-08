# ADR 0003: Load TMX and TSX through a strict `tiled` adapter

- Status: Accepted
- Date: 2026-08-07
- Decision owner: M0.07
- Source snapshot: `../agentic-rpg` at
  `08970359d6cb03586948625d29b0d3351dbbf785`
- Evaluated Rust stack: Rust 1.97.1, Bevy 0.19.0, `tiled` 0.16.0

## Context

ADR 0002 makes the scenario's TMX and TSX authoring files canonical runtime
inputs. M0.07 must select the Rust loading approach before M4 builds the first
rendered map. The selection has to satisfy several boundaries at once:

- load the pinned files without conversion or source-tree path assumptions;
- preserve Tiled's containing-file-relative TMX-to-TSX and TSX-to-image paths;
- enforce the port's narrower, auditable format profile rather than silently
  accepting every construct a general Tiled parser understands;
- share parsing and validation between a windowless command-line validator and
  the Bevy runtime;
- work through Bevy's asset I/O and dependency tracking instead of opening
  asset files behind the `AssetServer`;
- leave rendering, collision, portals, spawn markers, and entity ordering under
  game-owned systems; and
- produce contextual errors and transactional map activation.

The Python implementation uses `pytmx.load_pygame`, then projects the parsed
map into game-specific behavior. `engine/world/tile_map.py` pre-renders visible
tile layers in source order. `tile_map_factory.py`, `portal_loader.py`, and
`collision.py` interpret the reserved `spawn_tile`, `portals`, `boss_enemy`,
and `collision` structures. The Rust port needs the same separation between
format parsing and game semantics, but with stricter validation than the
permissive Python call sites.

## Evidence and confidence boundary

The corpus facts below come from a complete XML audit of all normal `.tmx` and
`.tsx` files in the pinned scenario. The two `.tmx.bak` editor backups were
excluded. As a separate compatibility check, a temporary, ignored Cargo
project loaded every audited file through exact `tiled` 0.16.0 with default
features disabled: all 47 TMX and 283 TSX files parsed, with zero failures.
That check did not add a production dependency or implementation to this
repository.

Published crate and Bevy API facts were rechecked on 2026-08-07 against the
primary documentation linked in [References](#references). The integration
shape, strict profile, cache policy, and rejection rationale are this ADR's
design decisions. They remain to be proved in production code and tests during
M4; successful standalone corpus parsing is not evidence that the Bevy asset
boundary or renderer is already implemented.

## Pinned corpus audit

### Files and map structure

| Area | Complete-corpus observation |
| --- | --- |
| TMX files | 47: 45 campaign maps and 2 explicit samples |
| TSX files | 283: 18 map tilesets and 265 sprite tilesets |
| XML versions | Every root declares Tiled format `version="1.10"`; TMX producer versions are 1.12.1 or 1.12.2 and TSX producer versions are 1.12.0 or 1.12.1 |
| Map grid | All maps are finite, orthogonal, right-down, with 32 by 32 pixel map tiles |
| Layers | 170 direct tile layers and 55 direct object groups; no group or image layers |
| Tile-layer names | `collision` 47, `ground` 46, `decoration` 38, `spawn_tile` 17, `terrain` 13, `signs` 5, and `over_ground` 4 |
| Visibility | Two layers are explicitly hidden, both in `zone_09_volcanic_region.tmx`: `collision` and `spawn_tile` |
| Layer data | All 170 tile layers use uncompressed CSV with an exact width-times-height cell count; 161,066 cells total |
| Object groups | `portals` 45 and `boss_enemy` 10 |
| Objects | 109 rectangle objects: 99 have non-zero bounds and 10 are zero-sized rectangle markers; there are no Tiled `<point>`, polygon, polyline, ellipse, text, or tile objects |
| Object properties | 92 portals each have `target_map` as a string plus `target_position_x` and `target_position_y` as integers; 276 object properties total |

Layer names are a format-profile inventory, not a statement that all layers
are rendered. In particular, `collision` and `spawn_tile` carry gameplay data
regardless of Tiled visibility, and collision tiles must never be drawn.

### Tilesets, references, GIDs, and metadata

| Area | Complete-corpus observation |
| --- | --- |
| TMX tilesets | 264 declarations: 263 external references to 17 distinct TSX files and one inline atlas in `sample_01.tmx` |
| Reference integrity | Every external TSX and all 283 TSX image references resolve inside the scenario tree |
| Atlas layout | Every TSX is a single-image, zero-margin, zero-spacing atlas; no image-collection tilesets occur |
| Tile sizes | 158 TSX use 64 by 64 tiles, 106 use 256 by 256, and 19 use 32 by 32; every TSX attached to a TMX map uses 32 by 32 tiles |
| Images | Every atlas source is a PNG, and declared image dimensions, columns, and tile counts agree arithmetically |
| GIDs | 59,833 non-zero cells; every stripped GID is in a declared tileset range, and `firstgid` ranges are ordered and non-overlapping |
| Flip flags | 202 flagged cells in 13 maps: 10 horizontal, 2 vertical, 189 horizontal-plus-diagonal, and 1 vertical-plus-diagonal; no 120-degree rotation bit occurs |
| Animation | `assets/sprites/party/01_aric_walk.tsx` has the only animations: four animations, eight frames each, all at 100 ms |
| Tile properties | `grass_cave_walls_24x14.tsx` has 30 string `desc` properties; no other per-tile properties occur |
| Wang metadata | `assets/tilesets/ground/terrain-v7.tsx` has 19 Wang sets, 35 Wang colors, and 950 Wang tiles; no legacy terrain definitions occur |
| Absent structures | No infinite chunks, nested groups, image layers, embedded images, templates, tile collision object groups, image collections, terrain attributes, class/type attributes, probability attributes, tile offsets, grids, or transformations |

The 283-file TSX audit matters even though only 17 external TSX files are
currently reached from TMX. Standalone sprite TSX is canonical scenario data,
and Aric's animation and the 64/256-pixel sprite atlases are later port inputs.

## Decision

Use the standalone [`tiled`](https://crates.io/crates/tiled) crate as the TMX
and TSX semantic parser, behind a project-owned strict adapter. The first M4
implementation will pin:

```toml
tiled = { version = "=0.16.0", default-features = false }
```

Default features are disabled because the accepted profile permits only
uncompressed CSV layer data and does not need the crate's optional Zstandard
support. The adapter must reject unsupported encodings before semantic parse;
disabling a crate feature is not itself validation.

The project-owned boundary is provisionally named `TmxProfileAdapter`. The
name is descriptive, not a required module path. It will:

1. resolve every input through a scenario-bounded resource abstraction;
2. perform a shallow XML/profile pass that inventories elements, attributes,
   external references, data encodings, and object/property shapes;
3. reject constructs outside the support matrix below;
4. give prefetched TMX/TSX bytes to a fresh `tiled::Loader` through a custom
   synchronous `ResourceReader`;
5. project `tiled` values into game-owned, renderer-independent Rust data;
6. apply domain validation for dimensions, GID ranges, reserved structures,
   paths, properties, and animation frames; and
7. return structured diagnostics with scenario-relative provenance.

The shallow pass is not a second semantic TMX parser. It exists because a
general-purpose parser may successfully ignore or support XML that this pinned
profile must reject, and because the synchronous `ResourceReader` needs its
external XML inputs prefetched at Bevy's asynchronous asset boundary.

No Bevy-specific Tiled integration crate is selected. `tiled` parses authoring
data only. Game code owns entities, transforms, render layers, atlases,
collision occupancy, portal destinations, spawn markers, animation clocks,
and the ordering of tiles versus Y-sorted world entities.

## Supported profile

“Observed” means the construct occurs in the complete pinned corpus. “Required
fixture” means it is absent from the corpus but already required by the port
plan and must be proved with a small synthetic fixture before the relevant M4
task closes.

| Construct | Evidence | Accepted behavior |
| --- | --- | --- |
| TMX and TSX XML | Observed | UTF-8 Tiled XML with root format version 1.10 loads directly. Producer `tiledversion`, element IDs, and `nextlayerid`/`nextobjectid` are retained as diagnostics/editor metadata but do not change gameplay. |
| Finite orthogonal maps | Observed | `orientation="orthogonal"`, `renderorder="right-down"`, `infinite="0"`, positive dimensions, and a 32 by 32 map grid are required. |
| Direct tile layers | Observed | The seven audited names load in document order. Dimensions must equal the map dimensions. `visible`, opacity, and IDs are parsed; collision and spawn semantics do not depend on visibility. |
| CSV data | Observed | Decimal GIDs plus the audited whitespace and line-break form load. Exactly width times height values are required. Empty cells use GID zero. |
| External TSX | Observed | `firstgid` plus a containing-TMX-relative `source` load. Normalized references must be unique, contained, present, ordered, and non-overlapping. |
| Inline atlas tileset | Observed | The single-image inline form used by `sample_01.tmx` loads under the same atlas rules as external TSX. |
| Single-image atlases | Observed | Positive tile size, columns, tile count, PNG source, and declared image dimensions load. Atlas arithmetic must be consistent. Map-attached tilesets must match the 32 by 32 map grid; standalone sprite TSX may use the audited 32, 64, or 256 pixel square tiles. |
| GID flags | Five of eight combinations observed; all require fixtures | Empty GID zero and all horizontal/vertical/diagonal combinations are decoded by masking Tiled's flags before tileset lookup. Rendering must use Tiled's orthogonal transformation order. |
| Global-to-local GID lookup | Observed | A stripped non-zero GID resolves through ordered `firstgid` ranges and must be below the selected tileset's exclusive end. |
| Rectangle object groups | Observed | Direct `portals` and `boss_enemy` groups load in source order. Object ID, optional name, bounds, and properties are retained. Zero-sized rectangle markers remain valid; they are not reclassified as `<point>` objects. |
| Properties | String/integer observed; float/boolean required fixture | String, signed integer, finite float, and boolean Tiled property values load without broad coercion. Portal properties keep their source types and required names. |
| Tile animation | Observed | Ordered tile IDs and positive integer millisecond durations load. Aric's four eight-frame, 100 ms animations must remain exact. |
| Wang sets | Observed metadata | Syntactically valid Wang sets/colors/tiles are accepted and ignored by runtime because authored layer GIDs already encode the result. Their presence is available to validation diagnostics. |
| Tile `desc` strings | Observed metadata | The audited per-tile string is accepted and retained as non-gameplay metadata. |
| External PNG images | Observed | TSX-relative or inline-TMX-relative forward-slash paths load only when contained in the scenario and present. Image decode remains Bevy's responsibility. |

Missing optional elements use only Tiled-standard defaults exercised by this
profile, such as `visible=true`, `opacity=1`, margin/spacing zero, and an empty
optional object name. Reserved game structures get explicit validation:

- layer and object-group names must be from the audited allowlist, must not be
  duplicated where game lookup expects one, and must remain direct children;
- every map has exactly one `collision` layer, while other audited layers may
  be absent as they are in the corpus;
- portal objects require one string `target_map`, one integer
  `target_position_x`, and one integer `target_position_y`, with no missing or
  duplicate property;
- a `boss_enemy` marker must be a rectangle object whose source position can be
  snapped to the map grid by game code; and
- animation frame IDs, ordinary tile IDs, and stripped GIDs must fit the
  owning tileset.

## Explicitly unsupported profile

`tiled` can parse constructs beyond this list. The adapter must still reject
the following until a snapshot/profile review deliberately adds one:

| Category | Rejected constructs |
| --- | --- |
| Other map projections | Isometric, staggered, and hexagonal orientation; render orders other than right-down; 120-degree rotation flags |
| Infinite or sparse maps | `infinite="1"`, chunks, or layer dimensions that differ from the map |
| Other layer encodings | XML `<tile>` children, Base64 data, gzip/zlib/Zstandard compression, or malformed/extra CSV cells |
| Layer composition/effects | Group layers, image layers, nested layers, parallax origins/factors, offsets, tint colors, blend modes, or repeating image layers |
| Other object forms | Tiled `<point>`, tile, polygon, polyline, ellipse, text, or template-backed objects; per-tile collision object groups |
| Other tileset forms | Image-collection tilesets, embedded image data, non-PNG images, non-zero margin/spacing, tile offsets, grid/transform declarations, tile probability, and legacy terrain definitions |
| Other properties/schema | Color, file, object-reference, class, or custom property types; custom class/type fields; duplicate properties; unknown reserved layer/object-group names |
| Other containers | JSON TMJ/TSJ, Tiled world files, external templates, and remote URI references |
| Path exceptions | Absolute paths, backslash aliases, case probing, missing targets, lexical escapes, or symlink/canonical escapes from the configured scenario root |

Unknown XML elements or attributes are errors unless they are on a small,
documented editor-metadata allowlist. The initial allowlist covers the observed
Tiled format/producer versions, IDs, next-ID counters, Wang metadata, and tile
`desc` metadata. Parser acceptance is never sufficient reason to silently
broaden the profile.

## Asset I/O and path integration

The same adapter serves two resource frontends:

### Bevy runtime

A custom Bevy 0.19 `AssetLoader` owns `.tmx` loads; standalone `.tsx` loads may
use the same core through a companion loader or a typed scenario load. The
loader must not use `std::fs` or `tiled::FilesystemResourceReader` for scenario
assets.

For a TMX load, the Bevy boundary will:

1. receive the root TMX bytes and owning asset path from Bevy;
2. run the shallow profile/reference scan;
3. normalize each TSX path relative to its owning TMX, reject containment
   violations, and fetch it with `LoadContext::read_asset_bytes`;
4. scan fetched TSX plus an inline tileset for PNG references, resolve each
   relative to its owning XML file, and verify the target through Bevy asset
   I/O;
5. construct an in-memory `ResourceReader` over the root TMX and external TSX
   byte map, then call a fresh `tiled::Loader`; and
6. create `Handle<Image>` values with `LoadContext::load` for normalized PNG
   paths and retain owner/target provenance alongside those dependencies.

`read_asset_bytes` records loader dependencies in Bevy 0.19, and `load` records
normal asset dependencies. Consequently a TSX change can invalidate/reparse
the owning TMX asset and image loads remain under Bevy's image loader. The game
must wait for the map and all recursive image dependencies to be loaded before
publishing the map as live. A failed image decode, reference, or reload is a
map-load failure, not permission to render a partial map.

The profile scanner and resource abstraction receive an opaque configured
scenario root. This ADR does not choose its in-repository path; M0.08 owns that
decision. Relative paths are normalized using the containing XML path before
the root-containment check. A filesystem-backed validator additionally
canonicalizes the root, owner, and existing target to prevent symlink escape.
Asset-source implementations without symlinks must enforce the equivalent
logical containment rule.

### Headless validator

The standalone validator supplies the same adapter with a bounded filesystem
reader. It performs no Bevy window, renderer, GPU, or image upload work. It
loads all 47 normal TMX files and all in-scope TSX files, validates every XML
and image reference, and aggregates diagnostics rather than stopping after the
first bad map. `.tmx.bak` remains excluded and the sample maps are explicit
authoring fixtures rather than campaign reachability evidence.

Production code must not maintain a validator-only parser or a runtime-only
path resolver. Frontends may differ in I/O mechanics, but profile and domain
validation are shared.

## Renderer and gameplay ownership

The adapter returns owned, renderer-independent data. It must preserve:

- map and layer dimensions, source layer order, IDs, names, and visibility;
- raw stripped tile IDs plus independent horizontal, vertical, and diagonal
  flags;
- tileset ranges, atlas metadata, image dependency identities, tile metadata,
  and animations;
- object-group and object order, object IDs/names/bounds, and typed properties;
  and
- scenario-relative provenance for each external dependency.

Bevy game systems then decide representation. They render eligible display
layers in source order, never render `collision`, build occupancy from non-zero
collision GIDs, interpret spawn and boss markers, create portals from typed
properties, and place Y-sorted world entities relative to tile layers. The
parser does not spawn ECS entities or select `bevy_ecs_tilemap` versus ordinary
Bevy sprites. M4 may evaluate `bevy_ecs_tilemap` as a renderer without changing
this parsing decision, provided game code retains the ownership above.

## Validation and diagnostics

Validation has three ordered stages:

1. **XML/profile validation:** syntax, allowed elements/attributes, encodings,
   geometry forms, property types, and reference spellings.
2. **Tiled semantic parse:** `tiled` resolves the prefetched XML graph and
   exposes typed Tiled values.
3. **Game-domain validation:** map/layer dimensions, atlas arithmetic,
   first-GID ordering/ranges, stripped GIDs, reserved names and properties,
   animation ranges/durations, path containment, and required assets.

A diagnostic includes the scenario-relative owning file, XML element and
element ID/name where available, property or cell coordinate where relevant,
external target for reference failures, and the parser's line/column when it
provides one. It never exposes a developer-machine absolute path. The adapter
adds domain context around a lower-level `tiled` error instead of returning an
opaque parser string.

The command-line validator aggregates independent errors. Runtime loading may
stop at the failing map boundary, but map/session activation is transactional:
parse and validate into staging data, wait for required image dependencies,
then switch the live world. A failure leaves the prior menu or live map intact
and emits the same structured diagnostic category as the validator.

## Cache and reload policy

- Create a fresh `tiled::Loader` for each root TMX or standalone TSX parse.
  Its internal external-tileset cache is useful only during that parse and is
  then dropped, so a changed TSX cannot survive in a long-lived parser cache.
- Let Bevy's `AssetServer` own persistent runtime asset caching. Do not add an
  on-disk parsed-map cache or generated map artifact.
- Track external TSX bytes as loader dependencies and images as normal Bevy
  dependencies. When development file watching is enabled, a TSX edit reparses
  its owning map and an image edit reloads through Bevy.
- Hot reload is a development convenience, not part of the compatibility
  promise. Edit-and-relaunch must work without recompiling even when the
  watcher is disabled.
- On explicit scenario reload or scenario change, discard game-owned parsed
  map/tileset indexes and request assets again through the selected scenario
  root. Never reuse an entry solely because two scenarios contain the same
  relative filename.
- Keep the current fully activated map until a replacement and all required
  dependencies pass. A failed hot reload reports an error but cannot publish
  partial replacement data.

## Alternatives considered

### `bevy_ecs_tiled`

Version 0.13.1 is the current Bevy-specific integration reviewed. Its published
compatibility matrix targets Bevy 0.19 and `bevy_ecs_tilemap` 0.19, and it
offers TMX loading, entity hierarchies, rendering, properties, and hot reload.
It is not selected because its parser-to-ECS/render/reload lifecycle overlaps
the project's game-owned projection, transactional activation, custom
layer/entity ordering, strict path boundary, and shared headless validator.
Using it and then overriding those responsibilities would add a second asset
lifecycle rather than remove work.

This is not a quality judgment on the crate. It is an ownership mismatch for
this port. Its documented patterns may still inform tests or renderer
experiments.

### `bevy_ecs_tilemap`

Version 0.19 is a Bevy tile renderer, not a TMX/TSX compatibility boundary. It
can be evaluated during M4.10-M4.12 after parsing produces project-owned data.
Selecting or rejecting it later does not supersede this ADR.

### Legacy Bevy integrations

`bevy_tiled` documents a Bevy 0.5-era API, and `bevy_tmx` 0.2.0 exposes the old
`App::build`/plugin API and incomplete format coverage. Neither is compatible
with the repository's Bevy 0.19 baseline, so neither merits a compatibility
shim.

### Custom semantic XML parser

A parser built directly on `quick-xml` would give total control over errors and
asynchronous resource acquisition, but it would also make this project own
Tiled's GID flags, atlas rules, properties, animations, object forms, external
reference behavior, and future parser security fixes. The zero-failure corpus
check demonstrates that `tiled` already handles the pinned semantics. A narrow
profile/reference scanner plus a mature semantic parser has less duplicate
format logic while retaining the strict compatibility boundary.

### Pre-convert TMX/TSX

Conversion to a Bevy-native or generated Rust format is rejected by ADR 0002.
It would also break the same-file editor and edit/relaunch contracts.

## Risks and required implementation spikes

M4 must close these risks before relying on the loader beyond a narrow slice:

1. **Synchronous parser boundary:** prove that a Bevy `AssetLoader` can prefetch
   one real map's TSX bytes asynchronously, then parse only the in-memory graph
   through `ResourceReader` without hidden filesystem access.
2. **Strictness gap:** seed one unsupported element, attribute, layer encoding,
   object form, and escaping path and prove the profile pass rejects each even
   when `tiled` would parse it.
3. **Diagnostics:** prove a malformed CSV cell, bad property type, missing TSX,
   missing PNG, and out-of-range GID report stable scenario-relative owner and
   target context.
4. **Flip transforms:** render or mathematically verify all eight orthogonal
   H/V/D combinations, including the two combinations present in the corpus,
   against Tiled reference behavior.
5. **Dependency reload:** with Bevy file watching enabled, change an external
   TSX and image and verify invalidation; introduce a bad edit and verify the
   last good live map stays active.
6. **Memory/latency:** measure prefetched XML/image-reference work on the
   largest map. If reading image bytes solely for existence tracking is too
   expensive, retain Bevy image dependencies and move decode/existence gating
   to the transactional activation step without bypassing shared validation.
7. **Crate upgrade discipline:** a future `tiled` upgrade requires the complete
   corpus check, strict negative fixtures, `cargo tree`/license review, and a
   dedicated compatibility diff. Do not use a floating minor version for the
   first implementation.

## Required verification

M4 implementation is not complete until automated tests cover:

- all supported header, external/inline atlas, CSV, object, property,
  animation, and GID/flip cases assigned to M4.01-M4.09;
- exact parsing of Aric's animation and representative map-attached atlases;
- synthetic float and boolean properties, since the pinned corpus does not
  exercise them;
- every unsupported category above with focused negative fixtures;
- normalized contained paths plus absolute, lexical-escape, symlink-escape,
  wrong-case, and missing-reference failures;
- a headless load of all 47 TMX and all 283 TSX files from the pinned snapshot;
- map activation failure that leaves prior state intact; and
- rendering/collision behavior tests assigned to M4.10-M4.14, including source
  display-layer order, invisible collision, and H/V/D transforms.

M13.10 later extends corpus parse coverage into a several-frame headless map
sweep. It does not replace the M4 parser/profile tests.

## Consequences for later milestones

- **M0.08:** chooses the canonical scenario root and packaging layout. It must
  provide a root that the opaque path boundary can enforce; this ADR does not
  preselect that directory.
- **M2:** YAML remains independent, but YAML and Tiled loaders must share
  scenario-relative path identities, structured diagnostics, strict unknown
  field/construct policy, and transactional scenario publication.
- **M4:** M4.01-M4.09 implement the adapter in the existing small task slices.
  M4.10-M4.14 consume project-owned output; parser work must not be hidden in a
  renderer plugin.
- **M12:** copied TMX/TSX/PNG files keep their internal relative layout. The
  migration and license ledger must cover every shipped dependency, while the
  parser sweep distinguishes 45 campaign maps, 2 samples, and backups.
- **M13:** the map-load sweep uses the same adapter; editor decisions preserve
  this profile; author instructions document edit, validate, run-map, and
  edit/relaunch without conversion.
- **M14:** parity and packaging run without `../agentic-rpg`, Python, or a
  working-directory assumption. The package contains canonical authoring
  files, and unused-source removal must be backed by reference and map sweeps.

## References

Primary sources reviewed on 2026-08-07:

- [`tiled` 0.16.0 crate documentation](https://docs.rs/crate/tiled/0.16.0)
- [`tiled::Loader`](https://docs.rs/tiled/0.16.0/tiled/struct.Loader.html)
  and [`ResourceReader`](https://docs.rs/tiled/0.16.0/tiled/trait.ResourceReader.html)
- [Bevy 0.19 `AssetLoader`](https://docs.rs/bevy/0.19.0/bevy/asset/trait.AssetLoader.html)
  and [`LoadContext`](https://docs.rs/bevy/0.19.0/bevy/asset/struct.LoadContext.html)
- [`bevy_ecs_tiled` 0.13.1](https://docs.rs/crate/bevy_ecs_tiled/0.13.1)
  and its [getting-started guide](https://adrien-bon.github.io/bevy_ecs_tiled/guides/getting-started.html)
- [`bevy_ecs_tilemap` 0.19.0](https://docs.rs/crate/bevy_ecs_tilemap/0.19.0)
- [legacy `bevy_tiled`](https://github.com/StarArawn/bevy_tiled) and
  [legacy `bevy_tmx` 0.2.0](https://docs.rs/bevy_tmx/0.2.0/bevy_tmx/)
