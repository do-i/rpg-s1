use bevy::{input::InputSystems, prelude::*};

use crate::input_record::NormalizedAction;

/// A semantic menu action shared by application-shell screens.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum AppAction {
    Back,
    Confirm,
    Up,
    Down,
    Left,
    Right,
}

impl AppAction {
    const ALL: [Self; 6] = [
        Self::Back,
        Self::Confirm,
        Self::Up,
        Self::Down,
        Self::Left,
        Self::Right,
    ];

    const fn index(self) -> usize {
        match self {
            Self::Back => 0,
            Self::Confirm => 1,
            Self::Up => 2,
            Self::Down => 3,
            Self::Left => 4,
            Self::Right => 5,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum MovementAction {
    Up,
    Left,
    Down,
    Right,
}

impl MovementAction {
    const ALL: [Self; 4] = [Self::Up, Self::Left, Self::Down, Self::Right];

    const fn index(self) -> usize {
        match self {
            Self::Up => 0,
            Self::Left => 1,
            Self::Down => 2,
            Self::Right => 3,
        }
    }
}

/// Keyboard bindings for semantic application-shell actions.
#[derive(Resource)]
pub(crate) struct ActionMap {
    bindings: [Vec<KeyCode>; 6],
    movement_bindings: [Vec<KeyCode>; 4],
}

impl Default for ActionMap {
    fn default() -> Self {
        Self {
            bindings: [
                vec![KeyCode::Escape],
                vec![KeyCode::Enter, KeyCode::Space, KeyCode::NumpadEnter],
                vec![KeyCode::ArrowUp],
                vec![KeyCode::ArrowDown],
                vec![KeyCode::ArrowLeft],
                vec![KeyCode::ArrowRight],
            ],
            movement_bindings: [
                vec![KeyCode::ArrowUp],
                vec![KeyCode::ArrowLeft],
                vec![KeyCode::ArrowDown],
                vec![KeyCode::ArrowRight],
            ],
        }
    }
}

impl ActionMap {
    fn bindings(&self, action: AppAction) -> &[KeyCode] {
        &self.bindings[action.index()]
    }

    fn movement_bindings(&self, action: MovementAction) -> &[KeyCode] {
        &self.movement_bindings[action.index()]
    }
}

/// Semantic actions that began during the current input frame.
#[derive(Resource, Default)]
pub(crate) struct ActionState {
    just_pressed: [bool; 6],
    movement_pressed: [bool; 4],
}

impl ActionState {
    pub(crate) fn just_pressed(&self, action: AppAction) -> bool {
        self.just_pressed[action.index()]
    }

    /// Resolves same-frame menu navigation to one movement.
    ///
    /// The Python menu consumes key-down events in platform arrival order, so pressing Up and
    /// Down in one frame has no source-defined simultaneous-key outcome. The Rust port fixes
    /// that ambiguity: Up wins. This prevents a conflicting pair from taking two menu steps and
    /// keeps the result independent of platform event ordering.
    pub(crate) fn menu_navigation(&self) -> Option<isize> {
        if self.just_pressed(AppAction::Up) {
            Some(-1)
        } else if self.just_pressed(AppAction::Down) {
            Some(1)
        } else {
            None
        }
    }

    /// Resolves same-frame horizontal menu navigation to one movement.
    ///
    /// The pinned engine's battle target picker treats Left as "previous" and Right as "next"
    /// alongside Up/Down (`battle_input.py:106-109`). Left wins a conflicting pair for the same
    /// reason Up does above: one keypress pair must never take two steps.
    pub(crate) fn menu_navigation_horizontal(&self) -> Option<isize> {
        if self.just_pressed(AppAction::Left) {
            Some(-1)
        } else if self.just_pressed(AppAction::Right) {
            Some(1)
        } else {
            None
        }
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
        ] {
            if self.just_pressed(action) {
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
        self.just_pressed.fill(false);
        self.movement_pressed.fill(false);
        for action in normalized {
            match action {
                NormalizedAction::Back => self.just_pressed[AppAction::Back.index()] = true,
                NormalizedAction::Confirm => self.just_pressed[AppAction::Confirm.index()] = true,
                NormalizedAction::MenuUp => self.just_pressed[AppAction::Up.index()] = true,
                NormalizedAction::MenuDown => self.just_pressed[AppAction::Down.index()] = true,
                NormalizedAction::MenuLeft => self.just_pressed[AppAction::Left.index()] = true,
                NormalizedAction::MenuRight => self.just_pressed[AppAction::Right.index()] = true,
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

pub(crate) struct ActionInputPlugin;

impl Plugin for ActionInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActionMap>()
            .init_resource::<ActionState>()
            .add_systems(PreUpdate, update_action_state.after(InputSystems));
    }
}

pub(crate) fn update_action_state(
    keys: Res<ButtonInput<KeyCode>>,
    map: Res<ActionMap>,
    mut actions: ResMut<ActionState>,
) {
    for action in AppAction::ALL {
        actions.just_pressed[action.index()] = map
            .bindings(action)
            .iter()
            .any(|key| keys.just_pressed(*key));
    }
    for action in MovementAction::ALL {
        actions.movement_pressed[action.index()] = map
            .movement_bindings(action)
            .iter()
            .any(|key| keys.pressed(*key));
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
        let map = ActionMap::default();

        assert_eq!(map.bindings(AppAction::Back), [KeyCode::Escape]);
        assert_eq!(
            map.bindings(AppAction::Confirm),
            [KeyCode::Enter, KeyCode::Space, KeyCode::NumpadEnter]
        );
        assert_eq!(map.bindings(AppAction::Up), [KeyCode::ArrowUp]);
        assert_eq!(map.bindings(AppAction::Down), [KeyCode::ArrowDown]);
        assert_eq!(map.bindings(AppAction::Left), [KeyCode::ArrowLeft]);
        assert_eq!(map.bindings(AppAction::Right), [KeyCode::ArrowRight]);
        assert_eq!(
            map.movement_bindings(MovementAction::Up),
            [KeyCode::ArrowUp]
        );
        assert_eq!(
            map.movement_bindings(MovementAction::Left),
            [KeyCode::ArrowLeft]
        );
        assert_eq!(
            map.movement_bindings(MovementAction::Down),
            [KeyCode::ArrowDown]
        );
        assert_eq!(
            map.movement_bindings(MovementAction::Right),
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
}
