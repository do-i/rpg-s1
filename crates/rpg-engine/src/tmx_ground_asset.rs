//! Bevy-native loading and ECS projection for static visible TMX tile layers.

use std::{collections::BTreeMap, error::Error, fmt, path::PathBuf, str};

use bevy::{
    asset::{
        Asset, AssetApp, AssetLoader, AssetPath, AssetServer, Assets, Handle, LoadContext,
        io::Reader,
    },
    ecs::schedule::ApplyDeferred,
    ecs::{bundle::Bundle, component::Component},
    prelude::{
        App, Commands, Entity, FromWorld, IntoScheduleConfigs, OnEnter, OnExit, Plugin, PostUpdate,
        Quat, Query, Res, ResMut, Resource, Sprite, Transform, Update, Vec2, Vec3, With, World,
        in_state,
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
    scenario_inventory::ScenarioInventory,
    scenario_path::{ScenarioRelativePath, ScenarioRelativePathError},
    scenario_root::ScenarioRoot,
    tile_coordinates::tmx_tile_center,
    tmx_header::{
        TmxGidResolutionError, TmxMapDocument, TmxMapDocumentError, TmxTilesetRanges,
        parse_tmx_map_document,
    },
    tsx_atlas_asset::{TsxAtlasAsset, TsxAtlasTileError},
};

#[cfg(test)]
pub(crate) const ARDEL_TMX_PATH: &str = "media/maps/town_01_ardel.tmx";
const GROUND_LAYER_NAME: &str = "ground";
#[cfg(test)]
const COLLISION_LAYER_NAME: &str = "collision";
const FOREGROUND_LAYER_NAME: &str = "foreground";
const GROUND_Z: f32 = 0.0;
const STATIC_Z_SPAN: f32 = 1.0;
const Y_SORT_Z_BASE: f32 = 10.0;
const Y_SORT_Z_PER_PIXEL: f32 = 0.001;

/// Registers the project-owned `.tmx` static-map asset loader.
pub(crate) struct TmxGroundAssetPlugin;

impl Plugin for TmxGroundAssetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScenarioRoot>()
            .init_asset::<TmxGroundAsset>()
            .init_asset_loader::<TmxGroundAssetLoader>()
            .init_resource::<StaticMapRenderState>()
            .add_systems(OnEnter(AppState::World), begin_static_map_load)
            .add_systems(
                Update,
                (sync_static_map_request, ApplyDeferred, spawn_static_map)
                    .chain()
                    .in_set(crate::world_publication::WorldPublicationSet::Load)
                    .run_if(in_state(AppState::World)),
            )
            .add_systems(
                PostUpdate,
                follow_map_camera.run_if(in_state(AppState::World)),
            )
            .add_systems(OnExit(AppState::World), cleanup_static_map);
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum StaticMapRenderStatus {
    #[default]
    Idle,
    Loading,
    Spawned,
    Failed,
}

/// Strong handle and publication state for the active World's static map.
#[derive(Clone, Debug, Default, Resource)]
pub(crate) struct StaticMapRenderState {
    map_id: Option<String>,
    handle: Option<Handle<TmxGroundAsset>>,
    status: StaticMapRenderStatus,
    failure: Option<String>,
}

impl StaticMapRenderState {
    fn fail(&mut self, reason: impl Into<String>) {
        self.status = StaticMapRenderStatus::Failed;
        self.failure = Some(reason.into());
    }

    pub(crate) fn publication_status(
        &self,
        map_id: &str,
    ) -> crate::world_publication::PublicationStatus {
        use crate::world_publication::PublicationStatus;
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
        self.handle.iter().map(|h| h.id().untyped()).collect()
    }

    pub(crate) fn map<'a>(&self, maps: &'a Assets<TmxGroundAsset>) -> Option<&'a TmxGroundAsset> {
        maps.get(self.handle.as_ref()?)
    }

    pub(crate) fn is_spawned_for(&self, map_id: &str) -> bool {
        self.map_id.as_deref() == Some(map_id) && self.status == StaticMapRenderStatus::Spawned
    }
}

/// A parsed TMX document with the atlases used by visible tile layers requested from Bevy.
#[derive(Asset, Clone, Debug, TypePath)]
pub(crate) struct TmxGroundAsset {
    document: TmxMapDocument,
    visible_layer_indices: Vec<usize>,
    atlas_reference_indices: Vec<usize>,
    #[dependency]
    atlases: Vec<Handle<TsxAtlasAsset>>,
}

impl TmxGroundAsset {
    pub(crate) const fn document(&self) -> &TmxMapDocument {
        &self.document
    }

    #[cfg(test)]
    fn ground_layer_index(&self) -> usize {
        unique_ground_layer_index(&self.document).expect("the loader validated one ground layer")
    }

    #[cfg(test)]
    pub(crate) fn ground_layer(&self) -> &crate::tmx_header::TmxTileLayer {
        &self.document.tile_layers()[self.ground_layer_index()]
    }

