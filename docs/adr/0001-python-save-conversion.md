# ADR 0001: Convert Python YAML saves once

- Status: Accepted
- Date: 2026-08-07
- Decision owner: M0.05
- Source snapshot: `../agentic-rpg` at
  `08970359d6cb03586948625d29b0d3351dbbf785`

## Implementation status

Implemented in M7 on 2026-08-12. `rpg-s1 import-python-save` now performs the
one-way conversion described below, and normal runtime enumeration/load reads
only native versioned slots. The checked-in source-produced input and
converted-native golden live under `tests/fixtures/python-save-0897035/`.
Gate 7 and RK-SAV-006 passed through separate live game processes; detailed
evidence is in `docs/m7-manual-play-checklist.md`.

## Context

Players may have progress saved by the pinned Python/Pygame game. The Rust
port needs a deliberate compatibility boundary before its native save format
is designed in M7.01.

The pinned source establishes these facts:

- `engine/io/save_manager.py` uses 101 slot files, `000.yaml` through
  `100.yaml`. Slot 0 is the autosave. It writes an unversioned YAML mapping
  directly to the destination file.
- A current save has no format version, scenario id, or scenario version. The
  scenario manifest separately identifies the campaign as `my_rpg_story`
  version `1.0.0`.
- The optional `checksum` is CRC32 over a PyYAML re-serialization of the parsed
  mapping after removing `checksum`; it is not a checksum of the original file
  bytes. Current saves contain it, but `_load_save_payload` also accepts files
  without it.
- `_serialize` persists party order and member state, the controlled member,
  repository GP/items, flags, current map/position/visited maps, opened boxes,
  and playtime metadata. It does not persist RNG state. Facing is transient.
  Abilities and other class-derived data are rebuilt from scenario class YAML,
  quests are represented by flags, and status effects are intentionally not
  saved.
- `engine/io/game_state_loader.py` also accepts several older omissions. It
  derives a missing `exp_next`, defaults a missing member row from class data,
  defaults missing opened boxes to empty, defaults missing item loot fields,
  and selects the protagonist or first party member when
  `controlled_member_id` is missing.
- Loading depends on external scenario data, including class definitions and
  item metadata. A YAML document alone cannot prove which scenario revision
  produced its ids.
- `tools/migrate_saves.py` is already a one-time filename/checksum migration
  for an older Python layout. It does not add schema or scenario versions and,
  without `--archive`, deletes the old files after conversion.
- The pinned repository has unit tests that create temporary saves, but no
  tracked, serializer-produced save fixture.

These properties make the Python document useful as an import source, but a
poor permanent runtime contract. In particular, the PyYAML-specific checksum,
unversioned schema, missing scenario identity, and missing Rust-required state
would force every normal load to carry legacy detection and defaulting.

## Decision

The game runtime will **not** read Python YAML saves directly.

M7.15 will provide a separate, one-way Rust converter that imports one
explicitly selected Python YAML file into one native Rust save slot. After a
successful conversion, the normal game sees and loads only the native save.
The converter will not run during game startup, slot enumeration, or normal
load.

The converter is part of the Rust project and release package. It must not
invoke Python, import Python engine code, require PyYAML, or require the Python
source checkout. Its compatibility adapter is identified by the full pinned
source commit above. This preserves a self-contained Rust runtime and a
self-contained migration path.

This is a one-way conversion. The Rust project will not write Python-format
saves and makes no promise that converted progress can be reopened by the
Python game.

## Supported input contract

The converter supports the mapping accepted by the pinned
`_load_save_payload` and `from_save` functions, limited to values that can be
validated against the ported `my_rpg_story` version `1.0.0` content.

The input filename is not trusted as schema, scenario, slot, or integrity
evidence. The player selects the destination native slot explicitly. This
allows a current numbered file or an older same-schema file to be selected
without reproducing Python's historical filename rules.

For compatibility and safety:

- exactly one YAML document with a mapping root is accepted;
- custom YAML tags and values outside the supported scalar/list/mapping model
  are rejected;
- all current serializer-required fields and their types/ranges are checked;
- class, member, map, item, and equipment ids are checked against the selected
  target scenario before any output is written;
- arbitrary string flags are preserved because the Python flag registry is
  intentionally open-ended;
- duplicate or malformed state that a current serializer cannot produce is
  rejected instead of being silently normalized;
