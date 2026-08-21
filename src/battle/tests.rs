use super::{
    action::{BattleAction, BattleEvent},
    model::{
        BattleCombatant, BattlePhase, BattleState, CombatantKey, FleeOutcome, TargetGroup,
        TargetSelector,
    },
    resolver::resolve_action,
    rules::{
        calculate_turn_order, critical_damage, critical_hit_chance, flee_chance,
        phase_after_flee_confirmation, physical_damage, physical_hit_chance, roll_flee,
        roll_succeeds,
    },
};
use crate::{
    encounter::BattleSide, gameplay_rng::GameplayRng, scenario_balance::BalanceData,
    scenario_party::PartyRow,
};

fn actor(side: BattleSide, index: usize, dex: i64, health: u32) -> BattleCombatant {
    BattleCombatant {
        key: CombatantKey { side, index },
        id: format!("{side:?}-{index}"),
        name: format!("{side:?} {index}"),
        class_id: String::new(),
        health,
        max_health: health.max(1),
        mana: 0,
        max_mana: 0,
        attack: 10,
        defense: 3,
        dexterity: dex,
        row: PartyRow::Front,
        boss: false,
    }
}

fn state_with(combatants: Vec<BattleCombatant>) -> BattleState {
    BattleState {
        phase: BattlePhase::Command,
        turn_order: calculate_turn_order(&combatants),
        combatants,
        active_turn: 0,
        command_index: 0,
        target: None,
        message: String::new(),
        transcript: Vec::new(),
        flee_outcome: None,
    }
}

#[test]
fn phase_graph_names_and_bounds_every_minimum_loop_transition() {
    assert!(BattlePhase::Start.allows(BattlePhase::Command));
    assert!(BattlePhase::Command.allows(BattlePhase::Target));
    assert!(BattlePhase::Target.allows(BattlePhase::Resolve));
    assert!(BattlePhase::Resolve.allows(BattlePhase::Advance));
    assert!(BattlePhase::Advance.allows(BattlePhase::Command));
    assert!(BattlePhase::Resolve.allows(BattlePhase::Victory));
    assert!(BattlePhase::Resolve.allows(BattlePhase::Defeat));
    assert!(BattlePhase::Command.allows(BattlePhase::Flee));
    assert!(BattlePhase::Flee.allows(BattlePhase::Advance));
    assert!(!BattlePhase::Victory.allows(BattlePhase::Command));
    assert!(!BattlePhase::Defeat.allows(BattlePhase::Advance));
}

#[test]
fn enemy_target_selection_handles_single_wrap_cancel_and_no_target() {
    let actors = vec![
        actor(BattleSide::Party, 0, 4, 10),
        actor(BattleSide::Enemy, 0, 4, 0),
        actor(BattleSide::Enemy, 1, 4, 10),
        actor(BattleSide::Enemy, 2, 4, 10),
    ];
    let mut selector = TargetSelector::new(TargetGroup::Enemy, &actors, false).unwrap();
    assert_eq!(selector.selected(), CombatantKey::enemy(1));
    selector.navigate(-1);
    assert_eq!(selector.selected(), CombatantKey::enemy(2));
    selector.navigate(1);
    assert_eq!(selector.selected(), CombatantKey::enemy(1));
    assert!(TargetSelector::new(TargetGroup::Enemy, &actors[..2], false).is_none());

    let one = vec![actor(BattleSide::Enemy, 0, 1, 1)];
    let mut single = TargetSelector::new(TargetGroup::Enemy, &one, false).unwrap();
    single.navigate(1);
    assert_eq!(single.selected(), CombatantKey::enemy(0));
}

#[test]
fn ally_targeting_distinguishes_living_and_ko_eligibility() {
    let actors = vec![
        actor(BattleSide::Party, 0, 3, 10),
        actor(BattleSide::Party, 1, 2, 0),
        actor(BattleSide::Enemy, 0, 1, 10),
    ];
    assert_eq!(
        TargetSelector::new(TargetGroup::Ally, &actors, false)
            .unwrap()
            .selected(),
        CombatantKey::party(0)
    );
    assert_eq!(
        TargetSelector::new(TargetGroup::Ally, &actors, true)
            .unwrap()
            .selected(),
        CombatantKey::party(1)
    );
}

#[test]
fn turn_order_uses_dexterity_then_party_then_authored_order() {
    let actors = vec![
        actor(BattleSide::Enemy, 0, 8, 10),
        actor(BattleSide::Party, 0, 8, 10),
        actor(BattleSide::Party, 1, 12, 10),
        actor(BattleSide::Enemy, 1, 99, 0),
    ];
    assert_eq!(
        calculate_turn_order(&actors),
        vec![
            CombatantKey::party(1),
            CombatantKey::party(0),
            CombatantKey::enemy(0)
        ]
    );
}

