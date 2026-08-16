# M8 Encounters and Enemy Presence Manual Play Checklist

This Gate 8 check exercises the real Bevy/X11 window, production Starting
Forest map, visible-enemy simulation, contact transition, battle handoff, and
production battle audiovisual assets. Focused Rust tests remain the
authoritative exhaustive proof for deterministic selection, cadence,
modifiers, collision, one-shot requests, and return-context restoration.

## Environment

- Linux/X11 on `DISPLAY=:0`
- Bevy Vulkan backend using Mesa llvmpipe
- isolated native save root `/tmp/rpg-s1-m8-saves`
- frame-held X11 key events, not direct ECS resource mutation
- production `rusted_kingdoms` package
- pinned Python source fixture at revision
  `08970359d6cb03586948625d29b0d3351dbbf785`

## 2026-08-15 Gate 8 result

- [x] The standalone importer installed the pinned Python Slot 07 fixture in
  the isolated save root, and the title picker showed `Imported Aric` in
  `zone_01_starting_forest` as the latest valid save.
- [x] Loading the slot entered the production Starting Forest at `[29, 1]`
  with the imported facing and rendered five regular visible enemies plus the
  authored Grik boss.
- [x] Regular enemies visibly wandered between captured frames while remaining
  on open map tiles; the boss remained at its authored tile.
- [x] Standing still did not synthesize a hidden random battle. Spawn cadence
  only reactivates an inactive visible enemy, as covered by the focused cadence
  test.
- [x] Walking down the source path brought a visible goblin into chase/contact
  range. Two further fresh movement actions produced one white-flash battle
  transition and froze world input.
- [x] The final battle handoff selected the configured `Goblin` plus `Goblin`
  formation and rendered the production
  `zone1-bg-1280x468.webp` forest background.
- [x] The live battle state started the normal `battle.normal` BGM selection;
  the boss fixture separately resolves `battle.boss`. The live process reported
  no audio asset error, and `ffprobe` decoded the encounter SFX plus both 30.77-
  second BGM files (the SFX duration is 5.33 seconds).
- [x] Automated handoff inspection retained the complete party/enemy initial
  combatant data, inactive/engaged world-enemy snapshot, map, player position,
  facing, and pre-battle world BGM key. Contact emitted exactly one battle
  request.
- [x] Town/no-zone lookup returns no encounter state, while the Starting Forest
  lookup admits only formations declared by its encounter-zone document.

## Captured evidence

The temporary screenshots are intentionally not committed:

- `/tmp/rpg-s1-m8-final-world.png` — final production replay with visible
  enemies, SHA-256
  `eacaf246c1ff275fc54684c4880c00db7eda6ff864bdd1057eeeb1aed760d1fe`.
- `/tmp/rpg-s1-m8-final-approach.png` — white-flash contact frame, SHA-256
  `02ae10587b1fe4eac37844ded3bcd8162462d5cb5f136cefdff98a79d150c0ce`.
- `/tmp/rpg-s1-m8-final-battle.png` — final replay entered battle with the
  configured formation and forest background, SHA-256
  `710c510af7b8cf0cc0d50950713b56fd9866ecec784d79faf75d0adc021fcf51`.

The ordinary full suite passed 405 tests with its 23 opt-in source audits
skipped. A second run supplied every pinned-corpus environment variable and
passed all 23 source audits, including the complete encounter, enemy, and
battle-background catalogs. Strict Clippy also passed with warnings denied.

`cargo run -- validate-scenario` still reports the intentionally partial
campaign package: later-zone maps, sprites, battle assets, quests, recipes,
boss move sets, and other future-milestone dependencies have not been copied.
Its 227 diagnostics are not presented as a successful full-package validation;
the production Gate 8 slice and the complete pinned-source catalog audits above
are the scoped M8 acceptance paths.
