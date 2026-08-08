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
intentionally fails it with 37 errors and one warning rather than weakening
strict validation:

- one missing manifest cursor asset, covered by ADR 0005's one-entry migration
  repair;
- five consumed-but-undefined flags:
  `story_ultimate_earth`, `story_ultimate_fire`, `story_ultimate_water`,
  `story_ultimate_wind`, and `transport_warp_unlocked`;
- 28 enemy-drop references to seven absent item ids:
  `fire_dragon_horn`, `goblin_ear`, `goblin_fang`, `goblin_shield`,
  `rusty_blade`, `stone_dragon_horn`, and `void_core`;
- missing BGM id `zone.open_plains`;
- missing map id `dungeon_ruinwatch` and its scoped NPC `jep`; and
- one stable warning for
  `data/maps/zone_05_mountain_foothills.yaml`, which has no same-stem TMX.

These findings are compatibility decisions for Gate 2, not successful runtime
substitutions. Migrated content must repair or explicitly retain each one in a
reviewable later content task.
