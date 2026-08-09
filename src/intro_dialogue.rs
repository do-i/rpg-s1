//! Manifest-selected introductory cutscene presentation and linear progression.
//!
//! This slice renders complete authored lines immediately. Typewriter timing, cancellation,
//! completion effects, and the transition into the world remain owned by later milestones.

use bevy::{ecs::system::SystemParam, prelude::*, text::LineBreak};

use crate::{
    action_input::{ActionState, AppAction},
    app_state::AppState,
    gameplay_canvas::fixed_gameplay_camera,
    scenario_balance::BalanceData,
    scenario_dialogue::{CutsceneDialogue, DialogueActions},
    scenario_manifest::Manifest,
    scenario_manifest_asset::ActiveManifestLoad,
    scenario_new_game_assets::{
        ActiveNewGameInputs, ActiveNewGameInputsStatus, track_new_game_inputs,
    },
    scenario_party::PartyCatalog,
    ui_theme::UiTheme,
};

pub(crate) const INTRO_PANEL_WIDTH: f32 = 600.0;
pub(crate) const INTRO_PANEL_HEIGHT: f32 = 180.0;
const INTRO_TEXT_WIDTH: f32 = 552.0;
const INTRO_TEXT_HEIGHT: f32 = 112.0;
const LOADING_MESSAGE: &str = "Loading introduction...";

pub(crate) struct IntroDialoguePlugin;

impl Plugin for IntroDialoguePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<IntroDialogueCompleted>()
            .init_resource::<IntroDialogueProgress>()
            .init_resource::<IntroDialogueViewState>()
            .init_resource::<UiTheme>()
            .add_systems(OnEnter(AppState::Dialogue), setup_intro_dialogue)
            .add_systems(
                Update,
                sync_intro_dialogue_view
                    .after(track_new_game_inputs)
                    .run_if(in_state(AppState::Dialogue)),
            )
            .add_systems(
                Update,
                advance_intro_dialogue
                    .before(sync_intro_dialogue_view)
                    .run_if(in_state(AppState::Dialogue)),
            )
            .add_systems(OnExit(AppState::Dialogue), cleanup_intro_dialogue);
    }
}

/// Typed handoff consumed by M3.15 and M3.16 without applying effects in this screen.
#[derive(Clone, Debug, Eq, Message, PartialEq)]
pub(crate) struct IntroDialogueCompleted {
    on_complete: DialogueActions,
}

impl IntroDialogueCompleted {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "M3.14 defines the typed completion handoff for M3.15 and M3.16"
        )
    )]
    pub(crate) fn on_complete(&self) -> &DialogueActions {
        &self.on_complete
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Resource)]
pub(crate) enum IntroDialogueViewState {
    #[default]
    Loading,
    Ready,
    Failed,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Resource)]
pub(crate) struct IntroDialogueProgress {
    line_index: usize,
    completed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProgressEffect {
    AdvancedTo(usize),
    Completed,
    None,
}

impl IntroDialogueProgress {
    fn confirm(&mut self, line_count: usize) -> ProgressEffect {
        if self.completed {
            return ProgressEffect::None;
        }
        if self.line_index.saturating_add(1) < line_count {
            self.line_index += 1;
            ProgressEffect::AdvancedTo(self.line_index)
        } else {
            self.completed = true;
            ProgressEffect::Completed
        }
    }
}

#[derive(Component)]
struct IntroDialogueEntity;

#[derive(Component)]
struct IntroDialogueCamera;

#[derive(Component)]
struct IntroDialogueRoot;

#[derive(Component)]
struct IntroDialoguePanel;

#[derive(Component)]
struct IntroDialogueLine;

#[derive(Component)]
struct IntroDialogueHint;

#[derive(Component)]
struct IntroDialogueStatus;

#[derive(SystemParam)]
struct ActiveIntroInputs<'w> {
    active: Res<'w, ActiveNewGameInputs>,
    manifest_load: Res<'w, ActiveManifestLoad>,
    manifests: Res<'w, Assets<Manifest>>,
    parties: Res<'w, Assets<PartyCatalog>>,
    balances: Res<'w, Assets<BalanceData>>,
    intros: Res<'w, Assets<CutsceneDialogue>>,
    asset_server: Res<'w, AssetServer>,
}

