use crate::scenario_class::AbilityElement;

use super::{
    model::CombatantKey,
    status::{ActiveStatus, StatusEffect},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BattleAction {
    Physical {
        attacker: CombatantKey,
        target: CombatantKey,
    },
}

impl BattleAction {
    pub(super) const fn attacker(self) -> CombatantKey {
        match self {
            Self::Physical { attacker, .. } => attacker,
        }
    }

    pub(super) const fn target(self) -> CombatantKey {
        match self {
            Self::Physical { target, .. } => target,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum BattleEvent {
    Miss {
        action: BattleAction,
    },
    Damage {
        action: BattleAction,
        amount: u32,
        critical: bool,
        knocked_out: bool,
    },
    MagicDamage {
        source: CombatantKey,
        target: CombatantKey,
        element: AbilityElement,
        amount: u32,
        knocked_out: bool,
    },
    Heal {
        source: CombatantKey,
        target: CombatantKey,
        amount: u32,
        revived: bool,
    },
    #[expect(dead_code, reason = "consumed by the M10 battle-item slice")]
    ManaRestored {
        source: CombatantKey,
        target: CombatantKey,
        amount: u32,
    },
    StatusApplied {
        source: CombatantKey,
        target: CombatantKey,
        status: ActiveStatus,
    },
    StatusCured {
        source: CombatantKey,
        target: CombatantKey,
        effect: Option<StatusEffect>,
    },
    StatusDamage {
        target: CombatantKey,
        effect: StatusEffect,
        amount: u32,
        knocked_out: bool,
    },
}
