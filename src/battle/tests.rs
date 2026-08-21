use super::{
    ability::{AbilityError, ElementalAffinity, elemental_damage, resolve_ability},
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
    status::{ActiveStatus, StatusEffect},
};
use crate::{
    encounter::BattleSide,
    gameplay_rng::GameplayRng,
    scenario_balance::BalanceData,
    scenario_class::{Ability, ClassDefinition},
    scenario_enemy::EnemyType,
    scenario_party::PartyRow,
    scenario_yaml,
};

fn ability(id: &str) -> Ability {
    [
        include_str!("../../assets/scenarios/rusted_kingdoms/data/classes/cleric.yaml"),
        include_str!("../../assets/scenarios/rusted_kingdoms/data/classes/hero.yaml"),
        include_str!("../../assets/scenarios/rusted_kingdoms/data/classes/rogue.yaml"),
        include_str!("../../assets/scenarios/rusted_kingdoms/data/classes/sorcerer.yaml"),
        include_str!("../../assets/scenarios/rusted_kingdoms/data/classes/warrior.yaml"),
    ]
    .into_iter()
    .map(|document| scenario_yaml::from_str::<ClassDefinition>(document).unwrap())
    .flat_map(|class| class.abilities)
    .find(|ability| ability.id == id)
    .unwrap_or_else(|| panic!("missing ability fixture {id}"))
}

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
        magic_resistance: 2,
        dexterity: dex,
        abilities: Vec::new(),
        status_effects: Vec::new(),
        row: PartyRow::Front,
        boss: false,
        enemy_type: None,
        immunities: Vec::new(),
        behavior: None,
        experience_yield: 0,
        drops: None,
    }
}

fn state_with(combatants: Vec<BattleCombatant>) -> BattleState {
    BattleState {
        phase: BattlePhase::Command,
        turn_order: calculate_turn_order(&combatants),
        combatants,
        active_turn: 0,
        command_index: 0,
        ability_index: 0,
        pending_ability: None,
        target: None,
        message: String::new(),
        transcript: Vec::new(),
        feedback_events: Vec::new(),
        flee_outcome: None,
    }
}

#[test]
fn phase_graph_names_and_bounds_every_minimum_loop_transition() {
    assert!(BattlePhase::Start.allows(BattlePhase::Command));
    assert!(BattlePhase::Command.allows(BattlePhase::Ability));
    assert!(BattlePhase::Ability.allows(BattlePhase::Target));
    assert!(BattlePhase::Target.allows(BattlePhase::Ability));
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
fn elemental_affinity_scales_weak_neutral_and_resistant_damage() {
    assert_eq!(elemental_damage(11, ElementalAffinity::Weak), 16);
    assert_eq!(elemental_damage(11, ElementalAffinity::Neutral), 11);
    assert_eq!(elemental_damage(11, ElementalAffinity::Resistant), 5);
    assert_eq!(elemental_damage(1, ElementalAffinity::Resistant), 1);
}

#[test]
fn offensive_spell_charges_once_and_resolves_single_and_all_target_shapes() {
    let mut caster = actor(BattleSide::Party, 0, 10, 40);
    caster.magic_resistance = 15;
    caster.mana = 30;
    caster.max_mana = 30;
    caster.abilities = vec![ability("fire_bolt"), ability("flame_wall")];
    let mut first = actor(BattleSide::Enemy, 0, 1, 50);
    first.magic_resistance = 3;
    let second = actor(BattleSide::Enemy, 1, 1, 50);
    let mut state = state_with(vec![caster, first, second]);

    let events = resolve_ability(
        &mut state,
        CombatantKey::party(0),
        0,
        &[CombatantKey::enemy(0)],
        &mut GameplayRng::from_seed(2),
    )
    .unwrap();
    assert_eq!(state.actor(CombatantKey::party(0)).unwrap().mana, 26);
    assert_eq!(state.actor(CombatantKey::enemy(0)).unwrap().health, 38);
    assert!(matches!(
        events.as_slice(),
        [BattleEvent::MagicDamage { amount: 12, .. }]
    ));

    state.phase = BattlePhase::Ability;
    let events = resolve_ability(
        &mut state,
        CombatantKey::party(0),
        1,
        &[CombatantKey::enemy(0), CombatantKey::enemy(1)],
        &mut GameplayRng::from_seed(3),
    )
    .unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, BattleEvent::MagicDamage { .. }))
            .count(),
        2
    );
    assert_eq!(state.actor(CombatantKey::party(0)).unwrap().mana, 6);
}

