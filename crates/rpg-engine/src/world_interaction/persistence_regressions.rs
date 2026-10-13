//! Authored dialogue rewards exercised through disk saves and fresh OS processes.

use std::fs;

use super::*;
use crate::{
    runtime_quest::QuestStatus,
    save_data::SavePayload,
    save_store::SaveStore,
    scenario_quest::QuestCatalogFile,
    test_support::persistence::{FixtureDirectory, fresh_game, load, save},
};

fn session(id: &str, game: &GameState) -> DialogueSession {
    let path = crate::test_support::scenario_package_dir().join(format!("data/dialogue/{id}.yaml"));
    let DialogueDocument::Entries(document) =
        scenario_yaml::from_str(&fs::read_to_string(path).unwrap()).unwrap()
    else {
        panic!("expected entry dialogue");
    };
    DialogueSession::resolve(id, None, document, game.flags())
        .unwrap()
        .unwrap()
}

fn apply(event: DialogueEvent, game: &mut GameState) {
    if let DialogueEvent::Apply(actions) = event {
        for action in actions {
            apply_dialogue_actions(
                &action,
                game,
                None,
                None,
                &FieldMenuCatalog::production_class_fixture(),
            )
            .unwrap();
        }
    }
}

fn finish(session: &mut DialogueSession, game: &mut GameState) {
    for _ in 0..128 {
        if session.phase() == DialoguePhase::Closed {
            assert_eq!(session.confirm(game.flags()), DialogueEvent::None);
            return;
        }
        assert_ne!(session.phase(), DialoguePhase::Choosing);
        apply(session.confirm(game.flags()), game);
    }
    panic!("dialogue did not terminate");
}

fn choices(session: &mut DialogueSession, game: &mut GameState) {
    for _ in 0..32 {
        if session.phase() == DialoguePhase::Choosing {
            return;
        }
        apply(session.confirm(game.flags()), game);
    }
    panic!("Pike never offered choices");
}

fn talk(id: &str, game: &mut GameState) {
    finish(&mut session(id, game), game);
}

fn quest(game: &GameState, id: &str) -> QuestStatus {
    let quests: QuestCatalogFile =
        scenario_yaml::from_str(include_str!(scenario_file!("data/quests.yaml"))).unwrap();
    game.quest_status(
        quests
            .entries()
            .iter()
            .find(|quest| quest.id == id)
            .unwrap(),
    )
}

fn save_operator_fixture(store: &SaveStore, slot: usize, game: &GameState) {
    crate::debug_launch::validate_debug_launch(
        &crate::test_support::scenario_package_dir(),
        &crate::debug_launch::DebugLaunchConfig {
            start_map: Some(game.map().current().unwrap().as_str().into()),
            start_position: Some(game.map().position()),
            ..default()
        },
    )
    .expect("operator fixture must start on a walkable tile");
    save(store, slot, game);
}

const BOUNDARIES: [(&str, &str); 6] = [
    ("story_quest_started", "town_01_ardel"),
    ("story_act2_started", "town_02_millhaven"),
    ("story_act3_started", "town_04_frostholm"),
    ("story_act4_started", "town_05_ashenveil"),
    ("boss_zone10_defeated", "zone_10_hearth_descent_01"),
    ("game_complete", "town_01_ardel_epilogue"),
];

#[test]
fn authored_rewards_and_campaign_boundaries_survive_fresh_processes() {
    let directory = FixtureDirectory::new("persistence");
    let child = "world_interaction::persistence_regressions::persistence_child";
    for phase in ["seed", "advance", "reopen"] {
        directory.child(child, phase);
    }
    // This opt-in exports resettable, generated saves for the operator checklist. Never
    // replace an existing fixture/profile directory or touch a player's normal save root.
    if let Some(destination) = std::env::var_os("RPG_S1_RF10_EXPORT") {
        let destination = std::path::PathBuf::from(destination);
        fs::create_dir(&destination).expect("choose a new RF-10 export directory");
        for entry in fs::read_dir(&directory.0).unwrap() {
            let path = entry.unwrap().path();
            if path
                .extension()
                .is_some_and(|extension| extension == "yaml")
            {
                fs::copy(&path, destination.join(path.file_name().unwrap())).unwrap();
            }
        }
    }
}

