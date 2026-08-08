mod action_input;
pub mod app_state;
pub mod gameplay_canvas;
pub mod gameplay_rng;
pub mod manifest_path_validation;
pub mod playtime;
pub mod scenario_class;
pub mod scenario_condition;
pub mod scenario_dialogue;
pub mod scenario_item;
pub mod scenario_manifest;
pub mod scenario_map;
pub mod scenario_party;
pub mod scenario_path;
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
    prelude::*,
    window::{PresentMode, WindowPlugin},
};
use gameplay_canvas::{FixedGameplayCanvasPlugin, LOGICAL_CANVAS_HEIGHT, LOGICAL_CANVAS_WIDTH};
use gameplay_rng::GameplayRngPlugin;
use playtime::Playtime;
use scenario_root::ScenarioRoot;
use title_screen::TitleScreenPlugin;
use ui_theme::UiTheme;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Chronicles of the Lost Flame".into(),
                resolution: (LOGICAL_CANVAS_WIDTH, LOGICAL_CANVAS_HEIGHT).into(),
                present_mode: PresentMode::AutoVsync,
                resizable: true,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(UiTheme::default().clear_color))
        .init_resource::<ScenarioRoot>()
        .init_resource::<Playtime>()
        .add_plugins(GameplayRngPlugin)
        .insert_state(AppState::Title)
        .add_plugins(AppStateTransitionPlugin)
        .add_plugins(ActionInputPlugin)
        .add_plugins(FixedGameplayCanvasPlugin)
        .add_plugins(TitleScreenPlugin)
        .run();
}
