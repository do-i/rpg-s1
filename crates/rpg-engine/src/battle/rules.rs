use crate::{
    encounter::BattleSide, gameplay_rng::GameplayRng, scenario_balance::BalanceData,
    scenario_party::PartyRow,
};

use super::model::{BattleCombatant, BattlePhase, BattleState, CombatantKey, FleeOutcome};

pub(super) fn calculate_turn_order(combatants: &[BattleCombatant]) -> Vec<CombatantKey> {
    let mut actors = combatants
        .iter()
        .filter(|actor| actor.is_alive())
        .collect::<Vec<_>>();
    actors.sort_by(|left, right| {
        right
            .dexterity
            .cmp(&left.dexterity)
            .then_with(|| side_priority(left.key.side).cmp(&side_priority(right.key.side)))
            .then_with(|| left.key.index.cmp(&right.key.index))
    });
    actors.into_iter().map(|actor| actor.key).collect()
}

pub(super) const fn side_priority(side: BattleSide) -> u8 {
    match side {
        BattleSide::Party => 0,
        BattleSide::Enemy => 1,
    }
}

pub(super) fn physical_hit_chance(attacker_dex: i64, defender_dex: i64) -> f64 {
    (0.70 + (attacker_dex - defender_dex) as f64 * 0.02).clamp(0.05, 0.95)
}

pub(super) fn critical_hit_chance(attacker_dex: i64) -> f64 {
    (attacker_dex.max(0) as f64 * 0.02).min(0.25)
}

pub(super) fn critical_damage(damage: u32) -> u32 {
    damage.saturating_mul(3) / 2
}

pub(super) fn physical_damage(attacker: &BattleCombatant, defender: &BattleCombatant) -> u32 {
    let mut damage = (attacker.effective_attack() - defender.effective_defense()).max(1) as u32;
    if attacker.row == PartyRow::Back {
        damage = (damage / 2).max(1);
    }
    if defender.key.side == BattleSide::Party && defender.row == PartyRow::Back {
        damage = (damage / 2).max(1);
    }
    defender.mitigated_damage(damage).min(defender.health)
}

pub(super) fn roll_succeeds(rng: &mut GameplayRng, chance: f64) -> bool {
    let roll = (rng.next_u64() >> 11) as f64 * (1.0 / ((1_u64 << 53) as f64));
    roll < chance
}

pub(super) fn roll_flee(boss: bool, chance: f64, rng: &mut GameplayRng) -> FleeOutcome {
    if boss {
        FleeOutcome::Blocked
    } else if chance >= 1.0 || roll_succeeds(rng, chance) {
        FleeOutcome::Success
    } else {
        FleeOutcome::Failed
    }
}

pub(super) const fn phase_after_flee_confirmation(outcome: FleeOutcome) -> Option<BattlePhase> {
    match outcome {
        FleeOutcome::Success => None,
        FleeOutcome::Failed | FleeOutcome::Blocked => Some(BattlePhase::Advance),
    }
}

pub(super) fn wrap_index(index: usize, movement: isize, length: usize) -> usize {
    (index as isize + movement).rem_euclid(length as isize) as usize
}

pub(super) fn flee_chance(state: &BattleState, balance: &BalanceData) -> f64 {
    let rogue_dexterity = state
        .combatants
        .iter()
        .filter(|actor| actor.key.side == BattleSide::Party && actor.class_id == "rogue")
        .map(|actor| actor.dexterity.max(0) as f64)
        .sum::<f64>();
    (balance.battle.flee_base_chance.get()
        + rogue_dexterity * balance.battle.flee_rogue_dex_bonus.get())
    .min(1.0)
}
