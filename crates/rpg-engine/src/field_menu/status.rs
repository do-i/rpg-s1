//! Status chooser, row selection, and confirmed field appearance changes.

use super::{
    input::MenuInput,
    main_deck::apply_character_choice,
    member_at, member_id_at,
    state::{FieldMenuMode, FieldMenuState, StatusPage, cycle_member, wrapped_or_zero},
};
use crate::{
    game_state::GameState, scenario_inventory::ScenarioInventory, scenario_party::PartyRow,
};
use bevy::prelude::*;

pub(super) const STATUS_ROWS: [PartyRow; 2] = [PartyRow::Front, PartyRow::Back];

pub(super) fn handle_input(
    state: &mut ResMut<'_, FieldMenuState>,
    game: &mut ResMut<'_, GameState>,
    input: &MenuInput<'_>,
    inventory: Option<&ScenarioInventory>,
) {
    if state.mode != FieldMenuMode::Browse {
        return;
    }
    let member_delta = if input.keys.just_pressed(KeyCode::KeyQ) {
        Some(-1)
    } else if input.keys.just_pressed(KeyCode::KeyE) {
        Some(1)
    } else if state.status_page == StatusPage::Roster {
        input.page
    } else {
        None
    };
    if let Some(delta) = member_delta {
        cycle_member(state, game.party().len(), delta);
        state.status_page = StatusPage::Roster;
        state.message.clear();
        return;
    }
    if input.keys.just_pressed(KeyCode::Tab) || input.vertical.is_some() {
        let delta = input.vertical.unwrap_or_else(|| {
            if input.keys.pressed(KeyCode::ShiftLeft) || input.keys.pressed(KeyCode::ShiftRight) {
                -1
            } else {
                1
            }
        });
        let index = match state.status_page {
            StatusPage::Roster => 0,
            StatusPage::Position => 1,
            StatusPage::Sprite => 2,
        };
        focus_status_control(state, game, inventory, wrapped_or_zero(index, 3, delta));
        return;
    }
    match state.status_page {
        StatusPage::Roster if input.confirm => focus_status_control(state, game, inventory, 1),
        StatusPage::Position => {
            if let Some(delta) = input.page {
                state.selected = wrapped_or_zero(state.selected, STATUS_ROWS.len(), delta);
            }
            if input.confirm {
                set_member_row(game, state);
            }
        }
        StatusPage::Sprite => {
            if let Some(delta) = input.page {
                state.sprite_choice = wrapped_or_zero(
                    state.sprite_choice,
                    status_sprite_count(game, state, inventory),
                    delta,
                );
            }
            if input.confirm {
                state.message = match apply_character_choice(
                    game,
                    state.member_index,
                    state.sprite_choice,
                    inventory,
                ) {
                    Ok(name) => format!("Now controlling {name}."),
                    Err(message) => message.to_owned(),
                };
            }
        }
        StatusPage::Roster => {}
    }
}

fn focus_status_control(
    state: &mut FieldMenuState,
    game: &GameState,
    inventory: Option<&ScenarioInventory>,
    index: usize,
) {
    state.message.clear();
    state.status_page = match index {
        1 => {
            state.selected = member_at(game, state.member_index)
                .and_then(|member| STATUS_ROWS.iter().position(|row| *row == member.row()))
                .unwrap_or_default();
            StatusPage::Position
        }
        2 => {
            state.sprite_choice =
                inventory.map_or(0, |inventory| inventory.sprite_variant_index(game));
            StatusPage::Sprite
        }
        _ => StatusPage::Roster,
    };
}

pub(super) fn status_sprite_count(
    game: &GameState,
    state: &FieldMenuState,
    inventory: Option<&ScenarioInventory>,
) -> usize {
    if member_at(game, state.member_index).is_some_and(|member| member.is_protagonist()) {
        inventory.map_or(1, |inventory| {
            inventory.protagonist_sprite_variants.len().max(1)
        })
    } else {
        1
    }
}

pub(super) fn status_hint(state: &FieldMenuState) -> &'static str {
    match state.status_page {
        StatusPage::Roster => {
            "Q/E  CHARACTER    LEFT/RIGHT  SELECT    TAB / ENTER  POSITION    ESC  BACK"
        }
        StatusPage::Position => {
            "Q/E  CHARACTER    LEFT/RIGHT  ROW    ENTER  SET    TAB  FIELD SPRITE    ESC  CHARACTERS"
        }
        StatusPage::Sprite => {
            "Q/E  CHARACTER    LEFT/RIGHT  SPRITE    ENTER  USE ON MAP    TAB  CHARACTERS    ESC  CANCEL"
        }
    }
}

/// Applies the Position picker's selection to the focused member.
///
/// Mirrors `status_scene._confirm_position`: re-picking the row the member already holds is a
/// no-op rather than an error, and the picker stays open either way.
pub(super) fn set_member_row(game: &mut GameState, state: &mut FieldMenuState) {
    let Some(row) = STATUS_ROWS.get(state.selected).copied() else {
        return;
    };
    let Some(member_id) = member_id_at(game, state.member_index).map(ToOwned::to_owned) else {
        return;
    };
    let name = member_at(game, state.member_index)
        .map_or_else(|| member_id.clone(), |member| member.name().to_owned());
    match game.party_mut().set_row(&member_id, row) {
        Ok(previous) if previous == row => {
            state.message = format!("{name} already holds the {} row.", row_label(row));
        }
        Ok(_) => {
            state.message = format!("{name} moved to the {} row.", row_label(row));
        }
        Err(_) => {
            state.message = "That party member is no longer available.".to_owned();
        }
    }
}

/// Player-facing name for a battle row. Keeps `{:?}` out of the status page.
pub(super) const fn row_label(row: PartyRow) -> &'static str {
    match row {
        PartyRow::Front => "Front",
        PartyRow::Back => "Back",
    }
}

#[cfg(test)]
mod tests {
    use super::super::state::FieldMenuScreen;
    use super::*;
    use crate::save_data::tests::fixture_game;

    #[test]
    fn confirming_a_row_moves_the_member_and_repeating_it_reports_no_change() {
        let mut game = fixture_game();
        let member_id = game
            .party()
            .members()
            .next()
            .expect("fixture party has members")
            .id()
            .to_owned();
        assert_eq!(game.party().row_of(&member_id), Some(PartyRow::Front));

        let mut state = FieldMenuState::default();
        state.open(FieldMenuScreen::Status);
        state.status_page = StatusPage::Position;
        state.selected = 1;

        set_member_row(&mut game, &mut state);
        assert_eq!(game.party().row_of(&member_id), Some(PartyRow::Back));
        assert!(state.message.contains("moved to the Back row"));

        set_member_row(&mut game, &mut state);
        assert_eq!(game.party().row_of(&member_id), Some(PartyRow::Back));
        assert!(state.message.contains("already holds the Back row"));

        state.selected = 0;
        set_member_row(&mut game, &mut state);
        assert_eq!(game.party().row_of(&member_id), Some(PartyRow::Front));
    }
}
