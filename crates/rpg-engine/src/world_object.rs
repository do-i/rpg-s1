//! Map-authored signs and TMX chest placements joined to cached reusable definitions.

use std::collections::BTreeSet;

use bevy::{asset::LoadState, ecs::schedule::ApplyDeferred, prelude::*};

use crate::{
    app_state::AppState,
    game_state::GameState,
    runtime_map::RuntimeMapId,
    runtime_opened_boxes::OpenedBoxKey,
    scenario_inventory::ScenarioInventory,
    scenario_item_box::{ItemBoxCatalog, item_box_placements},
    scenario_manifest::{Manifest, ManifestSigns},
    scenario_map::{
        ItemBoxLoot, ItemBoxTrap, MapMetadata, OptionalMapMetadata, optional_map_metadata,
    },
    scenario_path::ScenarioRelativePath,
    scenario_root::{SCENARIO_MANIFEST_PATH, ScenarioRoot},
    scenario_spatial::Position,
    scenario_yaml_asset::ScenarioYamlLoader,
    tile_coordinates::tmx_tile_center,
    tmx_ground_asset::{StaticMapRenderState, TmxGroundAsset, world_entity_y_z},
    tmx_header::TmxMapDocument,
    tsx_atlas_asset::TsxAtlasAsset,
};

const TILE_SIZE: u32 = 32;

pub(crate) struct WorldObjectPlugin;

