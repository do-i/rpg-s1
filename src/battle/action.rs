use super::model::CombatantKey;

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BattleEvent {
    Miss {
        action: BattleAction,
    },
    Damage {
        action: BattleAction,
        amount: u32,
        knocked_out: bool,
    },
}
