mod action_input;
pub mod app_state;
mod autosave;
mod battle;
mod cli;
pub mod encounter;
mod encounter_assets;
mod field_menu;
mod field_menu_domain;
mod game_over;
pub mod game_state;
pub mod gameplay_canvas;
pub mod gameplay_rng;
mod intro_completion;
mod intro_dialogue;
mod intro_transition;
mod menu_chrome;
mod name_entry;
pub mod new_game;
mod new_game_install;
pub mod playtime;
mod python_save_import;
pub mod runtime_flags;
pub mod runtime_map;
pub mod runtime_member;
pub mod runtime_opened_boxes;
pub mod runtime_party;
pub mod runtime_quest;
pub mod runtime_repository;
mod save_data;
mod save_store;
mod save_ui;
mod scenario_dialogue_report;
pub mod scenario_manifest_asset;
mod scenario_map_report;
mod scenario_map_sweep;
pub mod scenario_new_game_assets;
pub mod scenario_spatial;
pub mod service_domain;
mod service_ui;
mod tile_coordinates;
mod title_screen;
mod tmx_ground_asset;
mod tsx_atlas_asset;
mod ui_theme;
mod world_actor;
mod world_audio;
mod world_debug_overlay;
mod world_dialogue;
mod world_encounter;
mod world_interaction;
mod world_object;
mod world_player;
mod world_transition;

pub use rpg_content::{
    manifest_path_validation, scenario_audio, scenario_balance, scenario_battle_background,
    scenario_class, scenario_condition, scenario_cross_reference, scenario_dialogue,
    scenario_duplicate_id, scenario_encounter, scenario_enemy, scenario_item, scenario_manifest,
    scenario_map, scenario_party, scenario_path, scenario_quest, scenario_recipe, scenario_root,
    scenario_yaml, tmx_header, tsx_metadata,
};

#[cfg(test)]
mod test_support;

use action_input::ActionInputPlugin;
use app_state::{AppState, AppStateTransitionPlugin};
use autosave::AutosavePlugin;
use battle::BattlePlugin;
use bevy::{
    asset::{AssetApp, AssetPlugin},
    audio::{AudioPlugin, GlobalVolume, Volume},
    prelude::*,
    window::{PresentMode, PrimaryWindow, WindowPlugin},
};
use encounter_assets::EncounterAssetPlugin;
use field_menu::FieldMenuPlugin;
use field_menu_domain::FieldMenuDomainPlugin;
use game_over::GameOverPlugin;
use gameplay_canvas::{FixedGameplayCanvasPlugin, LOGICAL_CANVAS_HEIGHT, LOGICAL_CANVAS_WIDTH};
use gameplay_rng::GameplayRngPlugin;
use intro_completion::IntroCompletionPlugin;
use intro_dialogue::IntroDialoguePlugin;
use intro_transition::IntroTransitionPlugin;
use name_entry::NameEntryPlugin;
use new_game_install::NewGameInstallPlugin;
use playtime::Playtime;
use save_ui::SaveUiPlugin;
use scenario_audio::{BgmIndex, SfxIndex};
use scenario_manifest::Manifest;
use scenario_manifest_asset::{ActiveManifestLoad, ScenarioManifestAssetPlugin};
use scenario_new_game_assets::ScenarioNewGameAssetsPlugin;
use scenario_root::ScenarioRoot;
use service_ui::ServiceUiPlugin;
use title_screen::TitleScreenPlugin;
use tmx_ground_asset::TmxGroundAssetPlugin;
use tsx_atlas_asset::TsxAtlasAssetPlugin;
use ui_theme::UiTheme;
use world_actor::WorldActorPlugin;
use world_audio::BgmIndexAssetLoader;
use world_audio::WorldAudioPlugin;
use world_debug_overlay::WorldDebugOverlayPlugin;
use world_encounter::{BattleEntryPlugin, WorldEncounterPlugin};
use world_interaction::SfxIndexAssetLoader;
use world_interaction::WorldInteractionPlugin;
use world_object::WorldObjectPlugin;
use world_player::WorldPlayerPlugin;
use world_transition::WorldTransitionPlugin;

