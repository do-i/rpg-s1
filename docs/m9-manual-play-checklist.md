# M9 Minimum Complete Battle Loop Manual Play Checklist

This Gate 9 check exercises the real Bevy/X11 production scene rather than a
synthetic battle-only harness. Focused Rust tests remain the exhaustive proof
for formulas, phase boundaries, selectors, deterministic ordering, flee rate,
KO skipping, and transcript stability.

## Environment

- Linux/X11 on `DISPLAY=:0`
- Bevy Vulkan backend using Mesa llvmpipe
- production `rusted_kingdoms` package
- isolated native save roots under `/tmp`
- frame-held X11 key events through the normal action-input systems
- pinned Python source revision
  `08970359d6cb03586948625d29b0d3351dbbf785`

## 2026-08-16 Gate 9 result

- [x] A visible Starting Forest enemy launched a configured Goblin formation
  over the production forest background with battle BGM.
- [x] Enemy sprites, enemy HP and target state, party HP/MP/row, active-member
  marker, command availability, and KO labels were visible in the real window.
- [x] Attack selected a living target, resolved deterministic hit/damage, and
  advanced through party and enemy turns. KO actors were skipped.
- [x] Escape/Run produced visible failure feedback; confirming it consumed the
  active actor's turn without leaving battle.
- [x] A later Run succeeded and restored the captured player location, world
  visuals/BGM, and enemy pool. The engaged enemy remained inactive, matching
  the source's separation/safety policy.
- [x] Re-entering a regular encounter and attacking to victory displayed the
  Victory phase, restored the world once, retained battle HP damage, and kept
  the engaged enemy inactive.
- [x] An isolated 1-HP, low-attack copy of the same imported save let the real
  enemy resolver KO both members. Game Over appeared only after the final KO.
- [x] Retry rebuilt the original battle entry with both members at their
  pre-battle 1 HP and the enemy at full HP; it granted no outcome rewards.
- [x] Load Game left Game Over and opened the native title load picker directly
  on the valid isolated slot. The Title route shares the tested cleanup and
  state-transition reducer and discards the transient session.

## Temporary evidence

The screenshots are intentionally not committed:

- `/tmp/rpg-s1-m9-state.png` — production two-Goblin battle UI.
- `/tmp/rpg-s1-m9-flee-result.png` — failed flee feedback.
- `/tmp/rpg-s1-m9-flee-confirmed.png` — restored world after successful flee.
- `/tmp/rpg-s1-m9-final-ko.png` — victory phase with enemy KO presentation.
- `/tmp/rpg-s1-m9-defeat-progress3.png` — final party KO in Resolve.
- `/tmp/rpg-s1-m9-game-over.png` — Retry/Load/Title Game Over screen.
- `/tmp/rpg-s1-m9-retry.png` — reconstructed pre-battle state after Retry.
- `/tmp/rpg-s1-m9-gameover-load.png` — native load picker opened from Game Over.

The ordinary suite passed 418 tests with the 23 opt-in pinned-source audits
skipped; a separate run supplied every pinned catalog path and passed all 23
audits. Strict Clippy with warnings denied, formatting, `git diff --check`, and
the Ardel RGBA screenshot oracle also passed (hash
`85ce229c04604258debbad65643ebcc62177f084727178d29b968baeb35b2012`).
Full-package validation remains scoped separately because later campaign waves
and Milestone 10+ content are still intentionally absent.
