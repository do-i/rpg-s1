//! Active-map collision data shared by every moving World entity.

use bevy::{asset::LoadState, prelude::*};

use crate::{
    app_state::AppState, game_state::GameState, scenario_inventory::ScenarioInventory,
    scenario_root::ScenarioRoot, scenario_spatial::collision_occupancy::CollisionOccupancy,
    tmx_ground_asset::TmxGroundAsset,
};

pub(crate) struct WorldCollisionPlugin;

impl Plugin for WorldCollisionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldCollision>()
            .add_systems(
                Update,
                load_world_collision.run_if(in_state(AppState::World)),
            )
            .add_systems(OnExit(AppState::World), clear_world_collision);
    }
}

#[derive(Resource, Default)]
pub(crate) struct WorldCollision {
    pub(super) map_id: Option<String>,
    pub(super) handle: Option<Handle<TmxGroundAsset>>,
    pub(super) occupancy: Option<CollisionOccupancy>,
    pub(super) failed: bool,
}

impl WorldCollision {
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
    if matches!(asset_server.load_state(handle.id()), LoadState::Failed(_)) {
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
