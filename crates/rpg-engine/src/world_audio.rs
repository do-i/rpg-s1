//! Scenario-authored background music for the active World map.
//!
//! World entry loads the current map's same-stem YAML metadata and the scenario BGM index through
//! Bevy's asset boundary. The resolved track is spawned only after every previously marked BGM
//! entity has been despawned, so title-to-world handoff can never leave two logical players alive.
//!
//! A map transition only stops and replaces the currently playing loop once the destination's
//! metadata is loaded *and* names a `bgm` key. A map with no `bgm` field at all (e.g.
//! `zone_03_marshland`) leaves whatever was already playing running, matching the pinned Python
//! engine's `if bgm_key: bgm_manager.play_key(bgm_key)` guard (`engine/world/world_map_init.py`) —
//! it never stops on a silent map, only when it has somewhere new to send the loop.

use std::{error::Error, fmt};

use bevy::{
    asset::{
        AssetApp, AssetLoader, AssetServer, Assets, Handle, LoadContext, LoadState, io::Reader,
    },
    audio::{PlaybackMode, Volume},
    ecs::system::SystemParam,
    prelude::*,
    reflect::TypePath,
};

use crate::{
    app_state::AppState,
    game_state::GameState,
    scenario_audio::{BGM_INDEX_PATH, BgmIndex},
    scenario_inventory::ScenarioInventory,
    scenario_map::MapMetadata,
    scenario_path::ScenarioRelativePath,
    scenario_root::ScenarioRoot,
    scenario_yaml::{self, ScenarioYamlError},
};

const WORLD_BGM_VOLUME: f32 = 0.3;

/// Loads and owns the looping BGM selected by the active map metadata.
pub(crate) struct WorldAudioPlugin;

impl Plugin for WorldAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<MapMetadata>()
            .init_asset_loader::<MapMetadataAssetLoader>()
            .init_resource::<WorldBgmState>()
            .add_systems(OnEnter(AppState::World), begin_world_audio)
            .add_systems(Update, drive_world_audio.run_if(in_state(AppState::World)))
            .add_systems(OnExit(AppState::World), cleanup_world_audio);
    }
}

/// Marks every long-lived background-music player, regardless of the state that owns it.
///
/// One-shot UI and gameplay sounds deliberately do not carry this marker. The shared marker lets
/// the incoming state enforce one logical BGM player before it starts its own loop.
#[derive(Component)]
pub(crate) struct LogicalBgmPlayer;

/// Identifies the active scenario-selected World loop for diagnostics and lifecycle tests.
#[derive(Component, Debug, Eq, PartialEq)]
pub(crate) struct WorldBgmPlayer {
    map_id: String,
    key: String,
    asset_path: String,
}

impl WorldBgmPlayer {
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "runtime diagnostics are tested now")
    )]
    pub(crate) fn map_id(&self) -> &str {
        &self.map_id
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "runtime diagnostics are tested now")
    )]
    pub(crate) fn key(&self) -> &str {
        &self.key
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "runtime diagnostics are tested now")
    )]
    pub(crate) fn asset_path(&self) -> &str {
        &self.asset_path
    }
}

/// Observable lifecycle of the World BGM request.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum WorldBgmStatus {
    #[default]
    Idle,
    WaitingForGame,
    Loading,
    StoppingPrevious,
    Playing,
    Silent,
    Failed,
}

/// Strong typed handles and publication state for the active map's BGM.
#[derive(Debug, Default, Resource)]
pub(crate) struct WorldBgmState {
    request: Option<WorldBgmRequest>,
    player: Option<Entity>,
    status: WorldBgmStatus,
    failure: Option<WorldBgmFailure>,
}

impl WorldBgmState {
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "runtime diagnostics are tested now")
    )]
    pub(crate) const fn status(&self) -> WorldBgmStatus {
        self.status
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "runtime diagnostics are tested now")
    )]
    pub(crate) const fn failure(&self) -> Option<&WorldBgmFailure> {
        self.failure.as_ref()
    }
}

#[derive(Debug)]
struct WorldBgmRequest {
    map_id: String,
    metadata: Handle<MapMetadata>,
    index: Handle<BgmIndex>,
    /// Set once this map's BGM decision (play a resolved track, or stay silent) has been made, so
    /// a map with no authored `bgm` key is not re-evaluated — and does not retroactively stop
    /// whatever is already playing — on every subsequent frame.
    resolved: bool,
}