#[test]
fn holy_spell_uses_enemy_taxonomy_affinity() {
    let mut caster = actor(BattleSide::Party, 0, 10, 40);
    caster.magic_resistance = 20;
    caster.mana = 20;
    caster.max_mana = 20;
    let mut spell = ability("earth_spike");
    if let crate::scenario_class::AbilityKind::Spell(value) = &mut spell.kind {
        value.element = crate::scenario_class::AbilityElement::Holy;
    }
    caster.abilities.push(spell);
    let mut undead = actor(BattleSide::Enemy, 0, 1, 50);
    undead.magic_resistance = 4;
    undead.enemy_type = Some(EnemyType::Undead);
    let mut state = state_with(vec![caster, undead]);

    let events = resolve_ability(
        &mut state,
        CombatantKey::party(0),
        0,
        &[CombatantKey::enemy(0)],
        &mut GameplayRng::from_seed(4),
    )
    .unwrap();
    assert!(matches!(
        events.as_slice(),
        [BattleEvent::MagicDamage { amount: 24, .. }]
    ));
}

#[test]
fn invalid_casts_do_not_spend_mana_or_mutate_targets() {
    let mut caster = actor(BattleSide::Party, 0, 10, 40);
    caster.mana = 3;
    caster.max_mana = 30;
    caster.abilities.push(ability("fire_bolt"));
    let enemy = actor(BattleSide::Enemy, 0, 1, 50);
    let mut state = state_with(vec![caster, enemy]);

    assert_eq!(
        resolve_ability(
            &mut state,
            CombatantKey::party(0),
            0,
            &[CombatantKey::enemy(0)],
            &mut GameplayRng::from_seed(1),
        ),
        Err(AbilityError::InsufficientMana)
    );
    assert_eq!(state.actor(CombatantKey::party(0)).unwrap().mana, 3);
    assert_eq!(state.actor(CombatantKey::enemy(0)).unwrap().health, 50);

    state.actor_mut(CombatantKey::party(0)).unwrap().mana = 30;
    assert_eq!(
        resolve_ability(
            &mut state,
            CombatantKey::party(0),
            0,
            &[CombatantKey::party(0)],
            &mut GameplayRng::from_seed(1),
        ),
        Err(AbilityError::InvalidTarget)
    );
    assert_eq!(state.actor(CombatantKey::party(0)).unwrap().mana, 30);
}

#[test]
fn healing_caps_living_hp_and_revive_only_accepts_ko_allies() {
    let mut caster = actor(BattleSide::Party, 0, 10, 80);
    caster.magic_resistance = 20;
    caster.mana = 40;
    caster.max_mana = 40;
    caster.abilities = vec![ability("heal"), ability("revive")];
    let mut living = actor(BattleSide::Party, 1, 5, 100);
    living.health = 90;
    let mut fallen = actor(BattleSide::Party, 2, 4, 100);
    fallen.health = 0;
    let mut state = state_with(vec![caster, living, fallen]);

    let heal = resolve_ability(
        &mut state,
        CombatantKey::party(0),
        0,
        &[CombatantKey::party(1)],
        &mut GameplayRng::from_seed(1),
    )
    .unwrap();
    assert!(matches!(
        heal.as_slice(),
        [BattleEvent::Heal {
            amount: 10,
            revived: false,
            ..
        }]
    ));
    assert_eq!(state.actor(CombatantKey::party(1)).unwrap().health, 100);

    state.phase = BattlePhase::Ability;
    let revive = resolve_ability(
        &mut state,
        CombatantKey::party(0),
        1,
        &[CombatantKey::party(2)],
        &mut GameplayRng::from_seed(1),
    )
    .unwrap();
    assert!(matches!(
        revive.as_slice(),
        [BattleEvent::Heal {
            amount: 50,
            revived: true,
            ..
        }]
    ));
    let mana = state.actor(CombatantKey::party(0)).unwrap().mana;
    assert_eq!(
        resolve_ability(
            &mut state,
            CombatantKey::party(0),
            1,
            &[CombatantKey::party(1)],
            &mut GameplayRng::from_seed(1),
        ),
        Err(AbilityError::InvalidTarget)
    );
    assert_eq!(state.actor(CombatantKey::party(0)).unwrap().mana, mana);
}

#[test]
fn buffs_debuffs_refresh_and_expire_at_end_of_affected_turn() {
    let mut caster = actor(BattleSide::Party, 0, 10, 40);
    caster.mana = 30;
    caster.max_mana = 30;
    caster.abilities = vec![ability("rally"), ability("war_cry")];
    let ally = actor(BattleSide::Party, 1, 5, 40);
    let enemy = actor(BattleSide::Enemy, 0, 1, 40);
    let mut state = state_with(vec![caster, ally, enemy]);
    state.actor_mut(CombatantKey::party(1)).unwrap().defense = 10;

    resolve_ability(
        &mut state,
        CombatantKey::party(0),
        0,
        &[CombatantKey::party(0), CombatantKey::party(1)],
        &mut GameplayRng::from_seed(1),
    )
    .unwrap();
    assert_eq!(
        state
            .actor(CombatantKey::party(1))
            .unwrap()
            .effective_defense(),
        12
    );
    let defense = state.actor(CombatantKey::party(1)).unwrap().status_effects[0];
    assert_eq!(defense.remaining_turns, Some(3));

    state.phase = BattlePhase::Ability;
    resolve_ability(
        &mut state,
        CombatantKey::party(0),
        1,
        &[CombatantKey::enemy(0)],
        &mut GameplayRng::from_seed(1),
    )
    .unwrap();
    assert_eq!(
        state
            .actor(CombatantKey::enemy(0))
            .unwrap()
            .effective_attack(),
        8
    );
    assert_eq!(
        state
            .actor_mut(CombatantKey::enemy(0))
            .unwrap()
            .tick_statuses()
            .expired,
        0
    );
    assert_eq!(
        state
            .actor_mut(CombatantKey::enemy(0))
            .unwrap()
            .tick_statuses()
            .expired,
        1
    );
    assert_eq!(
        state
            .actor(CombatantKey::enemy(0))
            .unwrap()
            .effective_attack(),
        10
    );
}

