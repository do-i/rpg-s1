//! Routes keyboard and action input to the open field-menu screen.

use bevy::{
    input::{ButtonState, keyboard::KeyboardInput},
    prelude::*,
};

use super::{
    battle::{self, MenuSettings},
    equipment, inventory, main_deck, save, spells,
    state::{FieldMenuMode, FieldMenuScreen, FieldMenuState, wrapped_or_zero},
    status,
};
use crate::{
    action_input::{ActionState, AppAction},
    field_menu_domain::FieldMenuCatalog,
    game_state::GameState,
    save_store::SaveStore,
    save_ui::SaveSlotCatalog,
    scenario_inventory::ScenarioInventory,
    service_ui::ServiceUiState,
    sfx_cue::{MenuSfx, cue},
    transport_ui::TransportUiState,
    world_interaction::WorldInteractionState,
    world_transition::WorldTransition,
};

/// One frame's menu input, read once and handed to the open screen's handler.
pub(super) struct MenuInput<'a> {
    pub(super) keys: &'a ButtonInput<KeyCode>,
    pub(super) vertical: Option<isize>,
    /// Left/Right arrow keys only; column moves and quantity jumps.
    pub(super) horizontal: Option<isize>,
    /// Horizontal menu navigation from any bound input; pages the battle log.
    pub(super) page: Option<isize>,
    pub(super) confirm: bool,
    /// Pressed keys with their text, for the new-tag prompt.
    pub(super) typed: Vec<KeyboardInput>,
}

