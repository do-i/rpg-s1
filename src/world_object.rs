//! Map-authored readable signs and persistent item boxes.

use std::collections::BTreeSet;

use bevy::{asset::LoadState, ecs::schedule::ApplyDeferred, prelude::*};

use crate::{
    app_state::AppState,
    game_state::GameState,
    runtime_map::RuntimeMapId,
    runtime_opened_boxes::OpenedBoxKey,
    scenario_manifest::{Manifest, ManifestSigns},
    scenario_map::{ItemBoxLoot, MapMetadata, optional_scenario_asset_is_missing},
    scenario_path::ScenarioRelativePath,
    scenario_root::{SCENARIO_MANIFEST_PATH, ScenarioRoot},
    scenario_spatial::Position,
    tile_coordinates::tmx_tile_center,
    tmx_ground_asset::{StaticMapRenderState, TmxGroundAsset, world_entity_y_z},
    tmx_header::TmxMapDocument,
    tsx_atlas_asset::TsxAtlasAsset,
};

const TILE_SIZE: u32 = 32;

pub(crate) struct WorldObjectPlugin;

impl Plugin for WorldObjectPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldObjectState>()
            .add_systems(OnEnter(AppState::World), reset_world_objects)
            .add_systems(
                Update,
                (
                    sync_world_object_request,
                    ApplyDeferred,
                    drive_world_object_load,
                )
                    .chain()
                    .run_if(in_state(AppState::World)),
            )
            .add_systems(
                Update,
                sync_opened_box_sprites.run_if(in_state(AppState::World)),
            )
            .add_systems(OnExit(AppState::World), cleanup_world_objects);
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum WorldObjectStatus {
    #[default]
    Idle,
    Loading,
    Spawned,
    Failed,
}

#[derive(Debug, Default, Resource)]
pub(crate) struct WorldObjectState {
    map_id: Option<String>,
    flags: Vec<String>,
    metadata: Option<Handle<MapMetadata>>,
    manifest: Option<Handle<Manifest>>,
    box_atlas: Option<Handle<TsxAtlasAsset>>,
    status: WorldObjectStatus,
}

impl WorldObjectState {
    pub(crate) fn is_spawned_for(&self, map_id: &str) -> bool {
        self.map_id.as_deref() == Some(map_id) && self.status == WorldObjectStatus::Spawned
    }
}

#[derive(Component, Debug)]
pub(crate) struct WorldSign {
    id: String,
    dialogue_id: String,
    position: Position,
}

impl WorldSign {
    #[cfg(test)]
    pub(crate) fn for_test(
        id: impl Into<String>,
        dialogue_id: impl Into<String>,
        position: Position,
    ) -> Self {
        Self {
            id: id.into(),
            dialogue_id: dialogue_id.into(),
            position,
        }
    }

    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn dialogue_id(&self) -> &str {
        &self.dialogue_id
    }

    pub(crate) const fn tile_position(&self) -> Position {
        self.position
    }
}

#[derive(Component, Debug)]
pub(crate) struct WorldItemBox {
    map_id: RuntimeMapId,
    id: String,
    position: Position,
    loot: ItemBoxLoot,
}

impl WorldItemBox {
    #[cfg(test)]
    pub(crate) fn for_test(
        map_id: RuntimeMapId,
        id: impl Into<String>,
        position: Position,
        loot: ItemBoxLoot,
    ) -> Self {
        Self {
            map_id,
            id: id.into(),
            position,
            loot,
        }
    }

    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    pub(crate) const fn tile_position(&self) -> Position {
        self.position
    }

    pub(crate) const fn loot(&self) -> &ItemBoxLoot {
        &self.loot
    }

    pub(crate) fn key(&self) -> OpenedBoxKey {
        OpenedBoxKey::try_new(self.map_id.clone(), self.id.clone())
            .expect("validated map and box ids must form a key")
    }
}

fn reset_world_objects(mut state: ResMut<WorldObjectState>) {
    *state = WorldObjectState::default();
}

