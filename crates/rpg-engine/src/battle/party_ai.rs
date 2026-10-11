//! Turn planning for party members set to Auto on the field menu's Battle page.
//!
//! The planner only reads the battle and returns an [`AutoAction`]; the battle UI resolves it
//! through the same paths a manual command takes, so an auto turn can never do anything the
//! player could not. `Smart` draws on the gameplay RNG for its thresholds and tie-breaks, so it
//! does not settle into one fixed rotation.

use crate::{
    battle_tactics::AutoCommand,
    encounter::BattleSide,
    gameplay_rng::GameplayRng,
    scenario_class::{Ability, AbilityKind, HealingMethod},
    scenario_item::{ConsumableEffect, ConsumableItem},
};

use super::{
    ability::{AbilityTargetPlan, battle_ability, target_plan},
    model::{BattleCombatant, BattleState, CombatantKey, TargetGroup},
    rules::{physical_damage, roll_succeeds},
};

/// What an auto member does this turn.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum AutoAction {
    Attack(CombatantKey),
    Ability {
        index: usize,
        targets: Vec<CombatantKey>,
    },
    Item {
        id: String,
        target: CombatantKey,
    },
    Flee,
}

/// Rounds of plain attacks the party would need, past which a hopeless fight is worth fleeing.
const HOPELESS_ROUNDS: f64 = 8.0;

pub(super) fn plan_auto_action(
    state: &BattleState,
    actor: CombatantKey,
    command: AutoCommand,
    items: &[&ConsumableItem],
    rng: &mut GameplayRng,
) -> Option<AutoAction> {
    let me = state.actor(actor)?;
    let fallback = || attack_weakest(state).map(AutoAction::Attack);
    match command {
        AutoCommand::Attack => fallback(),
        AutoCommand::Spell => best_offense(state, me, rng).or_else(fallback),
        AutoCommand::Smart => smart(state, me, items, rng).or_else(fallback),
    }
}

fn smart(
    state: &BattleState,
    me: &BattleCombatant,
    items: &[&ConsumableItem],
    rng: &mut GameplayRng,
) -> Option<AutoAction> {
    if outmatched(state) && roll_succeeds(rng, 0.7) {
        return Some(AutoAction::Flee);
    }
    if let Some(action) = revive(state, me, items, rng) {
        return Some(action);
    }
    if let Some(action) = heal(state, me, items, rng) {
        return Some(action);
    }
    if let Some(action) = restore_own_mana(me, items, rng) {
        return Some(action);
    }
    let mana_ratio = ratio(me.mana, me.max_mana);
    let cast_chance = if mana_ratio > 0.4 { 0.6 } else { 0.2 };
    if roll_succeeds(rng, cast_chance)
        && let Some(action) = best_offense(state, me, rng)
    {
        return Some(action);
    }
    if roll_succeeds(rng, 0.7) {
        attack_weakest(state).map(AutoAction::Attack)
    } else {
        random_living(state, BattleSide::Enemy, rng).map(AutoAction::Attack)
    }
}

/// True when the party would take far longer to win than to fall, judged on plain attacks.
fn outmatched(state: &BattleState) -> bool {
    let party = living(state, BattleSide::Party).collect::<Vec<_>>();
    let enemies = living(state, BattleSide::Enemy).collect::<Vec<_>>();
    if party.is_empty() || enemies.is_empty() || enemies.iter().any(|enemy| enemy.boss) {
        return false;
    }
    let rounds = |attackers: &[&BattleCombatant], defenders: &[&BattleCombatant]| {
        let per_round = attackers
            .iter()
            .map(|attacker| {
                defenders
                    .iter()
                    .map(|defender| f64::from(physical_damage(attacker, defender)))
                    .sum::<f64>()
                    / defenders.len() as f64
            })
            .sum::<f64>()
            .max(1.0);
        let health = defenders
            .iter()
            .map(|defender| f64::from(defender.health))
            .sum::<f64>();
        health / per_round
    };
    let to_win = rounds(&party, &enemies);
    let to_lose = rounds(&enemies, &party);
    to_win > HOPELESS_ROUNDS && to_lose * 2.0 < to_win
}

