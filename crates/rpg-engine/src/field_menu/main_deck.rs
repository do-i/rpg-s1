//! The main command deck and the modals it opens directly: character switch and quit.

use bevy::prelude::*;

use super::{
    input::MenuInput,
    member_at, member_id_at,
    state::{FieldMenuMode, FieldMenuScreen, FieldMenuState, wrapped, wrapped_or_zero},
};
use crate::{
    game_state::GameState, save_store::FIRST_PLAYER_SLOT, scenario_inventory::ScenarioInventory,
};

/// Rows per column on the main command deck; the deck grows a column as commands are added.
pub(super) const MAIN_COMMAND_ROWS: usize = 4;
pub(super) const QUIT_COMMAND_INDEX: usize = 3;
pub(super) const SAVE_COMMAND_INDEX: usize = 4;
pub(super) const CHARACTER_COMMAND_INDEX: usize = 7;

#[derive(Clone, Copy)]
pub(super) struct MainCommand {
    pub(super) label: &'static str,
    /// Letter badge drawn when the scenario ships no icon sheet.
    pub(super) badge: &'static str,
    /// Cell in the scenario's `ui.menu_icons` sheet.
    pub(super) icon: usize,
    pub(super) description: &'static str,
    pub(super) screen: Option<FieldMenuScreen>,
}

pub(super) const MAIN_COMMANDS: [MainCommand; 11] = [
    MainCommand {
        label: "Status",
        badge: "ST",
        icon: 0,
        description: "review health, rows, and growth",
        screen: Some(FieldMenuScreen::Status),
    },
    MainCommand {
        label: "Spells",
        badge: "SP",
        icon: 1,
        description: "cast field magic and utilities",
        screen: Some(FieldMenuScreen::Spells),
    },
    MainCommand {
        label: "Items",
        badge: "IT",
        icon: 2,
        description: "use, sort, and inspect supplies",
        screen: Some(FieldMenuScreen::Items),
    },
    MainCommand {
        label: "Quit",
        badge: "QT",
        icon: 3,
        description: "exit the game to desktop",
        screen: None,
    },
    MainCommand {
        label: "Save",
        badge: "SV",
        icon: 4,
        description: "record the current journey",
        screen: Some(FieldMenuScreen::Save),
    },
    MainCommand {
        label: "Equipment",
        badge: "EQ",
        icon: 5,
        description: "tune gear and compare stats",
        screen: Some(FieldMenuScreen::Equipment),
    },
    MainCommand {
        label: "Quests",
        badge: "QU",
        icon: 6,
        description: "review active and completed quests",
        screen: Some(FieldMenuScreen::Quests),
    },
    MainCommand {
        label: "Character",
        badge: "CH",
        icon: 7,
        description: "choose your character and sprite",
        screen: None,
    },
    MainCommand {
        label: "Recipes",
        badge: "RC",
        icon: 8,
        description: "study known recipes and ingredients",
        screen: Some(FieldMenuScreen::Recipes),
    },
    MainCommand {
        label: "Battle",
        badge: "BT",
        icon: 9,
        description: "set each member to manual or auto",
        screen: Some(FieldMenuScreen::Battle),
    },
    MainCommand {
        label: "Settings",
        badge: "SE",
        icon: 10,
        description: "sound and other player preferences",
        screen: Some(FieldMenuScreen::Settings),
    },
];

pub(super) fn handle_input(
    state: &mut ResMut<'_, FieldMenuState>,
    game: &mut ResMut<'_, GameState>,
    input: &MenuInput<'_>,
    inventory: Option<&ScenarioInventory>,
    exit: &mut MessageWriter<'_, AppExit>,
) {
    match state.mode {
        FieldMenuMode::Browse => {
            if input.vertical.is_some() || input.horizontal.is_some() {
                state.selected =
                    stepped_main_command(state.selected, input.vertical, input.horizontal);
            }
            if input.confirm {
                if state.selected == CHARACTER_COMMAND_INDEX {
                    state.mode = FieldMenuMode::CharacterSwitch;
                    state.selected = controlled_member_index(game);
                    state.sprite_choice =
                        inventory.map_or(0, |inventory| inventory.sprite_variant_index(game));
                } else if state.selected == SAVE_COMMAND_INDEX {
                    state.screen = FieldMenuScreen::Save;
                    state.selected = FIRST_PLAYER_SLOT;
                } else if state.selected == QUIT_COMMAND_INDEX {
                    state.mode = FieldMenuMode::QuitConfirm;
                    state.message = "Exit to desktop? Unsaved progress will be lost.".to_owned();
                } else if let Some(screen) = main_command_screen(state.selected) {
                    state.screen = screen;
                    state.selected = 0;
                }
            }
        }
        FieldMenuMode::CharacterSwitch => {
            if let Some(delta) = input.vertical {
                state.selected = wrapped_or_zero(state.selected, game.party().len(), delta);
                state.sprite_choice =
                    inventory.map_or(0, |inventory| inventory.sprite_variant_index(game));
            }
            if let (Some(delta), Some(inventory)) = (input.page, inventory)
                && character_has_sprite_choices(game, state, Some(inventory))
            {
                state.sprite_choice = wrapped_or_zero(
                    state.sprite_choice,
                    inventory.protagonist_sprite_variants.len(),
                    delta,
                );
            }
            if input.confirm {
                switch_controlled_member(game, state, inventory);
            }
        }
        FieldMenuMode::QuitConfirm => {
            if input.keys.just_pressed(KeyCode::KeyN) {
                state.mode = FieldMenuMode::Browse;
                state.message.clear();
            } else if input.keys.just_pressed(KeyCode::KeyY) || input.confirm {
                state.close();
                exit.write(AppExit::Success);
            }
        }
        _ => {}
    }
}

