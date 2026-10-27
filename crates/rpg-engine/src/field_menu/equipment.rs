//! Equipment focus: party, equipped cards, inventory, then the comparison's commit button.

use bevy::prelude::*;

use super::{
    input::MenuInput,
    member_id_at,
    state::{FieldMenuMode, FieldMenuState, cycle_member},
    ui,
};
use crate::{
    field_menu_domain::{FieldMenuCatalog, can_equip, equip_item, item_name, unequip_item},
    game_state::GameState,
    runtime_member::EquipmentSlot,
    scenario_item::ItemDefinition,
    sfx_cue::{MenuSfx, cue},
};

pub(super) fn handle_input(
    state: &mut ResMut<'_, FieldMenuState>,
    game: &mut ResMut<'_, GameState>,
    input: &MenuInput<'_>,
    catalog: &FieldMenuCatalog,
    menu_sfx: &mut MenuSfx<'_>,
) {
    match state.mode {
        FieldMenuMode::Browse => {
            if let Some(delta) = input.vertical {
                cycle_member(state, game.party().len(), delta);
            }
            if input.horizontal == Some(1) || input.confirm {
                state.mode = FieldMenuMode::EquipmentSlots;
                state.selected = 0;
                state.pending_id = None;
            }
        }
        FieldMenuMode::EquipmentSlots => {
            if input.horizontal == Some(-1) && state.selected == 0 {
                state.back();
            } else if let Some(delta) = input.horizontal {
                state.selected = state
                    .selected
                    .saturating_add_signed(delta)
                    .min(EquipmentSlot::ALL.len() - 1);
            } else if input.vertical == Some(1) || input.confirm {
                state.quantity = state.selected as u32 + 1;
                state.mode = FieldMenuMode::EquipmentPicker;
                state.selected = 0;
                state.pending_id = None;
            }
        }
        FieldMenuMode::EquipmentPicker => {
            if input.horizontal == Some(-1) || (input.vertical == Some(-1) && state.selected == 0) {
                state.back();
                return;
            }
            let slot = EquipmentSlot::ALL[state_slot_index(state)];
            let candidates = equipment_candidates(game, catalog, slot);
            if let Some(delta) = input.vertical {
                state.selected = state
                    .selected
                    .saturating_add_signed(delta)
                    .min(candidates.len());
            }
            if input.horizontal == Some(1) || input.confirm {
                let Some(member) = super::member_at(game, state.member_index) else {
                    return;
                };
                let candidate = candidates.get(state.selected);
                let refusal = candidate.map_or_else(
                    || {
                        member
                            .equipment()
                            .get(slot)
                            .is_none()
                            .then(|| "That slot is already empty.".to_owned())
                    },
                    |id| equip_refusal(member, id, catalog),
                );
                if let Some(refusal) = refusal {
                    menu_sfx.blocked();
                    state.message = refusal;
                    return;
                }
                state.pending_id = candidate.cloned();
                state.mode = FieldMenuMode::EquipmentConfirm;
            }
        }
        FieldMenuMode::EquipmentConfirm => {
            if input.horizontal == Some(-1) {
                state.back();
                return;
            }
            if !input.confirm {
                return;
            }
            let Some(member_id) = member_id_at(game, state.member_index).map(str::to_owned) else {
                return;
            };
            let slot = EquipmentSlot::ALL[state_slot_index(state)];
            let result = if let Some(id) = state.pending_id.as_deref() {
                equip_item(game, catalog, &member_id, id).map(|_| true)
            } else {
                unequip_item(game, &member_id, slot).map(|removed| removed.is_some())
            };
            match result {
                Ok(true) => {
                    let equipped = state.pending_id.is_some();
                    menu_sfx.play(if equipped { cue::EQUIP } else { cue::UNEQUIP });
                    state.mode = FieldMenuMode::EquipmentPicker;
                    state.pending_id = None;
                    state.selected = state
                        .selected
                        .min(equipment_candidates(game, catalog, slot).len());
                    state.message = if equipped {
                        "Equipment changed."
                    } else {
                        "Equipment returned to the repository."
                    }
                    .to_owned();
                }
                Ok(false) => {
                    menu_sfx.blocked();
                    state.message = "That slot is already empty.".to_owned();
                }
                Err(error) => {
                    menu_sfx.blocked();
                    state.message = error.to_string();
                }
            }
        }
        _ => {}
    }
}

pub(super) fn state_slot_index(state: &FieldMenuState) -> usize {
    if matches!(
        state.mode,
        FieldMenuMode::EquipmentPicker | FieldMenuMode::EquipmentConfirm
    ) {
        // Picker selection replaces the slot cursor, so retain the chosen slot in quantity.
        state.quantity.saturating_sub(1) as usize
    } else {
        state.selected.min(EquipmentSlot::ALL.len() - 1)
    }
}

/// The confirmation keeps the item identity stable even if the repository changes.
pub(super) fn equipment_candidate<'a>(
    state: &'a FieldMenuState,
    candidates: &'a [String],
) -> Option<&'a str> {
    if state.mode == FieldMenuMode::EquipmentConfirm {
        state.pending_id.as_deref()
    } else {
        candidates.get(state.selected).map(String::as_str)
    }
}

/// Why `member` cannot equip `item_id`, phrased for the player, or `None` when they can.
pub(super) fn equip_refusal(
    member: &crate::runtime_member::RuntimeMember,
    item_id: &str,
    catalog: &FieldMenuCatalog,
) -> Option<String> {
    let item = catalog.item(item_id)?;
    can_equip(member, item, catalog).err().map(|_| {
        format!(
            "{} ({}) can't equip {}.",
            member.name(),
            ui::class_name(member.class_id(), catalog),
            item_name(item)
        )
    })
}

pub(super) fn equipment_candidates(
    game: &GameState,
    catalog: &FieldMenuCatalog,
    slot: EquipmentSlot,
) -> Vec<String> {
    game.repository()
        .item_counts()
        .filter_map(|(id, _)| {
            let candidate_slot = match catalog.item(id)? {
                ItemDefinition::Weapon(_) => EquipmentSlot::Weapon,
                ItemDefinition::Shield(_) => EquipmentSlot::Shield,
                ItemDefinition::Helmet(_) => EquipmentSlot::Helmet,
                ItemDefinition::Body(_) => EquipmentSlot::Body,
                ItemDefinition::Accessory(_) => EquipmentSlot::Accessory,
                _ => return None,
            };
            (candidate_slot == slot).then(|| id.to_owned())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_menu::tests::fixture_game_with_recruit;

    #[test]
    fn a_class_that_cannot_wear_an_item_gets_a_plain_refusal() {
        let game = fixture_game_with_recruit();
        let catalog = crate::field_menu_domain::tests::catalog();
        let elise = game.party().member("elise").unwrap();
        assert_eq!(
            equip_refusal(elise, "leather_vest", &catalog).as_deref(),
            Some("Elise (Cleric) can't equip Leather Vest.")
        );
        assert_eq!(equip_refusal(elise, "cloth_robe", &catalog), None);
        let aric = game.party().member("aric").unwrap();
        assert_eq!(equip_refusal(aric, "leather_vest", &catalog), None);
    }
}