#[test]
fn persistence_child() {
    let Ok(phase) = std::env::var("RPG_S1_REGRESSION_CHILD") else {
        return;
    };
    let root = std::path::PathBuf::from(std::env::var_os("RPG_S1_REGRESSION_ROOT").unwrap());
    let store = SaveStore::new(root.clone());
    if phase == "seed" {
        fs::write(root.join("writer-pid"), std::process::id().to_string()).unwrap();
        let mut game = fresh_game();
        game.flags_mut().set("story_act2_started");
        game.map_mut().move_to(
            RuntimeMapId::try_new("zone_01_starting_forest").unwrap(),
            Position::new(3, 8),
            CardinalDirection::Right,
        );
        save_operator_fixture(&store, 1, &game);
        game.flags_mut().set("sq_smith_done");
        save_operator_fixture(&store, 2, &game);

        let mut game = fresh_game();
        game.flags_mut().set("boss_zone01_defeated");
        game.map_mut().move_to(
            RuntimeMapId::try_new("town_01_ardel_house_01").unwrap(),
            Position::new(10, 4),
            CardinalDirection::Right,
        );
        save_operator_fixture(&store, 3, &game);

        let mut game = fresh_game();
        assert_eq!(quest(&game, "sq_glyph"), QuestStatus::Inactive);
        talk("ruinwatch_archivist", &mut game);
        assert_eq!(quest(&game, "sq_glyph"), QuestStatus::Active);
        talk("ruinwatch_mason", &mut game);
        assert!(game.flags().is_set("sq_glyph_relayed"));
        game.map_mut().move_to(
            RuntimeMapId::try_new("town_03_ruinwatch").unwrap(),
            Position::new(12, 6),
            CardinalDirection::Down,
        );
        save_operator_fixture(&store, 4, &game);

        // These are serialization probes, deliberately synthesized, not evidence that a
        // campaign reached these places. The operator must create natural boundary saves.
        let mut boundary = fresh_game();
        for (index, (flag, map)) in BOUNDARIES.iter().enumerate() {
            boundary.flags_mut().set(*flag);
            boundary.map_mut().move_to(
                RuntimeMapId::try_new(*map).unwrap(),
                Position::new(10, 10),
                CardinalDirection::Down,
            );
            boundary.rng_mut().next_u64();
            save(&store, 10 + index, &boundary);
        }
    } else if phase == "advance" {
        assert_ne!(
            fs::read_to_string(root.join("writer-pid")).unwrap(),
            std::process::id().to_string()
        );
        fs::write(root.join("advancer-pid"), std::process::id().to_string()).unwrap();
        let mut game = load(&store, 1);
        let before = game.clone();
        let mut pike = session("bridge_guard_zone5", &game);
        choices(&mut pike, &mut game);
        assert!(!pike.choices()[1].enabled());
        pike.move_choice(1);
        assert_eq!(pike.confirm(game.flags()), DialogueEvent::Blocked);
        assert_eq!(game, before);
        assert_eq!(pike.cancel(), DialogueEvent::Cancelled);
        assert_eq!(game, before);
        let mut pike = session("bridge_guard_zone5", &game);
        choices(&mut pike, &mut game);
        apply(pike.confirm(game.flags()), &mut game);
        finish(&mut pike, &mut game);
        assert!(game.flags().is_set("bridge_zone5_pledged"));
        assert_eq!(game.repository(), before.repository());
        save(&store, 21, &game);

        let mut game = load(&store, 2);
        let before = game.clone();
        let mut pike = session("bridge_guard_zone5", &game);
        choices(&mut pike, &mut game);
        pike.move_choice(1);
        assert!(pike.choices()[1].enabled());
        apply(pike.confirm(game.flags()), &mut game);
        finish(&mut pike, &mut game);
        assert!(game.flags().is_set("bridge_zone5_vouched"));
        assert_eq!(
            game.repository().item_count("potion"),
            before.repository().item_count("potion") + 3
        );
        assert_eq!(
            game.repository().item_count("antidote"),
            before.repository().item_count("antidote") + 2
        );
        save(&store, 22, &game);

        let mut game = load(&store, 1);
        let before = game.clone();
        let mut pike = session("bridge_guard_zone5", &game);
        choices(&mut pike, &mut game);
        pike.move_choice(2);
        apply(pike.confirm(game.flags()), &mut game);
        finish(&mut pike, &mut game);
        // Authored turn-back has no effects and the offer remains available.
        assert_eq!(game, before);
        save(&store, 25, &game);

        let mut game = load(&store, 3);
        let before = game.clone();
        talk("elder_intro", &mut game);
        assert!(game.flags().is_set("npc_elder_reward_given"));
        assert_eq!(quest(&game, "main_act1"), QuestStatus::Completed);
        assert_eq!(quest(&game, "main_act2"), QuestStatus::Active);
        assert_eq!(
            game.repository().item_count("hi_potion"),
            before.repository().item_count("hi_potion") + 2
        );
        assert_eq!(
            game.repository().item_count("tent"),
            before.repository().item_count("tent") + 1
        );
        save(&store, 23, &game);

        let mut game = load(&store, 4);
        assert_eq!(quest(&game, "sq_glyph"), QuestStatus::Active);
        let ethers = game.repository().item_count("ether");
        talk("ruinwatch_archivist", &mut game);
        assert_eq!(quest(&game, "sq_glyph"), QuestStatus::Completed);
        assert_eq!(game.repository().item_count("ether"), ethers + 2);
        save(&store, 24, &game);
    } else {
        assert_eq!(phase, "reopen");
        for filename in ["writer-pid", "advancer-pid"] {
            assert_ne!(
                fs::read_to_string(root.join(filename)).unwrap(),
                std::process::id().to_string()
            );
        }
        for (slot, id) in [
            (21, "bridge_guard_zone5"),
            (22, "bridge_guard_zone5"),
            (23, "elder_intro"),
            (24, "ruinwatch_archivist"),
        ] {
            let mut game = load(&store, slot);
            let before = SavePayload::from_game_state(&game).unwrap();
            talk(id, &mut game);
            talk(id, &mut game);
            assert_eq!(
                SavePayload::from_game_state(&game).unwrap(),
                before,
                "reward/flag replay in slot {slot}"
            );
        }
        let mut game = load(&store, 25);
        let mut pike = session("bridge_guard_zone5", &game);
        choices(&mut pike, &mut game);
        assert_eq!(game, load(&store, 1));
        for (index, (flag, map)) in BOUNDARIES.iter().enumerate() {
            let game = load(&store, 10 + index);
            assert!(game.flags().is_set(flag));
            assert_eq!(game.map().current().unwrap().as_str(), *map);
            for (prior_flag, _) in BOUNDARIES.iter().take(index) {
                assert!(game.flags().is_set(prior_flag));
            }
            let path = store.slot_path(10 + index).unwrap();
            let bytes = fs::read(&path).unwrap();
            assert!(
                store
                    .load(
                        10 + index,
                        "different_campaign",
                        "1.0.0",
                        &crate::save_data::tests::fixture_balance()
                    )
                    .is_err()
            );
            assert_eq!(fs::read(path).unwrap(), bytes);
        }
    }
}
