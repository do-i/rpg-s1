use crate::{
    encounter::{BattleEntry, BattleSide},
    gameplay_rng::GameplayRng,
};

use super::{
    action::{BattleAction, BattleEvent},
    model::{BattleCombatant, BattleCommand, BattlePhase, BattleState, CombatantKey},
    rules::{
        calculate_turn_order, critical_damage, critical_hit_chance, physical_damage,
        physical_hit_chance, roll_succeeds,
    },
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
            turn_count: 1,
            command_index: 0,
            ability_index: 0,
            pending_ability: None,
            item_index: 0,
            item_choices: Vec::new(),
            pending_item: None,
            target: None,
            message: "Battle start!".to_owned(),
            transcript: vec![format!("START {}", entry.encounter_id)],
            feedback_events: Vec::new(),
            used_enemy_moves: std::collections::HashSet::new(),
            rewards: None,
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
            BattleCommand::Spell => {
                active.is_alive()
                    && !active.is_silenced()
                    && active.abilities.iter().any(super::ability::battle_ability)
            }
            BattleCommand::Item => active.is_alive() && !self.item_choices.is_empty(),
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
        if let Some(effect) = active.skip_turn_reason() {
            self.message = format!("{} can't move ({effect:?})!", active.name);
            self.transcript
                .push(format!("SKIP {} {effect:?}", active.id));
            self.phase = BattlePhase::Resolve;
            return;
        }
        if active.key.side == BattleSide::Party {
            self.phase = BattlePhase::Command;
            self.command_index = 0;
            self.message = format!("{}: choose an action.", active.name);
        } else {
            self.resolve_enemy_action(rng);
        }
    }

    pub(super) fn resolve_enemy_action(&mut self, rng: &mut GameplayRng) {
        super::enemy_ai::resolve_enemy_turn(self, rng);
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
            _ => {
                self.feedback_events.push(event);
                return;
            }
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
                critical,
                knocked_out,
                ..
            } => {
                let actual = self
                    .actor_mut(target_key)
                    .map(|target| target.apply_resolved_damage(amount))
                    .unwrap_or(0);
                debug_assert_eq!(actual, amount);
                self.message = format!(
                    "{}{attacker_name} attacks {target_name} for {actual} damage.{}",
                    if critical { "Critical hit! " } else { "" },
                    if knocked_out { " KO!" } else { "" }
                );
                self.transcript.push(format!(
                    "{} {attacker_id} -> {target_id} {actual}{}",
                    if critical { "CRITICAL" } else { "HIT" },
                    if knocked_out { " KO" } else { "" }
                ));
            }
            _ => unreachable!("non-physical events return before actor lookup"),
        }
        self.feedback_events.push(event);
        self.phase = BattlePhase::Resolve;
    }

    pub(super) fn assess_result(&mut self) {
        self.tick_active_statuses();
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

    fn tick_active_statuses(&mut self) {
        let Some(active) = self.active_key() else {
            return;
        };
        let effect = self.actor(active).and_then(|actor| {
            actor
                .has_status(super::status::StatusEffect::Poison)
                .then_some(super::status::StatusEffect::Poison)
                .or_else(|| {
                    actor
                        .has_status(super::status::StatusEffect::Burn)
                        .then_some(super::status::StatusEffect::Burn)
                })
        });
        let tick = self
            .actor_mut(active)
            .map(BattleCombatant::tick_statuses)
            .unwrap_or_default();
        if tick.damage > 0 {
            let knocked_out = self.actor(active).is_none_or(|actor| !actor.is_alive());
            self.feedback_events.push(BattleEvent::StatusDamage {
                target: active,
                effect: effect.unwrap_or(super::status::StatusEffect::Poison),
                amount: tick.damage,
                knocked_out,
            });
            self.transcript.push(format!(
                "STATUS {:?}:{} {}{}",
                active.side,
                active.index,
                tick.damage,
                if knocked_out { " KO" } else { "" }
            ));
        }
    }

    pub(super) fn advance(&mut self, rng: &mut GameplayRng) {
        if !self.turn_order.is_empty() {
            self.step_turn();
        }
        self.begin_active_turn(rng);
    }

    pub(super) fn skip_knocked_out(&mut self) {
        for _ in 0..self.turn_order.len() {
            if self.active().is_some_and(BattleCombatant::is_alive) {
                break;
            }
            self.step_turn();
        }
    }

    fn step_turn(&mut self) {
        let previous = self.active_turn;
        self.active_turn = (self.active_turn + 1) % self.turn_order.len();
        if self.active_turn <= previous {
            self.turn_count = self.turn_count.saturating_add(1);
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
            let target = state
                .actor(target)
                .and_then(BattleCombatant::redirected_target)
                .filter(|redirect| {
                    state
                        .actor(*redirect)
                        .is_some_and(BattleCombatant::is_alive)
                })
                .unwrap_or(target);
            let action = BattleAction::Physical { attacker, target };
            let attacker_actor = state.actor(attacker)?;
            let defender_actor = state.actor(target)?;
            let chance = (physical_hit_chance(attacker_actor.dexterity, defender_actor.dexterity)
                * attacker_actor.hit_chance_multiplier())
            .clamp(0.05, 0.95);
            if !roll_succeeds(rng, chance) {
                return Some(BattleEvent::Miss { action });
            }
            let critical = roll_succeeds(rng, critical_hit_chance(attacker_actor.dexterity));
            let amount = if critical {
                critical_damage(physical_damage(attacker_actor, defender_actor))
            } else {
                physical_damage(attacker_actor, defender_actor)
            }
            .min(defender_actor.health);
            Some(BattleEvent::Damage {
                action,
                amount,
                critical,
                knocked_out: amount >= defender_actor.health,
            })
        }
    }
}