#[test]
fn hit_chance_clamps_at_both_boundaries_and_seeded_rolls_repeat() {
    assert_eq!(physical_hit_chance(100, 1), 0.95);
    assert_eq!(physical_hit_chance(1, 100), 0.05);
    assert_eq!(physical_hit_chance(20, 20), 0.70);
    let mut left = GameplayRng::from_seed(77);
    let mut right = GameplayRng::from_seed(77);
    let left = (0..16)
        .map(|_| roll_succeeds(&mut left, 0.7))
        .collect::<Vec<_>>();
    let right = (0..16)
        .map(|_| roll_succeeds(&mut right, 0.7))
        .collect::<Vec<_>>();
    assert_eq!(left, right);
}

#[test]
fn flee_rate_uses_rogue_dex_caps_and_boss_or_failure_cost_rules() {
    let balance = BalanceData::default();
    let plain = state_with(vec![
        actor(BattleSide::Party, 0, 8, 10),
        actor(BattleSide::Enemy, 0, 4, 10),
    ]);
    assert_eq!(flee_chance(&plain, &balance), 0.30);

    let mut rogue = actor(BattleSide::Party, 0, 35, 10);
    rogue.class_id = "rogue".to_owned();
    let guaranteed = state_with(vec![rogue, actor(BattleSide::Enemy, 0, 4, 10)]);
    assert_eq!(flee_chance(&guaranteed, &balance), 1.0);

    let mut rng = GameplayRng::from_seed(5);
    let before = rng.state();
    assert_eq!(roll_flee(false, 1.0, &mut rng), FleeOutcome::Success);
    assert_eq!(
        rng.state(),
        before,
        "guaranteed flee consumes no random roll"
    );
    assert_eq!(roll_flee(true, 1.0, &mut rng), FleeOutcome::Blocked);
    assert_eq!(
        rng.state(),
        before,
        "boss restriction consumes no random roll"
    );

    assert_eq!(
        phase_after_flee_confirmation(FleeOutcome::Failed),
        Some(BattlePhase::Advance)
    );
    assert_eq!(
        phase_after_flee_confirmation(FleeOutcome::Blocked),
        Some(BattlePhase::Advance)
    );
    assert_eq!(phase_after_flee_confirmation(FleeOutcome::Success), None);
}

#[test]
fn physical_damage_applies_attack_defense_rows_minimum_and_health_cap() {
    let mut front = actor(BattleSide::Party, 0, 1, 99);
    front.attack = 20;
    let mut enemy = actor(BattleSide::Enemy, 0, 1, 50);
    enemy.defense = 6;
    assert_eq!(physical_damage(&front, &enemy), 14);
    front.row = PartyRow::Back;
    assert_eq!(physical_damage(&front, &enemy), 7);
    enemy.health = 3;
    assert_eq!(physical_damage(&front, &enemy), 3);
    front.attack = 1;
    enemy.defense = 999;
    enemy.health = 50;
    assert_eq!(physical_damage(&front, &enemy), 1);
    let mut back_party = actor(BattleSide::Party, 1, 1, 50);
    back_party.row = PartyRow::Back;
    let enemy_attacker = actor(BattleSide::Enemy, 0, 1, 50);
    assert_eq!(physical_damage(&enemy_attacker, &back_party), 3);
}

#[test]
fn resolving_an_action_is_pure_until_its_typed_event_is_applied() {
    let mut state = state_with(vec![
        actor(BattleSide::Party, 0, 100, 20),
        actor(BattleSide::Enemy, 0, 1, 20),
    ]);
    let action = BattleAction::Physical {
        attacker: CombatantKey::party(0),
        target: CombatantKey::enemy(0),
    };
    let before = state.combatants[1].health;
    let event = resolve_action(&state, action, &mut GameplayRng::from_seed(9)).unwrap();

    assert_eq!(state.combatants[1].health, before);
    assert_eq!(
        event,
        BattleEvent::Damage {
            action,
            amount: 7,
            critical: false,
            knocked_out: false,
        }
    );

    state.apply_event(event);
    assert_eq!(state.combatants[1].health, before - 7);
    assert_eq!(state.phase, BattlePhase::Resolve);
}

#[test]
fn critical_chance_and_damage_match_the_source_formula_boundaries() {
    assert_eq!(critical_hit_chance(-1), 0.0);
    assert_eq!(critical_hit_chance(1), 0.02);
    assert_eq!(critical_hit_chance(10), 0.20);
    assert_eq!(critical_hit_chance(13), 0.25);
    assert_eq!(critical_hit_chance(100), 0.25);
    assert_eq!(critical_damage(1), 1);
    assert_eq!(critical_damage(7), 10);
    assert_eq!(critical_damage(20), 30);
}

