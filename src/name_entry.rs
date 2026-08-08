//! Manifest-driven rendering for the new-game name entry screen.
//!
//! This slice deliberately owns presentation and the editable draft only. Keyboard editing and
//! confirmation are introduced by the following milestones, so this plugin never changes the
//! draft after initializing it from the active manifest.

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app_state::AppState,
    gameplay_canvas::fixed_gameplay_camera,
    scenario_manifest::{Manifest, ManifestFont},
    scenario_manifest_asset::{ActiveManifestLoad, ActiveManifestStatus},
    scenario_root::ScenarioRoot,
    ui_theme::UiTheme,
};

pub(crate) const NAME_MAX_LENGTH: usize = 12;
const PROMPT: &str = "Enter your name";
const HINT: &str = "ENTER to confirm";

/// Adds the manifest-backed name-entry presentation lifecycle.
pub struct NameEntryPlugin;

impl Plugin for NameEntryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiTheme>()
            .init_resource::<NameEntryDraft>()
            .init_resource::<NameEntryViewState>()
            .add_systems(OnEnter(AppState::NameEntry), reset_name_entry)
            .add_systems(
                Update,
                render_name_entry_when_manifest_is_terminal.run_if(in_state(AppState::NameEntry)),
            )
            .add_systems(OnExit(AppState::NameEntry), cleanup_name_entry);
    }
}

/// The name that M3.12 will edit and M3.13 will apply to runtime state.
#[derive(Debug, Default, Resource, Eq, PartialEq)]
pub(crate) struct NameEntryDraft {
    name: String,
}

/// Whether this screen is still awaiting a manifest, has rendered it, or rendered its error.
#[derive(Debug, Default, Resource, Eq, PartialEq)]
pub(crate) enum NameEntryViewState {
    #[default]
    WaitingForManifest,
    Ready,
    Failed,
}

#[derive(Component)]
struct NameEntryEntity;

#[derive(Component)]
struct NameEntryCamera;

#[derive(Component)]
struct NameEntryPrompt;

#[derive(Component)]
struct NameEntryInput;

#[derive(Component)]
struct NameEntryCount;

#[derive(Component)]
struct NameEntryHint;

#[derive(Component)]
struct NameEntryFailure;

#[derive(SystemParam)]
struct ActiveNameEntryManifest<'w> {
    asset_server: Res<'w, AssetServer>,
    root: Res<'w, ScenarioRoot>,
    active: Res<'w, ActiveManifestLoad>,
    manifests: Res<'w, Assets<Manifest>>,
}

fn reset_name_entry(mut draft: ResMut<NameEntryDraft>, mut view: ResMut<NameEntryViewState>) {
    *draft = NameEntryDraft::default();
    *view = NameEntryViewState::WaitingForManifest;
}

fn render_name_entry_when_manifest_is_terminal(
    mut commands: Commands,
    source: ActiveNameEntryManifest,
    theme: Res<UiTheme>,
    mut draft: ResMut<NameEntryDraft>,
    mut view: ResMut<NameEntryViewState>,
) {
    if *view != NameEntryViewState::WaitingForManifest {
        return;
    }

    match source.active.status() {
        ActiveManifestStatus::Loading => {}
        ActiveManifestStatus::Ready => {
            let Some(manifest) = source.active.manifest(&source.manifests) else {
                return;
            };
            draft.name = manifest.protagonist.name.clone();
            spawn_name_entry(
                &mut commands,
                &source.asset_server,
                &source.root,
                manifest,
                &theme,
                &draft,
            );
            *view = NameEntryViewState::Ready;
        }
        ActiveManifestStatus::Failed => {
            let message = source
                .active
                .failure()
                .map(ToString::to_string)
                .unwrap_or_else(|| "scenario manifest loading failed".to_owned());
            spawn_name_entry_failure(&mut commands, &theme, &message);
            *view = NameEntryViewState::Failed;
        }
    }
}

