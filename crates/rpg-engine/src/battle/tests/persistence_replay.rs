use std::fs;

use bevy::{prelude::*, state::app::StatesPlugin};

use super::*;
use crate::{
    action_input::{ActionInputPlugin, ActionState, AppAction},
    app_state::AppState,
    input_record::{InputAutomation, InputRecord, InputRecordPlugin, RecordSource},
    save_data::{SavePayload, tests::fixture_balance},
    save_store::SaveStore,
    test_support::persistence::{FixtureDirectory, fresh_game, load, save},
};

fn battle_step(actions: Res<ActionState>, mut game: ResMut<GameState>) {
    if !actions.just_pressed(AppAction::Confirm) {
        return;
    }
    // A resettable combat fixture: seeded encounter choice, actual physical resolution,
    // then the production EXP/GP/loot transaction. Live map timing remains a manual gate.
    let enemy_id = if game.rng_mut().next_u64().is_multiple_of(2) {
        "goblin"
    } else {
        "goblin_warrior"
    };
    let member = game.party().member("aric").unwrap();
    let mut aric = actor(BattleSide::Party, 0, 30, member.health());
    aric.id = "aric".into();
    aric.class_id = "hero".into();
    aric.mana = member.mana();
    aric.attack = 100;
    let mut enemy = reward_enemy(enemy_id, 0);
    enemy.health = 12;
    let mut battle = state_with(vec![aric, enemy]);
    for _ in 0..64 {
        battle.resolve_physical(
            CombatantKey::party(0),
            CombatantKey::enemy(0),
            game.rng_mut(),
        );
        battle.assess_result();
        if battle.phase == BattlePhase::Victory {
            break;
        }
    }
    assert_eq!(battle.phase, BattlePhase::Victory);
    let rewards = apply_rewards(
        &mut battle,
        &mut game,
        &FieldMenuCatalog::production_class_fixture(),
        &fixture_balance(),
        None,
        &[],
    )
    .unwrap();
    assert!(rewards.total_experience > 0);
    assert!(rewards.gp_gained > 0);
    assert!(!rewards.loot.is_empty());
    let before = game.clone();
    assert!(matches!(
        apply_rewards(
            &mut battle,
            &mut game,
            &FieldMenuCatalog::production_class_fixture(),
            &fixture_balance(),
            None,
            &[]
        ),
        Err(RewardError::AlreadyApplied)
    ));
    assert_eq!(*game, before);
}

fn app(automation: InputAutomation, seed: u64) -> App {
    let mut game = fresh_game();
    *game.rng_mut() = GameplayRng::from_seed(seed);
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(StatesPlugin)
        .add_message::<AppExit>()
        .init_resource::<ButtonInput<KeyCode>>()
        .insert_state(AppState::World)
        .insert_resource(game)
        .insert_resource(automation)
        .add_plugins(ActionInputPlugin)
        .add_plugins(InputRecordPlugin)
        .add_systems(Update, battle_step);
    app
}

#[test]
fn seeded_gameplay_record_replay_and_corrupted_copy_across_processes() {
    let directory = FixtureDirectory::new("replay");
    let child = "battle::tests::persistence_replay::replay_child";
    for phase in ["record", "repeat", "different", "replay", "corrupt"] {
        directory.child(child, phase);
    }
    let read = |name: &str| {
        InputRecord::decode(&fs::read(directory.0.join(format!("{name}.yaml"))).unwrap()).unwrap()
    };
    let original = read("record");
    assert_eq!(original, read("repeat"));
    let different = read("different");
    assert_eq!(original.action_frames.len(), 3);
    assert!(
        original
            .action_frames
            .iter()
            .zip(&different.action_frames)
            .all(|(left, right)| left.actions == right.actions)
    );
    assert_ne!(
        original.action_frames[0].state_hash,
        different.action_frames[0].state_hash
    );
    let store = SaveStore::new(directory.0.join("saves"));
    let recorded = SavePayload::from_game_state(&load(&store, 1)).unwrap();
    assert_eq!(
        recorded,
        SavePayload::from_game_state(&load(&store, 2)).unwrap()
    );
    assert_eq!(
        recorded,
        SavePayload::from_game_state(&load(&store, 4)).unwrap()
    );
    let mut different_game = SavePayload::from_game_state(&load(&store, 3)).unwrap();
    assert_ne!(recorded.rng_state, different_game.rng_state);
    // A different seed must alter gameplay, not merely the saved RNG cursor.
    different_game.rng_state = recorded.rng_state;
    assert_ne!(recorded, different_game);
}

