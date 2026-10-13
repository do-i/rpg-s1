//! Marker components tagging the field menu's UI entities.

use bevy::prelude::*;

#[derive(Component)]
pub(super) struct FieldMenuRoot;

#[derive(Component)]
pub(super) struct FieldMenuTitle;

#[derive(Component)]
pub(super) struct FieldMenuBody;

#[derive(Component)]
pub(super) struct FieldMenuHint;

#[derive(Component)]
pub(super) struct FieldMenuGenericContent;

#[derive(Component)]
pub(super) struct FieldMenuMainPage;

#[derive(Component)]
pub(super) struct MainCommandRow;

/// The drawn icon inside a command's badge frame, when the scenario ships an icon sheet.
#[derive(Component)]
pub(super) struct MainCommandIcon;

#[derive(Component)]
pub(super) struct SelectedMainCommandRow;

#[derive(Component)]
pub(super) struct FieldMenuQuitModal;

#[derive(Component)]
pub(super) struct FieldMenuCharacterModal;

#[derive(Component)]
pub(super) struct FieldMenuStatusPage;

#[derive(Component)]
pub(super) struct FieldMenuItemsPage;

#[derive(Component)]
pub(super) struct ItemPouchRow;

#[derive(Component)]
pub(super) struct ItemListRow;

#[derive(Component)]
pub(super) struct SelectedItemListRow;

#[derive(Component)]
pub(super) struct ItemActionModal;

#[derive(Component)]
pub(super) struct FieldMenuEquipmentPage;

#[derive(Component)]
pub(super) struct EquipmentSlotRow;

#[derive(Component)]
pub(super) struct SelectedEquipmentSlotRow;

#[derive(Component)]
pub(super) struct EquipmentPickerRow;

#[derive(Component)]
pub(super) struct FieldMenuSpellsPage;

#[derive(Component)]
pub(super) struct SpellbookRow;

#[derive(Component)]
pub(super) struct SelectedSpellbookRow;

#[derive(Component)]
pub(super) struct SpellTargetOverlay;

#[derive(Component)]
pub(super) struct FieldMenuQuestsPage;

#[derive(Component)]
pub(super) struct FieldMenuRecipesPage;

#[derive(Component)]
pub(super) struct FieldMenuBattlePage;

#[derive(Component)]
pub(super) struct BattleTacticsRow;

#[derive(Component)]
pub(super) struct SelectedBattleTacticsRow;

#[derive(Component)]
pub(super) struct RecipeBookRow;

#[derive(Component)]
pub(super) struct SelectedRecipeBookRow;

#[derive(Component)]
pub(super) struct QuestBoardRow;

#[derive(Component)]
pub(super) struct SelectedQuestBoardRow;

#[derive(Component)]
pub(super) struct FieldMenuSavePage;

#[derive(Component)]
pub(super) struct FieldSaveSlotRow;

#[derive(Component)]
pub(super) struct SelectedFieldSaveSlotRow;

#[derive(Component)]
pub(super) struct SaveOverwriteModal;

#[derive(Component)]
pub(super) struct StatusMemberCard;

#[derive(Component)]
pub(super) struct SelectedStatusMemberCard;
