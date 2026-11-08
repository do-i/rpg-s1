//! Exercise the production input handler at battle outcome and cancellation boundaries.

use super::*;
use crate::{
    app_state::AppStateTransitionPlugin,
    battle::tests::{actor, state_with},
    encounter::PreBattleReturnContext,
    input_record::NormalizedAction,
    save_data::{SavePayload, tests::fixture_balance},
    scenario_spatial::Position,
    test_support::persistence::fresh_game,
};
use bevy::state::app::StatesPlugin;

fn fixture(phase: BattlePhase) -> App {
    let game = fresh_game();
    let member = game.party().member("aric").unwrap();
    let mut aric = actor(BattleSide::Party, 0, 30, member.health());
    aric.id = "aric".into();
    aric.class_id = "hero".into();
    aric.mana = member.mana();
    let mut enemy = actor(BattleSide::Enemy, 0, 1, 0);
    enemy.boss = true;
    enemy.experience_yield = 30;
    enemy.gold_yield = 10;
    let mut battle = state_with(vec![aric, enemy]);
    battle.phase = phase;
    let entry = BattleEntry {
        encounter_id: "regression".into(),
        participants: vec![],
        background_id: String::new(),
        background_asset: String::new(),
        bgm_key: String::new(),
        boss_completion_flag: Some("boss_zone01_defeated".into()),
        victory_flags: vec!["story_act2_started".into()],
        barrier_messages: vec![],
        return_context: PreBattleReturnContext {
            map_id: game.map().current().unwrap().as_str().into(),
            position: game.map().position(),
            facing: game.map().facing(),
            world_bgm_key: None,
            world_enemies: vec![],
        },
    };
    let mut balances = Assets::<BalanceData>::default();
    balances.add(fixture_balance());
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(StatesPlugin)
        .insert_state(AppState::Battle)
        .add_plugins(AppStateTransitionPlugin)
        .add_message::<PlaySfx>()
        .insert_resource(balances)
        .insert_resource(FieldMenuCatalog::production_class_fixture())
        .insert_resource(ActionState::default())
        .insert_resource(deck::BattleDeck::default())
        .insert_resource(game)
        .insert_resource(entry)
        .insert_resource(battle)
        .add_systems(
            Update,
            handle_battle_input.run_if(in_state(AppState::Battle)),
        );
    app
}

fn press(app: &mut App, action: NormalizedAction) {
    app.world_mut()
        .resource_mut::<ActionState>()
        .replace_with_normalized(&[action]);
    app.update();
}

fn payload(app: &App) -> SavePayload {
    SavePayload::from_game_state(app.world().resource::<GameState>()).unwrap()
}

#[test]
fn persistence_victory_banks_rewards_once_and_restores_world_context() {
    let mut app = fixture(BattlePhase::Victory);
    let before = payload(&app);
    // A battle return must restore the captured position/facing, even if changed in battle.
    app.world_mut()
        .resource_mut::<GameState>()
        .map_mut()
        .set_position(Position::new(1, 1));
    press(&mut app, NormalizedAction::Confirm);
    assert_eq!(
        app.world().resource::<BattleState>().phase,
        BattlePhase::Rewards
    );
    assert_eq!(payload(&app).repository.gp, before.repository.gp + 10);
    assert_eq!(
        app.world()
            .resource::<BattleState>()
            .rewards
            .as_ref()
            .unwrap()
            .total_experience,
        30
    );
    assert!(
        app.world()
            .resource::<GameState>()
            .flags()
            .is_set("boss_zone01_defeated")
    );
    assert!(
        app.world()
            .resource::<GameState>()
            .flags()
            .is_set("story_act2_started")
    );
    let rewarded = payload(&app);
    press(&mut app, NormalizedAction::Confirm);
    app.update();
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::World
    );
    let restored = payload(&app);
    let mut expected = rewarded;
    expected.map = before.map;
    assert_eq!(restored, expected);
    assert!(app.world().contains_resource::<WorldEncounterRestore>());
    press(&mut app, NormalizedAction::Confirm);
    assert_eq!(
        payload(&app),
        restored,
        "World input must not replay battle rewards"
    );
}