fn setup_intro_dialogue(
    mut commands: Commands,
    theme: Res<UiTheme>,
    mut progress: ResMut<IntroDialogueProgress>,
    mut view: ResMut<IntroDialogueViewState>,
) {
    *progress = IntroDialogueProgress::default();
    *view = IntroDialogueViewState::Loading;

    commands.spawn((
        fixed_gameplay_camera(),
        Camera {
            clear_color: ClearColorConfig::Custom(theme.clear_color),
            ..default()
        },
        IntroDialogueCamera,
        IntroDialogueEntity,
    ));

    let root = commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            IntroDialogueRoot,
            IntroDialogueEntity,
        ))
        .id();
    spawn_status(&mut commands, root, &theme, LOADING_MESSAGE, None);
}

fn sync_intro_dialogue_view(
    mut commands: Commands,
    inputs: ActiveIntroInputs,
    theme: Res<UiTheme>,
    root: Single<Entity, With<IntroDialogueRoot>>,
    mut progress: ResMut<IntroDialogueProgress>,
    mut view: ResMut<IntroDialogueViewState>,
) {
    let selected = inputs.active.inputs(
        &inputs.manifest_load,
        &inputs.manifests,
        &inputs.parties,
        &inputs.balances,
        &inputs.intros,
    );

    if let Some(selected) = selected {
        if *view != IntroDialogueViewState::Ready {
            *progress = IntroDialogueProgress::default();
            let font = inputs.asset_server.load(
                inputs
                    .manifest_load
                    .root()
                    .resolve(&selected.manifest.font.path),
            );
            let line = selected.intro.lines.first().map_or("", String::as_str);
            replace_with_dialogue(
                &mut commands,
                *root,
                &theme,
                &font,
                line,
                progress.line_index,
                selected.intro.lines.len(),
            );
            *view = IntroDialogueViewState::Ready;
        }
        return;
    }

    let (next_view, message) = match inputs.active.status() {
        ActiveNewGameInputsStatus::Failed => (
            IntroDialogueViewState::Failed,
            inputs
                .active
                .failure()
                .map(ToString::to_string)
                .unwrap_or_else(|| selected_intro_unavailable_message(&inputs)),
        ),
        ActiveNewGameInputsStatus::Loading if *view == IntroDialogueViewState::Ready => (
            IntroDialogueViewState::Failed,
            selected_intro_unavailable_message(&inputs),
        ),
        ActiveNewGameInputsStatus::Ready => (
            IntroDialogueViewState::Failed,
            selected_intro_unavailable_message(&inputs),
        ),
        ActiveNewGameInputsStatus::Loading => {
            (IntroDialogueViewState::Loading, LOADING_MESSAGE.to_owned())
        }
    };

    if *view != next_view {
        *progress = IntroDialogueProgress::default();
        commands.entity(*root).despawn_children();
        spawn_status(
            &mut commands,
            *root,
            &theme,
            &message,
            manifest_font(&inputs),
        );
        *view = next_view;
    }
}

fn advance_intro_dialogue(
    actions: Res<ActionState>,
    inputs: ActiveIntroInputs,
    view: Res<IntroDialogueViewState>,
    mut progress: ResMut<IntroDialogueProgress>,
    mut lines: Query<&mut Text, (With<IntroDialogueLine>, Without<IntroDialogueHint>)>,
    mut hints: Query<&mut Text, (With<IntroDialogueHint>, Without<IntroDialogueLine>)>,
    mut completions: MessageWriter<IntroDialogueCompleted>,
) {
    if *view != IntroDialogueViewState::Ready || !actions.just_pressed(AppAction::Confirm) {
        return;
    }
    let Some(selected) = inputs.active.inputs(
        &inputs.manifest_load,
        &inputs.manifests,
        &inputs.parties,
        &inputs.balances,
        &inputs.intros,
    ) else {
        return;
    };
    let (Ok(mut line), Ok(mut hint)) = (lines.single_mut(), hints.single_mut()) else {
        return;
    };

    match progress.confirm(selected.intro.lines.len()) {
        ProgressEffect::AdvancedTo(index) => {
            line.0 = selected.intro.lines[index].clone();
            hint.0 = continuation_hint(index, selected.intro.lines.len(), false);
        }
        ProgressEffect::Completed => {
            hint.0 = continuation_hint(progress.line_index, selected.intro.lines.len(), true);
            completions.write(IntroDialogueCompleted {
                on_complete: selected.intro.on_complete.clone(),
            });
        }
        ProgressEffect::None => {}
    }
}

