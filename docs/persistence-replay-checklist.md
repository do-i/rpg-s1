# Persistence and replay operator checklist

RF-10 prepares the existing manual verification entries (`temp/plans/manual-verification.md`):
`RK-SVC-001`, `RK-SVC-017`, clean campaign, and `M14.04` / `RK-DBG-001`–`RK-DBG-005`.
These entries remain pending until the user performs or explicitly accepts the
visible observations. Headless tests and generated saves do not complete them.

## Prepare an isolated profile

Run from the repository root with the installed Rust toolchain. Build once, then
generate fresh native saves through the regression test. The export refuses an
existing destination. Use the same release-scoped binary throughout acceptance.

```bash
cargo build -j 1
rf10_run=$(realpath "$(mktemp -d temp/refactoring/rf10-operator.XXXXXX)")
RPG_S1_RF10_EXPORT="$rf10_run/pristine" cargo test -p rpg-engine -j 1 \
  world_interaction::persistence_regressions::authored_rewards_and_campaign_boundaries_survive_fresh_processes \
  -- --exact
mkdir "$rf10_run/saves" "$rf10_run/config"
cp "$rf10_run"/pristine/*.yaml "$rf10_run/saves/"
RPG_S1_SAVE_DIR="$rf10_run/saves" XDG_CONFIG_HOME="$rf10_run/config" \
  target/debug/rpg-s1 play rusted_kingdoms
```

Keep `rf10_run` in the same shell. If the normal Cargo target is unavailable, set
`CARGO_TARGET_DIR` to a writable build directory for all Cargo commands and use
that directory's binary. All fixture saves and run evidence belong under ignored
`temp/refactoring/`; the profile above isolates both saves and options.

| Slot | Generated state | Purpose |
| ---: | --- | --- |
| 1 | Act II, facing Pike in Starting Forest, no smith completion | Disabled choice, cancel, pledge, turn back |
| 2 | Same location, `sq_smith_done` set | Enabled vouch branch; +3 Potion, +2 Antidote |
| 3 | Forest boss defeated, before Elder Maeve's reward, in her house | +2 Hi-Potion, +1 Tent; Act II advancement |
| 4 | Hesta's Flame-glyph quest relayed, before reward, in Ruinwatch | +2 Ether; quest completion |
| 21–24 | Completed pledge, vouch, elder, and archivist respectively | Reopen without repeating rewards or flags |
| 25 | Pike's turn-back path completed | Offer remains available, no state change |
| 10–15 | Synthesized act/finale flags and map/RNG progression | Serialization probes only; use natural saves for campaign acceptance |

Load Game marks valid saves with `HIGHEST LEVEL`, `LONGEST PLAYTIME`, and
`MOST RECENT` badges. Rankings include autosave and all numbered slots;
ties share badges. Level uses the protagonist's level, and most recent uses
the save timestamp rather than the last time a slot was loaded.

Use title → Load Game to select a slot. Default controls: arrows move/select,
Enter interacts/confirms, Escape backs out, `M` opens the field menu, `T` opens
Travel. NPCs may move; face an adjacent NPC before interacting. Fully exit via
field menu → Quit → confirm when a step calls for process restart.

Reset only a disposable slot by copying its pristine counterpart, for example:

```bash
cp "$rf10_run/pristine/001.yaml" "$rf10_run/saves/001.yaml"
```

## Pike and quest rewards

1. Load slot 1; note GP, HP/MP, and item quantities. Open Pike's offer. The middle
   vouch choice stays visible but disabled; confirming it must not close the offer
   or change resources. Cancel, reopen, and confirm the same offer returns.
2. Choose turn back. The party receives nothing and the offer remains available
   on the next conversation. Save into an empty scratch slot, quit, restart, and
   confirm the offer and unchanged resources persist.
3. Reset slot 1, choose pledge, and check the follow-up text acknowledges it.
   Save to another empty scratch slot; restart and talk twice. No reward appears,
   and the offer stays retired. Inspect the scratch YAML for
   `bridge_zone5_answered` and `bridge_zone5_pledged`.
4. Load slot 2 and choose the enabled vouch branch. Compare inventory: exactly
   +3 Potion and +2 Antidote. Save, restart, and talk twice. Counts must not grow;
   `bridge_zone5_answered` and `bridge_zone5_vouched` remain set.
5. Load slot 3 and complete Elder Maeve's conversation. Verify +2 Hi-Potion and
   +1 Tent, Act I completed and Act II active on the Quest Board. Save, restart,
   and talk twice: no second reward, and quest statuses remain unchanged.
6. Load slot 4 and finish Archivist Hesta's conversation. Verify +2 Ether and the
   Flame-glyph quest completed. Save, restart, and talk twice: no second reward.
   Repeat steps 5–6 with naturally obtained before-completion saves from the
   campaign run below; the generated fixtures contain injected story state.

## Battle and transport

Use a separate disposable save directory for focused battles. For a quick
victory/reward observation, the existing rewards fixture wins in one hit:

```bash
mkdir "$rf10_run/battle-saves"
RPG_S1_SAVE_DIR="$rf10_run/battle-saves" XDG_CONFIG_HOME="$rf10_run/config" \
  target/debug/rpg-s1 play rusted_kingdoms --seed 23 --battle-fixture rewards
```