fn sync_world_object_request(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    scenario_root: Res<ScenarioRoot>,
    game: Option<Res<GameState>>,
    signs: Query<Entity, With<WorldSign>>,
    boxes: Query<Entity, With<WorldItemBox>>,
    mut state: ResMut<WorldObjectState>,
) {
    let current = game
        .as_deref()
        .and_then(|game| game.map().current())
        .map(|map| map.as_str());
    let flags = game
        .as_deref()
        .map(|game| game.flags().iter().map(str::to_owned).collect::<Vec<_>>())
        .unwrap_or_default();
    if current == state.map_id.as_deref() && flags == state.flags {
        return;
    }
    for entity in &signs {
        commands.entity(entity).despawn();
    }
    for entity in &boxes {
        commands.entity(entity).despawn();
    }
    *state = WorldObjectState::default();
    let Some(map_id) = current else {
        return;
    };
    let Ok(metadata_path) =
        ScenarioRelativePath::try_from(format!("data/maps/{map_id}.yaml").as_str())
    else {
        state.status = WorldObjectStatus::Failed;
        return;
    };
    state.map_id = Some(map_id.to_owned());
    state.flags = flags;
    state.metadata = Some(asset_server.load(scenario_root.resolve(&metadata_path)));
    state.manifest = Some(asset_server.load(scenario_root.resolve(
        &ScenarioRelativePath::try_from(SCENARIO_MANIFEST_PATH).expect("canonical manifest path"),
    )));
    state.status = WorldObjectStatus::Loading;
}

#[expect(
    clippy::too_many_arguments,
    reason = "atomic sign and item-box publication needs each independent Bevy asset boundary"
)]
fn drive_world_object_load(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    metadata_assets: Res<Assets<MapMetadata>>,
    manifests: Res<Assets<Manifest>>,
    atlases: Res<Assets<TsxAtlasAsset>>,
    maps: Res<Assets<TmxGroundAsset>>,
    render: Res<StaticMapRenderState>,
    scenario_root: Res<ScenarioRoot>,
    game: Option<Res<GameState>>,
    existing_signs: Query<(), With<WorldSign>>,
    existing_boxes: Query<(), With<WorldItemBox>>,
    mut state: ResMut<WorldObjectState>,
) {
    if state.status != WorldObjectStatus::Loading {
        return;
    }
    let (Some(metadata_handle), Some(manifest_handle)) =
        (state.metadata.as_ref(), state.manifest.as_ref())
    else {
        state.status = WorldObjectStatus::Failed;
        return;
    };
    if matches!(
        asset_server.load_state(manifest_handle.id()),
        LoadState::Failed(_)
    ) {
        state.status = WorldObjectStatus::Failed;
        return;
    }
    // A missing `data/maps/<id>.yaml` is a valid runtime state for a TMX-only map (the pinned
    // engine's `load_yaml_optional`, see `MapMetadata::empty`), not a load failure; only a real
    // reader/parse error is fatal here. The manifest itself has no such exception.
    let empty_metadata;
    let metadata = match asset_server.load_state(metadata_handle.id()) {
        LoadState::Failed(error) if optional_scenario_asset_is_missing(&error) => {
            empty_metadata = MapMetadata::empty();
            Some(&empty_metadata)
        }
        LoadState::Failed(_) => {
            state.status = WorldObjectStatus::Failed;
            return;
        }
        _ => metadata_assets.get(metadata_handle),
    };
    let (Some(metadata), Some(manifest), Some(game), Some(map)) = (
        metadata,
        manifests.get(manifest_handle),
        game.as_deref(),
        render.map(&maps),
    ) else {
        return;
    };
    let Some(map_id) = state.map_id.as_deref() else {
        return;
    };
    if metadata.effective_id(map_id) != map_id {
        state.status = WorldObjectStatus::Failed;
        return;
    }
    if state.box_atlas.is_none() {
        state.box_atlas = Some(asset_server.load(scenario_root.resolve(&manifest.item_box.sprite)));
        return;
    }
    let box_atlas_handle = state.box_atlas.as_ref().expect("set above");
    if matches!(
        asset_server.load_state(box_atlas_handle.id()),
        LoadState::Failed(_)
    ) {
        state.status = WorldObjectStatus::Failed;
        return;
    }
    if !asset_server.is_loaded_with_dependencies(box_atlas_handle.id()) {
        return;
    }
    if !existing_signs.is_empty() || !existing_boxes.is_empty() {
        state.status = WorldObjectStatus::Spawned;
        return;
    }

    for (index, position) in sign_tiles(map.document(), &manifest.signs)
        .into_iter()
        .enumerate()
    {
        commands.spawn((
            Transform::from_translation(tile_center(position).extend(0.0)),
            WorldSign {
                id: format!("sign_{map_id}_{index}"),
                dialogue_id: format!("sign_{map_id}"),
                position,
            },
        ));
    }
    let map_id_value = RuntimeMapId::try_new(map_id.to_owned())
        .expect("GameState current map id is validated nonempty");
    let box_atlas = atlases.get(box_atlas_handle);
    for item_box in metadata
        .item_boxes
        .iter()
        .filter(|item_box| game.flags().satisfies(&item_box.present))
    {
        let opened = game.opened_boxes().contains(
            &OpenedBoxKey::try_new(map_id_value.clone(), item_box.id.clone())
                .expect("source box id is nonempty"),
        );
        let sprite = box_atlas
            .and_then(|atlas| atlas.sprite_for_tile(u32::from(opened)).ok())
            .unwrap_or_else(|| Sprite::from_color(Color::srgb_u8(140, 90, 30), Vec2::splat(32.0)));
        let center = tile_center(item_box.position);
        commands.spawn((
            sprite,
            Transform::from_translation(center.extend(world_entity_y_z(center.y, 16.0))),
            WorldItemBox {
                map_id: map_id_value.clone(),
                id: item_box.id.clone(),
                position: item_box.position,
                loot: item_box.loot.clone(),
            },
        ));
    }
    state.status = WorldObjectStatus::Spawned;
}

