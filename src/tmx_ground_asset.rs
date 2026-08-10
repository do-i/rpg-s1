//! Bevy-native loading and ECS projection for static visible TMX tile layers.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "M4.11-M4.13 establish static map entities before later M4 world activation consumes them"
    )
)]

use std::{collections::BTreeMap, error::Error, fmt, path::PathBuf, str};

use bevy::{
    asset::{
        Asset, AssetApp, AssetLoader, AssetPath, AssetServer, Assets, Handle, LoadContext,
        LoadState, io::Reader,
    },
    ecs::{bundle::Bundle, component::Component},
    prelude::{
        App, Commands, Entity, FromWorld, IntoScheduleConfigs, OnEnter, OnExit, Plugin, PostUpdate,
        Query, Res, ResMut, Resource, Sprite, Transform, Update, Vec2, Vec3, With, World, in_state,
    },
    reflect::TypePath,
};

use crate::{
    app_state::AppState,
    game_state::GameState,
    gameplay_canvas::{
        camera_follow::{MapCameraFollow, MapPixelBounds, follow_map_camera},
        fixed_gameplay_camera,
    },
    scenario_path::{ScenarioRelativePath, ScenarioRelativePathError},
    scenario_root::ScenarioRoot,
    tile_coordinates::tmx_tile_center,
    tmx_header::{
        TmxGidResolutionError, TmxMapDocument, TmxMapDocumentError, TmxTilesetRanges,
        parse_tmx_map_document,
    },
    tsx_atlas_asset::{TsxAtlasAsset, TsxAtlasTileError},
};

pub(crate) const ARDEL_TMX_PATH: &str = "assets/maps/town_01_ardel.tmx";
const GROUND_LAYER_NAME: &str = "ground";
const COLLISION_LAYER_NAME: &str = "collision";
const GROUND_Z: f32 = 0.0;
const LAYER_Z_STEP: f32 = 1.0;
const Y_SORT_Z_BASE: f32 = 10.0;
const Y_SORT_Z_PER_PIXEL: f32 = 0.001;
const Y_SORT_SOURCE_TIE: f32 = 0.000_01;
const FOREGROUND_Z_BASE: f32 = 900.0;

/// Registers the project-owned `.tmx` static-map asset loader.
pub(crate) struct TmxGroundAssetPlugin;

impl Plugin for TmxGroundAssetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScenarioRoot>()
            .init_asset::<TmxGroundAsset>()
            .init_asset_loader::<TmxGroundAssetLoader>()
            .init_resource::<StaticMapRenderState>()
            .add_systems(OnEnter(AppState::World), begin_static_map_load)
            .add_systems(Update, spawn_static_map.run_if(in_state(AppState::World)))
            .add_systems(
                PostUpdate,
                follow_map_camera.run_if(in_state(AppState::World)),
            )
            .add_systems(OnExit(AppState::World), cleanup_static_map);
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum StaticMapRenderStatus {
    #[default]
    Idle,
    Loading,
    Spawned,
    Failed,
}

/// Strong handle and publication state for the active World's static map.
#[derive(Debug, Default, Resource)]
struct StaticMapRenderState {
    handle: Option<Handle<TmxGroundAsset>>,
    status: StaticMapRenderStatus,
}

/// A parsed TMX document with the atlases used by visible tile layers requested from Bevy.
#[derive(Asset, Debug, TypePath)]
pub(crate) struct TmxGroundAsset {
    document: TmxMapDocument,
    ground_layer_index: usize,
    visible_layer_indices: Vec<usize>,
    atlas_reference_indices: Vec<usize>,
    #[dependency]
    atlases: Vec<Handle<TsxAtlasAsset>>,
}

impl TmxGroundAsset {
    pub(crate) const fn document(&self) -> &TmxMapDocument {
        &self.document
    }

    pub(crate) fn ground_layer(&self) -> &crate::tmx_header::TmxTileLayer {
        &self.document.tile_layers()[self.ground_layer_index]
    }

    pub(crate) fn atlas_handles(&self) -> &[Handle<TsxAtlasAsset>] {
        &self.atlases
    }

