//! Spells page actions: casters, the spellbook, heal targeting, and teleport.

use bevy::prelude::*;

use super::{
    input::MenuInput,
    member_at, member_id_at,
    state::{FieldMenuMode, FieldMenuState, cycle_member, wrapped_or_zero},
};
use crate::{
    field_menu_domain::{
        FieldMenuCatalog, cast_heal, cast_party_heal, field_castable, heals_whole_party,
        learned_abilities,
    },
    game_state::GameState,
    runtime_map::RuntimeMapId,
    scenario_class::{Ability, AbilityKind, UtilityAbility},
    scenario_spatial::CardinalDirection,
    sfx_cue::{MenuSfx, cue},
    transport_ui::TransportUiState,
    world_transition::WorldTransition,
};

pub(super) fn handle_input(
    state: &mut ResMut<'_, FieldMenuState>,
    game: &mut ResMut<'_, GameState>,
    input: &MenuInput<'_>,
    catalog: &FieldMenuCatalog,
    transport_ui: Option<ResMut<'_, TransportUiState>>,
    transition: &mut ResMut<'_, WorldTransition>,
    menu_sfx: &mut MenuSfx<'_>,
) {
    match state.mode {
        // Caster column: Up/Down picks the member, Right or Enter moves into their spellbook.
        FieldMenuMode::Browse => {
            if let Some(delta) = input.vertical {
                cycle_member(state, game.party().len(), delta);
            }
            if input.horizontal == Some(1) || input.confirm {
                state.mode = FieldMenuMode::SpellList;
            }
        }
        // Spellbook: arrows follow the two-column grid; Escape returns to casters.
        FieldMenuMode::SpellList => {
            let abilities = member_at(game, state.member_index)
                .map_or_else(Vec::new, |member| learned_abilities(member, game, catalog));
            if let Some(delta) = input.vertical {
                state.selected = spell_grid_vertical(state.selected, abilities.len(), delta);
            } else if let Some(delta) = input.horizontal {
                let column = state.selected % 2;
                if delta < 0 && column == 0 {
                    state.back();
                    return;
                }
                if (delta < 0 && column == 1)
                    || (delta > 0 && column == 0 && state.selected + 1 < abilities.len())
                {
                    state.selected = state.selected.saturating_add_signed(delta);
                }
            }
            if input.confirm {
                let Some(ability) = abilities.get(state.selected).cloned().cloned() else {
                    state.message = "No learned field abilities.".to_owned();
                    return;
                };
                let Some(caster) = member_at(game, state.member_index) else {
                    return;
                };
                if !field_castable(&ability) {
                    state.message = format!("{} can only be cast in battle.", ability.name);
                    return;
                }
                if caster.mana() < ability.mp_cost {
                    state.message = "Not enough MP.".to_owned();
                    return;
                }
                match &ability.kind {
                    // A party-wide heal has no one to pick, so it lands straight from the list.
                    AbilityKind::Heal(_) if heals_whole_party(&ability) => {
                        let caster_id = caster.id().to_owned();
                        state.message = match cast_party_heal(game, &ability, &caster_id) {
                            Ok(amount) => format!("Restored {amount} HP to the party."),
                            Err(error) => error.to_string(),
                        };
                    }
                    AbilityKind::Heal(_) => {
                        state.pending_id = Some(ability.id.clone());
                        state.leave_list(FieldMenuMode::SpellTarget);
                    }
                    AbilityKind::Utility(UtilityAbility::Warp { .. }) => {
                        if catalog.eligible_warp_destinations(game.map()).is_empty() {
                            state.message = "Nowhere eligible to teleport to yet.".to_owned();
                        } else {
                            let Some(mut transport_ui) = transport_ui else {
                                state.message = "The Ember Atlas is unavailable.".to_owned();
                                return;
                            };
                            transport_ui.open_warp(
                                caster.id().to_owned(),
                                ability.id.clone(),
                                ability.mp_cost,
                            );
                            state.close();
                        }
                    }
                    _ => {
                        state.message =
                            "This field ability is display-only in the current slice.".to_owned()
                    }
                }
            }
        }
        FieldMenuMode::SpellTarget => {
            if let Some(delta) = input.vertical {
                state.selected = wrapped_or_zero(state.selected, game.party().len(), delta);
            }
            if input.confirm {
                let Some(caster_id) = member_id_at(game, state.member_index).map(str::to_owned)
                else {
                    return;
                };
                let Some(target_id) = member_id_at(game, state.selected).map(str::to_owned) else {
                    return;
                };
                let Some(ability) = ability_by_id(
                    game,
                    catalog,
                    &caster_id,
                    state
                        .pending_id
                        .as_deref()
                        .expect("spell target requires ability"),
                )
                .cloned() else {
                    return;
                };
                match cast_heal(game, &ability, &caster_id, &target_id) {
                    Ok(amount) => {
                        state.return_to_list();
                        state.message = format!("Restored {amount} HP.");
                    }
                    Err(error) => state.message = error.to_string(),
                }
            }
        }
        FieldMenuMode::TeleportPicker => {
            let destinations = catalog.eligible_warp_destinations(game.map());
            if let Some(delta) = input.vertical {
                state.selected = wrapped_or_zero(state.selected, destinations.len(), delta);
            }
            if input.confirm {
                let Some(destination) = destinations.get(state.selected).cloned() else {
                    return;
                };
                let Some(caster_id) = member_id_at(game, state.member_index).map(str::to_owned)
                else {
                    return;
                };
                let ability_id = state
                    .pending_id
                    .as_deref()
                    .expect("teleport requires ability");
                let Some(ability) = ability_by_id(game, catalog, &caster_id, ability_id).cloned()
                else {
                    return;
                };
                if game
                    .party()
                    .member(&caster_id)
                    .is_none_or(|member| member.mana() < ability.mp_cost)
                {
                    state.message = "Not enough MP.".to_owned();
                    return;
                }
                let map_id = RuntimeMapId::try_new(destination.map_id.clone())
                    .expect("loaded warp id is valid");
                if transition.request_destination(
                    map_id,
                    destination.position,
                    CardinalDirection::Down,
                ) {
                    menu_sfx.play(cue::TELEPORT);
                    game.party_mut()
                        .member_mut(&caster_id)
                        .expect("validated")
                        .spend_mana(ability.mp_cost);
                    state.close();
                } else {
                    menu_sfx.blocked();
                    state.message = "A map transition is already active.".to_owned();
                }
            }
        }
        _ => {}
    }
}

fn spell_grid_vertical(selected: usize, len: usize, delta: isize) -> usize {
    if len == 0 {
        return 0;
    }
    let row = wrapped_or_zero(selected / 2, len.div_ceil(2), delta);
    (row * 2 + selected % 2).min(len - 1)
}

pub(super) fn ability_by_id<'a>(
    game: &GameState,
    catalog: &'a FieldMenuCatalog,
    member_id: &str,
    ability_id: &str,
) -> Option<&'a Ability> {
    let member = game.party().member(member_id)?;
    learned_abilities(member, game, catalog)
        .into_iter()
        .find(|ability| ability.id == ability_id)
}