- an embedded checksum must match the pinned PyYAML checksum algorithm; and
- a missing checksum is accepted only through an explicit
  `--allow-unchecked`-style opt-in, with an unambiguous warning recorded in the
  result and import provenance.

Implementing the checksum verifier requires matching the pinned serializer's
mapping order and PyYAML scalar formatting for this closed schema. A fixture
produced by the pinned serializer, not a hand-written approximation, is the
acceptance oracle. A checksum mismatch is never downgraded to an unchecked
import.

This ADR does not promise support for a Python save from any later source
revision. Supporting another revision requires a separately identified input
adapter, source-produced fixture, and reviewed mapping. It must not silently
broaden the adapter attached to this snapshot.

## State mapping and guarantees

On success, the converter creates a complete native save envelope and records
import provenance containing:

- source kind (`python-yaml`);
- the adapter's pinned source commit;
- a SHA-256 digest of the original input bytes;
- the original timestamp string, if present;
- the original location display string, if present; and
- whether the embedded Python checksum was verified or explicitly absent.

The input path is not stored because it may expose a machine-specific absolute
path. The native envelope's scenario id/version come from the validated target
scenario, not from the Python file, which has no such fields.

The converter guarantees preservation of the following Python state:

| Python state | Native result |
| --- | --- |
| Party list/order and member identity/name/class | Preserved in order after content-reference validation. |
| Level, EXP, current/max HP/MP, and STR/DEX/CON/INT | Preserved exactly when valid. Missing `exp_next` is derived by the pinned rule; otherwise it is preserved. |
| Equipment and row | Equipment is preserved after validation. A missing row uses the member's scenario class default, matching the pinned loader. |
| Controlled member | Preserved when present and valid; otherwise protagonist, then first member, matching the pinned loader. |
| Repository GP and item stacks | GP, quantities, tags, lock state, `is_loot`, and `loot_batch` are preserved. Pinned defaults apply only when an older optional field is absent; the loader-derived `magic_core` tag is restored for `mc_*` items. |
| Flags and quests | Every flag is preserved. Quest state remains derived from flags; no nonexistent Python quest payload is invented. |
| Map and position | Current map, tile position, and visited maps are preserved and validated. |
| Facing | Set to Down, matching `MapState.from_dict` in the pinned loader, because Python saves do not contain facing. |
| Opened boxes | Preserved. Absence means empty, matching the pinned loader; malformed entries are rejected rather than dropped. |
| Playtime | `playtime_seconds` is preserved exactly. The naive source timestamp and location display text are retained as provenance/display hints, not trusted as world state. |
| Abilities and class-derived state | Recomputed from the validated target scenario, matching the pinned loader. |
| Status effects | Empty, matching the documented Python save rule. |
| RNG | Initialized deterministically from a domain-separated hash of the normalized imported state and adapter id. No cross-engine next-random-value parity is promised because Python did not persist RNG state. |

The conversion guarantee is therefore state parity for information the pinned
Python format actually persists, plus documented deterministic defaults for
information it omits. It does not claim bit-for-bit Python runtime continuation
or preservation of transient scene, audio, input, battle, or menu state.

## Errors, writes, and backups

Conversion is transactional:

1. Read and validate the complete input without changing it.
2. Build a native state in memory and serialize it through the M7 native save
   writer.
3. Decode the produced bytes into a fresh native state and compare the promised
   fields before installation.
4. Install the result using the temporary-file and atomic-replace behavior from
   M7.05-M7.06.

The converter never edits, renames, archives, or deletes the Python input. An
existing native destination is refused by default. An explicit replace option
must first preserve the exact old destination bytes in an import-backup area
under the M7.04 save root; the backup name includes the destination slot and a
content digest. Backup creation and verification must succeed before the
destination can change. Reusing an existing identical digest backup is safe;
a conflicting backup is an error.

Unreadable YAML, excessive input size/nesting, unsupported YAML features,
checksum failure, missing/wrong fields, invalid bounds, duplicate serialized
identities, unavailable content references, scenario mismatch, native-version
incompatibility, destination conflict, backup failure, and write/verification
failure all produce an actionable error. No such error may create a loadable
partial slot or alter an existing slot. Slot enumeration must continue to work
for all other native saves.

The pinned `tools/migrate_saves.py` is evidence about historical inputs, not an
implementation dependency. Its delete-by-default behavior is specifically not
copied.