    pub(crate) fn pixel_bounds(&self) -> MapPixelBounds {
        let header = self.document.header();
        let width = header.width() as f32 * header.tile_width() as f32;
        let height = header.height() as f32 * header.tile_height() as f32;
        MapPixelBounds::from_min_size(Vec2::new(0.0, -height), Vec2::new(width, height))
            .expect("validated positive u32 TMX dimensions must produce finite pixel bounds")
    }

    /// Projects every non-empty ground cell to a Bevy sprite bundle in TMX row-major order.
    pub(crate) fn ground_bundles(
        &self,
        loaded_atlases: &Assets<TsxAtlasAsset>,
    ) -> Result<Vec<StaticMapTileBundle>, StaticLayerBuildError> {
        self.bundles_for_layers(loaded_atlases, &[self.ground_layer_index])
    }

    /// Projects all non-empty, non-collision cells in authored layer and row-major order.
    pub(crate) fn visible_bundles(
        &self,
        loaded_atlases: &Assets<TsxAtlasAsset>,
    ) -> Result<Vec<StaticMapTileBundle>, StaticLayerBuildError> {
        self.bundles_for_layers(loaded_atlases, &self.visible_layer_indices)
    }

    fn bundles_for_layers(
        &self,
        loaded_atlases: &Assets<TsxAtlasAsset>,
        layer_indices: &[usize],
    ) -> Result<Vec<StaticMapTileBundle>, StaticLayerBuildError> {
        let mut references = Vec::with_capacity(self.atlases.len());
        for (&reference_index, handle) in
            self.atlas_reference_indices.iter().zip(self.atlases.iter())
        {
            let atlas = loaded_atlases
                .get(handle)
                .ok_or(StaticLayerBuildError::AtlasNotLoaded(reference_index))?;
            references.push((
                &self.document.external_tilesets()[reference_index],
                atlas.metadata(),
            ));
        }
        let ranges =
            TmxTilesetRanges::try_new(references).map_err(StaticLayerBuildError::GidResolution)?;
        let header = self.document.header();
        let capacity = layer_indices.iter().fold(0usize, |capacity, &index| {
            capacity.saturating_add(self.document.tile_layers()[index].gids().len())
        });
        let mut bundles = Vec::with_capacity(capacity);

        for &layer_index in layer_indices {
            let layer = &self.document.tile_layers()[layer_index];
            debug_assert_ne!(layer.name(), COLLISION_LAYER_NAME);
            for row in 0..layer.height() {
                for column in 0..layer.width() {
                    let gid = layer
                        .gid_at(column, row)
                        .expect("validated finite layer coordinates must resolve");
                    let Some(resolved) = ranges
                        .resolve(gid)
                        .map_err(StaticLayerBuildError::GidResolution)?
                    else {
                        continue;
                    };
                    if resolved.gid().flip_horizontally()
                        || resolved.gid().flip_vertically()
                        || resolved.gid().flip_diagonally()
                    {
                        return Err(StaticLayerBuildError::TransformedVisibleTile {
                            layer_id: layer.id(),
                            column,
                            row,
                        });
                    }
                    let dependency_index = self
                        .atlas_reference_indices
                        .iter()
                        .position(|&index| {
                            std::ptr::eq(
                                &self.document.external_tilesets()[index],
                                resolved.tileset(),
                            )
                        })
                        .expect("resolved range must belong to one selected visible dependency");
                    let atlas = loaded_atlases.get(&self.atlases[dependency_index]).ok_or(
                        StaticLayerBuildError::AtlasNotLoaded(
                            self.atlas_reference_indices[dependency_index],
                        ),
                    )?;
                    let sprite = atlas
                        .sprite_for_tile(resolved.local_id())
                        .map_err(StaticLayerBuildError::AtlasTile)?;
                    let center =
                        tmx_tile_center(column, row, header.tile_width(), header.tile_height());
                    let z = static_tile_z(layer.name(), layer_index, row, header.tile_height());
                    bundles.push(StaticMapTileBundle {
                        tile: StaticMapTile {
                            layer_index,
                            layer_id: layer.id(),
                            column,
                            row,
                            global_id: gid.global_id(),
                            local_id: resolved.local_id(),
                        },
                        sprite,
                        transform: Transform::from_translation(Vec3::new(center.x, center.y, z)),
                    });
                }
            }
        }
        Ok(bundles)
    }
}

