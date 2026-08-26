use crate::{
    encounter::BattleSide,
    runtime_repository::RuntimeRepository,
    scenario_item::{ConsumableEffect, ConsumableItem, EnemyTrait, ItemDefinition, ItemUseContext},
};

use super::{
    ability::{ElementalAffinity, elemental_damage},
    action::BattleEvent,
    model::{BattlePhase, BattleState, CombatantKey, TargetGroup},
    status::StatusEffect,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ItemTargetPlan {
    pub(super) group: TargetGroup,
    pub(super) ko_eligible: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ItemUseError {
    UnknownSource,
    InvalidTarget,
    Unavailable,
    Unsupported,
}

pub(super) fn battle_item(definition: &ItemDefinition) -> Option<&ConsumableItem> {
    let ItemDefinition::Consumable(item) = definition else {
        return None;
    };
    (item.use_context.contains(&ItemUseContext::Battle)
        && matches!(
            item.effect,
            ConsumableEffect::FullRecovery(_)
                | ConsumableEffect::RestoreHp(_)
                | ConsumableEffect::RestoreMp(_)
                | ConsumableEffect::Revive(_)
                | ConsumableEffect::Cure(_)
                | ConsumableEffect::Throw(_)
        ))
    .then_some(item)
}

pub(super) fn target_plan(item: &ConsumableItem) -> ItemTargetPlan {
    match item.effect {
        ConsumableEffect::Throw(_) => ItemTargetPlan {
            group: TargetGroup::Enemy,
            ko_eligible: false,
        },
        ConsumableEffect::Revive(_) => ItemTargetPlan {
            group: TargetGroup::Ally,
            ko_eligible: true,
        },
        _ => ItemTargetPlan {
            group: TargetGroup::Ally,
            ko_eligible: false,
        },
    }
}

pub(super) fn resolve_item(
    state: &mut BattleState,
    source: CombatantKey,
    item: &ConsumableItem,
    target: CombatantKey,
    repository: &mut RuntimeRepository,
) -> Result<Vec<BattleEvent>, ItemUseError> {
    let (source_id, source_name) = state
        .actor(source)
        .filter(|actor| actor.is_alive())
        .map(|actor| (actor.id.clone(), actor.name.clone()))
        .ok_or(ItemUseError::UnknownSource)?;
    let plan = target_plan(item);
    let valid_target = state.actor(target).is_some_and(|actor| {
        actor.key.side
            == match plan.group {
                TargetGroup::Enemy => BattleSide::Enemy,
                TargetGroup::Ally => BattleSide::Party,
            }
            && actor.is_alive() != plan.ko_eligible
    });
    if !valid_target {
        return Err(ItemUseError::InvalidTarget);
    }
    if repository.item_count(&item.id) == 0 {
        return Err(ItemUseError::Unavailable);
    }

    let mut events = Vec::new();
    match &item.effect {
        ConsumableEffect::RestoreHp(effect) => {
            let amount = state
                .actor_mut(target)
                .expect("validated item target")
                .apply_heal(effect.restore_hp.get());
            events.push(BattleEvent::Heal {
                source,
                target,
                amount,
                revived: false,
            });
        }
        ConsumableEffect::RestoreMp(effect) => {
            let amount = state
                .actor_mut(target)
                .expect("validated item target")
                .restore_mana(effect.restore_mp.get());
            events.push(BattleEvent::ManaRestored {
                source,
                target,
                amount,
            });
        }
        ConsumableEffect::FullRecovery(_) => {
            let actor = state.actor_mut(target).expect("validated item target");
            let health = actor.apply_heal(actor.max_health);
            let mana = actor.restore_mana(actor.max_mana);
            events.push(BattleEvent::Heal {
                source,
                target,
                amount: health,
                revived: false,
            });
            events.push(BattleEvent::ManaRestored {
                source,
                target,
                amount: mana,
            });
        }
        ConsumableEffect::Revive(effect) => {
            let amount = state
                .actor_mut(target)
                .expect("validated item target")
                .revive(effect.revive_hp_pct.get());
            events.push(BattleEvent::Heal {
                source,
                target,
                amount,
                revived: true,
            });
        }
        ConsumableEffect::Cure(effect) => {
            for status in &effect.cure {
                let status = StatusEffect::from(*status);
                if state
                    .actor_mut(target)
                    .expect("validated item target")
                    .remove_status(status)
                {
                    events.push(BattleEvent::StatusCured {
                        source,
                        target,
                        effect: Some(status),
                    });
                }
            }
        }
        ConsumableEffect::Throw(effect) => {
            let defender = state.actor(target).cloned().expect("validated item target");
            let vulnerable = effect
                .bonus_vs
                .as_deref()
                .is_some_and(|traits| enemy_matches_any(defender.enemy_type, traits));
            let amount = defender
                .mitigated_damage(elemental_damage(
                    effect.damage.get(),
                    if vulnerable {
                        ElementalAffinity::Weak
                    } else {
                        ElementalAffinity::Neutral
                    },
                ))
                .min(defender.health);
            state
                .actor_mut(target)
                .expect("validated item target")
                .apply_resolved_damage(amount);
            events.push(BattleEvent::ItemDamage {
                source,
                target,
                element: effect.element,
                amount,
                knocked_out: amount >= defender.health,
            });
        }
        ConsumableEffect::BypassBarrier(_)
        | ConsumableEffect::Rest(_)
        | ConsumableEffect::Action(_) => return Err(ItemUseError::Unsupported),
    }
    repository
        .remove_item(&item.id, 1)
        .map_err(|_| ItemUseError::Unavailable)?;
    state.feedback_events.extend(events.iter().copied());
    state.message = format!("{source_name} uses {}!", item.name);
    state.transcript.push(format!(
        "ITEM {source_id} {} {:?}:{}",
        item.id, target.side, target.index
    ));
    state.phase = BattlePhase::Resolve;
    Ok(events)
}

fn enemy_matches_any(
    enemy_type: Option<crate::scenario_enemy::EnemyType>,
    traits: &[EnemyTrait],
) -> bool {
    traits.iter().any(|trait_| {
        matches!(
            (enemy_type, trait_),
            (
                Some(crate::scenario_enemy::EnemyType::Undead),
                EnemyTrait::Undead
            ) | (
                Some(crate::scenario_enemy::EnemyType::Demon),
                EnemyTrait::Demon
            )
        )
    })
}