#[test]
fn replay_child() {
    let Ok(phase) = std::env::var("RPG_S1_REGRESSION_CHILD") else {
        return;
    };
    let root = std::path::PathBuf::from(std::env::var_os("RPG_S1_REGRESSION_ROOT").unwrap());
    let store = SaveStore::new(root.join("saves"));
    if ["record", "repeat", "different"].contains(&phase.as_str()) {
        let seed = if phase == "different" { 43 } else { 42 };
        let path = root.join(format!("{phase}.yaml"));
        let record = InputRecord::new(
            "regression",
            RecordSource {
                package_key: "rusted_kingdoms".into(),
                scenario_id: "my_rpg_story".into(),
                scenario_version: "1.0.0".into(),
            },
            seed,
        );
        let mut app = app(InputAutomation::record(path.clone(), record), seed);
        app.update();
        for _ in 0..3 {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::Enter);
            app.update();
            let InputAutomation::Record { failure, .. } = app.world().resource::<InputAutomation>()
            else {
                unreachable!()
            };
            assert_eq!(failure, &None);
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.release(KeyCode::Enter);
            keys.clear();
            app.update();
        }
        let record = InputRecord::decode(&fs::read(path).unwrap()).unwrap();
        assert_eq!(
            record
                .action_frames
                .iter()
                .map(|frame| frame.frame)
                .collect::<Vec<_>>(),
            [1, 3, 5]
        );
        let slot = match phase.as_str() {
            "record" => 1,
            "repeat" => 2,
            _ => 3,
        };
        save(&store, slot, app.world().resource::<GameState>());
    } else {
        let original = root.join("record.yaml");
        let original_bytes = fs::read(&original).unwrap();
        let mut record = InputRecord::decode(&original_bytes).unwrap();
        let path = if phase == "corrupt" {
            // Preserve a valid prefix; two corrupt hashes prove we stop at the first one.
            record.action_frames[1].state_hash = "0".repeat(64);
            record.action_frames[2].state_hash = "f".repeat(64);
            let copy = root.join("corrupt.yaml");
            fs::write(&copy, record.encode().unwrap()).unwrap();
            let malformed = root.join("malformed.yaml");
            fs::write(&malformed, b"format_version: [").unwrap();
            assert!(InputRecord::decode(&fs::read(&malformed).unwrap()).is_err());
            copy
        } else {
            assert_eq!(phase, "replay");
            original.clone()
        };
        let input_bytes = fs::read(&path).unwrap();
        let seed = record.seed;
        let mut replay = app(
            InputAutomation::replay(path.clone(), InputRecord::decode(&input_bytes).unwrap()),
            seed,
        );
        // Replay must suppress unrelated physical gameplay input.
        replay
            .world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        for _ in 0..10 {
            replay.update();
            let InputAutomation::Replay {
                cursor,
                failure,
                complete,
                ..
            } = replay.world().resource::<InputAutomation>()
            else {
                unreachable!()
            };
            if phase == "corrupt" && failure.is_some() {
                assert_eq!(*cursor, 1);
                assert!(!complete);
                assert!(
                    failure
                        .as_ref()
                        .unwrap()
                        .starts_with("action 1 at frame 3:")
                );
                assert!(matches!(replay.should_exit(), Some(AppExit::Error(_))));
                break;
            }
            assert_eq!(failure, &None);
            if *complete {
                assert_eq!(*cursor, 3);
                assert_eq!(phase, "replay");
                assert!(matches!(replay.should_exit(), Some(AppExit::Success)));
                save(&store, 4, replay.world().resource::<GameState>());
                break;
            }
        }
        let InputAutomation::Replay {
            failure, complete, ..
        } = replay.world().resource::<InputAutomation>()
        else {
            unreachable!()
        };
        assert!(if phase == "corrupt" {
            failure.is_some()
        } else {
            *complete
        });
        assert_eq!(fs::read(original).unwrap(), original_bytes);
        assert_eq!(fs::read(path).unwrap(), input_bytes);
    }
}