fn revive(
    state: &BattleState,
    me: &BattleCombatant,
    items: &[&ConsumableItem],
    rng: &mut GameplayRng,
) -> Option<AutoAction> {
    let fallen = state
        .combatants
        .iter()
        .find(|actor| actor.key.side == BattleSide::Party && !actor.is_alive())?;
    if let Some(index) = castable(me)
        .find(|(_, ability)| is_revive(ability))
        .map(|(i, _)| i)
    {
        return Some(ability_action(state, me, index, Some(fallen.key)));
    }
    let item = items
        .iter()
        .find(|item| matches!(item.effect, ConsumableEffect::Revive(_)))?;
    roll_succeeds(rng, 0.85).then(|| AutoAction::Item {
        id: item.id.clone(),
        target: fallen.key,
    })
}

fn heal(
    state: &BattleState,
    me: &BattleCombatant,
    items: &[&ConsumableItem],
    rng: &mut GameplayRng,
) -> Option<AutoAction> {
    // A little jitter keeps the healer from topping up at exactly the same HP every time.
    let threshold = 0.4 + (rng.next_u64() % 21) as f64 / 100.0;
    let wounded = living(state, BattleSide::Party)
        .filter(|ally| ratio(ally.health, ally.max_health) < threshold)
        .collect::<Vec<_>>();
    let worst = wounded
        .iter()
        .min_by(|a, b| ratio(a.health, a.max_health).total_cmp(&ratio(b.health, b.max_health)))?;
    let heals = castable(me)
        .filter(|(_, ability)| is_restore(ability))
        .collect::<Vec<_>>();
    let spell = if wounded.len() >= 2 {
        heals
            .iter()
            .find(|(_, ability)| matches!(target_plan(ability), AbilityTargetPlan::All { .. }))
            .or_else(|| heals.first())
    } else {
        heals
            .iter()
            .find(|(_, ability)| matches!(target_plan(ability), AbilityTargetPlan::Select { .. }))
            .or_else(|| heals.first())
    };
    if let Some((index, ability)) = spell {
        let target_self = matches!(target_plan(ability), AbilityTargetPlan::SelfTarget);
        if !target_self || worst.key == me.key {
            return Some(ability_action(state, me, *index, Some(worst.key)));
        }
    }
    // Members without healing magic reach for the pouch readily; casters out of MP less so.
    let item_chance = if heals.is_empty() { 0.85 } else { 0.5 };
    let missing = worst.max_health.saturating_sub(worst.health);
    let item = smallest_sufficient_potion(items, missing)?;
    roll_succeeds(rng, item_chance).then(|| AutoAction::Item {
        id: item.id.clone(),
        target: worst.key,
    })
}

fn restore_own_mana(
    me: &BattleCombatant,
    items: &[&ConsumableItem],
    rng: &mut GameplayRng,
) -> Option<AutoAction> {
    let needs_mana = me.max_mana > 0
        && ratio(me.mana, me.max_mana) < 0.2
        && me.abilities.iter().any(|ability| ability.mp_cost > me.mana);
    if !needs_mana {
        return None;
    }
    let item = items
        .iter()
        .find(|item| matches!(item.effect, ConsumableEffect::RestoreMp(_)))?;
    roll_succeeds(rng, 0.3).then(|| AutoAction::Item {
        id: item.id.clone(),
        target: me.key,
    })
}

/// The strongest affordable damaging ability, preferring a sweep when several enemies stand.
fn best_offense(
    state: &BattleState,
    me: &BattleCombatant,
    rng: &mut GameplayRng,
) -> Option<AutoAction> {
    if me.is_silenced() {
        return None;
    }
    let enemies = living(state, BattleSide::Enemy).count().max(1) as f64;
    let (index, _) = castable(me)
        .filter(|(_, ability)| ability.mp_cost > 0)
        .filter_map(|(index, ability)| {
            let (coefficient, hits_all) = match &ability.kind {
                AbilityKind::Spell(spell) => (spell.spell_coeff.get(), all_enemies(ability)),
                AbilityKind::Physical(physical) => {
                    (physical.damage_coeff.get(), all_enemies(ability))
                }
                _ => return None,
            };
            Some((index, coefficient * if hits_all { enemies } else { 1.0 }))
        })
        .max_by(|a, b| a.1.total_cmp(&b.1))?;
    let target = if roll_succeeds(rng, 0.7) {
        attack_weakest(state)
    } else {
        random_living(state, BattleSide::Enemy, rng)
    };
    Some(ability_action(state, me, index, target))
}

