use bevy::{input::InputSystems, prelude::*};

/// A semantic menu action shared by application-shell screens.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum AppAction {
    Back,
    Confirm,
    Up,
    Down,
}

impl AppAction {
    const ALL: [Self; 4] = [Self::Back, Self::Confirm, Self::Up, Self::Down];

    const fn index(self) -> usize {
        match self {
            Self::Back => 0,
            Self::Confirm => 1,
            Self::Up => 2,
            Self::Down => 3,
        }
    }
}

/// Keyboard bindings for semantic application-shell actions.
#[derive(Resource)]
pub(crate) struct ActionMap {
    bindings: [Vec<KeyCode>; 4],
}

impl Default for ActionMap {
    fn default() -> Self {
        Self {
            bindings: [
                vec![KeyCode::Escape],
                vec![KeyCode::Enter, KeyCode::Space],
                vec![KeyCode::ArrowUp],
                vec![KeyCode::ArrowDown],
            ],
        }
    }
}

impl ActionMap {
    fn bindings(&self, action: AppAction) -> &[KeyCode] {
        &self.bindings[action.index()]
    }
}

/// Semantic actions that began during the current input frame.
#[derive(Resource, Default)]
pub(crate) struct ActionState {
    just_pressed: [bool; 4],
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
}

pub(crate) struct ActionInputPlugin;

impl Plugin for ActionInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActionMap>()
            .init_resource::<ActionState>()
            .add_systems(PreUpdate, update_action_state.after(InputSystems));
    }
}

fn update_action_state(
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
            [KeyCode::Enter, KeyCode::Space]
        );
        assert_eq!(map.bindings(AppAction::Up), [KeyCode::ArrowUp]);
        assert_eq!(map.bindings(AppAction::Down), [KeyCode::ArrowDown]);
    }

    #[test]
    fn keyboard_presses_map_only_to_their_actions() {
        let cases = [
            (KeyCode::Escape, AppAction::Back),
            (KeyCode::Enter, AppAction::Confirm),
            (KeyCode::Space, AppAction::Confirm),
            (KeyCode::ArrowUp, AppAction::Up),
            (KeyCode::ArrowDown, AppAction::Down),
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
