//! Isolated debug-session construction and pre-window map validation.

use std::{collections::BTreeMap, fs, path::Path};

use bevy::{ecs::system::SystemParam, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    app_state::{AppState, AppStateTransitionRequest},
    game_state::GameState,
    gameplay_rng::GameplayRng,
    new_game::{NewGameScenario, build_new_game_state_with_seed},
    playtime::Playtime,
    runtime_map::{RuntimeMapId, RuntimeMapState},
    runtime_member::RuntimeMember,
    scenario_balance::BalanceData,
    scenario_dialogue::CutsceneDialogue,
    scenario_manifest::{Manifest, ManifestRefs, ManifestStart},
    scenario_manifest_asset::ActiveManifestLoad,
    scenario_new_game_assets::{ActiveNewGameInputs, ActiveNewGameInputsStatus},
    scenario_party::PartyCatalog,
    scenario_path::ScenarioRelativePath,
    scenario_spatial::{CardinalDirection, Position, collision_occupancy::CollisionOccupancy},
    scenario_yaml,
    tmx_header::parse_tmx_map_document,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DebugPartyPreset {
    Solo,
    Full,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize, Resource)]
#[serde(deny_unknown_fields)]
pub(crate) struct DebugLaunchConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) start_map: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) start_position: Option<Position>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) party_preset: Option<DebugPartyPreset>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) flag_overrides: BTreeMap<String, bool>,
}

impl DebugLaunchConfig {
    pub(crate) fn is_active(&self) -> bool {
        self.start_map.is_some()
            || self.start_position.is_some()
            || self.party_preset.is_some()
            || !self.flag_overrides.is_empty()
    }
}

/// Marks a session whose automatic persistence is suppressed.
///
/// The manual save screen intentionally ignores this marker: choosing Save is the explicit opt-in
/// that permits debug overrides to reach disk.
#[derive(Resource)]
pub(crate) struct DebugSession;

#[derive(Deserialize)]
struct DebugManifest {
    start: ManifestStart,
    refs: ManifestRefs,
}

/// Validates a requested debug location against the exact production TMX collision projection.
pub(crate) fn validate_debug_launch(
    package: &Path,
    config: &DebugLaunchConfig,
) -> Result<(), String> {
    let (Some(map_id), Some(position)) = (&config.start_map, config.start_position) else {
        if config.start_map.is_some() || config.start_position.is_some() {
            return Err("--start-map and --start-position must be supplied together".to_owned());
        }
        return Ok(());
    };
    let manifest_text = fs::read_to_string(package.join("manifest.yaml"))
        .map_err(|error| format!("could not read selected scenario manifest: {error}"))?;
    let manifest: DebugManifest = scenario_yaml::from_str(&manifest_text)
        .map_err(|error| format!("selected scenario manifest is invalid: {error}"))?;
    let _ = manifest.start;
    let logical =
        ScenarioRelativePath::try_from(format!("{}/{map_id}.tmx", manifest.refs.tmx.as_str()))
            .map_err(|error| format!("debug map id `{map_id}` is invalid: {error}"))?;
    let tmx_path = package.join(logical.as_str());
    let tmx_text = fs::read_to_string(&tmx_path)
        .map_err(|_| format!("debug map `{map_id}` has no migrated TMX"))?;
    let document = parse_tmx_map_document(&tmx_text, &logical)
        .map_err(|error| format!("debug map `{map_id}` is invalid: {error}"))?;
    let collision = CollisionOccupancy::from_tmx_document(&document)
        .map_err(|error| format!("debug map `{map_id}` collision is invalid: {error}"))?;
    match collision.is_open(position.x, position.y) {
        Some(true) => Ok(()),
        Some(false) => Err(format!(
            "debug start position [{}, {}] is blocked on map `{map_id}`",
            position.x, position.y
        )),
        None => Err(format!(
            "debug start position [{}, {}] is outside map `{map_id}`",
            position.x, position.y
        )),
    }
}

pub(crate) struct DebugLaunchPlugin;

impl Plugin for DebugLaunchPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugLaunchState>().add_systems(
            Update,
            install_debug_session.run_if(in_state(AppState::Boot)),
        );
    }
}

