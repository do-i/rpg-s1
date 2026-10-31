use crate::scenario_item::ItemStatus;

use super::model::CombatantKey;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StatusEffect {
    Poison,
    Sleep,
    Stun,
    Silence,
    Burn,
    Freeze,
    Knockback,
    Taunt,
    AttackModifier,
    DefenseModifier,
    MagicResistanceModifier,
    HitChanceModifier,
    DamageReduction,
    RedirectDamage,
    Haste,
    Slow,
    Regen,
}

impl From<ItemStatus> for StatusEffect {
    fn from(value: ItemStatus) -> Self {
        match value {
            ItemStatus::Poison => Self::Poison,
            ItemStatus::Silence => Self::Silence,
            ItemStatus::Sleep => Self::Sleep,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum StatusPotency {
    None,
    DamagePerTurn(u32),
    HealingPercent(f64),
    Multiplier(f64),
    Reduction(f64),
    Redirect(CombatantKey),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ActiveStatus {
    pub(super) effect: StatusEffect,
    pub(super) remaining_turns: Option<u32>,
    pub(super) potency: StatusPotency,
    /// Speed effects begin at the next round boundary and last full rounds.
    pub(super) starts_next_round: bool,
}

impl ActiveStatus {
    pub(super) const fn timed(effect: StatusEffect, turns: u32) -> Self {
        Self {
            effect,
            remaining_turns: Some(turns),
            potency: StatusPotency::None,
            starts_next_round: false,
        }
    }

    pub(super) const fn persistent(effect: StatusEffect) -> Self {
        Self {
            effect,
            remaining_turns: None,
            potency: StatusPotency::None,
            starts_next_round: false,
        }
    }

    pub(super) const fn damage_over_time(
        effect: StatusEffect,
        turns: Option<u32>,
        damage: u32,
    ) -> Self {
        Self {
            effect,
            remaining_turns: turns,
            potency: StatusPotency::DamagePerTurn(damage),
            starts_next_round: false,
        }
    }

    pub(super) const fn modifier(effect: StatusEffect, turns: u32, multiplier: f64) -> Self {
        Self {
            effect,
            remaining_turns: Some(turns),
            potency: StatusPotency::Multiplier(multiplier),
            starts_next_round: false,
        }
    }

    pub(super) const fn reduction(turns: u32, reduction: f64) -> Self {
        Self {
            effect: StatusEffect::DamageReduction,
            remaining_turns: Some(turns),
            potency: StatusPotency::Reduction(reduction),
            starts_next_round: false,
        }
    }

    pub(super) const fn redirect(turns: u32, source: CombatantKey) -> Self {
        Self {
            effect: StatusEffect::RedirectDamage,
            remaining_turns: Some(turns),
            potency: StatusPotency::Redirect(source),
            starts_next_round: false,
        }
    }

    pub(super) const fn speed(turns: u32, multiplier: f64) -> Self {
        Self {
            effect: if multiplier >= 1.0 {
                StatusEffect::Haste
            } else {
                StatusEffect::Slow
            },
            remaining_turns: Some(turns),
            potency: StatusPotency::Multiplier(multiplier),
            starts_next_round: true,
        }
    }

    pub(super) const fn regeneration(turns: u32, percent: f64) -> Self {
        Self {
            effect: StatusEffect::Regen,
            remaining_turns: Some(turns),
            potency: StatusPotency::HealingPercent(percent),
            starts_next_round: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct StatusTick {
    pub(super) damage: u32,
    pub(super) healing: u32,
    pub(super) expired: u32,
}
