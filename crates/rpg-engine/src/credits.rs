//! The end-of-game credits roll.
//!
//! B0.3. Entered from the world once the epilogue sets `game_complete` — the flag is declared in
//! the manifest's `engine_managed_flags` because content produces it and the engine, not other
//! content, consumes it. The roll names the ending the player earned, scrolls its body once, and
//! then waits on a keypress before returning to the title.
//!
//! Both the trigger and the earned title read their flags out of [`GameState`], which is the only
//! place a session's [`RuntimeFlags`] ever live — `RuntimeFlags` is never a free-standing
//! resource, so watching for one matched nothing and stranded finished runs in the epilogue.
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

/// Pixels per second the roll travels.
const SCROLL_SPEED: f32 = 42.0;
/// Where the roll starts, as a fraction of the viewport height below the top.
const START_OFFSET: f32 = 620.0;
/// How long the finished roll rests on its last frame before returning to the title by itself.
///
/// The roll ends with its tail just off the top, so this is a beat of empty screen — short enough
/// to read as a fade rather than as a hang. A keypress still leaves early.
const HOLD_SECONDS: f32 = 2.5;

/// Type metrics the roll is laid out with. [`roll_height`] reproduces the column's height from
/// them, so the travel distance tracks the body instead of being guessed at.
const TITLE_FONT_PX: f32 = 56.0;
const BODY_FONT_PX: f32 = 22.0;
const COLUMN_GAP_PX: f32 = 28.0;
/// Bevy lays a text line out at roughly 1.2x its font size.
const LINE_HEIGHT: f32 = 1.2;

/// The rendered height of the whole column, title and body together.
fn roll_height(body_lines: usize) -> f32 {
    TITLE_FONT_PX * LINE_HEIGHT + COLUMN_GAP_PX + body_lines as f32 * BODY_FONT_PX * LINE_HEIGHT
}

/// Where the roll stops: far enough for the last line to clear the top of the screen, and no
/// further. A fixed constant here used to overshoot the body by hundreds of pixels, which is what
/// left the player watching a blank screen scroll for five seconds and then hold there forever.
fn end_offset(body_lines: usize) -> f32 {
    -roll_height(body_lines)
}

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


— Direction —

Ninja


— Programming —

Claude  ·  Codex


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


— Characters and Terrain —

Liberated Pixel Cup and OpenGameArt community work, by

bluecarrot16  ·  Lanea Zimmerman (Sharm)
Stephen Challener (Redshrike)  ·  Eliza Wyatt (ElizaWy)
Daniel Eddeland (Daneeklu)  ·  Richard Kettering (Jetrel)
Zachariah Husiar (Zabin)  ·  Johannes Sjölund (wulax)
Matthew Krohn (makrohn)  ·  Benjamin K. Smith (BenCreating)
JaidynReiman  ·  Hyptosis  ·  Casper Nilsson  ·  Buko Studios
Nushio  ·  ZaPaper  ·  billknye  ·  William Thompson  ·  caeles
Bertram  ·  Rayane Félix (RayaneFLX)  ·  Evert  ·  TheraHedwig
MuffinElZangano  ·  Durrani  ·  Pierre Vigier (pvigier)

under OGA-BY 3.0, CC-BY 3.0, CC-BY-SA 3.0 and GPL 3.0.


— Tilesets —

Interiors and furniture by Astral Pixels
Caves and dungeons by schwarnhild


— Portraits and Music —

Party portraits and original score
generated with OpenAI ChatGPT


— Sound —

Sound effects by Leohpaz


— Type —

Philosopher, (c) 2011 The Philosopher Project Authors
SIL Open Font License 1.1


— Full attribution —

Per-file authors, licences and source links ship with
the game, in credits/ and alongside each asset.


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
    /// Where this roll stops, from [`end_offset`] over the body it was built with.
    end: f32,
    /// Seconds the roll has been sitting at [`Self::end`]. Drives the automatic return.
    held: f32,
}

/// Moves the app into the credits the first time the epilogue reports the run finished.
///
/// Restricted to the states a running game can be in, so returning to the title after the roll
/// cannot bounce straight back into it.
fn watch_for_game_complete(
    mut commands: Commands,
    state: Res<State<AppState>>,
    shown: Option<Res<CreditsShown>>,
    game: Option<Res<GameState>>,
    mut transitions: MessageWriter<AppStateTransitionRequest>,
) {
    if shown.is_some() || !matches!(state.get(), AppState::World | AppState::Dialogue) {
        return;
    }
    if game.is_some_and(|game| game.flags().is_set(GAME_COMPLETE_FLAG)) {
        commands.insert_resource(CreditsShown);
        transitions.write(AppStateTransitionRequest::new(AppState::Credits));
    }
}