/// Stable failure classes that avoid exposing the host AssetServer base path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum WorldBgmFailure {
    MissingCurrentMap,
    InvalidMapId(String),
    MapMetadataLoad { map_id: String },
    BgmIndexLoad,
    MapIdMismatch { expected: String, actual: String },
    UnknownBgmKey { map_id: String, key: String },
}

impl fmt::Display for WorldBgmFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCurrentMap => {
                formatter.write_str("game state has no current map for World BGM")
            }
            Self::InvalidMapId(map_id) => {
                write!(formatter, "World map id `{map_id}` cannot select metadata")
            }
            Self::MapMetadataLoad { map_id } => {
                write!(
                    formatter,
                    "World map metadata for `{map_id}` failed to load"
                )
            }
            Self::BgmIndexLoad => formatter.write_str("scenario BGM index failed to load"),
            Self::MapIdMismatch { expected, actual } => write!(
                formatter,
                "World map metadata id `{actual}` does not match current map `{expected}`"
            ),
            Self::UnknownBgmKey { map_id, key } => {
                write!(
                    formatter,
                    "World map `{map_id}` names unknown BGM key `{key}`"
                )
            }
        }
    }
}

impl Error for WorldBgmFailure {}

fn begin_world_audio(
    mut commands: Commands,
    logical_players: Query<Entity, With<LogicalBgmPlayer>>,
    mut state: ResMut<WorldBgmState>,
) {
    stop_logical_players(&mut commands, &logical_players);
    *state = WorldBgmState::default();
}

#[derive(SystemParam)]
struct WorldAudioAssets<'w> {
    server: Res<'w, AssetServer>,
    metadata: Res<'w, Assets<MapMetadata>>,
    indexes: Res<'w, Assets<BgmIndex>>,
    scenario_root: Res<'w, ScenarioRoot>,
    inventory: Res<'w, ScenarioInventory>,
}

