# rpg-s1

A native Rust and [Bevy](https://bevy.org/) RPG port in progress. The current
playable slice starts a new game, plays the intro, and enters a rendered,
walkable Ardel map using scenario-authored TMX/TSX/YAML content.

## Requirements

- Rust 1.97 or newer
- [lazymenu-cli](https://github.com/do-i/lazymenu-cli/) for the project menu
- Linux graphics and audio development dependencies required by Bevy
- Optional screenshot-check tools: Tiled's `tmxrasterizer`, ImageMagick 7's
  `magick`, and `sha256sum`

For Arch Linux systems without a Vulkan-capable GPU, install the software
Vulkan driver:

```sh
sudo pacman -S --needed vulkan-swrast vulkan-tools
```

## Run

From the repository root:

```sh
lazymenu-cli
```

Select **Run playable M7 slice**. The initial Bevy build is expected to
take longer than later incremental builds. You can also run it directly:

```sh
cargo run
```

Use the Up and Down arrows to select a title item, then Enter or Space to
confirm. New Game opens name entry; Enter confirms the name. Enter or Space
advances the intro, while Escape uses the supported intro-skip path. In Ardel,
tap the Arrow keys to move one tile; perpendicular arrows provide diagonal
movement. Load Game is enabled whenever at least one valid native slot exists.

In the World, press `M` for the field menu, `I` for Items, or `S` for Status.
Use Arrow keys to navigate, Enter to confirm, and Escape to return one level.
The M6 slice includes shared party/status, inventory tabs and item use,
equipment previews/swaps, learned field spells, and visited-map teleporting.
Save opens slots 1-100; empty slots write immediately, while occupied slots
require explicit overwrite confirmation. Quit asks before returning to title.

Native slots are stored under `$XDG_DATA_HOME/rpg-s1/saves` when
`XDG_DATA_HOME` is set, otherwise `$HOME/.local/share/rpg-s1/saves`. Set
`RPG_S1_SAVE_DIR` to use an explicit directory for testing or portable runs.
Writes use a verified temporary file and atomic replacement.

To convert one save from the pinned Python version into a native slot:

```sh
cargo run -- import-python-save path/to/007.yaml --slot 7
```

The converter is explicit and one-way. It does not need Python or a source
checkout, never scans for old saves, refuses an occupied destination by
default, and accepts `--replace` only after preserving a verified backup.
Checksumless input additionally requires `--allow-unchecked`; a checksum
mismatch is always rejected. Run `cargo run -- import-python-save --help` for
the complete syntax.

Run the deterministic Ardel composition check with:

```sh
scripts/check-ardel-screenshot.sh
```

## Releases

`dev` is the integration branch; feature branches merge into it. `main` never
receives commits directly — it only ever fast-forwards to `dev` and is then
tagged. Cut a release from `dev`:

```sh
scripts/release.sh status      # show state, change nothing
scripts/release.sh --dry-run cut
scripts/release.sh cut         # bump, wait for CI, fast-forward main, tag
```

Versions are calendar-based (`year.month.sequence`, tagged `v2026.8.1`), and
the sequence is computed from existing tags, so the common case needs no
version argument. `cut` bumps `Cargo.toml`/`Cargo.lock` on `dev` as its own
commit, waits for a green `ci.yml` run on that commit, then fast-forwards
`main` and pushes the branch and tag atomically.

Pushing the tag starts `release.yml`, which builds the Linux x86_64 binary,
bundles it with `assets/` (the layout an installed build expects — the game
reads `assets/` beside its executable), and attaches
`rpg-s1-<version>-x86_64-linux.tar.gz` to the GitHub release. The workflow
refuses to publish when the tag and `Cargo.toml` disagree.

## Scope

The current playable slice contains:

- a resizable 1280x766 Bevy window;
- the migrated title artwork, font, music, and menu sound effects;
- name entry and the opening linear cutscene;
- the production Ardel TMX map, visible layer ordering, and collision data;
- Aric spawning, four/eight-way tile movement, TSX-authored animation, and
  clamped camera behavior;
- title-to-map BGM replacement;
- source-authored NPC presence, occupancy, animation, wandering, and dialogue;
- atomic faded map portals with a playable Ardel interior and Starting Forest;
- Elise recruitment, configured signs, and persistent one-time treasure boxes;
- indexed World interaction sound effects; and
- source-authored class/item catalogs with party, status, inventory,
  equipment, field-item, spell, and teleport screens;
- versioned native save slots, atomic writes, recovery-aware title loading,
  exact runtime-state restoration, and one-way Python-save conversion;
- automated parser, runtime, production-package, and screenshot checks;
- build, run, test, lint, format, release, and clean menu actions.

Encounters and combat remain later milestones.
