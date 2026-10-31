use super::*;

#[test]
fn speed_changes_wait_for_round_boundary_and_last_full_rounds() {
    for side in [BattleSide::Party, BattleSide::Enemy] {
        let mut target = actor(side, 0, 8, 100);
        assert!(target.add_status(ActiveStatus::speed(2, 1.5)));
        assert_eq!(target.effective_speed(), 8);
        target.tick_statuses();
        assert_eq!(target.status_effects[0].remaining_turns, Some(2));
        target.begin_next_round();
        assert_eq!(target.effective_speed(), 12);
        target.begin_next_round();
        assert_eq!(target.effective_speed(), 12);
        target.begin_next_round();
        assert_eq!(target.effective_speed(), 8);
        assert!(target.status_effects.is_empty());
    }
}

#[test]
fn speed_changes_reorder_next_round_without_duplicating_or_skipping_actors() {
    let party = actor(BattleSide::Party, 0, 10, 100);
    let enemy = actor(BattleSide::Enemy, 0, 8, 100);
    let mut state = state_with(vec![party, enemy]);
    let mut rng = GameplayRng::from_seed(1);
    state.combatants[1].add_status(ActiveStatus::speed(3, 1.5));
    assert_eq!(state.active_key(), Some(CombatantKey::party(0)));
    state.advance(&mut rng);
    assert_eq!(state.active_key(), Some(CombatantKey::enemy(0)));
    assert_eq!(state.turn_count, 1);
    state.advance(&mut rng);
    assert_eq!(state.turn_count, 2);
    assert_eq!(
        state.turn_order,
        [CombatantKey::enemy(0), CombatantKey::party(0)]
    );
    assert_eq!(state.active_key(), Some(CombatantKey::enemy(0)));
    state.advance(&mut rng);
    assert_eq!(state.active_key(), Some(CombatantKey::party(0)));
    assert_eq!(
        state.combatants[1].dexterity, 8,
        "accuracy and critical DEX stays unchanged"
    );
}

#[test]
fn speed_effects_replace_opposites_refresh_and_clear() {
    let mut target = actor(BattleSide::Party, 0, 20, 100);
    target.add_status(ActiveStatus::speed(3, 1.5));
    target.begin_next_round();
    target.add_status(ActiveStatus::speed(2, 0.5));
    assert!(!target.has_status(StatusEffect::Haste));
    target.begin_next_round();
    assert_eq!(target.effective_speed(), 10);
    target.add_status(ActiveStatus::speed(4, 0.5));
    target.begin_next_round();
    assert_eq!(target.status_effects.len(), 1);
    assert_eq!(target.effective_speed(), 10);
    assert_eq!(target.status_effects[0].remaining_turns, Some(4));
    target.clear_statuses();
    assert_eq!(target.effective_speed(), 20);
}

#[test]
fn regen_heals_both_sides_caps_expires_and_never_revives() {
    for side in [BattleSide::Party, BattleSide::Enemy] {
        let mut target = actor(side, 0, 10, 100);
        target.health = 91;
        target.add_status(ActiveStatus::regeneration(3, 0.05));
        target.add_status(ActiveStatus::regeneration(3, 0.05));
        assert_eq!(target.status_effects.len(), 1);
        assert_eq!(target.tick_statuses().healing, 5);
        assert_eq!(target.tick_statuses().healing, 4);
        assert_eq!(target.tick_statuses().healing, 0);
        assert!(!target.has_status(StatusEffect::Regen));
        target.health = 4;
        target.add_status(ActiveStatus::regeneration(3, 0.05));
        target.add_status(ActiveStatus::damage_over_time(
            StatusEffect::Poison,
            None,
            4,
        ));
        let tick = target.tick_statuses();
        assert_eq!((tick.damage, tick.healing, target.health), (4, 0, 0));
        assert!(!target.has_status(StatusEffect::Regen));
        target.revive(0.5);
        assert!(!target.has_status(StatusEffect::Regen));
    }
    let mut small = actor(BattleSide::Party, 0, 1, 10);
    small.health = 1;
    small.add_status(ActiveStatus::regeneration(1, 0.05));
    assert_eq!(small.tick_statuses().healing, 1);
}

#[test]
fn scenario_player_skills_apply_real_statuses_and_spend_mana() {
    for (id, target, effect, cost) in [
        ("haste", CombatantKey::party(0), StatusEffect::Haste, 4),
        ("slow", CombatantKey::enemy(0), StatusEffect::Slow, 5),
        ("regen", CombatantKey::party(0), StatusEffect::Regen, 5),
    ] {
        let mut state = state_with(vec![
            actor(BattleSide::Party, 0, 10, 100),
            actor(BattleSide::Enemy, 0, 8, 100),
        ]);
        state.combatants[0].mana = 20;
        state.combatants[0].abilities.push(ability(id));
        let mut rng = GameplayRng::from_seed(1);
        resolve_ability(&mut state, CombatantKey::party(0), 0, &[target], &mut rng).unwrap();
        assert!(state.actor(target).unwrap().has_status(effect));
        assert_eq!(state.combatants[0].mana, 20 - cost);
    }
}

#[test]
fn authored_enemy_support_moves_are_relative_to_enemy_and_deal_no_damage() {
    for (id, effect_yaml, target, effect) in [
        (
            "haste",
            "target: self, stat: speed, modifier: 1.5",
            CombatantKey::enemy(0),
            StatusEffect::Haste,
        ),
        (
            "regen",
            "target: party, regen_hp_pct: 0.05",
            CombatantKey::enemy(0),
            StatusEffect::Regen,
        ),
        (
            "slow",
            "target: all_enemies, stat: speed, modifier: 0.5",
            CombatantKey::party(0),
            StatusEffect::Slow,
        ),
    ] {
        let mut enemy = actor(BattleSide::Enemy, 0, 8, 100);
        enemy.behavior = Some(enemy_behavior(&format!(
            "ai:\n  pattern: random\n  moves:\n    - action: ability\n      id: {id}\n      weight: 1\n      effect: {{ {effect_yaml}, duration_turns: 3 }}\ntargeting:\n  default: random_alive\n"
        )));
        let mut state = state_with(vec![actor(BattleSide::Party, 0, 10, 100), enemy.clone()]);
        let mut rng = GameplayRng::from_seed(1);
        resolve_enemy_ability(&mut state, &enemy, id, &[CombatantKey::party(0)], &mut rng);
        assert!(state.actor(target).unwrap().has_status(effect));
        assert!(state.combatants.iter().all(|actor| actor.health == 100));
        assert!(
            state
                .feedback_events
                .iter()
                .all(|event| matches!(event, BattleEvent::StatusApplied { .. }))
        );
    }
}

#[test]
fn regen_feedback_is_emitted_for_actual_healing() {
    let mut party = actor(BattleSide::Party, 0, 10, 100);
    party.health = 80;
    party.add_status(ActiveStatus::regeneration(1, 0.05));
    let mut state = state_with(vec![party, actor(BattleSide::Enemy, 0, 8, 100)]);
    state.assess_result();
    assert!(state.feedback_events.contains(&BattleEvent::StatusHeal {
        target: CombatantKey::party(0),
        amount: 5
    }));
    assert_eq!(state.combatants[0].health, 85);
    assert!(!state.combatants[0].has_status(StatusEffect::Regen));
}