#[test]
fn persistence_defeat_and_flee_outcomes_do_not_grant_rewards_or_flags() {
    for outcome in [
        None,
        Some(FleeOutcome::Success),
        Some(FleeOutcome::Failed),
        Some(FleeOutcome::Blocked),
    ] {
        let mut app = fixture(if outcome.is_some() {
            BattlePhase::Flee
        } else {
            BattlePhase::Defeat
        });
        app.world_mut().resource_mut::<BattleState>().flee_outcome = outcome;
        let before = payload(&app);
        press(&mut app, NormalizedAction::Confirm);
        if matches!(outcome, Some(FleeOutcome::Failed | FleeOutcome::Blocked)) {
            assert_eq!(
                app.world().resource::<BattleState>().phase,
                BattlePhase::Advance
            );
        }
        // Clear the input before the next update applies a queued scene transition.
        app.world_mut()
            .resource_mut::<ActionState>()
            .replace_with_normalized(&[]);
        app.update();
        let expected = match outcome {
            None => AppState::GameOver,
            Some(FleeOutcome::Success) => AppState::World,
            _ => AppState::Battle,
        };
        assert_eq!(*app.world().resource::<State<AppState>>().get(), expected);
        assert_eq!(
            payload(&app),
            before,
            "outcome {outcome:?} changed saved state"
        );
        assert_eq!(
            app.world().contains_resource::<WorldEncounterRestore>(),
            outcome == Some(FleeOutcome::Success)
        );
    }
}

#[test]
fn persistence_cancelled_item_and_ability_targets_do_not_spend_resources() {
    for item in [false, true] {
        let mut app = fixture(BattlePhase::Target);
        {
            let mut battle = app.world_mut().resource_mut::<BattleState>();
            battle.target = TargetSelector::new(TargetGroup::Ally, &battle.combatants, false);
            if item {
                battle.pending_item = Some("potion".into());
            } else {
                battle.pending_ability = Some(0);
            }
        }
        let before = payload(&app);
        press(&mut app, NormalizedAction::Back);
        assert_eq!(
            app.world().resource::<BattleState>().phase,
            if item {
                BattlePhase::Item
            } else {
                BattlePhase::Ability
            }
        );
        press(&mut app, NormalizedAction::Back);
        assert_eq!(
            app.world().resource::<BattleState>().phase,
            BattlePhase::Command
        );
        assert_eq!(payload(&app), before);
    }
}

#[test]
fn level_up_confirm_advances_from_loot_before_restoring_the_world() {
    let mut app = fixture(BattlePhase::Victory);
    app.world_mut()
        .resource_mut::<BattleState>()
        .combatants
        .iter_mut()
        .find(|actor| actor.key.side == BattleSide::Enemy)
        .unwrap()
        .experience_yield = 1_000;
    press(&mut app, NormalizedAction::Confirm);
    assert!(
        app.world()
            .resource::<BattleState>()
            .rewards
            .as_ref()
            .unwrap()
            .members
            .iter()
            .any(|member| !member.level_ups.is_empty())
    );
    let banked = payload(&app);
    assert_eq!(
        *app.world()
            .resource::<super::super::reward_modal::BattleRewardScreen>(),
        super::super::reward_modal::BattleRewardScreen::Loot
    );
    press(&mut app, NormalizedAction::Confirm);
    assert_eq!(
        *app.world()
            .resource::<super::super::reward_modal::BattleRewardScreen>(),
        super::super::reward_modal::BattleRewardScreen::LevelUp(0)
    );
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::Battle
    );
    assert_eq!(
        payload(&app),
        banked,
        "advancing reward screens must not award anything twice"
    );
    press(&mut app, NormalizedAction::Confirm);
    app.world_mut().resource_mut::<ActionState>().clear();
    app.update();
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::World
    );
    assert_eq!(payload(&app).repository, banked.repository);
}

