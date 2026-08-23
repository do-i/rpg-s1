//! Automatic checkpoint saves to the reserved autosave slot (index 0) when the player reaches a
//! new map.
//!
//! [`SaveSlot::label`](crate::save_store::SaveSlot::label) and the field save page already present
//! slot 0 as "Autosave", and the title load picker already ranks it alongside the player slots;
//! this plugin is what actually keeps that slot populated.
//!
//! The checkpoint is keyed off the session's committed map id rather than the transition fade,
//! because [`WorldTransition`] is reset on *every* `AppState::World` entry — returning from a
//! random encounter or a cutscene replays the same fade as a real map change. Comparing the map
//! id instead keeps one checkpoint per arrival and leaves battle and dialogue round trips alone.

use bevy::prelude::*;

use crate::{
    app_state::AppState,
    game_state::GameState,
    runtime_map::RuntimeMapId,
    save_data::NativeSaveEnvelope,
    save_store::{SaveStore, unix_timestamp_now},
    save_ui::SaveSlotCatalog,
    world_transition::WorldTransition,
};

const AUTOSAVE_SLOT: usize = 0;

pub(crate) struct AutosavePlugin;

impl Plugin for AutosavePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AutosaveTracker>()
            // A title visit is a session boundary: the next World entry belongs to a different
            // playthrough, so it must checkpoint even when it starts on the same map.
            .add_systems(OnEnter(AppState::Title), reset_autosave_tracker)
            .add_systems(
                Update,
                autosave_on_map_arrival.run_if(in_state(AppState::World)),
            );
    }
}

/// Remembers the map the last checkpoint was taken on, so one arrival writes one autosave.
#[derive(Debug, Default, Resource)]
struct AutosaveTracker {
    last_map: Option<RuntimeMapId>,
    failure: Option<String>,
}

impl AutosaveTracker {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "autosave diagnostics are exposed before the World error UI"
        )
    )]
    fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }
}

fn reset_autosave_tracker(mut tracker: ResMut<AutosaveTracker>) {
    *tracker = AutosaveTracker::default();
}