    #[cfg(test)]
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
    #[cfg(test)]
    pub(crate) fn ground_bundles(
        &self,
        loaded_atlases: &Assets<TsxAtlasAsset>,
    ) -> Result<Vec<StaticMapTileBundle>, StaticLayerBuildError> {
        self.bundles_for_layers(loaded_atlases, &[self.ground_layer_index()])
    }

    /// Projects all non-empty tile cells in authored layer and row-major order.
    ///
    /// `collision` is a gameplay role, not a visibility flag. Rusted Kingdoms deliberately
    /// paints Ardel's buildings and fences in that visible layer while also using its non-empty
    /// cells for collision, matching PyTMX's rendering of every visible tile layer.
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
                    let mut sprite = atlas
                        .sprite_for_tile(resolved.local_id())
                        .map_err(StaticLayerBuildError::AtlasTile)?;
                    let rotation = apply_tiled_transform(&mut sprite, resolved.gid());
                    let center =
                        tmx_tile_center(column, row, header.tile_width(), header.tile_height());
                    let z = if is_foreground_layer(layer.name())
                        && let Some(base_y) = layer
                            .sort_base_y_at(column, row)
                            .or_else(|| atlas.metadata().sort_base_y(resolved.local_id()))
                    {
                        // Keep every row of an object at one baseline. This draws the full
                        // transparent silhouette above actors behind it and below actors in front.
                        world_entity_y_z(
                            center.y,
                            base_y as f32 - header.tile_height() as f32 / 2.0,
                        )
                    } else {
                        tile_layer_z(
                            layer_index,
                            layer.name(),
                            header.height() as f32 * header.tile_height() as f32,
                        )
                    };
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
                        transform: Transform {
                            translation: Vec3::new(center.x, center.y, z),
                            rotation,
                            ..Default::default()
                        },
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
    #[cfg(test)]
    pub(crate) const fn layer_index(self) -> usize {
        self.layer_index
    }

    #[cfg(test)]
    pub(crate) const fn layer_id(self) -> u32 {
        self.layer_id
    }

    #[cfg(test)]
    pub(crate) const fn column(self) -> u32 {
        self.column
    }

    #[cfg(test)]
    pub(crate) const fn row(self) -> u32 {
        self.row
    }

    #[cfg(test)]
    pub(crate) const fn global_id(self) -> u32 {
        self.global_id
    }

    #[cfg(test)]
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
pub(crate) struct WorldMapCamera;

/// Places a dynamic entity in the shared bottom-of-sprite Y-sort band.
pub(crate) fn world_entity_y_z(world_y: f32, sprite_half_height: f32) -> f32 {
    Y_SORT_Z_BASE + (-world_y + sprite_half_height) * Y_SORT_Z_PER_PIXEL
}

/// Preserve source order in a bounded band below actors, even with many authored layers.
fn static_tile_z(layer_index: usize) -> f32 {
    let index = layer_index as f32;
    GROUND_Z + STATIC_Z_SPAN * (index / (index + 1.0))
}

/// Foreground contains overhead art, while bases stay in ordinary layers below actors.
/// Its depth follows the map height so actors at the bottom of tall maps stay behind it too.
fn tile_layer_z(layer_index: usize, layer_name: &str, map_pixel_height: f32) -> f32 {
    let authored_z = static_tile_z(layer_index);
    if is_foreground_layer(layer_name) {
        world_entity_y_z(-map_pixel_height, 64.0) + STATIC_Z_SPAN + authored_z
    } else {
        authored_z
    }
}

fn is_foreground_layer(name: &str) -> bool {
    name == FOREGROUND_LAYER_NAME
}

fn apply_tiled_transform(sprite: &mut Sprite, gid: crate::tmx_header::TmxTileGid) -> Quat {
    let horizontal = gid.flip_horizontally();
    let vertical = gid.flip_vertically();
    if !gid.flip_diagonally() {
        sprite.flip_x = horizontal;
        sprite.flip_y = vertical;
        return Quat::IDENTITY;
    }

    // Tiled applies the anti-diagonal reflection before its horizontal and vertical flags.
    // Square orthogonal tiles can represent all eight combinations with one quarter turn plus
    // Bevy's two sprite reflections.
    match (horizontal, vertical) {
        (false, false) => {
            sprite.flip_y = true;
            Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)
        }
        (true, false) => Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
        (false, true) => Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2),
        (true, true) => {
            sprite.flip_x = true;
            Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)
        }
    }
}

