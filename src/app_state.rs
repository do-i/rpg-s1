use bevy::prelude::States;

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
}

#[cfg(test)]
mod tests {
    use super::AppState;

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
        ];

        assert_eq!(
            states.map(|state| format!("{state:?}")).join(","),
            "Boot,Title,NameEntry,Dialogue,World,Battle,FieldMenu,PostBattle,GameOver"
        );
    }
}
