//! Applies only the authored map transition from a completed new-game introduction.

use bevy::prelude::*;

use crate::{
    app_state::{AppState, AppStateTransitionRequest},
    game_state::GameState,
    intro_completion::IntroCompletionSet,
    intro_dialogue::{IntroDialogueCompleted, IntroDialogueSet},
    runtime_map::RuntimeMapId,
    scenario_dialogue::DialogueFade,
};

pub(crate) struct IntroTransitionPlugin;

impl Plugin for IntroTransitionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<IntroTransitionState>()
            .add_systems(OnEnter(AppState::Dialogue), reset_intro_transition)
            .add_systems(
                Update,
                apply_intro_transition
                    .after(IntroDialogueSet::Advance)
                    .after(IntroCompletionSet::Flags),
            );
    }
}

#[derive(Debug, Default, Resource)]
struct IntroTransitionState {
    applied: bool,
}

fn reset_intro_transition(mut state: ResMut<IntroTransitionState>) {
    state.applied = false;
}

fn apply_intro_transition(
    mut completions: MessageReader<IntroDialogueCompleted>,
    app_state: Option<Res<State<AppState>>>,
    game: Option<ResMut<GameState>>,
    mut state: ResMut<IntroTransitionState>,
    mut transitions: MessageWriter<AppStateTransitionRequest>,
) {
    let in_dialogue = app_state
        .as_deref()
        .is_some_and(|state| state.get() == &AppState::Dialogue);
    let Some(mut game) = game.filter(|_| in_dialogue) else {
        completions.clear();
        return;
    };

    for completion in completions.read() {
        if state.applied {
            continue;
        }
        let Some(authored) = completion.on_complete().transition.as_ref() else {
            continue;
        };
        let Ok(map) = RuntimeMapId::try_new(authored.map.clone()) else {
            continue;
        };

        // The flag reader runs first and the state request is consumed in PostUpdate, so
        // the complete logical arrival is installed before World entry.
        let facing = authored.facing.unwrap_or_else(|| game.map().facing());
        game.map_mut().move_to(map, authored.position, facing);
        state.applied = true;
        match authored.fade {
            // WorldTransitionPlugin holds black until publication, then fades in and unlocks.
            DialogueFade::In => {
                transitions.write(AppStateTransitionRequest::new(AppState::World));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::*;
    use crate::{
        game_state::GameState,
        gameplay_rng::{DEFAULT_GAMEPLAY_SEED, GameplayRng},
        intro_dialogue::IntroDialogueViewState,
        name_entry::{NameEntryConfirmed, NameEntryViewState},
        scenario_dialogue::DialogueActions,
        scenario_new_game_assets::{ActiveNewGameInputs, ActiveNewGameInputsStatus},
        scenario_root::ScenarioRoot,
        scenario_spatial::{CardinalDirection, Position},
        scenario_yaml,
        test_support::headless_title_app_with_asset_base,
    };

    static NEXT_TEMPORARY_ID: AtomicU64 = AtomicU64::new(0);

    const INTRO: &str = r#"id: invented_transition_intro
type: cutscene
lines:
  - "An invented first line."
  - "An invented final line."
on_complete:
  set_flag: invented_transition_flag
  transition:
    map: invented_arrival
    position: [8, 9]
    fade: in
"#;

    struct InventedPackage(PathBuf);

    impl InventedPackage {
        fn new() -> Self {
            let unique = NEXT_TEMPORARY_ID.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "rpg-s1-intro-transition-{}-{unique}",
                std::process::id()
            ));
            let package = root.join("scenarios/invented_transition");
            fs::create_dir_all(package.join("data/dialogue")).expect("invented dialogue directory");
            fs::write(
                package.join("manifest.yaml"),
                include_str!("../../../tests/fixtures/rusted-kingdoms-manifest-complete.yaml"),
            )
            .expect("invented manifest");
            let party = include_str!("../../../tests/fixtures/party-catalog-shapes.yaml")
                .replacen("id: ember", "id: aric", 1)
                .replacen("class: vanguard", "class: hero", 1);
            fs::write(package.join("data/party.yaml"), party).expect("invented party");
            fs::write(
                package.join("data/balance.yaml"),
                include_str!("../../../tests/fixtures/balance-complete.yaml"),
            )
            .expect("invented balance");
            fs::write(package.join("data/dialogue/intro_cutscene.yaml"), INTRO)
                .expect("invented intro");
            Self(root)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for InventedPackage {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("invented package should be removed");
        }
    }

    fn installed_dialogue_app(package: &InventedPackage) -> App {
        let mut app = headless_title_app_with_asset_base(
            AppState::NameEntry,
            package.path().to_string_lossy().into_owned(),
            ScenarioRoot::try_for_package_key("invented_transition")
                .expect("invented key should be valid"),
        );
        for _ in 0..1_000 {
            app.update();
            if *app.world().resource::<NameEntryViewState>() == NameEntryViewState::Ready
                && app.world().resource::<ActiveNewGameInputs>().status()
                    == ActiveNewGameInputsStatus::Ready
            {
                break;
            }
            std::thread::yield_now();
        }
        assert_eq!(
            app.world().resource::<ActiveNewGameInputs>().status(),
            ActiveNewGameInputsStatus::Ready
        );
        app.world_mut()
            .write_message(NameEntryConfirmed::for_test("Invented Traveler"));
        for _ in 0..1_000 {
            app.update();
            if app.world().resource::<State<AppState>>().get() == &AppState::Dialogue
                && *app.world().resource::<IntroDialogueViewState>()
                    == IntroDialogueViewState::Ready
            {
                // The minimal test app retains consumed messages, so remove the historical
                // NameEntry-to-Dialogue request before measuring this system's output.
                app.world_mut()
                    .resource_mut::<Messages<AppStateTransitionRequest>>()
                    .clear();
                assert_eq!(
                    app.world()
                        .resource::<Messages<AppStateTransitionRequest>>()
                        .len(),
                    0
                );
                return app;
            }
            std::thread::yield_now();
        }
        panic!("invented game did not enter Dialogue");
    }

    fn press_and_release(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear_just_pressed(key);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(key);
        app.update();
    }

    fn transition_actions() -> DialogueActions {
        scenario_yaml::from_str(
            "set_flag: injected_flag\ntransition:\n  map: injected_arrival\n  position: [3, 4]\n  fade: in\n",
        )
        .expect("invented transition actions")
    }

    #[test]
    fn invalid_runtime_map_id_preserves_location_and_allows_a_later_valid_arrival() {
        let package = InventedPackage::new();
        let mut app = installed_dialogue_app(&package);
        let original_map = app.world().resource::<GameState>().map().clone();
        let mut invalid = transition_actions();
        invalid.transition.as_mut().unwrap().map.clear();
        app.world_mut()
            .write_message(IntroDialogueCompleted::for_test(invalid));
        app.update();

        assert_eq!(app.world().resource::<GameState>().map(), &original_map);
        assert_eq!(
            app.world()
                .resource::<Messages<AppStateTransitionRequest>>()
                .len(),
            0
        );
        assert_eq!(
            app.world().resource::<State<AppState>>().get(),
            &AppState::Dialogue
        );

        app.world_mut()
            .write_message(IntroDialogueCompleted::for_test(transition_actions()));
        app.update();
        let game = app.world().resource::<GameState>();
        assert_eq!(game.map().current().unwrap().as_str(), "injected_arrival");
        assert_eq!(game.map().position(), Position::new(3, 4));
        assert_eq!(
            app.world()
                .resource::<Messages<AppStateTransitionRequest>>()
                .len(),
            1
        );
    }

    #[test]
    fn real_final_intro_completion_applies_flags_and_arrival_before_world_entry() {
        let package = InventedPackage::new();
        let mut app = installed_dialogue_app(&package);
        let (party, repository, opened, controlled, playtime, session_start, previous_map) = {
            let game = app.world().resource::<GameState>();
            (
                game.party().clone(),
                game.repository().clone(),
                game.opened_boxes().clone(),
                game.controlled_member_id().to_owned(),
                game.playtime().total_seconds(),
                game.playtime().session_start(),
                game.map().current().unwrap().clone(),
            )
        };
        let mut expected_rng = GameplayRng::from_seed(DEFAULT_GAMEPLAY_SEED);

        press_and_release(&mut app, KeyCode::Enter);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Space);
        app.update();

        // The completion readers and map mutation run in Update. The central request consumer
        // records NextState in PostUpdate, but World is not entered until the next app update.
        assert_eq!(
            app.world().resource::<State<AppState>>().get(),
            &AppState::Dialogue
        );
        assert_eq!(
            app.world()
                .resource::<Messages<AppStateTransitionRequest>>()
                .len(),
            1
        );
        let game = app.world().resource::<GameState>();
        assert!(game.flags().is_set("invented_transition_flag"));
        assert_eq!(game.map().current().unwrap().as_str(), "invented_arrival");
        assert_eq!(game.map().position(), Position::new(8, 9));
        assert_eq!(game.map().facing(), CardinalDirection::Down);
        assert!(game.map().has_visited(&previous_map));
        let arrival_id = RuntimeMapId::try_new("invented_arrival").unwrap();
        assert!(!game.map().has_visited(&arrival_id));
        assert_eq!(game.party(), &party);
        assert_eq!(game.repository(), &repository);
        assert_eq!(game.opened_boxes(), &opened);
        assert_eq!(game.controlled_member_id(), controlled);
        assert_eq!(game.playtime().total_seconds(), playtime);
        assert_eq!(game.playtime().session_start(), session_start);
        let _ = game;
        assert_eq!(
            app.world_mut()
                .resource_mut::<GameState>()
                .rng_mut()
                .next_u64(),
            expected_rng.next_u64(),
            "transition must not advance gameplay RNG"
        );

        app.update();
        assert_eq!(
            app.world().resource::<State<AppState>>().get(),
            &AppState::World
        );
    }

    #[test]
    fn missing_transition_is_a_no_op_and_duplicate_completion_applies_once() {
        let package = InventedPackage::new();
        let mut app = installed_dialogue_app(&package);
        let original_map = app.world().resource::<GameState>().map().clone();
        app.world_mut()
            .write_message(IntroDialogueCompleted::for_test(DialogueActions::default()));
        app.update();
        assert_eq!(app.world().resource::<GameState>().map(), &original_map);
        assert_eq!(
            app.world()
                .resource::<Messages<AppStateTransitionRequest>>()
                .len(),
            0
        );

        app.world_mut()
            .resource_mut::<GameState>()
            .map_mut()
            .set_facing(CardinalDirection::Left);
        let actions = transition_actions();
        app.world_mut()
            .write_message(IntroDialogueCompleted::for_test(actions.clone()));
        app.world_mut()
            .write_message(IntroDialogueCompleted::for_test(actions));
        app.update();

        let game = app.world().resource::<GameState>();
        assert!(game.flags().is_set("injected_flag"));
        assert_eq!(game.map().current().unwrap().as_str(), "injected_arrival");
        assert_eq!(game.map().position(), Position::new(3, 4));
        assert_eq!(
            game.map().facing(),
            CardinalDirection::Left,
            "transition without authored facing preserves the current facing"
        );
        assert_eq!(game.map().visited().count(), 1);
        let injected = RuntimeMapId::try_new("injected_arrival").unwrap();
        assert!(!game.map().has_visited(&injected));
        assert_eq!(
            app.world()
                .resource::<Messages<AppStateTransitionRequest>>()
                .len(),
            1,
            "duplicate completion must emit one World request"
        );
    }

    #[test]
    fn completion_without_game_state_is_drained_and_cannot_replay() {
        let package = InventedPackage::new();
        let mut app = installed_dialogue_app(&package);
        let game = app.world_mut().remove_resource::<GameState>().unwrap();
        let original_map = game.map().clone();
        app.world_mut()
            .write_message(IntroDialogueCompleted::for_test(transition_actions()));

        app.update();

        assert_eq!(
            app.world()
                .resource::<Messages<AppStateTransitionRequest>>()
                .len(),
            0
        );
        app.world_mut().insert_resource(game);
        app.update();
        assert_eq!(app.world().resource::<GameState>().map(), &original_map);
        assert!(
            !app.world()
                .resource::<GameState>()
                .flags()
                .is_set("injected_flag")
        );
        assert_eq!(
            app.world().resource::<State<AppState>>().get(),
            &AppState::Dialogue
        );
    }
}
