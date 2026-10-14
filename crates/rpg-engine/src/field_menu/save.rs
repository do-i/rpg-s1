//! Save page actions: choosing a slot, confirming an overwrite, and writing the save.

use bevy::prelude::*;

use super::{
    input::MenuInput,
    state::{FieldMenuMode, FieldMenuState},
};
use crate::{
    game_state::GameState,
    runtime_map::RuntimeMapId,
    save_data::NativeSaveEnvelope,
    save_store::{FIRST_PLAYER_SLOT, LAST_PLAYER_SLOT, SaveStore, unix_timestamp_now},
    save_ui::SaveSlotCatalog,
};

pub(super) fn handle_input(
    state: &mut ResMut<'_, FieldMenuState>,
    game: &mut ResMut<'_, GameState>,
    input: &MenuInput<'_>,
    store: &SaveStore,
    saves: &mut ResMut<'_, SaveSlotCatalog>,
    time: &Time<Real>,
) {
    match state.mode {
        FieldMenuMode::Browse => {
            if let Some(delta) = input.vertical {
                state.selected = (state.selected as isize + delta)
                    .clamp(FIRST_PLAYER_SLOT as isize, LAST_PLAYER_SLOT as isize)
                    as usize;
            }
            if input.confirm {
                let Some(slot) = saves.slots().get(state.selected) else {
                    state.message = "Save slots are still loading.".to_owned();
                    return;
                };
                if slot.is_empty() {
                    save_game(game, store, saves, time, state, false);
                } else {
                    state.mode = FieldMenuMode::SaveConfirm;
                    state.message = format!("Overwrite {}?", slot.label());
                }
            }
        }
        FieldMenuMode::SaveConfirm => {
            if input.keys.just_pressed(KeyCode::KeyN) {
                state.mode = FieldMenuMode::Browse;
                state.message.clear();
            } else if input.keys.just_pressed(KeyCode::KeyY) || input.confirm {
                save_game(game, store, saves, time, state, true);
            }
        }
        _ => {}
    }
}

pub(super) fn save_game(
    game: &mut GameState,
    store: &SaveStore,
    saves: &mut SaveSlotCatalog,
    time: &Time<Real>,
    state: &mut FieldMenuState,
    overwrite: bool,
) {
    let Some(context) = saves.context() else {
        state.message = "Scenario data is still loading.".to_owned();
        return;
    };
    let scenario_id = context.scenario_id.to_owned();
    let scenario_version = context.scenario_version.to_owned();
    let balance = context.balance.clone();
    game.playtime_mut().commit_session(time.elapsed());
    let location = game
        .map()
        .current()
        .map_or("Unknown", RuntimeMapId::as_str)
        .to_owned();
    let timestamp = match unix_timestamp_now() {
        Ok(timestamp) => timestamp,
        Err(error) => {
            state.message = error.to_string();
            return;
        }
    };
    let result = NativeSaveEnvelope::from_game_state(
        game,
        scenario_id,
        scenario_version,
        timestamp,
        location,
    )
    .map_err(|error| (error.to_string(), false))
    .and_then(|envelope| {
        store
            .write(state.selected, &envelope, overwrite, &balance)
            .map_err(|error| (error.to_string(), error.published()))
    });
    match result {
        Ok(_) => {
            state.mode = FieldMenuMode::Browse;
            state.message = format!("{} saved.", slot_label(state.selected));
            saves.request_refresh();
        }
        // The slot holds the new save even though durability is unconfirmed, so the list must
        // show it and the menu must not imply the old save survived.
        Err((error, true)) => {
            state.mode = FieldMenuMode::Browse;
            state.message = format!(
                "{} saved, but disk durability could not be confirmed: {error}.",
                slot_label(state.selected)
            );
            saves.request_refresh();
        }
        Err((error, false)) => state.message = error,
    }
}

pub(super) fn slot_label(index: usize) -> String {
    format!("Slot {index:02}")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::save_data::tests::{fixture_balance, fixture_game};

    #[test]
    fn a_save_published_without_confirmed_durability_says_so_and_refreshes_the_list() {
        let root = std::env::temp_dir().join(format!(
            "rpg-s1-field-save-durability-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let mut store = SaveStore::new(root.clone());
        let mut saves = SaveSlotCatalog::ready_for_test("my_rpg_story", "1.0.0", fixture_balance());
        let mut game = fixture_game();
        let time = Time::<Real>::default();

        // Before publication: the slot stays empty and the overwrite prompt stays up.
        store.fail_operation = Some("write temporary save");
        let mut state = FieldMenuState {
            mode: FieldMenuMode::SaveConfirm,
            selected: 3,
            ..default()
        };
        saves.clear_refresh_for_test();
        save_game(&mut game, &store, &mut saves, &time, &mut state, true);
        assert_eq!(state.mode, FieldMenuMode::SaveConfirm);
        assert!(!saves.refresh_requested_for_test());
        assert!(!store.slot_path(3).unwrap().exists());

        // After publication: the slot holds the save, so the list refreshes and says so.
        store.fail_operation = Some("sync save directory");
        save_game(&mut game, &store, &mut saves, &time, &mut state, true);
        assert_eq!(state.mode, FieldMenuMode::Browse);
        assert!(
            state
                .message
                .starts_with("Slot 03 saved, but disk durability could not be confirmed"),
            "{}",
            state.message
        );
        assert!(saves.refresh_requested_for_test());
        assert!(store.slot_path(3).unwrap().exists());
        fs::remove_dir_all(&root).unwrap();
    }
}
