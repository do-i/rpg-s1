use bevy::prelude::*;

#[derive(States, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AppState {
    Boot,
    Title,
    NameEntry,
    Dialogue,
    World,
    Battle,
    FieldMenu,
    PostBattle,
    GameOver,
    Credits,
}

/// Requests a top-level application state change without constructing the target scene.
#[derive(Message, Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppStateTransitionRequest {
    target: AppState,
}

impl AppStateTransitionRequest {
    pub fn new(target: AppState) -> Self {
        Self { target }
    }
}

pub struct AppStateTransitionPlugin;

impl Plugin for AppStateTransitionPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<AppStateTransitionRequest>()
            .add_systems(PostUpdate, apply_app_state_transition_requests);
    }
}

/// Applies at most one transition per frame after draining every request.
///
/// When several systems request a transition in the same frame, the last request in message
/// insertion order wins. With no requests, `NextState<AppState>` is left unchanged.
fn apply_app_state_transition_requests(
    mut requests: MessageReader<AppStateTransitionRequest>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if let Some(request) = requests.read().last() {
        next_state.set(request.target);
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::{App, AppExtStates, MinimalPlugins, State};
    use bevy::state::app::StatesPlugin;

    use super::{AppState, AppStateTransitionPlugin, AppStateTransitionRequest};

    fn transition_app(initial_state: AppState) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(StatesPlugin)
            .insert_state(initial_state)
            .add_plugins(AppStateTransitionPlugin);
        app
    }

    fn advance_transition(app: &mut App) {
        app.update();
        app.update();
    }

    #[test]
    fn app_state_contract_has_all_top_level_states() {
        let states = [
            AppState::Boot,
            AppState::Title,
            AppState::NameEntry,
            AppState::Dialogue,
            AppState::World,
            AppState::Battle,
            AppState::FieldMenu,
            AppState::PostBattle,
            AppState::GameOver,
            AppState::Credits,
        ];

        assert_eq!(
            states.map(|state| format!("{state:?}")).join(","),
            "Boot,Title,NameEntry,Dialogue,World,Battle,FieldMenu,PostBattle,GameOver,Credits"
        );
    }

    #[test]
    fn transition_request_changes_state_through_central_system() {
        let mut app = transition_app(AppState::Title);

        app.world_mut()
            .write_message(AppStateTransitionRequest::new(AppState::NameEntry));
        advance_transition(&mut app);

        assert_eq!(
            app.world().resource::<State<AppState>>().get(),
            &AppState::NameEntry
        );
    }

    #[test]
    fn no_transition_request_leaves_state_unchanged() {
        let mut app = transition_app(AppState::Title);

        advance_transition(&mut app);

        assert_eq!(
            app.world().resource::<State<AppState>>().get(),
            &AppState::Title
        );
    }

    #[test]
    fn last_same_frame_transition_request_wins() {
        let mut app = transition_app(AppState::Title);

        app.world_mut()
            .write_message(AppStateTransitionRequest::new(AppState::World));
        app.world_mut()
            .write_message(AppStateTransitionRequest::new(AppState::Battle));
        advance_transition(&mut app);

        assert_eq!(
            app.world().resource::<State<AppState>>().get(),
            &AppState::Battle
        );
    }
}
