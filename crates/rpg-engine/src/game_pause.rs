//! Player pause: the P key freezes the game until it is pressed again.
//!
//! Everything that moves on its own reads Bevy's virtual clock -- NPC walks, wandering enemies,
//! map fades, battle effects -- so pausing [`Time<Virtual>`] stops all of it at once. Three things
//! do not follow that clock and are frozen here by hand:
//!
//! - audio, which plays on its own thread; every live sink is paused and only those are resumed,
//! - input, which would otherwise still drive menus and battle commands under the overlay,
//! - playtime, which is measured in wall-clock time; the paused span is cut from the session.
//!
//! Pause is offered only where a game is in progress. Title, name entry and options are left
//! alone: name entry consumes P as a letter, and nothing there runs on its own.

use std::time::Duration;

use bevy::{
    audio::{AudioSink, AudioSinkPlayback},
    ecs::system::SystemParam,
    input::InputSystems,
    prelude::*,
};

use crate::{
    action_input::{ActionState, update_action_state},
    app_state::AppState,
    game_state::GameState,
    scenario_inventory::ScenarioInventory,
    scenario_root::ScenarioRoot,
    ui_theme::UiTheme,
};

/// The key that toggles pause. Reserved, so it can never be rebound to a menu action.
pub(crate) const PAUSE_KEY: KeyCode = KeyCode::KeyP;

/// Whether the player has paused the game, and since when in wall-clock time.
#[derive(Resource, Default)]
pub(crate) struct GamePause {
    paused_at: Option<Duration>,
}

impl GamePause {
    pub(crate) const fn is_paused(&self) -> bool {
        self.paused_at.is_some()
    }
}

/// Marks a sink this module paused, so resuming never starts audio someone else stopped.
#[derive(Component)]
struct PausedByPlayer;

#[derive(Component)]
struct PauseOverlay;

pub(crate) struct GamePausePlugin;

impl Plugin for GamePausePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GamePause>()
            .add_systems(
                PreUpdate,
                (
                    toggle_pause.after(InputSystems).before(update_action_state),
                    swallow_paused_input.after(update_action_state),
                ),
            )
            .add_systems(Update, hold_audio_paused.run_if(is_paused));
    }
}

fn pausable(state: &AppState) -> bool {
    matches!(
        state,
        AppState::World
            | AppState::Battle
            | AppState::Dialogue
            | AppState::FieldMenu
            | AppState::PostBattle
    )
}

fn is_paused(pause: Res<GamePause>) -> bool {
    pause.is_paused()
}

/// The scenario font for the overlay, falling back to Bevy's default before it is known.
#[derive(SystemParam)]
struct OverlayFont<'w> {
    asset_server: Res<'w, AssetServer>,
    root: Option<Res<'w, ScenarioRoot>>,
    inventory: Option<Res<'w, ScenarioInventory>>,
    theme: Res<'w, UiTheme>,
}

impl OverlayFont<'_> {
    fn font(&self) -> Handle<Font> {
        let (Some(root), Some(inventory)) = (&self.root, &self.inventory) else {
            return Handle::default();
        };
        inventory
            .font
            .as_ref()
            .map(|path| self.asset_server.load(root.resolve(path)))
            .unwrap_or_default()
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "pausing touches the clock, audio, playtime and overlay together"
)]
fn toggle_pause(
    mut commands: Commands,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    state: Option<Res<State<AppState>>>,
    real_time: Res<Time<Real>>,
    mut virtual_time: ResMut<Time<Virtual>>,
    mut pause: ResMut<GamePause>,
    game: Option<ResMut<GameState>>,
    sinks: Query<(Entity, &AudioSink, Has<PausedByPlayer>)>,
    overlays: Query<Entity, With<PauseOverlay>>,
    overlay_font: OverlayFont,
) {
    let in_game = state.is_some_and(|state| pausable(state.get()));
    let pressed = keys.just_pressed(PAUSE_KEY);
    match pause.paused_at {
        None if in_game && pressed => {
            pause.paused_at = Some(real_time.elapsed());
            virtual_time.pause();
            for (entity, sink, _) in &sinks {
                if !sink.is_paused() {
                    sink.pause();
                    commands.entity(entity).insert(PausedByPlayer);
                }
            }
            spawn_overlay(&mut commands, overlay_font.font(), &overlay_font.theme);
        }
        // Leaving the game while paused (never expected, since input is swallowed) still resumes,
        // so a stray state change cannot strand the clock and audio stopped.
        Some(paused_at) if pressed || !in_game => {
            pause.paused_at = None;
            virtual_time.unpause();
            if let Some(mut game) = game {
                let span = real_time.elapsed().saturating_sub(paused_at);
                game.playtime_mut().exclude(span);
            }
            for (entity, sink, paused_by_player) in &sinks {
                if paused_by_player {
                    sink.play();
                    commands.entity(entity).remove::<PausedByPlayer>();
                }
            }
            for overlay in &overlays {
                commands.entity(overlay).despawn();
            }
        }
        _ => {}
    }
    // Screens that read raw keys must not see the toggle, nor anything pressed while paused.
    if pressed && in_game || pause.is_paused() {
        keys.reset_all();
    }
}

