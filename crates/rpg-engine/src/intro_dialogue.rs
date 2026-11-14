//! Manifest-selected introductory cutscene presentation and linear progression.
//!
//! This slice renders complete authored lines immediately. Typewriter timing is not applied here;
//! cancellation and completion hand off to the installed game flow.

use std::borrow::Cow;

use bevy::{ecs::system::SystemParam, prelude::*, text::LineBreak};

use crate::{
    action_input::{ActionState, AppAction},
    app_state::{AppState, AppStateTransitionRequest},
    game_state::GameState,
    gameplay_canvas::fixed_gameplay_camera,
    input_bindings::InputBindings,
    key_hints::{HintKey, HintStyle, key_hint},
    options_domain::Device,
    options_store::PlayerOptions,
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

/// The continuation hint: `ENTER / SPACE  ▶  1/3`.
const INTRO_HINT: HintStyle = HintStyle {
    key_gap: "  ▶  ",
    segment_gap: "",
    pair: "/",
    either: " / ",
    label_first: false,
};

pub(crate) struct IntroDialoguePlugin;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, SystemSet)]
pub(crate) enum IntroDialogueSet {
    Advance,
}

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
                cancel_intro_dialogue
                    .in_set(IntroDialogueSet::Advance)
                    .before(advance_intro_dialogue)
                    .run_if(in_state(AppState::Dialogue)),
            )
            .add_systems(
                Update,
                advance_intro_dialogue
                    .in_set(IntroDialogueSet::Advance)
                    .before(sync_intro_dialogue_view)
                    .run_if(in_state(AppState::Dialogue)),
            )
            .add_systems(OnExit(AppState::Dialogue), cleanup_intro_dialogue);
    }
}

/// Typed handoff consumed by the game-flow handoff without applying effects in this screen.
#[derive(Clone, Debug, Eq, Message, PartialEq)]
pub(crate) struct IntroDialogueCompleted {
    on_complete: DialogueActions,
}

impl IntroDialogueCompleted {
    #[cfg(test)]
    pub(crate) fn for_test(on_complete: DialogueActions) -> Self {
        Self { on_complete }
    }
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
    options: Option<Res<'w, PlayerOptions>>,
}

impl ActiveIntroInputs<'_> {
    /// The live binding table, or the defaults where no options resource exists.
    fn bindings(&self) -> Cow<'_, InputBindings> {
        match self.options.as_deref() {
            Some(options) => Cow::Borrowed(&options.bindings),
            None => Cow::Owned(InputBindings::default()),
        }
    }
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
                continuation_hint(
                    progress.line_index,
                    selected.intro.lines.len(),
                    false,
                    &inputs.bindings(),
                ),
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
    if *view != IntroDialogueViewState::Ready
        || actions.just_pressed(AppAction::Back)
        || !actions.just_pressed(AppAction::Confirm)
    {
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
            hint.0 =
                continuation_hint(index, selected.intro.lines.len(), false, &inputs.bindings());
        }
        ProgressEffect::Completed => {
            hint.0 = continuation_hint(
                progress.line_index,
                selected.intro.lines.len(),
                true,
                &inputs.bindings(),
            );
            completions.write(IntroDialogueCompleted {
                on_complete: selected.intro.on_complete.clone(),
            });
        }
        ProgressEffect::None => {}
    }
}

/// Back closes the overlay without dispatching authored
/// completion actions. It is ordered before progression and takes precedence over Confirm.
fn cancel_intro_dialogue(
    actions: Res<ActionState>,
    view: Res<IntroDialogueViewState>,
    game: Option<Res<GameState>>,
    mut transitions: MessageWriter<AppStateTransitionRequest>,
) {
    if *view == IntroDialogueViewState::Ready
        && actions.just_pressed(AppAction::Back)
        && game.is_some()
    {
        transitions.write(AppStateTransitionRequest::new(AppState::World));
    }
}

fn replace_with_dialogue(
    commands: &mut Commands,
    root: Entity,
    theme: &UiTheme,
    font: &Handle<Font>,
    line: &str,
    hint: String,
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
                Text::new(hint),
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

fn continuation_hint(
    line_index: usize,
    line_count: usize,
    completed: bool,
    bindings: &InputBindings,
) -> String {
    if completed {
        "Introduction complete".to_owned()
    } else {
        let position = format!(
            "{}/{}",
            line_index.saturating_add(1).min(line_count),
            line_count
        );
        key_hint(
            bindings,
            Device::Keyboard,
            &INTRO_HINT,
            &[(&[HintKey::Bindings(AppAction::Confirm, 2)], &position)],
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
mod tests;