fn begin_static_map_load(
    asset_server: Res<AssetServer>,
    scenario_root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
    game: Option<Res<GameState>>,
    mut state: ResMut<StaticMapRenderState>,
) {
    *state = StaticMapRenderState::default();
    let Some(map_id) = game
        .as_deref()
        .and_then(|game| game.map().current())
        .map(|map_id| map_id.as_str())
    else {
        state.fail("current map is missing");
        return;
    };
    let Some(logical) = inventory.tmx_path(map_id) else {
        state.fail("map path is invalid");
        return;
    };
    state.map_id = Some(map_id.to_owned());
    state.handle = Some(asset_server.load(scenario_root.resolve(&logical)));
    state.status = StaticMapRenderStatus::Loading;
}

#[expect(
    clippy::too_many_arguments,
    reason = "atomic Bevy map replacement keeps each asset and entity boundary explicit"
)]
fn sync_static_map_request(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    scenario_root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
    game: Option<Res<GameState>>,
    tiles: Query<Entity, With<StaticMapTile>>,
    cameras: Query<Entity, With<WorldMapCamera>>,
    mut state: ResMut<StaticMapRenderState>,
) {
    let current = game
        .as_deref()
        .and_then(|game| game.map().current())
        .map(|map| map.as_str());
    if current == state.map_id.as_deref() {
        return;
    }

    for entity in &tiles {
        commands.entity(entity).despawn();
    }
    for entity in &cameras {
        commands.entity(entity).despawn();
    }
    *state = StaticMapRenderState::default();
    let Some(map_id) = current else {
        state.fail("current map is missing");
        return;
    };
    let Some(logical) = inventory.tmx_path(map_id) else {
        state.fail("map path is invalid");
        return;
    };
    state.map_id = Some(map_id.to_owned());
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
        state.fail("map entities are incomplete");
        return;
    }
    let Some(handle) = state.handle.as_ref() else {
        state.fail("map handle is missing");
        return;
    };
    if crate::world_publication::asset_failed(&asset_server, handle.id()) {
        state.fail("map or image dependency failed to load");
        return;
    }
    if !asset_server.is_loaded_with_dependencies(handle.id()) {
        return;
    }
    let Some(map) = maps.get(handle) else {
        return;
    };
    let Ok(bundles) = map.visible_bundles(&atlases) else {
        state.fail("map tiles cannot render");
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
        unique_ground_layer_index(&document)?;
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
            visible_layer_indices,
            atlas_reference_indices,
            atlases,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["tmx"]
    }
}

/// Locates the map's one `ground` tile layer.
///
/// `pub(crate)` so [`crate::scenario_map_sweep`] can headlessly replay this exact
/// production selection rule without duplicating it.
pub(crate) fn unique_ground_layer_index(
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

/// `pub(crate)` so [`crate::scenario_map_sweep`] can select the same visible layers the
/// renderer would, without duplicating this rule.
pub(crate) fn visible_layer_indices(document: &TmxMapDocument) -> Vec<usize> {
    document
        .tile_layers()
        .iter()
        .enumerate()
        .filter_map(|(index, layer)| layer.visible().then_some(index))
        .collect()
}

/// `pub(crate)` so [`crate::scenario_map_sweep`] can determine exactly which external
/// tilesets the renderer would load for a map's visible layers, without duplicating this rule.
pub(crate) fn visible_atlas_reference_indices(
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

/// `pub(crate)` so [`crate::scenario_map_sweep`] can report the same ground-layer and
/// visible-atlas failures the production loader would, using its own [`fmt::Display`].
#[derive(Debug)]
pub(crate) enum TmxGroundAssetLoaderError {
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
}

impl fmt::Display for StaticLayerBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AtlasNotLoaded(index) => {
                write!(formatter, "visible atlas dependency {index} is not loaded")
            }
            Self::GidResolution(error) => write!(formatter, "visible GID is invalid: {error}"),
            Self::AtlasTile(error) => write!(formatter, "visible atlas tile is invalid: {error}"),
        }
    }
}

