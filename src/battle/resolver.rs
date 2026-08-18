use crate::{
    encounter::{BattleEntry, BattleSide},
    gameplay_rng::GameplayRng,
};

use super::{
    model::{BattleCombatant, BattleCommand, BattlePhase, BattleState, CombatantKey},
    rules::{calculate_turn_order, physical_damage, physical_hit_chance, roll_succeeds},
};

impl BattleState {
    pub(super) fn from_entry(entry: &BattleEntry) -> Self {
        let mut party_index = 0;
        let mut enemy_index = 0;
        let combatants = entry
            .participants
            .iter()
            .map(|participant| {
                let index = match participant.side {
                    BattleSide::Party => {
                        let current = party_index;
                        party_index += 1;
                        current
                    }
                    BattleSide::Enemy => {
                        let current = enemy_index;
                        enemy_index += 1;
                        current
                    }
                };
                BattleCombatant::from_participant(participant, index)
            })
            .collect::<Vec<_>>();
        Self {
            phase: BattlePhase::Start,
            turn_order: calculate_turn_order(&combatants),
            combatants,
            active_turn: 0,
            command_index: 0,
            target: None,
            message: "Battle start!".to_owned(),
            transcript: vec![format!("START {}", entry.encounter_id)],
            flee_outcome: None,
        }
    }

    pub(super) fn actor(&self, key: CombatantKey) -> Option<&BattleCombatant> {
        self.combatants.iter().find(|actor| actor.key == key)
    }

    pub(super) fn actor_mut(&mut self, key: CombatantKey) -> Option<&mut BattleCombatant> {
        self.combatants.iter_mut().find(|actor| actor.key == key)
    }

    pub(super) fn active_key(&self) -> Option<CombatantKey> {
        self.turn_order.get(self.active_turn).copied()
    }

    pub(super) fn active(&self) -> Option<&BattleCombatant> {
        self.active_key().and_then(|key| self.actor(key))
    }

    pub(super) fn all_defeated(&self, side: BattleSide) -> bool {
        self.combatants
            .iter()
            .filter(|actor| actor.key.side == side)
            .all(|actor| !actor.is_alive())
    }

    pub(super) fn command_available(&self, command: BattleCommand) -> bool {
        let Some(active) = self.active() else {
            return false;
        };
        match command {
            BattleCommand::Attack => active.is_alive(),
            // Spell and item effects enter in M10. They remain visible but explicitly disabled.
            BattleCommand::Spell | BattleCommand::Item => false,
            BattleCommand::Run => active.is_alive(),
        }
    }

    pub(super) fn begin_active_turn(&mut self, rng: &mut GameplayRng) {
        self.skip_knocked_out();
        let Some(active) = self.active().cloned() else {
            self.phase = BattlePhase::Defeat;
            return;
        };
        self.transcript.push(format!("TURN {}", active.id));
        if active.key.side == BattleSide::Party {
            self.phase = BattlePhase::Command;
            self.command_index = 0;
            self.message = format!("{}: choose an action.", active.name);
        } else {
            self.resolve_enemy_action(rng);
        }
    }

    pub(super) fn resolve_enemy_action(&mut self, rng: &mut GameplayRng) {
        let Some(attacker_key) = self.active_key() else {
            return;
        };
        let living = self
            .combatants
            .iter()
            .filter(|actor| actor.key.side == BattleSide::Party && actor.is_alive())
            .map(|actor| actor.key)
            .collect::<Vec<_>>();
        if living.is_empty() {
            self.phase = BattlePhase::Defeat;
            return;
        }
        let target = living[(rng.next_u64() % living.len() as u64) as usize];
        self.resolve_physical(attacker_key, target, rng);
    }

    pub(super) fn resolve_physical(
        &mut self,
        attacker_key: CombatantKey,
        target_key: CombatantKey,
        rng: &mut GameplayRng,
    ) {
        let Some(attacker) = self.actor(attacker_key).cloned() else {
            return;
        };
        let Some(defender) = self.actor(target_key).cloned() else {
            return;
        };
        let chance = physical_hit_chance(attacker.dexterity, defender.dexterity);
        if !roll_succeeds(rng, chance) {
            self.message = format!("{} attacks {}, but misses.", attacker.name, defender.name);
            self.transcript
                .push(format!("MISS {} -> {}", attacker.id, defender.id));
            self.phase = BattlePhase::Resolve;
            return;
        }
        let raw = physical_damage(&attacker, &defender);
        let actual = self
            .actor_mut(target_key)
            .map(|target| target.apply_damage(raw))
            .unwrap_or(0);
        let knocked_out = self
            .actor(target_key)
            .is_some_and(|target| !target.is_alive());
        self.message = format!(
            "{} attacks {} for {} damage.{}",
            attacker.name,
            defender.name,
            actual,
            if knocked_out { " KO!" } else { "" }
        );
        self.transcript.push(format!(
            "HIT {} -> {} {}{}",
            attacker.id,
            defender.id,
            actual,
            if knocked_out { " KO" } else { "" }
        ));
        self.phase = BattlePhase::Resolve;
    }

    pub(super) fn assess_result(&mut self) {
        if self.all_defeated(BattleSide::Enemy) {
            self.phase = BattlePhase::Victory;
            self.message = "Victory! Press Enter to return to the world.".to_owned();
            self.transcript.push("VICTORY".to_owned());
        } else if self.all_defeated(BattleSide::Party) {
            self.phase = BattlePhase::Defeat;
            self.message = "The party was defeated.".to_owned();
            self.transcript.push("DEFEAT".to_owned());
        } else {
            self.phase = BattlePhase::Advance;
        }
    }

    pub(super) fn advance(&mut self, rng: &mut GameplayRng) {
        if !self.turn_order.is_empty() {
            self.active_turn = (self.active_turn + 1) % self.turn_order.len();
        }
        self.begin_active_turn(rng);
    }

    pub(super) fn skip_knocked_out(&mut self) {
        for _ in 0..self.turn_order.len() {
            if self.active().is_some_and(BattleCombatant::is_alive) {
                break;
            }
            self.active_turn = (self.active_turn + 1) % self.turn_order.len();
        }
    }
}
