//! Transactional World-state spawning for Aric's scenario-authored map sprite.
//!
//! The logical runtime position remains a TMX tile coordinate owned by [`GameState`]. Rendering
//! projects that coordinate through the shared top-left-TMX-to-Bevy convention and selects the
//! current facing's validated TSX animation-owner tile as the idle frame.

use std::{error::Error, fmt, time::Duration};

use bevy::{
    asset::{AssetServer, Assets, Handle, LoadState},
    prelude::*,
};

use crate::{
    app_state::AppState, game_state::GameState, gameplay_canvas::camera_follow::CameraFollowTarget,
    scenario_path::ScenarioRelativePath, scenario_root::ScenarioRoot,
    scenario_spatial::aric_atlas::AricAtlasLayout, tile_coordinates::tmx_tile_center,
    tmx_ground_asset::world_entity_y_z, tsx_atlas_asset::TsxAtlasAsset,
};

const ARIC_TSX_PATH: &str = "assets/sprites/party/01_aric_walk.tsx";
const MAP_TILE_WIDTH: u32 = 32;
const MAP_TILE_HEIGHT: u32 = 32;
const PLAYER_SPRITE_HALF_HEIGHT: f32 = 32.0;

/// Loads and owns the one visible World player for the active session.
pub(crate) struct WorldPlayerPlugin;

impl Plugin for WorldPlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldPlayerSpawnState>()
            .add_plugins(crate::scenario_spatial::cardinal_movement::CardinalMovementPlugin)
            .add_systems(OnEnter(AppState::World), begin_world_player_load)
            .add_systems(Update, spawn_world_player.run_if(in_state(AppState::World)))
            .add_systems(
                PostUpdate,
                update_world_player_y_order.run_if(in_state(AppState::World)),
            )
            .add_systems(OnExit(AppState::World), cleanup_world_player);
    }
}

/// Marks the single player sprite owned by the World state.
#[derive(Component)]
pub(crate) struct WorldPlayer;

/// Authored TSX walk-cycle state for the one World player.
///
/// Idle is represented by the direction's animation-owner tile. Walk frames retain the exact
/// TSX tile IDs and per-frame durations; the next-frame cursor survives an idle reset so separate
/// one-tile taps advance through the cycle like the pinned Python controller.
#[derive(Component, Debug)]
pub(crate) struct WorldPlayerAnimation {
    layout: AricAtlasLayout,
    direction: crate::scenario_spatial::CardinalDirection,
    current_walk_frame: Option<usize>,
    next_walk_frame: usize,
    elapsed: Duration,
}

impl WorldPlayerAnimation {
    pub(crate) fn new(
        layout: AricAtlasLayout,
        direction: crate::scenario_spatial::CardinalDirection,
    ) -> Self {
        Self {
            layout,
            direction,
            current_walk_frame: None,
            next_walk_frame: 0,
            elapsed: Duration::ZERO,
        }
    }

    /// Starts one visible grid-step pose or advances its authored hold time toward idle.
    ///
    /// Python can remain movement-active across many continuous pixel updates. The grid port
    /// commits a tile immediately, so each fresh action selects the next authored walk frame and
    /// holds it for that frame's TSX duration before settling to idle. This keeps a one-tile tap
    /// visibly meaningful without changing its logical displacement.
    pub(crate) fn update(
        &mut self,
        movement_facing: Option<crate::scenario_spatial::CardinalDirection>,
        delta: Duration,
    ) -> u32 {
        if let Some(direction) = movement_facing {
            self.direction = direction;
            let frame_count = self.layout.walk_frames(direction).len();
            debug_assert!(frame_count > 0, "validated TSX animations are nonempty");
            let current = self.next_walk_frame % frame_count;
            self.current_walk_frame = Some(current);
            self.next_walk_frame = following_frame(current, frame_count);
            self.elapsed = Duration::ZERO;
            return self.layout.walk_frames(direction)[current].tile_id();
        }

        let Some(current) = self.current_walk_frame else {
            return self.layout.base_frame(self.direction).tile_id();
        };
        self.elapsed = self.elapsed.saturating_add(delta);
        let duration = Duration::from_millis(u64::from(
            self.layout.walk_frames(self.direction)[current].duration_ms(),
        ));
        if self.elapsed >= duration {
            self.current_walk_frame = None;
            self.elapsed = Duration::ZERO;
            return self.layout.base_frame(self.direction).tile_id();
        }

        self.layout.walk_frames(self.direction)[current].tile_id()
    }
}

