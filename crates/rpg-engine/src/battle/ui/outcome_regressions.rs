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