/// Row of the member the player currently controls, so the switch overlay opens on them.
pub(super) fn controlled_member_index(game: &GameState) -> usize {
    game.party()
        .members()
        .position(|member| member.id() == game.controlled_member_id())
        .unwrap_or_default()
}

pub(super) fn character_has_sprite_choices(
    game: &GameState,
    state: &FieldMenuState,
    inventory: Option<&ScenarioInventory>,
) -> bool {
    inventory.is_some_and(|inventory| !inventory.protagonist_sprite_variants.is_empty())
        && member_at(game, state.selected).is_some_and(|member| member.is_protagonist())
}

/// Commits the selected character and appearance before returning to the deck.
pub(super) fn switch_controlled_member(
    game: &mut GameState,
    state: &mut FieldMenuState,
    inventory: Option<&ScenarioInventory>,
) {
    let Some(member_id) = member_id_at(game, state.selected).map(ToOwned::to_owned) else {
        return;
    };
    let name = member_at(game, state.selected)
        .map_or_else(|| member_id.clone(), |member| member.name().to_owned());
    if character_has_sprite_choices(game, state, inventory)
        && let Some(inventory) = inventory
        && let Some(variant) = inventory
            .protagonist_sprite_variants
            .get(state.sprite_choice)
    {
        let is_default = inventory
            .party_sprites
            .iter()
            .any(|(id, path)| id == &member_id && *path == variant.sprite);
        if game
            .set_protagonist_sprite_variant((!is_default).then(|| variant.id.clone()))
            .is_err()
        {
            state.message = "That sprite cannot be selected.".to_owned();
            return;
        }
    }
    state.mode = FieldMenuMode::Browse;
    state.selected = CHARACTER_COMMAND_INDEX;
    state.message = if game.set_controlled_member(member_id).is_ok() {
        format!("Now controlling {name}.")
    } else {
        "That party member cannot be controlled.".to_owned()
    };
}

/// Columns the command deck needs to hold every command at [`MAIN_COMMAND_ROWS`] per column.
pub(super) const fn main_command_columns() -> usize {
    MAIN_COMMANDS.len().div_ceil(MAIN_COMMAND_ROWS)
}

/// Commands in one deck column; the last column is short when the count does not divide evenly.
pub(super) const fn main_command_column_len(column: usize) -> usize {
    let remaining = MAIN_COMMANDS
        .len()
        .saturating_sub(column * MAIN_COMMAND_ROWS);
    if remaining < MAIN_COMMAND_ROWS {
        remaining
    } else {
        MAIN_COMMAND_ROWS
    }
}

/// Moves the deck cursor across the command grid, wrapping on both axes.
///
/// Commands fill each column top to bottom before the next one starts, so the index alone carries
/// the column. Crossing into a short column clamps the row rather than landing on nothing.
pub(super) fn stepped_main_command(
    selected: usize,
    vertical: Option<isize>,
    horizontal: Option<isize>,
) -> usize {
    let selected = selected.min(MAIN_COMMANDS.len() - 1);
    let mut column = selected / MAIN_COMMAND_ROWS;
    let mut row = selected % MAIN_COMMAND_ROWS;
    if let Some(delta) = horizontal {
        column = wrapped(column, main_command_columns(), delta);
        row = row.min(main_command_column_len(column) - 1);
    }
    if let Some(delta) = vertical {
        row = wrapped(row, main_command_column_len(column), delta);
    }
    column * MAIN_COMMAND_ROWS + row
}

