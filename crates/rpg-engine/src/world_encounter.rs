//! Visible World enemies and the transactional World-to-Battle handoff.

mod fixtures;
mod handoff;
mod loading;
mod presentation;
mod simulation;
mod state;
#[cfg(test)]
mod tests;

use bevy::{ecs::schedule::ApplyDeferred, prelude::*};

pub(crate) use self::{
    handoff::{BattleTransition, ScriptedBattleRequest},
    presentation::BattleBgmFadeOut,
    state::{WorldEncounterRestore, WorldEncounterState, WorldEnemy},
};
use self::{
    handoff::{
        advance_battle_transition, cleanup_battle_flash_overlay, detect_enemy_contact,
        spawn_battle_flash_overlay, start_scripted_battle, update_battle_flash_overlay,
    },
    loading::{drive_active_encounter_assets, request_active_encounter_assets},
    presentation::{
        BattlePresentationState, begin_battle_presentation, cleanup_battle_presentation,
        drive_battle_audio,
    },
    simulation::update_world_enemies,
    state::{cleanup_world_encounters, reset_world_encounters},
};
use crate::app_state::AppState;

const TILE_SIZE: u32 = 32;

pub(crate) struct WorldEncounterPlugin;

impl Plugin for WorldEncounterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldEncounterState>()
            .init_resource::<BattleTransition>()
            .add_systems(
                OnEnter(AppState::World),
                (reset_world_encounters, spawn_battle_flash_overlay).chain(),
            )
            .add_systems(
                Update,
                (
                    request_active_encounter_assets
                        .in_set(crate::world_publication::WorldPublicationSet::Load),
                    ApplyDeferred,
                    drive_active_encounter_assets
                        .in_set(crate::world_publication::WorldPublicationSet::Load),
                    ApplyDeferred,
                    // The ambient World holds still while any overlay owns the
                    // screen; asset loading and the battle hand-off below do not.
                    (update_world_enemies, detect_enemy_contact)
                        .chain()
                        .run_if(crate::world_pause::world_simulation_running)
                        .after(crate::world_publication::WorldPublicationSet::Barrier),
                    start_scripted_battle.run_if(crate::world_transition::transition_settled),
                    advance_battle_transition,
                    update_battle_flash_overlay,
                )
                    .chain()
                    .run_if(in_state(AppState::World)),
            )
            .add_systems(
                OnExit(AppState::World),
                (cleanup_world_encounters, cleanup_battle_flash_overlay),
            );
    }
}

/// Minimal M8 presentation proving background and battle-BGM selection before M9 owns the loop.
pub(crate) struct BattleEntryPlugin;

impl Plugin for BattleEntryPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<BattleBgmFadeOut>()
            .init_resource::<BattlePresentationState>()
            .add_systems(OnEnter(AppState::Battle), begin_battle_presentation)
            .add_systems(
                Update,
                drive_battle_audio.run_if(in_state(AppState::Battle)),
            )
            .add_systems(OnExit(AppState::Battle), cleanup_battle_presentation);
    }
}