fn autosave_on_map_arrival(
    transition: Res<WorldTransition>,
    mut tracker: ResMut<AutosaveTracker>,
    store: Res<SaveStore>,
    mut saves: ResMut<SaveSlotCatalog>,
    game: Option<ResMut<GameState>>,
    time: Res<Time<Real>>,
) {
    // Wait for the arrival fade to settle. A transition commits the destination before the map is
    // published, so checkpointing mid-fade would capture a session the player cannot act on yet.
    if transition.input_locked() {
        return;
    }
    let Some(mut game) = game else { return };
    let Some(current) = game.map().current().cloned() else {
        return;
    };
    if tracker.last_map.as_ref() == Some(&current) {
        return;
    }
    // Scenario data is still streaming in; retry on a later frame rather than recording the
    // arrival, so the first checkpoint is only skipped and never lost.
    let Some(context) = saves.context() else {
        return;
    };
    let scenario_id = context.scenario_id.to_owned();
    let scenario_version = context.scenario_version.to_owned();
    let balance = context.balance.clone();

    // Past this point the arrival counts as attempted. Recording it even when the write fails
    // keeps a persistent failure (a full disk, a read-only directory) from retrying every frame.
    tracker.last_map = Some(current.clone());
    game.playtime_mut().commit_session(time.elapsed());
    let result = unix_timestamp_now()
        .map_err(|error| error.to_string())
        .and_then(|timestamp| {
            NativeSaveEnvelope::from_game_state(
                &game,
                scenario_id,
                scenario_version,
                timestamp,
                current.as_str().to_owned(),
            )
            .map_err(|error| error.to_string())
        })
        .and_then(|envelope| {
            store
                .write(AUTOSAVE_SLOT, &envelope, true, &balance)
                .map_err(|error| error.to_string())
        });
    match result {
        Ok(_) => {
            tracker.failure = None;
            saves.request_refresh();
        }
        Err(error) => tracker.failure = Some(error),
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf, time::SystemTime};

    use super::*;
    use crate::{
        save_data::tests::{fixture_balance, fixture_game},
        scenario_spatial::{CardinalDirection, Position},
    };

    struct TempSaveDir(PathBuf);

    impl TempSaveDir {
        fn new(label: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "rpg-s1-autosave-test-{label}-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn autosave_path(&self) -> PathBuf {
            self.0.join("000.yaml")
        }
    }

    impl Drop for TempSaveDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn build_app(root: PathBuf, transition: WorldTransition) -> App {
        let mut app = App::new();
        app.insert_resource(SaveStore::new(root))
            .insert_resource(SaveSlotCatalog::ready_for_test(
                "my_rpg_story",
                "1.0.0",
                fixture_balance(),
            ))
            .insert_resource(transition)
            .insert_resource(fixture_game())
            .insert_resource(Time::<Real>::default())
            .init_resource::<AutosaveTracker>()
            .add_systems(Update, autosave_on_map_arrival);
        app
    }

    fn move_to_map(app: &mut App, map_id: &str) {
        app.world_mut()
            .resource_mut::<GameState>()
            .map_mut()
            .move_to(
                RuntimeMapId::try_new(map_id).unwrap(),
                Position::new(4, 4),
                CardinalDirection::Down,
            );
    }

    #[test]
    fn a_transition_still_fading_never_writes_the_autosave_slot() {
        let dir = TempSaveDir::new("locked");
        // `WorldTransition::default()` is the mid-fade-in state every real World entry starts in.
        let mut app = build_app(dir.0.clone(), WorldTransition::default());

        app.update();

        assert!(!dir.autosave_path().exists());
        assert!(app.world().resource::<AutosaveTracker>().last_map.is_none());
    }

    #[test]
    fn a_settled_arrival_checkpoints_slot_zero_once_and_not_again_on_the_same_map() {
        let dir = TempSaveDir::new("arrival");
        let mut app = build_app(dir.0.clone(), WorldTransition::idle_for_test());

        app.update();

        assert!(dir.autosave_path().exists());
        assert!(
            app.world()
                .resource::<AutosaveTracker>()
                .failure()
                .is_none()
        );

        // Deleting the checkpoint makes a redundant rewrite unambiguous: staying on the same map
        // must leave the slot alone, so the file must not reappear.
        fs::remove_file(dir.autosave_path()).unwrap();
        app.update();
        app.update();
        assert!(!dir.autosave_path().exists());
    }

    #[test]
    fn reaching_a_different_map_writes_a_fresh_checkpoint_naming_that_map() {
        let dir = TempSaveDir::new("map-change");
        let mut app = build_app(dir.0.clone(), WorldTransition::idle_for_test());

        app.update();
        let first = fs::read_to_string(dir.autosave_path()).unwrap();
        assert!(first.contains("zone_01_starting_forest"));

        move_to_map(&mut app, "town_01_ardel");
        app.update();

        let second = fs::read_to_string(dir.autosave_path()).unwrap();
        assert!(second.contains("town_01_ardel"));
        assert_ne!(first, second);
    }

    #[test]
    fn a_battle_or_cutscene_round_trip_on_one_map_does_not_recheckpoint() {
        let dir = TempSaveDir::new("round-trip");
        let mut app = build_app(dir.0.clone(), WorldTransition::idle_for_test());

        app.update();
        fs::remove_file(dir.autosave_path()).unwrap();

        // Leaving and re-entering World resets the transition to its locked fade-in, which is the
        // exact shape a real map arrival has. Only the unchanged map id distinguishes them.
        *app.world_mut().resource_mut::<WorldTransition>() = WorldTransition::default();
        app.update();
        *app.world_mut().resource_mut::<WorldTransition>() = WorldTransition::idle_for_test();
        app.update();

        assert!(!dir.autosave_path().exists());
    }

    #[test]
    fn a_title_visit_lets_the_next_session_checkpoint_the_same_map_again() {
        let dir = TempSaveDir::new("title-reset");
        let mut app = build_app(dir.0.clone(), WorldTransition::idle_for_test());
        app.add_systems(Update, reset_autosave_tracker);

        app.update();
        assert!(dir.autosave_path().exists());
        fs::remove_file(dir.autosave_path()).unwrap();

        // `reset_autosave_tracker` stands in for the real `OnEnter(AppState::Title)` schedule.
        app.update();

        assert!(dir.autosave_path().exists());
    }

    #[test]
    fn an_unwritable_directory_records_one_failure_without_retrying_the_same_arrival() {
        let dir = TempSaveDir::new("unwritable");
        let root = dir.0.join("nested");
        fs::create_dir_all(&root).unwrap();
        let mut permissions = fs::metadata(&root).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&root, permissions).unwrap();

        let mut app = build_app(root.clone(), WorldTransition::idle_for_test());
        app.update();

        // Assert on the write itself, so the test cannot pass because the checkpoint was skipped
        // for some earlier reason. Requires a non-root user; root ignores the readonly bit.
        let failure = app
            .world()
            .resource::<AutosaveTracker>()
            .failure()
            .expect("a readonly save directory must record a write failure")
            .to_owned();
        assert!(failure.contains("create temporary save"), "{failure}");
        assert!(!root.join("000.yaml").exists());
        // The arrival is recorded despite the failure, so the next frame does not retry it.
        assert!(app.world().resource::<AutosaveTracker>().last_map.is_some());

        let mut permissions = fs::metadata(&root).unwrap().permissions();
        #[expect(clippy::permissions_set_readonly_false, reason = "test cleanup")]
        permissions.set_readonly(false);
        fs::set_permissions(&root, permissions).unwrap();
    }
}