const fn following_frame(current: usize, frame_count: usize) -> usize {
    if current + 1 == frame_count {
        0
    } else {
        current + 1
    }
}

/// Observable lifecycle of the World player request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WorldPlayerSpawnStatus {
    Idle,
    Loading,
    WaitingForGame,
    Ready,
    Spawned,
    Failed,
}

/// Strong handle and publication state for the current World entry.
#[derive(Debug, Resource)]
pub(crate) struct WorldPlayerSpawnState {
    atlas: Option<Handle<TsxAtlasAsset>>,
    status: WorldPlayerSpawnStatus,
    failure: Option<WorldPlayerSpawnFailure>,
}

impl Default for WorldPlayerSpawnState {
    fn default() -> Self {
        Self {
            atlas: None,
            status: WorldPlayerSpawnStatus::Idle,
            failure: None,
        }
    }
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "spawn diagnostics are observable for tests before a World error UI consumes them"
    )
)]
impl WorldPlayerSpawnState {
    pub(crate) const fn status(&self) -> WorldPlayerSpawnStatus {
        self.status
    }

    pub(crate) const fn failure(&self) -> Option<&WorldPlayerSpawnFailure> {
        self.failure.as_ref()
    }
}

/// Stable failure classes that never expose a host filesystem path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum WorldPlayerSpawnFailure {
    AtlasLoad,
    InvalidAtlas(String),
    MissingCurrentMap,
    NegativeTilePosition { x: i32, y: i32 },
    InvalidBaseFrame(String),
}

impl fmt::Display for WorldPlayerSpawnFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AtlasLoad => formatter.write_str("Aric TSX atlas failed to load"),
            Self::InvalidAtlas(cause) => write!(formatter, "Aric TSX atlas is invalid: {cause}"),
            Self::MissingCurrentMap => {
                formatter.write_str("game state has no current map for the World player")
            }
            Self::NegativeTilePosition { x, y } => {
                write!(
                    formatter,
                    "World player tile position [{x}, {y}] must be non-negative"
                )
            }
            Self::InvalidBaseFrame(cause) => {
                write!(formatter, "Aric base frame is invalid: {cause}")
            }
        }
    }
}

impl Error for WorldPlayerSpawnFailure {}

fn begin_world_player_load(
    asset_server: Res<AssetServer>,
    scenario_root: Res<ScenarioRoot>,
    mut state: ResMut<WorldPlayerSpawnState>,
) {
    let logical = ScenarioRelativePath::try_from(ARIC_TSX_PATH)
        .expect("the canonical Aric TSX path must remain scenario-relative");
    *state = WorldPlayerSpawnState {
        atlas: Some(asset_server.load(scenario_root.resolve(&logical))),
        status: WorldPlayerSpawnStatus::Loading,
        failure: None,
    };
}

