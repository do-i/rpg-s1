# M5 manual play evidence

Date: 2026-08-10

Final runtime revision: `4b984e0`

Pinned source revision: `08970359d6cb03586948625d29b0d3351dbbf785`

This check exercises the real Bevy/X11 window and production scenario package.
The first NPC/sign/recruitment observations were made at checkpoint `00751b2`.
The portal defect found during that run was repaired in `4b984e0`; the complete
house round trip, forest transition, treasure flow, and terminal review were
then replayed on that final runtime revision.

## Environment

- Linux/X11 on `DISPLAY=:0`
- Bevy 0.19 Vulkan backend
- Mesa llvmpipe software renderer (the best available adapter on this VM)
- 1280x766 game client area
- actual `cargo run` from the repository root with frame-held X11 key events

## Results

| Check | Result | Evidence |
| --- | --- | --- |
| World NPC publication | PASS | Ardel published its source-authored static, step, and wandering NPCs with real atlases. The run remained active for several minutes while NPCs moved and faced Aric; no overlap or asset error appeared. Initial World capture: `/tmp/rpg-s1-m5-world.png`, SHA-256 `88c652c6a1378eeb9932d194330d8c97950320464f204a7635cf49cf15c8783e`. |
| Nearest facing interaction and readable dialogue | PASS | A deliberately coalesced diagnostic movement input left Aric at spawn; Enter selected nearby wandering Pip and rendered only Pip's typewriting dialogue. Frame-held keys then placed Aric immediately right of Elise and facing left. Elise capture: `/tmp/rpg-s1-m5-elise-live.png`, SHA-256 `3536cc11cc889b14d92ec12ff4d3ae1a0a36915419332949475d024ce574666b`. |
| Elise recruitment | PASS | The five source lines advanced to completion. The `npc_elise_joined` presence change removed Elise from the map immediately and the overlay closed cleanly. Post-recruitment capture: `/tmp/rpg-s1-m5-elise-recruited.png`, SHA-256 `5d286a33febd6d9de1660edcbe4f79b375cbce04781845327d8a4bbeb8aae804`. The exact initial member state and idempotent second application are additionally pinned by `elise_join_effect_uses_source_initial_state_and_is_idempotent`. |
| Ardel sign | PASS | Aric faced configured sign tile `[16, 4]`; Enter loaded `sign_town_01_ardel` and displayed `Notice Board — Ardel Village`. Capture: `/tmp/rpg-s1-m5-sign-live.png`, SHA-256 `5fb7d31848eb8d7001a502ead09464d04ac86db37b61945a0dea772e02aeda05`. The run exposed a missing-glyph triangle in the hint; `4b984e0` replaced it with the font-supported `>` marker, visible in the final treasure captures. |
| Interior portal entry | PASS | Walking into Ardel's source house rectangle faded to black, replaced the town with the complete interior, spawned Aric at `[10, 11]`, and published Elder Maeve. Final-revision capture: `/tmp/rpg-s1-m5-fixed-house.png`, SHA-256 `147ee381159dfec924de5c5dfd036a8aa42aa0478460dc43d5abd033568998d2`. |
| Interior round trip | PASS after repair | The first run found that return target `[3, 4]` overlaps Ardel's entrance and bounced back after contact was cleared. `4b984e0` seeds destination overlap until the player exits. The replay returned to Ardel and remained there after fade-in: `/tmp/rpg-s1-m5-fixed-roundtrip.png`, SHA-256 `1981efb3e7f151f256375fca28c5244dca739ff19ba6532a2803a74900b17f09`. |
| Forest destination and map replacement | PASS | The source Ardel south portal loaded `zone_01_starting_forest`, replaced town art/NPCs/BGM selection, and spawned Aric at `[29, 1]`. Capture: `/tmp/rpg-s1-m5-forest-enter2.png`, SHA-256 `7614eb11d3e2fde22c06f700e642671bd4d3d06eb2d76d520e59118ab4237518`. |
| One-time treasure | PASS | At source box `forest_box_02`, the first Enter changed the atlas to its open frame and reported `Found potion ×2.` Capture: `/tmp/rpg-s1-m5-box-first.png`, SHA-256 `0c73868a4a4d6af96911685de19c969868e2743978892d7644414f3110021a67`. Reinteraction reported `This treasure box is already open.` with the open frame retained: `/tmp/rpg-s1-m5-box-repeat.png`, SHA-256 `c62ad16920ed7c3b105685ab4ed23ca5abd7f7dab85f34c02c3c35257f631185`. The exact once-only inventory delta is pinned by `source_forest_box_grants_once_and_reports_open_on_repeat`. |
| Interaction SFX routing | PASS at logical/runtime boundary | Confirm/dialogue, blocked, box, and cancel events resolved through the copied source SFX index; the final run emitted no missing-audio error. The focused test pins every runtime-emitted key and destination path. This is not a claim that a human listener or acoustic capture accepted relative loudness. |
| Runtime diagnostics | PASS | The final `cargo run` log contained no panic, fatal error, missing asset, loader failure, or audio-path error through the round trip and repeated treasure. The only warnings were the known XSETTINGS reload and llvmpipe software-rendering notices; Symphonia logged one nonfatal unsupported ID3 metadata frame. The process exited by Ctrl-C after evidence capture. |

## Automated companion

The final revision passed:

- `cargo test --bin rpg-s1`: 357 passed, 23 source-checkout-dependent ignored, 0 failed;
- `cargo clippy --all-targets -- -D warnings`;
- `cargo fmt -- --check`;
- `git diff --check`; and
- `scripts/check-ardel-screenshot.sh`, preserving decoded RGBA8 hash
  `122b47cda515c384cb40a531fb3e86666d28a71676e944639525c8fd4924934c`.

`cargo run -- validate-scenario` still reports the intentionally partial campaign
package (future-wave catalogs/assets are not copied yet). That pre-existing
incremental-port condition is not presented as a successful full-package
validation and remains separate from this playable M5 gate.
