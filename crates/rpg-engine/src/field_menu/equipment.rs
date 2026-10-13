//! Equipment page actions: the party, slot, and inventory columns, and equip/unequip.

use bevy::prelude::*;

use super::{
    input::MenuInput,
    member_id_at,
    state::{FieldMenuMode, FieldMenuState, cycle_member, wrapped, wrapped_or_zero},
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
        // Party column: Up/Down picks the member, Right or Enter moves into their slots.
        FieldMenuMode::Browse => {
            if let Some(delta) = input.vertical {
                cycle_member(state, game.party().len(), delta);
            }
            if input.horizontal == Some(1) || input.confirm {
                state.mode = FieldMenuMode::EquipmentSlots;
            }
        }
        // Slot column: Up/Down picks the slot, Right or Enter opens its inventory, Left returns
        // to the party.
        FieldMenuMode::EquipmentSlots => {
            if let Some(delta) = input.vertical {
                state.selected = wrapped(state.selected, EquipmentSlot::ALL.len(), delta);
            }
            if input.horizontal == Some(-1) {
                state.back();
            } else if input.horizontal == Some(1) || input.confirm {
                state.quantity = state.selected as u32 + 1;
                state.mode = FieldMenuMode::EquipmentPicker;
                state.selected = 0;
            }
        }
        FieldMenuMode::EquipmentPicker => {
            if input.horizontal == Some(-1) {
                state.back();
                return;
            }
            let Some(member_id) = member_id_at(game, state.member_index).map(str::to_owned) else {
                return;
            };
            let slot = EquipmentSlot::ALL[state_slot_index(state)];
            let candidates = equipment_candidates(game, catalog, slot);
            if let Some(delta) = input.vertical {
                state.selected = wrapped_or_zero(state.selected, candidates.len() + 1, delta);
            }
            if input.confirm {
                if state.selected == 0 {
                    match unequip_item(game, &member_id, slot) {
                        Ok(Some(_)) => {
                            menu_sfx.play(cue::UNEQUIP);
                            state.message = "Equipment returned to the repository.".to_owned()
                        }
                        Ok(None) => {
                            menu_sfx.blocked();
                            state.message = "That slot is already empty.".to_owned()
                        }
                        Err(error) => {
                            menu_sfx.blocked();
                            state.message = error.to_string()
                        }
                    }
                } else if let Some(id) = candidates.get(state.selected - 1) {
                    let refusal = game
                        .party()
                        .member(&member_id)
                        .and_then(|member| equip_refusal(member, id, catalog));
                    if let Some(refusal) = refusal {
                        menu_sfx.blocked();
                        state.message = refusal;
                        return;
                    }
                    match equip_item(game, catalog, &member_id, id) {
                        Ok(_) => {
                            menu_sfx.play(cue::EQUIP);
                            state.mode = FieldMenuMode::EquipmentSlots;
                            state.selected = slot_index(slot);
                            state.message = "Equipment changed.".to_owned();
                        }
                        Err(error) => {
                            menu_sfx.blocked();
                            state.message = error.to_string()
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

pub(super) fn state_slot_index(state: &FieldMenuState) -> usize {
    if state.mode == FieldMenuMode::EquipmentPicker {
        // Picker selection replaces the slot cursor, so retain the chosen slot in quantity.
        state.quantity.saturating_sub(1) as usize
    } else {
        state.selected.min(EquipmentSlot::ALL.len() - 1)
    }
}

pub(super) fn slot_index(slot: EquipmentSlot) -> usize {
    EquipmentSlot::ALL
        .iter()
        .position(|candidate| *candidate == slot)
        .expect("known slot")
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
