# Rusted Kingdoms validator parity contract

This contract compares the Rust validator with the Python
`tools/validate.py` oracle pinned at
`08970359d6cb03586948625d29b0d3351dbbf785`. It records observable acceptance
and rejection behavior; it does not make Python's permissive dictionary reads
the Rust schema policy.

The durable cases are compact, invented, source-shaped scenarios created under
the system temporary directory. No campaign content is copied into the target
repository. Ordinary `cargo test` runs every case against Rust without needing
Python or the sibling source checkout. The ignored oracle test runs the same
case table through the pinned source virtual environment and validator:

```bash
RPG_S1_PINNED_SOURCE_DIR=../agentic-rpg \
  cargo test scenario_cross_reference::tests::compares_invented_parity_cases_with_the_pinned_python_validator -- --ignored --exact
```

The oracle test requires the exact pinned commit and a clean source worktree,
uses `.venv/bin/python`, passes only a fresh temporary scenario root to
`tools/validate.py`, and confirms the source remains clean afterward.

## Fixture matrix

| Case | Surface | Python | Rust | Contract |
| --- | --- | --- | --- | --- |
| Complete compact scenario | All baseline catalogs | Pass | Pass | Shared accepted case |
| Missing `start.intro_dialogue` file | Manifest/path | Fail | Fail | Shared rejected case |
| Unknown map-shop item | Map/item | Fail | Fail | Shared rejected case |
| Unknown encounter background | Encounter | Fail | Fail | Shared rejected case |
| Unknown `join_party` character | Dialogue/character | Fail | Fail | Shared rejected case |
| Unknown recipe output item | Recipe/item | Fail | Fail | Shared rejected case |
| Undefined quest completion flag | Quest/flag | Fail | Fail | Shared rejected case |
| Missing party portrait | Asset | Fail | Fail | Shared rejected case |
| Missing manifest cursor | Manifest/asset | Pass | Fail | Deliberate Rust strictness |
| Missing indexed BGM file | Audio/asset | Pass | Fail | Deliberate Rust strictness |
| Unknown encounter formation enemy | Encounter/enemy | Pass | Fail | Deliberate Rust strictness |

Each shared failure also asserts Python's focused output text and the Rust
diagnostic code and field path. This prevents an unrelated parse failure from
masquerading as parity.

## Intentional differences

The Rust validator remains stricter where Python has no check:

- every typed manifest and audio path must resolve inside the scenario;
- every indexed audio asset must exist;
- every encounter formation, boss, and barrier enemy must resolve;
- typed catalogs reject unknown or unsupported shapes instead of accepting
  arbitrary dictionaries; and
- cross-catalog references and all modeled flag consumers are checked even
  when `tools/validate.py` does not traverse them.

TMX/TSX XML internals are not a validator disagreement yet. The Python
validator does not inspect portal destinations, layers, GIDs, or external
tileset/image links, and the Rust XML loader is assigned to M4. M2 validates
map identity from contained TMX filename stems plus same-stem YAML metadata;
portal-level parity fixtures must be added with the M4 parser.

## Pinned campaign result

The Python validator passes the unmodified pinned campaign. The Rust validator
intentionally fails it with 37 errors and zero warnings rather than weakening
strict validation:

- one missing manifest cursor asset, covered by ADR 0005's one-entry migration
  repair;
- five consumed-but-undefined flags:
  `story_ultimate_earth`, `story_ultimate_fire`, `story_ultimate_water`,
  `story_ultimate_wind`, and `transport_warp_unlocked`;
- 28 enemy-drop references to seven absent item ids:
  `fire_dragon_horn`, `goblin_ear`, `goblin_fang`, `goblin_shield`,
  `rusty_blade`, `stone_dragon_horn`, and `void_core`;
- missing BGM id `zone.open_plains`; and
- missing map id `dungeon_ruinwatch` and its scoped NPC `jep`.

These findings are compatibility decisions for Gate 2, not successful runtime
substitutions. Migrated content must repair or explicitly retain each one in a
reviewable later content task.

`data/maps/zone_05_mountain_foothills.yaml` has no same-stem TMX but no longer
produces a warning; see the multi-segment convention below for why.

## The multi-segment TMX convention