fn spawn_name_entry(
    commands: &mut Commands,
    asset_server: &AssetServer,
    root: &ScenarioRoot,
    manifest: &Manifest,
    theme: &UiTheme,
    draft: &NameEntryDraft,
) {
    commands.spawn((
        fixed_gameplay_camera(),
        Camera {
            clear_color: ClearColorConfig::Custom(theme.clear_color),
            ..default()
        },
        NameEntryCamera,
        NameEntryEntity,
    ));

    let font = load_manifest_font(asset_server, root, &manifest.font);
    let count = format!("{}/{}", draft.name.chars().count(), NAME_MAX_LENGTH);
    commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            NameEntryEntity,
        ))
        .with_children(|root| {
            root.spawn(Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|column| {
                column.spawn((
                    Text::new(PROMPT),
                    text_font(&font, theme.name_entry_prompt_font_size),
                    TextColor(theme.name_entry_prompt_color),
                    Node {
                        margin: UiRect::bottom(px(28)),
                        ..default()
                    },
                    NameEntryPrompt,
                ));
                column
                    .spawn(Node {
                        width: px(320),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|input_group| {
                        input_group
                            .spawn((
                                Node {
                                    width: percent(100),
                                    height: px(60),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(px(2)),
                                    ..default()
                                },
                                BackgroundColor(theme.name_entry_box_color),
                                BorderColor::all(theme.name_entry_border_color),
                            ))
                            .with_children(|box_node| {
                                box_node.spawn((
                                    Text::new(format!("{}|", draft.name)),
                                    text_font(&font, theme.name_entry_input_font_size),
                                    TextColor(theme.name_entry_input_color),
                                    NameEntryInput,
                                ));
                            });
                        input_group.spawn((
                            Text::new(count),
                            text_font(&font, theme.name_entry_hint_font_size),
                            TextColor(theme.name_entry_hint_color),
                            Node {
                                align_self: AlignSelf::End,
                                margin: UiRect::top(px(4)),
                                ..default()
                            },
                            NameEntryCount,
                        ));
                    });
                column.spawn((
                    Text::new(HINT),
                    text_font(&font, theme.name_entry_hint_font_size),
                    TextColor(theme.name_entry_hint_color),
                    Node {
                        margin: UiRect::top(px(36)),
                        ..default()
                    },
                    NameEntryHint,
                ));
            });
        });
}

