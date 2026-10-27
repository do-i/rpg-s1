//! Which field-menu screen and mode is open, and where its shared cursor sits.

use bevy::prelude::*;

use super::{
    equipment::state_slot_index, inventory::ITEM_ACTION_TAGS, main_deck::CHARACTER_COMMAND_INDEX,
    status::STATUS_POSITION_CATEGORY,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum FieldMenuScreen {
    #[default]
    Main,
    Status,
    Items,
    Equipment,
    Spells,
    Quests,
    /// Read-only recipe book. Crafting stays with the apothecary service (roadmap B4.4).
    Recipes,
    Save,
    /// Per-member Manual/Auto tactics and the auto default command.
    Battle,
    /// Player preferences that are not tied to battle, such as sound.
    Settings,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum FieldMenuMode {
    #[default]
    Browse,
    ItemActions,
    /// Items page with the item list focused; `Browse` there is the pouch-tab column.
    ItemList,
    /// Spells page with the spellbook focused; `Browse` there is the caster column.
    SpellList,
    /// Equipment page's horizontal row of equipped cards; Down opens the inventory below.
    EquipmentSlots,
    DiscardQuantity,
    ItemTarget,
    /// Party picker for equipping the pending item, previewing each member's stat change.
    ItemEquipTarget,
    EquipmentPicker,
    /// Equipment comparison's Equip/Unequip button; only Confirm here changes equipment.
    EquipmentConfirm,
    SpellTarget,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "legacy picker retained only for regression fixtures; runtime uses Ember Atlas"
        )
    )]
    TeleportPicker,
    SaveConfirm,
    QuitConfirm,
    /// Roster overlay for choosing which member the World sprite follows.
    CharacterSwitch,
    /// Tag editor for one item: toggle curatorial tags, drop custom ones, add a new one.
    ItemTags,
    /// Free-text entry for a new custom tag.
    ItemNewTag,
    /// Show/hide manager listing every owned item, hidden ones included.
    ItemManage,
    /// Y/N guard before an item is spent on the whole party. Gated on `item.use_aoe_confirm`.
    ItemAoeConfirm,
    /// Battle page list of Manual / Auto commands for the focused member.
    BattleTacticPicker,
    /// Battle page reading the last battle's log; Up/Down scroll it.
    BattleLog,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum StatusPage {
    #[default]
    Roster,
    Details,
    /// Front/back row picker, reached from the Position category.
    Position,
}

#[derive(Debug, Default, Resource)]
pub(crate) struct FieldMenuState {
    pub(super) open: bool,
    pub(super) screen: FieldMenuScreen,
    pub(super) mode: FieldMenuMode,
    pub(super) selected: usize,
    pub(super) member_index: usize,
    /// Draft appearance in the Character overlay; committed only by Confirm.
    pub(super) sprite_choice: usize,
    pub(super) status_page: StatusPage,
    pub(super) tab_index: usize,
    /// Highlighted entry in the Battle page's tactic picker, an index into `TACTIC_CHOICES`.
    pub(super) battle_choice: usize,
    /// First line shown while reading the last battle's log.
    pub(super) log_scroll: usize,
    /// The item or spell the list cursor was on when a modal took over [`Self::selected`], so
    /// leaving the modal puts the cursor back where it was.
    pub(super) list_cursor: usize,
    pub(super) quantity: u32,
    pub(super) pending_id: Option<String>,
    /// Draft text for the new-custom-tag prompt.
    pub(super) text_input: String,
    pub(super) message: String,
    /// The Settings page's right-column cursor: `Some(row)` while focus is on the selected card's
    /// sub switches, `None` while it is on the left list.
    pub(super) settings_sub: Option<usize>,
    /// Where each of this screen's lists was last scrolled.
    pub(super) scroll: crate::menu_chrome::ScrollMemory,
}

impl FieldMenuState {
    pub(crate) const fn input_locked(&self) -> bool {
        self.open
    }

