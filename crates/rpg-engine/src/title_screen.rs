use bevy::{
    asset::LoadState,
    audio::{AudioSink, PlaybackMode, Volume},
    ecs::system::SystemParam,
    prelude::*,
};
use std::time::Duration;

use crate::{
    action_input::{ActionState, AppAction},
    app_state::{AppState, AppStateTransitionRequest},
    gameplay_canvas::fixed_gameplay_camera,
    save_ui::{SaveSlotCatalog, TitleLoadMenu},
    scenario_audio::{BGM_INDEX_PATH, BgmIndex, SFX_INDEX_PATH, SfxIndex},
    scenario_manifest::Manifest,
    scenario_manifest_asset::{ActiveManifestLoad, ActiveManifestStatus},
    scenario_path::ScenarioRelativePath,
    scenario_root::ScenarioRoot,
    ui_theme::UiTheme,
    world_audio::LogicalBgmPlayer,
};

const MENU_LABELS: [&str; 4] = ["New Game", "Load Game", "Options", "Quit"];
const LOAD_GAME_INDEX: usize = 1;
const OPTIONS_INDEX: usize = 2;
const QUIT_INDEX: usize = 3;
/// Row height for one title menu entry.
const MENU_ENTRY_HEIGHT: f32 = 42.0;
/// Bounded wait for the Quit confirmation to reach the audio device.
///
/// The chime is started, not awaited: shutdown must feel as immediate as the field menu's Quit,
/// which writes [`AppExit`] on the spot. This deadline only covers a device that never hands back
/// a sink, so a missing audio backend cannot stall the exit.
const QUIT_START_TIMEOUT: Duration = Duration::from_millis(400);

pub struct TitleScreenPlugin;

impl Plugin for TitleScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TitleMenu>()
            .init_resource::<crate::options_ui::OptionsReturn>()
            .init_resource::<QuitLifecycle>()
            .init_resource::<TitlePresentation>()
            .init_resource::<UiTheme>()
            .add_systems(OnEnter(AppState::Title), setup_title_screen)
            .add_systems(OnExit(AppState::Title), cleanup_title_screen)
            .add_systems(
                Update,
                (
                    apply_title_presentation,
                    handle_menu_input,
                    observe_quit_playback,
                    update_menu_colors,
                )
                    .chain()
                    .run_if(in_state(AppState::Title)),
            );
    }
}

#[derive(Resource, Default)]
struct TitleMenu {
    selected: usize,
}

impl TitleMenu {
    fn move_by(&mut self, delta: isize) {
        self.selected =
            (self.selected as isize + delta).rem_euclid(MENU_LABELS.len() as isize) as usize;
    }
}

#[derive(Component)]
struct MenuEntry(usize);

/// Title text whose face is published from the manifest once it loads.
///
/// [`TitleScreenEntity`] marks only the root node, so it cannot select the descendant text that
/// needs the scenario font.
#[derive(Component)]
struct TitleText;

#[derive(Component)]
struct StatusMessage;

#[derive(Component)]
struct TitlePanel;

#[derive(Component)]
struct QuitConfirmSound;

#[derive(SystemParam)]
struct TitleMenuInputOutput<'w, 's> {
    status: Single<'w, 's, &'static mut Text, With<StatusMessage>>,
    options_return: ResMut<'w, crate::options_ui::OptionsReturn>,
    asset_server: Res<'w, AssetServer>,
    transitions: MessageWriter<'w, AppStateTransitionRequest>,
    commands: Commands<'w, 's>,
}

/// Marks every independently spawned part of the title screen.
///
/// UI descendants belong to the marked root node, while cameras, sprites, and audio players are
/// marked directly. This keeps the title state self-contained across state transitions.
#[derive(Component)]
struct TitleScreenEntity;

#[derive(Debug, Default, Eq, PartialEq)]
enum QuitLifecycleState {
    #[default]
    Idle,
    WaitingForStart {
        accepted_at: Duration,
    },
    ExitSent,
}