/// Renderer-independent identity retained on each spawned static map entity.
#[derive(Clone, Copy, Component, Debug, Eq, PartialEq)]
pub(crate) struct StaticMapTile {
    layer_index: usize,
    layer_id: u32,
    column: u32,
    row: u32,
    global_id: u32,
    local_id: u32,
}

impl StaticMapTile {
    /// Zero-based tile-layer index in the authored TMX source.
    pub(crate) const fn layer_index(self) -> usize {
        self.layer_index
    }

    pub(crate) const fn layer_id(self) -> u32 {
        self.layer_id
    }

    pub(crate) const fn column(self) -> u32 {
        self.column
    }

    pub(crate) const fn row(self) -> u32 {
        self.row
    }

    pub(crate) const fn global_id(self) -> u32 {
        self.global_id
    }

    pub(crate) const fn local_id(self) -> u32 {
        self.local_id
    }
}

/// The complete ECS representation of one static visible map tile.
#[derive(Bundle)]
pub(crate) struct StaticMapTileBundle {
    tile: StaticMapTile,
    sprite: Sprite,
    transform: Transform,
}

/// Marks the one fixed-canvas camera owned by the active static World map.
#[derive(Component)]
struct WorldMapCamera;

/// Places a dynamic entity in the shared bottom-of-sprite Y-sort band.
pub(crate) fn world_entity_y_z(world_y: f32, sprite_half_height: f32) -> f32 {
    Y_SORT_Z_BASE + (-world_y + sprite_half_height) * Y_SORT_Z_PER_PIXEL
}

fn static_tile_z(layer_name: &str, layer_index: usize, row: u32, tile_height: u32) -> f32 {
    if layer_name == "ground" || layer_name == "terrain" {
        return GROUND_Z + layer_index as f32 * LAYER_Z_STEP;
    }
    if layer_name == "top" || layer_name.ends_with("_top") {
        return FOREGROUND_Z_BASE + layer_index as f32 * Y_SORT_SOURCE_TIE;
    }

    let screen_bottom = row.saturating_add(1) as f32 * tile_height as f32;
    Y_SORT_Z_BASE + screen_bottom * Y_SORT_Z_PER_PIXEL + layer_index as f32 * Y_SORT_SOURCE_TIE
}

fn begin_static_map_load(
    asset_server: Res<AssetServer>,
    scenario_root: Res<ScenarioRoot>,
    game: Option<Res<GameState>>,
    mut state: ResMut<StaticMapRenderState>,
) {
    *state = StaticMapRenderState::default();
    let Some(map_id) = game
        .as_deref()
        .and_then(|game| game.map().current())
        .map(|map_id| map_id.as_str())
    else {
        state.status = StaticMapRenderStatus::Failed;
        return;
    };
    let logical = format!("assets/maps/{map_id}.tmx");
    let Ok(logical) = ScenarioRelativePath::try_from(logical.as_str()) else {
        state.status = StaticMapRenderStatus::Failed;
        return;
    };
    state.handle = Some(asset_server.load(scenario_root.resolve(&logical)));
    state.status = StaticMapRenderStatus::Loading;
}

fn spawn_static_map(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    maps: Res<Assets<TmxGroundAsset>>,
    atlases: Res<Assets<TsxAtlasAsset>>,
    existing_tiles: Query<(), With<StaticMapTile>>,
    existing_cameras: Query<(), With<WorldMapCamera>>,
    mut state: ResMut<StaticMapRenderState>,
) {
    if state.status != StaticMapRenderStatus::Loading {
        return;
    }
    if !existing_tiles.is_empty() && !existing_cameras.is_empty() {
        state.status = StaticMapRenderStatus::Spawned;
        return;
    }
    if !existing_tiles.is_empty() || !existing_cameras.is_empty() {
        state.status = StaticMapRenderStatus::Failed;
        return;
    }
    let Some(handle) = state.handle.as_ref() else {
        state.status = StaticMapRenderStatus::Failed;
        return;
    };
    if matches!(asset_server.load_state(handle.id()), LoadState::Failed(_)) {
        state.status = StaticMapRenderStatus::Failed;
        return;
    }
    if !asset_server.is_loaded_with_dependencies(handle.id()) {
        return;
    }
    let Some(map) = maps.get(handle) else {
        return;
    };
    let Ok(bundles) = map.visible_bundles(&atlases) else {
        state.status = StaticMapRenderStatus::Failed;
        return;
    };
    let bounds = map.pixel_bounds();

    commands.spawn_batch(bundles);
    commands.spawn((
        fixed_gameplay_camera(),
        MapCameraFollow::new(bounds),
        WorldMapCamera,
    ));
    state.status = StaticMapRenderStatus::Spawned;
}

