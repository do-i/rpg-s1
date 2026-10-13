//! Status page navigation: the roster, a member's details, and the row picker.

use bevy::prelude::*;

use super::{
    input::MenuInput,
    member_at, member_id_at,
    state::{FieldMenuMode, FieldMenuScreen, FieldMenuState, StatusPage, cycle_member, wrapped},
};
use crate::{game_state::GameState, scenario_party::PartyRow};

pub(super) const STATUS_CATEGORIES: [&str; 2] = ["Spells", "Position"];
/// Index of `Position` within [`STATUS_CATEGORIES`].
pub(super) const STATUS_POSITION_CATEGORY: usize = 1;
/// Battle rows in picker order, matching Python's `status_renderer.ROWS`.
pub(super) const STATUS_ROWS: [PartyRow; 2] = [PartyRow::Front, PartyRow::Back];

pub(super) fn handle_input(
    state: &mut ResMut<'_, FieldMenuState>,
    game: &mut ResMut<'_, GameState>,
    input: &MenuInput<'_>,
) {
    if state.mode != FieldMenuMode::Browse {
        return;
    }
    match state.status_page {
        // Party column: Up/Down picks the member, Right or Enter opens their details.
        StatusPage::Roster => {
            if let Some(delta) = input.vertical {
                cycle_member(state, game.party().len(), delta);
            }
            if (input.horizontal == Some(1) || input.confirm) && !game.party().is_empty() {
                state.status_page = StatusPage::Details;
                state.selected = 0;
            }
        }
        StatusPage::Details => {
            if input.horizontal == Some(-1) {
                state.back();
                return;
            }
            if let Some(delta) = input.vertical {
                state.selected = wrapped(state.selected, STATUS_CATEGORIES.len(), delta);
            }
            if input.confirm {
                if state.selected == STATUS_POSITION_CATEGORY {
                    let current = member_at(game, state.member_index).map(|member| member.row());
                    state.status_page = StatusPage::Position;
                    state.selected = current
                        .and_then(|row| STATUS_ROWS.iter().position(|entry| *entry == row))
                        .unwrap_or_default();
                    state.message.clear();
                } else {
                    // The spellbook is its own screen in this port; carry the member across.
                    state.screen = FieldMenuScreen::Spells;
                    state.status_page = StatusPage::Roster;
                    state.selected = 0;
                    state.message.clear();
                }
            }
        }
        StatusPage::Position => {
            if input.horizontal == Some(-1) {
                state.back();
                return;
            }
            if let Some(delta) = input.vertical {
                state.selected = wrapped(state.selected, STATUS_ROWS.len(), delta);
            }
            if input.confirm {
                set_member_row(game, state);
            }
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
