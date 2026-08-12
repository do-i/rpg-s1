# M6 Field Menu Manual Play Checklist

This Gate 6 check exercises the real Bevy/X11 window and the production
scenario package. It complements the focused Rust domain fixtures; it does not
replace them.

## Environment

- Linux/X11 on `DISPLAY=:0`
- Bevy Vulkan backend using Mesa llvmpipe
- `cargo run` from the repository root
- frame-held X11 key events (not synthetic direct resource mutation)

## 2026-08-12 result

- [x] Title New Game, name confirmation, and all 13 intro pages reached World.
- [x] `M` opened the top-level field menu only after the World fade completed.
- [x] The menu showed Aric's live name, level, HP/MP, row, and shared GP.
- [x] Enter opened Status; its base/derived stats and five equipment slots were
  readable and agreed with the runtime state.
- [x] The field-menu backdrop filled and darkened the complete fixed canvas;
  the World stayed visible only as the intended subdued background.
- [x] `M`/Escape closed or backed out without moving Aric; World input resumed.
- [x] Direct `I` and `S` shortcuts opened Items and Status from World.
- [x] The Items screen displayed every tab and accurate context controls.
- [x] Equipment and Spells were reachable from the main menu and used the same
  party/repository state shown by Status and Items.

Captured inspection images were temporary `/tmp` artifacts and are not
committed. Automated proof for empty/short/scrolling item lists, hidden/new
semantics, discard bounds/rejections, item effects, equipment atomicity,
ability gates, healing cost/caps, and teleport eligibility lives in the
focused `field_menu_domain` and `field_menu` tests.
