# Chronicles of the Lost Flame — Rust port

`rpg-s1` is the native Rust and [Bevy](https://bevy.org/) port of **Chronicles
of the Lost Flame**. It loads story, maps, dialogue, encounters, art, and audio
directly from a selected scenario package. Normal gameplay and save conversion
are implemented in Rust and do not invoke Python; the formal packaged-runtime
proof required by M14.12 remains open.

The port is in final parity validation, not ready for public distribution. The
normal campaign route has live acceptance through Harborgate (W12.3), while
Ancient Ruins/Ruinwatch and the later campaign waves still need their recorded
playthrough checks. The bundled parity assets also include unresolved
redistribution-rights blockers described under [Licensing and credits](#licensing-and-credits).

## Screenshots

<p align="center">
  <img src="docs/screenshots/title-screen.png" alt="Chronicles of the Lost Flame title screen" width="49%">
  <img src="docs/screenshots/ardel-overworld.png" alt="Ardel overworld with the player, townspeople, shops, and inn" width="49%">
</p>
<p align="center">
  <img src="docs/screenshots/volcanic-battle.png" alt="Five-character party battling two minotaur brutes in a volcanic region" width="49%">
  <img src="docs/screenshots/field-menu.png" alt="Field menu showing status, spells, items, equipment, quests, character, save, and quit commands" width="49%">
</p>

## Run the game

### From a packaged build

Extract the Linux x86-64 archive and keep the executable beside its `assets/`
directory:

```text
rpg-s1-<version>-x86_64-linux/
├── rpg-s1
└── assets/
```

Launch it from any working directory:

```sh
path/to/rpg-s1-<version>-x86_64-linux/rpg-s1 play rusted_kingdoms --seed 1
```

No Rust toolchain, Python interpreter, Python package, or source checkout is
needed by a packaged game. A self-contained Milestone 14 release candidate has
not yet completed clean-profile acceptance.

### From this repository

Requirements:

- Rust 1.97 or newer;
- Git LFS, with the repository's binary assets materialized;
- the Linux graphics, windowing, audio, C/C++ linker, and `pkg-config`
  dependencies required by Bevy; and
- optionally, [lazymenu-cli](https://github.com/do-i/lazymenu-cli/) for the
  searchable developer menu; and
- optionally, Tiled's `tmxrasterizer`, ImageMagick 7's `magick`, and
  `sha256sum` for the deterministic screenshot check.

After cloning, run these commands from the repository root:

```sh
git lfs install
git lfs pull
cargo run -- play rusted_kingdoms --seed 1
```

Running `cargo run` with no arguments selects the same default scenario and
seed. For an optimized build:

```sh
cargo run --release -- play rusted_kingdoms --seed 1
```

On Arch Linux without a Vulkan-capable GPU, the Mesa software Vulkan driver is
available as `vulkan-swrast`:

```sh
sudo pacman -S --needed vulkan-swrast vulkan-tools
```

Alternatively, run `lazymenu-cli` from the repository root and select **Play -
Seed 1**. The menu also exposes the test suite, validation and sweep commands,
record/replay, debug-map launches, the map editor, and release utilities. Use
`/` to search and `q`, Escape, or Ctrl+C to leave the launcher.

## Controls

| Context | Controls |
| --- | --- |
| Title and menus | Up/Down selects; Enter, Numpad Enter, or Space confirms; Escape goes back. |
| Name entry | Type a name, Backspace deletes, Enter confirms, and Escape cancels. |
| Intro and dialogue | Enter or Space advances; Escape follows the supported intro-skip/back path. |
| World | Hold Arrow keys to move in four or eight directions; Enter or Space interacts with the facing NPC, sign, box, or service. |
| Field menu | `M` or Escape opens it; `M` closes it; Arrow keys navigate; Enter or Space confirms; Escape backs out. |
| Field shortcuts | `I` opens Items, `S` opens Status, and `Q` opens Quests. |
| Battle | Up/Down selects a command, ability, item, or target; Enter or Space confirms; Escape cancels a nested choice or attempts to flee from the command menu. |
| Confirmations | `Y` or any confirm key accepts save-overwrite and quit prompts; `N` declines. |
| Gamepad | South confirms, East backs out, the D-pad navigates and walks, North travels, and the left stick both walks and steps menus. |
| Mouse | On the title and options screens, hovering highlights a row and clicking activates it. Clicks in the letterbox bars are ignored. |
| Options | Choose Options on the title screen. Up/Down picks a row, Left/Right switches between the keyboard and gamepad column, Confirm starts a rebind, Delete removes a binding, and Escape cancels a rebind or leaves. |

The field menu contains Status, Spells, Items, Equipment, Quests, Save, and
Quit. Opening any full-screen overlay pauses world movement, encounters, NPC
wandering, interaction, and transitions until the overlay closes.

## Options

The Options entry on the title screen edits keyboard and gamepad bindings and
the dialogue text speed. Every action keeps at least one binding: assigning an
input that another action in the same category already holds moves it, but an
action's last binding is never taken away, and the screen says which action
would have been left unbound. Menu bindings and movement bindings are separate
categories, so the same arrow key can step a menu and walk the world.

Changes are written immediately. Options-file precedence is:

1. `RPG_S1_CONFIG_DIR`, when set;
2. `$XDG_CONFIG_HOME/rpg-s1/options.yaml`, when `XDG_CONFIG_HOME` is set; or
3. `$HOME/.config/rpg-s1/options.yaml`.

The file is plain YAML and safe to hand-edit. Deleting it restores the shipped
defaults, as does the screen's own Reset To Defaults row. A row the game cannot
honor is reported and left at its default rather than dropped silently, and a
file written by a newer build is refused rather than partly applied. Options
live apart from saves on purpose, so moving `RPG_S1_SAVE_DIR` between fixture
slots does not reset the player's bindings.

## Saves

The game provides native slots 1–100 and checkpoints autosave slot 0 after a
settled arrival on each new map. Empty manual slots save immediately; occupied
slots require explicit overwrite confirmation. Writes use a verified temporary
file and atomic replacement.

Save-directory precedence is:

1. `RPG_S1_SAVE_DIR`, when set;
2. `$XDG_DATA_HOME/rpg-s1/saves`, when `XDG_DATA_HOME` is set; or
3. `$HOME/.local/share/rpg-s1/saves`.

To convert one save from the pinned Python version into a native slot:

```sh
cargo run -- import-python-save path/to/007.yaml --slot 7
```

The converter is explicit, one-way, and implemented in Rust. It never scans
for legacy saves, refuses an occupied destination by default, and accepts
`--replace` only after preserving a verified backup. Checksumless input also
requires `--allow-unchecked`; a checksum mismatch is always rejected. Run
`cargo run -- import-python-save --help` for the full syntax.

## Settings

`assets/settings.yaml` carries the engine settings, ported from the Python
build's `engine/settings/settings.yaml` with the same block names. The port
reads four of its keys — dialogue text speed, the font-size scale, whether an
item spent on the whole party asks first, and whether a large magic-core
exchange asks first. Every other key is documented in the file as decided
elsewhere in this engine (the fixed canvas policy, save-directory resolution,
or an environment variable) and is ignored rather than rejected. Deleting the
file runs the game on the same values.

## Validation and developer tools

Run the normal project checks with:

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -- validate-scenario rusted_kingdoms --baseline assets/scenarios/rusted_kingdoms/validation-baseline.txt
scripts/check-ardel-screenshot.sh
```

Scenario commands default to `rusted_kingdoms`:

```sh
cargo run -- validate-scenario rusted_kingdoms
cargo run -- map-report rusted_kingdoms
cargo run -- map-sweep rusted_kingdoms
cargo run -- dialogue-report rusted_kingdoms
cargo run -- dialogue-sweep rusted_kingdoms
cargo run -- encounter-sweep rusted_kingdoms
```

The production scenario intentionally retains four undefined Sorcerer ultimate
flags and one orphan transport flag from the inherited content, so a bare
`validate-scenario` exits unsuccessfully by design. The accepted diagnostics
and their source-parity rationale are kept beside the scenario rather than
being hidden by disabling a check.

That is why CI gates on `--baseline` instead. The accepted diagnostics are
enumerated in
[`assets/scenarios/rusted_kingdoms/validation-baseline.txt`](assets/scenarios/rusted_kingdoms/validation-baseline.txt),
and the run passes only when the report matches that file exactly: a diagnostic
the file omits fails as a regression, and a line the validator no longer
produces fails too, so paying down a debt includes deleting its line. Adding a
line records a decision — the file is not a mute button. The runtime map,
dialogue, and encounter sweeps remain the passing production load checks.

Gameplay defaults to deterministic seed `1`. A debug launch requires a map and
walkable position together and can add a party preset or session-only flags:

```sh
cargo run -- play rusted_kingdoms --seed 13 --timings \
  --start-map town_01_ardel --start-position 10,0 \
  --party-preset full --set-flag story_quest_started
```

`--timings` logs world and battle hotspots every 120 frames. Debug overrides
are logged and remain session-only unless you explicitly save. Set
`RPG_S1_MUTE_AUDIO=1` to mute audio or `RPG_S1_DEBUG_COLLISION=1` to draw world
collision and portal outlines.

The four focused battle-acceptance fixtures enter battle directly and use a
safe Starting Forest return point. Keep their saves isolated from campaign
slots (and leave audio enabled when checking cues):

```sh
RPG_S1_SAVE_DIR=/tmp/rpg-s1-battle-acceptance cargo run -- play --seed 7 --battle-fixture feedback
RPG_S1_SAVE_DIR=/tmp/rpg-s1-battle-acceptance cargo run -- play --seed 7 --battle-fixture status
RPG_S1_SAVE_DIR=/tmp/rpg-s1-battle-acceptance cargo run -- play --seed 17 --battle-fixture enemy-ai
RPG_S1_SAVE_DIR=/tmp/rpg-s1-battle-acceptance cargo run -- play --seed 23 --battle-fixture rewards
```

`feedback` supplies matched front/back attackers, Power Strike for normal hit
comparison, Shadow Step for a guaranteed critical, and a 5%-hit basic attack
for MISS feedback. `status` supplies Rally, War Cry, guaranteed-stun Boulder
Crash, and a Charm-only Troll Sage. `enemy-ai` starts the Troll Sage at full HP
and gives Aric Rally so its first turn exercises the no-eligible-move fallback;
repeat the same inputs at seed 17 to compare later choices. `rewards` ends in
one hit and awards exactly 1,000 EXP plus deterministic authored loot, taking
Aric from level 1 to 3.

Record normalized actions to a fresh path, then replay them without physical
input:

```sh
cargo run -- record /tmp/rpg-s1-check.yaml rusted_kingdoms --seed 13 \
  --start-map town_01_ardel --start-position 10,0
cargo run -- replay /tmp/rpg-s1-check.yaml
```

Replay verifies the game/scenario identity and every recorded state checkpoint
and exits unsuccessfully at the first divergence. Use
`scripts/map-editor.sh setup` and `scripts/map-editor.sh check` before launching
the web map editor (`scripts/map-editor.sh web`); review the TMX diff and run the
validation, sweep, debug-map, and replay commands above after every authored
change. The editor lives in `tools/map_editor` and needs no external checkout.

## Current game coverage

The Rust runtime currently includes:

- scenario-selected title, name-entry, intro, world, audio, and font assets;
- TMX/TSX maps with layered rendering, collision, portals, camera movement,
  animated party/NPC/enemy sprites, signs, treasure boxes, and encounters;
- dialogue conditions/effects, recruitment, quests, shops, inns,
  apothecary crafting, inventory, equipment, field spells, and teleporting;
- deterministic turn-based party combat with rows, abilities, items, status
  effects, enemy AI, bosses, flee, rewards, progression, and game-over flow;
- native save/load/autosave and one-way Python-save conversion; and
- manifest-selected scenario packages, deterministic seeds, record/replay,
  validation reports, production sweeps, and map-authoring integrations.

This list describes implemented systems, not final campaign acceptance. Active
acceptance evidence and blockers are maintained locally under the ignored
`plans/` directory.

## Licensing and credits

The Rust source code is available under the [MIT License](LICENSE). Scenario
data and bundled art, audio, fonts, and tilesets retain their own terms; the
code license does not grant rights to those assets.

Required notices and known provenance are preserved beside the applicable
assets, including:

- `assets/scenarios/rusted_kingdoms/credits/01_aric_credits.txt` for the
  Liberated Pixel Cup components used by Aric;
- `assets/scenarios/rusted_kingdoms/media/tilesets/ground/CREDITS-terrain.txt`
  for the LPC terrain atlas;
- the bundled SIL Open Font License notices for Philosopher and Quintessential;
  and
- the other creator/source notices under the scenario asset tree.

The auditable path-by-path record is
[`docs/asset-license-inventory.md`](docs/asset-license-inventory.md), with a
shorter overview in [`assets/README.md`](assets/README.md). Most copied parity
assets still lack complete ownership, acquisition, license, or redistribution
evidence. Do not publish or redistribute the current asset bundle until every
shipped entry is approved, replaced, or excluded; local parity use is not
public redistribution permission.

### Development assistance

Large parts of this port were written with [Claude
Code](https://claude.com/claude-code) (Anthropic) acting as a pair programmer,
under human direction and review. This note is the project's single record of
that; individual commits carry no `Co-Authored-By` or session trailers, so
`git log` stays readable. Commit messages here are a subject line only — put
the reasoning in `plans/`, `docs/`, or the pull request instead.

## Maintainer release flow

`dev` is the integration branch. `main` only fast-forwards to a validated
`dev` commit before that commit is tagged. Inspect or dry-run the calendar
versioned release flow with:

```sh
scripts/release.sh status
scripts/release.sh --dry-run cut
```

`scripts/release.sh cut` bumps `Cargo.toml` and `Cargo.lock`, waits for the
matching CI run, fast-forwards `main`, and pushes the branch and tag. A tag
starts `.github/workflows/release.yml`, which builds the locked Linux x86-64
binary and bundles it beside `assets/`. Do not publish a release while the
Milestone 14 parity and asset-rights blockers above remain open.
