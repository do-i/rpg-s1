use crate::{
    encounter::BattleSide,
    gameplay_rng::GameplayRng,
    scenario_class::{
        Ability, AbilityElement, AbilityKind, AbilitySideEffect, AttackRange, BuffEffect,
        EffectStat, EffectTarget, HealingMethod, PhysicalAbility, StatModifierEffect,
        UtilityAbility,
    },
    scenario_enemy::EnemyType,
};

use super::{
    action::{BattleAction, BattleEvent},
    model::{BattlePhase, BattleState, CombatantKey, TargetGroup},
    rules::{critical_damage, critical_hit_chance, roll_succeeds},
    status::{ActiveStatus, StatusEffect},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ElementalAffinity {
    Weak,
    Neutral,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the current scenario has no authored elemental resistance entries"
        )
    )]
    Resistant,
}

pub(super) fn elemental_damage(damage: u32, affinity: ElementalAffinity) -> u32 {
    match affinity {
        ElementalAffinity::Weak => damage.saturating_mul(3) / 2,
        ElementalAffinity::Neutral => damage,
        ElementalAffinity::Resistant => (damage / 2).max(1),
    }
}

pub(super) fn elemental_affinity(
    element: AbilityElement,
    enemy_type: Option<EnemyType>,
) -> ElementalAffinity {
    if element == AbilityElement::Holy
        && matches!(enemy_type, Some(EnemyType::Undead | EnemyType::Demon))
    {
        ElementalAffinity::Weak
    } else {
        ElementalAffinity::Neutral
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum AbilityTargetPlan {
    Select {
        group: TargetGroup,
        ko_eligible: bool,
    },
    All {
        side: BattleSide,
        ko_eligible: bool,
    },
    SelfTarget,
}

pub(super) fn battle_ability(ability: &Ability) -> bool {
    !matches!(
        &ability.kind,
        AbilityKind::Utility(UtilityAbility::Steal { .. } | UtilityAbility::Warp { .. })
    )
}

pub(super) fn target_plan(ability: &Ability) -> AbilityTargetPlan {
    use crate::scenario_class::AbilityTarget;

    let target = match &ability.kind {
        AbilityKind::Physical(value) => value.target,
        AbilityKind::Spell(value) => value.target,
        AbilityKind::Heal(value) => value.target,
        AbilityKind::Utility(UtilityAbility::RemoveStatus { target, .. })
        | AbilityKind::Utility(UtilityAbility::Steal { target, .. })
        | AbilityKind::Utility(UtilityAbility::Warp { target, .. }) => *target,
        AbilityKind::Buff(value) => return effect_target_plan(buff_target(&value.effect)),
        AbilityKind::Debuff(value) => return effect_target_plan(value.effect.target),
    };
    let revive = matches!(
        &ability.kind,
        AbilityKind::Heal(value) if matches!(value.healing, HealingMethod::Revive { .. })
    );
    match target {
        AbilityTarget::SingleEnemy => AbilityTargetPlan::Select {
            group: TargetGroup::Enemy,
            ko_eligible: false,
        },
        AbilityTarget::AllEnemies | AbilityTarget::GroupEnemies => AbilityTargetPlan::All {
            side: BattleSide::Enemy,
            ko_eligible: false,
        },
        AbilityTarget::SingleAlly => AbilityTargetPlan::Select {
            group: TargetGroup::Ally,
            ko_eligible: revive,
        },
        AbilityTarget::SelfTarget => AbilityTargetPlan::SelfTarget,
        AbilityTarget::AllAllies => AbilityTargetPlan::All {
            side: BattleSide::Party,
            ko_eligible: revive,
        },
    }
}

fn buff_target(effect: &BuffEffect) -> EffectTarget {
    match effect {
        BuffEffect::StatModifier(value) => value.target,
        BuffEffect::Aggro(value) => value.target,
        BuffEffect::RedirectDamage(value) => value.target,
        BuffEffect::DamageReduction(value) => value.target,
    }
}

fn effect_target_plan(target: EffectTarget) -> AbilityTargetPlan {
    match target {
        EffectTarget::Party => AbilityTargetPlan::All {
            side: BattleSide::Party,
            ko_eligible: false,
        },
        EffectTarget::AllEnemies => AbilityTargetPlan::All {
            side: BattleSide::Enemy,
            ko_eligible: false,
        },
        EffectTarget::SingleEnemy => AbilityTargetPlan::Select {
            group: TargetGroup::Enemy,
            ko_eligible: false,
        },
        EffectTarget::SelfTarget => AbilityTargetPlan::SelfTarget,
        EffectTarget::SingleAlly => AbilityTargetPlan::Select {
            group: TargetGroup::Ally,
            ko_eligible: false,
        },
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum AbilityError {
    UnknownCaster,
    UnknownAbility,
    InvalidTarget,
    InsufficientMana,
    Silenced,
    Unsupported,
}

pub(super) fn resolve_ability(
    state: &mut BattleState,
    caster_key: CombatantKey,
    ability_index: usize,
    targets: &[CombatantKey],
    rng: &mut GameplayRng,
) -> Result<Vec<BattleEvent>, AbilityError> {
    let caster = state
        .actor(caster_key)
        .cloned()
        .ok_or(AbilityError::UnknownCaster)?;
    let ability = caster
        .abilities
        .get(ability_index)
        .cloned()
        .ok_or(AbilityError::UnknownAbility)?;
    if !battle_ability(&ability) {
        return Err(AbilityError::Unsupported);
    }
    if caster.is_silenced() {
        return Err(AbilityError::Silenced);
    }
    if targets.is_empty() || !targets_match_plan(state, caster_key, &ability, targets) {
        return Err(AbilityError::InvalidTarget);
    }
    state
        .actor_mut(caster_key)
        .and_then(|caster| caster.spend_mana(ability.mp_cost))
        .ok_or(AbilityError::InsufficientMana)?;

    let mut events = Vec::new();
    for &target in targets {
        match &ability.kind {
            AbilityKind::Physical(value) => {
                resolve_physical_ability(state, caster_key, target, value, rng, &mut events)?
            }
            AbilityKind::Spell(value) => {
                resolve_spell(state, &caster, caster_key, target, value, rng, &mut events)?;
            }
            AbilityKind::Heal(value) => {
                let amount = match value.healing {
                    HealingMethod::Restore {
                        coefficient,
                        max_hp_percent,
                    } => {
                        let target_actor =
                            state.actor(target).ok_or(AbilityError::InvalidTarget)?;
                        let raw = max_hp_percent.map_or_else(
                            || (caster.magic_resistance as f64 * coefficient.get()) as u32,
                            |percent| (f64::from(target_actor.max_health) * percent.get()) as u32,
                        );
                        state
                            .actor_mut(target)
                            .expect("validated target")
                            .apply_heal(raw)
                    }
                    HealingMethod::Revive { hp_percent } => state
                        .actor_mut(target)
                        .expect("validated target")
                        .revive(hp_percent.get()),
                };
                events.push(BattleEvent::Heal {
                    source: caster_key,
                    target,
                    amount,
                    revived: matches!(value.healing, HealingMethod::Revive { .. }) && amount > 0,
                });
            }
            AbilityKind::Buff(value) => {
                apply_buff(state, caster_key, target, &value.effect, &mut events)
            }
            AbilityKind::Debuff(value) => {
                apply_modifier(state, caster_key, target, &value.effect, &mut events)
            }
            AbilityKind::Utility(UtilityAbility::RemoveStatus { .. }) => {
                if state
                    .actor_mut(target)
                    .expect("validated target")
                    .clear_statuses()
                    > 0
                {
                    events.push(BattleEvent::StatusCured {
                        source: caster_key,
                        target,
                        effect: None,
                    });
                }
            }
            AbilityKind::Utility(UtilityAbility::Steal { .. } | UtilityAbility::Warp { .. }) => {
                return Err(AbilityError::Unsupported);
            }
        }
    }
    state.feedback_events.extend(events.iter().copied());
    state.message = format!("{} uses {}!", caster.name, ability.name);
    state.transcript.push(format!(
        "ABILITY {} {} {}",
        caster.id,
        ability.id,
        targets
            .iter()
            .map(|target| format!("{:?}:{}", target.side, target.index))
            .collect::<Vec<_>>()
            .join(",")
    ));
    state.phase = BattlePhase::Resolve;
    Ok(events)
}

fn resolve_spell(
    state: &mut BattleState,
    caster: &super::model::BattleCombatant,
    source: CombatantKey,
    target: CombatantKey,
    spell: &crate::scenario_class::SpellAbility,
    rng: &mut GameplayRng,
    events: &mut Vec<BattleEvent>,
) -> Result<(), AbilityError> {
    let target_actor = state
        .actor(target)
        .cloned()
        .ok_or(AbilityError::InvalidTarget)?;
    let raw = (caster.magic_resistance as f64 * spell.spell_coeff.get()) as i64
        - target_actor.effective_magic_resistance();
    let amount = target_actor
        .mitigated_damage(elemental_damage(
            raw.max(1) as u32,
            elemental_affinity(spell.element, target_actor.enemy_type),
        ))
        .min(target_actor.health);
    state
        .actor_mut(target)
        .expect("validated target")
        .apply_resolved_damage(amount);
    events.push(BattleEvent::MagicDamage {
        source,
        target,
        element: spell.element,
        amount,
        knocked_out: amount >= target_actor.health,
    });
    apply_side_effects(state, source, target, &spell.side_effects, rng, events);
    Ok(())
}

fn targets_match_plan(
    state: &BattleState,
    caster: CombatantKey,
    ability: &Ability,
    targets: &[CombatantKey],
) -> bool {
    match target_plan(ability) {
        AbilityTargetPlan::SelfTarget => targets == [caster],
        AbilityTargetPlan::Select { group, ko_eligible } => {
            targets.len() == 1 && target_eligible(state, targets[0], group, ko_eligible)
        }
        AbilityTargetPlan::All { side, ko_eligible } => {
            let expected = state
                .combatants
                .iter()
                .filter(|actor| actor.key.side == side && actor.is_alive() != ko_eligible)
                .map(|actor| actor.key)
                .collect::<Vec<_>>();
            targets == expected
        }
    }
}

fn target_eligible(
    state: &BattleState,
    target: CombatantKey,
    group: TargetGroup,
    ko_eligible: bool,
) -> bool {
    let side = match group {
        TargetGroup::Enemy => BattleSide::Enemy,
        TargetGroup::Ally => BattleSide::Party,
    };
    state
        .actor(target)
        .is_some_and(|actor| actor.key.side == side && actor.is_alive() != ko_eligible)
}

fn resolve_physical_ability(
    state: &mut BattleState,
    caster: CombatantKey,
    target: CombatantKey,
    ability: &PhysicalAbility,
    rng: &mut GameplayRng,
    events: &mut Vec<BattleEvent>,
) -> Result<(), AbilityError> {
    let target = state
        .actor(target)
        .and_then(super::model::BattleCombatant::redirected_target)
        .filter(|redirect| {
            state
                .actor(*redirect)
                .is_some_and(super::model::BattleCombatant::is_alive)
        })
        .unwrap_or(target);
    let source = state
        .actor(caster)
        .cloned()
        .ok_or(AbilityError::UnknownCaster)?;
    for _ in 0..ability.hits.map_or(1, |hits| hits.get()) {
        let defender = state
            .actor(target)
            .cloned()
            .ok_or(AbilityError::InvalidTarget)?;
        if !defender.is_alive() {
            break;
        }
        let base = (source.effective_attack() - defender.effective_defense()).max(1) as u32;
        let mut amount = (f64::from(base) * ability.damage_coeff.get()) as u32;
        if ability.attack_range == AttackRange::Melee
            && source.row == crate::scenario_party::PartyRow::Back
        {
            amount = (amount / 2).max(1);
        }
        if defender.key.side == BattleSide::Party
            && defender.row == crate::scenario_party::PartyRow::Back
        {
            amount = (amount / 2).max(1);
        }
        let critical = ability.guaranteed_crit == Some(true)
            || roll_succeeds(rng, critical_hit_chance(source.dexterity));
        if critical {
            amount = critical_damage(amount);
        }
        if ability.instant_kill.as_ref().is_some_and(|kill| {
            !defender.boss
                && !defender
                    .immunities
                    .contains(&crate::scenario_enemy::EnemyImmunity::InstantKill)
                && roll_succeeds(rng, kill.chance.get())
        }) {
            amount = defender.health;
        } else {
            amount = defender.mitigated_damage(amount);
        }
        amount = amount.min(defender.health);
        state
            .actor_mut(target)
            .expect("validated target")
            .apply_resolved_damage(amount);
        events.push(BattleEvent::Damage {
            action: BattleAction::Physical {
                attacker: caster,
                target,
            },
            amount,
            critical,
            knocked_out: amount >= defender.health,
        });
    }
    if let Some(effect) = &ability.effect {
        apply_modifier(state, caster, target, effect, events);
    }
    Ok(())
}

fn apply_side_effects(
    state: &mut BattleState,
    source: CombatantKey,
    target: CombatantKey,
    effects: &[AbilitySideEffect],
    rng: &mut GameplayRng,
    events: &mut Vec<BattleEvent>,
) {
    for effect in effects {
        let (chance, status) = match effect {
            AbilitySideEffect::Burn {
                chance,
                duration_turns,
                ..
            } => {
                let damage = state
                    .actor(source)
                    .map_or(1, |actor| (actor.attack.max(0) as u32 / 10).max(1));
                (
                    chance.get(),
                    ActiveStatus::damage_over_time(
                        StatusEffect::Burn,
                        Some(duration_turns.get()),
                        damage,
                    ),
                )
            }
            AbilitySideEffect::Freeze {
                chance,
                duration_turns,
            } => (
                chance.get(),
                ActiveStatus::timed(StatusEffect::Freeze, duration_turns.get()),
            ),
            AbilitySideEffect::Stun {
                chance,
                duration_turns,
            } => (
                chance.get(),
                ActiveStatus::timed(StatusEffect::Stun, duration_turns.get()),
            ),
            AbilitySideEffect::Silence {
                chance,
                duration_turns,
            } => (
                chance.get(),
                ActiveStatus::timed(StatusEffect::Silence, duration_turns.get()),
            ),
            AbilitySideEffect::Knockback {
                chance,
                atk_modifier,
                duration_turns,
            } => (
                chance.get(),
                ActiveStatus::modifier(
                    StatusEffect::Knockback,
                    duration_turns.get(),
                    atk_modifier.get(),
                ),
            ),
        };
        if roll_succeeds(rng, chance)
            && state
                .actor_mut(target)
                .is_some_and(|actor| actor.is_alive() && actor.add_status(status))
        {
            events.push(BattleEvent::StatusApplied {
                source,
                target,
                status,
            });
        }
    }
}

fn apply_buff(
    state: &mut BattleState,
    source: CombatantKey,
    target: CombatantKey,
    effect: &BuffEffect,
    events: &mut Vec<BattleEvent>,
) {
    let status = match effect {
        BuffEffect::StatModifier(value) => status_from_modifier(value),
        BuffEffect::Aggro(value) => {
            ActiveStatus::timed(StatusEffect::Taunt, value.duration_turns.get())
        }
        BuffEffect::RedirectDamage(value) => {
            ActiveStatus::redirect(value.duration_turns.get(), source)
        }
        BuffEffect::DamageReduction(value) => {
            ActiveStatus::reduction(value.duration_turns.get(), value.damage_reduction.get())
        }
    };
    apply_status(state, source, target, status, events);
}

fn apply_modifier(
    state: &mut BattleState,
    source: CombatantKey,
    target: CombatantKey,
    effect: &StatModifierEffect,
    events: &mut Vec<BattleEvent>,
) {
    apply_status(state, source, target, status_from_modifier(effect), events);
}

fn status_from_modifier(effect: &StatModifierEffect) -> ActiveStatus {
    let status = match effect.stat {
        EffectStat::Attack => StatusEffect::AttackModifier,
        EffectStat::Defense => StatusEffect::DefenseModifier,
        EffectStat::MagicResistance => StatusEffect::MagicResistanceModifier,
        EffectStat::HitChance => StatusEffect::HitChanceModifier,
    };
    ActiveStatus::modifier(status, effect.duration_turns.get(), effect.modifier.get())
}

fn apply_status(
    state: &mut BattleState,
    source: CombatantKey,
    target: CombatantKey,
    status: ActiveStatus,
    events: &mut Vec<BattleEvent>,
) {
    if state
        .actor_mut(target)
        .is_some_and(|actor| actor.is_alive() && actor.add_status(status))
    {
        events.push(BattleEvent::StatusApplied {
            source,
            target,
            status,
        });
    }
}