impl Plugin for WorldObjectPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<ItemBoxCatalog>()
            .register_asset_loader(ScenarioYamlLoader::<ItemBoxCatalog>::new("item boxes"))
            .init_resource::<WorldObjectState>()
            .add_systems(OnEnter(AppState::World), reset_world_objects)
            .add_systems(
                Update,
                (
                    sync_world_object_request,
                    ApplyDeferred,
                    drive_world_object_load,
                )
                    .chain()
                    .in_set(crate::world_publication::WorldPublicationSet::Load)
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

#[derive(Clone, Debug, Default, Resource)]
pub(crate) struct WorldObjectState {
    map_id: Option<String>,
    flags: Vec<String>,
    metadata: Option<Handle<MapMetadata>>,
    manifest: Option<Handle<Manifest>>,
    box_atlas: Option<Handle<TsxAtlasAsset>>,
    definitions: Option<Handle<ItemBoxCatalog>>,
    status: WorldObjectStatus,
    failure: Option<String>,
}

impl WorldObjectState {
    fn fail(&mut self, reason: impl Into<String>) {
        self.status = WorldObjectStatus::Failed;
        self.failure = Some(reason.into());
    }

    pub(crate) fn publication_status(
        &self,
        map_id: &str,
    ) -> crate::world_publication::PublicationStatus {
        use crate::world_publication::PublicationStatus;
        if self.map_id.as_deref() != Some(map_id) {
            return PublicationStatus::Pending;
        }
        if let Some(reason) = &self.failure {
            return PublicationStatus::Failed(reason.clone());
        }
        if self.is_spawned_for(map_id) {
            PublicationStatus::Ready
        } else {
            PublicationStatus::Pending
        }
    }

    pub(crate) fn asset_ids(&self) -> Vec<bevy::asset::UntypedAssetId> {
        self.metadata
            .iter()
            .map(|h| h.id().untyped())
            .chain(self.manifest.iter().map(|h| h.id().untyped()))
            .chain(self.box_atlas.iter().map(|h| h.id().untyped()))
            .chain(self.definitions.iter().map(|h| h.id().untyped()))
            .collect()
    }

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
    trap: Option<ItemBoxTrap>,
    closed_tile: usize,
}

impl WorldItemBox {
    #[cfg(test)]
    pub(crate) fn for_test(
        map_id: RuntimeMapId,
        id: impl Into<String>,
        position: Position,
        loot: ItemBoxLoot,
        trap: Option<ItemBoxTrap>,
    ) -> Self {
        Self {
            map_id,
            id: id.into(),
            position,
            loot,
            trap,
            closed_tile: 0,
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

    pub(crate) const fn trap(&self) -> Option<ItemBoxTrap> {
        self.trap
    }

    pub(crate) fn key(&self) -> OpenedBoxKey {
        OpenedBoxKey::try_new(self.map_id.clone(), self.id.clone())
            .expect("validated map and box ids must form a key")
    }
}

fn reset_world_objects(mut state: ResMut<WorldObjectState>) {
    *state = WorldObjectState::default();
}

#[expect(
    clippy::too_many_arguments,
    reason = "atomic Bevy object replacement keeps each resource and entity boundary explicit"
)]
fn sync_world_object_request(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    scenario_root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
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
    state.map_id = Some(map_id.to_owned());
    let Some(metadata_path) = inventory.map_metadata_path(map_id) else {
        state.fail("object metadata path is invalid");
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
    definitions: Res<Assets<ItemBoxCatalog>>,
    atlases: Res<Assets<TsxAtlasAsset>>,
    maps: Res<Assets<TmxGroundAsset>>,
    render: Res<StaticMapRenderState>,
    scenario_root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
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
        state.fail("object metadata or manifest handle is missing");
        return;
    };
    if matches!(
        asset_server.load_state(manifest_handle.id()),
        LoadState::Failed(_)
    ) {
        state.fail("object manifest failed to load");
        return;
    }
    // Only metadata may be absent (a TMX-only map); the manifest has no such exception.
    let metadata = match optional_map_metadata(&asset_server, &metadata_assets, metadata_handle) {
        OptionalMapMetadata::Ready(metadata) => Some(metadata),
        OptionalMapMetadata::Loading => None,
        OptionalMapMetadata::Failed => {
            state.fail("object metadata failed to load");
            return;
        }
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
        state.fail("object metadata id does not match the destination");
        return;
    }
    if let Some(path) = &manifest.refs.item_boxes {
        if state.definitions.is_none() {
            state.definitions = Some(asset_server.load(scenario_root.resolve(path)));
            return;
        }
        let handle = state.definitions.as_ref().expect("requested definitions");
        if crate::world_publication::asset_failed(&asset_server, handle.id()) {
            state.fail("item-box definitions failed to load");
            return;
        }
        if definitions.get(handle).is_none() {
            return;
        }
    }
    if manifest.refs.item_boxes.is_none() {
        if state.box_atlas.is_none() {
            state.box_atlas =
                Some(asset_server.load(scenario_root.resolve(&manifest.item_box.sprite)));
            return;
        }
        let box_atlas_handle = state.box_atlas.as_ref().expect("set above");
        if crate::world_publication::asset_failed(&asset_server, box_atlas_handle.id()) {
            state.fail("item-box sprite or image dependency failed to load");
            return;
        }
        if !asset_server.is_loaded_with_dependencies(box_atlas_handle.id()) {
            return;
        }
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
                dialogue_id: sign_dialogue_id(map_id, position, &inventory.dialogue_ids),
                position,
            },
        ));
    }
    let map_id_value = RuntimeMapId::try_new(map_id.to_owned())
        .expect("GameState current map id is validated nonempty");
    let box_atlas = state
        .box_atlas
        .as_ref()
        .and_then(|handle| atlases.get(handle));
    if let Some(catalog) = state
        .definitions
        .as_ref()
        .and_then(|handle| definitions.get(handle))
    {
        let placements = match item_box_placements(map.document()) {
            Ok(placements) => placements,
            Err(error) => {
                state.fail(error);
                return;
            }
        };
        // Resolve every definition and image before publishing any chest entities.
        let mut resolved = Vec::new();
        for placement in placements {
            let Some(definition) = catalog.item_boxes.get(&placement.definition_id) else {
                state.fail(format!(
                    "item-box `{}` references unknown definition `{}`",
                    placement.instance_id, placement.definition_id
                ));
                return;
            };
            if !game.flags().satisfies(&definition.present) {
                continue;
            }
            let key = OpenedBoxKey::try_new(map_id_value.clone(), placement.instance_id.clone())
                .expect("validated chest identity");
            let opened = game.opened_boxes().contains(&key);
            let (sprite, rotation, closed_tile) =
                match map.item_box_sprite(placement.gid, opened, &atlases) {
                    Ok(image) => image,
                    Err(error) => {
                        state.fail(error);
                        return;
                    }
                };
            let center = Vec2::new(
                placement.x as f32 + TILE_SIZE as f32 / 2.0,
                -(placement.y as f32 + TILE_SIZE as f32 / 2.0),
            );
            let above_foreground =
                map.document()
                    .tile_layers()
                    .iter()
                    .enumerate()
                    .any(|(index, layer)| {
                        layer.name() == "foreground" && index < placement.tile_layer_index
                    });
            let z = if above_foreground {
                world_entity_y_z(
                    -(map.document().header().height() as f32 * TILE_SIZE as f32),
                    64.0,
                ) + 3.0
            } else {
                world_entity_y_z(center.y, 16.0)
            };
            resolved.push((
                sprite,
                Transform::from_translation(center.extend(z)).with_rotation(rotation),
                WorldItemBox {
                    map_id: map_id_value.clone(),
                    id: placement.instance_id,
                    position: placement.position,
                    loot: definition.loot.clone(),
                    trap: definition.trap,
                    closed_tile,
                },
            ));
        }
        commands.spawn_batch(resolved);
    } else {
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
                .unwrap_or_else(|| {
                    Sprite::from_color(Color::srgb_u8(140, 90, 30), Vec2::splat(32.0))
                });
            let center = tile_center(item_box.position);
            commands.spawn((
                sprite,
                Transform::from_translation(center.extend(world_entity_y_z(center.y, 16.0))),
                WorldItemBox {
                    map_id: map_id_value.clone(),
                    id: item_box.id.clone(),
                    position: item_box.position,
                    loot: item_box.loot.clone(),
                    trap: item_box.trap,
                    closed_tile: 0,
                },
            ));
        }
    }
    state.status = WorldObjectStatus::Spawned;
}

