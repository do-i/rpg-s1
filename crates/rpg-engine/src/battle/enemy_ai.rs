use crate::{
    encounter::BattleSide,
    gameplay_rng::GameplayRng,
    scenario_enemy::{
        AbilityHitOutcome, AccessoryBlockedTargetOverride, EnemyAiPattern, EnemyBehavior,
        EnemyDefaultTarget, EnemyMove, EnemyMoveCondition, EnemyOverrideTarget,
        EnemyTargetOverride,
    },
};

use super::{
    action::BattleEvent,
    model::{BattleCombatant, BattlePhase, BattleState, CombatantKey},
    status::{ActiveStatus, StatusEffect},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum EnemyAction {
    Attack,
    Ability { id: String, once: bool },
}

pub(super) fn resolve_enemy_turn(state: &mut BattleState, rng: &mut GameplayRng) {
    let Some(source_key) = state.active_key() else {
        return;
    };
    let Some(source) = state.actor(source_key).cloned() else {
        return;
    };
    if source.key.side != BattleSide::Enemy {
        return;
    }
    if state.all_defeated(BattleSide::Party) {
        state.phase = BattlePhase::Defeat;
        return;
    }

    let action = pick_enemy_action(&source, state, rng);
    let ability_id = match &action {
        EnemyAction::Attack => "",
        EnemyAction::Ability { id, .. } => id,
    };
    let targets = resolve_targets(&source, state, ability_id, rng);
    let Some(&first) = targets.first() else {
        state.phase = BattlePhase::Defeat;
        return;
    };
    match action {
        EnemyAction::Attack => {
            state.resolve_physical(source_key, first, rng);
        }
        EnemyAction::Ability { id, once } => {
            resolve_enemy_ability(state, &source, &id, &targets);
            if once {
                state.used_enemy_moves.insert((source_key, id));
            }
        }
    }
}

pub(super) fn pick_enemy_action(
    enemy: &BattleCombatant,
    state: &BattleState,
    rng: &mut GameplayRng,
) -> EnemyAction {
    let Some((ai, _)) = inline_behavior(enemy) else {
        return EnemyAction::Attack;
    };
    let eligible = ai
        .moves
        .iter()
        .filter(|movement| {
            !move_used(movement, enemy.key, state)
                && (ai.pattern == EnemyAiPattern::Random
                    || move_condition_matches(movement, enemy, state.turn_count))
        })
        .collect::<Vec<_>>();
    pick_weighted_move(&eligible, rng).map_or(EnemyAction::Attack, enemy_action)
}

fn move_used(movement: &EnemyMove, enemy: CombatantKey, state: &BattleState) -> bool {
    match movement {
        EnemyMove::Ability { id, once: true, .. } => {
            state.used_enemy_moves.contains(&(enemy, id.clone()))
        }
        _ => false,
    }
}

fn move_condition_matches(movement: &EnemyMove, enemy: &BattleCombatant, turn_count: u32) -> bool {
    let condition = match movement {
        EnemyMove::Attack { condition, .. } | EnemyMove::Ability { condition, .. } => condition,
    };
    match condition {
        None => true,
        Some(EnemyMoveCondition::HpBelow(value)) => {
            f64::from(enemy.health) / f64::from(enemy.max_health.max(1)) < value.hp_pct_below.get()
        }
        Some(EnemyMoveCondition::TurnModulo(value)) => {
            turn_count.is_multiple_of(value.turn_mod.every.get())
        }
    }
}

fn pick_weighted_move<'a>(moves: &[&'a EnemyMove], rng: &mut GameplayRng) -> Option<&'a EnemyMove> {
    let total = moves
        .iter()
        .map(|movement| u64::from(move_weight(movement)))
        .sum::<u64>();
    if total == 0 {
        return None;
    }
    let mut roll = rng.next_u64() % total;
    for &movement in moves {
        let weight = u64::from(move_weight(movement));
        if roll < weight {
            return Some(movement);
        }
        roll -= weight;
    }
    None
}

fn move_weight(movement: &EnemyMove) -> u32 {
    match movement {
        EnemyMove::Attack { weight, .. } | EnemyMove::Ability { weight, .. } => weight.get(),
    }
}

fn enemy_action(movement: &EnemyMove) -> EnemyAction {
    match movement {
        EnemyMove::Attack { .. } => EnemyAction::Attack,
        EnemyMove::Ability { id, once, .. } => EnemyAction::Ability {
            id: id.clone(),
            once: *once,
        },
    }
}

pub(super) fn resolve_targets(
    enemy: &BattleCombatant,
    state: &BattleState,
    ability_id: &str,
    rng: &mut GameplayRng,
) -> Vec<CombatantKey> {
    let targeting = inline_behavior(enemy).map(|(_, targeting)| targeting);
    let mode = targeting
        .and_then(|targeting| {
            targeting
                .overrides
                .iter()
                .find(|override_| override_ability(override_) == ability_id)
                .map(override_target)
                .or_else(|| Some(default_target(targeting.default)))
        })
        .unwrap_or(EnemyOverrideTarget::RandomAlive);
    if mode == EnemyOverrideTarget::SelfTarget {
        return vec![enemy.key];
    }
    let living = state
        .combatants
        .iter()
        .filter(|actor| actor.key.side == BattleSide::Party && actor.is_alive())
        .collect::<Vec<_>>();
    if mode == EnemyOverrideTarget::AllParty {
        return living.into_iter().map(|actor| actor.key).collect();
    }
    let taunters = living
        .iter()
        .copied()
        .filter(|actor| actor.is_taunting())
        .collect::<Vec<_>>();
    let pool = if taunters.is_empty() {
        &living
    } else {
        &taunters
    };
    let selected = match mode {
        EnemyOverrideTarget::LowestHp => pool.iter().min_by_key(|actor| actor.health).copied(),
        EnemyOverrideTarget::HighestHp => pool.iter().max_by_key(|actor| actor.health).copied(),
        EnemyOverrideTarget::HighestDex => pool.iter().max_by_key(|actor| actor.dexterity).copied(),
        EnemyOverrideTarget::RandomAlive => pool
            .get((rng.next_u64() % pool.len().max(1) as u64) as usize)
            .copied(),
        EnemyOverrideTarget::AllParty | EnemyOverrideTarget::SelfTarget => unreachable!(),
    };
    selected.into_iter().map(|actor| actor.key).collect()
}