    /// Which command the deck behind a modal should keep highlighted.
    ///
    /// [`Self::selected`] is a single cursor shared by every mode, so while the switch overlay is
    /// up it holds a *party member* index — and the command deck underneath the translucent modal
    /// was reading it as a command index, walking its highlight down Items, Equipment, Spells as
    /// the player moved between members. The deck is not the thing being steered, so it pins to
    /// the command that opened the modal, exactly where [`Self::back`] returns the cursor.
    pub(super) const fn main_command_cursor(&self) -> usize {
        match self.mode {
            FieldMenuMode::CharacterSwitch => CHARACTER_COMMAND_INDEX,
            _ => self.selected,
        }
    }

    pub(super) fn open(&mut self, screen: FieldMenuScreen) {
        *self = Self {
            open: true,
            screen,
            ..default()
        };
    }

    pub(crate) fn close(&mut self) {
        *self = Self::default();
    }

    /// Whether [`Self::selected`] is a list cursor rather than a modal's own choice.
    ///
    /// The pouch and spellbook read their cursor from it in these modes; once a modal opens,
    /// they show the entry the modal was opened for instead.
    pub(super) const fn selected_is_list_cursor(&self) -> bool {
        matches!(
            self.mode,
            FieldMenuMode::Browse | FieldMenuMode::ItemList | FieldMenuMode::SpellList
        )
    }

    /// Hands [`Self::selected`] to a modal opened from a list, remembering the list's place.
    pub(super) fn leave_list(&mut self, mode: FieldMenuMode) {
        self.list_cursor = self.selected;
        self.mode = mode;
        self.selected = 0;
    }

    /// Returns from a list's modal to the list itself, on the entry it was opened from.
    pub(super) fn return_to_list(&mut self) {
        self.mode = if self.screen == FieldMenuScreen::Spells {
            FieldMenuMode::SpellList
        } else {
            FieldMenuMode::ItemList
        };
        self.selected = self.list_cursor;
        self.pending_id = None;
    }

    pub(super) fn back(&mut self) {
        if self.screen == FieldMenuScreen::Status {
            match self.status_page {
                StatusPage::Roster => {}
                StatusPage::Details => {
                    self.status_page = StatusPage::Roster;
                    self.selected = 0;
                    self.message.clear();
                    return;
                }
                StatusPage::Position => {
                    self.status_page = StatusPage::Details;
                    self.selected = STATUS_POSITION_CATEGORY;
                    self.message.clear();
                    return;
                }
            }
        }
        match self.mode {
            FieldMenuMode::Browse => {
                if self.screen == FieldMenuScreen::Main {
                    self.close();
                } else {
                    self.screen = FieldMenuScreen::Main;
                    self.selected = 0;
                    self.message.clear();
                }
            }
            FieldMenuMode::ItemActions | FieldMenuMode::ItemManage => {
                self.return_to_list();
                self.message.clear();
            }
            // The list columns step back one column left, to the pouch or the caster.
            FieldMenuMode::ItemList | FieldMenuMode::SpellList => {
                self.mode = FieldMenuMode::Browse;
                self.message.clear();
            }
            FieldMenuMode::EquipmentConfirm => {
                self.mode = FieldMenuMode::EquipmentPicker;
                self.pending_id = None;
                self.message.clear();
            }
            // Inventory returns to the equipped card that opened it.
            FieldMenuMode::EquipmentPicker => {
                self.selected = state_slot_index(self);
                self.mode = FieldMenuMode::EquipmentSlots;
                self.message.clear();
            }
            FieldMenuMode::EquipmentSlots => {
                self.mode = FieldMenuMode::Browse;
                self.message.clear();
            }
            FieldMenuMode::ItemTags => {
                self.mode = FieldMenuMode::ItemActions;
                self.selected = ITEM_ACTION_TAGS;
                self.message.clear();
            }
            FieldMenuMode::ItemNewTag => {
                self.mode = FieldMenuMode::ItemTags;
                self.text_input.clear();
                self.message.clear();
            }
            FieldMenuMode::DiscardQuantity
            | FieldMenuMode::ItemTarget
            | FieldMenuMode::ItemEquipTarget
            | FieldMenuMode::SpellTarget
            | FieldMenuMode::TeleportPicker => {
                self.quantity = 1;
                self.message.clear();
                if self.screen == FieldMenuScreen::Spells {
                    self.return_to_list();
                } else {
                    self.mode = FieldMenuMode::ItemActions;
                    self.selected = 0;
                }
            }
            FieldMenuMode::SaveConfirm => {
                self.mode = FieldMenuMode::Browse;
                self.message.clear();
            }
            // Backing out of the guard returns to the item's action list, not to browsing, so
            // declining to spend a Tent does not also lose the player's place.
            FieldMenuMode::ItemAoeConfirm => {
                self.mode = FieldMenuMode::ItemActions;
                self.message.clear();
            }
            FieldMenuMode::QuitConfirm => {
                self.mode = FieldMenuMode::Browse;
                self.message.clear();
            }
            FieldMenuMode::CharacterSwitch => {
                self.mode = FieldMenuMode::Browse;
                self.selected = CHARACTER_COMMAND_INDEX;
                self.message.clear();
            }
            FieldMenuMode::BattleTacticPicker | FieldMenuMode::BattleLog => {
                self.mode = FieldMenuMode::Browse;
                self.message.clear();
            }
        }
    }
}

