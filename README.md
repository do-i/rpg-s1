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

Select **Run playable M4 slice**. The initial Bevy build is expected to
take longer than later incremental builds. You can also run it directly:

```sh
cargo run
```

Use the Up and Down arrows to select a title item, then Enter or Space to
confirm. New Game opens name entry; Enter confirms the name. Enter or Space
advances the intro, while Escape uses the supported intro-skip path. In Ardel,
tap the Arrow keys to move one tile; perpendicular arrows provide diagonal
movement. Load Game remains disabled because saves have not been migrated.

Run the deterministic Ardel composition check with:

```sh
scripts/check-ardel-screenshot.sh
```

## Scope

The current playable slice contains:

- a resizable 1280x766 Bevy window;
- the migrated title artwork, font, music, and menu sound effects;
- name entry and the opening linear cutscene;
- the production Ardel TMX map, visible layer ordering, and collision data;
- Aric spawning, four/eight-way tile movement, TSX-authored animation, and
  clamped camera behavior;
- title-to-Ardel BGM replacement; and
- automated parser, runtime, production-package, and screenshot checks;
- build, run, test, lint, format, release, and clean menu actions.

Portals, NPC interactions, saves, field menus, encounters, and combat remain
later milestones.