#[test]
fn reward_continue_button_uses_the_same_two_step_flow_as_confirm() {
    use crate::gameplay_canvas::pointer::CanvasPointer;
    let mut app = fixture(BattlePhase::Victory);
    app.world_mut()
        .resource_mut::<BattleState>()
        .combatants
        .iter_mut()
        .find(|actor| actor.key.side == BattleSide::Enemy)
        .unwrap()
        .experience_yield = 1_000;
    press(&mut app, NormalizedAction::Confirm);
    let banked = payload(&app);
    app.world_mut().resource_mut::<ActionState>().clear();
    app.world_mut().spawn((
        super::super::reward_modal::BattleRewardContinue,
        ComputedNode {
            size: Vec2::new(160.0, 48.0),
            ..default()
        },
        UiGlobalTransform::from_xy(640.0, 500.0),
    ));
    app.insert_resource(CanvasPointer::at_viewport_position(
        Vec2::new(640.0, 500.0),
        true,
    ));
    app.update();
    assert_eq!(
        *app.world()
            .resource::<super::super::reward_modal::BattleRewardScreen>(),
        super::super::reward_modal::BattleRewardScreen::LevelUp(0)
    );
    assert_eq!(payload(&app), banked);
    app.insert_resource(CanvasPointer::at_viewport_position(
        Vec2::new(640.0, 500.0),
        false,
    ));
    app.update();
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::Battle
    );
    app.insert_resource(CanvasPointer::at_viewport_position(
        Vec2::new(640.0, 500.0),
        true,
    ));
    app.update();
    app.insert_resource(CanvasPointer::at_viewport_position(
        Vec2::new(640.0, 500.0),
        false,
    ));
    app.update();
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::World
    );
    assert_eq!(payload(&app).repository, banked.repository);
}

#[test]
fn sequential_level_ups_require_a_fresh_confirm_per_member_and_never_rebank_rewards() {
    use crate::battle::reward_modal::BattleRewardScreen;
    let mut app = fixture(BattlePhase::Victory);
    app.world_mut()
        .resource_mut::<BattleState>()
        .combatants
        .iter_mut()
        .find(|actor| actor.key.side == BattleSide::Enemy)
        .unwrap()
        .experience_yield = 1_000;
    press(&mut app, NormalizedAction::Confirm);
    let banked = payload(&app);
    // Only the presentation rows change; the already-banked game is deliberately retained.
    let template = app
        .world()
        .resource::<BattleState>()
        .rewards
        .as_ref()
        .unwrap()
        .members[0]
        .clone();
    let members = ["Aric", "Elise", "Reiya", "Jep", "Kael"]
        .into_iter()
        .enumerate()
        .map(|(index, name)| {
            let mut member = template.clone();
            member.member_id = name.to_lowercase();
            member.member_name = name.to_owned();
            if [1, 4].contains(&index) {
                member.level_ups.clear();
            }
            member
        })
        .collect();
    app.world_mut()
        .resource_mut::<BattleState>()
        .rewards
        .as_mut()
        .unwrap()
        .members = members;
    for index in [0, 2, 3] {
        app.world_mut().resource_mut::<ActionState>().clear();
        app.update();
        press(&mut app, NormalizedAction::Confirm);
        assert_eq!(
            *app.world().resource::<BattleRewardScreen>(),
            BattleRewardScreen::LevelUp(index)
        );
        assert_eq!(
            *app.world().resource::<State<AppState>>().get(),
            AppState::Battle
        );
        assert_eq!(payload(&app), banked);
        app.world_mut().resource_mut::<ActionState>().clear();
        app.update();
        app.update();
        assert_eq!(
            *app.world().resource::<BattleRewardScreen>(),
            BattleRewardScreen::LevelUp(index)
        );
    }
    press(&mut app, NormalizedAction::Confirm);
    app.world_mut().resource_mut::<ActionState>().clear();
    app.update();
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::World
    );
    assert_eq!(payload(&app).repository, banked.repository);
}

