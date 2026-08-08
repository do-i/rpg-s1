use bevy::{
    asset::LoadState,
    audio::{AudioSink, PlaybackMode, Volume},
    prelude::*,
};
use std::time::Duration;

use crate::app_state::AppState;

const MENU_LABELS: [&str; 3] = ["New Game", "Load Game", "Quit"];
const LOAD_GAME_INDEX: usize = 1;
const NORMAL_COLOR: Color = Color::srgb_u8(170, 140, 100);
const SELECTED_COLOR: Color = Color::srgb_u8(220, 140, 60);
const DISABLED_COLOR: Color = Color::srgb_u8(80, 70, 55);
const QUIT_START_TIMEOUT: Duration = Duration::from_secs(3);
const QUIT_COMPLETION_TIMEOUT: Duration = Duration::from_secs(3);

pub struct TitleScreenPlugin;

impl Plugin for TitleScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TitleMenu>()
            .init_resource::<QuitLifecycle>()
            .add_systems(OnEnter(AppState::Title), setup_title_screen)
            .add_systems(OnExit(AppState::Title), cleanup_title_screen)
            .add_systems(
                Update,
                (handle_menu_input, observe_quit_playback, update_menu_colors)
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

#[derive(Component)]
struct StatusMessage;

#[derive(Component)]
struct QuitConfirmSound;

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
    WaitingForCompletion {
        started_at: Duration,
    },
    ExitSent,
}

#[derive(Resource, Debug, Default, Eq, PartialEq)]
struct QuitLifecycle {
    state: QuitLifecycleState,
}

impl QuitLifecycle {
    fn accepts_input(&self) -> bool {
        self.state == QuitLifecycleState::Idle
    }

    fn playback_started(&self) -> bool {
        matches!(self.state, QuitLifecycleState::WaitingForCompletion { .. })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum QuitPlaybackObservation {
    AwaitingStart,
    Started,
    CompletedAfterStart,
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
                if observation == QuitPlaybackObservation::AssetLoadFailed
                    || elapsed.saturating_sub(accepted_at) >= QUIT_START_TIMEOUT
                {
                    lifecycle.state = QuitLifecycleState::ExitSent;
                    Some(QuitLifecycleEffect::EmitExit)
                } else if observation == QuitPlaybackObservation::Started {
                    lifecycle.state = QuitLifecycleState::WaitingForCompletion {
                        started_at: elapsed,
                    };
                    None
                } else {
                    None
                }
            }
            QuitLifecycleState::WaitingForCompletion { started_at } => {
                if matches!(
                    observation,
                    QuitPlaybackObservation::CompletedAfterStart
                        | QuitPlaybackObservation::AssetLoadFailed
                ) || elapsed.saturating_sub(started_at) >= QUIT_COMPLETION_TIMEOUT
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
    LoadGameDisabled,
    Quit,
}

fn title_menu_action(selected: usize) -> TitleMenuAction {
    match selected {
        0 => TitleMenuAction::NewGame,
        LOAD_GAME_INDEX => TitleMenuAction::LoadGameDisabled,
        2 => TitleMenuAction::Quit,
        _ => unreachable!("menu selection must refer to a title menu entry"),
    }
}

fn setup_title_screen(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((Camera2d, TitleScreenEntity));

    commands.spawn((
        Sprite::from_image(asset_server.load("images/title_lost_flame.webp")),
        TitleScreenEntity,
    ));

    commands.spawn((
        AudioPlayer::new(asset_server.load("audio/title_theme.mp3")),
        PlaybackSettings {
            mode: PlaybackMode::Loop,
            volume: Volume::Linear(0.65),
            ..default()
        },
        TitleScreenEntity,
    ));

    let font = asset_server.load("fonts/Philosopher-Regular.ttf");

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
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.11)),
            ))
            .with_children(|panel| {
                for (index, label) in MENU_LABELS.into_iter().enumerate() {
                    panel.spawn((
                        Text::new(label),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(30.0),
                            ..default()
                        },
                        TextColor(if index == 0 {
                            SELECTED_COLOR
                        } else if index == LOAD_GAME_INDEX {
                            DISABLED_COLOR
                        } else {
                            NORMAL_COLOR
                        }),
                        TextLayout::justify(Justify::Center),
                        Node {
                            height: px(42),
                            ..default()
                        },
                        MenuEntry(index),
                    ));
                }
            });

            root.spawn((
                Text::new(""),
                TextFont {
                    font: font.into(),
                    font_size: FontSize::Px(17.0),
                    ..default()
                },
                TextColor(Color::srgb_u8(220, 190, 145)),
                TextLayout::justify(Justify::Center),
                Node {
                    position_type: PositionType::Absolute,
                    bottom: px(8),
                    left: px(0),
                    right: px(0),
                    ..default()
                },
                StatusMessage,
            ));
        });
}