#[expect(
    clippy::too_many_arguments,
    reason = "the field menu coordinates input, world locks, save storage, session state, and transitions"
)]
pub(super) fn handle_field_menu_input(
    keys: Res<ButtonInput<KeyCode>>,
    actions: Res<ActionState>,
    (catalog, inventory): (Res<FieldMenuCatalog>, Option<Res<ScenarioInventory>>),
    mut menu_settings: MenuSettings,
    interaction: Res<WorldInteractionState>,
    service: Option<Res<ServiceUiState>>,
    mut transition: ResMut<WorldTransition>,
    game: Option<ResMut<GameState>>,
    store: Res<SaveStore>,
    mut saves: ResMut<SaveSlotCatalog>,
    time: Res<Time<Real>>,
    mut state: ResMut<FieldMenuState>,
    transport_ui: Option<ResMut<TransportUiState>>,
    mut keyboard: MessageReader<KeyboardInput>,
    mut exit: MessageWriter<AppExit>,
    mut menu_sfx: MenuSfx,
) {
    let Some(mut game) = game else { return };

    if transport_ui
        .as_deref()
        .is_some_and(TransportUiState::input_locked)
    {
        return;
    }

    if !state.open {
        if interaction.input_locked()
            || service.as_deref().is_some_and(ServiceUiState::input_locked)
            || transition.input_locked()
        {
            return;
        }
        // Opening the menu from the world is the moment the world stops, so it gets the pause cue
        // rather than the plain confirm it shares with every in-menu acceptance.
        if keys.just_pressed(KeyCode::KeyM) || actions.just_pressed(AppAction::Back) {
            state.open(FieldMenuScreen::Main);
            menu_sfx.play(cue::PAUSE);
        } else if keys.just_pressed(KeyCode::KeyI) {
            state.open(FieldMenuScreen::Items);
            menu_sfx.play(cue::PAUSE);
        } else if keys.just_pressed(KeyCode::KeyS) {
            state.open(FieldMenuScreen::Status);
            menu_sfx.play(cue::PAUSE);
        } else if keys.just_pressed(KeyCode::KeyQ) {
            state.open(FieldMenuScreen::Quests);
            menu_sfx.play(cue::PAUSE);
        }
        return;
    }

    if keys.just_pressed(KeyCode::KeyM) {
        state.close();
        menu_sfx.play(cue::UNPAUSE);
        return;
    }
    if actions.just_pressed(AppAction::Back) {
        state.back();
        menu_sfx.cancel();
        return;
    }
    // Drained every frame, not only while the tag prompt is open, so entering the prompt never
    // replays the keypress that opened it.
    let typed = keyboard
        .read()
        .filter(|input| input.state == ButtonState::Pressed)
        .cloned()
        .collect::<Vec<_>>();

    let horizontal = if keys.just_pressed(KeyCode::ArrowLeft) {
        Some(-1)
    } else if keys.just_pressed(KeyCode::ArrowRight) {
        Some(1)
    } else {
        None
    };
    let vertical = actions.menu_navigation();
    let confirm = actions.just_pressed(AppAction::Confirm);

    // Messages persist until the player's next input rather than one frame. Clearing them
    // unconditionally made every refusal ("This tab is empty", "Invalid tag") unreadable: the
    // page rebuilds on state change, so the banner appeared and vanished inside one frame.
    if !state.message.is_empty()
        && !matches!(
            state.mode,
            FieldMenuMode::SaveConfirm | FieldMenuMode::QuitConfirm | FieldMenuMode::ItemAoeConfirm
        )
        && (vertical.is_some() || horizontal.is_some() || !typed.is_empty() || confirm)
    {
        state.message.clear();
    }

    // The four menu beats are wired here rather than inside each screen's handler. The handlers
    // return early from many of their confirm branches, so there is no single point after them
    // where "what actually changed" could be compared; hooking the shared entry points instead
    // keeps the cue wiring in one readable place.
    //
    // The trade-off is that Confirm sounds on every press while the menu is open, including a
    // press a branch ignores. Per-branch `blocked()` cues for the refusals are follow-up work.
    if vertical.is_some() || horizontal.is_some() {
        menu_sfx.hover();
    }
    if confirm {
        menu_sfx.confirm();
    }

    let input = MenuInput {
        keys: &keys,
        vertical,
        horizontal,
        page: actions.menu_navigation_horizontal(),
        confirm,
        typed,
    };
    match state.screen {
        FieldMenuScreen::Main => main_deck::handle_input(
            &mut state,
            &mut game,
            &input,
            inventory.as_deref(),
            &mut exit,
        ),
        FieldMenuScreen::Status => status::handle_input(&mut state, &mut game, &input),
        FieldMenuScreen::Items => inventory::handle_input(
            &mut state,
            &mut game,
            &input,
            &catalog,
            &menu_settings.engine,
            &mut menu_sfx,
        ),
        FieldMenuScreen::Equipment => {
            equipment::handle_input(&mut state, &mut game, &input, &catalog, &mut menu_sfx);
        }
        FieldMenuScreen::Spells => spells::handle_input(
            &mut state,
            &mut game,
            &input,
            &catalog,
            transport_ui,
            &mut transition,
            &mut menu_sfx,
        ),
        FieldMenuScreen::Quests => {
            if state.mode == FieldMenuMode::Browse
                && let Some(delta) = input.vertical
            {
                state.selected = wrapped_or_zero(state.selected, catalog.quests().len(), delta);
            }
        }
        // Browsing is the whole screen. Deliberately no Confirm arm: the recipe book is a
        // reference, and crafting stays with the apothecary service (roadmap B4.4).
        FieldMenuScreen::Recipes => {
            if state.mode == FieldMenuMode::Browse
                && let Some(delta) = input.vertical
            {
                state.selected = wrapped_or_zero(state.selected, catalog.recipes().len(), delta);
            }
        }
        FieldMenuScreen::Save => {
            save::handle_input(&mut state, &mut game, &input, &store, &mut saves, &time);
        }
        FieldMenuScreen::Settings => {
            super::settings::handle_input(&mut state, &input, &mut menu_settings);
        }
        FieldMenuScreen::Battle => {
            battle::handle_input(
                &mut state,
                &mut game,
                &input,
                &mut menu_settings,
                &mut menu_sfx,
            );
        }
    }
}
