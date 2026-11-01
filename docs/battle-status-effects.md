# Haste, Slow, and Regen

These temporary effects work on living party members and enemies. They are battle-only:
they are cleared by knockout and are never copied to the party's persistent field statuses.

| Effect | Initial scenario tuning | Timing |
| --- | --- | --- |
| Haste | 1.5 times turn-priority speed | Begins next round and lasts 3 full rounds |
| Slow | 0.5 times turn-priority speed | Begins next round and lasts 3 full rounds |
| Regen | 5% maximum HP per recipient turn | Heals at the end of the recipient's next 3 turns |

The engine rebuilds turn order between rounds, including revived actors and excluding
defeated actors. Every actor still gets one turn per round. Speed modifiers do not change
the DEX used for accuracy, critical chance, or fleeing. Existing tie-breaking stays stable.

Reapplying an effect replaces its strength and refreshes its duration; it does not stack.
Haste and Slow replace each other. Existing Cure removes all statuses, including these
effects. Constructs retain their immunity to harmful statuses, including Slow.

Regen rounds healing down to whole HP with a minimum of 1 HP, caps at maximum HP, and
never revives. Damage-over-time resolves before Regen; a lethal tick prevents healing.
Recovery appears as a green `+N REGEN` message. Status icons show remaining duration;
Haste and Slow count full rounds, while Regen counts recipient turns.
Enemy status cards form a row at the top center inside the sprite tile, above the enemy's
head, sized to 20% of its width (minimum 16 pixels). Icon cards have a black background
at 50% opacity; party icons retain their 24-pixel size.

## Scenario YAML

The engine implements timing and effect rules. The scenario defines targets, strengths,
durations, player skill MP costs and unlock levels, and enemy move weights. These values
belong on the individual skill or enemy move rather than in `data/balance.yaml`.

Player Haste is a `type: buff` skill with this `effect` mapping:

```yaml
effect:
  target: single_ally
  stat: speed
  modifier: 1.50
  duration_turns: 3
```

Player Slow uses `type: debuff`, `target: single_enemy`, and `modifier: 0.50`.
Regen is a `type: buff` skill:

```yaml
effect:
  target: single_ally
  regen_hp_pct: 0.05
  duration_turns: 3
```

Speed multipliers must be positive finite numbers. Values below 1 produce Slow;
values of 1 or above produce Haste. Regeneration must be greater than 0 and no more
than 1. Durations must be positive integers. Invalid or mixed mappings fail YAML loading.

Enemy ability moves accept the same effect mappings:

```yaml
- action: ability
  id: regen
  weight: 20
  effect: { target: self, regen_hp_pct: 0.05, duration_turns: 3 }
```

An explicitly configured effect move applies its effect without incidental attack damage.
Targets are relative to the caster: `party` means its allies and `all_enemies` means its
opponents. Existing enemy moves without an effect mapping retain their previous behavior.

## Initial content

- Rogue learns Haste at level 4 for 4 MP.
- Sorcerer learns Slow at level 5 for 5 MP.
- Cleric learns Regen at level 5 for 5 MP.
- Bat Demon can cast Haste on itself; Plague Doctor can cast Regen on itself.
- Pumpkin Jack can cast Slow on a party member.

The three skills reuse their matching status PNGs in the spellbook and battle ability list.

## Layout review fixtures

Run `target/debug/rpg-s1 play rusted_kingdoms --battle-fixture layout-three` to review
the currently authored maximum formation count, or use `--battle-fixture layout-five`
to review the largest renderer placement preset. Both are session-only stress tests:
they fill the party, use three or five copies of the largest authored enemy tile
(403px, size index AA), vary HP/MP meters, and fill every actor's three badge slots
with timed icons. All party abilities and three stocked battle items are available for
menu inspection. They do not change scenario encounter counts or the placement rules.

Current encounter YAML contains at most three enemies. The renderer defines fixed
staggered offsets for one through five enemies, then adjusts their positions to the
battle background's ground rectangle and sprite dimensions. Placement is deterministic;
it does not use random positions. The existing presets can overlap at large sprite sizes,
which these fixtures intentionally expose.
