# M6 field-menu visual evidence

Date: 2026-08-16

Runtime revision: `315badc`

This check exercises the real Bevy/X11 window and the production scenario
package. It starts from the isolated imported save in
`/tmp/rpg-s1-m8-saves`, opens each redesigned field submenu through the real
command list, and captures its primary and secondary states.

## Environment

- Linux/X11 on `DISPLAY=:0`
- Bevy 0.19 Vulkan backend
- Mesa llvmpipe software renderer
- 1280x766 game client area
- `RPG_S1_SAVE_DIR=/tmp/rpg-s1-m8-saves cargo run`
- frame-separated X11 key events against the game window

## Results

| Check | Result | Evidence |
| --- | --- | --- |
| Items | PASS after repair | The Pouch, Items, and Detail columns fit without clipping; selected, disabled, new, quantity, description, and action states remain distinct. The first live capture exposed missing glyph boxes for arrow controls. Revision `315badc` replaced every shared field-menu arrow legend and stat delta with supported ASCII text. Corrected capture: `/tmp/rpg-s1-items-corrected.png`, SHA-256 `7ac254e8c138e86f807972d941c2070e44f61bca46fe27c6f746461ffdceda56`. |
| Equipment slots | PASS | Both party portraits, five equipment slots, empty accessory state, derived totals, selected slot, and current item detail are readable without overlap. Capture: `/tmp/rpg-s1-equipment.png`, SHA-256 `b8952d1193558c4f4665bd990140bfc3b48b8824e8370587f09fb0d3c43c093d`. |
| Equipment picker | PASS | The picker retains member and slot context, reserves space for four candidates, and shows all four before/after stat cards with the changed value highlighted. Capture: `/tmp/rpg-s1-equipment-picker.png`, SHA-256 `3cb1ac290bac388ad5ae42fa49f7b9ad9895997f40cbed3d1c71acc6b7d93bc9`. |
| Spells | PASS | The party portraits, spellbook row, MP cost/readiness, spell type, target, description, and cast prompt are readable without clipping. Capture: `/tmp/rpg-s1-spells.png`, SHA-256 `532f0098fe2f7199ad6f2ef993fa9398fcff6072d46e60372fb7f5227e463a77`. |
| Teleport overlay | PASS | The focused destination overlay appears over the dimmed spellbook, retains caster/spell context, and presents the eligible Ardel destination and confirm/cancel controls. Capture: `/tmp/rpg-s1-spells-overlay.png`, SHA-256 `2dac361cf91ef47b2ca7d9703b3c688ec345257fd804e76d38e31f2f4a628acc`. |
| Save page one | PASS | The centered modal pins Autosave, shows six player slots, leaves all controls visible, and does not cover its border or footer. Capture: `/tmp/rpg-s1-save.png`, SHA-256 `f212a83e2a694032247e37aca3080a5e5bf1737076d9e48c37b285aa330d819c`. |
| Save metadata and paging | PASS | Moving to Slot 07 advances to page 02/17; the occupied slot shows map, protagonist, level, playtime, and saved-state badge while five empty rows remain readable. Capture: `/tmp/rpg-s1-save-slot7.png`, SHA-256 `8d9c17ad093664a93129967900f74e3969f1542fe8822ab685621556587c23fa`. |
| Save overwrite prompt | PASS | Enter on occupied Slot 07 opens a centered, focused overwrite modal with explicit destructive copy and confirm/cancel controls. Escape canceled the prompt; the isolated save was not changed. Capture: `/tmp/rpg-s1-save-overwrite.png`, SHA-256 `e100bb488b44a86fc4c71dcf5de0eb91f0c1a4434a3ad708f8c985e9993c20b3`. |
| Runtime diagnostics | PASS | The run produced no panic, missing asset, loader failure, or field-menu error. The only runtime notices were the known XSETTINGS warning, llvmpipe software-rendering warning, and one nonfatal unsupported ID3 metadata frame. The process exited by Ctrl-C after capture. |

## Automated companion

The final revision passed:

- `cargo test --all-targets`: 434 passed, 23 source-checkout-dependent ignored,
  0 failed;
- `cargo clippy --all-targets -- -D warnings`;
- `cargo fmt --all -- --check`; and
- `git diff --check`.

