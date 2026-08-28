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
record/replay, debug-map launches, both map editors, and release utilities. Use
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

The field menu contains Status, Spells, Items, Equipment, Quests, Save, and
Quit. Opening any full-screen overlay pauses world movement, encounters, NPC
wandering, interaction, and transitions until the overlay closes.

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

## Validation and developer tools

Run the normal project checks with:

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
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

The production scenario currently has 13 inherited content diagnostics, so
`validate-scenario` is expected to exit unsuccessfully until those issues are
resolved or approved as explicit differences. Their exact flags, item
references, stale recruitment references, and missing cursor asset are
recorded in
[`docs/adr/0007-inherited-scenario-data-debt.md`](docs/adr/0007-inherited-scenario-data-debt.md).
The runtime map, dialogue, and encounter sweeps are the passing production
load checks.

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

Record normalized actions to a fresh path, then replay them without physical
input:

```sh
cargo run -- record /tmp/rpg-s1-check.yaml rusted_kingdoms --seed 13 \
  --start-map town_01_ardel --start-position 10,0
cargo run -- replay /tmp/rpg-s1-check.yaml
```

Replay verifies the game/scenario identity and every recorded state checkpoint
and exits unsuccessfully at the first divergence. See
[`docs/content-authoring.md`](docs/content-authoring.md) for the complete Tiled,
Pygame/web editor, validation, debug-map, and replay workflow.

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

This list describes implemented systems, not final campaign acceptance. The
current completion evidence and blockers are maintained in
[`docs/m14-parity-audit.md`](docs/m14-parity-audit.md).

## Licensing and credits

The Rust source code is available under the [MIT License](LICENSE). Scenario
data and bundled art, audio, fonts, and tilesets retain their own terms; the
code license does not grant rights to those assets.

Required notices and known provenance are preserved beside the applicable
assets, including:

- `assets/scenarios/rusted_kingdoms/credits/01_aric_credits.txt` for the
  Liberated Pixel Cup components used by Aric;
- `assets/scenarios/rusted_kingdoms/assets/tilesets/ground/CREDITS-terrain.txt`
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
