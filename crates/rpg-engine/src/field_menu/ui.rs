use bevy::{ecs::hierarchy::ChildSpawnerCommands, prelude::*};

use super::{
    assets::*, battle::*, equipment::*, inventory::*, main_deck::*, markers::*, save::*, spells::*,
    state::*, status::*, *,
};
use crate::{
    field_menu_domain::{
        CUSTOM_TAG_MAX_LENGTH, CatalogStatus, EDITABLE_SYSTEM_TAGS, FieldMenuCatalog, InventoryTab,
        can_equip, derived_stats, inventory_ids, item_description, item_name,
        learned_field_abilities, preview_stats,
    },
    game_state::GameState,
    menu_chrome::{
        ScrollFades, location_display_name, selectable_card_colors, spawn_focus_panel,
        spawn_header_bars, spawn_meter, spawn_panel_rule, spawn_scroll_fades, spawn_section_rule,
        spawn_status_panel, spawn_status_text, spawn_windowed_card, status_border,
        status_border_active, status_ember, status_faint, status_gold, status_ink, status_muted,
        status_ready, status_teal, status_violet, visible_rows, window_start,
    },
    options_store::PlayerOptions,
    runtime_member::EquipmentSlot,
    runtime_quest::{QuestStatus, quest_status},
    save_store::{FIRST_PLAYER_SLOT, LAST_PLAYER_SLOT, SaveSlot, SaveSlotState},
    save_ui::{
        SaveSlotCatalog, save_slot_state_color, save_slot_state_label, spawn_save_message,
        spawn_save_slot_summary,
    },
    scenario_class::{Ability, AbilityKind, UtilityAbility},
    scenario_inventory::ScenarioInventory,
    scenario_item::ItemDefinition,
    scenario_path::ScenarioRelativePath,
    scenario_quest::{QuestDefinition, QuestKind},
    scenario_recipe::RecipeDefinition,
    scenario_root::ScenarioRoot,
    service_domain::{RecipeAvailability, recipe_availability, recipe_input_requirements},
    transport_domain::TransportDomain,
    tsx_atlas_asset::TsxAtlasAsset,
    ui_theme::UiTheme,
};

mod battle_page;
mod equipment_page;
mod fallback;
mod items_page;
mod main_page;
mod quests_page;
mod recipes_page;
mod save_page;
mod shared;
mod shell;
mod spells_page;
mod status_page;

pub(super) use battle_page::*;
pub(super) use equipment_page::*;
pub(super) use fallback::*;
pub(super) use items_page::*;
pub(super) use main_page::*;
pub(super) use quests_page::*;
pub(super) use recipes_page::*;
pub(super) use save_page::*;
pub(super) use shared::*;
pub(super) use shell::*;
pub(super) use spells_page::*;
pub(super) use status_page::*;

pub(super) const INVENTORY_PAGE_ROWS: usize = 11;
pub(super) const EQUIPMENT_PICKER_VISIBLE_ROWS: usize = 5;
pub(super) const SPELLBOOK_VISIBLE_ROWS: usize = 8;
/// Windowed lists show as many cards as their column fits (see the row-budget test), so a
/// scrolled list fills its panel and the fade on its edge card is what says more are hidden.
pub(super) const QUEST_VISIBLE_ROWS: usize = 8;
/// Quest and recipe cards grow from this to fill their list, up to the cap.
pub(super) const REGISTER_CARD_MIN_HEIGHT: f32 = 54.0;
pub(super) const REGISTER_CARD_MAX_HEIGHT: f32 = 76.0;
pub(super) const RECIPE_VISIBLE_ROWS: usize = 8;
pub(super) const SAVE_VISIBLE_ROWS: usize = 8;
/// Destination rows the teleport picker shows before it scrolls.
///
/// The modal is capped at 520px tall, and each row is 48px over a 9px gap, so six rows plus
/// the modal's own chrome is what fits. The list itself is unbounded in principle — every
/// visited non-submap with a `warp_order` is a destination, 29 of them in the shipped
/// scenario — so without this window a late-game cast paints far past the canvas.
pub(super) const TELEPORT_VISIBLE_ROWS: usize = 6;
pub(super) const ITEM_MANAGE_VISIBLE_ROWS: usize = 8;
/// Tag rows the editor shows before it scrolls.
///
/// Unlike the other lists here this one's length is scenario data, not code: it is the
/// three curatorial tags plus `max_tags_per_item` custom ones plus the new-tag prompt, and
/// `max_tags_per_item` lives in `data/balance.yaml`. At the shipped value of 5 the modal
/// fits without scrolling; the window is what keeps a larger value from walking the list off
/// the canvas, since the modal is centred and has no height cap of its own.
pub(super) const TAG_EDITOR_VISIBLE_ROWS: usize = 9;

pub(super) const STATUS_PARTY_WIDTH: f32 = 316.0;
pub(super) const STATUS_DETAIL_WIDTH: f32 = 404.0;
pub(super) const STATUS_COLUMN_GAP: f32 = 18.0;
/// Party card height bounds. Five cards at the minimum fill the column; fewer members grow up to
/// the maximum so their portraits read larger.
pub(super) const STATUS_CARD_MIN_HEIGHT: f32 = 92.0;
pub(super) const STATUS_CARD_MAX_HEIGHT: f32 = 128.0;
pub(super) const ITEMS_POUCH_WIDTH: f32 = 286.0;
pub(super) const ITEMS_DETAIL_WIDTH: f32 = 378.0;
pub(super) const ITEMS_COLUMN_GAP: f32 = 18.0;
pub(super) const EQUIPMENT_SLOT_WIDTH: f32 = 306.0;
pub(super) const MAIN_DECK_WIDTH: f32 = 772.0;
pub(super) const MAIN_DECK_COLUMN_GAP: f32 = 14.0;
pub(super) const QUEST_LIST_WIDTH: f32 = 512.0;
pub(super) const QUEST_COLUMN_GAP: f32 = 18.0;
pub(super) const RECIPE_LIST_WIDTH: f32 = 512.0;
pub(super) const RECIPE_COLUMN_GAP: f32 = 18.0;
pub(super) const BATTLE_LIST_WIDTH: f32 = 512.0;

/// The visible slice of the inventory list, scrolled only far enough to keep `selected` on screen.
pub(super) fn inventory_window(len: usize, selected: usize) -> std::ops::Range<usize> {
    let start = crate::menu_chrome::window_start(selected, len, INVENTORY_PAGE_ROWS);
    start..(start + INVENTORY_PAGE_ROWS).min(len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_window_scrolls_one_row_at_a_time() {
        let rows = INVENTORY_PAGE_ROWS;
        assert_eq!(inventory_window(0, 0), 0..0);
        assert_eq!(inventory_window(5, 4), 0..5);
        assert_eq!(inventory_window(25, rows - 1), 0..rows);
        // Stepping past the last visible row scrolls by one, not by a whole page.
        assert_eq!(inventory_window(25, rows), 1..rows + 1);
        // The end of the list fills the window rather than showing a short final page.
        assert_eq!(inventory_window(25, 24), 25 - rows..25);
    }
}
