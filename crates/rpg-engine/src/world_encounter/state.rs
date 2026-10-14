//! Encounter state shared by loading, enemy simulation, and the battle hand-off.

use std::collections::BTreeMap;

use bevy::{asset::Handle, prelude::*};

use crate::{
    encounter::{SpawnCadence, WorldEnemyReturnState},
    scenario_audio::SfxIndex,
    scenario_battle_background::BattleBackgroundCatalog,
    scenario_encounter::EncounterZone,
    scenario_enemy::{BossMoveSet, EnemyCatalogFile},
    scenario_map::MapMetadata,
    scenario_spatial::{CardinalDirection, Position},
    tsx_atlas_asset::TsxAtlasAsset,
    world_player::CharacterCollisionRect,
};

use super::{handoff::BattleTransition, simulation::enemy_collision_rect};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum WorldEncounterStatus {
    #[default]
    Idle,
    Loading,
    NoEncounters,
    Spawned,
    Failed,
}

#[derive(Clone, Debug, Default, Resource)]
pub(crate) struct WorldEncounterState {
    pub(super) map_id: Option<String>,
    pub(super) metadata: Option<Handle<MapMetadata>>,
    pub(super) zone: Option<Handle<EncounterZone>>,
    pub(super) enemy_files: Vec<Handle<EnemyCatalogFile>>,
    pub(super) boss_move_sets: Vec<(String, Handle<BossMoveSet>)>,
    pub(super) backgrounds: Option<Handle<BattleBackgroundCatalog>>,
    pub(super) sfx_index: Option<Handle<SfxIndex>>,
    pub(super) encounter_sfx: Option<String>,
    pub(super) pending: Vec<PendingWorldEnemy>,
    pub(super) sprite_handles: BTreeMap<String, Handle<TsxAtlasAsset>>,
    pub(super) cadence: Option<SpawnCadence>,
    pub(super) status: WorldEncounterStatus,
    pub(super) failure: Option<String>,
}

impl WorldEncounterState {
    pub(crate) fn publication_status(
        &self,
        map_id: &str,
    ) -> crate::world_publication::PublicationStatus {
        use crate::world_publication::PublicationStatus;
        if let Some(reason) = &self.failure {
            return PublicationStatus::Failed(reason.clone());
        }
        if self.map_id.as_deref() == Some(map_id)
            && matches!(
                self.status,
                WorldEncounterStatus::Spawned | WorldEncounterStatus::NoEncounters
            )
        {
            PublicationStatus::Ready
        } else {
            PublicationStatus::Pending
        }
    }

    pub(crate) fn asset_ids(&self) -> Vec<bevy::asset::UntypedAssetId> {
        self.metadata
            .iter()
            .map(|h| h.id().untyped())
            .chain(self.zone.iter().map(|h| h.id().untyped()))
            .chain(self.enemy_files.iter().map(|h| h.id().untyped()))
            .chain(self.boss_move_sets.iter().map(|(_, h)| h.id().untyped()))
            .chain(self.backgrounds.iter().map(|h| h.id().untyped()))
            .chain(self.sfx_index.iter().map(|h| h.id().untyped()))
            .chain(self.sprite_handles.values().map(|h| h.id().untyped()))
            .collect()
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "runtime diagnostics are test-facing")
    )]
    pub(crate) const fn status(&self) -> WorldEncounterStatus {
        self.status
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "runtime diagnostics are test-facing")
    )]
    pub(crate) fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }
}

#[derive(Clone, Debug)]
pub(super) struct PendingWorldEnemy {
    pub(super) encounter_id: String,
    pub(super) formation: Vec<String>,
    pub(super) origin: Position,
    pub(super) boss: bool,
    pub(super) chase_range: u32,
    pub(super) position: Position,
    pub(super) facing: CardinalDirection,
    pub(super) active: bool,
}

/// One-shot encounter pool restored after a battle outcome.
#[derive(Clone, Debug, Resource)]
pub(crate) struct WorldEncounterRestore {
    pub(crate) map_id: String,
    pub(crate) enemies: Vec<WorldEnemyReturnState>,
}

#[derive(Component, Clone, Debug, PartialEq)]
pub(crate) struct WorldEnemy {
    pub(super) encounter_id: String,
    pub(super) formation: Vec<String>,
    pub(super) origin: Position,
    pub(super) position: Position,
    pub(super) top_left: Vec2,
    pub(super) boss: bool,
    pub(super) chase_range: u32,
    pub(super) active: bool,
    pub(super) engaged: bool,
    pub(super) facing: CardinalDirection,
    pub(super) frame: u32,
    pub(super) frame_elapsed: f32,
    pub(super) wander_pause: f32,
    pub(super) wander_target: Option<Vec2>,
}

#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "runtime inspection accessors support Gate 8 diagnostics"
    )
)]
impl WorldEnemy {
    pub(crate) fn encounter_id(&self) -> &str {
        &self.encounter_id
    }

    pub(crate) const fn tile_position(&self) -> Position {
        self.position
    }

    pub(crate) fn formation(&self) -> &[String] {
        &self.formation
    }

    pub(crate) const fn is_boss(&self) -> bool {
        self.boss
    }

    pub(super) fn collision_rect(&self) -> CharacterCollisionRect {
        enemy_collision_rect(self.top_left)
    }
}

pub(super) fn reset_world_encounters(
    mut state: ResMut<WorldEncounterState>,
    mut battle: ResMut<BattleTransition>,
) {
    *state = WorldEncounterState::default();
    *battle = BattleTransition::default();
}

pub(super) fn fail_encounter(state: &mut WorldEncounterState, failure: String) {
    state.status = WorldEncounterStatus::Failed;
    state.failure = Some(failure);
}

pub(super) fn cleanup_world_encounters(
    mut commands: Commands,
    enemies: Query<Entity, With<WorldEnemy>>,
    mut state: ResMut<WorldEncounterState>,
) {
    for entity in &enemies {
        commands.entity(entity).despawn();
    }
    state.pending.clear();
    state.sprite_handles.clear();
}