#[test]
fn confirm_during_the_victory_sting_skips_it_without_dismissing_the_loot() {
    use crate::battle::reward_modal::BattleRewardPrelude;
    let mut app = fixture(BattlePhase::Victory);
    press(&mut app, NormalizedAction::Confirm);
    app.insert_resource(BattleRewardPrelude::new());

    press(&mut app, NormalizedAction::Confirm);

    assert_eq!(
        app.world().resource::<BattleState>().phase,
        BattlePhase::Rewards
    );
    assert!(app.world().resource::<BattleRewardPrelude>().is_skipped());
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::Battle
    );
}

#[test]
fn grouped_spell_input_skips_empty_schools_and_preserves_target_cancellation() {
    let mut app = fixture(BattlePhase::Command);
    let catalog = app.world().resource::<FieldMenuCatalog>();
    let fire = catalog
        .class("sorcerer")
        .unwrap()
        .abilities
        .iter()
        .find(|ability| ability.id == "fire_bolt")
        .unwrap()
        .clone();
    let water = catalog
        .class("sorcerer")
        .unwrap()
        .abilities
        .iter()
        .find(|ability| ability.id == "aqua_shot")
        .unwrap()
        .clone();
    let heal = catalog
        .class("cleric")
        .unwrap()
        .abilities
        .iter()
        .find(|ability| ability.id == "heal")
        .unwrap()
        .clone();
    {
        let mut state = app.world_mut().resource_mut::<BattleState>();
        state.command_index = 1;
        state.combatants[0].abilities = vec![fire, water, heal];
        state.combatants[0].mana = 20;
        state.combatants[1].health = 100;
    }
    press(&mut app, NormalizedAction::Confirm);
    assert_eq!(
        app.world().resource::<BattleState>().phase,
        BattlePhase::Ability
    );
    assert_eq!(
        app.world().resource::<deck::BattleDeck>().group,
        deck::SpellGroup::Fire
    );
    assert!(!app.world().resource::<deck::BattleDeck>().list_focused);
    press(&mut app, NormalizedAction::MenuDown);
    assert_eq!(
        app.world().resource::<deck::BattleDeck>().group,
        deck::SpellGroup::Water
    );
    press(&mut app, NormalizedAction::MenuDown);
    assert_eq!(
        app.world().resource::<deck::BattleDeck>().group,
        deck::SpellGroup::Heal
    );
    press(&mut app, NormalizedAction::MenuDown);
    assert_eq!(
        app.world().resource::<deck::BattleDeck>().group,
        deck::SpellGroup::Fire
    );
    press(&mut app, NormalizedAction::MenuRight);
    assert!(app.world().resource::<deck::BattleDeck>().list_focused);
    assert_eq!(
        app.world().resource::<BattleState>().phase,
        BattlePhase::Ability
    );
    press(&mut app, NormalizedAction::Confirm);
    assert_eq!(
        app.world().resource::<BattleState>().phase,
        BattlePhase::Target
    );
    assert_eq!(app.world().resource::<BattleState>().combatants[0].mana, 20);
    press(&mut app, NormalizedAction::Back);
    assert_eq!(
        app.world().resource::<BattleState>().phase,
        BattlePhase::Ability
    );
    assert!(app.world().resource::<deck::BattleDeck>().list_focused);
    press(&mut app, NormalizedAction::MenuLeft);
    assert!(!app.world().resource::<deck::BattleDeck>().list_focused);
    press(&mut app, NormalizedAction::Back);
    assert_eq!(
        app.world().resource::<BattleState>().phase,
        BattlePhase::Command
    );
    app.world_mut().resource_mut::<BattleState>().command_index = 0;
    press(&mut app, NormalizedAction::Confirm);
    assert_eq!(
        app.world().resource::<BattleState>().phase,
        BattlePhase::Target
    );
    assert!(
        app.world()
            .resource::<BattleState>()
            .pending_ability
            .is_none()
    );
    press(&mut app, NormalizedAction::Back);
    assert_eq!(
        app.world().resource::<BattleState>().phase,
        BattlePhase::Command
    );
}
