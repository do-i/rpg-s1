use crate::{
    encounter::{BattleEntry, BattleSide},
    gameplay_rng::GameplayRng,
};

use super::{
    action::{BattleAction, BattleEvent},
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
    ) -> Option<BattleEvent> {
        let event = resolve_action(
            self,
            BattleAction::Physical {
                attacker: attacker_key,
                target: target_key,
            },
            rng,
        )?;
        self.apply_event(event);
        Some(event)
    }

    pub(super) fn apply_event(&mut self, event: BattleEvent) {
        let action = match event {
            BattleEvent::Miss { action } | BattleEvent::Damage { action, .. } => action,
        };
        let attacker_key = action.attacker();
        let target_key = action.target();
        let Some((attacker_id, attacker_name)) = self
            .actor(attacker_key)
            .map(|actor| (actor.id.clone(), actor.name.clone()))
        else {
            return;
        };
        let Some((target_id, target_name)) = self
            .actor(target_key)
            .map(|actor| (actor.id.clone(), actor.name.clone()))
        else {
            return;
        };

        match event {
            BattleEvent::Miss { .. } => {
                self.message = format!("{attacker_name} attacks {target_name}, but misses.");
                self.transcript
                    .push(format!("MISS {attacker_id} -> {target_id}"));
            }
            BattleEvent::Damage {
                amount,
                knocked_out,
                ..
            } => {
                let actual = self
                    .actor_mut(target_key)
                    .map(|target| target.apply_damage(amount))
                    .unwrap_or(0);
                debug_assert_eq!(actual, amount);
                self.message = format!(
                    "{attacker_name} attacks {target_name} for {actual} damage.{}",
                    if knocked_out { " KO!" } else { "" }
                );
                self.transcript.push(format!(
                    "HIT {attacker_id} -> {target_id} {actual}{}",
                    if knocked_out { " KO" } else { "" }
                ));
            }
        }
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

pub(super) fn resolve_action(
    state: &BattleState,
    action: BattleAction,
    rng: &mut GameplayRng,
) -> Option<BattleEvent> {
    match action {
        BattleAction::Physical { attacker, target } => {
            let attacker_actor = state.actor(attacker)?;
            let defender_actor = state.actor(target)?;
            let chance = physical_hit_chance(attacker_actor.dexterity, defender_actor.dexterity);
            if !roll_succeeds(rng, chance) {
                return Some(BattleEvent::Miss { action });
            }
            let amount = physical_damage(attacker_actor, defender_actor);
            Some(BattleEvent::Damage {
                action,
                amount,
                knocked_out: amount >= defender_actor.health,
            })
        }
    }
}
