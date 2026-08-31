//! The end-of-game credits roll.
//!
//! B0.3. Entered from the world once the epilogue sets `game_complete` — the flag is declared in
//! the manifest's `engine_managed_flags` because content produces it and the engine, not other
//! content, consumes it. The roll names the ending the player earned, scrolls its body once, and
//! then waits on a keypress before returning to the title.
//!
//! The trigger is guarded by [`CreditsShown`], which is inserted on entry and never removed: a
//! finished run rolls credits exactly once. Loading a completed save in a *later* process rolls
//! them again, which is deliberate — it is the only way back to them.

use bevy::prelude::*;

use crate::{
    action_input::{ActionState, AppAction},
    app_state::{AppState, AppStateTransitionRequest},
    encounter::BattleEntry,
    game_state::GameState,
    gameplay_canvas::fixed_gameplay_camera,
    runtime_flags::RuntimeFlags,
    scenario_inventory::ScenarioInventory,
    scenario_root::ScenarioRoot,
    sfx_cue::{MenuSfx, PlaySfx},
};

/// Set by the epilogue's terminal dialogue line. Declared in `manifest.yaml`.
pub(crate) const GAME_COMPLETE_FLAG: &str = "game_complete";

/// Pixels per second the roll travels. The body is ~40 lines, so a full pass runs about a minute.
const SCROLL_SPEED: f32 = 42.0;
/// Where the roll starts, as a fraction of the viewport height below the top.
const START_OFFSET: f32 = 620.0;
/// How far past the top the roll travels before it stops and waits for a keypress.
const END_OFFSET: f32 = -1_500.0;

/// The three earned endings, in the order `plans/b0-ending-design.md` records them.
const ENDINGS: [(&str, &str); 3] = [
    ("ending_path_third", "THE THIRD FIRE"),
    ("ending_path_release", "THE RELEASE"),
    ("ending_path_rekindle", "THE REKINDLING"),
];

/// The ending title for a flag set, or the default when a save reaches credits without one.
fn ending_title(flags: &RuntimeFlags) -> &'static str {
    ENDINGS
        .iter()
        .find(|(flag, _)| flags.is_set(flag))
        .map_or("THE END", |(_, title)| *title)
}

const BODY: &str = "\
CHRONICLES OF THE LOST FLAME


— Cast —

Aric, banked in the ash of Ardel
Elise, who kept asking the fire
Reiya, who paid for the answer
Jep, who did not go down the shaft
Kael, who sat down on the fifth night

Keeper Joss, who swept the floor for sixteen years
Guardsman Pike, who held a bridge he could not hold
The Ashen Warden, who had no further instructions


— The Rusted Kingdoms —

Ardel  ·  Millhaven  ·  Ruinwatch
Frostholm  ·  Ashenveil  ·  Harborgate


— Art and Audio —

Sprites, tiles and portraits are Liberated Pixel Cup
and OpenGameArt community work.

Every author, licence and source link is listed in
credits/01_aric_credits.txt, shipped with this game.


— Thanks —

For playing all the way to the bottom.


";

pub(crate) struct CreditsPlugin;

impl Plugin for CreditsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PlaySfx>()
            .add_systems(Update, watch_for_game_complete)
            .add_systems(OnEnter(AppState::Credits), enter_credits)
            .add_systems(
                Update,
                (scroll_credits, handle_credits_input)
                    .chain()
                    .run_if(in_state(AppState::Credits)),
            )
            .add_systems(OnExit(AppState::Credits), cleanup_credits);
    }
}

/// Present once the credits have rolled in this process. See the module docs.
#[derive(Debug, Resource)]
struct CreditsShown;

#[derive(Component)]
struct CreditsUi;

/// The scrolling column, offset every frame.
#[derive(Component)]
struct CreditsScroll {
    offset: f32,
}

/// Moves the app into the credits the first time the epilogue reports the run finished.
///
/// Restricted to the states a running game can be in, so returning to the title after the roll
/// cannot bounce straight back into it.
fn watch_for_game_complete(
    mut commands: Commands,
    state: Res<State<AppState>>,
    shown: Option<Res<CreditsShown>>,
    flags: Option<Res<RuntimeFlags>>,
    mut transitions: MessageWriter<AppStateTransitionRequest>,
) {
    if shown.is_some() || !matches!(state.get(), AppState::World | AppState::Dialogue) {
        return;
    }
    if flags.is_some_and(|flags| flags.is_set(GAME_COMPLETE_FLAG)) {
        commands.insert_resource(CreditsShown);
        transitions.write(AppStateTransitionRequest::new(AppState::Credits));
    }
}