#[test]
fn construct_taxonomy_rejects_harmful_debuff_statuses() {
    let mut caster = actor(BattleSide::Party, 0, 10, 40);
    caster.mana = 10;
    caster.max_mana = 10;
    caster.abilities.push(ability("war_cry"));
    let mut construct = actor(BattleSide::Enemy, 0, 1, 40);
    construct.enemy_type = Some(EnemyType::Construct);
    let mut state = state_with(vec![caster, construct]);

    let events = resolve_ability(
        &mut state,
        CombatantKey::party(0),
        0,
        &[CombatantKey::enemy(0)],
        &mut GameplayRng::from_seed(1),
    )
    .unwrap();
    assert!(events.is_empty());
    assert!(
        state
            .actor(CombatantKey::enemy(0))
            .unwrap()
            .status_effects
            .is_empty()
    );
}

#[test]
fn defensive_buffs_reduce_or_redirect_physical_damage() {
    let mut warrior = actor(BattleSide::Party, 0, 20, 100);
    warrior.abilities = vec![ability("cover"), ability("fortress_stance")];
    let ally = actor(BattleSide::Party, 1, 10, 100);
    let mut enemy = actor(BattleSide::Enemy, 0, 100, 100);
    enemy.attack = 23;
    let mut state = state_with(vec![warrior, ally, enemy]);

    resolve_ability(
        &mut state,
        CombatantKey::party(0),
        0,
        &[CombatantKey::party(1)],
        &mut GameplayRng::from_seed(1),
    )
    .unwrap();
    state.resolve_physical(
        CombatantKey::enemy(0),
        CombatantKey::party(1),
        &mut GameplayRng::from_seed(9),
    );
    assert_eq!(state.actor(CombatantKey::party(0)).unwrap().health, 80);
    assert_eq!(state.actor(CombatantKey::party(1)).unwrap().health, 100);

    state.phase = BattlePhase::Ability;
    resolve_ability(
        &mut state,
        CombatantKey::party(0),
        1,
        &[CombatantKey::party(0)],
        &mut GameplayRng::from_seed(1),
    )
    .unwrap();
    state.resolve_physical(
        CombatantKey::enemy(0),
        CombatantKey::party(0),
        &mut GameplayRng::from_seed(9),
    );
    assert_eq!(state.actor(CombatantKey::party(0)).unwrap().health, 70);
    assert_eq!(
        state
            .actor(CombatantKey::party(0))
            .unwrap()
            .skip_turn_reason(),
        Some(StatusEffect::DamageReduction)
    );
}

#[test]
fn sleep_wakes_on_damage_stun_skips_and_silence_hides_abilities() {
    let mut caster = actor(BattleSide::Party, 0, 10, 40);
    caster.abilities.push(ability("heal"));
    caster.add_status(ActiveStatus::timed(StatusEffect::Silence, 2));
    let mut state = state_with(vec![caster, actor(BattleSide::Enemy, 0, 1, 40)]);
    assert!(!state.command_available(super::model::BattleCommand::Spell));

    let caster = state.actor_mut(CombatantKey::party(0)).unwrap();
    caster.remove_status(StatusEffect::Silence);
    caster.add_status(ActiveStatus::timed(StatusEffect::Sleep, 2));
    assert_eq!(caster.skip_turn_reason(), Some(StatusEffect::Sleep));
    caster.apply_damage(1);
    assert!(!caster.has_status(StatusEffect::Sleep));
    caster.add_status(ActiveStatus::timed(StatusEffect::Stun, 1));
    assert_eq!(caster.skip_turn_reason(), Some(StatusEffect::Stun));
    assert_eq!(caster.tick_statuses().expired, 1);
    assert!(caster.skip_turn_reason().is_none());
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
    let event = resolve_action(&critical_state, action, &mut GameplayRng::from_seed(7)).unwrap();
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
        ability_index: 0,
        pending_ability: None,
        target: None,
        message: String::new(),
        transcript: Vec::new(),
        feedback_events: Vec::new(),
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
        ability_index: 0,
        pending_ability: None,
        target: None,
        message: String::new(),
        transcript: vec![],
        feedback_events: Vec::new(),
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
            ability_index: 0,
            pending_ability: None,
            target: None,
            message: String::new(),
            transcript: vec!["START fixture".to_owned()],
            feedback_events: Vec::new(),
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