fn cleanup_static_map(
    mut commands: Commands,
    tiles: Query<Entity, With<StaticMapTile>>,
    cameras: Query<Entity, With<WorldMapCamera>>,
    mut state: ResMut<StaticMapRenderState>,
) {
    for entity in &tiles {
        commands.entity(entity).despawn();
    }
    for entity in &cameras {
        commands.entity(entity).despawn();
    }
    *state = StaticMapRenderState::default();
}

#[derive(TypePath)]
struct TmxGroundAssetLoader {
    scenario_root: ScenarioRoot,
}

impl FromWorld for TmxGroundAssetLoader {
    fn from_world(world: &mut World) -> Self {
        Self {
            scenario_root: world.resource::<ScenarioRoot>().clone(),
        }
    }
}

impl AssetLoader for TmxGroundAssetLoader {
    type Asset = TmxGroundAsset;
    type Settings = ();
    type Error = TmxGroundAssetLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let owner = scenario_owner(load_context.path(), &self.scenario_root)?;
        let mut bytes = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .await
            .map_err(TmxGroundAssetLoaderError::Io)?;
        let document = parse_tmx_map_document(
            str::from_utf8(&bytes).map_err(TmxGroundAssetLoaderError::Utf8)?,
            &owner,
        )
        .map_err(TmxGroundAssetLoaderError::Document)?;
        let ground_layer_index = unique_ground_layer_index(&document)?;
        let visible_layer_indices = visible_layer_indices(&document);
        let atlas_reference_indices =
            visible_atlas_reference_indices(&document, &visible_layer_indices)?;
        let atlases = atlas_reference_indices
            .iter()
            .map(|&index| {
                let source = document.external_tilesets()[index].source();
                let path =
                    AssetPath::from_path_buf(PathBuf::from(self.scenario_root.resolve(source)))
                        .with_source(load_context.path().source().clone_owned());
                load_context.load(path)
            })
            .collect();

