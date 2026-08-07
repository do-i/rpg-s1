# rpg-s1

A small native Rust and [Bevy](https://bevy.org/) RPG prototype. The first
playable slice migrates the title screen from `agentic-rpg` so the engine and
workflow can be evaluated before any wider port.

## Requirements

- Rust 1.97 or newer
- [lazymenu-cli](https://github.com/do-i/lazymenu-cli/) for the project menu
- Linux graphics and audio development dependencies required by Bevy

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

Select **Run title-screen prototype**. The initial Bevy build is expected to
take longer than later incremental builds. You can also run it directly:

```sh
cargo run
```

Use the Up and Down arrows to select an item, then Enter or Space to confirm.
Load Game is disabled because saves have not been migrated. New Game reports
the next migration boundary, and Quit exits the application.

## Scope

This baseline intentionally contains only:

- a resizable 1280x766 Bevy window;
- the migrated title artwork, font, music, and menu sound effects;
- keyboard menu navigation and quit behavior;
- one small unit test for menu wrapping;
- build, run, test, lint, format, release, and clean menu actions.

No world, save, combat, dialogue, or Tiled-map systems have been migrated yet.