fn drive_world_audio(
    mut commands: Commands,
    assets: WorldAudioAssets,
    game: Option<Res<GameState>>,
    logical_players: Query<Entity, With<LogicalBgmPlayer>>,
    mut state: ResMut<WorldBgmState>,
) {
    let Some(game) = game else {
        state.status = WorldBgmStatus::WaitingForGame;
        state.failure = None;
        return;
    };
    let Some(current_map) = game.map().current() else {
        fail(
            &mut state,
            WorldBgmFailure::MissingCurrentMap,
            &mut commands,
            &logical_players,
        );
        return;
    };
    let map_id = current_map.as_str();

    if state
        .request
        .as_ref()
        .is_none_or(|request| request.map_id != map_id)
    {
        // A new map is current. Only *load* its metadata here — do not touch any already-playing
        // loop yet. The pinned Python engine (`engine/world/world_map_init.py`) only calls
        // `bgm_manager.play_key` when the destination map authors a `bgm` key at all
        // (`if bgm_key: bgm_manager.play_key(bgm_key)`); a map with no `bgm` field at all (e.g.
        // `zone_03_marshland`) leaves whatever track was already playing running, untouched.
        // Stopping playback unconditionally on every map change — as this system used to, before
        // it knew whether the destination even named a track — would silence music the pinned
        // source keeps playing across such a boundary.
        let metadata_path = match assets.inventory.map_metadata_path(map_id) {
            Some(path) => path,
            None => {
                state.request = None;
                state.status = WorldBgmStatus::Failed;
                state.failure = Some(WorldBgmFailure::InvalidMapId(map_id.to_owned()));
                return;
            }
        };
        let bgm_index_path = ScenarioRelativePath::try_from(BGM_INDEX_PATH)
            .expect("the canonical BGM index path must remain scenario-relative");
        state.request = Some(WorldBgmRequest {
            map_id: map_id.to_owned(),
            metadata: assets
                .server
                .load(assets.scenario_root.resolve(&metadata_path)),
            index: assets
                .server
                .load(assets.scenario_root.resolve(&bgm_index_path)),
            resolved: false,
        });
        state.failure = None;
        state.status = WorldBgmStatus::Loading;
        return;
    }

    if state.request.as_ref().expect("checked above").resolved {
        if let Some(player) = state.player {
            if logical_players.get(player).is_ok() {
                // If an unrelated state or system introduced another BGM, retire it while
                // preserving this map's already-playing loop. Rechecked every World update.
                let mut removed_extra = false;
                for entity in &logical_players {
                    if entity != player {
                        commands.entity(entity).despawn();
                        removed_extra = true;
                    }
                }
                if removed_extra {
                    return;
                }
                state.status = WorldBgmStatus::Playing;
                state.failure = None;
                return;
            }
            // The tracked entity vanished outside this system's own decision path; this map's
            // already-finalized decision has nothing left to resume.
            state.player = None;
            state.status = WorldBgmStatus::Silent;
            state.failure = None;
            return;
        }
        // This map's resolved decision was "stay silent" (no authored `bgm`). A stray logical
        // player here was not spawned by this decision, so retire it defensively — the plugin's
        // "at most one logical player" invariant still holds even in the silent case.
        if !logical_players.is_empty() {
            stop_logical_players(&mut commands, &logical_players);
            state.status = WorldBgmStatus::StoppingPrevious;
            state.failure = None;
            return;
        }
        state.status = WorldBgmStatus::Silent;
        state.failure = None;
        return;
    }

    let request = state.request.as_ref().expect("checked above");
    if matches!(
        assets.server.load_state(request.metadata.id()),
        LoadState::Failed(_)
    ) {
        fail(
            &mut state,
            WorldBgmFailure::MapMetadataLoad {
                map_id: map_id.to_owned(),
            },
            &mut commands,
            &logical_players,
        );
        return;
    }
    if matches!(
        assets.server.load_state(request.index.id()),
        LoadState::Failed(_)
    ) {
        fail(
            &mut state,
            WorldBgmFailure::BgmIndexLoad,
            &mut commands,
            &logical_players,
        );
        return;
    }

    let (Some(metadata), Some(index)) = (
        assets.metadata.get(&request.metadata),
        assets.indexes.get(&request.index),
    ) else {
        state.status = WorldBgmStatus::Loading;
        state.failure = None;
        return;
    };

    let effective_id = metadata.effective_id(map_id);
    if effective_id != map_id {
        fail(
            &mut state,
            WorldBgmFailure::MapIdMismatch {
                expected: map_id.to_owned(),
                actual: effective_id.to_owned(),
            },
            &mut commands,
            &logical_players,
        );
        return;
    }
    let Some(key) = metadata.bgm.as_deref() else {
        // No authored `bgm`: resolve this map's decision without touching whatever loop is
        // already playing (see the comment on the map-change branch above). The observable status
        // reflects reality — if a previous map's track is still looping, that is `Playing`, not
        // `Silent`; `Silent` is reserved for when nothing is actually playing.
        state.request.as_mut().expect("checked above").resolved = true;
        state.failure = None;
        state.status = match state.player {
            Some(player) if logical_players.get(player).is_ok() => WorldBgmStatus::Playing,
            _ => {
                state.player = None;
                WorldBgmStatus::Silent
            }
        };
        return;
    };
    let Some(asset_path) = index.resolve_key(&assets.scenario_root, key) else {
        fail(
            &mut state,
            WorldBgmFailure::UnknownBgmKey {
                map_id: map_id.to_owned(),
                key: key.to_owned(),
            },
            &mut commands,
            &logical_players,
        );
        return;
    };

    if !logical_players.is_empty() {
        stop_logical_players(&mut commands, &logical_players);
        state.player = None;
        state.status = WorldBgmStatus::StoppingPrevious;
        state.failure = None;
        return;
    }

    let player = commands
        .spawn((
            AudioPlayer::new(assets.server.load(asset_path.clone())),
            PlaybackSettings {
                mode: PlaybackMode::Loop,
                volume: Volume::Linear(WORLD_BGM_VOLUME),
                ..default()
            },
            LogicalBgmPlayer,
            WorldBgmPlayer {
                map_id: map_id.to_owned(),
                key: key.to_owned(),
                asset_path,
            },
        ))
        .id();
    state.player = Some(player);
    state.request.as_mut().expect("checked above").resolved = true;
    state.status = WorldBgmStatus::Playing;
    state.failure = None;
}

fn fail(
    state: &mut WorldBgmState,
    failure: WorldBgmFailure,
    commands: &mut Commands,
    logical_players: &Query<Entity, With<LogicalBgmPlayer>>,
) {
    stop_logical_players(commands, logical_players);
    state.request = None;
    state.player = None;
    state.status = WorldBgmStatus::Failed;
    state.failure = Some(failure);
}