/// Runs the RPG process, routing CLI tools before launching the Bevy application.
pub fn run() -> std::process::ExitCode {
    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();
    std::process::ExitCode::from(cli::run(
        std::env::args_os().skip(1),
        &mut stdout,
        &mut stderr,
        run_game,
    ))
}

fn run_game(scenario_root: ScenarioRoot) {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(AudioPlugin {
                    global_volume: GlobalVolume::new(
                        if std::env::var_os("RPG_S1_MUTE_AUDIO").is_some() {
                            Volume::Linear(0.0)
                        } else {
                            Volume::Linear(1.0)
                        },
                    ),
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: cli::production_asset_base().to_string_lossy().into_owned(),
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "RPG".into(),
                        resolution: (LOGICAL_CANVAS_WIDTH, LOGICAL_CANVAS_HEIGHT).into(),
                        present_mode: PresentMode::AutoVsync,
                        resizable: true,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(ClearColor(UiTheme::default().clear_color))
        .insert_resource(scenario_root)
        .add_plugins(ScenarioManifestAssetPlugin)
        .add_plugins(ScenarioAudioAssetPlugin)
        .add_systems(Update, sync_window_title_from_manifest)
        .add_plugins(ScenarioNewGameAssetsPlugin)
        .add_plugins(FieldMenuDomainPlugin)
        .add_plugins(TsxAtlasAssetPlugin)
        .add_plugins(TmxGroundAssetPlugin)
        .add_plugins(EncounterAssetPlugin)
        .init_resource::<Playtime>()
        .add_plugins(GameplayRngPlugin)
        .insert_state(AppState::Title)
        .add_plugins(AppStateTransitionPlugin)
        .add_plugins(ActionInputPlugin)
        .add_plugins(FixedGameplayCanvasPlugin)
        .add_plugins(TitleScreenPlugin)
        .add_plugins(SaveUiPlugin)
        .add_plugins(NameEntryPlugin)
        .add_plugins(NewGameInstallPlugin)
        .add_plugins(IntroDialoguePlugin)
        .add_plugins(IntroCompletionPlugin)
        .add_plugins(IntroTransitionPlugin)
        .add_plugins(WorldAudioPlugin)
        .add_plugins(WorldActorPlugin)
        .add_plugins(WorldObjectPlugin)
        .add_plugins(WorldTransitionPlugin)
        .add_plugins(AutosavePlugin)
        .add_plugins(WorldEncounterPlugin)
        .add_plugins(WorldInteractionPlugin)
        .add_plugins(ServiceUiPlugin)
        .add_plugins(WorldPlayerPlugin)
        .add_plugins(WorldDebugOverlayPlugin)
        .add_plugins(FieldMenuPlugin)
        .add_plugins(BattleEntryPlugin)
        .add_plugins(BattlePlugin)
        .add_plugins(GameOverPlugin)
        .run();
}

pub(crate) struct ScenarioAudioAssetPlugin;

impl Plugin for ScenarioAudioAssetPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<BgmIndex>()
            .init_asset::<SfxIndex>()
            .init_asset_loader::<BgmIndexAssetLoader>()
            .init_asset_loader::<SfxIndexAssetLoader>();
    }
}

fn sync_window_title_from_manifest(
    active: Res<ActiveManifestLoad>,
    manifests: Res<Assets<Manifest>>,
    mut primary_window: Query<&mut Window, With<PrimaryWindow>>,
) {
    let Some(manifest) = active.manifest(&manifests) else {
        return;
    };
    let Ok(mut window) = primary_window.single_mut() else {
        return;
    };
    if window.title != manifest.window_title {
        window.title.clone_from(&manifest.window_title);
    }
}