fn inline_behavior(
    enemy: &BattleCombatant,
) -> Option<(
    &crate::scenario_enemy::EnemyAi,
    &crate::scenario_enemy::EnemyTargeting,
)> {
    match enemy.behavior.as_ref()? {
        EnemyBehavior::Inline { ai, targeting } => Some((ai, targeting)),
        EnemyBehavior::Referenced { .. } => None,
    }
}

fn default_target(target: EnemyDefaultTarget) -> EnemyOverrideTarget {
    match target {
        EnemyDefaultTarget::RandomAlive => EnemyOverrideTarget::RandomAlive,
        EnemyDefaultTarget::LowestHp => EnemyOverrideTarget::LowestHp,
        EnemyDefaultTarget::HighestHp => EnemyOverrideTarget::HighestHp,
    }
}

fn override_ability(target: &EnemyTargetOverride) -> &str {
    match target {
        EnemyTargetOverride::Standard(value) => &value.ability,
        EnemyTargetOverride::AccessoryBlocked(value) => &value.ability,
    }
}

fn override_target(target: &EnemyTargetOverride) -> EnemyOverrideTarget {
    match target {
        EnemyTargetOverride::Standard(value) => value.target,
        EnemyTargetOverride::AccessoryBlocked(value) => value.target,
    }
}

fn resolve_enemy_ability(
    state: &mut BattleState,
    source: &BattleCombatant,
    ability_id: &str,
    targets: &[CombatantKey],
) {
    let mut events = Vec::new();
    for &requested_target in targets {
        let target = state
            .actor(requested_target)
            .and_then(BattleCombatant::redirected_target)
            .filter(|redirect| {
                state
                    .actor(*redirect)
                    .is_some_and(BattleCombatant::is_alive)
            })
            .unwrap_or(requested_target);
        let Some(defender) = state.actor(target).cloned() else {
            continue;
        };
        if blocked_override(source, ability_id).is_some_and(|override_| {
            defender.accessory.as_deref() == Some(override_.blocked_by_accessory.as_str())
        }) {
            events.push(BattleEvent::EnemyAbilityBlocked {
                source: source.key,
                target,
            });
            continue;
        }
        let amount = if blocked_override(source, ability_id)
            .is_some_and(|override_| override_.on_hit == AbilityHitOutcome::HpTo1)
        {
            defender.health.saturating_sub(1)
        } else {
            defender
                .mitigated_damage((source.attack - defender.effective_defense()).max(1) as u32)
                .min(defender.health)
        };
        state
            .actor_mut(target)
            .expect("selected enemy ability target")
            .apply_resolved_damage(amount);
        events.push(BattleEvent::EnemyAbilityDamage {
            source: source.key,
            target,
            amount,
            knocked_out: amount >= defender.health,
        });
        if state.actor(target).is_some_and(BattleCombatant::is_alive) {
            for status in enemy_ability_statuses(ability_id, source.attack) {
                if state
                    .actor_mut(target)
                    .expect("selected enemy ability target")
                    .add_status(status)
                {
                    events.push(BattleEvent::StatusApplied {
                        source: source.key,
                        target,
                        status,
                    });
                }
            }
        }
    }
    state.feedback_events.extend(events.iter().copied());
    state.message = format!("{} uses {}!", source.name, display_ability_name(ability_id));
    state.transcript.push(format!(
        "ENEMY_ABILITY {} {} {}",
        source.id,
        ability_id,
        targets
            .iter()
            .map(|target| format!("{:?}:{}", target.side, target.index))
            .collect::<Vec<_>>()
            .join(",")
    ));
    state.phase = BattlePhase::Resolve;
}

fn enemy_ability_statuses(ability_id: &str, attack: i64) -> Vec<ActiveStatus> {
    match ability_id {
        "venom_bite" | "poison_bite" | "poison_tail" | "miasma" => {
            vec![ActiveStatus::damage_over_time(
                StatusEffect::Poison,
                None,
                (attack.max(0) as u32 / 10).max(1),
            )]
        }
        "plague_touch" => vec![
            ActiveStatus::damage_over_time(
                StatusEffect::Poison,
                None,
                (attack.max(0) as u32 / 10).max(1),
            ),
            ActiveStatus::persistent(StatusEffect::Silence),
        ],
        _ => Vec::new(),
    }
}

fn blocked_override<'a>(
    source: &'a BattleCombatant,
    ability_id: &str,
) -> Option<&'a AccessoryBlockedTargetOverride> {
    let (_, targeting) = inline_behavior(source)?;
    targeting
        .overrides
        .iter()
        .find_map(|override_| match override_ {
            EnemyTargetOverride::AccessoryBlocked(value) if value.ability == ability_id => {
                Some(value)
            }
            _ => None,
        })
}

fn display_ability_name(id: &str) -> String {
    id.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}