Observe the reward screen, acknowledge it, save, restart, and verify the
inventory, GP, levels, and return map/facing. Subsequent confirmation must not
repeat the reward. In the normal campaign, observe successful and failed fleeing,
boss-blocked fleeing, and defeat → Game Over. Only victory grants rewards or
victory flags; failed/blocked fleeing legitimately consumes a turn, and a flee
attempt may consume RNG. Cancel item/spell target selection and check that no
item or MP was spent.

After naturally unlocking Teleport and visiting two regional destinations,
record MP and location, open Spells → Teleport, then cancel destination browsing
and confirmation. Reopen and complete the trip: charge the displayed MP cost
once, restore the destination, then save/restart and inspect it again. Check Sail
and Fly once naturally unlocked. An unavailable destination or busy transition
must not charge resources. Headless coverage checks canceled Warp, busy requests,
and a lost unlock during animation; late destination asset failure remains an
operator observation and is not covered by those Warp resource assertions.

## Clean campaign and natural boundary saves

Create a separate empty profile. Launch without debug options or the debug-party
environment override; choose New Game and proceed normally to credits.

```bash
mkdir "$rf10_run/campaign-saves"
env -u RPG_S1_PARTY_PRESET RPG_S1_SAVE_DIR="$rf10_run/campaign-saves" \
  XDG_CONFIG_HOME="$rf10_run/config" target/debug/rpg-s1 play rusted_kingdoms
```

Save into distinct empty slots when `story_quest_started`, `story_act2_started`,
`story_act3_started`, `story_act4_started`, and `boss_zone10_defeated` naturally
appear. Save again on reaching the Ardel epilogue before speaking with Joss;
finishing his conversation sets `game_complete` and enters credits. Retain the
final autosave if one is published. At every saved boundary,
fully quit/relaunch/load and compare quest statuses, party order/rows, controlled
member, HP/MP, equipment, GP/items, opened boxes, visited maps, and position/facing.
Continue through the next encounter or interaction to detect delayed drift.

During that run, include Harborgate inn; Ancient Ruins gate, courtyard and
sanctum; and Ruinwatch town, inn, shop and monastery vaults. Confirm the Mountain
Pass missing sign is the source-matching silent no-op. Finish the Hearth route
and speak with Keeper Joss in the Ardel epilogue to reach credits. Do not use
slots 10–15 as evidence of campaign traversal. Preserve natural boundary saves
and a short observation log in the run directory.

## Deterministic replay and a corrupted copy

Run the focused automation first:

```bash
cargo test -p rpg-engine -j 1 \
  battle::tests::persistence_replay::seeded_gameplay_record_replay_and_corrupted_copy_across_processes \
  -- --exact
```

It records three seeded encounter choices, physical victories, and EXP/GP/loot
transactions in fresh processes; repeats the same seed; compares a different
seed's gameplay state; replays with physical input suppressed; and corrupts two
hashes after a valid prefix. The first mismatch must report action 1 at frame 3,
exit unsuccessfully, and preserve both input files. A malformed copy also fails
decoding. Successful fixtures clean themselves up; failures retain logs/saves
under `temp/refactoring/rf10-*`.

For the visible gate, record a representative battle with the installed binary:

```bash
RPG_S1_SAVE_DIR="$rf10_run/battle-saves" XDG_CONFIG_HOME="$rf10_run/config" \
  target/debug/rpg-s1 record "$rf10_run/live.yaml" rusted_kingdoms \
  --seed 17 --battle-fixture enemy-ai
```

Write down the normalized choices and their logical frames in `live.yaml`.
Repeat those actions from the same starting fixture into a new file with seed
17, and then seed 18. Compare matching action/frame sequences before comparing
hashes: visible input timing differences are not evidence of RNG divergence.
The same seed/sequence must match; the different seed must produce a valid
different encounter/battle/loot result. The automated fixture supplies precisely
matched frames; a hand-timed recording may require another attempt.

Replay the original without gameplay input; it must finish successfully. To
create a fresh corrupted copy without modifying the original:

```bash
python3 - "$rf10_run/live.yaml" "$rf10_run/live-corrupt.yaml" <<'PY'
import pathlib, re, sys
source, destination = map(pathlib.Path, sys.argv[1:])
lines = source.read_text().splitlines(keepends=True)
hashes = [i for i, line in enumerate(lines) if re.match(r"\s*state_hash:", line)]
assert len(hashes) >= 3, "record at least three action checkpoints"
for index, value in [(hashes[1], "0" * 64), (hashes[2], "f" * 64)]:
    lines[index] = re.sub(r"(state_hash:\s*).*", rf"\g<1>'{value}'", lines[index])
with destination.open("x") as output:
    output.writelines(lines)
PY
sha256sum "$rf10_run/live.yaml" "$rf10_run/live-corrupt.yaml" > "$rf10_run/inputs.sha256"
RPG_S1_SAVE_DIR="$rf10_run/battle-saves" XDG_CONFIG_HOME="$rf10_run/config" \
  target/debug/rpg-s1 replay "$rf10_run/live.yaml"
RPG_S1_SAVE_DIR="$rf10_run/battle-saves" XDG_CONFIG_HOME="$rf10_run/config" \
  target/debug/rpg-s1 replay "$rf10_run/live-corrupt.yaml"
sha256sum -c "$rf10_run/inputs.sha256"
```

The corrupted replay must stop at action index 1 (the second checkpoint), name
its frame and expected/actual hashes, and return a nonzero status. Both SHA-256
checks must pass afterward. Keep the original recording, corrupted copy, exit
statuses, and observed first-divergence message with the run evidence.