`zone_05_mountain_foothills` is metadata for a map split across several TMX
files rather than one: `assets/maps/zone_05_mountain_foothills_01.tmx`,
`_02.tmx`, and `_03.tmx` exist, but no
`assets/maps/zone_05_mountain_foothills.tmx` does. This mirrors the pinned
Python engine's general submap rule — `_is_submap` in
`engine/world/warp_logic.py`, which treats any map id that extends another
known id with an underscore as that map's interior/segment — narrowed here to
the purely numeric suffix pattern (`<id>_01`, `<id>_02`, ...) so a metadata
file naming its own segments is recognized specifically, rather than folding
in worded interiors like `..._shop_01`.

The Rust validator (`source.unmatched_map_metadata` in
`src/scenario_cross_reference.rs`) previously warned on every map YAML
without a same-stem TMX, including this one. It now checks whether numbered
segment TMX files name the metadata's stem as their parent (`is_numeric_tmx_segment`)
before warning; when they do, the metadata is parent content for a real,
playable multi-segment map, not orphaned metadata, so no warning is raised.
A map YAML with no same-stem TMX and no numbered segments is still warned on
exactly as before.

## `map-report`: per-map reachability report

```bash
cargo run -- map-report [PACKAGE_KEY]
```

Where `validate-scenario` checks each catalog against ADR 0002's typed
schemas and stops at same-stem TMX presence (TMX/TSX XML internals are M4's
job, per "Intentional differences" above), `map-report` reads the M4 TMX
portal layer (`src/tmx_header.rs`, `src/world_transition::runtime_portals`)
to describe what every map in the scenario actually connects to. For each
map id — every `data/maps/*.yaml` stem and every `assets/maps/*.tmx` stem,
unioned — it prints:

- whether a same-stem TMX exists, or (per the segment convention above) which
  numbered segment TMX files exist instead;
- its NPCs: id, position, effective dialogue id, and whether `present`
  gates it;
- its TMX `portals` object-layer targets: target map id and target position;
- every dialogue id its NPCs reference (`dialogue` and `excuses`); and
- findings: a missing TMX (with no segment family either), a portal target
  with neither a same-stem nor segment TMX, or a referenced dialogue id
  absent from `data/dialogue/`.

It closes with a summary: total map ids, how many carry no finding, and how
many do. Unlike `validate-scenario`, `map-report` is informational — it
always exits `0`; the summary line is how a caller notices findings. Running
it against the shipped `rusted_kingdoms` package reports 48 map ids, 47 with
no findings, and one (`sample_01`, a placeholder TMX with an inline tileset
outside the M4 external-tileset profile) with a TMX parse finding unrelated
to this task.

## `dialogue-report`: per-dialogue reachability and effects report

```bash
cargo run -- dialogue-report [PACKAGE_KEY]
```

`validate-scenario` checks that every `data/dialogue/*.yaml` document is
shaped correctly and that its item/party/map references resolve; it does not
ask whether every authored branch can ever be seen. `dialogue-report` walks
every dialogue graph and, for each `DialogueDocument::Entries` document,
exhaustively enumerates every assignment of the document's referenced flags
(capped at 16 flags; none in the shipped scenario come close — the worst case
is 5) to evaluate the pinned Python engine's first-match rule from
`engine/dialogue/dialogue_engine.py`: entries are tried top to bottom, an
entry matches when every `requires` flag is set and no `excludes` flag is
set, and the first match wins. An entry that never wins under any assignment
is dead. This mirrors `DialogueSession::resolve` (`src/world_dialogue.rs`),
which selects by the identical rule at runtime and only ever considers
`node`-less entries — `node` entries are graph targets reached by `next` or a
choice, not flag-gated branches, so the report lists them separately and
never marks them dead. For each entry it also prints its line count (a
reachable entry with zero lines is flagged) and its `on_complete` effects:
`set_flag`/`unset_flag` (tagging any flag that follows the pinned `unlock`
sugar's `<kind>_<id>_unlocked` naming convention for
recipe/spell/location/transport, informationally — the schema has no separate
`unlock` action because no pinned document uses one), `give_items`,
`join_party`, map `transition`, and `open_shop`/`open_inn`/`open_apothecary`.
An unresolved `give_items` id is noted informationally rather than as an
error, since `validate-scenario` already checks that at error level.

