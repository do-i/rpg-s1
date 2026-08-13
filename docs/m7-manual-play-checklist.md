# M7 Save, Load, and Recovery Manual Play Checklist

This Gate 7 check exercises the real Bevy/X11 window, production scenario,
native filesystem boundary, process restart, and standalone Python-save
converter. Focused Rust tests remain the authoritative exhaustive state and
failure-injection proof.

## Environment

- Linux/X11 on `DISPLAY=:0`
- Bevy Vulkan backend using Mesa llvmpipe
- isolated native roots under `/tmp`
- frame-held X11 key events, not direct ECS resource mutation
- production `rusted_kingdoms` package

## 2026-08-12 Gate 7 result

- [x] A fresh profile showed Load Game disabled.
- [x] New Game completed through all 15 intro pages into playable Ardel.
- [x] Aric moved one tile to `[12, 8]` facing right before saving.
- [x] Field Save showed Slot 01 plus readable empty Slot 02-07 rows.
- [x] Saving Slot 01 produced a 1,001-byte native v1 envelope with scenario
  identity, timestamp, complete payload, and Ardel metadata.
- [x] The saved row refreshed to `Aric Lv1`, 52 seconds, and
  `town_01_ardel` without reopening the application.
- [x] Field Quit required confirmation and returned to one clean title screen.
- [x] Load Game became enabled after native-slot discovery.
- [x] The first process quit through the title command with exit status 0.
- [x] A new process discovered Slot 01, selected it as `[LATEST]`, and loaded
  Ardel at `[12, 8]` facing right with the same saved payload.
- [x] Reopening occupied Slot 01 displayed an explicit overwrite question;
  Escape canceled it and the slot SHA-256 remained
  `dff0099d61fb66f45df1eaa0c766a4b1d519541c8747b8093ea4443bff9a87c3`.
- [x] A deliberately malformed Slot 02 rendered `[CORRUPT]` and its parse
  reason; an old-format Slot 03 rendered `[INCOMPATIBLE]`; Slot 01 remained
  visible and independently loadable.
- [x] The standalone CLI imported the checked-in serializer-produced Python
  fixture into Slot 07, and a clean game launch loaded its Starting Forest
  position plus the expected two inventory stacks (`mc_s` x2, Potion x7).
- [x] Repeating the CLI import without `--replace` failed actionably and left
  the existing destination unchanged.

Captured inspection images and isolated live saves were temporary `/tmp`
artifacts and are not committed. Automated proof covers full persisted-state equality,
quest derivation, RNG/playtime restoration, schema goldens, harmless unknown
fields, old/unversioned/wrong-scenario rejection, all four slot states,
interrupted-write byte preservation, atomic replacement cleanup, overwrite
policy, checksum rules, content-reference validation, deterministic imported
RNG independent of YAML formatting, pinned older-field defaults, source-path
protection, verified import backup, and native post-conversion round trip.