fn spawn_name_entry_failure(commands: &mut Commands, theme: &UiTheme, message: &str) {
    commands.spawn((
        fixed_gameplay_camera(),
        Camera {
            clear_color: ClearColorConfig::Custom(theme.clear_color),
            ..default()
        },
        NameEntryCamera,
        NameEntryEntity,
    ));
    commands.spawn((
        Text::new(message),
        TextColor(theme.name_entry_hint_color),
        TextLayout::justify(Justify::Center),
        Node {
            width: percent(100),
            height: percent(100),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        NameEntryFailure,
        NameEntryEntity,
    ));
}

fn load_manifest_font(
    asset_server: &AssetServer,
    root: &ScenarioRoot,
    font: &ManifestFont,
) -> Handle<Font> {
    asset_server.load(root.resolve(&font.path))
}

fn text_font(font: &Handle<Font>, size: f32) -> TextFont {
    TextFont {
        font: font.clone().into(),
        font_size: FontSize::Px(size),
        ..default()
    }
}

fn cleanup_name_entry(
    mut commands: Commands,
    entities: Query<Entity, With<NameEntryEntity>>,
    mut draft: ResMut<NameEntryDraft>,
    mut view: ResMut<NameEntryViewState>,
) {
    for entity in &entities {
        commands.entity(entity).despawn();
    }
    *draft = NameEntryDraft::default();
    *view = NameEntryViewState::WaitingForManifest;
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    use bevy::text::FontSource;

    use super::*;
    use crate::{
        app_state::AppState, scenario_manifest_asset::ActiveManifestStatus,
        scenario_root::ScenarioRoot, test_support::headless_title_app_with_asset_base,
    };

    static NEXT_TEMPORARY_ID: AtomicU64 = AtomicU64::new(0);

    struct InventedAssetBase(PathBuf);

    impl InventedAssetBase {
        fn new(package_key: &str, manifest: &str) -> Self {
            let unique = NEXT_TEMPORARY_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "rpg-s1-name-entry-assets-{}-{unique}",
                std::process::id()
            ));
            let package = path.join("scenarios").join(package_key);
            fs::create_dir_all(&package).expect("invented package should be created");
            fs::write(package.join("manifest.yaml"), manifest)
                .expect("invented manifest should be written");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for InventedAssetBase {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("invented asset base should be removed");
        }
    }

    fn manifest_with_name(name: &str) -> String {
        include_str!("../tests/fixtures/rusted-kingdoms-manifest-complete.yaml").replacen(
            "  name: Aric\n",
            &format!("  name: {name}\n"),
            1,
        )
    }

    fn app_for(base: &InventedAssetBase, package_key: &str) -> App {
        headless_title_app_with_asset_base(
            AppState::NameEntry,
            base.path().to_string_lossy().into_owned(),
            ScenarioRoot::try_for_package_key(package_key).expect("invented key should be valid"),
        )
    }

    fn update_until_terminal(app: &mut App) -> ActiveManifestStatus {
        for _ in 0..1_000 {
            app.update();
            let status = app.world().resource::<ActiveManifestLoad>().status();
            if status != ActiveManifestStatus::Loading {
                return status;
            }
            std::thread::yield_now();
        }
        panic!("manifest request did not finish");
    }

    fn visible_texts(world: &mut World) -> Vec<String> {
        let mut texts = world
            .query::<&Text>()
            .iter(world)
            .map(|text| text.0.clone())
            .collect::<Vec<_>>();
        texts.sort();
        texts
    }

    fn name_entry_font_paths(world: &mut World) -> Vec<String> {
        let handles = world
            .query::<&TextFont>()
            .iter(world)
            .map(|font| match &font.font {
                FontSource::Handle(handle) => handle.clone(),
                source => panic!("name entry must use a manifest font handle, got {source:?}"),
            })
            .collect::<Vec<_>>();
        let asset_server = world.resource::<AssetServer>();
        handles
            .iter()
            .map(|handle| {
                asset_server
                    .get_path(handle.id())
                    .expect("manifest font handle should retain its AssetServer path")
                    .path()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect()
    }

    #[test]
    fn waits_for_the_manifest_then_renders_the_invented_default_name_once() {
        let assets = InventedAssetBase::new("invented_campaign", &manifest_with_name("Nyra"));
        let mut app = app_for(&assets, "invented_campaign");

        assert_eq!(
            app.world().resource::<NameEntryViewState>(),
            &NameEntryViewState::WaitingForManifest
        );
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<NameEntryEntity>>()
                .iter(app.world())
                .count(),
            0
        );

        assert_eq!(update_until_terminal(&mut app), ActiveManifestStatus::Ready);
        app.update();
        app.update();

        let texts = visible_texts(app.world_mut());
        assert_eq!(texts, vec!["4/12", HINT, PROMPT, "Nyra|"]);
        assert_eq!(
            name_entry_font_paths(app.world_mut()),
            vec!["scenarios/invented_campaign/assets/fonts/Philosopher-Regular.ttf"; 4]
        );
        assert_eq!(app.world().resource::<NameEntryDraft>().name, "Nyra");
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<NameEntryCamera>>()
                .iter(app.world())
                .count(),
            1
        );
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<NameEntryEntity>>()
                .iter(app.world())
                .count(),
            2
        );
    }

    #[test]
    fn failed_manifest_renders_one_package_relative_error_and_no_default_prompt() {
        let broken = manifest_with_name("Nyra").replacen(
            "  cursor_icon: assets/images/icons/arrow-head-right.webp\n",
            "",
            1,
        );
        let assets = InventedAssetBase::new("broken_campaign", &broken);
        let mut app = app_for(&assets, "broken_campaign");

        assert_eq!(
            update_until_terminal(&mut app),
            ActiveManifestStatus::Failed
        );
        app.update();
        app.update();

        let texts = visible_texts(app.world_mut());
        assert_eq!(texts.len(), 1);
        assert!(texts[0].starts_with("broken_campaign:manifest.yaml:"));
        assert!(!texts.contains(&PROMPT.to_owned()));
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<NameEntryCamera>>()
                .iter(app.world())
                .count(),
            1
        );
        assert_eq!(
            app.world().resource::<NameEntryViewState>(),
            &NameEntryViewState::Failed
        );
    }

    #[test]
    fn exit_cleans_owned_entities_and_resets_the_draft() {
        let assets = InventedAssetBase::new("invented_campaign", &manifest_with_name("Nyra"));
        let mut app = app_for(&assets, "invented_campaign");
        assert_eq!(update_until_terminal(&mut app), ActiveManifestStatus::Ready);
        app.update();
        app.update();

        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Title);
        app.update();

        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<NameEntryEntity>>()
                .iter(app.world())
                .count(),
            0
        );
        assert!(app.world().resource::<NameEntryDraft>().name.is_empty());
        assert_eq!(
            app.world().resource::<NameEntryViewState>(),
            &NameEntryViewState::WaitingForManifest
        );
    }
}
