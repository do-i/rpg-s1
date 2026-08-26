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
    Multiplier(f64),
    Reduction(f64),
    Redirect(CombatantKey),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ActiveStatus {
    pub(super) effect: StatusEffect,
    pub(super) remaining_turns: Option<u32>,
    pub(super) potency: StatusPotency,
}

impl ActiveStatus {
    pub(super) const fn timed(effect: StatusEffect, turns: u32) -> Self {
        Self {
            effect,
            remaining_turns: Some(turns),
            potency: StatusPotency::None,
        }
    }

    pub(super) const fn persistent(effect: StatusEffect) -> Self {
        Self {
            effect,
            remaining_turns: None,
            potency: StatusPotency::None,
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
        }
    }

    pub(super) const fn modifier(effect: StatusEffect, turns: u32, multiplier: f64) -> Self {
        Self {
            effect,
            remaining_turns: Some(turns),
            potency: StatusPotency::Multiplier(multiplier),
        }
    }

    pub(super) const fn reduction(turns: u32, reduction: f64) -> Self {
        Self {
            effect: StatusEffect::DamageReduction,
            remaining_turns: Some(turns),
            potency: StatusPotency::Reduction(reduction),
        }
    }

    pub(super) const fn redirect(turns: u32, source: CombatantKey) -> Self {
        Self {
            effect: StatusEffect::RedirectDamage,
            remaining_turns: Some(turns),
            potency: StatusPotency::Redirect(source),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct StatusTick {
    pub(super) damage: u32,
    pub(super) expired: u32,
}
