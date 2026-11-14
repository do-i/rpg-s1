//! Active-map collision data shared by every moving World entity.

use bevy::prelude::*;

use crate::{
    app_state::AppState,
    game_state::GameState,
    scenario_inventory::ScenarioInventory,
    scenario_root::ScenarioRoot,
    scenario_spatial::collision_occupancy::CollisionOccupancy,
    tmx_ground_asset::TmxGroundAsset,
    world_player::{
        CHARACTER_COLLISION_HEIGHT, CHARACTER_COLLISION_OFFSET_X, CHARACTER_COLLISION_OFFSET_Y,
        CHARACTER_COLLISION_WIDTH, CharacterCollisionRect,
    },
};

pub(crate) struct WorldCollisionPlugin;

impl Plugin for WorldCollisionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldCollision>()
            .add_systems(
                Update,
                load_world_collision
                    .in_set(crate::world_publication::WorldPublicationSet::Load)
                    .run_if(in_state(AppState::World)),
            )
            .add_systems(OnExit(AppState::World), clear_world_collision);
    }
}

/// Collision box of a character whose sprite top-left is `top_left` in source pixels.
///
/// Anchoring is the caller's business: players and enemies keep a collision-centred `top_left`,
/// NPCs keep the tile-corner one; both are the same uncentered sprite origin, so one fixed
/// 20x18 box at (22, 41) from it serves all of them.
pub(crate) fn character_collision_rect(top_left: Vec2) -> CharacterCollisionRect {
    CharacterCollisionRect {
        x: top_left.x + CHARACTER_COLLISION_OFFSET_X,
        y: top_left.y + CHARACTER_COLLISION_OFFSET_Y,
        width: CHARACTER_COLLISION_WIDTH,
        height: CHARACTER_COLLISION_HEIGHT,
    }
}

#[derive(Clone, Resource, Default)]
pub(crate) struct WorldCollision {
    pub(super) map_id: Option<String>,
    pub(super) handle: Option<Handle<TmxGroundAsset>>,
    pub(super) occupancy: Option<CollisionOccupancy>,
    pub(super) failed: bool,
}

impl WorldCollision {
    pub(crate) fn publication_status(
        &self,
        map_id: &str,
    ) -> crate::world_publication::PublicationStatus {
        use crate::world_publication::PublicationStatus;
        if self.map_id.as_deref() != Some(map_id) {
            return PublicationStatus::Pending;
        }
        if self.failed {
            return PublicationStatus::Failed("map collision failed to load".into());
        }
        if self.occupancy.is_some() {
            PublicationStatus::Ready
        } else {
            PublicationStatus::Pending
        }
    }

    pub(crate) fn occupancy_for(&self, map_id: &str) -> Option<&CollisionOccupancy> {
        self.occupancy
            .as_ref()
            .filter(|_| self.map_id.as_deref() == Some(map_id) && !self.failed)
    }

    fn reset_for_map(
        &mut self,
        map_id: &str,
        asset_server: &AssetServer,
        scenario_root: &ScenarioRoot,
        inventory: &ScenarioInventory,
    ) {
        self.map_id = Some(map_id.to_owned());
        self.handle = None;
        self.occupancy = None;
        self.failed = false;

        let Some(logical) = inventory.tmx_path(map_id) else {
            self.failed = true;
            return;
        };
        self.handle = Some(asset_server.load(scenario_root.resolve(&logical)));
    }

    #[cfg(test)]
    pub(crate) fn loaded_for(map_id: &str, occupancy: CollisionOccupancy) -> Self {
        Self {
            map_id: Some(map_id.to_owned()),
            occupancy: Some(occupancy),
            ..default()
        }
    }
}

fn load_world_collision(
    asset_server: Option<Res<AssetServer>>,
    scenario_root: Option<Res<ScenarioRoot>>,
    inventory: Option<Res<ScenarioInventory>>,
    maps: Option<Res<Assets<TmxGroundAsset>>>,
    game: Option<Res<GameState>>,
    mut collision: ResMut<WorldCollision>,
) {
    let Some(game) = game else {
        *collision = WorldCollision::default();
        return;
    };
    let Some(current) = game.map().current() else {
        *collision = WorldCollision::default();
        return;
    };
    let map_id = current.as_str();
    let asset_server = asset_server.as_deref();
    let scenario_root = scenario_root.as_deref();
    let maps = maps.as_deref();

    if collision.map_id.as_deref() != Some(map_id) {
        let (Some(asset_server), Some(scenario_root), Some(inventory), Some(_)) =
            (asset_server, scenario_root, inventory.as_deref(), maps)
        else {
            *collision = WorldCollision {
                map_id: Some(map_id.to_owned()),
                ..default()
            };
            return;
        };
        collision.reset_for_map(map_id, asset_server, scenario_root, inventory);
    }
    if collision.failed || collision.occupancy.is_some() {
        return;
    }

    let Some(handle) = collision.handle.as_ref() else {
        return;
    };
    let Some(asset_server) = asset_server else {
        return;
    };
    if crate::world_publication::asset_failed(asset_server, handle.id()) {
        collision.failed = true;
        return;
    }
    let Some(maps) = maps else {
        return;
    };
    let Some(map) = maps.get(handle) else {
        return;
    };
    match CollisionOccupancy::from_tmx_document(map.document()) {
        Ok(occupancy) => collision.occupancy = Some(occupancy),
        Err(_) => collision.failed = true,
    }
}

fn clear_world_collision(mut collision: ResMut<WorldCollision>) {
    *collision = WorldCollision::default();
}

#[cfg(test)]
mod character_collision_rect_tests {
    use super::*;

    /// The four pre-consolidation copies (player, NPC, enemy, movement) all spelled this.
    fn old_formula(top_left: Vec2) -> CharacterCollisionRect {
        CharacterCollisionRect {
            x: top_left.x + CHARACTER_COLLISION_OFFSET_X,
            y: top_left.y + CHARACTER_COLLISION_OFFSET_Y,
            width: CHARACTER_COLLISION_WIDTH,
            height: CHARACTER_COLLISION_HEIGHT,
        }
    }

    #[test]
    fn shared_rect_is_bit_identical_to_the_old_per_module_formula() {
        let coords = [
            -4096.0, -100.5, -22.0, -0.1, 0.0, 0.1, 0.333, 15.999, 22.0, 41.0, 1000.7, 4095.999,
            4096.0, 4096.5, 1.0e6,
        ];
        for x in coords {
            for y in coords {
                let top_left = Vec2::new(x, y);
                let new = character_collision_rect(top_left);
                let old = old_formula(top_left);
                assert_eq!(new.x.to_bits(), old.x.to_bits());
                assert_eq!(new.y.to_bits(), old.y.to_bits());
                assert_eq!(new.width.to_bits(), old.width.to_bits());
                assert_eq!(new.height.to_bits(), old.height.to_bits());
            }
        }
    }
}
