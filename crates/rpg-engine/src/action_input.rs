use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use bevy::{input::InputSystems, input::gamepad::Gamepad, prelude::*};

use crate::{
    input_bindings::{BindingTarget, InputBindings},
    input_record::NormalizedAction,
    options_store::PlayerOptions,
};

pub(crate) use crate::input_bindings::{AppAction, MovementAction};

/// How far a stick must leave center before it counts as a direction.
///
/// Chosen above the resting drift of a worn stick and below a deliberate half-push. The same
/// threshold serves menus and walking so that a player who can steer the world can also steer the
/// menu that world opens.
const STICK_DEADZONE: f32 = 0.4;

/// How long a menu direction must be held before the cursor starts moving on its own.
const MENU_REPEAT_DELAY: Duration = Duration::from_millis(400);

/// The gap between cursor steps once a held menu direction is repeating.
const MENU_REPEAT_INTERVAL: Duration = Duration::from_millis(90);

/// The menu actions that auto-repeat while held. Back, Confirm and Travel never do.
const MENU_DIRECTIONS: [AppAction; 4] = [
    AppAction::Up,
    AppAction::Down,
    AppAction::Left,
    AppAction::Right,
];

/// Semantic actions that began during the current input frame.
#[derive(Resource, Default)]
pub(crate) struct ActionState {
    just_pressed: [bool; 7],
    /// Held menu directions whose auto-repeat fired this frame. Kept apart from `just_pressed` so
    /// that only cursor movement repeats: battle reads a Right press as Confirm, and that must not
    /// fire again and again under a held key.
    repeated: [bool; 7],
    movement_pressed: [bool; 4],
    /// Set whenever a screen reads menu navigation, and cleared by the next input frame. A hold
    /// only repeats while some screen is listening, so walking with an arrow held and then
    /// opening a menu does not send its cursor racing.
    navigation_read: AtomicBool,
}

impl ActionState {
    pub(crate) fn just_pressed(&self, action: AppAction) -> bool {
        self.just_pressed[action.index()]
    }

    /// Drops every press, repeat and held direction read this frame.
    pub(crate) fn clear(&mut self) {
        self.just_pressed.fill(false);
        self.repeated.fill(false);
        self.movement_pressed.fill(false);
    }

    /// One cursor step toward `back` (-1) or `forward` (+1), a fresh press beating a repeat.
    fn navigation_step(&self, back: AppAction, forward: AppAction) -> Option<isize> {
        self.navigation_read.store(true, Ordering::Relaxed);
        if self.just_pressed(back) {
            Some(-1)
        } else if self.just_pressed(forward) {
            Some(1)
        } else if self.repeated[back.index()] {
            Some(-1)
        } else if self.repeated[forward.index()] {
            Some(1)
        } else {
            None
        }
    }

    /// Resolves same-frame menu navigation to one movement.
    ///
    /// The Python menu consumes key-down events in platform arrival order, so pressing Up and
    /// Down in one frame has no source-defined simultaneous-key outcome. The Rust port fixes
    /// that ambiguity: Up wins. This prevents a conflicting pair from taking two menu steps and
    /// keeps the result independent of platform event ordering.
    ///
    /// A held direction also steps here once it starts repeating; see [`MenuRepeat`].
    pub(crate) fn menu_navigation(&self) -> Option<isize> {
        self.navigation_step(AppAction::Up, AppAction::Down)
    }

    /// Resolves same-frame horizontal menu navigation to one movement.
    ///
    /// The pinned engine's battle target picker treats Left as "previous" and Right as "next"
    /// alongside Up/Down (`battle_input.py:106-109`). Left wins a conflicting pair for the same
    /// reason Up does above: one keypress pair must never take two steps.
    pub(crate) fn menu_navigation_horizontal(&self) -> Option<isize> {
        self.navigation_step(AppAction::Left, AppAction::Right)
    }