pub(crate) fn sign_tiles(document: &TmxMapDocument, signs: &ManifestSigns) -> Vec<Position> {
    let Some(reference_index) = document.external_tilesets().iter().position(|reference| {
        reference
            .source()
            .as_str()
            .rsplit('/')
            .next()
            .and_then(|filename| filename.strip_suffix(".tsx"))
            == Some(signs.tileset.as_str())
    }) else {
        return Vec::new();
    };
    let references = document.external_tilesets();
    let first_gid = references[reference_index].first_gid();
    let exclusive_end = references
        .get(reference_index + 1)
        .map_or(u32::MAX, |reference| reference.first_gid());
    let configured = signs.tile_ids.iter().copied().collect::<BTreeSet<_>>();
    let mut positions = BTreeSet::new();
    for layer in document.tile_layers() {
        for row in 0..layer.height() {
            for column in 0..layer.width() {
                let gid = layer.gid_at(column, row).expect("finite layer coordinate");
                if gid.global_id() >= first_gid
                    && gid.global_id() < exclusive_end
                    && configured.contains(&(gid.global_id() - first_gid))
                {
                    positions.insert((column, row));
                }
            }
        }
    }
    positions
        .into_iter()
        .map(|(column, row)| Position::new(column as i32, row as i32))
        .collect()
}

fn sync_opened_box_sprites(
    game: Option<Res<GameState>>,
    mut boxes: Query<(&WorldItemBox, &mut Sprite)>,
) {
    let Some(game) = game else {
        return;
    };
    for (item_box, mut sprite) in &mut boxes {
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = usize::from(game.opened_boxes().contains(&item_box.key()));
        }
    }
}

fn tile_center(position: Position) -> Vec2 {
    tmx_tile_center(
        u32::try_from(position.x).unwrap_or_default(),
        u32::try_from(position.y).unwrap_or_default(),
        TILE_SIZE,
        TILE_SIZE,
    )
}

fn cleanup_world_objects(
    mut commands: Commands,
    signs: Query<Entity, With<WorldSign>>,
    boxes: Query<Entity, With<WorldItemBox>>,
    mut state: ResMut<WorldObjectState>,
) {
    for entity in &signs {
        commands.entity(entity).despawn();
    }
    for entity in &boxes {
        commands.entity(entity).despawn();
    }
    *state = WorldObjectState::default();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{scenario_path::ScenarioRelativePath, tmx_header::parse_tmx_map_document};

    #[test]
    fn known_ardel_sign_cells_route_to_the_configured_tileset_profile() {
        let owner = ScenarioRelativePath::try_from("assets/maps/town_01_ardel.tmx").unwrap();
        let map = parse_tmx_map_document(
            include_str!("../assets/scenarios/rusted_kingdoms/assets/maps/town_01_ardel.tmx"),
            &owner,
        )
        .unwrap();
        let signs = ManifestSigns {
            tileset: "stone_tile_stares_16x16".into(),
            tile_ids: vec![18, 19, 20, 21],
        };
        assert_eq!(
            sign_tiles(&map, &signs),
            [
                Position::new(16, 4),
                Position::new(19, 16),
                Position::new(27, 11)
            ]
        );
    }
}