fn stop_logical_players(
    commands: &mut Commands,
    logical_players: &Query<Entity, With<LogicalBgmPlayer>>,
) {
    for entity in logical_players {
        commands.entity(entity).despawn();
    }
}

fn cleanup_world_audio(
    mut commands: Commands,
    logical_players: Query<Entity, With<LogicalBgmPlayer>>,
    mut state: ResMut<WorldBgmState>,
) {
    stop_logical_players(&mut commands, &logical_players);
    *state = WorldBgmState::default();
}

#[derive(Default, TypePath)]
struct MapMetadataAssetLoader;

impl AssetLoader for MapMetadataAssetLoader {
    type Asset = MapMetadata;
    type Settings = ();
    type Error = WorldYamlAssetLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        load_yaml(reader).await
    }

    fn extensions(&self) -> &[&str] {
        &["yaml", "yml"]
    }
}

#[derive(Default, TypePath)]
pub(crate) struct BgmIndexAssetLoader;

impl AssetLoader for BgmIndexAssetLoader {
    type Asset = BgmIndex;
    type Settings = ();
    type Error = WorldYamlAssetLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        load_yaml(reader).await
    }

    fn extensions(&self) -> &[&str] {
        &["yaml", "yml"]
    }
}

async fn load_yaml<T: serde::de::DeserializeOwned>(
    reader: &mut dyn Reader,
) -> Result<T, WorldYamlAssetLoaderError> {
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .await
        .map_err(WorldYamlAssetLoaderError::Io)?;
    let document = std::str::from_utf8(&bytes).map_err(WorldYamlAssetLoaderError::Utf8)?;
    scenario_yaml::from_str(document).map_err(WorldYamlAssetLoaderError::Yaml)
}

#[derive(Debug)]
pub(crate) enum WorldYamlAssetLoaderError {
    Io(std::io::Error),
    Utf8(std::str::Utf8Error),
    Yaml(ScenarioYamlError),
}

impl fmt::Display for WorldYamlAssetLoaderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "asset I/O failed ({:?})", error.kind()),
            Self::Utf8(error) => write!(formatter, "asset is not UTF-8: {error}"),
            Self::Yaml(error) => write!(formatter, "invalid scenario YAML: {error}"),
        }
    }
}