#[derive(Debug, Default, Resource)]
struct DebugLaunchState {
    finished: bool,
    failure: Option<String>,
}

#[derive(SystemParam)]
struct DebugAssets<'w> {
    active: Res<'w, ActiveNewGameInputs>,
    manifest_load: Res<'w, ActiveManifestLoad>,
    manifests: Res<'w, Assets<Manifest>>,
    parties: Res<'w, Assets<PartyCatalog>>,
    balances: Res<'w, Assets<BalanceData>>,
    intros: Res<'w, Assets<CutsceneDialogue>>,
}

#[expect(
    clippy::too_many_arguments,
    reason = "debug installation validates and publishes one transactional scenario boundary"
)]
fn install_debug_session(
    mut commands: Commands,
    config: Res<DebugLaunchConfig>,
    assets: DebugAssets,
    startup_rng: Res<GameplayRng>,
    real_time: Res<Time<Real>>,
    mut state: ResMut<DebugLaunchState>,
    mut transitions: MessageWriter<AppStateTransitionRequest>,
    mut exit: MessageWriter<AppExit>,
) {
    if state.finished {
        return;
    }
    let Some(inputs) = assets.active.inputs(
        &assets.manifest_load,
        &assets.manifests,
        &assets.parties,
        &assets.balances,
        &assets.intros,
    ) else {
        if assets.active.status() == ActiveNewGameInputsStatus::Failed {
            fail_debug_launch(
                &mut state,
                assets
                    .active
                    .failure()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "debug new-game inputs failed".to_owned()),
                &mut exit,
            );
        }
        return;
    };
    let result = build_new_game_state_with_seed(
        NewGameScenario {
            manifest: inputs.manifest,
            party: inputs.party,
            balance: inputs.balance,
        },
        real_time.elapsed(),
        startup_rng.state(),
    )
    .map_err(|error| error.to_string())
    .and_then(|mut game| {
        apply_debug_overrides(&mut game, inputs.party, inputs.balance, &config)?;
        Ok(game)
    });
    match result {
        Ok(game) => {
            commands.queue(move |world: &mut World| {
                world.insert_resource(game);
                world.insert_resource(DebugSession);
                world.remove_resource::<GameplayRng>();
                world.remove_resource::<Playtime>();
            });
            transitions.write(AppStateTransitionRequest::new(AppState::World));
            state.finished = true;
        }
        Err(error) => fail_debug_launch(&mut state, error, &mut exit),
    }
}

fn fail_debug_launch(
    state: &mut DebugLaunchState,
    error: String,
    exit: &mut MessageWriter<AppExit>,
) {
    eprintln!("Debug launch failed: {error}");
    state.failure = Some(error);
    state.finished = true;
    exit.write(AppExit::error());
}

