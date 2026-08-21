use std::collections::HashSet;

use bevy::prelude::Resource;

use crate::{
    encounter::{BattleParticipant, BattleSide},
    scenario_party::PartyRow,
};

use super::{
    rules::wrap_index,
    status::{ActiveStatus, StatusEffect, StatusPotency, StatusTick},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BattlePhase {
    Start,
    Command,
    Ability,
    Item,
    Target,
    Resolve,
    Advance,
    Victory,
    Rewards,
    Defeat,
    Flee,
}

impl BattlePhase {
    pub(super) const fn allows(self, next: Self) -> bool {
        match self {
            Self::Start => matches!(next, Self::Command | Self::Resolve | Self::Defeat),
            Self::Command => matches!(
                next,
                Self::Ability | Self::Item | Self::Target | Self::Victory | Self::Flee
            ),
            Self::Ability => matches!(next, Self::Command | Self::Target | Self::Resolve),
            Self::Item => matches!(next, Self::Command | Self::Target | Self::Resolve),
            Self::Target => matches!(
                next,
                Self::Command | Self::Ability | Self::Item | Self::Resolve | Self::Victory
            ),
            Self::Resolve => matches!(next, Self::Advance | Self::Victory | Self::Defeat),
            Self::Advance => matches!(next, Self::Command | Self::Resolve | Self::Defeat),
            Self::Victory => matches!(next, Self::Rewards),
            Self::Rewards | Self::Defeat => false,
            Self::Flee => matches!(next, Self::Advance),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BattleCommand {
    Attack,
    Spell,
    Item,
    Run,
}

impl BattleCommand {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Attack => "Attack",
            Self::Spell => "Spell",
            Self::Item => "Item",
            Self::Run => "Run",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct CombatantKey {
    pub(super) side: BattleSide,
    pub(super) index: usize,
}

impl CombatantKey {
    pub(super) const fn party(index: usize) -> Self {
        Self {
            side: BattleSide::Party,
            index,
        }
    }

    pub(super) const fn enemy(index: usize) -> Self {
        Self {
            side: BattleSide::Enemy,
            index,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct BattleCombatant {
    pub(super) key: CombatantKey,
    pub(super) id: String,
    pub(super) name: String,
    pub(super) class_id: String,
    pub(super) health: u32,
    pub(super) max_health: u32,
    pub(super) mana: u32,
    pub(super) max_mana: u32,
    pub(super) attack: i64,
    pub(super) defense: i64,
    pub(super) magic_resistance: i64,
    pub(super) dexterity: i64,
    pub(super) abilities: Vec<crate::scenario_class::Ability>,
    pub(super) status_effects: Vec<ActiveStatus>,
    pub(super) accessory: Option<String>,
    pub(super) row: PartyRow,
    pub(super) boss: bool,
    pub(super) enemy_type: Option<crate::scenario_enemy::EnemyType>,
    pub(super) immunities: Vec<crate::scenario_enemy::EnemyImmunity>,
    pub(super) behavior: Option<crate::scenario_enemy::EnemyBehavior>,
    pub(super) experience_yield: u32,
    pub(super) drops: Option<crate::scenario_enemy::EnemyDrops>,
}

impl BattleCombatant {
    pub(super) fn from_participant(participant: &BattleParticipant, index: usize) -> Self {
        Self {
            key: CombatantKey {
                side: participant.side,
                index,
            },
            id: participant.id.clone(),
            name: participant.name.clone(),
            class_id: participant.class_id.clone(),
            health: participant.health,
            max_health: participant.max_health,
            mana: participant.mana,
            max_mana: participant.max_mana,
            attack: participant.attack,
            defense: participant.defense,
            magic_resistance: participant.magic_resistance,
            dexterity: participant.dexterity,
            abilities: participant.abilities.clone(),
            status_effects: participant
                .status_effects
                .iter()
                .copied()
                .map(StatusEffect::from)
                .map(ActiveStatus::persistent)
                .collect(),
            accessory: participant.accessory.clone(),
            row: participant.row,
            boss: participant.boss,
            enemy_type: participant.enemy_type,
            immunities: participant.immunities.clone(),
            behavior: participant.behavior.clone(),
            experience_yield: participant.experience_yield,
            drops: participant.drops.clone(),
        }
    }

    pub(super) const fn is_alive(&self) -> bool {
        self.health > 0
    }

    pub(super) fn apply_damage(&mut self, amount: u32) -> u32 {
        let amount = self.mitigated_damage(amount);
        self.apply_resolved_damage(amount)
    }

    pub(super) fn mitigated_damage(&self, amount: u32) -> u32 {
        let reduction = self
            .status_effects
            .iter()
            .filter_map(|status| match status.potency {
                StatusPotency::Reduction(reduction) => Some(reduction),
                _ => None,
            })
            .fold(0.0_f64, |combined, reduction| {
                1.0 - (1.0 - combined) * (1.0 - reduction.clamp(0.0, 1.0))
            });
        if amount == 0 {
            0
        } else {
            ((f64::from(amount) * (1.0 - reduction)) as u32).max(1)
        }
    }

    pub(super) fn apply_resolved_damage(&mut self, amount: u32) -> u32 {
        let actual = amount.min(self.health);
        self.health -= actual;
        if actual > 0 {
            self.remove_status(StatusEffect::Sleep);
        }
        actual
    }

    pub(super) fn apply_heal(&mut self, amount: u32) -> u32 {
        if !self.is_alive() {
            return 0;
        }
        let before = self.health;
        self.health = self.health.saturating_add(amount).min(self.max_health);
        self.health - before
    }

    pub(super) fn restore_mana(&mut self, amount: u32) -> u32 {
        let before = self.mana;
        self.mana = self.mana.saturating_add(amount).min(self.max_mana);
        self.mana - before
    }

    pub(super) fn spend_mana(&mut self, amount: u32) -> Option<u32> {
        (self.mana >= amount).then(|| {
            self.mana -= amount;
            amount
        })
    }

    pub(super) fn revive(&mut self, fraction: f64) -> u32 {
        if self.is_alive() || self.max_health == 0 {
            return 0;
        }
        self.health = ((f64::from(self.max_health) * fraction) as u32)
            .max(1)
            .min(self.max_health);
        self.health
    }

    pub(super) fn add_status(&mut self, status: ActiveStatus) -> bool {
        if self
            .enemy_type
            .is_some_and(|kind| kind == crate::scenario_enemy::EnemyType::Construct)
            && status.effect.is_harmful()
        {
            return false;
        }
        if let Some(current) = self
            .status_effects
            .iter_mut()
            .find(|current| current.effect == status.effect)
        {
            *current = status;
        } else {
            self.status_effects.push(status);
        }
        true
    }

    pub(super) fn has_status(&self, effect: StatusEffect) -> bool {
        self.status_effects
            .iter()
            .any(|status| status.effect == effect)
    }

    pub(super) fn remove_status(&mut self, effect: StatusEffect) -> bool {
        let before = self.status_effects.len();
        self.status_effects.retain(|status| status.effect != effect);
        self.status_effects.len() != before
    }

    pub(super) fn clear_statuses(&mut self) -> usize {
        let removed = self.status_effects.len();
        self.status_effects.clear();
        removed
    }

    pub(super) fn is_silenced(&self) -> bool {
        self.has_status(StatusEffect::Silence)
    }

    pub(super) fn skip_turn_reason(&self) -> Option<StatusEffect> {
        self.status_effects
            .iter()
            .map(|status| status.effect)
            .find(|effect| {
                matches!(
                    effect,
                    StatusEffect::Sleep
                        | StatusEffect::Stun
                        | StatusEffect::Freeze
                        | StatusEffect::DamageReduction
                )
            })
    }

    pub(super) fn redirected_target(&self) -> Option<CombatantKey> {
        self.status_effects.iter().find_map(|status| {
            if let StatusPotency::Redirect(target) = status.potency {
                Some(target)
            } else {
                None
            }
        })
    }

    pub(super) fn is_taunting(&self) -> bool {
        self.has_status(StatusEffect::Taunt)
    }

    pub(super) fn effective_attack(&self) -> i64 {
        modified_stat(
            self.attack,
            &self.status_effects,
            &[StatusEffect::AttackModifier, StatusEffect::Knockback],
            1,
        )
    }

    pub(super) fn effective_defense(&self) -> i64 {
        modified_stat(
            self.defense,
            &self.status_effects,
            &[StatusEffect::DefenseModifier],
            0,
        )
    }

    pub(super) fn effective_magic_resistance(&self) -> i64 {
        modified_stat(
            self.magic_resistance,
            &self.status_effects,
            &[StatusEffect::MagicResistanceModifier],
            0,
        )
    }

    pub(super) fn hit_chance_multiplier(&self) -> f64 {
        status_multiplier(&self.status_effects, &[StatusEffect::HitChanceModifier])
    }

    pub(super) fn tick_statuses(&mut self) -> StatusTick {
        let damage = self
            .status_effects
            .iter()
            .filter_map(|status| match status.potency {
                StatusPotency::DamagePerTurn(amount) => Some(amount),
                _ => None,
            })
            .fold(0_u32, u32::saturating_add);
        let before = self.status_effects.len();
        for status in &mut self.status_effects {
            if let Some(turns) = status.remaining_turns.as_mut() {
                *turns = turns.saturating_sub(1);
            }
        }
        self.status_effects
            .retain(|status| status.remaining_turns != Some(0));
        let expired = u32::try_from(before - self.status_effects.len()).unwrap_or(u32::MAX);
        let damage = self.apply_damage(damage);
        StatusTick { damage, expired }
    }
}

impl StatusEffect {
    fn is_harmful(self) -> bool {
        matches!(
            self,
            Self::Poison
                | Self::Sleep
                | Self::Stun
                | Self::Silence
                | Self::Burn
                | Self::Freeze
                | Self::Knockback
                | Self::AttackModifier
                | Self::DefenseModifier
                | Self::MagicResistanceModifier
                | Self::HitChanceModifier
        )
    }
}

fn modified_stat(
    base: i64,
    statuses: &[ActiveStatus],
    effects: &[StatusEffect],
    minimum: i64,
) -> i64 {
    ((base as f64 * status_multiplier(statuses, effects)) as i64).max(minimum)
}

fn status_multiplier(statuses: &[ActiveStatus], effects: &[StatusEffect]) -> f64 {
    statuses
        .iter()
        .filter(|status| effects.contains(&status.effect))
        .filter_map(|status| match status.potency {
            StatusPotency::Multiplier(multiplier) => Some(multiplier),
            _ => None,
        })
        .product()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TargetGroup {
    Enemy,
    Ally,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct TargetSelector {
    pub(super) group: TargetGroup,
    pub(super) eligible: Vec<CombatantKey>,
    pub(super) selected: usize,
}

impl TargetSelector {
    pub(super) fn new(
        group: TargetGroup,
        combatants: &[BattleCombatant],
        ko_eligible: bool,
    ) -> Option<Self> {
        let side = match group {
            TargetGroup::Enemy => BattleSide::Enemy,
            TargetGroup::Ally => BattleSide::Party,
        };
        let eligible = combatants
            .iter()
            .filter(|actor| actor.key.side == side)
            .filter(|actor| actor.is_alive() != ko_eligible)
            .map(|actor| actor.key)
            .collect::<Vec<_>>();
        (!eligible.is_empty()).then_some(Self {
            group,
            eligible,
            selected: 0,
        })
    }

    pub(super) fn selected(&self) -> CombatantKey {
        self.eligible[self.selected]
    }

    pub(super) fn navigate(&mut self, movement: isize) {
        self.selected = wrap_index(self.selected, movement, self.eligible.len());
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum FleeOutcome {
    Success,
    Failed,
    Blocked,
}

#[derive(Debug, Resource)]
pub(super) struct BattleState {
    pub(super) phase: BattlePhase,
    pub(super) combatants: Vec<BattleCombatant>,
    pub(super) turn_order: Vec<CombatantKey>,
    pub(super) active_turn: usize,
    pub(super) turn_count: u32,
    pub(super) command_index: usize,
    pub(super) ability_index: usize,
    pub(super) pending_ability: Option<usize>,
    pub(super) item_index: usize,
    pub(super) item_choices: Vec<BattleItemChoice>,
    pub(super) pending_item: Option<String>,
    pub(super) target: Option<TargetSelector>,
    pub(super) message: String,
    pub(super) transcript: Vec<String>,
    pub(super) feedback_events: Vec<super::action::BattleEvent>,
    pub(super) used_enemy_moves: HashSet<(CombatantKey, String)>,
    pub(super) rewards: Option<super::rewards::BattleRewards>,
    pub(super) flee_outcome: Option<FleeOutcome>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct BattleItemChoice {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) quantity: u32,
}

impl BattleState {
    pub(super) fn battle_ability_indices(&self) -> Vec<usize> {
        self.active()
            .into_iter()
            .flat_map(|actor| &actor.abilities)
            .enumerate()
            .filter(|(_, ability)| super::ability::battle_ability(ability))
            .map(|(index, _)| index)
            .collect()
    }
}
