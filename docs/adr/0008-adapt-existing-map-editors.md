# ADR 0008: Adapt the existing map editors with a target-repository launcher

- Status: Accepted
- Date: 2026-08-27
- Decision owner: M13.13
- Editor snapshot: `../agentic-rpg` at
  `08970359d6cb03586948625d29b0d3351dbbf785`

## Context

The pinned Python source contains two map-editor frontends: a Pygame graph
editor and a React/FastAPI web graph editor. They share the same Python
scenario loader, portal graph, and TMX mutation service. Both accept a
`--scenario` directory containing `manifest.yaml`; neither needs the Python
game runtime to launch the Rust game.

The target repository keeps the canonical scenario at
`assets/scenarios/rusted_kingdoms`. A read-only compatibility probe against
that directory at the pinned editor commit loaded all 47 TMX files and all 92
portal objects. The editor's writes are limited to portal object geometry and
the typed properties `target_map`, `target_position_x`, and
`target_position_y`; it creates a sibling `.tmx.bak` before the first write.

The editors are portal-graph tools, not general tile painters. Tiled remains
the supported tool for painting tile layers, changing tilesets, and editing
other object layers.

## Decision

Reuse both existing frontends and adapt their invocation at the target
repository boundary. `scripts/map-editor.sh` locates the pinned sibling
checkout, supplies the target scenario root, checks or installs declared
developer-only prerequisites, and launches either frontend. The editor source
is not copied into this repository and no new in-Bevy editor is introduced.

The default checkout is `../agentic-rpg`; `RPG_S1_EDITOR_REPO` can select a
different location. The launcher requires the pinned commit unless
`RPG_S1_EDITOR_ALLOW_UNPINNED=1` explicitly opts into testing another editor
revision. `RPG_S1_SCENARIO_PACKAGE` selects another in-repository package and
`RPG_S1_SCENARIO_ROOT` can select a writable scenario copy for editor-only
work.

Python, Pygame, FastAPI, and Node are development-tool dependencies only. The
Rust game, validator, replay, tests, and release package do not invoke this
launcher or load Python code.

## File-format constraints

Editor output must remain within the direct-authoring contract in ADR 0002 and
the supported Tiled subset in ADR 0003:

- canonical inputs remain YAML plus finite orthogonal TMX and external TSX;
- TMX identity remains the filename stem and optional map YAML remains
  same-stem beneath the manifest's `refs.maps` directory;
- TMX-to-TSX and TSX-to-image references stay containing-file-relative and
  within the selected scenario package;
- gameplay layer and object-group names remain exact, including `ground`,
  `collision`, `portals`, `spawn_tile`, and `boss_enemy` where used;
- portal destination coordinates remain Tiled integer properties while
  `target_map` remains a string property;
- tile GID flip bits, layer visibility, object geometry, external references,
  and editor metadata must survive a supported edit; and
- `.tmx.bak`, `.cache/map_editor`, thumbnails, and frontend build output are
  derived tooling artifacts, never runtime inputs or release content.

Every saved edit must be reviewed with `git diff`, then pass
`validate-scenario` and the map-load sweep before it is accepted. Dialogue or
encounter edits additionally run their dedicated sweep. A map must then load
through the debug start-map command, and behavior-sensitive edits should be
captured in a deterministic record and replayed.

## Consequences

The existing editor UX remains available without adding editor code to the
Rust engine, and both frontends write the exact files the Rust runtime reads.
The pinned editor checkout remains an explicit developer prerequisite. Its
current loader assumes the conventional `assets/maps` directory for TMX files
instead of consuming `refs.tmx`; therefore this decision targets Rusted
Kingdoms and other packages using that conventional layout. Supporting a
nonstandard TMX directory in the editor requires a separately reviewed editor
change, while the Rust runtime itself remains manifest-driven.

Replacing these tools with a new frontend remains possible later, but that
frontend must preserve the same canonical formats and validation loop.

## Rejected alternatives

### Build a new in-Bevy editor

Rejected for this milestone because it duplicates two working frontends and
mixes developer tooling into the game runtime.

### Vendor the Python and web editor sources here

Rejected because it creates a second copy with unclear ownership and update
rules. The pinned checkout plus launcher makes the dependency reproducible.

### Convert edited maps before Rust can load them

Rejected by ADR 0002. A successful editor save must be sufficient input for
the validator and the same game binary.