## Consequences

Benefits:

- normal loading has one versioned native schema and one validation path;
- the game and importer remain independent of Python and the source checkout;
- legacy parsing risk is isolated from startup and ordinary save discovery;
- conversion can validate all references before state becomes playable; and
- the original input and any replaced native slot remain recoverable.

Costs:

- players must run and confirm a separate import step once per desired save;
- the Rust project must maintain a small pinned compatibility adapter and its
  fixture;
- the PyYAML checksum needs a deliberately narrow compatibility implementation;
  and
- saves that reference content not yet ported receive an actionable
  incompatibility result until that content exists.

## Rejected alternatives

### Read Python YAML in the normal runtime

Rejected because it permanently couples every slot scan and load to an
unversioned, scenario-implicit schema. It also makes PyYAML checksum emulation,
legacy defaults, and incomplete-state synthesis part of the trusted runtime
path. Format sniffing cannot reliably establish scenario identity that the
file does not contain.

### Provide no migration path

Rejected because persisted progress is player-visible behavior and RK-SAV-006
already requires a real compatibility result. Explicit conversion gives that
path without weakening native saves.

### Reuse the Python migration script or build a Python-dependent converter

Rejected because `tools/migrate_saves.py` only changes Python filenames and
checksums, does not produce a versioned native envelope, and can delete source
files. A Python-dependent release tool would also undermine the self-contained
port and clean-profile release checks.

## Validation strategy

M7.15 must add a checked-in fixture produced by the pinned
`GameStateManager.save`, using fixed time and deterministic state construction.
The fixture record must include its generation command, the full source commit,
and its expected field mapping. No representative save fixture is currently
tracked in the source repository.

Automated coverage must include:

- successful verified-checksum conversion and native load of the real fixture;
- field-by-field comparison for every guarantee in the mapping table;
- a golden native output with an injected conversion clock;
- deterministic RNG initialization from the normalized imported state;
- the pinned optional-field defaults;
- explicit opt-in for a checksumless fixture;
- rejection of checksum mismatch, truncation, wrong types, invalid ranges,
  duplicate identities, and unavailable content references;
- refusal to overwrite by default;
- backup plus atomic replacement when explicitly requested;
- unchanged source and destination bytes after every injected failure; and
- a native round trip after conversion.

RK-SAV-006 then exercises the documented converter command and launches the
resulting native slot. A clean packaged test must run the converter without a
Python interpreter or either source checkout.

## Effect on later plan tasks

- **M7.01:** define one native envelope with format version, scenario
  id/version, timestamp, payload, and optional import provenance.
- **M7.02-M7.03:** keep native serialization independent of Python field names;
  native golden fixtures remain separate from the import fixture.
- **M7.04:** choose locations for native slots and import backups. Do not scan
  the Python save directory automatically.
- **M7.05-M7.06:** expose the same verified temporary-write and atomic-replace
  path to the converter.
- **M7.07-M7.11:** enumerate and present native slots only. Python YAML does not
  enable Load Game and no import UI is required.
- **M7.12:** native saves still restore all native state, including RNG. The
  narrower imported-state guarantees above apply only at the conversion
  boundary.
- **M7.13:** native unknown-field tolerance does not broaden the pinned Python
  adapter.
- **M7.14:** native old-version routing occurs after conversion. Adapter
  versions and native save versions are independent.
- **M7.15:** implement the standalone Rust converter, fixture, provenance,
  validation, backup, atomic-write, and focused tests specified here.
- **M7.16:** cover both corrupt native-slot isolation and proof that a failed
  import cannot damage another slot.
- **M14.01:** RK-SAV-006 must pass or show an actionable incompatibility without
  partial import.
- **M14.02-M14.04:** clean playthrough, native save soak, and native replay
  determinism remain based on native saves. Imported RNG starts from the
  deterministic boundary documented above, not a Python RNG continuation.
- **M14.08-M14.09:** ship and exercise the Rust converter in the self-contained
  release package, including a clean-profile import smoke test.
- **M14.10:** document the one-way command, guarantees, opt-in for unchecked
  input, errors, destination conflict, and backup recovery.
- **M14.11:** archive the adapter id, fixture provenance, source hash, and final
  RK-SAV-006 result with the parity report.
- **M14.12:** the game and converter must demonstrate that no Python runtime,
  package, engine code, or source checkout is loaded at execution time.
