pub mod app_state;
mod title_screen;

#[cfg(test)]
mod test_support;

use app_state::AppState;
use bevy::{
    prelude::*,
    window::{PresentMode, WindowPlugin},
};
use title_screen::TitleScreenPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Chronicles of the Lost Flame".into(),
                resolution: (1280, 766).into(),
                present_mode: PresentMode::AutoVsync,
                resizable: true,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb_u8(10, 10, 30)))
        .insert_state(AppState::Title)
        .add_plugins(TitleScreenPlugin)
        .run();
}