Like `map-report`, this command is informational and always exits `0`.
`docs/m12-content-migration-ledger.md` ("Pinned-source differences affecting
W12.1") documents that `ardel_fisherman` ends with two flavor entries that
are provably dead under this rule; the report marks exactly those two
`documented-accepted` rather than as findings. Running it against the shipped
package turns up the identical dead-trailing-entry shape, undocumented, in
six more dialogues — `ashenveil_ashgatherer`, `elder_intro`,
`frostholm_courtier`, `harborgate_fishwife`, `millhaven_carter`, and
`ruinwatch_digger` — each a sub-quest giver whose first four entries already
exhaustively partition the relevant `sq_*_started`/`_relayed`/`_done` states
(or, for `elder_intro`, a reward entry and its own source-commented "safety
fallback" duplicate), leaving a trailing flavor or story-flag entry
unreachable regardless of any other flag. That is a content finding, not a
report bug: the report does not alter pinned YAML, only describes it. Of the
91 dialogues: 83 are fully clean; 1 (`ardel_fisherman`) carries only the
documented-accepted dead entries; 6 carry a new dead-entry finding as just
described; and 1 (`sign_zone_05_mountain_foothills`, whose authored id names
a specific numbered segment) carries a purely informational id/filename-stem
note and nothing else. The summary footer reports these counts directly.

## `map-sweep`: headless production-pipeline sweep

```bash
cargo run -- map-sweep [PACKAGE_KEY]
```

`map-report` and `dialogue-report` describe what the authored source *says*.
`map-sweep` actually runs it: for every `assets/maps/*.tmx` file it drives the
same production loaders the running game uses, headlessly, with no window and
no GPU. Most of the pipeline is plain functions over already-read file bytes,
so this reuses them directly rather than spinning up a Bevy `App`:

- **TMX/TSX pipeline** — parses the TMX with `tmx_header::parse_tmx_map_document`
  (the exact function `TmxGroundAssetLoader` uses), then replays
  `tmx_ground_asset`'s ground-layer and visible-atlas selection
  (`unique_ground_layer_index`, `visible_layer_indices`,
  `visible_atlas_reference_indices` — made `pub(crate)` for this sweep to call
  directly instead of duplicating the rule), parses each visible external TSX
  with `tsx_metadata::parse_tsx_tileset_metadata`, and resolves every visible
  layer's tile GIDs through `TmxTilesetRanges`, checking the same
  atlas-tilecount boundary `TsxAtlasAsset::sprite_for_tile` enforces at
  render time. A parse or reference failure is reported per map; it never
  panics, and the sweep still visits every other map.
- **Collision** — `scenario_spatial::collision_occupancy::CollisionOccupancy::from_tmx_document`,
  the same call `WorldCollision` makes when entering a map.
- **NPCs** — loads the map's same-stem metadata YAML (when one exists) and
  calls `world_actor::present_npcs` — the exact production spawn-set filter,
  made `pub(crate)` for this sweep rather than duplicated — under the empty
  flag state and, for each distinct `requires`/`excludes` flag referenced by
  the map's NPCs, that one flag toggled on alone. Every NPC the production
  filter would spawn under any of those states has its position checked
  against the TMX's own dimensions and its dialogue (and `excuses`) id
  checked against `data/dialogue/`.
- **Portals** — `world_transition::runtime_portals` (already shared with
  `map-report`), checking each target map id resolves via the same same-stem
  or numbered-segment rule (`scenario_map_report::is_numeric_tmx_segment`).
- **Signs and item boxes** — `world_object::sign_tiles` (the production
  sign-tile scan) plus a structural walk of the map's `item_boxes`: position
  bounds, `runtime_opened_boxes::OpenedBoxKey` construction, and a non-empty
  loot check. No interaction is simulated.

Like the other two reports, `map-sweep` is informational and always exits
`0`. It prints each map's portal/sign/item-box/flag-state counts and any
findings, tagged by category (`tmx`, `collision`, `npc`, `portal`, `object`),
followed by a summary footer with per-category finding counts. Running it
against the shipped `rusted_kingdoms` package sweeps all 47 TMX files: 44 are
fully clean. `sample_01.tmx` is the same known inline-tileset placeholder
`map-report` already surfaces (one `tmx` finding). Two more are new content
findings, not report bugs — the sweep does not alter pinned YAML, only
actually runs the game's loaders against it: `zone_03_marshland` paints two
sign tiles but `data/dialogue/sign_zone_03_marshland.yaml` does not exist, and
`zone_06_mountain_pass_01` paints one sign tile but its dialogue id, formed
by the exact `sign_{map_id}` convention `world_object::drive_world_object_load`
uses, is `sign_zone_06_mountain_pass_01` — a document that does not exist;
only the unsuffixed `sign_zone_06_mountain_pass.yaml` (and the `_02`/`_03`
segment documents) do.
