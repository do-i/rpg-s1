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
    gameplay_canvas::FixedGameplayCanvasPlugin,
    gameplay_rng::{DEFAULT_GAMEPLAY_SEED, GameplayRng, GameplayRngPlugin},
    intro_completion::IntroCompletionPlugin,
    intro_dialogue::IntroDialoguePlugin,
    intro_transition::IntroTransitionPlugin,
    name_entry::NameEntryPlugin,
    new_game_install::NewGameInstallPlugin,
    playtime::Playtime,
    save_ui::{SaveSlotCatalog, TitleLoadMenu},
    scenario_manifest_asset::ScenarioManifestAssetPlugin,
    scenario_new_game_assets::ScenarioNewGameAssetsPlugin,
    scenario_root::ScenarioRoot,
    title_screen::TitleScreenPlugin,
};

/// Builds the real title-screen app surface without platform, rendering, or audio plugins.
///
/// `AssetPlugin` provides typed handles to the title entry systems, but intentionally no image,
/// font, or audio loaders. Tests exercise ECS construction without reading live assets or
/// initializing GPU and audio backends.
pub(crate) fn headless_title_app(initial_state: AppState) -> App {
    headless_title_app_with_asset_base(
        initial_state,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets").to_owned(),
        ScenarioRoot::default(),
    )
}

/// Builds the headless app against an explicitly selected AssetServer base and scenario package.
pub(crate) fn headless_title_app_with_asset_base(
    initial_state: AppState,
    asset_base: String,
    scenario_root: ScenarioRoot,
) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(StatesPlugin)
        .add_plugins(AssetPlugin {
            file_path: asset_base,
            meta_check: AssetMetaCheck::Never,
            ..default()
        })
        .init_asset::<Image>()
        .init_asset::<Font>()
        .init_asset::<AudioSource>()
        .init_resource::<ButtonInput<KeyCode>>()
        .insert_resource(scenario_root)
        .add_plugins(ScenarioManifestAssetPlugin)
        .add_plugins(ScenarioNewGameAssetsPlugin)
        .init_resource::<Playtime>()
        .add_plugins(GameplayRngPlugin)
        .insert_state(initial_state)
        .add_plugins(AppStateTransitionPlugin)
        .add_plugins(ActionInputPlugin)
        .add_plugins(FixedGameplayCanvasPlugin)
        .init_resource::<SaveSlotCatalog>()
        .init_resource::<TitleLoadMenu>()
        .add_plugins(TitleScreenPlugin)
        .add_plugins(NameEntryPlugin)
        .add_plugins(NewGameInstallPlugin)
        .add_plugins(IntroDialoguePlugin)
        .add_plugins(IntroCompletionPlugin)
        .add_plugins(IntroTransitionPlugin);
    app
}

#[test]
fn headless_title_app_advances_without_a_window() {
    let mut app = headless_title_app(AppState::Title);

    assert!(!app.is_plugin_added::<WindowPlugin>());
    assert!(!app.is_plugin_added::<RenderPlugin>());
    assert!(!app.is_plugin_added::<AudioPlugin>());
    assert_eq!(app.world().resource::<Playtime>().total_seconds(), 0);

    app.update();

    assert_eq!(
        app.world_mut().query::<&Window>().iter(app.world()).count(),
        0
    );

    let actual = app.world_mut().resource_mut::<GameplayRng>().next_u64();
    let expected = GameplayRng::from_seed(DEFAULT_GAMEPLAY_SEED).next_u64();
    assert_eq!(actual, expected);
}
