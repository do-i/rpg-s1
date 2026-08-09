//! Manifest-driven rendering and text editing for the new-game name entry screen.
//!
//! Input is contextual: this screen consumes ordered [`KeyboardInput`] messages directly rather
//! than the shell action map, where Space also means Confirm. The pinned Python screen has no
//! cancel action; Escape-to-Title is the additional behavior required by the Rust port plan.

use bevy::{
    ecs::system::SystemParam,
    input::{ButtonState, keyboard::KeyboardInput},
    prelude::*,
};

use crate::{
    app_state::{AppState, AppStateTransitionRequest},
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
        app.add_message::<KeyboardInput>()
            .add_message::<NameEntryConfirmed>()
            .init_resource::<UiTheme>()
            .init_resource::<NameEntryDraft>()
            .init_resource::<NameEntryViewState>()
            .add_systems(OnEnter(AppState::NameEntry), reset_name_entry)
            .add_systems(
                Update,
                render_name_entry_when_manifest_is_terminal.run_if(in_state(AppState::NameEntry)),
            )
            // This reader must advance in every state so the key that selects New Game (and any
            // other out-of-context text) cannot replay when NameEntry becomes ready.
            .add_systems(
                Update,
                handle_name_entry_keyboard.after(render_name_entry_when_manifest_is_terminal),
            )
            .add_systems(
                PostUpdate,
                sync_name_entry_text.run_if(in_state(AppState::NameEntry)),
            )
            .add_systems(OnExit(AppState::NameEntry), cleanup_name_entry);
    }
}

/// The editable value and immutable manifest fallback for one screen visit.
#[derive(Debug, Default, Resource, Eq, PartialEq)]
pub(crate) struct NameEntryDraft {
    name: String,
    default_name: String,
}

/// Confirmed normalized protagonist name consumed by M3.13.
#[derive(Clone, Debug, Eq, Message, PartialEq)]
pub(crate) struct NameEntryConfirmed {
    name: String,
}