        Ok(TmxGroundAsset {
            document,
            ground_layer_index,
            visible_layer_indices,
            atlas_reference_indices,
            atlases,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["tmx"]
    }
}

fn unique_ground_layer_index(
    document: &TmxMapDocument,
) -> Result<usize, TmxGroundAssetLoaderError> {
    let mut matches = document
        .tile_layers()
        .iter()
        .enumerate()
        .filter(|(_, layer)| layer.name() == GROUND_LAYER_NAME);
    let (index, _) = matches
        .next()
        .ok_or(TmxGroundAssetLoaderError::MissingGroundLayer)?;
    if matches.next().is_some() {
        return Err(TmxGroundAssetLoaderError::MultipleGroundLayers);
    }
    Ok(index)
}

fn visible_layer_indices(document: &TmxMapDocument) -> Vec<usize> {
    document
        .tile_layers()
        .iter()
        .enumerate()
        .filter_map(|(index, layer)| (layer.name() != COLLISION_LAYER_NAME).then_some(index))
        .collect()
}

fn visible_atlas_reference_indices(
    document: &TmxMapDocument,
    visible_layer_indices: &[usize],
) -> Result<Vec<usize>, TmxGroundAssetLoaderError> {
    let references = document.external_tilesets();
    let mut selected = BTreeMap::new();
    for &layer_index in visible_layer_indices {
        for gid in document.tile_layers()[layer_index]
            .gids()
            .iter()
            .copied()
            .filter(|gid| !gid.is_empty())
        {
            let upper =
                references.partition_point(|reference| reference.first_gid() <= gid.global_id());
            let index =
                upper
                    .checked_sub(1)
                    .ok_or(TmxGroundAssetLoaderError::UnmappedVisibleGid(
                        gid.global_id(),
                    ))?;
            selected.insert(index, ());
        }
    }
    Ok(selected.into_keys().collect())
}

fn scenario_owner(
    asset_path: &AssetPath<'_>,
    scenario_root: &ScenarioRoot,
) -> Result<ScenarioRelativePath, TmxGroundAssetLoaderError> {
    let relative = asset_path
        .path()
        .strip_prefix(scenario_root.logical_root())
        .map_err(|_| TmxGroundAssetLoaderError::OutsideScenarioRoot)?;
    let relative = relative
        .to_str()
        .ok_or(TmxGroundAssetLoaderError::NonUtf8Owner)?;
    ScenarioRelativePath::try_from(relative).map_err(TmxGroundAssetLoaderError::OwnerPath)
}

#[derive(Debug)]
enum TmxGroundAssetLoaderError {
    OutsideScenarioRoot,
    NonUtf8Owner,
    OwnerPath(ScenarioRelativePathError),
    Io(std::io::Error),
    Utf8(str::Utf8Error),
    Document(TmxMapDocumentError),
    MissingGroundLayer,
    MultipleGroundLayers,
    UnmappedVisibleGid(u32),
}

impl fmt::Display for TmxGroundAssetLoaderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutsideScenarioRoot => {
                formatter.write_str("TMX asset is outside the active scenario root")
            }
            Self::NonUtf8Owner => formatter.write_str("TMX asset path must be UTF-8"),
            Self::OwnerPath(error) => write!(formatter, "invalid TMX owner path: {error}"),
            Self::Io(error) => write!(formatter, "TMX read failed: {error}"),
            Self::Utf8(error) => write!(formatter, "TMX is not UTF-8: {error}"),
            Self::Document(error) => write!(formatter, "TMX document is invalid: {error}"),
            Self::MissingGroundLayer => formatter.write_str("TMX has no `ground` tile layer"),
            Self::MultipleGroundLayers => {
                formatter.write_str("TMX has multiple `ground` tile layers")
            }
            Self::UnmappedVisibleGid(gid) => {
                write!(
                    formatter,
                    "visible global tile ID {gid} precedes every tileset"
                )
            }
        }
    }
}

impl Error for TmxGroundAssetLoaderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::OwnerPath(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::Utf8(error) => Some(error),
            Self::Document(error) => Some(error),
            Self::OutsideScenarioRoot
            | Self::NonUtf8Owner
            | Self::MissingGroundLayer
            | Self::MultipleGroundLayers
            | Self::UnmappedVisibleGid(_) => None,
        }
    }
}

#[derive(Debug)]
pub(crate) enum StaticLayerBuildError {
    AtlasNotLoaded(usize),
    GidResolution(TmxGidResolutionError),
    AtlasTile(TsxAtlasTileError),
    TransformedVisibleTile {
        layer_id: u32,
        column: u32,
        row: u32,
    },
}

impl fmt::Display for StaticLayerBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AtlasNotLoaded(index) => {
                write!(formatter, "visible atlas dependency {index} is not loaded")
            }
            Self::GidResolution(error) => write!(formatter, "visible GID is invalid: {error}"),
            Self::AtlasTile(error) => write!(formatter, "visible atlas tile is invalid: {error}"),
            Self::TransformedVisibleTile {
                layer_id,
                column,
                row,
            } => write!(
                formatter,
                "transformed visible tile in layer {layer_id} at ({column}, {row}) is unsupported"
            ),
        }
    }
}

