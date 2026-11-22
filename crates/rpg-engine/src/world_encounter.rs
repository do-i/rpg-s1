//! Visible World enemies and the transactional World-to-Battle handoff.

mod contact_cue;
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
    contact_cue::contact_beat_inactive,
    handoff::{BattleTransition, ScriptedBattleRequest},
    presentation::BattleBgmFadeOut,
    state::{WorldEncounterRestore, WorldEncounterState, WorldEnemy},
};
use self::{
    contact_cue::{
        ChaseWarningClock, ContactCueSettings, EnemyContactCue, UnseenChaseStarted,
        animate_encounter_cues, cleanup_encounter_cues, drive_encounter_cues,
    },
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

pub(crate) struct WorldEncounterPlugin;

impl Plugin for WorldEncounterPlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<ContactCueSettings>() {
            app.insert_resource(ContactCueSettings::from_environment(|name| {
                std::env::var_os(name)
            }));
        }
        app.init_resource::<WorldEncounterState>()
            .init_resource::<BattleTransition>()
            .init_resource::<ChaseWarningClock>()
            .add_message::<EnemyContactCue>()
            .add_message::<UnseenChaseStarted>()
            .add_message::<crate::sfx_cue::PlaySfx>()
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
                    drive_encounter_cues,
                    animate_encounter_cues,
                    advance_battle_transition,
                    update_battle_flash_overlay,
                )
                    .chain()
                    .run_if(in_state(AppState::World)),
            )
            .add_systems(
                OnExit(AppState::World),
                (
                    cleanup_world_encounters,
                    cleanup_battle_flash_overlay,
                    cleanup_encounter_cues,
                ),
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