impl Error for WorldYamlAssetLoaderError {}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
        thread,
        time::Duration,
    };

    use super::*;
    use crate::{
        name_entry::NameEntryConfirmed,
        new_game::{NewGameScenario, build_new_game_state},
        runtime_map::RuntimeMapId,
        scenario_balance::BalanceData,
        scenario_manifest::Manifest,
        scenario_new_game_assets::{ActiveNewGameInputs, ActiveNewGameInputsStatus},
        scenario_party::PartyCatalog,
        scenario_spatial::{CardinalDirection, Position},
        test_support::headless_title_app_with_asset_base,
        tmx_ground_asset::{StaticMapTile, TmxGroundAssetPlugin},
        tsx_atlas_asset::TsxAtlasAssetPlugin,
        world_player::{WorldPlayer, WorldPlayerPlugin},
    };
    use bevy::{
        audio::{AudioLoader, Decodable},
        image::{CompressedImageFormats, ImageLoader, ImagePlugin},
        text::FontLoader,
    };

    static NEXT_PACKAGE: AtomicU64 = AtomicU64::new(0);

    struct TestPackage {
        asset_base: PathBuf,
    }

    impl TestPackage {
        fn new(map_metadata: &str, bgm_index: &str) -> Self {
            Self::with_maps(&[("town_01_ardel", map_metadata)], bgm_index)
        }

        /// Writes several map metadata documents into one invented scenario package, so a test
        /// can drive `GameState::map_mut().move_to(...)` between them.
        fn with_maps(maps: &[(&str, &str)], bgm_index: &str) -> Self {
            let asset_base = std::env::temp_dir().join(format!(
                "rpg-s1-world-audio-{}-{}",
                std::process::id(),
                NEXT_PACKAGE.fetch_add(1, Ordering::Relaxed)
            ));
            let scenario = asset_base.join("scenarios/invented");
            for directory in [
                "assets/maps",
                "data/audio",
                "data/classes",
                "data/encount",
                "data/enemies",
                "data/items",
                "data/maps",
                "data/recipe",
            ] {
                fs::create_dir_all(scenario.join(directory)).unwrap();
            }
            fs::write(
                scenario.join("manifest.yaml"),
                include_str!("../../../tests/fixtures/rusted-kingdoms-manifest-complete.yaml"),
            )
            .unwrap();
            for (map_id, map_metadata) in maps {
                fs::write(
                    scenario.join(format!("data/maps/{map_id}.yaml")),
                    map_metadata,
                )
                .unwrap();
            }
            fs::write(
                scenario.join("data/audio/bgm_index.yaml"),
                format!("title:\n  default: bgm/invented-title.mp3\n{bgm_index}"),
            )
            .unwrap();
            fs::write(
                scenario.join("data/audio/sfx_index.yaml"),
                "ui:\n  hover: sfx/hover.mp3\n  confirm: sfx/confirm.mp3\n",
            )
            .unwrap();
            Self { asset_base }
        }
    }

    impl Drop for TestPackage {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.asset_base).unwrap();
        }
    }

    fn new_game() -> GameState {
        let manifest: Manifest = scenario_yaml::from_str(include_str!(
            "../../../tests/fixtures/rusted-kingdoms-manifest-complete.yaml"
        ))
        .unwrap();
        let party_document = include_str!("../../../tests/fixtures/party-catalog-shapes.yaml")
            .replacen("id: ember", "id: aric", 1)
            .replacen("class: vanguard", "class: hero", 1);
        let party: PartyCatalog = scenario_yaml::from_str(&party_document).unwrap();
        let balance: BalanceData = scenario_yaml::from_str(include_str!(
            "../../../tests/fixtures/balance-complete.yaml"
        ))
        .unwrap();
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

    fn app(package: &TestPackage, initial_state: AppState) -> App {
        let mut app = headless_title_app_with_asset_base(
            initial_state,
            package.asset_base.to_string_lossy().into_owned(),
            ScenarioRoot::try_for_package_key("invented").unwrap(),
        );
        app.insert_resource(new_game())
            .add_plugins(WorldAudioPlugin);
        app
    }

    fn logical_players(app: &mut App) -> Vec<Entity> {
        app.world_mut()
            .query_filtered::<Entity, With<LogicalBgmPlayer>>()
            .iter(app.world())
            .collect()
    }

    fn wait_for_status(app: &mut App, expected: WorldBgmStatus) {
        for _ in 0..5_000 {
            app.update();
            assert!(
                logical_players(app).len() <= 1,
                "the shared BGM marker must never expose overlapping logical players"
            );
            if app.world().resource::<WorldBgmState>().status() == expected {
                return;
            }
            thread::sleep(Duration::from_millis(1));
        }
        let state = app.world().resource::<WorldBgmState>();
        panic!(
            "World BGM did not reach {expected:?}: status={:?}, failure={:?}",
            state.status(),
            state.failure()
        );
    }

    #[test]
    fn title_loop_is_gone_before_ardel_starts_one_indexed_loop() {
        let package = TestPackage::new(
            include_str!("../../../tests/fixtures/ardel-map-metadata-complete.yaml"),
            include_str!("../../../tests/fixtures/audio-bgm-index.yaml"),
        );
        let mut app = app(&package, AppState::Title);

        for _ in 0..5_000 {
            app.update();
            if !logical_players(&mut app).is_empty() {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        let [title_player] = logical_players(&mut app)[..] else {
            panic!("Title must own exactly one logically marked BGM before transition");
        };
        assert!(app.world().get::<WorldBgmPlayer>(title_player).is_none());

        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::World);
        wait_for_status(&mut app, WorldBgmStatus::Playing);

        let [world_player] = logical_players(&mut app)[..] else {
            panic!("World must own exactly one logically marked BGM after transition");
        };
        assert_ne!(world_player, title_player);
        assert!(app.world().get_entity(title_player).is_err());

        let selection = app
            .world()
            .get::<WorldBgmPlayer>(world_player)
            .expect("the remaining logical player must be World-owned");
        assert_eq!(selection.map_id(), "town_01_ardel");
        assert_eq!(selection.key(), "town.default");
        assert_eq!(
            selection.asset_path(),
            "scenarios/invented/assets/audio/bgm/Invented_Town.mp3"
        );
        let settings = app.world().get::<PlaybackSettings>(world_player).unwrap();
        assert!(matches!(settings.mode, PlaybackMode::Loop));
        assert_eq!(settings.volume, Volume::Linear(WORLD_BGM_VOLUME));

        for _ in 0..10 {
            app.update();
            assert_eq!(logical_players(&mut app), [world_player]);
        }
    }

    #[test]
    fn an_unexpected_second_bgm_is_removed_without_restarting_ardel() {
        let package = TestPackage::new(
            include_str!("../../../tests/fixtures/ardel-map-metadata-complete.yaml"),
            include_str!("../../../tests/fixtures/audio-bgm-index.yaml"),
        );
        let mut app = app(&package, AppState::World);
        wait_for_status(&mut app, WorldBgmStatus::Playing);
        let original = logical_players(&mut app)[0];
        let duplicate = app
            .world_mut()
            .spawn((
                AudioPlayer::new(Handle::<AudioSource>::default()),
                PlaybackSettings::LOOP,
                LogicalBgmPlayer,
            ))
            .id();
        assert_eq!(logical_players(&mut app).len(), 2);

        app.update();

        assert_eq!(logical_players(&mut app), [original]);
        assert!(app.world().get_entity(duplicate).is_err());
        assert_eq!(
            app.world().resource::<WorldBgmState>().status(),
            WorldBgmStatus::Playing
        );
    }

    /// Regression for a real runtime divergence found while fixturing W12.3 (Marshland has no
    /// `bgm` field at all, unlike every W12.1/W12.2 map). The pinned Python engine only calls
    /// `bgm_manager.play_key` when the destination names a `bgm` key
    /// (`engine/world/world_map_init.py`); a silent map leaves whatever was already playing
    /// running. Before this fix, `drive_world_audio` stopped the previous loop unconditionally on
    /// every map change, before it even knew whether the destination had a track to switch to —
    /// so stepping onto a `bgm`-less map cut the music instead of leaving it alone.
    #[test]
    fn a_map_with_no_authored_bgm_leaves_the_previous_track_playing() {
        let package = TestPackage::with_maps(
            &[
                (
                    "town_01_ardel",
                    "id: town_01_ardel\nname: Ardel Village\nbgm: town.default\n",
                ),
                ("zone_03_marshland", "name: Marshland\nwarp_order: 50\n"),
            ],
            include_str!("../../../tests/fixtures/audio-bgm-index.yaml"),
        );
        let mut app = app(&package, AppState::World);
        wait_for_status(&mut app, WorldBgmStatus::Playing);
        let [ardel_player] = logical_players(&mut app)[..] else {
            panic!("Ardel must own exactly one logical BGM player once Playing");
        };
        let asset_path = app
            .world()
            .get::<WorldBgmPlayer>(ardel_player)
            .unwrap()
            .asset_path()
            .to_owned();

        app.world_mut()
            .resource_mut::<GameState>()
            .map_mut()
            .move_to(
                RuntimeMapId::try_new("zone_03_marshland").unwrap(),
                Position::new(4, 1),
                CardinalDirection::Down,
            );
        // The map change is detected and re-enters `Loading` while Marshland's own (bgm-less)
        // metadata resolves, then settles back on `Playing` once the decision is made — the
        // Ardel loop never stops in between.
        wait_for_status(&mut app, WorldBgmStatus::Loading);
        assert_eq!(logical_players(&mut app), [ardel_player]);
        wait_for_status(&mut app, WorldBgmStatus::Playing);

        assert_eq!(logical_players(&mut app), [ardel_player]);
        let still_playing = app.world().get::<WorldBgmPlayer>(ardel_player).unwrap();
        assert_eq!(still_playing.map_id(), "town_01_ardel");
        assert_eq!(still_playing.asset_path(), asset_path);

        for _ in 0..10 {
            app.update();
            assert_eq!(logical_players(&mut app), [ardel_player]);
            assert_eq!(
                app.world().resource::<WorldBgmState>().status(),
                WorldBgmStatus::Playing
            );
        }
    }

    #[test]
    fn unknown_map_bgm_key_fails_without_a_partial_player() {
        let package = TestPackage::new(
            "id: town_01_ardel\nname: Ardel Village\nbgm: town.missing\n",
            include_str!("../../../tests/fixtures/audio-bgm-index.yaml"),
        );
        let mut app = app(&package, AppState::World);

        wait_for_status(&mut app, WorldBgmStatus::Failed);

        assert!(logical_players(&mut app).is_empty());
        assert_eq!(
            app.world().resource::<WorldBgmState>().failure(),
            Some(&WorldBgmFailure::UnknownBgmKey {
                map_id: "town_01_ardel".to_owned(),
                key: "town.missing".to_owned(),
            })
        );
    }

    #[test]
    fn production_package_loads_new_game_font_world_art_and_decodable_ardel_bgm() {
        let asset_base = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets").to_owned();
        let mut app = headless_title_app_with_asset_base(
            AppState::NameEntry,
            asset_base,
            ScenarioRoot::default(),
        );
        app.init_asset_loader::<FontLoader>()
            .init_asset_loader::<AudioLoader>()
            .add_plugins(ImagePlugin::default_nearest())
            .register_asset_loader(ImageLoader::new(CompressedImageFormats::empty()))
            .add_plugins(TsxAtlasAssetPlugin)
            .add_plugins(TmxGroundAssetPlugin)
            .add_plugins(WorldAudioPlugin)
            .add_plugins(WorldPlayerPlugin);

        let font = app
            .world()
            .resource::<AssetServer>()
            .load::<Font>("scenarios/rusted_kingdoms/assets/fonts/Philosopher-Regular.ttf");
        for _ in 0..5_000 {
            app.update();
            let inputs_ready = app.world().resource::<ActiveNewGameInputs>().status()
                == ActiveNewGameInputsStatus::Ready;
            let font_ready = app.world().resource::<Assets<Font>>().contains(&font);
            if inputs_ready && font_ready {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(
            app.world().resource::<ActiveNewGameInputs>().status(),
            ActiveNewGameInputsStatus::Ready,
            "production manifest, party, balance, and intro must load together"
        );
        assert!(
            app.world().resource::<Assets<Font>>().contains(&font),
            "the manifest-selected production font must parse through Bevy's FontLoader"
        );

        app.world_mut()
            .resource_mut::<Messages<NameEntryConfirmed>>()
            .write(NameEntryConfirmed::for_test("Aric"));
        for _ in 0..5_000 {
            app.update();
            if app.world().get_resource::<GameState>().is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        assert!(
            app.world().get_resource::<GameState>().is_some(),
            "production inputs must construct the new-game session"
        );

        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::World);
        for _ in 0..5_000 {
            app.update();
            let player_count = app
                .world_mut()
                .query_filtered::<Entity, With<WorldPlayer>>()
                .iter(app.world())
                .count();
            let tile_count = app
                .world_mut()
                .query_filtered::<Entity, With<StaticMapTile>>()
                .iter(app.world())
                .count();
            if app.world().resource::<WorldBgmState>().status() == WorldBgmStatus::Playing
                && player_count == 1
                && tile_count > 0
            {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }

        let world = app.world_mut();
        assert_eq!(
            world
                .query_filtered::<Entity, With<WorldPlayer>>()
                .iter(world)
                .count(),
            1,
            "the production Aric atlas must load and spawn exactly one player"
        );
        assert!(
            world
                .query_filtered::<Entity, With<StaticMapTile>>()
                .iter(world)
                .count()
                > 0,
            "the production Ardel TMX dependency graph must spawn visible tiles"
        );
        let (selection, player) = world
            .query::<(&WorldBgmPlayer, &AudioPlayer<AudioSource>)>()
            .single(world)
            .expect("production World must own one BGM player");
        assert_eq!(selection.map_id(), "town_01_ardel");
        assert_eq!(selection.key(), "town.default");
        assert_eq!(
            selection.asset_path(),
            "scenarios/rusted_kingdoms/assets/audio/bgm/Whiteveil_Streets.mp3"
        );
        let audio_handle = player.0.clone();

        for _ in 0..5_000 {
            app.update();
            if app
                .world()
                .resource::<Assets<AudioSource>>()
                .contains(&audio_handle)
            {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        let source = app
            .world()
            .resource::<Assets<AudioSource>>()
            .get(&audio_handle)
            .expect("the production Whiteveil MP3 must load through Bevy's AudioLoader");
        assert!(
            source.decoder().next().is_some(),
            "the production Whiteveil MP3 must decode at least one sample"
        );
    }
}
