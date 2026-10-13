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
    .map_err(|error| error.to_string())
    .and_then(|envelope| {
        store
            .write(state.selected, &envelope, overwrite, &balance)
            .map_err(|error| error.to_string())
    });
    match result {
        Ok(_) => {
            state.mode = FieldMenuMode::Browse;
            state.message = format!("{} saved.", slot_label(state.selected));
            saves.request_refresh();
        }
        Err(error) => state.message = error,
    }
}

pub(super) fn slot_label(index: usize) -> String {
    format!("Slot {index:02}")
}