fn ability_action(
    state: &BattleState,
    me: &BattleCombatant,
    index: usize,
    chosen: Option<CombatantKey>,
) -> AutoAction {
    let targets = match target_plan(&me.abilities[index]) {
        AbilityTargetPlan::SelfTarget => vec![me.key],
        AbilityTargetPlan::All { side, ko_eligible } => state
            .combatants
            .iter()
            .filter(|actor| actor.key.side == side && actor.is_alive() != ko_eligible)
            .map(|actor| actor.key)
            .collect(),
        AbilityTargetPlan::Select { group, .. } => chosen
            .filter(|key| {
                key.side
                    == match group {
                        TargetGroup::Enemy => BattleSide::Enemy,
                        TargetGroup::Ally => BattleSide::Party,
                    }
            })
            .or_else(|| match group {
                TargetGroup::Enemy => attack_weakest(state),
                TargetGroup::Ally => Some(me.key),
            })
            .into_iter()
            .collect(),
    };
    AutoAction::Ability { index, targets }
}

fn castable(me: &BattleCombatant) -> impl Iterator<Item = (usize, &Ability)> {
    let silenced = me.is_silenced();
    me.abilities.iter().enumerate().filter(move |(_, ability)| {
        !silenced && battle_ability(ability) && ability.mp_cost <= me.mana
    })
}

fn is_revive(ability: &Ability) -> bool {
    matches!(&ability.kind, AbilityKind::Heal(heal) if matches!(heal.healing, HealingMethod::Revive { .. }))
}

fn is_restore(ability: &Ability) -> bool {
    matches!(&ability.kind, AbilityKind::Heal(heal) if matches!(heal.healing, HealingMethod::Restore { .. }))
}

fn all_enemies(ability: &Ability) -> bool {
    matches!(
        target_plan(ability),
        AbilityTargetPlan::All {
            side: BattleSide::Enemy,
            ..
        }
    )
}

/// The cheapest HP potion that covers the wound, or the biggest one when none does.
fn smallest_sufficient_potion<'a>(
    items: &[&'a ConsumableItem],
    missing: u32,
) -> Option<&'a ConsumableItem> {
    let mut potions = items
        .iter()
        .filter_map(|item| match &item.effect {
            ConsumableEffect::RestoreHp(effect) => Some((*item, effect.restore_hp.get())),
            ConsumableEffect::FullRecovery(_) => Some((*item, u32::MAX)),
            _ => None,
        })
        .collect::<Vec<_>>();
    potions.sort_by_key(|(_, amount)| *amount);
    potions
        .iter()
        .find(|(_, amount)| *amount >= missing)
        .or_else(|| potions.last())
        .map(|(item, _)| *item)
}

fn attack_weakest(state: &BattleState) -> Option<CombatantKey> {
    living(state, BattleSide::Enemy)
        .min_by_key(|enemy| enemy.health)
        .map(|enemy| enemy.key)
}

fn random_living(
    state: &BattleState,
    side: BattleSide,
    rng: &mut GameplayRng,
) -> Option<CombatantKey> {
    let pool = living(state, side).collect::<Vec<_>>();
    if pool.is_empty() {
        return None;
    }
    Some(pool[(rng.next_u64() % pool.len() as u64) as usize].key)
}

fn living(state: &BattleState, side: BattleSide) -> impl Iterator<Item = &BattleCombatant> {
    state
        .combatants
        .iter()
        .filter(move |actor| actor.key.side == side && actor.is_alive())
}

fn ratio(current: u32, max: u32) -> f64 {
    if max == 0 {
        1.0
    } else {
        f64::from(current) / f64::from(max)
    }
}