    /// Resolves the currently held source-compatible eight-way movement input.
    ///
    /// World movement is continuous, so held directions remain active every frame. The vector
    /// uses every held direction exactly as Python's summed key state does; opposites cancel.
    pub(crate) fn movement(&self) -> Option<crate::scenario_spatial::EightWayDirection> {
        use crate::scenario_spatial::EightWayDirection;

        let horizontal = i8::from(self.movement_pressed[MovementAction::Right.index()])
            - i8::from(self.movement_pressed[MovementAction::Left.index()]);
        let vertical = i8::from(self.movement_pressed[MovementAction::Down.index()])
            - i8::from(self.movement_pressed[MovementAction::Up.index()]);
        match (horizontal, vertical) {
            (0, -1) => Some(EightWayDirection::Up),
            (1, -1) => Some(EightWayDirection::UpRight),
            (1, 0) => Some(EightWayDirection::Right),
            (1, 1) => Some(EightWayDirection::DownRight),
            (0, 1) => Some(EightWayDirection::Down),
            (-1, 1) => Some(EightWayDirection::DownLeft),
            (-1, 0) => Some(EightWayDirection::Left),
            (-1, -1) => Some(EightWayDirection::UpLeft),
            _ => None,
        }
    }

    pub(crate) fn normalized_actions(&self) -> Vec<NormalizedAction> {
        let mut normalized = Vec::new();
        for (action, value) in [
            (AppAction::Back, NormalizedAction::Back),
            (AppAction::Confirm, NormalizedAction::Confirm),
            (AppAction::Up, NormalizedAction::MenuUp),
            (AppAction::Down, NormalizedAction::MenuDown),
            (AppAction::Left, NormalizedAction::MenuLeft),
            (AppAction::Right, NormalizedAction::MenuRight),
            (AppAction::Travel, NormalizedAction::Travel),
        ] {
            if self.just_pressed(action) {
                normalized.push(value);
            }
        }
        for (action, value) in [
            (AppAction::Up, NormalizedAction::MenuRepeatUp),
            (AppAction::Down, NormalizedAction::MenuRepeatDown),
            (AppAction::Left, NormalizedAction::MenuRepeatLeft),
            (AppAction::Right, NormalizedAction::MenuRepeatRight),
        ] {
            if self.repeated[action.index()] {
                normalized.push(value);
            }
        }
        for (action, value) in [
            (MovementAction::Up, NormalizedAction::MoveUp),
            (MovementAction::Left, NormalizedAction::MoveLeft),
            (MovementAction::Down, NormalizedAction::MoveDown),
            (MovementAction::Right, NormalizedAction::MoveRight),
        ] {
            if self.movement_pressed[action.index()] {
                normalized.push(value);
            }
        }
        normalized.sort_unstable();
        normalized.dedup();
        normalized
    }

    pub(crate) fn replace_with_normalized(&mut self, normalized: &[NormalizedAction]) {
        self.clear();
        for action in normalized {
            match action {
                NormalizedAction::Back => self.just_pressed[AppAction::Back.index()] = true,
                NormalizedAction::Confirm => self.just_pressed[AppAction::Confirm.index()] = true,
                NormalizedAction::MenuUp => self.just_pressed[AppAction::Up.index()] = true,
                NormalizedAction::MenuDown => self.just_pressed[AppAction::Down.index()] = true,
                NormalizedAction::MenuLeft => self.just_pressed[AppAction::Left.index()] = true,
                NormalizedAction::MenuRight => self.just_pressed[AppAction::Right.index()] = true,
                NormalizedAction::Travel => self.just_pressed[AppAction::Travel.index()] = true,
                NormalizedAction::MenuRepeatUp => self.repeated[AppAction::Up.index()] = true,
                NormalizedAction::MenuRepeatDown => self.repeated[AppAction::Down.index()] = true,
                NormalizedAction::MenuRepeatLeft => self.repeated[AppAction::Left.index()] = true,
                NormalizedAction::MenuRepeatRight => self.repeated[AppAction::Right.index()] = true,
                NormalizedAction::MoveUp => {
                    self.movement_pressed[MovementAction::Up.index()] = true
                }
                NormalizedAction::MoveLeft => {
                    self.movement_pressed[MovementAction::Left.index()] = true
                }
                NormalizedAction::MoveDown => {
                    self.movement_pressed[MovementAction::Down.index()] = true
                }
                NormalizedAction::MoveRight => {
                    self.movement_pressed[MovementAction::Right.index()] = true
                }
            }
        }
    }
}