fn spawn_world_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    atlases: Res<Assets<TsxAtlasAsset>>,
    game: Option<Res<GameState>>,
    existing_players: Query<Entity, With<WorldPlayer>>,
    mut state: ResMut<WorldPlayerSpawnState>,
) {
    if matches!(
        state.status,
        WorldPlayerSpawnStatus::Idle
            | WorldPlayerSpawnStatus::Spawned
            | WorldPlayerSpawnStatus::Failed
    ) {
        return;
    }
    if !existing_players.is_empty() {
        state.status = WorldPlayerSpawnStatus::Spawned;
        state.failure = None;
        return;
    }

    let Some(handle) = state.atlas.as_ref() else {
        return;
    };
    if matches!(asset_server.load_state(handle.id()), LoadState::Failed(_)) {
        fail_spawn(&mut state, WorldPlayerSpawnFailure::AtlasLoad);
        return;
    }
    if !asset_server.is_loaded_with_dependencies(handle.id()) {
        state.status = WorldPlayerSpawnStatus::Loading;
        state.failure = None;
        return;
    }
    let Some(atlas) = atlases.get(handle) else {
        state.status = WorldPlayerSpawnStatus::Loading;
        state.failure = None;
        return;
    };
    let layout = match AricAtlasLayout::from_tsx_metadata(atlas.metadata()) {
        Ok(layout) => layout,
        Err(error) => {
            fail_spawn(
                &mut state,
                WorldPlayerSpawnFailure::InvalidAtlas(error.to_string()),
            );
            return;
        }
    };

    let Some(game) = game else {
        state.status = WorldPlayerSpawnStatus::WaitingForGame;
        state.failure = None;
        return;
    };
    if game.map().current().is_none() {
        fail_spawn(&mut state, WorldPlayerSpawnFailure::MissingCurrentMap);
        return;
    }
    let tile = game.map().position();
    let Ok(column) = u32::try_from(tile.x) else {
        fail_spawn(
            &mut state,
            WorldPlayerSpawnFailure::NegativeTilePosition {
                x: tile.x,
                y: tile.y,
            },
        );
        return;
    };
    let Ok(row) = u32::try_from(tile.y) else {
        fail_spawn(
            &mut state,
            WorldPlayerSpawnFailure::NegativeTilePosition {
                x: tile.x,
                y: tile.y,
            },
        );
        return;
    };
    let base_frame = layout.base_frame(game.map().facing());
    let sprite = match atlas.sprite_for_tile(base_frame.tile_id()) {
        Ok(sprite) => sprite,
        Err(error) => {
            fail_spawn(
                &mut state,
                WorldPlayerSpawnFailure::InvalidBaseFrame(error.to_string()),
            );
            return;
        }
    };
    let center = tmx_tile_center(column, row, MAP_TILE_WIDTH, MAP_TILE_HEIGHT);
    let translation = center.extend(world_entity_y_z(center.y, PLAYER_SPRITE_HALF_HEIGHT));

    // Publish only after the atlas, image dependency, strict Aric profile, runtime map, selected
    // base frame, and world position have all succeeded.
    if state.status != WorldPlayerSpawnStatus::Ready {
        state.status = WorldPlayerSpawnStatus::Ready;
        state.failure = None;
        return;
    }
    let animation = WorldPlayerAnimation::new(layout, game.map().facing());
    commands.spawn((
        sprite,
        Transform::from_translation(translation),
        WorldPlayer,
        CameraFollowTarget,
        animation,
    ));
    state.status = WorldPlayerSpawnStatus::Spawned;
    state.failure = None;
}

fn update_world_player_y_order(mut players: Query<&mut Transform, With<WorldPlayer>>) {
    for mut transform in &mut players {
        transform.translation.z =
            world_entity_y_z(transform.translation.y, PLAYER_SPRITE_HALF_HEIGHT);
    }
}

fn fail_spawn(state: &mut WorldPlayerSpawnState, failure: WorldPlayerSpawnFailure) {
    state.status = WorldPlayerSpawnStatus::Failed;
    state.failure = Some(failure);
}

