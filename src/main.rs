mod action_input;
pub mod app_state;
mod cli;
pub mod game_state;
pub mod gameplay_canvas;
pub mod gameplay_rng;
pub mod manifest_path_validation;
mod name_entry;
pub mod new_game;
pub mod playtime;
pub mod runtime_flags;
pub mod runtime_map;
pub mod runtime_member;
pub mod runtime_opened_boxes;
pub mod runtime_party;
pub mod runtime_quest;
pub mod runtime_repository;
pub mod scenario_audio;
pub mod scenario_balance;
pub mod scenario_battle_background;
pub mod scenario_class;
pub mod scenario_condition;
pub mod scenario_cross_reference;
pub mod scenario_dialogue;
pub mod scenario_duplicate_id;
pub mod scenario_encounter;
pub mod scenario_enemy;
pub mod scenario_item;
pub mod scenario_manifest;
pub mod scenario_manifest_asset;
pub mod scenario_map;
pub mod scenario_new_game_assets;
pub mod scenario_party;
pub mod scenario_path;
pub mod scenario_quest;
pub mod scenario_recipe;
pub mod scenario_root;
pub mod scenario_spatial;
pub mod scenario_yaml;
mod title_screen;
mod ui_theme;

#[cfg(test)]
mod test_support;

use action_input::ActionInputPlugin;
use app_state::{AppState, AppStateTransitionPlugin};
use bevy::{
    asset::AssetPlugin,
    prelude::*,
    window::{PresentMode, WindowPlugin},
};
use gameplay_canvas::{FixedGameplayCanvasPlugin, LOGICAL_CANVAS_HEIGHT, LOGICAL_CANVAS_WIDTH};
use gameplay_rng::GameplayRngPlugin;
use name_entry::NameEntryPlugin;
use playtime::Playtime;
use scenario_manifest_asset::ScenarioManifestAssetPlugin;
use scenario_new_game_assets::ScenarioNewGameAssetsPlugin;
use scenario_root::ScenarioRoot;
use title_screen::TitleScreenPlugin;
use ui_theme::UiTheme;

fn main() -> std::process::ExitCode {
    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();
    std::process::ExitCode::from(cli::run(
        std::env::args_os().skip(1),
        &mut stdout,
        &mut stderr,
        run_game,
    ))
}

fn run_game() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: cli::production_asset_base().to_string_lossy().into_owned(),
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Chronicles of the Lost Flame".into(),
                        resolution: (LOGICAL_CANVAS_WIDTH, LOGICAL_CANVAS_HEIGHT).into(),
                        present_mode: PresentMode::AutoVsync,
                        resizable: true,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(ClearColor(UiTheme::default().clear_color))
        .init_resource::<ScenarioRoot>()
        .add_plugins(ScenarioManifestAssetPlugin)
        .add_plugins(ScenarioNewGameAssetsPlugin)
        .init_resource::<Playtime>()
        .add_plugins(GameplayRngPlugin)
        .insert_state(AppState::Title)
        .add_plugins(AppStateTransitionPlugin)
        .add_plugins(ActionInputPlugin)
        .add_plugins(FixedGameplayCanvasPlugin)
        .add_plugins(TitleScreenPlugin)
        .add_plugins(NameEntryPlugin)
        .run();
}