/// One frame's stick position, quantized to the nine directions a menu understands.
///
/// Screen convention, not stick convention: `vertical == 1` means down the screen, matching
/// [`ActionState::movement`]. A gamepad reports +Y as up, so the sign is flipped once, here.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct StickDirection {
    horizontal: i8,
    vertical: i8,
}

impl StickDirection {
    fn from_stick(stick: Vec2) -> Self {
        let quantize = |value: f32| {
            if value >= STICK_DEADZONE {
                1
            } else if value <= -STICK_DEADZONE {
                -1
            } else {
                0
            }
        };
        Self {
            horizontal: quantize(stick.x),
            vertical: quantize(-stick.y),
        }
    }
}

/// The previous frame's stick direction, so a stick can drive edge-triggered menu navigation.
///
/// Menu actions are edge-triggered; a stick reports a level. Without this, holding the stick would
/// step a menu every frame. An edge is emitted only when an axis newly leaves center or reverses;
/// a stick held off center then repeats on the same schedule as a held key, via [`MenuRepeat`].
#[derive(Resource, Default)]
pub(crate) struct StickNavigation {
    previous: StickDirection,
}

impl StickNavigation {
    /// Advances to `current`, returning the directions that began this frame.
    fn advance(&mut self, current: StickDirection) -> StickDirection {
        let edge = |now: i8, before: i8| if now != 0 && now != before { now } else { 0 };
        let began = StickDirection {
            horizontal: edge(current.horizontal, self.previous.horizontal),
            vertical: edge(current.vertical, self.previous.vertical),
        };
        self.previous = current;
        began
    }
}

/// How long each menu direction has been held, so a long press walks the cursor.
///
/// Repeats start after [`MENU_REPEAT_DELAY`] and then come every [`MENU_REPEAT_INTERVAL`]. A hold
/// that spans a frame in which no screen read menu navigation goes stale and never repeats until
/// it is released: the arrows double as walking keys, and a direction held through the world
/// must not scroll the menu or dialogue that opens under it.
#[derive(Resource, Default)]
pub(crate) struct MenuRepeat {
    held_for: [Option<Duration>; 7],
}

impl MenuRepeat {
    /// Advances one direction by `delta`, returning whether its repeat fires this frame.
    fn advance(&mut self, action: AppAction, held: bool, began: bool, delta: Duration) -> bool {
        let slot = &mut self.held_for[action.index()];
        if !held {
            *slot = None;
            return false;
        }
        if began {
            *slot = Some(Duration::ZERO);
            return false;
        }
        let Some(before) = *slot else {
            return false;
        };
        let after = before + delta;
        *slot = Some(after);
        repeats_due(after) > repeats_due(before)
    }

    fn mark_stale(&mut self) {
        self.held_for.fill(None);
    }
}

/// How many repeat steps a hold of this length has earned.
fn repeats_due(held: Duration) -> u128 {
    match held.checked_sub(MENU_REPEAT_DELAY) {
        Some(past) => 1 + past.as_nanos() / MENU_REPEAT_INTERVAL.as_nanos(),
        None => 0,
    }
}

pub(crate) struct ActionInputPlugin;

impl Plugin for ActionInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerOptions>()
            .init_resource::<ActionState>()
            .init_resource::<StickNavigation>()
            .init_resource::<MenuRepeat>()
            .add_systems(PreUpdate, update_action_state.after(InputSystems));
    }
}

/// The stick of the connected pad that is pushed furthest.
///
/// Several pads can be connected at once and Bevy reports each separately. Taking the furthest
/// rather than the first means an idle second controller cannot cancel out the one being played.
fn dominant_left_stick(gamepads: &Query<&Gamepad>) -> Vec2 {
    gamepads
        .iter()
        .map(Gamepad::left_stick)
        .fold(Vec2::ZERO, |furthest, stick| {
            if stick.length_squared() > furthest.length_squared() {
                stick
            } else {
                furthest
            }
        })
}

fn menu_stick_edge(action: AppAction, began: StickDirection) -> bool {
    match action {
        AppAction::Up => began.vertical == -1,
        AppAction::Down => began.vertical == 1,
        AppAction::Left => began.horizontal == -1,
        AppAction::Right => began.horizontal == 1,
        AppAction::Back | AppAction::Confirm | AppAction::Travel => false,
    }
}

