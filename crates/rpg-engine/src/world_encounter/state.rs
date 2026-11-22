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
    world_publication::WorldLayerState,
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
        // Not map-gated, and a map with no encounters is Ready without being spawned.
        self.publication_from(
            self.map_id.as_deref() == Some(map_id)
                && matches!(
                    self.status,
                    WorldEncounterStatus::Spawned | WorldEncounterStatus::NoEncounters
                ),
        )
    }

    /// Whether `map_id` has wandering enemies, or `None` while its encounters are unresolved.
    pub(crate) fn ambient_encounters(&self, map_id: &str) -> Option<bool> {
        if self.map_id.as_deref() != Some(map_id) {
            return None;
        }
        match self.status {
            WorldEncounterStatus::Spawned => Some(true),
            WorldEncounterStatus::NoEncounters => Some(false),
            _ => None,
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

    #[cfg(test)]
    pub(crate) const fn status(&self) -> WorldEncounterStatus {
        self.status
    }

    #[cfg(test)]
    pub(crate) fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }
}

impl WorldLayerState for WorldEncounterState {
    type Status = WorldEncounterStatus;
    type Failure = String;
    const SPAWNED: Self::Status = WorldEncounterStatus::Spawned;
    const FAILED: Self::Status = WorldEncounterStatus::Failed;

    fn layer_map_id(&self) -> Option<&str> {
        self.map_id.as_deref()
    }

    fn layer_status(&self) -> Self::Status {
        self.status
    }

    fn layer_failure(&self) -> Option<&Self::Failure> {
        self.failure.as_ref()
    }

    fn layer_status_and_failure_mut(&mut self) -> (&mut Self::Status, &mut Option<Self::Failure>) {
        (&mut self.status, &mut self.failure)
    }
}

#[cfg(test)]
impl crate::world_publication::PublicationFixture for WorldEncounterState {
    const FAILURE: &'static str = "invalid encounter map id `map_a`";

    fn failure_reason() -> String {
        Self::FAILURE.to_owned()
    }

    fn loading(map_id: &str) -> Self {
        Self {
            map_id: Some(map_id.to_owned()),
            status: WorldEncounterStatus::Loading,
            ..Self::default()
        }
    }

    fn spawned(map_id: &str) -> Self {
        Self {
            status: WorldEncounterStatus::Spawned,
            ..Self::loading(map_id)
        }
    }

    fn probe_status(&self, map_id: &str) -> crate::world_publication::PublicationStatus {
        self.publication_status(map_id)
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
    /// Whether the last simulation step had this enemy chasing, so only a chase start warns.
    pub(super) chasing: bool,
}

impl WorldEnemy {
    #[cfg(test)]
    pub(crate) fn encounter_id(&self) -> &str {
        &self.encounter_id
    }

    #[cfg(test)]
    pub(crate) const fn tile_position(&self) -> Position {
        self.position
    }

    #[cfg(test)]
    pub(crate) fn formation(&self) -> &[String] {
        &self.formation
    }

    pub(crate) const fn is_boss(&self) -> bool {
        self.boss
    }

    pub(crate) const fn is_active(&self) -> bool {
        self.active
    }

    pub(crate) const fn is_engaged(&self) -> bool {
        self.engaged
    }

    /// The centre of the collision rectangle, in source pixels.
    pub(crate) fn centre_px(&self) -> Vec2 {
        let rect = self.collision_rect();
        Vec2::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
    }

    #[cfg(test)]
    pub(crate) fn for_test(tile: Position, boss: bool, active: bool, engaged: bool) -> Self {
        Self {
            encounter_id: "test".to_owned(),
            formation: Vec::new(),
            origin: tile,
            position: tile,
            top_left: super::simulation::enemy_top_left(tile),
            boss,
            chase_range: 0,
            active,
            engaged,
            facing: CardinalDirection::Down,
            frame: 0,
            frame_elapsed: 0.0,
            wander_pause: 0.0,
            wander_target: None,
            chasing: false,
        }
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
    state.fail(failure);
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