fn enter_credits(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
    game: Option<Res<GameState>>,
) {
    commands.spawn((fixed_gameplay_camera(), CreditsUi));
    let Some(font_path) = inventory.font.as_ref() else {
        return;
    };
    let font = asset_server.load(root.resolve(font_path));
    let title = game.map_or("THE END", |game| ending_title(game.flags()));
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
                        end: end_offset(BODY.lines().count()),
                        held: 0.0,
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

/// Advances the roll by one frame, stopping dead at `end` once the tail has cleared the top.
fn next_offset(offset: f32, end: f32, delta_seconds: f32) -> f32 {
    (offset - SCROLL_SPEED * delta_seconds).max(end)
}

fn scroll_credits(time: Res<Time>, mut columns: Query<(&mut Node, &mut CreditsScroll)>) {
    for (mut node, mut scroll) in &mut columns {
        scroll.offset = next_offset(scroll.offset, scroll.end, time.delta_secs());
        if scroll.offset <= scroll.end {
            scroll.held += time.delta_secs();
        }
        node.top = px(scroll.offset);
    }
}

/// Whether a roll that has held this long should return to the title on its own.
fn roll_is_over(held: f32) -> bool {
    held >= HOLD_SECONDS
}

/// Leaves the credits, either because the player pressed something or because the roll finished.
///
/// Without the second reason the game simply stopped: the roll ran out, and a finished run sat on
/// a blank screen with no exit, because the epilogue map it came from has no portal to walk back
/// through.
fn handle_credits_input(
    mut commands: Commands,
    actions: Res<ActionState>,
    columns: Query<&CreditsScroll>,
    mut transitions: MessageWriter<AppStateTransitionRequest>,
    mut menu_sfx: MenuSfx,
) {
    let dismissed =
        actions.just_pressed(AppAction::Confirm) || actions.just_pressed(AppAction::Back);
    let finished = columns.iter().any(|scroll| roll_is_over(scroll.held));
    if !dismissed && !finished {
        return;
    }
    // An automatic return is not a menu choice, so it does not click.
    if dismissed {
        menu_sfx.confirm();
    }
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
    use bevy::state::app::StatesPlugin;

    use super::*;
    use crate::save_data::tests::fixture_game;

    /// The watcher alone, over a session that has or has not reached the epilogue's last line.
    fn watcher_app(state: AppState, complete: bool) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(StatesPlugin)
            .insert_state(state)
            .add_message::<AppStateTransitionRequest>()
            .add_systems(Update, watch_for_game_complete);
        let mut game = fixture_game();
        if complete {
            game.flags_mut().set(GAME_COMPLETE_FLAG);
        }
        app.insert_resource(game);
        app
    }

    fn requested_transitions(app: &App) -> usize {
        app.world()
            .resource::<Messages<AppStateTransitionRequest>>()
            .len()
    }

    /// The regression this module was silently failing: the flag the epilogue sets lives in
    /// `GameState`, which is the only place `RuntimeFlags` is ever stored. Watching for a
    /// free-standing `Res<RuntimeFlags>` matched nothing, so a finished run stood in the epilogue
    /// shrine — a map with no portal — with no way out and no credits.
    #[test]
    fn the_epilogues_flag_is_read_from_the_session_that_actually_holds_it() {
        let mut app = watcher_app(AppState::World, true);

        app.update();

        assert_eq!(requested_transitions(&app), 1);
        assert!(app.world().get_resource::<CreditsShown>().is_some());
    }

    #[test]
    fn a_run_still_in_progress_is_left_alone() {
        let mut app = watcher_app(AppState::World, false);

        app.update();

        assert_eq!(requested_transitions(&app), 0);
        assert!(app.world().get_resource::<CreditsShown>().is_none());
    }

    /// Joss's line closes from `Dialogue`, so the watcher has to fire there too.
    #[test]
    fn the_roll_is_reachable_from_the_dialogue_that_ends_the_game() {
        let mut app = watcher_app(AppState::Dialogue, true);

        app.update();

        assert_eq!(requested_transitions(&app), 1);
    }

    #[test]
    fn a_finished_run_rolls_the_credits_once_per_process() {
        let mut app = watcher_app(AppState::World, true);

        app.update();
        app.world_mut()
            .resource_mut::<Messages<AppStateTransitionRequest>>()
            .clear();
        app.update();

        assert_eq!(requested_transitions(&app), 0);
    }

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
        let end = end_offset(BODY.lines().count());
        let stepped = next_offset(START_OFFSET, end, 1.0);
        assert!(
            stepped < START_OFFSET,
            "the roll must travel toward the top"
        );
        assert_eq!(next_offset(end, end, 10.0), end);
        assert_eq!(next_offset(end + 1.0, end, 10.0), end);
    }

    /// The roll has to be longer than the screen or it would never move.
    #[test]
    fn the_body_is_long_enough_to_scroll() {
        assert!(BODY.lines().count() > 30);
    }

    /// The blank-screen bug: the travel distance was a fixed -1500 while the body was worth about
    /// 1300px, so the roll spent seconds scrolling nothing and then stopped on an empty screen.
    /// Deriving it from the body keeps the stop where the last line leaves.
    #[test]
    fn the_roll_travels_exactly_as_far_as_its_body_is_tall() {
        let lines = BODY.lines().count();
        assert_eq!(end_offset(lines), -roll_height(lines));
        assert!(
            end_offset(lines) < 0.0,
            "the roll must end above where it started"
        );
    }

    #[test]
    fn a_longer_body_scrolls_further() {
        assert!(end_offset(80) < end_offset(40));
    }

    #[test]
    fn the_finished_roll_returns_to_the_title_on_its_own() {
        assert!(!roll_is_over(0.0));
        assert!(!roll_is_over(HOLD_SECONDS - 0.1));
        assert!(roll_is_over(HOLD_SECONDS));
    }

    /// The attribution the art licences actually require has to survive edits to the roll.
    #[test]
    fn the_roll_names_the_people_whose_licences_require_naming() {
        for required in [
            "bluecarrot16",
            "Lanea Zimmerman (Sharm)",
            "Stephen Challener (Redshrike)",
            "Astral Pixels",
            "schwarnhild",
            "Leohpaz",
            "SIL Open Font License",
            "CC-BY-SA 3.0",
        ] {
            assert!(BODY.contains(required), "the roll dropped `{required}`");
        }
    }

    /// Who made the game, as the director recorded it. Separate from the licence-required names
    /// above because these are credits rather than obligations.
    #[test]
    fn the_roll_credits_the_people_and_tools_that_made_the_game() {
        for required in ["Ninja", "Claude", "Codex", "ChatGPT"] {
            assert!(BODY.contains(required), "the roll dropped `{required}`");
        }
    }
}
