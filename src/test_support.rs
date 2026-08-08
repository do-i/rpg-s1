use bevy::{
    asset::{AssetMetaCheck, AssetPlugin},
    audio::{AudioPlugin, AudioSource},
    prelude::*,
    render::RenderPlugin,
    state::app::StatesPlugin,
    window::WindowPlugin,
};

use crate::{
    action_input::ActionInputPlugin,
    app_state::{AppState, AppStateTransitionPlugin},
    title_screen::TitleScreenPlugin,
};

/// Builds the real title-screen app surface without platform, rendering, or audio plugins.
///
/// `AssetPlugin` provides typed handles to the title entry systems, but intentionally no image,
/// font, or audio loaders. Tests exercise ECS construction without reading live assets or
/// initializing GPU and audio backends.
pub(crate) fn headless_title_app(initial_state: AppState) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(StatesPlugin)
        .add_plugins(AssetPlugin {
            meta_check: AssetMetaCheck::Never,
            ..default()
        })
        .init_asset::<Image>()
        .init_asset::<Font>()
        .init_asset::<AudioSource>()
        .init_resource::<ButtonInput<KeyCode>>()
        .insert_state(initial_state)
        .add_plugins(AppStateTransitionPlugin)
        .add_plugins(ActionInputPlugin)
        .add_plugins(TitleScreenPlugin);
    app
}

#[test]
fn headless_title_app_advances_without_a_window() {
    let mut app = headless_title_app(AppState::Title);

    assert!(!app.is_plugin_added::<WindowPlugin>());
    assert!(!app.is_plugin_added::<RenderPlugin>());
    assert!(!app.is_plugin_added::<AudioPlugin>());

    app.update();

    assert_eq!(
        app.world_mut().query::<&Window>().iter(app.world()).count(),
        0
    );
}