fn menu_stick_held(action: AppAction, current: StickDirection) -> bool {
    match action {
        AppAction::Up => current.vertical == -1,
        AppAction::Down => current.vertical == 1,
        AppAction::Left => current.horizontal == -1,
        AppAction::Right => current.horizontal == 1,
        AppAction::Back | AppAction::Confirm | AppAction::Travel => false,
    }
}

fn movement_stick_held(action: MovementAction, current: StickDirection) -> bool {
    match action {
        MovementAction::Up => current.vertical == -1,
        MovementAction::Down => current.vertical == 1,
        MovementAction::Left => current.horizontal == -1,
        MovementAction::Right => current.horizontal == 1,
    }
}

pub(crate) fn update_action_state(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    options: Res<PlayerOptions>,
    time: Res<Time>,
    mut stick_navigation: ResMut<StickNavigation>,
    mut menu_repeat: ResMut<MenuRepeat>,
    mut actions: ResMut<ActionState>,
) {
    let bindings: &InputBindings = &options.bindings;
    let stick = StickDirection::from_stick(dominant_left_stick(&gamepads));
    let began = stick_navigation.advance(stick);

    for action in AppAction::ALL {
        let target = BindingTarget::Menu(action);
        let from_keyboard = bindings
            .keys(target)
            .iter()
            .any(|key| keys.just_pressed(*key));
        let from_gamepad = bindings
            .buttons(target)
            .iter()
            .any(|button| gamepads.iter().any(|gamepad| gamepad.just_pressed(*button)));
        actions.just_pressed[action.index()] =
            from_keyboard || from_gamepad || menu_stick_edge(action, began);
    }
    if !actions.navigation_read.swap(false, Ordering::Relaxed) {
        menu_repeat.mark_stale();
    }
    actions.repeated.fill(false);
    for action in MENU_DIRECTIONS {
        let target = BindingTarget::Menu(action);
        let held = bindings.keys(target).iter().any(|key| keys.pressed(*key))
            || bindings
                .buttons(target)
                .iter()
                .any(|button| gamepads.iter().any(|gamepad| gamepad.pressed(*button)))
            || menu_stick_held(action, stick);
        actions.repeated[action.index()] =
            menu_repeat.advance(action, held, actions.just_pressed(action), time.delta());
    }
    for action in MovementAction::ALL {
        let target = BindingTarget::Movement(action);
        let from_keyboard = bindings.keys(target).iter().any(|key| keys.pressed(*key));
        let from_gamepad = bindings
            .buttons(target)
            .iter()
            .any(|button| gamepads.iter().any(|gamepad| gamepad.pressed(*button)));
        actions.movement_pressed[action.index()] =
            from_keyboard || from_gamepad || movement_stick_held(action, stick);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .add_plugins(ActionInputPlugin);
        app
    }

    fn action_state(app: &App, action: AppAction) -> bool {
        app.world().resource::<ActionState>().just_pressed(action)
    }

    #[test]
    fn default_keyboard_bindings_are_exact() {
        let bindings = InputBindings::default();

        assert_eq!(
            bindings.keys(BindingTarget::Menu(AppAction::Back)),
            [KeyCode::Escape]
        );
        assert_eq!(
            bindings.keys(BindingTarget::Menu(AppAction::Confirm)),
            [KeyCode::Enter, KeyCode::Space, KeyCode::NumpadEnter]
        );
        assert_eq!(
            bindings.keys(BindingTarget::Menu(AppAction::Up)),
            [KeyCode::ArrowUp]
        );
        assert_eq!(
            bindings.keys(BindingTarget::Menu(AppAction::Down)),
            [KeyCode::ArrowDown]
        );
        assert_eq!(
            bindings.keys(BindingTarget::Menu(AppAction::Left)),
            [KeyCode::ArrowLeft]
        );
        assert_eq!(
            bindings.keys(BindingTarget::Menu(AppAction::Right)),
            [KeyCode::ArrowRight]
        );
        assert_eq!(
            bindings.keys(BindingTarget::Menu(AppAction::Travel)),
            [KeyCode::KeyT]
        );
        assert_eq!(
            bindings.keys(BindingTarget::Movement(MovementAction::Up)),
            [KeyCode::ArrowUp]
        );
        assert_eq!(
            bindings.keys(BindingTarget::Movement(MovementAction::Left)),
            [KeyCode::ArrowLeft]
        );
        assert_eq!(
            bindings.keys(BindingTarget::Movement(MovementAction::Down)),
            [KeyCode::ArrowDown]
        );
        assert_eq!(
            bindings.keys(BindingTarget::Movement(MovementAction::Right)),
            [KeyCode::ArrowRight]
        );
    }

    #[test]
    fn keyboard_presses_map_only_to_their_actions() {
        let cases = [
            (KeyCode::Escape, AppAction::Back),
            (KeyCode::Enter, AppAction::Confirm),
            (KeyCode::Space, AppAction::Confirm),
            (KeyCode::NumpadEnter, AppAction::Confirm),
            (KeyCode::ArrowUp, AppAction::Up),
            (KeyCode::ArrowDown, AppAction::Down),
            (KeyCode::ArrowLeft, AppAction::Left),
            (KeyCode::ArrowRight, AppAction::Right),
            (KeyCode::KeyT, AppAction::Travel),
        ];

        for (key, expected) in cases {
            let mut app = action_app();
            app.update();
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(key);
            app.update();

            for action in AppAction::ALL {
                assert_eq!(
                    action_state(&app, action),
                    action == expected,
                    "{key:?} should map only to {expected:?}"
                );
            }
        }

        let mut app = action_app();
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyA);
        app.update();
        assert!(
            AppAction::ALL
                .into_iter()
                .all(|action| !action_state(&app, action))
        );
    }

    #[test]
    fn held_keys_do_not_retrigger_but_release_and_repress_does() {
        let mut app = action_app();
        app.update();

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        app.update();
        assert!(action_state(&app, AppAction::Confirm));

        // `InputPlugin` clears edge states before processing each platform input frame. The
        // headless action-map fixture injects directly into ButtonInput, so perform that normal
        // frame-boundary step explicitly before checking a held key.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.update();
        assert!(!action_state(&app, AppAction::Confirm));

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::Enter);
        app.update();
        assert!(!action_state(&app, AppAction::Confirm));

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        app.update();
        assert!(action_state(&app, AppAction::Confirm));
    }

    #[test]
    fn horizontal_navigation_is_edge_triggered_and_prefers_left() {
        let mut app = action_app();
        app.update();
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::ArrowLeft);
            keys.press(KeyCode::ArrowRight);
        }
        app.update();
        let actions = app.world().resource::<ActionState>();
        assert_eq!(actions.menu_navigation_horizontal(), Some(-1));
        // Vertical navigation is untouched by a horizontal press, so a target picker can consult
        // both without one swallowing the other.
        assert_eq!(actions.menu_navigation(), None);

        // Holding a direction must not walk the pool: this is menu navigation, not world movement.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.update();
        assert_eq!(
            app.world()
                .resource::<ActionState>()
                .menu_navigation_horizontal(),
            None
        );

        let mut app = action_app();
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowRight);
        app.update();
        assert_eq!(
            app.world()
                .resource::<ActionState>()
                .menu_navigation_horizontal(),
            Some(1)
        );
    }

    /// Runs one frame with `key` held, then reads the frame as a listening menu would.
    fn held_frame(app: &mut App, read: bool) -> (Option<isize>, Option<isize>, bool) {
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        let actions = app.world().resource::<ActionState>();
        if read {
            (
                actions.menu_navigation(),
                actions.menu_navigation_horizontal(),
                actions.just_pressed(AppAction::Right),
            )
        } else {
            (None, None, actions.just_pressed(AppAction::Right))
        }
    }

    fn repeat_app() -> App {
        let mut app = action_app();
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_millis(50),
        ));
        app.update();
        app
    }

    #[test]
    fn a_held_direction_repeats_after_a_delay_while_a_menu_listens() {
        let mut app = repeat_app();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowDown);
        let steps: Vec<_> = (0..16).map(|_| held_frame(&mut app, true).0).collect();

        // The press itself, a 400 ms pause, then a step each time another 90 ms has passed —
        // every other 50 ms frame.
        let mut expected = vec![Some(1)];
        expected.extend([None; 7]);
        expected.extend([Some(1), None].repeat(4));
        assert_eq!(steps, expected);

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::ArrowDown);
        assert_eq!(held_frame(&mut app, true).0, None);
    }

    #[test]
    fn a_held_right_repeats_the_cursor_but_never_the_press() {
        let mut app = repeat_app();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowRight);
        let frames: Vec<_> = (0..12).map(|_| held_frame(&mut app, true)).collect();

        assert!(frames[0].2, "the press itself is a Right edge");
        assert!(frames[1..].iter().all(|frame| !frame.2));
        assert!(frames[8..].iter().any(|frame| frame.1 == Some(1)));
        assert!(frames.iter().all(|frame| frame.0.is_none()));
    }

    #[test]
    fn a_direction_held_while_no_menu_listens_never_repeats() {
        let mut app = repeat_app();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowDown);
        // Walking: the press lands in the world, which never reads menu navigation.
        held_frame(&mut app, false);
        held_frame(&mut app, false);
        // A menu opens under the still-held key and listens from now on.
        assert!((0..20).all(|_| held_frame(&mut app, true).0.is_none()));

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::ArrowDown);
        held_frame(&mut app, true);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowDown);
        assert_eq!(held_frame(&mut app, true).0, Some(1));
    }

    #[test]
    fn simultaneous_opposite_navigation_prefers_up() {
        let mut app = action_app();
        app.update();
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::ArrowUp);
            keys.press(KeyCode::ArrowDown);
        }
        app.update();

        let actions = app.world().resource::<ActionState>();
        assert!(actions.just_pressed(AppAction::Up));
        assert!(actions.just_pressed(AppAction::Down));
        assert_eq!(actions.menu_navigation(), Some(-1));
        assert_eq!(actions.movement(), None);
    }

    #[test]
    fn simultaneous_perpendicular_input_resolves_all_four_diagonals() {
        use crate::scenario_spatial::EightWayDirection;

        for (keys, expected) in [
            (
                [KeyCode::ArrowUp, KeyCode::ArrowRight],
                EightWayDirection::UpRight,
            ),
            (
                [KeyCode::ArrowDown, KeyCode::ArrowRight],
                EightWayDirection::DownRight,
            ),
            (
                [KeyCode::ArrowDown, KeyCode::ArrowLeft],
                EightWayDirection::DownLeft,
            ),
            (
                [KeyCode::ArrowUp, KeyCode::ArrowLeft],
                EightWayDirection::UpLeft,
            ),
        ] {
            let mut app = action_app();
            app.update();
            {
                let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
                for key in keys {
                    input.press(key);
                }
            }
            app.update();

            assert_eq!(
                app.world().resource::<ActionState>().movement(),
                Some(expected)
            );
        }
    }

    #[test]
    fn held_direction_remains_active_and_combines_with_a_second_direction() {
        use crate::scenario_spatial::EightWayDirection;

        let mut app = action_app();
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowUp);
        app.update();
        assert_eq!(
            app.world().resource::<ActionState>().movement(),
            Some(EightWayDirection::Up)
        );

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.update();
        assert_eq!(
            app.world().resource::<ActionState>().movement(),
            Some(EightWayDirection::Up)
        );

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowRight);
        app.update();
        assert_eq!(
            app.world().resource::<ActionState>().movement(),
            Some(EightWayDirection::UpRight)
        );

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.update();
        assert_eq!(
            app.world().resource::<ActionState>().movement(),
            Some(EightWayDirection::UpRight)
        );
    }

    #[test]
    fn simultaneous_confirm_keys_still_produce_one_confirm_action() {
        let mut app = action_app();
        app.update();
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::Enter);
            keys.press(KeyCode::Space);
        }
        app.update();

        let actions = app.world().resource::<ActionState>();
        assert!(actions.just_pressed(AppAction::Confirm));
        assert_eq!(actions.just_pressed.len(), AppAction::ALL.len());
        assert_eq!(
            AppAction::ALL
                .into_iter()
                .filter(|action| actions.just_pressed(*action))
                .count(),
            1,
            "two physical confirm keys must collapse to one semantic action"
        );
    }

    #[test]
    fn a_rebound_key_takes_effect_on_the_next_frame() {
        let mut app = action_app();
        app.update();

        app.world_mut()
            .resource_mut::<PlayerOptions>()
            .bindings
            .bind_key(BindingTarget::Menu(AppAction::Travel), KeyCode::KeyG)
            .expect("a free key");
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyG);
        app.update();

        assert!(action_state(&app, AppAction::Travel));
    }

    #[test]
    fn stick_direction_flips_the_gamepad_y_axis_to_screen_space() {
        // A gamepad reports +Y as up; the port's movement vector reports +1 as down.
        assert_eq!(
            StickDirection::from_stick(Vec2::new(0.0, 1.0)),
            StickDirection {
                horizontal: 0,
                vertical: -1
            }
        );
        assert_eq!(
            StickDirection::from_stick(Vec2::new(0.0, -1.0)),
            StickDirection {
                horizontal: 0,
                vertical: 1
            }
        );
    }

    #[test]
    fn a_stick_inside_the_deadzone_reports_center() {
        for drift in [0.0, 0.2, 0.39, -0.39] {
            assert_eq!(
                StickDirection::from_stick(Vec2::splat(drift)),
                StickDirection::default(),
                "{drift} is resting drift, not a direction"
            );
        }
        assert_eq!(
            StickDirection::from_stick(Vec2::new(0.4, 0.0)),
            StickDirection {
                horizontal: 1,
                vertical: 0
            },
            "the threshold itself counts as a push"
        );
    }

    #[test]
    fn a_held_stick_produces_one_menu_edge_and_walks_every_frame() {
        let mut navigation = StickNavigation::default();
        let down = StickDirection {
            horizontal: 0,
            vertical: 1,
        };

        // Frame one: the stick leaves center, so the menu takes one step.
        assert_eq!(navigation.advance(down), down);
        // Frames two and three: still held. The menu must not step again, but walking continues,
        // which is why the level (`down`), not the edge, drives movement.
        assert_eq!(navigation.advance(down), StickDirection::default());
        assert_eq!(navigation.advance(down), StickDirection::default());
        assert!(movement_stick_held(MovementAction::Down, down));

        // Returning to center and pushing again is a fresh edge.
        assert_eq!(
            navigation.advance(StickDirection::default()),
            StickDirection::default()
        );
        assert_eq!(navigation.advance(down), down);
    }

    #[test]
    fn reversing_the_stick_without_passing_through_center_is_a_new_edge() {
        let mut navigation = StickNavigation::default();
        let left = StickDirection {
            horizontal: -1,
            vertical: 0,
        };
        let right = StickDirection {
            horizontal: 1,
            vertical: 0,
        };

        assert_eq!(navigation.advance(left), left);
        // A fast flick can skip the center sample entirely; that must still register.
        assert_eq!(navigation.advance(right), right);
    }

    #[test]
    fn a_diagonal_push_edges_only_the_axis_that_changed() {
        let mut navigation = StickNavigation::default();
        let down = StickDirection {
            horizontal: 0,
            vertical: 1,
        };
        let down_right = StickDirection {
            horizontal: 1,
            vertical: 1,
        };

        assert_eq!(navigation.advance(down), down);
        assert_eq!(
            navigation.advance(down_right),
            StickDirection {
                horizontal: 1,
                vertical: 0
            },
            "vertical was already held, so only Right begins"
        );
    }

    #[test]
    fn menu_stick_edges_reach_only_the_directional_actions() {
        let down_right = StickDirection {
            horizontal: 1,
            vertical: 1,
        };

        assert!(menu_stick_edge(AppAction::Down, down_right));
        assert!(menu_stick_edge(AppAction::Right, down_right));
        assert!(!menu_stick_edge(AppAction::Up, down_right));
        // A stick can never confirm or cancel: those need a real button, so that leaning on the
        // stick cannot dismiss a dialogue.
        assert!(!menu_stick_edge(AppAction::Confirm, down_right));
        assert!(!menu_stick_edge(AppAction::Back, down_right));
        assert!(!menu_stick_edge(AppAction::Travel, down_right));
    }
}
