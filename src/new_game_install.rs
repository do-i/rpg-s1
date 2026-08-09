//! Atomic handoff from Name Entry confirmation into one complete runtime session.

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app_state::{AppState, AppStateTransitionRequest},
    gameplay_rng::GameplayRng,
    name_entry::NameEntryConfirmed,
    new_game::{NewGameScenario, build_new_game_state},
    playtime::Playtime,
    scenario_balance::BalanceData,
    scenario_dialogue::CutsceneDialogue,
    scenario_manifest::Manifest,
    scenario_manifest_asset::ActiveManifestLoad,
    scenario_new_game_assets::{ActiveNewGameInputs, ActiveNewGameInputsStatus},
    scenario_party::PartyCatalog,
};

pub struct NewGameInstallPlugin;
impl Plugin for NewGameInstallPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NewGameInstallState>()
            .add_systems(OnEnter(AppState::NameEntry), reset_install_state)
            .add_systems(
                Update,
                receive_confirmation.run_if(in_state(AppState::NameEntry)),
            )
            .add_systems(
                Update,
                install_confirmed_game
                    .after(receive_confirmation)
                    .run_if(in_state(AppState::NameEntry)),
            )
            .add_systems(OnExit(AppState::NameEntry), reset_install_state);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NewGameInstallStatus {
    Idle,
    WaitingForInputs,
    Failed,
    Installed,
}
#[derive(Resource, Debug)]
pub struct NewGameInstallState {
    pub(crate) pending_name: Option<String>,
    pub(crate) status: NewGameInstallStatus,
    pub(crate) failure: Option<String>,
}
impl Default for NewGameInstallState {
    fn default() -> Self {
        Self {
            pending_name: None,
            status: NewGameInstallStatus::Idle,
            failure: None,
        }
    }
}
fn reset_install_state(mut state: ResMut<NewGameInstallState>) {
    *state = NewGameInstallState::default();
}
fn receive_confirmation(
    mut confirmations: MessageReader<NameEntryConfirmed>,
    mut state: ResMut<NewGameInstallState>,
) {
    if state.status == NewGameInstallStatus::Installed {
        confirmations.clear();
        return;
    }
    for confirmation in confirmations.read() {
        if state.pending_name.is_none() && state.status != NewGameInstallStatus::Failed {
            state.pending_name = Some(confirmation.name().to_owned());
            state.status = NewGameInstallStatus::WaitingForInputs;
            state.failure = None;
        }
    }
}
#[derive(SystemParam)]
struct NewGameAssets<'w> {
    active: Res<'w, ActiveNewGameInputs>,
    manifest_load: Res<'w, ActiveManifestLoad>,
    manifests: Res<'w, Assets<Manifest>>,
    parties: Res<'w, Assets<PartyCatalog>>,
    balances: Res<'w, Assets<BalanceData>>,
    intros: Res<'w, Assets<CutsceneDialogue>>,
}
fn install_confirmed_game(
    mut commands: Commands,
    mut state: ResMut<NewGameInstallState>,
    assets: NewGameAssets,
    real_time: Res<Time<Real>>,
    mut transitions: MessageWriter<AppStateTransitionRequest>,
) {
    let Some(name) = state.pending_name.clone() else {
        return;
    };
    let Some(inputs) = assets.active.inputs(
        &assets.manifest_load,
        &assets.manifests,
        &assets.parties,
        &assets.balances,
        &assets.intros,
    ) else {
        if assets.active.status() == ActiveNewGameInputsStatus::Failed {
            state.status = NewGameInstallStatus::Failed;
            state.failure = assets
                .active
                .failure()
                .map(ToString::to_string)
                .or_else(|| Some("new-game inputs failed".to_owned()));
        }
        return;
    };
    let mut game = match build_new_game_state(
        NewGameScenario {
            manifest: inputs.manifest,
            party: inputs.party,
            balance: inputs.balance,
        },
        real_time.elapsed(),
    ) {
        Ok(game) => game,
        Err(error) => {
            state.status = NewGameInstallStatus::Failed;
            state.failure = Some(error.to_string());
            return;
        }
    };
    if let Err(error) = game.party_mut().rename_protagonist(name) {
        state.status = NewGameInstallStatus::Failed;
        state.failure = Some(error.to_string());
        return;
    }
    commands.queue(move |world: &mut World| {
        world.insert_resource(game);
        world.remove_resource::<GameplayRng>();
        world.remove_resource::<Playtime>();
    });
    transitions.write(AppStateTransitionRequest::new(AppState::Dialogue));
    state.pending_name = None;
    state.failure = None;
    state.status = NewGameInstallStatus::Installed;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app_state::AppState, game_state::GameState, gameplay_rng::DEFAULT_GAMEPLAY_SEED,
        scenario_root::ScenarioRoot, test_support::headless_title_app_with_asset_base,
    };
    use bevy::{
        input::{
            ButtonState,
            keyboard::{Key, KeyboardInput},
        },
        prelude::{Entity, KeyCode},
    };
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Package(PathBuf);
    impl Package {
        fn new(valid: bool) -> Self {
            Self::with_intro(valid, true)
        }
        fn with_intro(valid: bool, intro: bool) -> Self {
            let root = std::env::temp_dir().join(format!(
                "rpg-s1-install-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let package = root.join("scenarios/invented");
            fs::create_dir_all(package.join("data")).unwrap();
            fs::create_dir_all(package.join("data/dialogue")).unwrap();
            fs::write(
                package.join("manifest.yaml"),
                include_str!("../tests/fixtures/rusted-kingdoms-manifest-complete.yaml"),
            )
            .unwrap();
            if intro {
                fs::write(
                    package.join("data/dialogue/intro_cutscene.yaml"),
                    include_str!("../tests/fixtures/dialogue-intro-cutscene.yaml"),
                )
                .unwrap();
            }
            let party = include_str!("../tests/fixtures/party-catalog-shapes.yaml")
                .replacen("id: ember", "id: aric", 1)
                .replacen("class: vanguard", "class: hero", 1);
            fs::write(
                package.join("data/party.yaml"),
                if valid {
                    party
                } else {
                    include_str!("../tests/fixtures/party-catalog-shapes.yaml").to_owned()
                },
            )
            .unwrap();
            fs::write(
                package.join("data/balance.yaml"),
                include_str!("../tests/fixtures/balance-complete.yaml"),
            )
            .unwrap();
            Self(root)
        }
    }
    impl Drop for Package {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
    fn app(package: &Package) -> App {
        headless_title_app_with_asset_base(
            AppState::NameEntry,
            package.0.to_string_lossy().into_owned(),
            ScenarioRoot::try_for_package_key("invented").unwrap(),
        )
    }
    fn until(app: &mut App, ready: bool) {
        for _ in 0..1000 {
            app.update();
            if app.world().get_resource::<GameState>().is_some() == ready {
                return;
            }
            std::thread::yield_now()
        }
        panic!("install did not reach expected state")
    }
    fn confirm(app: &mut App, name: &str) {
        app.world_mut()
            .resource_mut::<Messages<NameEntryConfirmed>>()
            .write(NameEntryConfirmed::for_test(name));
    }
    fn press(app: &mut App, key: KeyCode, text: Option<&str>) {
        let logical_key = match key {
            KeyCode::Backspace => Key::Backspace,
            KeyCode::Enter => Key::Enter,
            _ => Key::Character(text.unwrap_or_default().into()),
        };
        app.world_mut().write_message(KeyboardInput {
            key_code: key,
            logical_key,
            state: ButtonState::Pressed,
            text: text.map(Into::into),
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    }

    #[test]
    fn confirmation_installs_one_owned_session_and_leaves_catalog_assets_unchanged() {
        let package = Package::new(true);
        let mut app = app(&package);
        assert!(app.world().get_resource::<GameState>().is_none());
        assert!(app.world().get_resource::<GameplayRng>().is_some());
        assert!(app.world().get_resource::<Playtime>().is_some());
        for _ in 0..1000 {
            app.update();
            if *app
                .world()
                .resource::<crate::name_entry::NameEntryViewState>()
                == crate::name_entry::NameEntryViewState::Ready
            {
                break;
            }
        }
        for _ in 0..4 {
            press(&mut app, KeyCode::Backspace, None);
        }
        press(&mut app, KeyCode::KeyN, Some("  Nova Ω  "));
        press(&mut app, KeyCode::Enter, None);
        until(&mut app, true);
        app.update();
        let world = app.world();
        let game = world.resource::<GameState>();
        assert_eq!(game.party().protagonist().unwrap().name(), "Nova Ω");
        assert_eq!(game.playtime().total_seconds(), 0);
        let start = game.playtime().session_start().unwrap();
        assert!(world.get_resource::<GameplayRng>().is_none());
        assert!(world.get_resource::<Playtime>().is_none());
        assert_eq!(
            world.resource::<State<AppState>>().get(),
            &AppState::Dialogue
        );
        assert_eq!(
            world
                .resource::<Assets<Manifest>>()
                .iter()
                .next()
                .unwrap()
                .1
                .protagonist
                .name,
            "Aric"
        );
        assert_eq!(
            world
                .resource::<Assets<PartyCatalog>>()
                .iter()
                .next()
                .unwrap()
                .1
                .party[0]
                .data()
                .name,
            "Ember"
        );
        let _ = world;
        app.world_mut()
            .resource_mut::<GameState>()
            .playtime_mut()
            .commit_session(start + std::time::Duration::from_secs(7));
        assert_eq!(
            app.world()
                .resource::<GameState>()
                .playtime()
                .total_seconds(),
            7
        );
        let mut expected = GameplayRng::from_seed(DEFAULT_GAMEPLAY_SEED);
        assert_eq!(
            app.world_mut()
                .resource_mut::<GameState>()
                .rng_mut()
                .next_u64(),
            expected.next_u64()
        );
        app.update();
        assert_eq!(
            app.world().resource::<NewGameInstallState>().status,
            NewGameInstallStatus::Idle,
            "leaving NameEntry resets the screen-local installer after one transition"
        );
    }
    #[test]
    fn early_confirmation_waits_for_assets_then_installs_once() {
        let package = Package::new(true);
        let mut app = app(&package);
        confirm(&mut app, "Early");
        confirm(&mut app, "Ignored");
        app.update();
        assert!(app.world().get_resource::<GameState>().is_none());
        assert_eq!(
            app.world().resource::<NewGameInstallState>().status,
            NewGameInstallStatus::WaitingForInputs
        );
        until(&mut app, true);
        assert_eq!(
            app.world()
                .resource::<GameState>()
                .party()
                .protagonist()
                .unwrap()
                .name(),
            "Early"
        );
        assert_eq!(
            app.world()
                .resource::<Messages<AppStateTransitionRequest>>()
                .len(),
            1,
            "two confirmations must request only one transition"
        );
        let mut expected = GameplayRng::from_seed(DEFAULT_GAMEPLAY_SEED);
        assert_eq!(
            app.world_mut()
                .resource_mut::<GameState>()
                .rng_mut()
                .next_u64(),
            expected.next_u64()
        );

        app.update();
        confirm(&mut app, "Too Late");
        app.update();

        assert_eq!(
            app.world().resource::<State<AppState>>().get(),
            &AppState::Dialogue
        );
        assert_eq!(
            app.world()
                .resource::<GameState>()
                .party()
                .protagonist()
                .unwrap()
                .name(),
            "Early"
        );
        assert_eq!(
            app.world_mut()
                .resource_mut::<GameState>()
                .rng_mut()
                .next_u64(),
            expected.next_u64(),
            "a later confirmation must not replace the installed session"
        );
    }
    #[test]
    fn invalid_construct_keeps_transitional_resources_and_reports_failure() {
        let package = Package::new(false);
        let mut app = app(&package);
        confirm(&mut app, "Nope");
        for _ in 0..1000 {
            app.update();
            if app.world().resource::<NewGameInstallState>().status == NewGameInstallStatus::Failed
            {
                break;
            }
            std::thread::yield_now();
        }
        assert!(app.world().get_resource::<GameState>().is_none());
        assert!(app.world().get_resource::<GameplayRng>().is_some());
        assert!(app.world().get_resource::<Playtime>().is_some());
        assert_eq!(
            app.world().resource::<State<AppState>>().get(),
            &AppState::NameEntry
        );
        assert!(
            app.world()
                .resource::<NewGameInstallState>()
                .failure
                .as_deref()
                .unwrap()
                .contains("protagonist")
        );
        let failure = app
            .world()
            .resource::<NewGameInstallState>()
            .failure
            .clone();
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(
            app.world().resource::<NewGameInstallState>().failure,
            failure
        );
        assert!(app.world().get_resource::<GameState>().is_none());
        assert!(app.world().get_resource::<GameplayRng>().is_some());
        assert!(app.world().get_resource::<Playtime>().is_some());
        assert_eq!(
            app.world().resource::<State<AppState>>().get(),
            &AppState::NameEntry
        );
        assert_eq!(
            app.world()
                .resource::<Messages<AppStateTransitionRequest>>()
                .len(),
            0
        );
    }
    #[test]
    fn intro_failure_keeps_name_entry_and_transitional_ownership() {
        let package = Package::with_intro(true, false);
        let mut app = app(&package);
        confirm(&mut app, "Queued");
        for _ in 0..1000 {
            app.update();
            if app.world().resource::<NewGameInstallState>().status == NewGameInstallStatus::Failed
            {
                break;
            }
            std::thread::yield_now();
        }
        assert!(app.world().get_resource::<GameState>().is_none());
        assert!(app.world().get_resource::<GameplayRng>().is_some());
        assert!(app.world().get_resource::<Playtime>().is_some());
        assert_eq!(
            app.world().resource::<State<AppState>>().get(),
            &AppState::NameEntry
        );
        assert!(
            app.world()
                .resource::<NewGameInstallState>()
                .failure
                .as_deref()
                .unwrap()
                .contains("data/dialogue/intro_cutscene.yaml")
        );
    }
}
