# M11 Economy, Services, and Quests Manual Play Checklist

This Gate 11 check combines production Bevy/X11 interaction with focused
transaction tests and a clean pinned-source audit. The live replay starts from
a normal New Game and reaches Ardel's service interior without debug state
injection.

## Environment

- Linux/X11 on `DISPLAY=:0`
- Bevy Vulkan backend using Mesa llvmpipe
- production `rusted_kingdoms` package and normal New Game flow
- isolated native save root under `/tmp`
- sound disabled with `RPG_S1_MUTE_AUDIO=1`
- frame-held X11 key events through the normal action-input systems
- pinned Python source revision
  `08970359d6cb03586948625d29b0d3351dbbf785`

## Source-shape decision

The pinned `data/quests.yaml` has sixteen rows, each containing exactly seven
required strings: id, name, type, location, description, started flag, and
completed flag. It has no objective collection, item threshold, turn-in rule,
reward record, or mutable quest-progress store. Python derives the board state
as Completed, Active, or Inactive from the two flags. Relay flags and rewards
are authored in mutually exclusive dialogue entries, with `give_items` beside
the completion flag.

M11.16-M11.18 therefore preserve those dialogue-authored flag and item effects.
M11.17 is an audited absence: adding an item-objective counter or implicit item
removal would create behavior that does not exist in the pinned game.

## 2026-08-22 Gate 11 result

- [x] A normal title, name-entry, and complete intro replay entered Ardel and
  walked through the production shop portal.
- [x] The source's pixel-coordinate interaction rule reaches Wilkin across the
  counter. Dialogue completion opened the Item Shop exactly once and locked
  world/menu input while the service was active.
- [x] The Item Shop showed current GP, only unlocked stock, item names, prices,
  owned quantities, affordability, descriptions, and Buy/Sell navigation.
- [x] The same interior routed armor and magic-core NPCs to visibly distinct
  service titles. An empty core inventory produced the correct read-only empty
  state.
- [x] Yarrow's dialogue opened the Apothecary. All eleven recipes remained
  visible with locked or missing-input state, output, cost, selected recipe
  ingredients, and owned/required quantities. Confirming an unavailable recipe
  displayed the rejection without changing state.
- [x] The field-menu Quest command displayed all sixteen ordered quests with
  active/inactive state, location, and description. `main_act1` was Active from
  the normal source bootstrap flag.
- [x] Focused transactions cover unlocked and locked stock, affordable and
  rejected buys, sell eligibility, GP/item caps, owned-core exchange, high-value
  confirmation data, exact inn payment/recovery (including KO, MP, and status),
  every recipe availability class, atomic craft rollback, and duplicate-unique
  rejection.
- [x] A native save encode/decode round trip retained purchased inventory, GP,
  dialogue quest flags, and the derived completed quest state.

## Temporary evidence

The screenshots are intentionally not committed:

- `/tmp/rpg-s1-m11-quest-board.png` — all sixteen source quests in the field
  menu, including the active main quest.
- `/tmp/rpg-s1-m11-shop-interior-replay.png` — production Ardel shop entered
  from a normal New Game.
- `/tmp/rpg-s1-m11-item-dialogue.png` — Wilkin reached across the production
  counter after the source pixel-range correction.
- `/tmp/rpg-s1-m11-item-service-menu.png` — dialogue-routed Item Shop.
- `/tmp/rpg-s1-m11-item-buy-rows.png` — unlocked stock, GP, quantities,
  affordability, and selected-item description.
- `/tmp/rpg-s1-m11-apothecary-service.png` — complete recipe list, availability,
  output, ingredients, and rejected craft feedback.
- `/tmp/rpg-s1-m11-magic-core-service.png` — distinct Magic Core Exchange empty
  state.

The ordinary suite passed 490 tests with 24 opt-in audits skipped. A separate
run supplied every clean pinned-corpus path and passed all 24 audits, including
the Python battle transcript and validator comparison. Strict Clippy with
warnings denied, formatting, `git diff --check`, and the Ardel RGBA screenshot
oracle passed (hash
`85ce229c04604258debbad65643ebcc62177f084727178d29b968baeb35b2012`).

`cargo run -- validate-scenario` loaded and checked the complete currently
copied catalogs (`recipes=11`, `quests=16`, 1,710 references). Its remaining 37
errors and one warning are later content-wave gaps: missing future items/maps,
four ultimate-spell producers, one future BGM, one future TMX, and the existing
title cursor asset. None names an M11 service, recipe, quest, or dialogue
reference.