impl Error for StaticLayerBuildError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::GidResolution(error) => Some(error),
            Self::AtlasTile(error) => Some(error),
            Self::AtlasNotLoaded(_) | Self::TransformedVisibleTile { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use bevy::{
        asset::{AssetApp, AssetMetaCheck, AssetPlugin, AssetServer, Assets, LoadState},
        ecs::system::RunSystemOnce,
        image::{CompressedImageFormats, ImageLoader, ImagePlugin},
        prelude::{App, Image, MinimalPlugins, Sprite, Transform, UVec2},
        state::app::{AppExtStates, StatesPlugin},
    };

    use super::*;
    use crate::{
        gameplay_canvas::{GameplayCanvasCamera, camera_follow::CameraFollowTarget},
        tsx_atlas_asset::TsxAtlasAssetPlugin,
    };

    const ASSET_BASE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets");
    const TERRAIN_TSX_ASSET_PATH: &str =
        "scenarios/rusted_kingdoms/assets/tilesets/ground/terrain-v7.tsx";
    const TERRAIN_IMAGE_ASSET_PATH: &str =
        "scenarios/rusted_kingdoms/assets/tilesets/ground/terrain-v7.png";
    const GRASS_TSX_ASSET_PATH: &str =
        "scenarios/rusted_kingdoms/assets/tilesets/grass_cave_walls_24x14.tsx";
    const ICONS_TSX_ASSET_PATH: &str =
        "scenarios/rusted_kingdoms/assets/tilesets/icon_table_stage_14x9.tsx";
    const WINDOWS_TSX_ASSET_PATH: &str =
        "scenarios/rusted_kingdoms/assets/tilesets/astralpixels/finestre.tsx";

    fn ground_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .add_plugins(AssetPlugin {
                file_path: ASSET_BASE.to_owned(),
                meta_check: AssetMetaCheck::Never,
                ..Default::default()
            })
            .add_plugins(ImagePlugin::default_nearest())
            .register_asset_loader(ImageLoader::new(CompressedImageFormats::empty()))
            .insert_resource(ScenarioRoot::default())
            .insert_state(AppState::Title)
            .add_plugins(TsxAtlasAssetPlugin)
            .add_plugins(TmxGroundAssetPlugin);
        app
    }

    fn load_ardel(app: &mut App) -> Handle<TmxGroundAsset> {
        let handle = app
            .world()
            .resource::<AssetServer>()
            .load(format!("scenarios/rusted_kingdoms/{ARDEL_TMX_PATH}"));
        for _ in 0..2_000 {
            app.update();
            let server = app.world().resource::<AssetServer>();
            if server.is_loaded_with_dependencies(handle.id()) {
                return handle;
            }
            if let LoadState::Failed(error) = server.load_state(handle.id()) {
                panic!("Ardel ground load failed: {error}");
            }
            std::thread::yield_now();
        }
        let server = app.world().resource::<AssetServer>();
        panic!(
            "Ardel ground dependencies did not load: root={:?}, recursive={:?}",
            server.load_state(handle.id()),
            server.recursive_dependency_load_state(handle.id())
        );
    }

    #[test]
    fn real_ardel_ground_loads_visible_atlases_and_spawns_600_exact_tile_sprites() {
        let mut app = ground_app();
        let map_handle = load_ardel(&mut app);

        let bundles = {
            let maps = app.world().resource::<Assets<TmxGroundAsset>>();
            let map = maps.get(&map_handle).unwrap();
            let header = map.document().header();
            assert_eq!((header.width(), header.height()), (30, 20));
            assert_eq!((header.tile_width(), header.tile_height()), (32, 32));
            assert_eq!(map.ground_layer().id(), 1);
            assert_eq!(map.ground_layer().name(), GROUND_LAYER_NAME);
            assert_eq!(map.ground_layer().gids().len(), 600);
            assert!(
                map.ground_layer()
                    .gids()
                    .iter()
                    .all(|gid| gid.global_id() == 1322)
            );
            let atlas_paths = map
                .atlas_handles()
                .iter()
                .map(|handle| handle.path().unwrap().path().to_string_lossy().into_owned())
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(
                atlas_paths,
                [
                    GRASS_TSX_ASSET_PATH.to_owned(),
                    ICONS_TSX_ASSET_PATH.to_owned(),
                    WINDOWS_TSX_ASSET_PATH.to_owned(),
                    TERRAIN_TSX_ASSET_PATH.to_owned(),
                ]
                .into_iter()
                .collect()
            );
            let atlases = app.world().resource::<Assets<TsxAtlasAsset>>();
            map.ground_bundles(atlases).unwrap()
        };
        assert_eq!(bundles.len(), 600);
        app.world_mut().spawn_batch(bundles);

        let world = app.world_mut();
        let mut query = world.query::<(&StaticMapTile, &Sprite, &Transform)>();
        assert_eq!(query.iter(world).count(), 600);
        for (tile, sprite, transform) in query.iter(world) {
            assert_eq!(tile.global_id(), 1322);
            assert_eq!(tile.local_id(), 386);
            assert_eq!(tile.layer_index(), 0);
            assert_eq!(tile.layer_id(), 1);
            let expected = tmx_tile_center(tile.column(), tile.row(), 32, 32);
            assert_eq!(
                transform.translation,
                Vec3::new(expected.x, expected.y, 0.0)
            );
            let atlas = sprite.texture_atlas.as_ref().unwrap();
            assert_eq!(atlas.index, 386);
            assert_eq!(
                sprite.image.path().unwrap().path().to_string_lossy(),
                TERRAIN_IMAGE_ASSET_PATH
            );
        }

        let images = world.resource::<Assets<Image>>();
        let image_handle = query.iter(world).next().unwrap().1.image.clone();
        assert_eq!(
            images.get(&image_handle).unwrap().size(),
            UVec2::new(1024, 2048)
        );
        let bottom_right = query
            .iter(world)
            .find(|(tile, _, _)| tile.column() == 29 && tile.row() == 19)
            .unwrap();
        assert_eq!(
            bottom_right.2.translation,
            Vec3::new(944.0, -624.0, GROUND_Z)
        );
    }

    #[test]
    fn real_ardel_visible_projection_preserves_source_order_and_never_draws_collision() {
        let mut app = ground_app();
        let map_handle = load_ardel(&mut app);

        let expected_count = {
            let maps = app.world().resource::<Assets<TmxGroundAsset>>();
            let map = maps.get(&map_handle).unwrap();
            assert_eq!(
                map.visible_layer_indices
                    .iter()
                    .map(|&index| (index, map.document().tile_layers()[index].name()))
                    .collect::<Vec<_>>(),
                [(0, "ground"), (1, "terrain"), (3, "decoration")]
            );
            let collision = map
                .document()
                .tile_layers()
                .iter()
                .find(|layer| layer.name() == COLLISION_LAYER_NAME)
                .unwrap();
            assert_eq!(collision.id(), 4);
            assert!(collision.gids().iter().any(|gid| !gid.is_empty()));
            assert!(
                collision.gids().iter().any(|gid| gid.flip_horizontally()),
                "the excluded layer deliberately includes a transformed tile"
            );

            let expected_count = map
                .visible_layer_indices
                .iter()
                .flat_map(|&index| map.document().tile_layers()[index].gids())
                .filter(|gid| !gid.is_empty())
                .count();
            let atlases = app.world().resource::<Assets<TsxAtlasAsset>>();
            let bundles = map.visible_bundles(atlases).unwrap();
            assert_eq!(bundles.len(), expected_count);
            assert!(bundles.iter().all(|bundle| bundle.tile.layer_id() != 4));
            assert!(
                bundles
                    .windows(2)
                    .all(|pair| { pair[0].tile.layer_index() <= pair[1].tile.layer_index() })
            );
            expected_count
        };
        *app.world_mut().resource_mut::<StaticMapRenderState>() = StaticMapRenderState {
            handle: Some(map_handle),
            status: StaticMapRenderStatus::Loading,
        };
        app.world_mut().run_system_once(spawn_static_map).unwrap();

        let world = app.world_mut();
        assert_eq!(
            world.resource::<StaticMapRenderState>().status,
            StaticMapRenderStatus::Spawned
        );
        let mut query = world.query::<(&StaticMapTile, &Sprite, &Transform)>();
        let rendered = query.iter(world).collect::<Vec<_>>();
        assert_eq!(rendered.len(), expected_count);
        assert!(rendered.iter().all(|(tile, _, _)| tile.layer_id() != 4));
        assert_eq!(
            rendered
                .iter()
                .map(|(tile, _, _)| tile.layer_index())
                .collect::<std::collections::BTreeSet<_>>(),
            [0, 1, 3].into_iter().collect()
        );
        for (tile, sprite, transform) in rendered {
            let expected = tmx_tile_center(tile.column(), tile.row(), 32, 32);
            assert_eq!(transform.translation.x, expected.x);
            assert_eq!(transform.translation.y, expected.y);
            let layer_name = match tile.layer_index() {
                0 => "ground",
                1 => "terrain",
                3 => "decoration",
                index => panic!("unexpected rendered layer index {index}"),
            };
            assert_eq!(
                transform.translation.z,
                static_tile_z(layer_name, tile.layer_index(), tile.row(), 32)
            );
            assert_eq!(
                sprite.texture_atlas.as_ref().unwrap().index,
                tile.local_id() as usize
            );
        }

        let (bounds, camera_z) = {
            let (follow, camera_before) = world
                .query_filtered::<(&MapCameraFollow, &Transform), With<WorldMapCamera>>()
                .single(world)
                .expect("one World map camera");
            (follow.bounds(), camera_before.translation.z)
        };
        assert_eq!(bounds.min(), Vec2::new(0.0, -640.0));
        assert_eq!(bounds.max(), Vec2::new(960.0, 0.0));
        world.spawn((
            Transform::from_xyz(464.0, -176.0, 10.208),
            CameraFollowTarget,
        ));
        world.run_system_once(follow_map_camera).unwrap();
        let camera_after = world
            .query_filtered::<&Transform, (With<WorldMapCamera>, With<GameplayCanvasCamera>)>()
            .single(world)
            .expect("one fixed-canvas World map camera");
        assert_eq!(camera_after.translation, Vec3::new(480.0, -320.0, camera_z));

        world.run_system_once(cleanup_static_map).unwrap();
        assert_eq!(
            world.resource::<StaticMapRenderState>().status,
            StaticMapRenderStatus::Idle
        );
        assert_eq!(
            world
                .query_filtered::<(), With<StaticMapTile>>()
                .iter(world)
                .count(),
            0
        );
        assert_eq!(
            world
                .query_filtered::<(), With<WorldMapCamera>>()
                .iter(world)
                .count(),
            0
        );
    }

    #[test]
    fn y_sort_band_orders_player_around_decoration_and_keeps_static_source_ties() {
        let player_row_five = world_entity_y_z(-176.0, 32.0);
        let decoration_above = static_tile_z("decoration", 3, 4, 32);
        let decoration_below = static_tile_z("decoration", 3, 6, 32);

        assert!(static_tile_z("ground", 0, 19, 32) < player_row_five);
        assert!(static_tile_z("terrain", 1, 19, 32) < player_row_five);
        assert!(decoration_above < player_row_five);
        assert!(player_row_five < decoration_below);
        assert!(static_tile_z("decoration", 3, 5, 32) < static_tile_z("over_ground", 4, 5, 32));
        assert!(player_row_five < static_tile_z("top", 5, 0, 32));
    }

    #[test]
    #[ignore = "requires RPG_S1_PINNED_SCENARIO_DIR pointing at the pinned source scenario"]
    fn copied_ardel_ground_source_graph_is_byte_identical_to_pinned_scenario() {
        let source_root = std::env::var_os("RPG_S1_PINNED_SCENARIO_DIR")
            .expect("RPG_S1_PINNED_SCENARIO_DIR must name rusted_kingdoms");
        let destination_root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/scenarios/rusted_kingdoms");
        for relative in [
            "assets/maps/town_01_ardel.tmx",
            "assets/tilesets/grass_cave_walls_24x14.tsx",
            "assets/tilesets/grass_cave_walls_24x14.png",
            "assets/tilesets/icon_table_stage_14x9.tsx",
            "assets/tilesets/icon_table_stage_14x9.png",
            "assets/tilesets/astralpixels/finestre.tsx",
            "assets/tilesets/astralpixels/finestre.png",
            "assets/tilesets/astralpixels/credit.txt",
            "assets/tilesets/ground/terrain-v7.tsx",
            "assets/tilesets/ground/terrain-v7.png",
            "assets/tilesets/ground/CREDITS-terrain.txt",
        ] {
            assert_eq!(
                fs::read(Path::new(&source_root).join(relative)).unwrap(),
                fs::read(destination_root.join(relative)).unwrap(),
                "copied source differs: {relative}"
            );
        }
    }
}