pub(super) fn cycle_member(state: &mut FieldMenuState, count: usize, delta: isize) {
    state.member_index = wrapped_or_zero(state.member_index, count, delta);
    state.selected = 0;
}

pub(super) fn wrapped_or_zero(current: usize, count: usize, delta: isize) -> usize {
    if count == 0 {
        0
    } else {
        wrapped(current.min(count - 1), count, delta)
    }
}

pub(super) fn wrapped(current: usize, count: usize, delta: isize) -> usize {
    (current as isize + delta).rem_euclid(count as isize) as usize
}

pub(super) fn wrapped_quantity(current: u32, max: u32, delta: isize) -> u32 {
    ((current.saturating_sub(1) as isize + delta).rem_euclid(max as isize) + 1) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_navigation_and_quantity_bounds_are_deterministic() {
        assert_eq!(wrapped(0, 1, -1), 0);
        assert_eq!(wrapped(4, 5, 1), 0);
        assert_eq!(wrapped(0, 5, -1), 4);
        assert_eq!(wrapped(0, 6, -1), 5);
        assert_eq!(wrapped(5, 6, 1), 0);
        assert_eq!(wrapped_or_zero(3, 0, 1), 0);
        assert_eq!(wrapped_quantity(1, 4, -1), 4);
        assert_eq!(wrapped_quantity(4, 4, 1), 1);
    }

    #[test]
    fn status_back_returns_from_details_before_leaving_status() {
        let mut state = FieldMenuState::default();
        state.open(FieldMenuScreen::Status);
        state.status_page = StatusPage::Details;
        state.selected = 1;

        state.back();

        assert!(state.open);
        assert_eq!(state.screen, FieldMenuScreen::Status);
        assert_eq!(state.status_page, StatusPage::Roster);
        assert_eq!(state.selected, 0);

        state.back();
        assert_eq!(state.screen, FieldMenuScreen::Main);
    }

    #[test]
    fn status_back_steps_out_of_the_position_picker_one_page_at_a_time() {
        let mut state = FieldMenuState::default();
        state.open(FieldMenuScreen::Status);
        state.status_page = StatusPage::Position;
        state.selected = 1;

        state.back();

        assert_eq!(state.status_page, StatusPage::Details);
        assert_eq!(state.selected, STATUS_POSITION_CATEGORY);

        state.back();
        assert_eq!(state.status_page, StatusPage::Roster);
    }

    #[test]
    fn cancelling_the_switch_overlay_returns_the_cursor_to_the_character_command() {
        let mut state = FieldMenuState::default();
        state.open(FieldMenuScreen::Main);
        state.mode = FieldMenuMode::CharacterSwitch;
        state.selected = 3;

        state.back();

        assert!(state.open);
        assert_eq!(state.mode, FieldMenuMode::Browse);
        assert_eq!(state.selected, CHARACTER_COMMAND_INDEX);
    }
}
