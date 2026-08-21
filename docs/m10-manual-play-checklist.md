# M10 Full Combat and Progression Manual Play Checklist

This Gate 10 check exercises the real Bevy/X11 production scene rather than a
synthetic battle-only harness. Focused Rust tests and the pinned Python oracle
remain the exhaustive proof for formulas, abilities, items, statuses, enemy AI,
boss restrictions, rewards, loot, and multi-level progression.

## Environment

- Linux/X11 on `DISPLAY=:0`
- Bevy Vulkan backend using Mesa llvmpipe
- production `rusted_kingdoms` package
- isolated native save roots under `/tmp`
- sound disabled with `RPG_S1_MUTE_AUDIO=1`
- frame-held X11 key events through the normal action-input systems
- pinned Python source revision
  `08970359d6cb03586948625d29b0d3351dbbf785`

## 2026-08-21 Gate 10 result

- [x] A visible Starting Forest Goblin launched the production battle scene
  through normal world contact.
- [x] A physical attack resolved through the normal turn loop, displayed its
  damage feedback, removed the defeated enemy, and entered Victory.
- [x] The reward screen displayed EXP, deterministic loot quantities, one
  level for each living party member, stat increases, restored maxima, and the
  newly learned Power Strike ability.
- [x] Party HP/MP meters immediately reflected the post-level maxima rather
  than retaining stale pre-reward values.
- [x] Advancing from rewards restored the world once without regranting the
  outcome.
- [x] Contact with the next visible enemy opened another production battle and
  Power Strike was immediately present in Aric's command list.
- [x] Focused production-catalog fixtures cover the complete imported enemy
  action schema and boss-only restrictions. The representative physical,
  spell, item, status, boss, and reward transcript exactly matches the pinned
  Python implementation.

## Temporary evidence

The screenshots are intentionally not committed:

- `/tmp/rpg-s1-m10-live-battle.png` — production Goblin battle.
- `/tmp/rpg-s1-m10-live-hit.png` — resolved hit feedback and enemy removal.
- `/tmp/rpg-s1-m10-victory.png` — Victory phase.
- `/tmp/rpg-s1-m10-rewards-fixed.png` — corrected reward layout, stat deltas,
  level gains, loot, learned ability, and updated party meters.
- `/tmp/rpg-s1-m10-unlocked-ability.png` — Power Strike available in the next
  production battle.

The ordinary suite passed 482 tests with the 24 opt-in pinned-source audits
skipped. A separate run supplied the clean pinned checkout plus materialized
Git-LFS catalog assets and passed all 24 audits, including the exact Python
battle transcript. Strict Clippy with warnings denied, formatting,
`git diff --check`, and the Ardel RGBA screenshot oracle also passed (hash
`85ce229c04604258debbad65643ebcc62177f084727178d29b968baeb35b2012`).
Full-package validation remains scoped separately because later campaign waves
and Milestone 11+ content are intentionally absent.