impl NameEntryConfirmed {
    fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
    #[cfg(test)]
    pub(crate) fn for_test(name: impl Into<String>) -> Self {
        Self::new(name)
    }
    pub(crate) fn name(&self) -> &str {
        &self.name
    }
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
            draft.default_name = manifest.protagonist.name.clone();
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

fn handle_name_entry_keyboard(
    mut keyboard: MessageReader<KeyboardInput>,
    view: Res<NameEntryViewState>,
    mut draft: ResMut<NameEntryDraft>,
    mut confirmations: MessageWriter<NameEntryConfirmed>,
    mut transitions: MessageWriter<AppStateTransitionRequest>,
) {
    let ready = *view == NameEntryViewState::Ready;
    let mut terminal_action_seen = false;

    for input in keyboard.read() {
        if !ready || terminal_action_seen || input.state != ButtonState::Pressed {
            continue;
        }

        match input.key_code {
            KeyCode::Escape if !input.repeat => {
                transitions.write(AppStateTransitionRequest::new(AppState::Title));
                terminal_action_seen = true;
            }
            KeyCode::Enter | KeyCode::NumpadEnter if !input.repeat => {
                confirmations.write(NameEntryConfirmed::new(normalized_name(&draft)));
                terminal_action_seen = true;
            }
            KeyCode::Backspace => {
                draft.name.pop();
            }
            _ => {
                if let Some(text) = input.text.as_deref() {
                    append_name_text(&mut draft.name, text);
                }
            }
        }
    }
}

/// Appends one scalar at a time so a multi-scalar platform text event cannot cross the cap.
///
/// Python checks its length only before appending an entire `TEXTINPUT` payload, so a dead-key
/// event carrying multiple scalars can exceed 12 there. Rust deliberately keeps the invariant
/// strict for every event and counts Unicode scalar values rather than UTF-8 bytes.
fn append_name_text(name: &mut String, text: &str) {
    let remaining = NAME_MAX_LENGTH.saturating_sub(name.chars().count());
    name.extend(
        text.chars()
            .filter(|character| !character.is_control())
            .take(remaining),
    );
}

fn normalized_name(draft: &NameEntryDraft) -> String {
    let trimmed = draft.name.trim();
    if trimmed.is_empty() {
        draft.default_name.clone()
    } else {
        trimmed.to_owned()
    }
}

fn sync_name_entry_text(
    draft: Res<NameEntryDraft>,
    view: Res<NameEntryViewState>,
    mut texts: Query<(&mut Text, Option<&NameEntryInput>, Option<&NameEntryCount>)>,
) {
    if *view != NameEntryViewState::Ready || !draft.is_changed() {
        return;
    }

    for (mut text, input, count) in &mut texts {
        if input.is_some() {
            text.0 = format!("{}|", draft.name);
        } else if count.is_some() {
            text.0 = format!("{}/{}", draft.name.chars().count(), NAME_MAX_LENGTH);
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

    use bevy::{input::keyboard::Key, text::FontSource};

    use super::*;
    use crate::{
        action_input::{ActionState, AppAction},
        app_state::{AppState, AppStateTransitionRequest},
        game_state::GameState,
        scenario_manifest_asset::ActiveManifestStatus,
        scenario_root::ScenarioRoot,
        test_support::headless_title_app_with_asset_base,
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
            &format!("  name: \"{name}\"\n"),
            1,
        )
    }

    fn app_for(base: &InventedAssetBase, package_key: &str) -> App {
        app_for_state(base, package_key, AppState::NameEntry)
    }

    fn app_for_state(base: &InventedAssetBase, package_key: &str, initial_state: AppState) -> App {
        headless_title_app_with_asset_base(
            initial_state,
            base.path().to_string_lossy().into_owned(),
            ScenarioRoot::try_for_package_key(package_key).expect("invented key should be valid"),
        )
    }

    fn ready_app(default_name: &str) -> (InventedAssetBase, App) {
        let base = InventedAssetBase::new("invented_campaign", &manifest_with_name(default_name));
        let mut app = app_for(&base, "invented_campaign");
        assert_eq!(update_until_terminal(&mut app), ActiveManifestStatus::Ready);
        app.update();
        app.update();
        assert_eq!(
            app.world().resource::<NameEntryViewState>(),
            &NameEntryViewState::Ready
        );
        (base, app)
    }

    fn input_only_app(view: NameEntryViewState, name: &str, default_name: &str) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<KeyboardInput>()
            .add_message::<NameEntryConfirmed>()
            .add_message::<AppStateTransitionRequest>()
            .insert_resource(view)
            .insert_resource(NameEntryDraft {
                name: name.to_owned(),
                default_name: default_name.to_owned(),
            })
            .add_systems(Update, handle_name_entry_keyboard);
        app
    }

    fn write_keyboard(
        app: &mut App,
        key_code: KeyCode,
        state: ButtonState,
        text: Option<&str>,
        repeat: bool,
    ) {
        let logical_key = match key_code {
            KeyCode::Backspace => Key::Backspace,
            KeyCode::Enter | KeyCode::NumpadEnter => Key::Enter,
            KeyCode::Escape => Key::Escape,
            KeyCode::Space => Key::Space,
            _ => Key::Character(text.unwrap_or_default().into()),
        };
        app.world_mut().write_message(KeyboardInput {
            key_code,
            logical_key,
            state,
            text: text.map(Into::into),
            repeat,
            window: Entity::PLACEHOLDER,
        });
    }

    fn press(app: &mut App, key_code: KeyCode, text: Option<&str>) {
        write_keyboard(app, key_code, ButtonState::Pressed, text, false);
    }

    fn confirmed_names(app: &mut App) -> Vec<String> {
        app.world_mut()
            .resource_mut::<Messages<NameEntryConfirmed>>()
            .drain()
            .map(|confirmation| confirmation.name().to_owned())
            .collect()
    }

    fn marked_text<M: Component>(world: &mut World) -> String {
        world
            .query_filtered::<&Text, With<M>>()
            .single(world)
            .expect("marked text should exist exactly once")
            .0
            .clone()
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

    fn update_until_name_entry_ready_without_confirmation(app: &mut App) {
        for _ in 0..1_000 {
            app.update();
            assert!(
                confirmed_names(app).is_empty(),
                "input from an earlier state must not confirm the name entry"
            );
            if *app.world().resource::<NameEntryViewState>() == NameEntryViewState::Ready {
                return;
            }
            std::thread::yield_now();
        }
        panic!("name entry did not become ready");
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

    #[test]
    fn scalar_reducer_filters_controls_deletes_unicode_and_is_safe_when_empty() {
        let mut name = String::new();

        append_name_text(&mut name, "A\u{7}é🙂\n");
        assert_eq!(name, "Aé🙂");
        assert_eq!(name.chars().count(), 3);

        assert_eq!(name.pop(), Some('🙂'));
        assert_eq!(name.pop(), Some('é'));
        assert_eq!(name.pop(), Some('A'));
        assert_eq!(name.pop(), None);
        assert!(name.is_empty());
    }

    #[test]
    fn multi_scalar_text_event_stops_at_twelve_instead_of_overshooting_like_python() {
        let mut name = "1234567890".to_owned();

        append_name_text(&mut name, "é🙂漢");

        assert_eq!(name, "1234567890é🙂");
        assert_eq!(name.chars().count(), NAME_MAX_LENGTH);
        append_name_text(&mut name, "Z");
        assert_eq!(name, "1234567890é🙂");
    }

    #[test]
    fn ordered_unicode_input_and_backspace_update_visible_value_and_count_same_frame() {
        let (_base, mut app) = ready_app("1234567890");
        press(&mut app, KeyCode::KeyA, Some("é🙂漢"));

        app.update();

        assert_eq!(
            app.world().resource::<NameEntryDraft>().name,
            "1234567890é🙂"
        );
        assert_eq!(
            marked_text::<NameEntryInput>(app.world_mut()),
            "1234567890é🙂|"
        );
        assert_eq!(marked_text::<NameEntryCount>(app.world_mut()), "12/12");

        press(&mut app, KeyCode::Backspace, None);
        write_keyboard(
            &mut app,
            KeyCode::Backspace,
            ButtonState::Pressed,
            None,
            true,
        );
        app.update();

        assert_eq!(app.world().resource::<NameEntryDraft>().name, "1234567890");
        assert_eq!(
            marked_text::<NameEntryInput>(app.world_mut()),
            "1234567890|"
        );
        assert_eq!(marked_text::<NameEntryCount>(app.world_mut()), "10/12");
    }

    #[test]
    fn same_frame_text_backspace_repeat_and_confirmation_follow_message_order() {
        let mut app = input_only_app(NameEntryViewState::Ready, "", "Nyra");
        press(&mut app, KeyCode::KeyA, Some("Aβ"));
        press(&mut app, KeyCode::Backspace, None);
        write_keyboard(
            &mut app,
            KeyCode::KeyZ,
            ButtonState::Pressed,
            Some("Z"),
            true,
        );
        press(&mut app, KeyCode::Enter, None);

        app.update();

        assert_eq!(app.world().resource::<NameEntryDraft>().name, "AZ");
        assert_eq!(confirmed_names(&mut app), ["AZ"]);
    }

    #[test]
    fn space_inserts_text_even_when_the_shell_action_map_reports_confirm() {
        let (_base, mut app) = ready_app("A");
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Space);
        press(&mut app, KeyCode::Space, Some(" "));

        app.update();

        assert!(
            app.world()
                .resource::<ActionState>()
                .just_pressed(AppAction::Confirm)
        );
        assert_eq!(app.world().resource::<NameEntryDraft>().name, "A ");
        assert_eq!(marked_text::<NameEntryInput>(app.world_mut()), "A |");
        assert!(confirmed_names(&mut app).is_empty());
    }

    #[test]
    fn title_new_game_enter_is_drained_before_manifest_ready_name_entry() {
        let base = InventedAssetBase::new("invented_campaign", &manifest_with_name("Nyra"));
        let mut app = app_for_state(&base, "invented_campaign", AppState::Title);
        app.update();

        // These are the two views of the same physical press: ButtonInput selects New Game,
        // while KeyboardInput carries the platform event that NameEntry must consume now.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        press(&mut app, KeyCode::Enter, None);
        app.update();
        assert!(confirmed_names(&mut app).is_empty());

        app.update();
        assert_eq!(
            app.world().resource::<State<AppState>>().get(),
            &AppState::NameEntry
        );
        assert!(confirmed_names(&mut app).is_empty());
        update_until_name_entry_ready_without_confirmation(&mut app);

        assert_eq!(app.world().resource::<NameEntryDraft>().name, "Nyra");
        assert_eq!(marked_text::<NameEntryInput>(app.world_mut()), "Nyra|");

        press(&mut app, KeyCode::Enter, None);
        app.update();
        assert_eq!(confirmed_names(&mut app), ["Nyra"]);
    }

    #[test]
    fn keyboard_emitted_in_another_app_state_is_drained_before_name_entry() {
        let base = InventedAssetBase::new("invented_campaign", &manifest_with_name("Nyra"));
        let mut app = app_for_state(&base, "invented_campaign", AppState::World);
        app.update();

        press(&mut app, KeyCode::KeyA, Some("X"));
        press(&mut app, KeyCode::Backspace, None);
        press(&mut app, KeyCode::Enter, None);
        press(&mut app, KeyCode::Escape, None);
        app.update();
        assert!(confirmed_names(&mut app).is_empty());

        app.world_mut()
            .write_message(AppStateTransitionRequest::new(AppState::NameEntry));
        app.update();
        assert!(confirmed_names(&mut app).is_empty());
        app.update();
        assert_eq!(
            app.world().resource::<State<AppState>>().get(),
            &AppState::NameEntry
        );
        assert!(confirmed_names(&mut app).is_empty());
        update_until_name_entry_ready_without_confirmation(&mut app);

        assert_eq!(app.world().resource::<NameEntryDraft>().name, "Nyra");
        assert_eq!(marked_text::<NameEntryInput>(app.world_mut()), "Nyra|");
    }

    #[test]
    fn enter_and_keypad_enter_emit_one_trimmed_confirmation_without_transition() {
        for confirm_key in [KeyCode::Enter, KeyCode::NumpadEnter] {
            let mut app = input_only_app(NameEntryViewState::Ready, "  Éla  ", "Nyra");
            write_keyboard(&mut app, confirm_key, ButtonState::Released, None, false);
            write_keyboard(&mut app, confirm_key, ButtonState::Pressed, None, true);
            press(&mut app, confirm_key, None);
            press(&mut app, confirm_key, None);

            app.update();

            assert_eq!(confirmed_names(&mut app), ["Éla"]);
            assert_eq!(
                app.world()
                    .resource::<Messages<AppStateTransitionRequest>>()
                    .len(),
                0
            );
            assert!(app.world().get_resource::<GameState>().is_none());
        }
    }

    #[test]
    fn whitespace_confirmation_falls_back_to_the_separate_manifest_default() {
        let mut app = input_only_app(NameEntryViewState::Ready, " \t\u{2003} ", "Nyra");
        press(&mut app, KeyCode::Enter, None);

        app.update();

        assert_eq!(confirmed_names(&mut app), ["Nyra"]);
        assert_eq!(
            app.world().resource::<NameEntryDraft>().default_name,
            "Nyra"
        );
    }

    #[test]
    fn escape_requests_title_once_discards_through_exit_and_creates_no_game_state() {
        let (_base, mut app) = ready_app("Nyra");
        write_keyboard(
            &mut app,
            KeyCode::Escape,
            ButtonState::Released,
            None,
            false,
        );
        write_keyboard(&mut app, KeyCode::Escape, ButtonState::Pressed, None, true);
        press(&mut app, KeyCode::Escape, None);
        press(&mut app, KeyCode::Enter, None);

        app.update();

        assert_eq!(
            app.world()
                .resource::<Messages<AppStateTransitionRequest>>()
                .len(),
            1
        );
        assert!(confirmed_names(&mut app).is_empty());
        assert!(app.world().get_resource::<GameState>().is_none());

        app.update();

        assert_eq!(
            app.world().resource::<State<AppState>>().get(),
            &AppState::Title
        );
        assert_eq!(
            app.world().resource::<NameEntryDraft>(),
            &NameEntryDraft::default()
        );
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<NameEntryEntity>>()
                .iter(app.world())
                .count(),
            0
        );
    }

    #[test]
    fn waiting_and_failed_input_is_consumed_and_cannot_replay_after_ready() {
        let mut app = input_only_app(NameEntryViewState::WaitingForManifest, "Nyra", "Nyra");
        press(&mut app, KeyCode::KeyA, Some("X"));
        press(&mut app, KeyCode::Backspace, None);
        press(&mut app, KeyCode::Enter, None);
        press(&mut app, KeyCode::Escape, None);
        app.update();

        *app.world_mut().resource_mut::<NameEntryViewState>() = NameEntryViewState::Failed;
        press(&mut app, KeyCode::KeyA, Some("Y"));
        press(&mut app, KeyCode::Enter, None);
        app.update();

        *app.world_mut().resource_mut::<NameEntryViewState>() = NameEntryViewState::Ready;
        app.update();

        assert_eq!(app.world().resource::<NameEntryDraft>().name, "Nyra");
        assert!(confirmed_names(&mut app).is_empty());
        assert_eq!(
            app.world()
                .resource::<Messages<AppStateTransitionRequest>>()
                .len(),
            0
        );
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