pub(super) fn main_command_screen(index: usize) -> Option<FieldMenuScreen> {
    MAIN_COMMANDS.get(index).and_then(|command| command.screen)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_menu::tests::fixture_game_with_recruit;

    #[test]
    fn confirming_the_switch_overlay_changes_control_and_returns_to_the_deck() {
        let mut game = fixture_game_with_recruit();
        assert_eq!(game.controlled_member_id(), "aric");

        let mut state = FieldMenuState::default();
        state.open(FieldMenuScreen::Main);
        state.mode = FieldMenuMode::CharacterSwitch;
        state.selected = 1;

        switch_controlled_member(&mut game, &mut state, None);

        assert_eq!(game.controlled_member_id(), "elise");
        assert_eq!(state.mode, FieldMenuMode::Browse);
        assert_eq!(state.selected, CHARACTER_COMMAND_INDEX);
        assert!(state.message.contains("Now controlling Elise"));
    }

    #[test]
    fn the_switch_overlay_opens_on_the_member_already_being_controlled() {
        let mut game = fixture_game_with_recruit();
        game.set_controlled_member("elise").unwrap();

        assert_eq!(controlled_member_index(&game), 1);
    }

    #[test]
    fn main_commands_distinguish_enabled_disabled_and_cancel_paths() {
        assert_eq!(main_command_screen(0), Some(FieldMenuScreen::Status));
        assert_eq!(main_command_screen(1), Some(FieldMenuScreen::Spells));
        assert_eq!(main_command_screen(2), Some(FieldMenuScreen::Items));
        assert_eq!(main_command_screen(5), Some(FieldMenuScreen::Equipment));
        assert_eq!(main_command_screen(6), Some(FieldMenuScreen::Quests));
        assert_eq!(main_command_screen(8), Some(FieldMenuScreen::Recipes));
        // Character and Quit open modals rather than screens.
        assert!(main_command_screen(CHARACTER_COMMAND_INDEX).is_none());
        assert_eq!(
            main_command_screen(SAVE_COMMAND_INDEX),
            Some(FieldMenuScreen::Save)
        );
        assert!(main_command_screen(QUIT_COMMAND_INDEX).is_none());

        let mut state = FieldMenuState::default();
        state.open(FieldMenuScreen::Items);
        state.back();
        assert!(state.open);
        assert_eq!(state.screen, FieldMenuScreen::Main);
        state.back();
        assert!(!state.open);
    }

    #[test]
    fn deck_navigation_wraps_within_a_column_and_crosses_between_them() {
        // Column 0 holds Status, Spells, Items, Quit; column 1 holds Save, Equipment, Quests,
        // Character; column 2 holds Recipes, Battle, and Settings.
        assert_eq!(stepped_main_command(0, Some(1), None), 1);
        assert_eq!(stepped_main_command(3, Some(1), None), 0);
        assert_eq!(stepped_main_command(0, Some(-1), None), 3);
        assert_eq!(stepped_main_command(4, Some(-1), None), 7);

        assert_eq!(stepped_main_command(1, None, Some(1)), 5);
        assert_eq!(stepped_main_command(5, None, Some(-1)), 1);
        // Rightward out of the last column wraps back to the first.
        assert_eq!(stepped_main_command(8, None, Some(1)), 0);
        // The short column 2 keeps the row while it can and clamps below its last command.
        assert_eq!(stepped_main_command(5, None, Some(1)), 9);
        assert_eq!(stepped_main_command(7, None, Some(1)), 10);
        // Vertical movement wraps inside the three-command column.
        assert_eq!(stepped_main_command(8, Some(1), None), 9);
        assert_eq!(stepped_main_command(9, Some(1), None), 10);
        assert_eq!(stepped_main_command(10, Some(1), None), 8);
        assert_eq!(stepped_main_command(8, Some(-1), None), 10);
    }

    #[test]
    fn crossing_into_the_short_column_clamps_to_its_last_command() {
        // Recipes opened a third column and Battle joined it, so the clamp is load bearing: every
        // row below the second has to land on the column's last entry.
        assert_eq!(main_command_columns(), 3);
        assert_eq!(main_command_column_len(0), 4);
        assert_eq!(main_command_column_len(1), 4);
        assert_eq!(main_command_column_len(2), 3);

        assert_eq!(stepped_main_command(3, None, Some(1)), 7);
        for row in 0..MAIN_COMMAND_ROWS {
            let crossed = stepped_main_command(row, None, Some(1));
            assert_eq!(
                crossed % MAIN_COMMAND_ROWS,
                row.min(main_command_column_len(1) - 1)
            );
        }
        // Crossing from the full middle column into the short one keeps the row or clamps.
        for row in 0..MAIN_COMMAND_ROWS {
            assert_eq!(
                stepped_main_command(MAIN_COMMAND_ROWS + row, None, Some(1)),
                2 * MAIN_COMMAND_ROWS + row.min(2),
                "row {row} must land in the short last column"
            );
        }
    }

    #[test]
    fn the_deck_cursor_never_leaves_the_command_list() {
        for selected in 0..MAIN_COMMANDS.len() {
            for vertical in [None, Some(-1), Some(1)] {
                for horizontal in [None, Some(-1), Some(1)] {
                    let stepped = stepped_main_command(selected, vertical, horizontal);
                    assert!(
                        stepped < MAIN_COMMANDS.len(),
                        "{selected} + {vertical:?}/{horizontal:?} left the deck at {stepped}"
                    );
                }
            }
        }
    }
}