impl Error for StaticLayerBuildError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::GidResolution(error) => Some(error),
            Self::AtlasTile(error) => Some(error),
            Self::AtlasNotLoaded(_) => None,
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

    use crate::test_support::scenario_package_dir;
    use crate::{
        gameplay_canvas::{GameplayCanvasCamera, camera_follow::CameraFollowTarget},
        tsx_atlas_asset::TsxAtlasAssetPlugin,
    };

    const ASSET_BASE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets");
    const TERRAIN_TSX_ASSET_PATH: &str = scenario_asset!("media/tilesets/terrain-v7.tsx");
    const FLOOR_TSX_ASSET_PATH: &str = scenario_asset!("media/tilesets/floor.tsx");
    const FLOOR_IMAGE_ASSET_PATH: &str = scenario_asset!("media/tilesets/floor.png");
    const WALL_TSX_ASSET_PATH: &str = scenario_asset!("media/tilesets/wall.tsx");
    const ROOF_TSX_ASSET_PATH: &str = scenario_asset!("media/tilesets/roof.tsx");
    const DOOR_TSX_ASSET_PATH: &str = scenario_asset!("media/tilesets/door.tsx");
    const WINDOWS_TSX_ASSET_PATH: &str = scenario_asset!("media/tilesets/windows.tsx");
    const OUTDOOR_TSX_ASSET_PATH: &str = scenario_asset!("media/tilesets/outdoor.tsx");
    const SHOP_TSX_ASSET_PATH: &str = scenario_asset!("media/tilesets/shop.tsx");
    const INTERIOR_TSX_ASSET_PATH: &str = scenario_asset!("media/tilesets/interior.tsx");

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

    fn load_map(app: &mut App, relative: &str) -> Handle<TmxGroundAsset> {
        let handle = app
            .world()
            .resource::<AssetServer>()
            .load(format!("{}{relative}", scenario_asset!("")));
        for _ in 0..2_000 {
            app.update();
            let server = app.world().resource::<AssetServer>();
            if server.is_loaded_with_dependencies(handle.id()) {
                return handle;
            }
            if let LoadState::Failed(error) = server.load_state(handle.id()) {
                panic!("map load failed for {relative}: {error}");
            }
            std::thread::yield_now();
        }
        let server = app.world().resource::<AssetServer>();
        panic!(
            "map dependencies did not load for {relative}: root={:?}, recursive={:?}",
            server.load_state(handle.id()),
            server.recursive_dependency_load_state(handle.id())
        );
    }

    fn load_ardel(app: &mut App) -> Handle<TmxGroundAsset> {
        load_map(app, ARDEL_TMX_PATH)
    }

    #[test]
    fn starting_forest_keeps_spawn_markers_semantic_and_out_of_visible_projection() {
        let mut app = ground_app();
        let handle = load_map(&mut app, "media/maps/zone_01_starting_forest.tmx");
        let maps = app.world().resource::<Assets<TmxGroundAsset>>();
        let map = maps.get(&handle).unwrap();
        let spawn = map
            .document()
            .tile_layers()
            .iter()
            .find(|layer| layer.name() == "spawn_tile")
            .unwrap();

        assert!(!spawn.visible());
        assert_eq!(
            spawn.gids().iter().filter(|gid| !gid.is_empty()).count(),
            10
        );
        let visible_names = map
            .visible_layer_indices
            .iter()
            .map(|&index| map.document().tile_layers()[index].name())
            .collect::<Vec<_>>();
        assert!(
            visible_names
                .iter()
                .all(|name| !matches!(*name, "collision" | "spawn_tile"))
        );
        assert!(visible_names.contains(&"ground"));
        assert!(visible_names.contains(&"foreground"));
        let atlases = app.world().resource::<Assets<TsxAtlasAsset>>();
        assert!(
            map.visible_bundles(atlases)
                .unwrap()
                .iter()
                .all(|bundle| bundle.tile.layer_id() != 6)
        );
    }

    #[test]
    fn ardel_house_loads_all_visible_dependencies_and_applies_tiled_transforms() {
        let mut app = ground_app();
        let handle = load_map(&mut app, "media/maps/town_01_ardel_house_01.tmx");
        let maps = app.world().resource::<Assets<TmxGroundAsset>>();
        let map = maps.get(&handle).unwrap();
        assert_eq!(
            (
                map.document().header().width(),
                map.document().header().height()
            ),
            (25, 13)
        );
        assert_eq!(
            map.atlas_handles()
                .iter()
                .map(|handle| handle.path().unwrap().path().to_string_lossy().into_owned())
                .collect::<std::collections::BTreeSet<_>>(),
            [
                INTERIOR_TSX_ASSET_PATH.to_owned(),
                FLOOR_TSX_ASSET_PATH.to_owned(),
                WALL_TSX_ASSET_PATH.to_owned(),
                WINDOWS_TSX_ASSET_PATH.to_owned(),
            ]
            .into_iter()
            .collect()
        );
        let atlases = app.world().resource::<Assets<TsxAtlasAsset>>();
        let bundles = map.visible_bundles(atlases).unwrap();
        let transformed = bundles
            .iter()
            .find(|bundle| {
                bundle.tile.layer_id() == 3 && bundle.tile.column() == 18 && bundle.tile.row() == 3
            })
            .expect("source H+D transformed floor tile should render");
        assert_eq!(transformed.tile.local_id(), 1);
        assert_eq!(
            transformed
                .sprite
                .image
                .path()
                .unwrap()
                .path()
                .to_string_lossy(),
            FLOOR_IMAGE_ASSET_PATH
        );
        assert_eq!(
            transformed.transform.rotation,
            Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)
        );
        assert!(!transformed.sprite.flip_x);
        assert!(!transformed.sprite.flip_y);
    }

    #[test]
    fn real_ardel_ground_loads_visible_atlases_and_spawns_exact_tile_sprites() {
        let mut app = ground_app();
        let map_handle = load_ardel(&mut app);

        let (bundles, floor_first_gid) = {
            let maps = app.world().resource::<Assets<TmxGroundAsset>>();
            let map = maps.get(&map_handle).unwrap();
            let floor_first_gid = map
                .document()
                .external_tilesets()
                .iter()
                .find(|reference| reference.source().as_str() == "media/tilesets/floor.tsx")
                .unwrap()
                .first_gid();
            let header = map.document().header();
            assert_eq!((header.width(), header.height()), (55, 20));
            assert_eq!((header.tile_width(), header.tile_height()), (32, 32));
            assert_eq!(map.ground_layer().id(), 1);
            assert_eq!(map.ground_layer().name(), GROUND_LAYER_NAME);
            assert_eq!(map.ground_layer().gids().len(), 1100);
            assert!(map.ground_layer().gids().iter().all(|gid| {
                [floor_first_gid + 74, floor_first_gid + 75].contains(&gid.global_id())
            }));
            let atlas_paths = map
                .atlas_handles()
                .iter()
                .map(|handle| handle.path().unwrap().path().to_string_lossy().into_owned())
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(
                atlas_paths,
                [
                    OUTDOOR_TSX_ASSET_PATH.to_owned(),
                    SHOP_TSX_ASSET_PATH.to_owned(),
                    FLOOR_TSX_ASSET_PATH.to_owned(),
                    WALL_TSX_ASSET_PATH.to_owned(),
                    WINDOWS_TSX_ASSET_PATH.to_owned(),
                    TERRAIN_TSX_ASSET_PATH.to_owned(),
                    ROOF_TSX_ASSET_PATH.to_owned(),
                    DOOR_TSX_ASSET_PATH.to_owned(),
                ]
                .into_iter()
                .collect()
            );
            let atlases = app.world().resource::<Assets<TsxAtlasAsset>>();
            (map.ground_bundles(atlases).unwrap(), floor_first_gid)
        };
        assert_eq!(bundles.len(), 1100);
        app.world_mut().spawn_batch(bundles);

        let world = app.world_mut();
        let mut query = world.query::<(&StaticMapTile, &Sprite, &Transform)>();
        assert_eq!(query.iter(world).count(), 1100);
        for (tile, sprite, transform) in query.iter(world) {
            assert!(matches!(tile.local_id(), 74 | 75));
            assert_eq!(tile.global_id(), floor_first_gid + tile.local_id());
            assert_eq!(tile.layer_index(), 1);
            assert_eq!(tile.layer_id(), 1);
            let expected = tmx_tile_center(tile.column(), tile.row(), 32, 32);
            assert_eq!(
                transform.translation,
                Vec3::new(expected.x, expected.y, static_tile_z(1))
            );
            let atlas = sprite.texture_atlas.as_ref().unwrap();
            assert_eq!(atlas.index, tile.local_id() as usize);
            assert_eq!(
                sprite.image.path().unwrap().path().to_string_lossy(),
                FLOOR_IMAGE_ASSET_PATH
            );
        }

        let images = world.resource::<Assets<Image>>();
        let image_handle = query.iter(world).next().unwrap().1.image.clone();
        assert_eq!(
            images.get(&image_handle).unwrap().size(),
            UVec2::new(512, 384)
        );
        let bottom_right = query
            .iter(world)
            .find(|(tile, _, _)| tile.column() == 29 && tile.row() == 19)
            .unwrap();
        assert_eq!(
            bottom_right.2.translation,
            Vec3::new(944.0, -624.0, static_tile_z(1))
        );
    }

    #[test]
    fn real_ardel_visible_projection_preserves_every_authored_tile_layer_in_source_order() {
        let mut app = ground_app();
        let map_handle = load_ardel(&mut app);

        let (expected_count, outdoor_first_gid) = {
            let maps = app.world().resource::<Assets<TmxGroundAsset>>();
            let map = maps.get(&map_handle).unwrap();
            assert_eq!(
                map.visible_layer_indices
                    .iter()
                    .map(|&index| (index, map.document().tile_layers()[index].name()))
                    .collect::<Vec<_>>(),
                [
                    (1, "ground"),
                    (2, "terrain"),
                    (3, "architecture"),
                    (4, "decoration"),
                    (5, "foreground")
                ]
            );
            let collision = map
                .document()
                .tile_layers()
                .iter()
                .find(|layer| layer.name() == COLLISION_LAYER_NAME)
                .unwrap();
            assert_eq!(collision.id(), 4);
            assert!(collision.gids().iter().any(|gid| !gid.is_empty()));
            assert!(!collision.visible());
            assert!(
                map.document().tile_layers()[5]
                    .gids()
                    .iter()
                    .any(|gid| gid.flip_horizontally())
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
            let outdoor_first_gid = map
                .document()
                .external_tilesets()
                .iter()
                .find(|reference| reference.source().as_str() == "media/tilesets/outdoor.tsx")
                .unwrap()
                .first_gid();
            (expected_count, outdoor_first_gid)
        };
        *app.world_mut().resource_mut::<StaticMapRenderState>() = StaticMapRenderState {
            map_id: Some("town_01_ardel".to_owned()),
            handle: Some(map_handle),
            status: StaticMapRenderStatus::Loading,
            failure: None,
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
            [1, 2, 3, 4, 5].into_iter().collect()
        );
        for (tile, sprite, transform) in rendered {
            let expected = tmx_tile_center(tile.column(), tile.row(), 32, 32);
            assert_eq!(transform.translation.x, expected.x);
            assert_eq!(transform.translation.y, expected.y);
            let names = [
                "collision",
                "ground",
                "terrain",
                "architecture",
                "decoration",
                "foreground",
            ];
            assert_eq!(
                transform.translation.z,
                match (
                    names[tile.layer_index()],
                    tile.global_id().checked_sub(outdoor_first_gid),
                ) {
                    ("foreground", Some(0..=3)) => world_entity_y_z(expected.y, 48.0),
                    ("foreground", Some(16..=19)) => world_entity_y_z(expected.y, 16.0),
                    _ => tile_layer_z(tile.layer_index(), names[tile.layer_index()], 640.0),
                }
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
        assert_eq!(bounds.max(), Vec2::new(1760.0, 0.0));
        world.spawn((
            Transform::from_xyz(464.0, -176.0, 10.208),
            CameraFollowTarget,
        ));
        world.run_system_once(follow_map_camera).unwrap();
        let camera_after = world
            .query_filtered::<&Transform, (With<WorldMapCamera>, With<GameplayCanvasCamera>)>()
            .single(world)
            .expect("one fixed-canvas World map camera");
        assert_eq!(camera_after.translation, Vec3::new(640.0, -320.0, camera_z));

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
    fn authored_tile_layers_stay_below_y_sorted_world_entities() {
        let player_row_five = world_entity_y_z(-176.0, 32.0);

        assert_eq!(static_tile_z(0), 0.0);
        for index in 1..100 {
            assert!(static_tile_z(index - 1) < static_tile_z(index));
            assert!(static_tile_z(index) < player_row_five);
        }
    }

    #[test]
    fn foreground_stays_above_actors_throughout_short_and_tall_maps() {
        for height in [576.0, 100_000.0] {
            let actor_at_top = world_entity_y_z(0.0, 32.0);
            let actor_at_bottom = world_entity_y_z(-height, 32.0);
            for index in [0, 1, 20, 99] {
                let foreground = tile_layer_z(index, "foreground", height);
                assert!(foreground > actor_at_top);
                assert!(foreground > actor_at_bottom);
                assert!(tile_layer_z(index, "decoration", height) < actor_at_top);
                assert!(tile_layer_z(index, "architecture", height) < actor_at_top);
            }
        }
    }

    #[test]
    fn migrated_roofs_block_movement_and_complete_trees_sort_at_their_bases() {
        use crate::scenario_spatial::{Position, collision_occupancy::CollisionOccupancy};
        use crate::world_player::WorldPlayerMotion;

        let mut app = ground_app();
        for path in [
            "media/maps/town_01_ardel.tmx",
            "media/maps/town_02_millhaven.tmx",
        ] {
            let handle = load_map(&mut app, path);
            let maps = app.world().resource::<Assets<TmxGroundAsset>>();
            let map = maps.get(&handle).unwrap();
            let document = map.document();
            let occupancy = CollisionOccupancy::from_tmx_document(document).unwrap();
            let first_gid = |suffix: &str| {
                document
                    .external_tilesets()
                    .iter()
                    .find(|reference| reference.source().as_str().ends_with(suffix))
                    .unwrap()
                    .first_gid()
            };
            let roof = first_gid("/roof.tsx");
            let outdoor = first_gid("/outdoor.tsx");
            let foreground = document
                .tile_layers()
                .iter()
                .find(|layer| layer.name() == "foreground")
                .unwrap();
            let mut roof_count = 0;
            let mut tree_position = None;
            for row in 0..foreground.height() {
                for column in 0..foreground.width() {
                    let gid = foreground.gid_at(column, row).unwrap().global_id();
                    if gid.checked_sub(roof).is_some_and(|id| id < 84) {
                        roof_count += 1;
                        assert_eq!(
                            occupancy.is_blocked(column as i32, row as i32),
                            Some(true),
                            "{path} roof at {column},{row}"
                        );
                    }
                    if gid
                        .checked_sub(outdoor)
                        .is_some_and(|id| matches!(id, 0 | 2))
                        && row > 0
                        && occupancy.is_open(column as i32, row as i32) == Some(true)
                        && occupancy.is_open(column as i32, row as i32 + 2) == Some(true)
                    {
                        tree_position = Some((column, row));
                    }
                }
            }
            assert!(roof_count > 100);
            let (column, row) =
                tree_position.expect("an accessible tree with space in front and behind");
            let atlases = app.world().resource::<Assets<TsxAtlasAsset>>();
            let bundles = map.visible_bundles(atlases).unwrap();
            let behind = WorldPlayerMotion::from_tile(Position::new(column as i32, row as i32));
            let in_front =
                WorldPlayerMotion::from_tile(Position::new(column as i32, row as i32 + 2));
            let behind_z = world_entity_y_z(behind.sprite_center_world(0.0).y, 32.0);
            let front_z = world_entity_y_z(in_front.sprite_center_world(0.0).y, 32.0);
            let base_z = world_entity_y_z(-(((row + 2) * 32) as f32), 0.0);
            for dy in 0..2 {
                for dx in 0..2 {
                    let tile = bundles
                        .iter()
                        .find(|bundle| {
                            bundle.tile.column() == column + dx
                                && bundle.tile.row() == row + dy
                                && bundle.tile.layer_id() == foreground.id()
                        })
                        .expect("every quarter of the tree is in foreground");
                    assert_eq!(tile.transform.translation.z, base_z);
                    assert!(tile.transform.translation.z > behind_z);
                    assert!(tile.transform.translation.z < front_z);
                    assert_eq!(
                        occupancy.is_blocked((column + dx) as i32, (row + dy) as i32),
                        Some(dy == 1)
                    );
                }
            }
        }
    }

    #[test]
    fn shrine_column_hides_characters_behind_its_upper_tiles_but_not_in_front_of_its_base() {
        use crate::scenario_spatial::{Position, collision_occupancy::CollisionOccupancy};
        use crate::world_player::WorldPlayerMotion;

        let mut app = ground_app();
        let handle = load_map(&mut app, "media/maps/town_01_ardel_shrine.tmx");
        let maps = app.world().resource::<Assets<TmxGroundAsset>>();
        let map = maps.get(&handle).unwrap();
        let document = map.document();
        let collision = CollisionOccupancy::from_tmx_document(document).unwrap();
        let atlases = app.world().resource::<Assets<TsxAtlasAsset>>();
        let bundles = map.visible_bundles(atlases).unwrap();
        assert!(bundles.iter().all(|bundle| {
            document.tile_layers()[bundle.tile.layer_index()].name() != "collision"
        }));
        let column_tile = |row| {
            bundles
                .iter()
                .find(|bundle| {
                    bundle.tile.column() == 2
                        && bundle.tile.row() == row
                        && document.tile_layers()[bundle.tile.layer_index()].name()
                            == if row == 5 { "decoration" } else { "foreground" }
                })
                .unwrap()
        };
        let behind = WorldPlayerMotion::from_tile(Position::new(2, 4));
        let in_front = WorldPlayerMotion::from_tile(Position::new(2, 6));
        let behind_z = world_entity_y_z(behind.sprite_center_world(0.0).y, 32.0);
        let in_front_z = world_entity_y_z(in_front.sprite_center_world(0.0).y, 32.0);
        for row in [3, 4] {
            assert!(column_tile(row).transform.translation.z > behind_z);
            assert_eq!(collision.is_open(2, row as i32), Some(true));
        }
        assert!(column_tile(5).transform.translation.z < in_front_z);
        assert_eq!(collision.is_blocked(2, 5), Some(true));
        assert_eq!(collision.is_open(2, 6), Some(true));
    }

    #[test]
    fn starting_forest_dense_crowns_sort_at_their_roots_when_walkable() {
        use crate::scenario_spatial::{Position, collision_occupancy::CollisionOccupancy};
        use crate::world_player::WorldPlayerMotion;

        let mut app = ground_app();
        let handle = load_map(&mut app, "media/maps/zone_01_starting_forest.tmx");
        let maps = app.world().resource::<Assets<TmxGroundAsset>>();
        let map = maps.get(&handle).unwrap();
        let document = map.document();
        let occupancy = CollisionOccupancy::from_tmx_document(document).unwrap();
        let atlases = app.world().resource::<Assets<TsxAtlasAsset>>();
        let bundles = map.visible_bundles(atlases).unwrap();

        for (column, crown_row, root_row, foreground_last_row) in [
            (9_u32, 22_u32, 28_u32, 28_u32),
            (25, 21, 29, 29),
            (35, 21, 30, 30),
            // These freestanding trees have their bases in decoration.
            (2, 4, 5, 4),
            (4, 7, 8, 7),
        ] {
            assert_eq!(
                occupancy.is_open(column as i32, crown_row as i32),
                Some(true)
            );
            let behind =
                WorldPlayerMotion::from_tile(Position::new(column as i32, crown_row as i32));
            let front =
                WorldPlayerMotion::from_tile(Position::new(column as i32, (root_row + 1) as i32));
            let behind_z = world_entity_y_z(behind.sprite_center_world(0.0).y, 32.0);
            let front_z = world_entity_y_z(front.sprite_center_world(0.0).y, 32.0);
            let root_z = world_entity_y_z(-(((root_row + 1) * 32) as f32), 0.0);
            for row in crown_row..=foreground_last_row {
                let tile = bundles
                    .iter()
                    .find(|bundle| {
                        bundle.tile.column() == column
                            && bundle.tile.row() == row
                            && document.tile_layers()[bundle.tile.layer_index()].name()
                                == "foreground"
                    })
                    .expect("each connected crown row renders in foreground");
                assert_eq!(
                    tile.transform.translation.z, root_z,
                    "tree at {column},{crown_row}; row {row}"
                );
                assert!(tile.transform.translation.z > behind_z);
                assert!(tile.transform.translation.z < front_z);
            }
        }
    }

    #[test]
    fn repeated_forest_canopy_rows_share_the_actual_root_baseline() {
        use crate::scenario_spatial::{Position, collision_occupancy::CollisionOccupancy};
        use crate::world_player::WorldPlayerMotion;
        // Keep the three-row sample in Corrupted Forest: Starting Forest's
        // edited layout no longer contains that tree.
        for (path, trees) in [
            (
                "media/maps/zone_01_starting_forest.tmx",
                &[(47_u32, 15_u32, 4_u32, 1_u32, 1_u32), (0, 27, 6, 2, 5)][..],
            ),
            (
                "media/maps/zone_08_corrupted_forest.tmx",
                &[(13_u32, 4_u32, 3_u32, 2_u32, 2_u32)][..],
            ),
        ] {
            let mut app = ground_app();
            let handle = load_map(&mut app, path);
            let maps = app.world().resource::<Assets<TmxGroundAsset>>();
            let map = maps.get(&handle).unwrap();
            let document = map.document();
            let occupancy = CollisionOccupancy::from_tmx_document(document).unwrap();
            let atlases = app.world().resource::<Assets<TsxAtlasAsset>>();
            let bundles = map.visible_bundles(atlases).unwrap();
            // The four-row exposed edge is one column wide: its right neighbor is
            // a connected forest barrier rather than the other half of that tree.
            for &(column, row, height, width, first_solid_row) in trees {
                let behind = WorldPlayerMotion::from_tile(Position::new(
                    column as i32,
                    (row + height - 2) as i32,
                ));
                let front = WorldPlayerMotion::from_tile(Position::new(
                    column as i32,
                    (row + height) as i32,
                ));
                let behind_z = world_entity_y_z(behind.sprite_center_world(0.0).y, 32.0);
                let front_z = world_entity_y_z(front.sprite_center_world(0.0).y, 32.0);
                let baseline = world_entity_y_z(-(((row + height) * 32) as f32), 0.0);
                for dy in 0..height {
                    for dx in 0..width {
                        let tile = bundles
                            .iter()
                            .find(|bundle| {
                                bundle.tile.column() == column + dx
                                    && bundle.tile.row() == row + dy
                                    && is_foreground_layer(
                                        document.tile_layers()[bundle.tile.layer_index()].name(),
                                    )
                            })
                            .expect("all connecting crown tiles render in foreground");
                        assert_eq!(tile.transform.translation.z, baseline);
                        assert!(baseline > behind_z && baseline < front_z);
                        assert_eq!(
                            occupancy.is_blocked((column + dx) as i32, (row + dy) as i32),
                            Some(dy >= first_solid_row || column + dx == 0)
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn copied_ardel_ground_source_graph_is_byte_identical_to_pinned_scenario() {
        let source_root = scenario_package_dir();
        let destination_root = crate::test_support::scenario_package_dir();
        for relative in [
            "media/maps/town_01_ardel.tmx",
            "media/maps/town_01_ardel_house_01.tmx",
            "media/tilesets/outdoor.tsx",
            "media/tilesets/outdoor.png",
            "media/tilesets/shop.tsx",
            "media/tilesets/shop.png",
            "media/tilesets/interior.tsx",
            "media/tilesets/interior.png",
            "media/tilesets/floor.tsx",
            "media/tilesets/floor.png",
            "media/tilesets/wall.tsx",
            "media/tilesets/wall.png",
            "media/tilesets/roof.tsx",
            "media/tilesets/roof.png",
            "media/tilesets/door.tsx",
            "media/tilesets/door.png",
            "media/tilesets/windows.tsx",
            "media/tilesets/windows.png",
            "media/tilesets/CREDITS-floor.txt",
            "media/tilesets/terrain-v7.tsx",
            "media/tilesets/terrain-v7.png",
            "media/tilesets/CREDITS-terrain.txt",
            "data/maps/town_01_ardel_house_01.yaml",
        ] {
            assert_eq!(
                fs::read(Path::new(&source_root).join(relative)).unwrap(),
                fs::read(destination_root.join(relative)).unwrap(),
                "copied source differs: {relative}"
            );
        }
    }
}