/// A board can override the map's shared message without depending on sign discovery order.
pub(crate) fn sign_dialogue_id(
    map_id: &str,
    position: Position,
    dialogue_ids: &BTreeSet<String>,
) -> String {
    let specific = format!("sign_{map_id}_{}_{}", position.x, position.y);
    if dialogue_ids.contains(&specific) {
        specific
    } else {
        format!("sign_{map_id}")
    }
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
            atlas.index =
                item_box.closed_tile + usize::from(game.opened_boxes().contains(&item_box.key()));
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
    fn sign_dialogue_prefers_its_position_and_preserves_the_map_fallback() {
        let ids = BTreeSet::from(["sign_village".to_owned(), "sign_village_2_3".to_owned()]);
        assert_eq!(
            sign_dialogue_id("village", Position::new(2, 3), &ids),
            "sign_village_2_3"
        );
        assert_eq!(
            sign_dialogue_id("village", Position::new(3, 2), &ids),
            "sign_village"
        );
        assert_eq!(
            sign_dialogue_id("other", Position::new(2, 3), &ids),
            "sign_other"
        );
    }

    #[test]
    fn zone_one_and_two_boards_have_individual_readable_dialogues() {
        let package = crate::test_support::scenario_package_dir();
        let assets = package.parent().unwrap().parent().unwrap();
        let inventory = ScenarioInventory::discover(assets, &ScenarioRoot::default());
        assert!(inventory.failure.is_none(), "{:?}", inventory.failure);
        let manifest: Manifest = crate::scenario_yaml::from_str(
            &std::fs::read_to_string(package.join("manifest.yaml")).unwrap(),
        )
        .unwrap();
        for (map_id, expected) in [
            ("zone_01_starting_forest", &[(2, 24), (27, 2)][..]),
            ("zone_02_open_plains_cave_02", &[(6, 27), (57, 17)][..]),
            (
                "zone_02_open_plains",
                &[(3, 3), (5, 18), (10, 7), (24, 3), (29, 3)][..],
            ),
        ] {
            let path = format!("media/maps/{map_id}.tmx");
            let owner = ScenarioRelativePath::try_from(path.as_str()).unwrap();
            let document = parse_tmx_map_document(
                &std::fs::read_to_string(package.join(&path)).unwrap(),
                &owner,
            )
            .unwrap();
            let positions = sign_tiles(&document, &manifest.signs);
            assert_eq!(
                positions,
                expected
                    .iter()
                    .map(|&(x, y)| Position::new(x, y))
                    .collect::<Vec<_>>()
            );
            for position in positions {
                let dialogue = sign_dialogue_id(map_id, position, &inventory.dialogue_ids);
                assert_eq!(
                    dialogue,
                    format!("sign_{map_id}_{}_{}", position.x, position.y)
                );
                let text =
                    std::fs::read_to_string(package.join(format!("data/dialogue/{dialogue}.yaml")))
                        .unwrap();
                crate::scenario_yaml::from_str::<crate::scenario_dialogue::DialogueDocument>(&text)
                    .unwrap();
            }
        }
    }

    #[test]
    fn known_ardel_sign_cells_route_to_the_configured_tileset_profile() {
        let owner = ScenarioRelativePath::try_from("media/maps/town_01_ardel.tmx").unwrap();
        let map = parse_tmx_map_document(
            include_str!(scenario_file!("media/maps/town_01_ardel.tmx")),
            &owner,
        )
        .unwrap();
        let manifest: Manifest =
            crate::scenario_yaml::from_str(include_str!(scenario_file!("manifest.yaml"))).unwrap();
        assert_eq!(
            sign_tiles(&map, &manifest.signs),
            [
                Position::new(16, 4),
                Position::new(19, 16),
                Position::new(27, 11)
            ]
        );
    }

    #[test]
    fn ruinwatch_sign_cells_route_to_the_configured_tileset_profile() {
        let owner = ScenarioRelativePath::try_from("media/maps/town_03_ruinwatch.tmx").unwrap();
        let map = parse_tmx_map_document(
            include_str!(scenario_file!("media/maps/town_03_ruinwatch.tmx")),
            &owner,
        )
        .unwrap();
        let manifest: Manifest =
            crate::scenario_yaml::from_str(include_str!(scenario_file!("manifest.yaml"))).unwrap();
        assert_eq!(
            sign_tiles(&map, &manifest.signs),
            [Position::new(5, 0), Position::new(17, 28)]
        );
    }
}