fn apply_debug_overrides(
    game: &mut GameState,
    party: &PartyCatalog,
    balance: &BalanceData,
    config: &DebugLaunchConfig,
) -> Result<(), String> {
    if let (Some(map_id), Some(position)) = (&config.start_map, config.start_position) {
        game.map_mut().clone_from(&RuntimeMapState::new(
            RuntimeMapId::try_new(map_id.clone()).map_err(|error| error.to_string())?,
            position,
            CardinalDirection::Down,
        ));
    }
    if config.party_preset == Some(DebugPartyPreset::Full) {
        for authored in &party.party {
            if game.party().contains(&authored.data().id) {
                continue;
            }
            let member = RuntimeMember::try_from_catalog(authored, &balance.progression)
                .map_err(|error| error.to_string())?;
            game.party_mut()
                .try_add(member)
                .map_err(|error| error.to_string())?;
        }
    }
    for (flag, value) in &config.flag_overrides {
        if *value {
            game.flags_mut().set(flag.clone());
        } else {
            game.flags_mut().unset(flag);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::{new_game::build_new_game_state, scenario_party::PartyMember};

    fn manifest() -> Manifest {
        scenario_yaml::from_str(include_str!(
            "../../../tests/fixtures/rusted-kingdoms-manifest-complete.yaml"
        ))
        .unwrap()
    }

    fn party() -> PartyCatalog {
        let mut party: PartyCatalog = scenario_yaml::from_str(include_str!(
            "../../../tests/fixtures/party-catalog-shapes.yaml"
        ))
        .unwrap();
        let PartyMember::Protagonist(protagonist) = &mut party.party[0] else {
            unreachable!()
        };
        protagonist.id = "aric".to_owned();
        protagonist.class_id = "hero".to_owned();
        party
    }

    fn balance() -> BalanceData {
        scenario_yaml::from_str(include_str!(
            "../../../tests/fixtures/balance-complete.yaml"
        ))
        .unwrap()
    }

    #[test]
    fn debug_overrides_are_applied_after_normal_new_game_construction() {
        let manifest = manifest();
        let party = party();
        let balance = balance();
        let mut game = build_new_game_state(
            NewGameScenario {
                manifest: &manifest,
                party: &party,
                balance: &balance,
            },
            Duration::ZERO,
        )
        .unwrap();
        let normal = game.clone();
        let config = DebugLaunchConfig {
            start_map: Some("invented_map".to_owned()),
            start_position: Some(Position::new(3, 4)),
            party_preset: Some(DebugPartyPreset::Full),
            flag_overrides: BTreeMap::from([
                ("debug_enabled".to_owned(), true),
                ("story_quest_started".to_owned(), false),
            ]),
        };

        apply_debug_overrides(&mut game, &party, &balance, &config).unwrap();

        assert_eq!(normal.party().len(), 1);
        assert_eq!(normal.map().current().unwrap().as_str(), "town_01_ardel");
        assert!(normal.flags().is_set("story_quest_started"));
        assert_eq!(game.party().len(), party.party.len());
        assert_eq!(game.map().current().unwrap().as_str(), "invented_map");
        assert_eq!(game.map().position(), Position::new(3, 4));
        assert!(game.flags().is_set("debug_enabled"));
        assert!(!game.flags().is_set("story_quest_started"));
    }

    #[test]
    fn production_map_and_position_are_validated_before_launch() {
        let package =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/scenarios/rusted_kingdoms");
        let config = |map: &str, position| DebugLaunchConfig {
            start_map: Some(map.to_owned()),
            start_position: Some(position),
            ..Default::default()
        };

        assert_eq!(
            validate_debug_launch(&package, &config("town_01_ardel", Position::new(10, 0))),
            Ok(())
        );
        assert!(
            validate_debug_launch(&package, &config("town_01_ardel", Position::new(0, 0)))
                .unwrap_err()
                .contains("blocked")
        );
        assert!(
            validate_debug_launch(&package, &config("town_01_ardel", Position::new(300, 200)))
                .unwrap_err()
                .contains("outside")
        );
        assert!(
            validate_debug_launch(&package, &config("not_a_migrated_map", Position::new(1, 1)))
                .unwrap_err()
                .contains("has no migrated TMX")
        );
    }

    #[test]
    fn production_full_party_preset_installs_all_five_members() {
        let package =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/scenarios/rusted_kingdoms");
        let manifest: Manifest =
            scenario_yaml::from_str(&fs::read_to_string(package.join("manifest.yaml")).unwrap())
                .unwrap();
        let party: PartyCatalog =
            scenario_yaml::from_str(&fs::read_to_string(package.join("data/party.yaml")).unwrap())
                .unwrap();
        let balance: BalanceData = scenario_yaml::from_str(
            &fs::read_to_string(package.join("data/balance.yaml")).unwrap(),
        )
        .unwrap();
        let mut game = build_new_game_state(
            NewGameScenario {
                manifest: &manifest,
                party: &party,
                balance: &balance,
            },
            Duration::ZERO,
        )
        .unwrap();

        apply_debug_overrides(
            &mut game,
            &party,
            &balance,
            &DebugLaunchConfig {
                party_preset: Some(DebugPartyPreset::Full),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(game.party().len(), 5);
        assert_eq!(
            game.party()
                .members()
                .map(|member| member.id())
                .collect::<Vec<_>>(),
            ["aric", "elise", "reiya", "jep", "kael"]
        );
    }
}