fn cleanup_world_player(
    mut commands: Commands,
    players: Query<Entity, With<WorldPlayer>>,
    mut state: ResMut<WorldPlayerSpawnState>,
) {
    for entity in &players {
        commands.entity(entity).despawn();
    }
    *state = WorldPlayerSpawnState::default();
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
        thread,
        time::Duration,
    };

    use bevy::{
        asset::{AssetApp, AssetMetaCheck, AssetPlugin},
        ecs::system::RunSystemOnce,
        image::{CompressedImageFormats, ImageLoader, ImagePlugin, TextureAtlas},
        state::app::StatesPlugin,
    };

    use super::*;
    use crate::{
        game_state::GameState,
        new_game::{NewGameScenario, build_new_game_state},
        runtime_map::RuntimeMapId,
        scenario_balance::BalanceData,
        scenario_manifest::Manifest,
        scenario_party::PartyCatalog,
        scenario_spatial::{CardinalDirection, Position},
        scenario_yaml,
        tsx_atlas_asset::TsxAtlasAssetPlugin,
    };

    const REPOSITORY_ASSET_BASE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets");
    const ARIC_PNG: &[u8] =
        include_bytes!("../assets/scenarios/rusted_kingdoms/assets/sprites/party/01_aric_walk.png");
    const ARIC_TSX: &str =
        include_str!("../assets/scenarios/rusted_kingdoms/assets/sprites/party/01_aric_walk.tsx");
    static NEXT_PACKAGE: AtomicU64 = AtomicU64::new(0);

    struct TestAssetBase {
        root: PathBuf,
    }

    impl TestAssetBase {
        fn empty(package_key: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "rpg-s1-world-player-{}-{}",
                std::process::id(),
                NEXT_PACKAGE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(root.join("scenarios").join(package_key)).unwrap();
            Self { root }
        }

        fn invalid_aric(package_key: &str) -> Self {
            let base = Self::empty(package_key);
            let party = base
                .root
                .join("scenarios")
                .join(package_key)
                .join("assets/sprites/party");
            fs::create_dir_all(&party).unwrap();
            let invalid = ARIC_TSX
                .replace("tilecount=\"36\"", "tilecount=\"45\"")
                .replace("height=\"256\"", "height=\"320\"");
            fs::write(party.join("01_aric_walk.tsx"), invalid).unwrap();
            fs::write(party.join("01_aric_walk.png"), ARIC_PNG).unwrap();
            base
        }
    }

    impl Drop for TestAssetBase {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.root).unwrap();
        }
    }

    fn manifest() -> Manifest {
        scenario_yaml::from_str(include_str!(
            "../tests/fixtures/rusted-kingdoms-manifest-complete.yaml"
        ))
        .unwrap()
    }

    fn party() -> PartyCatalog {
        let fixture = include_str!("../tests/fixtures/party-catalog-shapes.yaml")
            .replacen("id: ember", "id: aric", 1)
            .replacen("class: vanguard", "class: hero", 1);
        scenario_yaml::from_str(&fixture).unwrap()
    }

    fn balance() -> BalanceData {
        scenario_yaml::from_str(include_str!("../tests/fixtures/balance-complete.yaml")).unwrap()
    }

    fn new_game() -> GameState {
        let manifest = manifest();
        let party = party();
        let balance = balance();
        build_new_game_state(
            NewGameScenario {
                manifest: &manifest,
                party: &party,
                balance: &balance,
            },
            Duration::ZERO,
        )
        .unwrap()
    }

    fn world_app(asset_base: &Path, root: ScenarioRoot, game: Option<GameState>) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(StatesPlugin)
            .add_plugins(AssetPlugin {
                file_path: asset_base.to_string_lossy().into_owned(),
                meta_check: AssetMetaCheck::Never,
                ..default()
            })
            .add_plugins(ImagePlugin::default_nearest())
            .register_asset_loader(ImageLoader::new(CompressedImageFormats::empty()))
            .insert_resource(root)
            .add_plugins(TsxAtlasAssetPlugin)
            .insert_state(AppState::World)
            .add_plugins(WorldPlayerPlugin);
        if let Some(game) = game {
            app.insert_resource(game);
        }
        app
    }

    fn player_count(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<Entity, With<WorldPlayer>>()
            .iter(app.world())
            .count()
    }

    fn wait_for_status(app: &mut App, expected: WorldPlayerSpawnStatus) {
        for _ in 0..5_000 {
            app.update();
            if app.world().resource::<WorldPlayerSpawnState>().status() == expected {
                return;
            }
            thread::sleep(Duration::from_millis(1));
        }
        let state = app.world().resource::<WorldPlayerSpawnState>();
        panic!(
            "World player did not reach {expected:?}: status={:?}, failure={:?}",
            state.status(),
            state.failure()
        );
    }

    fn one_player(app: &mut App) -> (Entity, Transform, usize) {
        let world = app.world_mut();
        let (entity, transform, atlas) = world
            .query_filtered::<(Entity, &Transform, &Sprite), With<WorldPlayer>>()
            .single(world)
            .expect("one World player");
        let atlas_index = atlas
            .texture_atlas
            .as_ref()
            .map(|atlas: &TextureAtlas| atlas.index)
            .expect("Aric sprite should select an atlas frame");
        (entity, *transform, atlas_index)
    }

    #[test]
    fn moving_player_depth_tracks_sprite_bottom_without_changing_world_xy() {
        let mut app = App::new();
        let player = app
            .world_mut()
            .spawn((Transform::from_xyz(80.0, -48.0, 777.0), WorldPlayer))
            .id();

        app.world_mut()
            .run_system_once(update_world_player_y_order)
            .unwrap();
        let first = *app.world().entity(player).get::<Transform>().unwrap();
        assert_eq!(first.translation, Vec3::new(80.0, -48.0, 10.08));

        app.world_mut()
            .entity_mut(player)
            .get_mut::<Transform>()
            .unwrap()
            .translation
            .y = -240.0;
        app.world_mut()
            .run_system_once(update_world_player_y_order)
            .unwrap();
        let second = app.world().entity(player).get::<Transform>().unwrap();
        assert_eq!(second.translation, Vec3::new(80.0, -240.0, 10.272));
        assert!(second.translation.z > first.translation.z);
    }

    #[test]
    fn defers_then_spawns_exact_new_game_position_and_down_base_frame_once() {
        let mut app = world_app(
            Path::new(REPOSITORY_ASSET_BASE),
            ScenarioRoot::default(),
            Some(new_game()),
        );

        app.update();
        assert_eq!(player_count(&mut app), 0);
        assert_ne!(
            app.world().resource::<WorldPlayerSpawnState>().status(),
            WorldPlayerSpawnStatus::Spawned
        );
        wait_for_status(&mut app, WorldPlayerSpawnStatus::Spawned);

        let (entity, transform, atlas_index) = one_player(&mut app);
        assert_eq!(transform.translation, Vec3::new(464.0, -176.0, 10.208));
        assert_eq!(atlas_index, 18);
        assert_eq!(player_count(&mut app), 1);
        assert_eq!(
            app.world_mut()
                .query_filtered::<(), (With<WorldPlayer>, With<CameraFollowTarget>)>()
                .iter(app.world())
                .count(),
            1
        );
        for _ in 0..10 {
            app.update();
        }
        assert_eq!(one_player(&mut app).0, entity);
        assert_eq!(player_count(&mut app), 1);
    }

    #[test]
    fn consumes_current_non_ardel_position_and_facing_without_hardcoding_the_map() {
        let mut game = new_game();
        game.map_mut().move_to(
            RuntimeMapId::try_new("invented_keep").unwrap(),
            Position::new(3, 7),
            CardinalDirection::Left,
        );
        let mut app = world_app(
            Path::new(REPOSITORY_ASSET_BASE),
            ScenarioRoot::default(),
            Some(game),
        );

        wait_for_status(&mut app, WorldPlayerSpawnStatus::Spawned);

        let (_, transform, atlas_index) = one_player(&mut app);
        assert_eq!(transform.translation, Vec3::new(112.0, -240.0, 10.272));
        assert_eq!(atlas_index, 9);
    }

    #[test]
    fn loaded_atlas_waits_safely_for_a_game_then_recovers() {
        let mut app = world_app(
            Path::new(REPOSITORY_ASSET_BASE),
            ScenarioRoot::default(),
            None,
        );

        wait_for_status(&mut app, WorldPlayerSpawnStatus::WaitingForGame);
        assert_eq!(player_count(&mut app), 0);
        app.insert_resource(new_game());
        wait_for_status(&mut app, WorldPlayerSpawnStatus::Spawned);
        assert_eq!(player_count(&mut app), 1);
    }

    #[test]
    fn missing_or_non_aric_assets_fail_without_a_partial_sprite() {
        let missing = TestAssetBase::empty("missing");
        let mut missing_app = world_app(
            &missing.root,
            ScenarioRoot::try_for_package_key("missing").unwrap(),
            Some(new_game()),
        );
        wait_for_status(&mut missing_app, WorldPlayerSpawnStatus::Failed);
        assert_eq!(
            missing_app
                .world()
                .resource::<WorldPlayerSpawnState>()
                .failure(),
            Some(&WorldPlayerSpawnFailure::AtlasLoad)
        );
        assert_eq!(player_count(&mut missing_app), 0);

        let invalid = TestAssetBase::invalid_aric("invalid");
        let mut invalid_app = world_app(
            &invalid.root,
            ScenarioRoot::try_for_package_key("invalid").unwrap(),
            Some(new_game()),
        );
        wait_for_status(&mut invalid_app, WorldPlayerSpawnStatus::Failed);
        let failure = invalid_app
            .world()
            .resource::<WorldPlayerSpawnState>()
            .failure()
            .expect("invalid atlas failure");
        assert!(failure.to_string().contains("tile count must be 36"));
        assert_eq!(player_count(&mut invalid_app), 0);
    }

    #[test]
    fn leaving_world_despawns_the_player_and_resets_its_load_state() {
        let mut app = world_app(
            Path::new(REPOSITORY_ASSET_BASE),
            ScenarioRoot::default(),
            Some(new_game()),
        );
        wait_for_status(&mut app, WorldPlayerSpawnStatus::Spawned);
        assert_eq!(player_count(&mut app), 1);

        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Title);
        app.update();
        app.update();

        assert_eq!(player_count(&mut app), 0);
        let state = app.world().resource::<WorldPlayerSpawnState>();
        assert_eq!(state.status(), WorldPlayerSpawnStatus::Idle);
        assert!(state.failure().is_none());
    }
}