fn replace_with_dialogue(
    commands: &mut Commands,
    root: Entity,
    theme: &UiTheme,
    font: &Handle<Font>,
    line: &str,
    line_index: usize,
    line_count: usize,
) {
    commands.entity(root).despawn_children();
    commands.entity(root).with_children(|root| {
        root.spawn((
            Node {
                width: px(INTRO_PANEL_WIDTH),
                max_width: px(INTRO_PANEL_WIDTH),
                height: px(INTRO_PANEL_HEIGHT),
                max_height: px(INTRO_PANEL_HEIGHT),
                padding: UiRect::all(px(22)),
                border: UiRect::all(px(2)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(theme.name_entry_box_color),
            BorderColor::all(theme.name_entry_border_color),
            IntroDialoguePanel,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new(line),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(theme.name_entry_hint_font_size),
                    ..default()
                },
                TextColor(theme.name_entry_prompt_color),
                TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
                Node {
                    width: px(INTRO_TEXT_WIDTH),
                    max_width: px(INTRO_TEXT_WIDTH),
                    height: px(INTRO_TEXT_HEIGHT),
                    max_height: px(INTRO_TEXT_HEIGHT),
                    overflow: Overflow::clip(),
                    ..default()
                },
                IntroDialogueLine,
            ));
            panel.spawn((
                Text::new(continuation_hint(line_index, line_count, false)),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(theme.status_font_size),
                    ..default()
                },
                TextColor(theme.name_entry_hint_color),
                TextLayout::justify(Justify::Right),
                Node {
                    position_type: PositionType::Absolute,
                    right: px(18),
                    bottom: px(10),
                    max_width: px(INTRO_TEXT_WIDTH),
                    ..default()
                },
                IntroDialogueHint,
            ));
        });
    });
}