#[test]
fn seeded_physical_resolution_emits_distinct_miss_and_critical_feedback() {
    let mut critical_state = state_with(vec![
        actor(BattleSide::Party, 0, 100, 20),
        actor(BattleSide::Enemy, 0, 1, 20),
    ]);
    let action = BattleAction::Physical {
        attacker: CombatantKey::party(0),
        target: CombatantKey::enemy(0),
    };
    let event = resolve_action(
        &critical_state,
        action,
        &mut GameplayRng::from_seed(7),
    )
    .unwrap();
    assert_eq!(
        event,
        BattleEvent::Damage {
            action,
            amount: 10,
            critical: true,
            knocked_out: false,
        }
    );
    critical_state.apply_event(event);
    assert!(critical_state.message.starts_with("Critical hit!"));
    assert_eq!(
        critical_state.transcript.last().map(String::as_str),
        Some("CRITICAL Party-0 -> Enemy-0 10")
    );

    let miss_state = state_with(vec![
        actor(BattleSide::Party, 0, 1, 20),
        actor(BattleSide::Enemy, 0, 100, 20),
    ]);
    assert_eq!(
        resolve_action(&miss_state, action, &mut GameplayRng::from_seed(0)),
        Some(BattleEvent::Miss { action })
    );
}

#[test]
fn damage_clamps_to_zero_and_turn_advance_skips_ko_actors() {
    let mut target = actor(BattleSide::Party, 0, 1, 4);
    assert_eq!(target.apply_damage(99), 4);
    assert!(!target.is_alive());

    let mut state = BattleState {
        phase: BattlePhase::Advance,
        combatants: vec![
            target,
            actor(BattleSide::Party, 1, 3, 10),
            actor(BattleSide::Enemy, 0, 2, 10),
        ],
        turn_order: vec![
            CombatantKey::party(0),
            CombatantKey::party(1),
            CombatantKey::enemy(0),
        ],
        active_turn: 0,
        command_index: 0,
        target: None,
        message: String::new(),
        transcript: Vec::new(),
        flee_outcome: None,
    };
    state.skip_knocked_out();
    assert_eq!(state.active_key(), Some(CombatantKey::party(1)));
}

#[test]
fn victory_and_defeat_require_every_member_of_one_side_to_be_ko() {
    let mut state = BattleState {
        phase: BattlePhase::Resolve,
        combatants: vec![
            actor(BattleSide::Party, 0, 2, 10),
            actor(BattleSide::Enemy, 0, 1, 0),
            actor(BattleSide::Enemy, 1, 1, 1),
        ],
        turn_order: vec![],
        active_turn: 0,
        command_index: 0,
        target: None,
        message: String::new(),
        transcript: vec![],
        flee_outcome: None,
    };
    state.assess_result();
    assert_eq!(state.phase, BattlePhase::Advance);
    state.combatants[2].health = 0;
    state.assess_result();
    assert_eq!(state.phase, BattlePhase::Victory);
    state.combatants[0].health = 0;
    state.combatants[2].health = 1;
    state.assess_result();
    assert_eq!(state.phase, BattlePhase::Defeat);
}

#[test]
fn deterministic_basic_battle_transcript_is_stable() {
    fn replay(seed: u64) -> Vec<String> {
        let mut state = BattleState {
            phase: BattlePhase::Command,
            combatants: vec![
                actor(BattleSide::Party, 0, 8, 40),
                actor(BattleSide::Enemy, 0, 4, 16),
            ],
            turn_order: vec![CombatantKey::party(0), CombatantKey::enemy(0)],
            active_turn: 0,
            command_index: 0,
            target: None,
            message: String::new(),
            transcript: vec!["START fixture".to_owned()],
            flee_outcome: None,
        };
        state.combatants[0].attack = 14;
        state.combatants[1].defense = 2;
        let mut rng = GameplayRng::from_seed(seed);
        while !state.all_defeated(BattleSide::Enemy) {
            if state.active_key() == Some(CombatantKey::party(0)) {
                state.resolve_physical(CombatantKey::party(0), CombatantKey::enemy(0), &mut rng);
            } else {
                state.resolve_enemy_action(&mut rng);
            }
            state.assess_result();
            if state.phase == BattlePhase::Advance {
                state.advance(&mut rng);
            }
        }
        state.transcript
    }
    let transcript = replay(9);
    assert_eq!(transcript, replay(9));
    assert_eq!(transcript.last().map(String::as_str), Some("VICTORY"));
}
