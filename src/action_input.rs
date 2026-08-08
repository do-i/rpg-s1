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
