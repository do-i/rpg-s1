# Enemy battler tiles

Each tile is 256×256. Poses share one scale. Feet sit 4 pixels above the bottom of the cell.

The v1 battler is still the LPC sheet: 9 columns, idle tile 18
(`SPELLCAST_ROW * LPC_COLUMNS` in `crates/rpg-engine/src/battle/ui.rs`).
These sheets do not use that layout. A v2 status sheet, when present, lives at
`media/sprites/enemies/v2/{id}-status.tsx`.

## Status sheet

The status sheet is 1024×1024, transparent, with no labels and no grid lines.
It has 4 columns and 16 tiles.

```text
index = row * 4 + col
origin = (col * 256, row * 256)
```

| Index | Col | Row | Pose | Engine value |
| ---: | ---: | ---: | --- | --- |
| 0 | 0 | 0 | Ready | no status |
| 1 | 1 | 0 | Poison | `StatusEffect::Poison` |
| 2 | 2 | 0 | Sleep | `StatusEffect::Sleep` |
| 3 | 3 | 0 | Stun | `StatusEffect::Stun` |
| 4 | 0 | 1 | Silence | `StatusEffect::Silence` |
| 5 | 1 | 1 | Burn | `StatusEffect::Burn` |
| 6 | 2 | 1 | Freeze | `StatusEffect::Freeze` |
| 7 | 3 | 1 | Knockback | `StatusEffect::Knockback` |
| 8 | 0 | 2 | Taunt | `StatusEffect::Taunt` |
| 9 | 1 | 2 | Guard | `StatusEffect::DamageReduction` |
| 10 | 2 | 2 | Cover | `StatusEffect::RedirectDamage` |
| 11 | 3 | 2 | Haste | `StatusEffect::Haste` |
| 12 | 0 | 3 | Slow | `StatusEffect::Slow` |
| 13 | 1 | 3 | Regen | `StatusEffect::Regen` |
| 14 | 2 | 3 | KO | knocked out, not a `StatusEffect` |
| 15 | 3 | 3 | empty | unused |

`AttackModifier`, `DefenseModifier`, `MagicResistanceModifier`, and
`HitChanceModifier` stay on tile 0. Their badges already carry the sign.

One body shows one tile. When several statuses are active, use the first
match: KO if health is 0, then Freeze, Sleep, Stun, Burn, Poison, Silence,
Knockback, Cover, Taunt, Guard, Slow, Haste, Regen. Otherwise use tile 0.

## Attack sheet

Attack poses are a separate sheet. The status grid stays 15 poses plus the empty tile.

The attack sheet is 512×256, transparent, with no labels and no grid lines.
It has 2 columns and 1 row. `index = col`. The origin of a tile is
`(col * 256, 0)`.

| Index | Pose | When it plays |
| ---: | --- | --- |
| 0 | Thrust | basic attack, `AttackKind::Thrust` |
| 1 | Lunge | the lunge ability, `AttackKind::Spellcast` |

The thrust is a one-hand downward chop. The lunge is a wide step with both hands on the sword, the point toward the viewer's right.

## Paint notes for this test

Poison's lower face went green. Sleep is a kneel with both eyes shut. KO is
on his side with the sword on the ground. Wide poses set the shared scale, so
the standing body is smaller than the ready crop.
