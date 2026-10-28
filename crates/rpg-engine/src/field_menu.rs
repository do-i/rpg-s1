//! In-world field-menu shell over the shared M6 runtime domain.

mod assets;
mod battle;
mod equipment;
mod input;
mod inventory;
mod main_deck;
mod markers;
mod save;
mod settings;
mod spells;
mod state;
mod status;
#[cfg(test)]
mod tests;
mod ui;

use bevy::{ecs::schedule::ApplyDeferred, input::keyboard::KeyboardInput, prelude::*};

pub(crate) use self::state::FieldMenuState;
use self::{
    assets::{PartyWalkSheets, load_party_walk_sheets},
    input::handle_field_menu_input,
    ui::{
        cleanup_field_menu, sync_battle_page, sync_custom_field_menu_content_visibility,
        sync_equipment_page, sync_field_menu_generic_text, sync_field_menu_overlay_lifecycle,
        sync_items_page, sync_main_menu_page, sync_quests_page, sync_recipes_page, sync_save_page,
        sync_settings_page, sync_spells_page, sync_status_page,
    },
};
use crate::{app_state::AppState, game_state::GameState, sfx_cue::PlaySfx};

pub(crate) struct FieldMenuPlugin;

impl Plugin for FieldMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PlaySfx>()
            .add_message::<KeyboardInput>()
            .init_resource::<FieldMenuState>()
            .init_resource::<PartyWalkSheets>()
            .add_systems(
                OnEnter(AppState::World),
                (reset_field_menu, load_party_walk_sheets),
            )
            .add_systems(
                Update,
                (
                    handle_field_menu_input
                        .after(crate::service_ui::handle_service_input)
                        .after(crate::world_interaction::drive_dialogue_session),
                    sync_field_menu_overlay_lifecycle,
                    ApplyDeferred,
                    sync_field_menu_generic_text,
                    sync_custom_field_menu_content_visibility,
                    sync_main_menu_page,
                    sync_status_page,
                    sync_items_page,
                    sync_equipment_page,
                    sync_spells_page,
                    sync_quests_page,
                    sync_recipes_page,
                    sync_battle_page,
                    sync_save_page,
                    sync_settings_page,
                )
                    .chain()
                    .run_if(in_state(AppState::World)),
            )
            .add_systems(OnExit(AppState::World), cleanup_field_menu);
    }
}

fn reset_field_menu(mut state: ResMut<FieldMenuState>) {
    state.close();
}

fn member_at(game: &GameState, index: usize) -> Option<&crate::runtime_member::RuntimeMember> {
    game.party().members().nth(index)
}

fn member_id_at(game: &GameState, index: usize) -> Option<&str> {
    member_at(game, index).map(crate::runtime_member::RuntimeMember::id)
}