fn cleanup_title_screen(
    mut commands: Commands,
    title_entities: Query<Entity, With<TitleScreenEntity>>,
    mut menu: ResMut<TitleMenu>,
    mut quit: ResMut<QuitLifecycle>,
) {
    for entity in &title_entities {
        commands.entity(entity).despawn();
    }

    *menu = TitleMenu::default();
    *quit = QuitLifecycle::default();
}

fn handle_menu_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut menu: ResMut<TitleMenu>,
    mut quit: ResMut<QuitLifecycle>,
    time: Res<Time>,
    mut status: Single<&mut Text, With<StatusMessage>>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    if !quit.accepts_input() {
        return;
    }

    let direction = if keys.just_pressed(KeyCode::ArrowUp) {
        Some(-1)
    } else if keys.just_pressed(KeyCode::ArrowDown) {
        Some(1)
    } else {
        None
    };

    if let Some(delta) = direction {
        menu.move_by(delta);
        commands.spawn((
            AudioPlayer::new(asset_server.load("audio/menu_hover.mp3")),
            PlaybackSettings::DESPAWN,
            TitleScreenEntity,
        ));
        status.0.clear();
    }

    if !(keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space)) {
        return;
    }

    match title_menu_action(menu.selected) {
        TitleMenuAction::NewGame => {
            commands.spawn((
                AudioPlayer::new(asset_server.load("audio/menu_confirm.mp3")),
                PlaybackSettings::DESPAWN,
                TitleScreenEntity,
            ));
            status.0 = "New Game is the next migration slice.".into();
        }
        TitleMenuAction::LoadGameDisabled => {}
        TitleMenuAction::Quit => {
            if reduce_quit_lifecycle(&mut quit, time.elapsed(), QuitLifecycleEvent::Activate)
                == Some(QuitLifecycleEffect::SpawnConfirm)
            {
                commands.spawn((
                    AudioPlayer::new(asset_server.load("audio/menu_confirm.mp3")),
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
    } else if quit.playback_started() {
        QuitPlaybackObservation::CompletedAfterStart
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

fn update_menu_colors(menu: Res<TitleMenu>, mut entries: Query<(&MenuEntry, &mut TextColor)>) {
    if !menu.is_changed() {
        return;
    }

    for (entry, mut color) in &mut entries {
        color.0 = if entry.0 == LOAD_GAME_INDEX {
            DISABLED_COLOR
        } else if entry.0 == menu.selected {
            SELECTED_COLOR
        } else {
            NORMAL_COLOR
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn activate_at(lifecycle: &mut QuitLifecycle, elapsed: Duration) {
        assert_eq!(
            reduce_quit_lifecycle(lifecycle, elapsed, QuitLifecycleEvent::Activate),
            Some(QuitLifecycleEffect::SpawnConfirm)
        );
    }

    fn observe_at(
        lifecycle: &mut QuitLifecycle,
        elapsed: Duration,
        observation: QuitPlaybackObservation,
    ) -> Option<QuitLifecycleEffect> {
        reduce_quit_lifecycle(lifecycle, elapsed, QuitLifecycleEvent::Observe(observation))
    }

    #[test]
    fn menu_navigation_wraps_in_both_directions() {
        let mut menu = TitleMenu::default();

        menu.move_by(-1);
        assert_eq!(menu.selected, 2);

        menu.move_by(1);
        assert_eq!(menu.selected, 0);
    }

    #[test]
    fn title_menu_entries_resolve_to_distinct_actions() {
        assert_eq!(title_menu_action(0), TitleMenuAction::NewGame);
        assert_eq!(
            title_menu_action(LOAD_GAME_INDEX),
            TitleMenuAction::LoadGameDisabled
        );
        assert_eq!(title_menu_action(2), TitleMenuAction::Quit);
    }

    #[test]
    fn duplicate_quit_activation_spawns_only_one_confirmation() {
        let mut lifecycle = QuitLifecycle::default();

        activate_at(&mut lifecycle, Duration::from_secs(10));
        assert_eq!(
            reduce_quit_lifecycle(
                &mut lifecycle,
                Duration::from_secs(10),
                QuitLifecycleEvent::Activate
            ),
            None
        );
        assert_eq!(
            lifecycle.state,
            QuitLifecycleState::WaitingForStart {
                accepted_at: Duration::from_secs(10)
            }
        );

        assert_eq!(
            observe_at(
                &mut lifecycle,
                Duration::from_millis(10_001),
                QuitPlaybackObservation::Started
            ),
            None
        );
        assert_eq!(
            reduce_quit_lifecycle(
                &mut lifecycle,
                Duration::from_millis(10_002),
                QuitLifecycleEvent::Activate
            ),
            None
        );
        assert_eq!(
            lifecycle.state,
            QuitLifecycleState::WaitingForCompletion {
                started_at: Duration::from_millis(10_001)
            }
        );
    }

    #[test]
    fn normal_playback_exits_only_after_start_then_completion() {
        let mut lifecycle = QuitLifecycle::default();
        activate_at(&mut lifecycle, Duration::ZERO);

        assert_eq!(
            observe_at(
                &mut lifecycle,
                Duration::from_millis(250),
                QuitPlaybackObservation::AwaitingStart
            ),
            None
        );
        assert_eq!(
            observe_at(
                &mut lifecycle,
                Duration::from_millis(500),
                QuitPlaybackObservation::Started
            ),
            None
        );
        assert_eq!(
            observe_at(
                &mut lifecycle,
                Duration::from_millis(1_900),
                QuitPlaybackObservation::CompletedAfterStart
            ),
            Some(QuitLifecycleEffect::EmitExit)
        );
        assert_eq!(lifecycle.state, QuitLifecycleState::ExitSent);
    }

    #[test]
    fn asset_failure_emits_exit_immediately() {
        let mut lifecycle = QuitLifecycle::default();
        activate_at(&mut lifecycle, Duration::from_secs(2));

        assert_eq!(
            observe_at(
                &mut lifecycle,
                Duration::from_millis(2_001),
                QuitPlaybackObservation::AssetLoadFailed
            ),
            Some(QuitLifecycleEffect::EmitExit)
        );
        assert_eq!(lifecycle.state, QuitLifecycleState::ExitSent);
    }

    #[test]
    fn start_timeout_fires_at_the_exact_boundary() {
        let mut lifecycle = QuitLifecycle::default();
        activate_at(&mut lifecycle, Duration::from_secs(4));

        assert_eq!(
            observe_at(
                &mut lifecycle,
                Duration::from_millis(6_999),
                QuitPlaybackObservation::AwaitingStart
            ),
            None
        );
        assert_eq!(
            observe_at(
                &mut lifecycle,
                Duration::from_secs(7),
                QuitPlaybackObservation::AwaitingStart
            ),
            Some(QuitLifecycleEffect::EmitExit)
        );
    }

    #[test]
    fn completion_timeout_fires_at_the_exact_boundary() {
        let mut lifecycle = QuitLifecycle::default();
        activate_at(&mut lifecycle, Duration::ZERO);
        assert_eq!(
            observe_at(
                &mut lifecycle,
                Duration::from_millis(250),
                QuitPlaybackObservation::Started
            ),
            None
        );

        assert_eq!(
            observe_at(
                &mut lifecycle,
                Duration::from_millis(3_249),
                QuitPlaybackObservation::Started
            ),
            None
        );
        assert_eq!(
            observe_at(
                &mut lifecycle,
                Duration::from_millis(3_250),
                QuitPlaybackObservation::Started
            ),
            Some(QuitLifecycleEffect::EmitExit)
        );
    }

    #[test]
    fn completion_cannot_be_inferred_before_a_sink_was_seen() {
        let mut lifecycle = QuitLifecycle::default();
        activate_at(&mut lifecycle, Duration::ZERO);

        assert_eq!(
            observe_at(
                &mut lifecycle,
                Duration::from_secs(1),
                QuitPlaybackObservation::CompletedAfterStart
            ),
            None
        );
        assert_eq!(
            lifecycle.state,
            QuitLifecycleState::WaitingForStart {
                accepted_at: Duration::ZERO
            }
        );
    }

    #[test]
    fn exit_effect_is_emitted_exactly_once() {
        let mut lifecycle = QuitLifecycle::default();
        activate_at(&mut lifecycle, Duration::ZERO);
        assert_eq!(
            observe_at(
                &mut lifecycle,
                Duration::from_secs(3),
                QuitPlaybackObservation::AwaitingStart
            ),
            Some(QuitLifecycleEffect::EmitExit)
        );

        for observation in [
            QuitPlaybackObservation::AwaitingStart,
            QuitPlaybackObservation::Started,
            QuitPlaybackObservation::CompletedAfterStart,
            QuitPlaybackObservation::AssetLoadFailed,
        ] {
            assert_eq!(
                observe_at(&mut lifecycle, Duration::from_secs(4), observation),
                None
            );
        }
        assert_eq!(
            reduce_quit_lifecycle(
                &mut lifecycle,
                Duration::from_secs(4),
                QuitLifecycleEvent::Activate
            ),
            None
        );
        assert_eq!(lifecycle.state, QuitLifecycleState::ExitSent);
    }

    #[test]
    fn headless_quit_input_spawns_one_marked_confirmation_and_suppresses_input() {
        let mut app = crate::test_support::headless_title_app(AppState::Title);
        app.update();
        app.world_mut().resource_mut::<TitleMenu>().selected = 2;
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);

        app.update();

        let world = app.world_mut();
        let marked = world
            .query_filtered::<(&AudioPlayer<AudioSource>, &PlaybackSettings), With<QuitConfirmSound>>()
            .iter(world)
            .map(|(player, settings)| (player.0.id(), settings.mode))
            .collect::<Vec<_>>();
        assert_eq!(marked.len(), 1);
        assert!(matches!(marked[0].1, PlaybackMode::Despawn));
        let confirm_path = world
            .resource::<AssetServer>()
            .get_path(marked[0].0)
            .expect("Quit confirmation should retain its asset path")
            .path()
            .to_string_lossy()
            .into_owned();
        assert_eq!(confirm_path, "audio/menu_confirm.mp3");

        world.resource_mut::<TitleMenu>().selected = 0;
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowDown);
        app.update();

        let world = app.world_mut();
        assert_eq!(world.resource::<TitleMenu>().selected, 0);
        assert_eq!(
            world
                .query_filtered::<&QuitConfirmSound, With<AudioPlayer<AudioSource>>>()
                .iter(world)
                .count(),
            1
        );
        assert_eq!(
            world
                .query::<&AudioPlayer<AudioSource>>()
                .iter(world)
                .count(),
            2,
            "only title music and the single Quit confirmation should exist"
        );
    }

    #[test]
    fn headless_start_timeout_emits_one_app_exit_message() {
        let mut app = crate::test_support::headless_title_app(AppState::Title);
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::ZERO,
        ));
        app.update();

        // Input-to-spawn behavior is covered above. Enter the adapter seam directly here so an
        // asynchronous missing-loader result cannot replace the fallback timeout under test.
        let accepted_at = app.world().resource::<Time>().elapsed();
        app.world_mut().resource_mut::<QuitLifecycle>().state =
            QuitLifecycleState::WaitingForStart { accepted_at };

        let mut exit_cursor = app.world().resource::<Messages<AppExit>>().get_cursor();
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_millis(250),
        ));

        for _ in 1..12 {
            app.update();
            assert_eq!(
                exit_cursor
                    .read(app.world().resource::<Messages<AppExit>>())
                    .count(),
                0,
                "an absent pre-sink entity must not complete Quit before the deadline"
            );
        }

        app.update();
        assert_eq!(
            exit_cursor
                .read(app.world().resource::<Messages<AppExit>>())
                .collect::<Vec<_>>(),
            [&AppExit::Success]
        );
        assert_eq!(
            app.world().resource::<QuitLifecycle>().state,
            QuitLifecycleState::ExitSent
        );

        app.update();
        assert_eq!(
            exit_cursor
                .read(app.world().resource::<Messages<AppExit>>())
                .count(),
            0
        );
    }

    #[test]
    fn title_screen_is_removed_after_leaving_title() {
        let mut app = crate::test_support::headless_title_app(AppState::Boot);
        app.update();

        let world = app.world_mut();
        assert_eq!(world.resource::<State<AppState>>().get(), &AppState::Boot);
        assert_eq!(world.query::<&Camera2d>().iter(world).count(), 0);
        assert_eq!(world.query::<&Sprite>().iter(world).count(), 0);
        assert_eq!(world.query::<&MenuEntry>().iter(world).count(), 0);
        assert_eq!(world.query::<&StatusMessage>().iter(world).count(), 0);
        assert_eq!(
            world
                .query::<&AudioPlayer<AudioSource>>()
                .iter(world)
                .count(),
            0
        );

        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Title);
        app.update();

        let world = app.world_mut();
        assert_eq!(world.resource::<State<AppState>>().get(), &AppState::Title);
        assert_eq!(world.query::<&Camera2d>().iter(world).count(), 1);

        let background_handles = world
            .query::<&Sprite>()
            .iter(world)
            .map(|sprite| sprite.image.id())
            .collect::<Vec<_>>();

        let mut entries = world
            .query::<(&MenuEntry, &Text)>()
            .iter(world)
            .map(|(entry, text)| (entry.0, text.0.clone()))
            .collect::<Vec<_>>();
        entries.sort_by_key(|(index, _)| *index);
        assert_eq!(
            entries,
            [
                (0, "New Game".into()),
                (1, "Load Game".into()),
                (2, "Quit".into())
            ]
        );

        let statuses = world
            .query_filtered::<&Text, With<StatusMessage>>()
            .iter(world)
            .map(|text| text.0.clone())
            .collect::<Vec<_>>();
        assert_eq!(statuses, [""]);

        let title_music = world
            .query::<(&AudioPlayer<AudioSource>, &PlaybackSettings)>()
            .iter(world)
            .map(|(player, settings)| (player.0.id(), settings.mode, settings.volume))
            .collect::<Vec<_>>();

        let asset_server = world.resource::<AssetServer>();
        let backgrounds = background_handles
            .iter()
            .map(|handle| {
                asset_server
                    .get_path(*handle)
                    .expect("title background handle should retain its asset path")
                    .path()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect::<Vec<_>>();
        assert_eq!(backgrounds, ["images/title_lost_flame.webp"]);

        assert_eq!(title_music.len(), 1);
        let title_music_path = asset_server
            .get_path(title_music[0].0)
            .expect("title music handle should retain its asset path")
            .path()
            .to_string_lossy()
            .into_owned();
        assert_eq!(title_music_path, "audio/title_theme.mp3");
        assert!(matches!(title_music[0].1, PlaybackMode::Loop));
        assert_eq!(title_music[0].2, Volume::Linear(0.65));

        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::NameEntry);
        app.update();

        let world = app.world_mut();
        assert_eq!(
            world.resource::<State<AppState>>().get(),
            &AppState::NameEntry
        );
        assert_eq!(world.query::<&Camera2d>().iter(world).count(), 0);
        assert_eq!(world.query::<&Sprite>().iter(world).count(), 0);
        assert_eq!(world.query::<&Node>().iter(world).count(), 0);
        assert_eq!(world.query::<&Text>().iter(world).count(), 0);
        assert_eq!(world.query::<&MenuEntry>().iter(world).count(), 0);
        assert_eq!(world.query::<&StatusMessage>().iter(world).count(), 0);
        assert_eq!(
            world
                .query::<&AudioPlayer<AudioSource>>()
                .iter(world)
                .count(),
            0
        );
        assert_eq!(world.query::<&TitleScreenEntity>().iter(world).count(), 0);
        assert_eq!(world.resource::<TitleMenu>().selected, 0);
        assert_eq!(
            world.resource::<QuitLifecycle>().state,
            QuitLifecycleState::Idle
        );

        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Title);
        app.update();

        let world = app.world_mut();
        assert_eq!(world.resource::<State<AppState>>().get(), &AppState::Title);
        assert_eq!(world.query::<&Camera2d>().iter(world).count(), 1);
        assert_eq!(world.query::<&Sprite>().iter(world).count(), 1);
        assert_eq!(world.query::<&Node>().iter(world).count(), 6);
        assert_eq!(world.query::<&Text>().iter(world).count(), 4);
        assert_eq!(world.query::<&MenuEntry>().iter(world).count(), 3);
        assert_eq!(world.query::<&StatusMessage>().iter(world).count(), 1);
        assert_eq!(
            world
                .query::<&AudioPlayer<AudioSource>>()
                .iter(world)
                .count(),
            1
        );
        assert_eq!(world.query::<&TitleScreenEntity>().iter(world).count(), 4);
    }
}