fn spawn_status(
    commands: &mut Commands,
    root: Entity,
    theme: &UiTheme,
    message: &str,
    font: Option<Handle<Font>>,
) {
    commands.entity(root).with_children(|root| {
        let mut status = root.spawn((
            Text::new(message),
            TextColor(theme.name_entry_hint_color),
            TextLayout::new(Justify::Center, LineBreak::WordOrCharacter),
            Node {
                width: px(INTRO_PANEL_WIDTH),
                max_width: px(INTRO_PANEL_WIDTH),
                min_height: px(80),
                max_height: px(INTRO_PANEL_HEIGHT),
                padding: UiRect::all(px(18)),
                border: UiRect::all(px(2)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(theme.name_entry_box_color),
            BorderColor::all(theme.name_entry_border_color),
            IntroDialogueStatus,
        ));
        if let Some(font) = font {
            status.insert(TextFont {
                font: font.into(),
                font_size: FontSize::Px(theme.status_font_size),
                ..default()
            });
        }
    });
}

fn continuation_hint(line_index: usize, line_count: usize, completed: bool) -> String {
    if completed {
        "Introduction complete".to_owned()
    } else {
        format!(
            "ENTER / SPACE  ▶  {}/{}",
            line_index.saturating_add(1).min(line_count),
            line_count
        )
    }
}

fn manifest_font(inputs: &ActiveIntroInputs) -> Option<Handle<Font>> {
    inputs
        .manifest_load
        .manifest(&inputs.manifests)
        .map(|manifest| {
            inputs
                .asset_server
                .load(inputs.manifest_load.root().resolve(&manifest.font.path))
        })
}

fn selected_intro_unavailable_message(inputs: &ActiveIntroInputs) -> String {
    let root = inputs.manifest_load.root();
    let path = inputs
        .manifest_load
        .manifest(&inputs.manifests)
        .map(|manifest| manifest.start.intro_dialogue.as_str())
        .unwrap_or("manifest.yaml");
    format!(
        "{}:{}: active intro dialogue is unavailable",
        root.package_key(),
        path
    )
}

fn cleanup_intro_dialogue(
    mut commands: Commands,
    entities: Query<Entity, With<IntroDialogueEntity>>,
    mut progress: ResMut<IntroDialogueProgress>,
    mut view: ResMut<IntroDialogueViewState>,
) {
    for entity in &entities {
        commands.entity(entity).despawn();
    }
    *progress = IntroDialogueProgress::default();
    *view = IntroDialogueViewState::Loading;
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    use bevy::{camera::ScalingMode, text::FontSource};

    use super::*;
    use crate::{
        app_state::AppStateTransitionRequest,
        game_state::GameState,
        gameplay_canvas::{GameplayCanvasCamera, LOGICAL_CANVAS_HEIGHT, LOGICAL_CANVAS_WIDTH},
        name_entry::{NameEntryConfirmed, NameEntryViewState},
        scenario_new_game_assets::ActiveNewGameInputsStatus,
        scenario_root::ScenarioRoot,
        test_support::headless_title_app_with_asset_base,
    };

    static NEXT_TEMPORARY_ID: AtomicU64 = AtomicU64::new(0);

    const INTRO_PATH: &str = "data/dialogue/invented-opening.yaml";
    const INTRO: &str = r#"id: invented_opening
type: cutscene
lines:
  - "The invented lantern woke."
  - "unbrokeninventedwordthatmustfallbacktocharacterwrappingwithoutescapingthepanel"
  - "The invented road waited."
on_complete:
  set_flag: invented_intro_completed_later
  transition:
    map: invented_destination
    position: [8, 9]
    fade: in
"#;

    struct InventedPackage(PathBuf);

    impl InventedPackage {
        fn new(intro: Option<&str>) -> Self {
            let unique = NEXT_TEMPORARY_ID.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "rpg-s1-intro-dialogue-{}-{unique}",
                std::process::id()
            ));
            let package = root.join("scenarios/invented_campaign");
            fs::create_dir_all(package.join("data/dialogue")).expect("invented dialogue directory");
            let manifest = include_str!("../tests/fixtures/rusted-kingdoms-manifest-complete.yaml")
                .replacen("data/dialogue/intro_cutscene.yaml", INTRO_PATH, 1);
            fs::write(package.join("manifest.yaml"), manifest).expect("invented manifest");
            let party = include_str!("../tests/fixtures/party-catalog-shapes.yaml")
                .replacen("id: ember", "id: aric", 1)
                .replacen("class: vanguard", "class: hero", 1);
            fs::write(package.join("data/party.yaml"), party).expect("invented party");
            fs::write(
                package.join("data/balance.yaml"),
                include_str!("../tests/fixtures/balance-complete.yaml"),
            )
            .expect("invented balance");
            if let Some(intro) = intro {
                fs::write(package.join(INTRO_PATH), intro).expect("invented intro");
            }
            Self(root)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for InventedPackage {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("invented package should be removed");
        }
    }

    fn app_for(package: &InventedPackage, initial_state: AppState) -> App {
        headless_title_app_with_asset_base(
            initial_state,
            package.path().to_string_lossy().into_owned(),
            ScenarioRoot::try_for_package_key("invented_campaign")
                .expect("invented key should be valid"),
        )
    }

    fn install_and_enter_dialogue(package: &InventedPackage) -> App {
        let mut app = app_for(package, AppState::NameEntry);
        for _ in 0..1_000 {
            app.update();
            if *app.world().resource::<NameEntryViewState>() == NameEntryViewState::Ready
                && app.world().resource::<ActiveNewGameInputs>().status()
                    == ActiveNewGameInputsStatus::Ready
            {
                break;
            }
            std::thread::yield_now();
        }
        assert_eq!(
            app.world().resource::<ActiveNewGameInputs>().status(),
            ActiveNewGameInputsStatus::Ready
        );
        app.world_mut()
            .write_message(NameEntryConfirmed::for_test("Invented Hero"));
        for _ in 0..1_000 {
            app.update();
            if app.world().resource::<State<AppState>>().get() == &AppState::Dialogue
                && *app.world().resource::<IntroDialogueViewState>()
                    == IntroDialogueViewState::Ready
            {
                return app;
            }
            std::thread::yield_now();
        }
        panic!("installed game did not enter a ready intro dialogue");
    }

    fn marked_text<M: Component>(world: &mut World) -> String {
        world
            .query_filtered::<&Text, With<M>>()
            .single(world)
            .expect("marked text should exist once")
            .0
            .clone()
    }

    fn press(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
    }

    fn release(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(key);
    }

    fn completions(app: &mut App) -> Vec<IntroDialogueCompleted> {
        app.world_mut()
            .resource_mut::<Messages<IntroDialogueCompleted>>()
            .drain()
            .collect()
    }

    #[test]
    fn empty_progression_completes_once_without_indexing_a_line() {
        let mut progress = IntroDialogueProgress::default();

        assert_eq!(progress.confirm(0), ProgressEffect::Completed);
        assert_eq!(progress.line_index, 0);
        assert!(progress.completed);
        assert_eq!(progress.confirm(0), ProgressEffect::None);
    }

    #[test]
    fn progression_visits_each_middle_line_then_completes_idempotently() {
        let mut progress = IntroDialogueProgress::default();

        assert_eq!(progress.line_index, 0);
        assert_eq!(progress.confirm(3), ProgressEffect::AdvancedTo(1));
        assert_eq!(progress.line_index, 1);
        assert_eq!(progress.confirm(3), ProgressEffect::AdvancedTo(2));
        assert_eq!(progress.line_index, 2);
        assert_eq!(progress.confirm(3), ProgressEffect::Completed);
        assert!(progress.completed);
        assert_eq!(progress.confirm(3), ProgressEffect::None);
        assert_eq!(progress.line_index, 2);
    }

    #[test]
    fn installed_manifest_intro_uses_one_fixed_canvas_tree_selected_font_and_bounded_wrapping() {
        let package = InventedPackage::new(Some(INTRO));
        let mut app = install_and_enter_dialogue(&package);
        let world = app.world_mut();

        assert_eq!(
            marked_text::<IntroDialogueLine>(world),
            "The invented lantern woke."
        );
        assert_eq!(
            world
                .query_filtered::<Entity, With<IntroDialogueCamera>>()
                .iter(world)
                .count(),
            1
        );
        assert_eq!(
            world
                .query_filtered::<Entity, With<IntroDialogueRoot>>()
                .iter(world)
                .count(),
            1
        );
        assert_eq!(
            world
                .query_filtered::<Entity, With<IntroDialoguePanel>>()
                .iter(world)
                .count(),
            1
        );

        let projection = world
            .query_filtered::<&Projection, With<GameplayCanvasCamera>>()
            .single(world)
            .expect("one intro gameplay camera");
        let Projection::Orthographic(projection) = projection else {
            panic!("intro camera should be orthographic");
        };
        let ScalingMode::Fixed { width, height } = projection.scaling_mode else {
            panic!("intro camera should use the fixed logical canvas");
        };
        assert_eq!(
            (width, height),
            (LOGICAL_CANVAS_WIDTH as f32, LOGICAL_CANVAS_HEIGHT as f32,)
        );

        let root = world
            .query_filtered::<&Node, With<IntroDialogueRoot>>()
            .single(world)
            .unwrap();
        assert_eq!(root.width, percent(100));
        assert_eq!(root.height, percent(100));
        assert_eq!(root.justify_content, JustifyContent::Center);
        assert_eq!(root.align_items, AlignItems::Center);

        let panel = world
            .query_filtered::<&Node, With<IntroDialoguePanel>>()
            .single(world)
            .unwrap();
        assert_eq!(panel.width, px(INTRO_PANEL_WIDTH));
        assert_eq!(panel.max_width, px(INTRO_PANEL_WIDTH));
        assert_eq!(panel.height, px(INTRO_PANEL_HEIGHT));
        assert_eq!(panel.max_height, px(INTRO_PANEL_HEIGHT));
        assert!(INTRO_PANEL_WIDTH < LOGICAL_CANVAS_WIDTH as f32);
        assert!(INTRO_PANEL_HEIGHT < LOGICAL_CANVAS_HEIGHT as f32);

        let (layout, node, font) = world
            .query_filtered::<(&TextLayout, &Node, &TextFont), With<IntroDialogueLine>>()
            .single(world)
            .unwrap();
        assert_eq!(layout.linebreak, LineBreak::WordOrCharacter);
        assert_eq!(node.width, px(INTRO_TEXT_WIDTH));
        assert_eq!(node.max_width, px(INTRO_TEXT_WIDTH));
        assert_eq!(node.height, px(INTRO_TEXT_HEIGHT));
        assert_eq!(node.max_height, px(INTRO_TEXT_HEIGHT));
        assert_eq!(node.overflow, Overflow::clip());
        let FontSource::Handle(font) = &font.font else {
            panic!("intro line should use the manifest font handle");
        };
        let font_path = world
            .resource::<AssetServer>()
            .get_path(font.id())
            .expect("intro font should retain its AssetServer path")
            .path()
            .to_string_lossy()
            .into_owned();
        assert_eq!(
            font_path,
            "scenarios/invented_campaign/assets/fonts/Philosopher-Regular.ttf"
        );

        let hint = world
            .query_filtered::<&Node, With<IntroDialogueHint>>()
            .single(world)
            .unwrap();
        assert_eq!(hint.position_type, PositionType::Absolute);
        assert_eq!(hint.right, px(18));
        assert_eq!(hint.bottom, px(10));
    }

    #[test]
    fn fresh_confirm_during_loading_cannot_skip_the_first_ready_line() {
        let package = InventedPackage::new(Some(INTRO));
        let mut app = install_and_enter_dialogue(&package);
        *app.world_mut().resource_mut::<IntroDialogueViewState>() = IntroDialogueViewState::Loading;
        *app.world_mut().resource_mut::<IntroDialogueProgress>() = IntroDialogueProgress::default();
        press(&mut app, KeyCode::Enter);

        app.update();

        assert_eq!(
            app.world().resource::<IntroDialogueViewState>(),
            &IntroDialogueViewState::Ready
        );
        assert_eq!(
            app.world().resource::<IntroDialogueProgress>(),
            &IntroDialogueProgress::default()
        );
        assert_eq!(
            marked_text::<IntroDialogueLine>(app.world_mut()),
            "The invented lantern woke."
        );
        assert!(completions(&mut app).is_empty());

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear_just_pressed(KeyCode::Enter);
        app.update();
        assert_eq!(
            marked_text::<IntroDialogueLine>(app.world_mut()),
            "The invented lantern woke."
        );
        assert!(completions(&mut app).is_empty());
    }

    #[test]
    fn fresh_enter_space_and_keypad_advance_once_then_complete_without_mutating_game() {
        let package = InventedPackage::new(Some(INTRO));
        let mut app = install_and_enter_dialogue(&package);
        let (flags, map, party, repository, opened, controlled) = {
            let game = app.world().resource::<GameState>();
            (
                game.flags().clone(),
                game.map().clone(),
                game.party().clone(),
                game.repository().clone(),
                game.opened_boxes().clone(),
                game.controlled_member_id().to_owned(),
            )
        };

        press(&mut app, KeyCode::Enter);
        app.update();
        assert!(marked_text::<IntroDialogueLine>(app.world_mut()).starts_with("unbroken"));
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear_just_pressed(KeyCode::Enter);
        app.update();
        assert!(marked_text::<IntroDialogueLine>(app.world_mut()).starts_with("unbroken"));

        release(&mut app, KeyCode::Enter);
        app.update();
        press(&mut app, KeyCode::Space);
        app.update();
        assert_eq!(
            marked_text::<IntroDialogueLine>(app.world_mut()),
            "The invented road waited."
        );
        assert!(completions(&mut app).is_empty());

        release(&mut app, KeyCode::Space);
        app.update();
        press(&mut app, KeyCode::NumpadEnter);
        app.update();
        let completed = completions(&mut app);
        assert_eq!(completed.len(), 1);
        assert_eq!(
            completed[0]
                .on_complete()
                .set_flag
                .as_ref()
                .expect("invented completion flag")
                .as_slice(),
            ["invented_intro_completed_later"]
        );
        assert_eq!(
            marked_text::<IntroDialogueHint>(app.world_mut()),
            "Introduction complete"
        );

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear_just_pressed(KeyCode::NumpadEnter);
        app.update();
        release(&mut app, KeyCode::NumpadEnter);
        app.update();
        press(&mut app, KeyCode::NumpadEnter);
        app.update();
        assert!(completions(&mut app).is_empty());

        let game = app.world().resource::<GameState>();
        assert_eq!(game.flags(), &flags);
        assert_eq!(game.map(), &map);
        assert_eq!(game.party(), &party);
        assert_eq!(game.repository(), &repository);
        assert_eq!(game.opened_boxes(), &opened);
        assert_eq!(game.controlled_member_id(), controlled);
        assert!(!game.flags().is_set("invented_intro_completed_later"));
        assert_eq!(game.map().current().unwrap().as_str(), "town_01_ardel");
        assert_eq!(
            game.map().position(),
            crate::scenario_spatial::Position::new(14, 5)
        );
        assert_eq!(
            app.world().resource::<State<AppState>>().get(),
            &AppState::Dialogue
        );
    }

    #[test]
    fn loading_then_failure_replaces_status_with_one_package_relative_error() {
        let package = InventedPackage::new(None);
        let mut app = app_for(&package, AppState::Dialogue);
        app.update();
        assert_eq!(
            marked_text::<IntroDialogueStatus>(app.world_mut()),
            LOADING_MESSAGE
        );

        for _ in 0..1_000 {
            app.update();
            if app.world().resource::<ActiveNewGameInputs>().status()
                == ActiveNewGameInputsStatus::Failed
                && *app.world().resource::<IntroDialogueViewState>()
                    == IntroDialogueViewState::Failed
            {
                break;
            }
            std::thread::yield_now();
        }
        assert_eq!(
            app.world().resource::<IntroDialogueViewState>(),
            &IntroDialogueViewState::Failed
        );
        let failure = marked_text::<IntroDialogueStatus>(app.world_mut());
        assert!(failure.starts_with(&format!("invented_campaign:{INTRO_PATH}:")));
        assert!(!failure.contains(package.path().to_string_lossy().as_ref()));
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<IntroDialogueStatus>>()
                .iter(app.world())
                .count(),
            1
        );
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<IntroDialogueLine>>()
                .iter(app.world())
                .count(),
            0
        );
    }

    #[test]
    fn revoked_ready_intro_removes_stale_line_and_renders_safe_selected_path() {
        let package = InventedPackage::new(Some(INTRO));
        let mut app = install_and_enter_dialogue(&package);
        let intro_id = app
            .world()
            .resource::<Assets<CutsceneDialogue>>()
            .iter()
            .next()
            .expect("one invented intro")
            .0;
        app.world_mut()
            .resource_mut::<Assets<CutsceneDialogue>>()
            .remove(intro_id);

        app.update();

        assert_eq!(
            app.world().resource::<IntroDialogueViewState>(),
            &IntroDialogueViewState::Failed
        );
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<IntroDialogueLine>>()
                .iter(app.world())
                .count(),
            0
        );
        let failure = marked_text::<IntroDialogueStatus>(app.world_mut());
        assert_eq!(
            failure,
            format!("invented_campaign:{INTRO_PATH}: active intro dialogue is unavailable")
        );
        assert!(!failure.contains(package.path().to_string_lossy().as_ref()));
    }

    #[test]
    fn repeated_updates_do_not_respawn_and_exit_cleans_every_owned_entity() {
        let package = InventedPackage::new(Some(INTRO));
        let mut app = install_and_enter_dialogue(&package);
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<IntroDialogueCamera>>()
                .iter(app.world())
                .count(),
            1
        );
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<IntroDialogueRoot>>()
                .iter(app.world())
                .count(),
            1
        );

        app.world_mut()
            .write_message(AppStateTransitionRequest::new(AppState::Title));
        app.update();
        app.update();

        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<IntroDialogueEntity>>()
                .iter(app.world())
                .count(),
            0
        );
        assert_eq!(
            app.world().resource::<IntroDialogueProgress>(),
            &IntroDialogueProgress::default()
        );
        assert_eq!(
            app.world().resource::<IntroDialogueViewState>(),
            &IntroDialogueViewState::Loading
        );
    }
}
