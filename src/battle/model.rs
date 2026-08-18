use bevy::prelude::Resource;

use crate::{
    encounter::{BattleParticipant, BattleSide},
    scenario_party::PartyRow,
};

use super::rules::wrap_index;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BattlePhase {
    Start,
    Command,
    Target,
    Resolve,
    Advance,
    Victory,
    Defeat,
    Flee,
}

impl BattlePhase {
    pub(super) const fn allows(self, next: Self) -> bool {
        match self {
            Self::Start => matches!(next, Self::Command | Self::Resolve | Self::Defeat),
            Self::Command => matches!(next, Self::Target | Self::Victory | Self::Flee),
            Self::Target => matches!(next, Self::Command | Self::Resolve | Self::Victory),
            Self::Resolve => matches!(next, Self::Advance | Self::Victory | Self::Defeat),
            Self::Advance => matches!(next, Self::Command | Self::Resolve | Self::Defeat),
            Self::Victory | Self::Defeat => false,
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
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "M9 ally-target contract is exercised before M10 effects"
        )
    )]
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
    pub(super) dexterity: i64,
    pub(super) row: PartyRow,
    pub(super) boss: bool,
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
            dexterity: participant.dexterity,
            row: participant.row,
            boss: participant.boss,
        }
    }

    pub(super) const fn is_alive(&self) -> bool {
        self.health > 0
    }

    pub(super) fn apply_damage(&mut self, amount: u32) -> u32 {
        let actual = amount.min(self.health);
        self.health -= actual;
        actual
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TargetGroup {
    Enemy,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "M9 verifies ally eligibility before M10 effects consume it"
        )
    )]
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
    pub(super) command_index: usize,
    pub(super) target: Option<TargetSelector>,
    pub(super) message: String,
    pub(super) transcript: Vec<String>,
    pub(super) flee_outcome: Option<FleeOutcome>,
}