#[derive(Resource, Debug, Default, Eq, PartialEq)]
struct QuitLifecycle {
    state: QuitLifecycleState,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum TitlePresentationStatus {
    #[default]
    Loading,
    Ready,
    Failed,
}

#[derive(Debug, Default, Resource)]
pub(crate) struct TitlePresentation {
    status: TitlePresentationStatus,
    bgm_index: Option<Handle<BgmIndex>>,
    sfx_index: Option<Handle<SfxIndex>>,
    hover_sfx: Option<String>,
    confirm_sfx: Option<String>,
}

impl TitlePresentation {
    /// Test-only since the menu stopped gating a cursor image on it: nothing the title draws now
    /// has to wait for the manifest, because colour needs no asset.
    #[cfg(test)]
    pub(crate) const fn is_ready(&self) -> bool {
        matches!(self.status, TitlePresentationStatus::Ready)
    }
}

impl QuitLifecycle {
    fn accepts_input(&self) -> bool {
        self.state == QuitLifecycleState::Idle
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum QuitPlaybackObservation {
    AwaitingStart,
    Started,
    AssetLoadFailed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum QuitLifecycleEvent {
    Activate,
    Observe(QuitPlaybackObservation),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum QuitLifecycleEffect {
    SpawnConfirm,
    EmitExit,
}

/// Advances the Quit lifecycle, emitting exactly one exit per activation.
///
/// Exit is emitted as soon as the confirmation reaches the device, rather than after the chime
/// finishes. Waiting out the full sample cost the shutdown well over a second, which read as a
/// hang next to the field menu's instant Quit.
fn reduce_quit_lifecycle(
    lifecycle: &mut QuitLifecycle,
    elapsed: Duration,
    event: QuitLifecycleEvent,
) -> Option<QuitLifecycleEffect> {
    match event {
        QuitLifecycleEvent::Activate => {
            if lifecycle.state != QuitLifecycleState::Idle {
                return None;
            }

            lifecycle.state = QuitLifecycleState::WaitingForStart {
                accepted_at: elapsed,
            };
            Some(QuitLifecycleEffect::SpawnConfirm)
        }
        QuitLifecycleEvent::Observe(observation) => match lifecycle.state {
            QuitLifecycleState::WaitingForStart { accepted_at } => {
                if matches!(
                    observation,
                    QuitPlaybackObservation::Started | QuitPlaybackObservation::AssetLoadFailed
                ) || elapsed.saturating_sub(accepted_at) >= QUIT_START_TIMEOUT
                {
                    lifecycle.state = QuitLifecycleState::ExitSent;
                    Some(QuitLifecycleEffect::EmitExit)
                } else {
                    None
                }
            }
            QuitLifecycleState::Idle | QuitLifecycleState::ExitSent => None,
        },
    }
}

#[derive(Debug, Eq, PartialEq)]
enum TitleMenuAction {
    NewGame,
    LoadGame,
    LoadGameDisabled,
    Options,
    Quit,
}

fn title_menu_action(selected: usize, has_valid_save: bool) -> TitleMenuAction {
    match selected {
        0 => TitleMenuAction::NewGame,
        LOAD_GAME_INDEX if has_valid_save => TitleMenuAction::LoadGame,
        LOAD_GAME_INDEX => TitleMenuAction::LoadGameDisabled,
        OPTIONS_INDEX => TitleMenuAction::Options,
        QUIT_INDEX => TitleMenuAction::Quit,
        _ => unreachable!("menu selection must refer to a title menu entry"),
    }
}

fn setup_title_screen(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    scenario_root: Res<ScenarioRoot>,
    theme: Res<UiTheme>,
    mut presentation: ResMut<TitlePresentation>,
) {
    let bgm_path = ScenarioRelativePath::try_from(BGM_INDEX_PATH)
        .expect("the BGM index path is scenario-relative");
    let sfx_path = ScenarioRelativePath::try_from(SFX_INDEX_PATH)
        .expect("the SFX index path is scenario-relative");
    *presentation = TitlePresentation {
        status: TitlePresentationStatus::Loading,
        bgm_index: Some(asset_server.load(scenario_root.resolve(&bgm_path))),
        sfx_index: Some(asset_server.load(scenario_root.resolve(&sfx_path))),
        hover_sfx: None,
        confirm_sfx: None,
    };

    commands.spawn((fixed_gameplay_camera(), TitleScreenEntity));

    commands
        .spawn(Node {
            width: percent(100),
            height: percent(100),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::End,
            padding: UiRect::bottom(px(32)),
            ..default()
        })
        .insert(TitleScreenEntity)
        .with_children(|root| {
            root.spawn((
                Node {
                    width: px(300),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(px(30), px(20)),
                    border_radius: BorderRadius::all(px(12)),
                    ..default()
                },
                BackgroundColor(theme.panel_color),
                TitlePanel,
            ))
            .with_children(|panel| {
                for (index, label) in MENU_LABELS.into_iter().enumerate() {
                    panel
                        .spawn(Node {
                            width: percent(100),
                            height: px(MENU_ENTRY_HEIGHT),
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        })
                        .with_children(|row| {
                            row.spawn((
                                Text::new(label),
                                TextFont {
                                    font: Handle::<Font>::default().into(),
                                    font_size: FontSize::Px(theme.menu_font_size),
                                    ..default()
                                },
                                TextColor(menu_entry_color(&theme, index, 0, false)),
                                TextLayout::justify(Justify::Center),
                                MenuEntry(index),
                                TitleText,
                            ));
                        });
                }
            });

            root.spawn((
                Text::new(""),
                TextFont {
                    font: Handle::<Font>::default().into(),
                    font_size: FontSize::Px(theme.status_font_size),
                    ..default()
                },
                TextColor(theme.status_color),
                TextLayout::justify(Justify::Center),
                Node {
                    position_type: PositionType::Absolute,
                    bottom: px(8),
                    left: px(0),
                    right: px(0),
                    ..default()
                },
                StatusMessage,
                TitleText,
            ));
        });
}

#[expect(
    clippy::too_many_arguments,
    reason = "title publication joins the manifest and both typed audio indexes transactionally"
)]
fn apply_title_presentation(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    scenario_root: Res<ScenarioRoot>,
    active_manifest: Res<ActiveManifestLoad>,
    manifests: Res<Assets<Manifest>>,
    bgm_indexes: Res<Assets<BgmIndex>>,
    sfx_indexes: Res<Assets<SfxIndex>>,
    mut presentation: ResMut<TitlePresentation>,
    mut fonts: Query<&mut TextFont, With<TitleText>>,
    mut status: Single<&mut Text, With<StatusMessage>>,
) {
    if presentation.status != TitlePresentationStatus::Loading {
        return;
    }
    if active_manifest.status() == ActiveManifestStatus::Failed {
        presentation.status = TitlePresentationStatus::Failed;
        status.0 = "Scenario manifest failed to load.".to_owned();
        return;
    }
    let Some(manifest) = active_manifest.manifest(&manifests) else {
        return;
    };
    let Some(bgm_handle) = presentation.bgm_index.as_ref() else {
        return;
    };
    let Some(sfx_handle) = presentation.sfx_index.as_ref() else {
        return;
    };
    if matches!(
        asset_server.load_state(bgm_handle.id()),
        LoadState::Failed(_)
    ) || matches!(
        asset_server.load_state(sfx_handle.id()),
        LoadState::Failed(_)
    ) {
        presentation.status = TitlePresentationStatus::Failed;
        status.0 = "Scenario audio index failed to load.".to_owned();
        return;
    }
    let (Some(bgm_index), Some(sfx_index)) =
        (bgm_indexes.get(bgm_handle), sfx_indexes.get(sfx_handle))
    else {
        return;
    };
    let Some(title_bgm) = bgm_index.resolve_key(&scenario_root, "title.default") else {
        presentation.status = TitlePresentationStatus::Failed;
        status.0 = "Scenario audio has no title.default track.".to_owned();
        return;
    };
    let (Some(hover_sfx), Some(confirm_sfx)) = (
        sfx_index.resolve_key(&scenario_root, "hover"),
        sfx_index.resolve_key(&scenario_root, "confirm"),
    ) else {
        presentation.status = TitlePresentationStatus::Failed;
        status.0 = "Scenario audio has no hover or confirm event.".to_owned();
        return;
    };

    let font = asset_server.load(scenario_root.resolve(&manifest.font.path));
    for mut text_font in &mut fonts {
        text_font.font = font.clone().into();
    }
    commands.spawn((
        Sprite::from_image(asset_server.load(scenario_root.resolve(&manifest.title.image))),
        TitleScreenEntity,
    ));
    commands.spawn((
        AudioPlayer::new(asset_server.load(title_bgm)),
        PlaybackSettings {
            mode: PlaybackMode::Loop,
            volume: Volume::Linear(0.65),
            ..default()
        },
        LogicalBgmPlayer,
        TitleScreenEntity,
    ));
    presentation.hover_sfx = Some(hover_sfx);
    presentation.confirm_sfx = Some(confirm_sfx);
    presentation.status = TitlePresentationStatus::Ready;
    status.0.clear();
}

fn cleanup_title_screen(
    mut commands: Commands,
    title_entities: Query<Entity, With<TitleScreenEntity>>,
    mut menu: ResMut<TitleMenu>,
    mut quit: ResMut<QuitLifecycle>,
    mut presentation: ResMut<TitlePresentation>,
) {
    for entity in &title_entities {
        commands.entity(entity).despawn();
    }

    *menu = TitleMenu::default();
    *quit = QuitLifecycle::default();
    *presentation = TitlePresentation::default();
}

#[expect(
    clippy::too_many_arguments,
    reason = "Bevy title input keeps lifecycle resources and output system parameters explicit"
)]
fn handle_menu_input(
    actions: Res<ActionState>,
    mut menu: ResMut<TitleMenu>,
    mut quit: ResMut<QuitLifecycle>,
    time: Res<Time>,
    catalog: Res<SaveSlotCatalog>,
    mut load_menu: ResMut<TitleLoadMenu>,
    mut output: TitleMenuInputOutput,
    presentation: Res<TitlePresentation>,
) {
    if !quit.accepts_input() || load_menu.open {
        return;
    }

    // The action map resolves Up+Down to one deterministic movement before this screen handles
    // a simultaneous Confirm against the resulting selection.
    let direction = actions.menu_navigation();

    if let Some(delta) = direction {
        menu.move_by(delta);
        if let Some(path) = presentation.hover_sfx.as_ref() {
            output.commands.spawn((
                AudioPlayer::new(output.asset_server.load(path)),
                PlaybackSettings::DESPAWN,
                TitleScreenEntity,
            ));
        }
        output.status.0.clear();
    }

    if !actions.just_pressed(AppAction::Confirm) {
        return;
    }

    match title_menu_action(menu.selected, catalog.has_valid()) {
        TitleMenuAction::NewGame => {
            if let Some(path) = presentation.confirm_sfx.as_ref() {
                output.commands.spawn((
                    AudioPlayer::new(output.asset_server.load(path)),
                    PlaybackSettings::DESPAWN,
                    TitleScreenEntity,
                ));
            }
            output.status.0.clear();
            output
                .transitions
                .write(AppStateTransitionRequest::new(AppState::NameEntry));
        }
        TitleMenuAction::LoadGame => {
            if let Some(path) = presentation.confirm_sfx.as_ref() {
                output.commands.spawn((
                    AudioPlayer::new(output.asset_server.load(path)),
                    PlaybackSettings::DESPAWN,
                    TitleScreenEntity,
                ));
            }
            output.status.0.clear();
            load_menu.open(catalog.slots());
        }
        TitleMenuAction::LoadGameDisabled => {}
        TitleMenuAction::Options => {
            if let Some(path) = presentation.confirm_sfx.as_ref() {
                output.commands.spawn((
                    AudioPlayer::new(output.asset_server.load(path)),
                    PlaybackSettings::DESPAWN,
                    TitleScreenEntity,
                ));
            }
            output.status.0.clear();
            output.options_return.0 = AppState::Title;
            output
                .transitions
                .write(AppStateTransitionRequest::new(AppState::Options));
        }
        TitleMenuAction::Quit => {
            if reduce_quit_lifecycle(&mut quit, time.elapsed(), QuitLifecycleEvent::Activate)
                == Some(QuitLifecycleEffect::SpawnConfirm)
                && let Some(path) = presentation.confirm_sfx.as_ref()
            {
                output.commands.spawn((
                    AudioPlayer::new(output.asset_server.load(path)),
                    PlaybackSettings::DESPAWN,
                    QuitConfirmSound,
                    TitleScreenEntity,
                ));
            }
        }
    }
}

fn observe_quit_playback(
    mut quit: ResMut<QuitLifecycle>,
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    marked_audio: Query<(&AudioPlayer<AudioSource>, Option<&AudioSink>), With<QuitConfirmSound>>,
    mut exit: MessageWriter<AppExit>,
) {
    if quit.accepts_input() || quit.state == QuitLifecycleState::ExitSent {
        return;
    }

    let mut marked_audio = marked_audio.iter();
    let observation = if let Some((player, sink)) = marked_audio.next() {
        debug_assert!(
            marked_audio.next().is_none(),
            "the Quit lifecycle must own exactly one marked sound"
        );

        if sink.is_some() {
            QuitPlaybackObservation::Started
        } else if matches!(
            asset_server.get_load_state(player.0.id()),
            Some(LoadState::Failed(_))
        ) {
            QuitPlaybackObservation::AssetLoadFailed
        } else {
            QuitPlaybackObservation::AwaitingStart
        }
    } else {
        QuitPlaybackObservation::AwaitingStart
    };

    if reduce_quit_lifecycle(
        &mut quit,
        time.elapsed(),
        QuitLifecycleEvent::Observe(observation),
    ) == Some(QuitLifecycleEffect::EmitExit)
    {
        exit.write(AppExit::Success);
    }
}

fn update_menu_colors(
    menu: Res<TitleMenu>,
    catalog: Res<SaveSlotCatalog>,
    theme: Res<UiTheme>,
    presentation: Res<TitlePresentation>,
    mut entries: Query<(&MenuEntry, &mut TextColor)>,
) {
    if !menu.is_changed() && !catalog.is_changed() && !presentation.is_changed() {
        return;
    }

    // Colour is the whole of the selection feedback; see [`menu_entry_color`].
    for (entry, mut color) in &mut entries {
        color.0 = menu_entry_color(&theme, entry.0, menu.selected, catalog.has_valid());
    }
}

/// Colour is the whole of the title menu's selection feedback.
///
/// The title menu centres its labels and uses the selected text color as its cursor.
fn menu_entry_color(theme: &UiTheme, index: usize, selected: usize, has_valid_save: bool) -> Color {
    if index == LOAD_GAME_INDEX && !has_valid_save {
        theme.menu_disabled_color
    } else if index == selected {
        theme.menu_selected_color
    } else {
        theme.menu_normal_color
    }
}

#[cfg(test)]
mod tests;