/// Drops gamepad and stick input too; the keyboard was already reset before it was read.
fn swallow_paused_input(pause: Res<GamePause>, mut actions: ResMut<ActionState>) {
    if pause.is_paused() {
        actions.clear();
    }
}

/// Catches sinks that start while paused, such as a track whose asset finished loading.
fn hold_audio_paused(
    mut commands: Commands,
    sinks: Query<(Entity, &AudioSink), Without<PausedByPlayer>>,
) {
    for (entity, sink) in &sinks {
        if !sink.is_paused() {
            sink.pause();
            commands.entity(entity).insert(PausedByPlayer);
        }
    }
}

fn spawn_overlay(commands: &mut Commands, font: Handle<Font>, theme: &UiTheme) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: px(16),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
            // Above every other overlay, including the battle hand-off flash.
            GlobalZIndex(12_000),
            PauseOverlay,
        ))
        .with_children(|overlay| {
            overlay.spawn((
                Text::new("PAUSED"),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(64.0),
                    ..default()
                },
                TextColor(theme.menu_selected_color),
            ));
            overlay.spawn((
                Text::new("Press P to resume"),
                TextFont {
                    font: font.into(),
                    font_size: FontSize::Px(theme.status_font_size + 5.0),
                    ..default()
                },
                TextColor(theme.menu_normal_color),
            ));
        });
}

#[cfg(test)]
mod tests {
    use bevy::state::app::StatesPlugin;

    use super::*;
    use crate::action_input::{ActionInputPlugin, AppAction};

    fn pause_app(state: AppState) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, AssetPlugin::default()))
            .init_asset::<Font>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<UiTheme>()
            .insert_state(state)
            .add_plugins((ActionInputPlugin, GamePausePlugin));
        app.update();
        app
    }

    fn tap(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(key);
        keys.clear();
    }

    fn paused(app: &App) -> bool {
        app.world().resource::<GamePause>().is_paused()
    }

    fn overlays(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<(), With<PauseOverlay>>()
            .iter(app.world())
            .count()
    }

    #[test]
    fn p_pauses_the_virtual_clock_and_p_again_resumes_it() {
        let mut app = pause_app(AppState::World);

        tap(&mut app, PAUSE_KEY);
        assert!(paused(&app));
        assert!(app.world().resource::<Time<Virtual>>().is_paused());
        assert_eq!(overlays(&mut app), 1);

        tap(&mut app, PAUSE_KEY);
        assert!(!paused(&app));
        assert!(!app.world().resource::<Time<Virtual>>().is_paused());
        app.update();
        assert_eq!(overlays(&mut app), 0);
    }

    #[test]
    fn menu_input_is_swallowed_while_paused() {
        let mut app = pause_app(AppState::Battle);
        tap(&mut app, PAUSE_KEY);

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        app.update();

        assert!(
            !app.world()
                .resource::<ActionState>()
                .just_pressed(AppAction::Confirm),
            "a paused battle must not take commands"
        );
    }

    #[test]
    fn p_is_left_alone_outside_a_game() {
        // Name entry types P as a letter; the title has nothing to freeze.
        for state in [AppState::NameEntry, AppState::Title, AppState::Options] {
            let mut app = pause_app(state);
            tap(&mut app, PAUSE_KEY);
            assert!(!paused(&app), "{state:?} must not pause");
        }
    }

    #[test]
    fn leaving_the_game_while_paused_resumes_the_clock() {
        let mut app = pause_app(AppState::World);
        tap(&mut app, PAUSE_KEY);

        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Title);
        app.update();
        app.update();

        assert!(!paused(&app));
        assert!(!app.world().resource::<Time<Virtual>>().is_paused());
    }
}