fn enter_credits(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
    flags: Option<Res<RuntimeFlags>>,
) {
    commands.spawn((fixed_gameplay_camera(), CreditsUi));
    let Some(font_path) = inventory.font.as_ref() else {
        return;
    };
    let font = asset_server.load(root.resolve(font_path));
    let title = flags.map_or("THE END", |flags| ending_title(&flags));
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgba(0.01, 0.008, 0.02, 1.0)),
            GlobalZIndex(6_000),
            CreditsUi,
        ))
        .with_children(|screen| {
            screen
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        top: px(START_OFFSET),
                        width: percent(100),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: px(28),
                        ..default()
                    },
                    CreditsScroll {
                        offset: START_OFFSET,
                    },
                ))
                .with_children(|column| {
                    column.spawn((
                        Text::new(title),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(56.0),
                            ..default()
                        },
                        TextColor(Color::srgb_u8(235, 190, 110)),
                        TextLayout::justify(Justify::Center),
                    ));
                    column.spawn((
                        Text::new(BODY),
                        TextFont {
                            font: font.into(),
                            font_size: FontSize::Px(22.0),
                            ..default()
                        },
                        TextColor(Color::srgb_u8(214, 210, 200)),
                        TextLayout::justify(Justify::Center),
                    ));
                });
        });
}

/// Advances the roll by one frame, stopping dead at [`END_OFFSET`] so the tail stays on screen
/// instead of scrolling away while the player is still reading it.
fn next_offset(offset: f32, delta_seconds: f32) -> f32 {
    (offset - SCROLL_SPEED * delta_seconds).max(END_OFFSET)
}

fn scroll_credits(time: Res<Time>, mut columns: Query<(&mut Node, &mut CreditsScroll)>) {
    for (mut node, mut scroll) in &mut columns {
        scroll.offset = next_offset(scroll.offset, time.delta_secs());
        node.top = px(scroll.offset);
    }
}

fn handle_credits_input(
    mut commands: Commands,
    actions: Res<ActionState>,
    mut transitions: MessageWriter<AppStateTransitionRequest>,
    mut menu_sfx: MenuSfx,
) {
    if !actions.just_pressed(AppAction::Confirm) && !actions.just_pressed(AppAction::Back) {
        return;
    }
    menu_sfx.confirm();
    commands.remove_resource::<BattleEntry>();
    commands.remove_resource::<GameState>();
    transitions.write(AppStateTransitionRequest::new(AppState::Title));
}

fn cleanup_credits(mut commands: Commands, entities: Query<Entity, With<CreditsUi>>) {
    for entity in &entities {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ending_titles_follow_the_recorded_precedence() {
        assert_eq!(
            ending_title(&RuntimeFlags::from_bootstrap(["ending_path_rekindle"])),
            "THE REKINDLING"
        );
        assert_eq!(
            ending_title(&RuntimeFlags::from_bootstrap(["ending_path_release"])),
            "THE RELEASE"
        );
        assert_eq!(
            ending_title(&RuntimeFlags::from_bootstrap(["ending_path_third"])),
            "THE THIRD FIRE"
        );
    }

    #[test]
    fn a_run_without_a_recorded_path_still_gets_a_title() {
        assert_eq!(ending_title(&RuntimeFlags::default()), "THE END");
    }

    /// The Third Fire is the hardest path to earn, so it wins if a save somehow carries two.
    #[test]
    fn the_earned_path_wins_over_the_default_one() {
        let both = RuntimeFlags::from_bootstrap(["ending_path_rekindle", "ending_path_third"]);
        assert_eq!(ending_title(&both), "THE THIRD FIRE");
    }

    #[test]
    fn the_roll_advances_upward_and_then_holds_at_its_tail() {
        let stepped = next_offset(START_OFFSET, 1.0);
        assert!(
            stepped < START_OFFSET,
            "the roll must travel toward the top"
        );
        assert_eq!(next_offset(END_OFFSET, 10.0), END_OFFSET);
        assert_eq!(next_offset(END_OFFSET + 1.0, 10.0), END_OFFSET);
    }

    /// The roll has to be longer than the screen or it would never move.
    #[test]
    fn the_body_is_long_enough_to_scroll() {
        assert!(BODY.lines().count() > 30);
    }
}
