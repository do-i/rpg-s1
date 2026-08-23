# ADR 0007: Triage inherited scenario data debt

- Status: Accepted
- Date: 2026-08-22
- Decision owner: Parity plan P0.5
- Source snapshot: `../agentic-rpg` at
  `08970359d6cb03586948625d29b0d3351dbbf785`

## Context

After the W12.1 zone-1 drop repair (`data/items/migration_zone1_drops.yaml`)
and the W12.2 BGM registration recorded in the M12 ledger, strict target
validation reports 13 errors and 1 warning. Every remaining diagnostic was
verified byte-for-byte present in the pinned Python source as well: none is a
port regression. The pinned Python engine plays around all of them, in ways
confirmed by reading the engine code, so each needs an explicit decision —
match the source behavior, or deliberately exceed it — rather than an
open-ended bug hunt.

## Findings and decisions

### Sorcerer "ultimate" ability flags (4 errors) — keep source behavior

`data/classes/sorcerer.yaml` gates four abilities on
`story_ultimate_{fire,water,wind,earth}`, and nothing in either repository
produces those flags. Under the pinned engine (`engine/spell/spell_logic.py`),
an ability whose `unlock_flag` is absent from the save's flag set simply never
unlocks, so these four abilities are unobtainable in the original. Parity
means they stay unobtainable. Inventing unlock points would be new game
design, out of scope for the port. Accepted as source debt.

### Endgame drop items `void_core`, `fire_dragon_horn`, `stone_dragon_horn` (5 errors) — defer to W12.7/W12.8

Rank-SS and rank-S enemy drop pools reference three item ids that no pinned
item catalog defines — the same shape as the four zone-1 drops repaired in
W12.1. Follow the `migration_zone1_drops.yaml` precedent: author minimal
target-side material definitions in the wave that makes those enemies
fightable (W12.7/W12.8), keeping source drop probabilities and adding no
inferred equipment or use behavior. Deferred, with the repair pattern already
proven.

### `transport_warp_unlocked` (1 error) — keep bytes; parity work lives elsewhere

`data/maps/zone_01_starting_forest.yaml` declares a `transport:` block whose
`warp.unlock_flag` nothing produces. No code in the pinned Python engine reads
the map-level `transport:` block at all — it is aspirational data. The
original's fast travel is implemented by `engine/world/warp_logic.py`
(Teleport spell over visited top-level maps, landing tiles derived from
incoming portals) plus the Warp Stone item. The source bytes stay; the actual
world-map-travel parity obligation is the `warp_logic.py` semantics check
tracked as parity-plan task P1.1. Producing the flag, or wiring a map-level
transport system, would be inventing behavior the original does not have.

### Jep join map `dungeon_ruinwatch` (2 errors) — keep bytes; recruitment already has a real home

`data/party.yaml#party[3].join.map` names a map that exists in neither
repository. Reading the pinned engine shows `join`/`recruit` blocks in
`party.yaml` are never consulted: recruitment happens when a dialogue emits a
`join_party` action (`engine/world/world_map_logic.py:apply_join_party`), and
Jep is authored exactly that way — an NPC on `town_03_ruinwatch_monastery_vaults`
at `[5, 9]` with dialogue `jep_join` and presence gated on
`excludes: [npc_jep_joined]`. Jep is recruitable in the original through that
path, and will be in the port once W12.4 verifies Ruinwatch. The dangling
`dungeon_ruinwatch` metadata stays as accounted source bytes; W12.4's
acceptance must prove the monastery-vaults recruitment live, not a
`dungeon_ruinwatch` map.

### Title cursor icon (1 error) — already decided

`manifest.yaml#title.cursor_icon` names a missing file. ADR 0005 already
records the repair decision; this ADR only notes the validator line remains
attributable to that accepted entry.

### `zone_05_mountain_foothills` same-stem warning (1 warning) — naming convention, plus a latent source crash

The base YAML is parent metadata for a three-segment map whose TMX files are
`zone_05_mountain_foothills_01..03.tmx` (present in both repositories); the
parent YAML legitimately has no same-stem TMX, and parity-plan task P0.1
taught the per-map report this convention.

Correction recorded 2026-08-22 after P1.1 ran the pinned `warp_logic.py`
directly against the scenario: `_is_submap` builds its id set from
`assets/maps/*.tmx` stems only, and no bare `zone_05_mountain_foothills.tmx`
exists — so the three segments are NOT submaps in the pinned engine; each is
an independent top-level warp destination. Because
`zone_05_mountain_foothills_01.yaml` exists in neither repository, a party
that has visited segment 1 makes the pinned engine's warp overlay raise
`ValueError` (missing required `warp_order`) — a latent crash in the
original. The port instead surfaces a descriptive catalog failure (P1.1's
fix in `field_menu_domain.rs`). W12.5 acceptance must decide: author a
target-side `zone_05_mountain_foothills_01.yaml` with `warp_order` (exceeding
the source, per the migration-repair precedent) or exclude segment 1 from
warp with a recorded difference.

### Dead trailing dialogue entries in six more dialogues — keep source bytes

The `dialogue-report` sweep (parity plan P0.3, 2026-08-22) found the
`ardel_fisherman` pattern — a flag partition that starves trailing
flavor/fallback entries under the engine's first-match rule — in six more
pinned dialogues: `ashenveil_ashgatherer` [5], `elder_intro` [2],
`frostholm_courtier` [4, 5], `harborgate_fishwife` [4], `millhaven_carter`
[4, 5], `ruinwatch_digger` [4]. All are inherited source content (the source
files carry author comments hinting at "flavor"/"safety fallback" intent),
unreachable in both engines. Decision: keep the source bytes, same as the
fisherman precedent. The `dialogue-report` module pins the exact inventory in
a regression test; as each wave's `C-DIALOGUE` acceptance verifies one of
these dialogues, move it to the documented-accepted classification with an
exhaustive flag-state regression like `ardel_fisherman`'s.

### Missing sign dialogues on Marshland and Mountain Pass segment 1 — keep bytes; verify silence at wave acceptance

The `map-sweep` (parity plan P0.2, 2026-08-22) found painted sign tiles whose
`sign_<map_id>` dialogue does not exist in either repository:
`zone_03_marshland` (2 signs, no `sign_zone_03_marshland.yaml` anywhere) and
`zone_06_mountain_pass_01` (1 sign; only the unsuffixed
`sign_zone_06_mountain_pass.yaml` plus `_02`/`_03` exist). Under the pinned
engine a missing sign dialogue is a silent no-op
(`DialogueEngine.resolve` → `load_yaml_optional_cached` → `None`), so these
signs do nothing in the original. Decision: keep the data as-is. One behavior
delta to settle in code at W12.3/W12.6 acceptance: the port currently plays
the dialogue-open interaction sound before the document load resolves
(`world_interaction.rs`), so a missing sign dialogue clicks and then shows
nothing, where the original stays fully silent — either match the silence or
record the click as an accepted difference when those waves close.

## Consequences

- Strict target validation is expected to hold at 13 errors / 1 warning until
  W12.7/W12.8 author the endgame drop materials; the count is not a parity
  score and must not be driven to zero by inventing unlocks, maps, or flags.
- Wave acceptance evidence (ledger `C-PLAY` rows) remains the parity metric.
- Any future decision to exceed source behavior (e.g., actually granting the
  Sorcerer ultimates) is a new ADR, not a validator cleanup.
