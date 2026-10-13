use super::*;
use crate::{
    app_state::AppStateTransitionPlugin,
    autosave::AutosavePlugin,
    encounter_assets::EncounterAssetPlugin,
    field_menu_domain::FieldMenuCatalog,
    save_data::tests::{fixture_balance, fixture_game},
    save_store::SaveStore,
    save_ui::SaveSlotCatalog,
    scenario_audio::SfxIndex,
    scenario_balance::BalanceData,
    scenario_manifest::Manifest,
    scenario_manifest_asset::ScenarioManifestAssetPlugin,
    tmx_ground_asset::TmxGroundAssetPlugin,
    tsx_atlas_asset::TsxAtlasAssetPlugin,
    world_actor::WorldActorPlugin,
    world_audio::WorldAudioPlugin,
    world_encounter::WorldEncounterPlugin,
    world_object::WorldObjectPlugin,
    world_player::WorldPlayerPlugin,
};
use bevy::{
    asset::{AssetMetaCheck, AssetPlugin},
    image::{CompressedImageFormats, ImageLoader, ImagePlugin},
    state::app::StatesPlugin,
    time::TimeUpdateStrategy,
};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::Duration,
};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);
const PLAYER_TSX: &str = include_str!(scenario_file!("media/sprites/party/01_aric_walk.tsx"));
const PLAYER_PNG: &[u8] = include_bytes!(scenario_file!("media/sprites/party/01_aric_walk.png"));
const BOX_TSX: &str = include_str!(scenario_file!("media/sprites/objects/item_box.tsx"));
const BOX_PNG: &[u8] = include_bytes!(scenario_file!("media/sprites/objects/item_box.png"));
const MAP: &str = "<map version=\"1.10\" orientation=\"orthogonal\" width=\"2\" height=\"2\" tilewidth=\"32\" tileheight=\"32\" infinite=\"0\"><tileset firstgid=\"1\" source=\"../sprites/objects/item_box.tsx\"/><layer id=\"1\" name=\"ground\" width=\"2\" height=\"2\"><data encoding=\"csv\">1,0,\n0,0</data></layer><layer id=\"2\" name=\"collision\" width=\"2\" height=\"2\"><data encoding=\"csv\">0,0,\n0,0</data></layer></map>";

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let fixture = Self(std::env::temp_dir().join(format!(
            "rpg-publication-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        )));
        for directory in ["items", "classes", "recipe", "encount", "enemies"] {
            fs::create_dir_all(fixture.0.join("scenarios/fixture/data").join(directory)).unwrap();
        }
        fixture.write(
            "manifest.yaml",
            include_str!("../../../../../tests/fixtures/rusted-kingdoms-manifest-complete.yaml")
                .replace("tileset: stone_tile_stares_16x16", "tileset: item_box")
                .replace("tile_ids: [18, 19, 20, 21]", "tile_ids: [0]")
                .as_bytes(),
        );
        for map in ["source", "destination"] {
            fixture.write(&format!("media/maps/{map}.tmx"), MAP.as_bytes());
            fixture.write(
                &format!("data/maps/{map}.yaml"),
                format!(
                    "id: {map}\nname: Fixture\nnpcs:\n  - id: witness\n    name: Witness\n    position: [1, 1]\nitem_boxes:\n  - id: source_box\n    position: [0, 1]\n"
                )
                .as_bytes(),
            );
        }
        fixture.write(
            "media/sprites/party/01_aric_walk.tsx",
            PLAYER_TSX.as_bytes(),
        );
        fixture.write("media/sprites/party/01_aric_walk.png", PLAYER_PNG);
        fixture.write("media/sprites/objects/item_box.tsx", BOX_TSX.as_bytes());
        fixture.write("media/sprites/objects/item_box.png", BOX_PNG);
        fixture
    }
    fn write(&self, path: &str, bytes: &[u8]) {
        let path = self.0.join("scenarios/fixture").join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    fn app(&self) -> App {
        self.app_for_package("fixture")
    }
    fn app_for_package(&self, package: &str) -> App {
        let root = ScenarioRoot::try_for_package_key(package).unwrap();
        let mut game = fixture_game();
        let party: crate::scenario_party::PartyCatalog =
            crate::scenario_yaml::from_str(include_str!(scenario_file!("data/party.yaml")))
                .unwrap();
        let companion = &party.party[1];
        game.party_mut()
            .try_add(
                crate::runtime_member::RuntimeMember::try_from_catalog(
                    companion,
                    &crate::runtime_member::test_class(&companion.data().class_id),
                    &fixture_balance().progression,
                )
                .unwrap(),
            )
            .unwrap();
        game.map_mut().move_to(
            RuntimeMapId::try_new("source").unwrap(),
            Position::new(0, 0),
            CardinalDirection::Left,
        );
        let inventory = ScenarioInventory::discover(&self.0, &root);
        assert!(inventory.failure.is_none(), "{:?}", inventory.failure);
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(StatesPlugin)
            .add_plugins(AssetPlugin {
                file_path: self.0.to_string_lossy().into_owned(),
                meta_check: AssetMetaCheck::Never,
                watch_for_changes_override: Some(false),
                ..default()
            })
            .add_plugins(ImagePlugin::default_nearest())
            .register_asset_loader(ImageLoader::new(CompressedImageFormats::empty()))
            .init_asset::<AudioSource>()
            .register_asset_loader(bevy::audio::AudioLoader)
            .init_asset::<BalanceData>()
            .init_asset::<SfxIndex>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<FieldMenuCatalog>()
            .add_message::<PlaySfx>()
            .insert_resource(inventory)
            .insert_resource(root)
            .insert_resource(game)
            .insert_resource(SaveStore::new(self.0.join("saves")))
            .insert_resource(SaveSlotCatalog::ready_for_test(
                "my_rpg_story",
                "1.0.0",
                fixture_balance(),
            ))
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
                10,
            )))
            .insert_state(AppState::World)
            .add_plugins((
                AppStateTransitionPlugin,
                ScenarioManifestAssetPlugin,
                crate::ScenarioAudioAssetPlugin,
                TsxAtlasAssetPlugin,
                TmxGroundAssetPlugin,
                WorldAudioPlugin,
                WorldActorPlugin,
                WorldObjectPlugin,
                WorldPlayerPlugin,
                EncounterAssetPlugin,
                WorldEncounterPlugin,
                WorldTransitionPlugin,
                AutosavePlugin,
            ));
        app
    }
    fn autosave(&self) -> PathBuf {
        self.0.join("saves/000.yaml")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn wait_for(app: &mut App, phase: TransitionPhase) {
    for _ in 0..4_000 {
        app.update();
        if app.world().resource::<WorldTransition>().phase == phase {
            return;
        }
        thread::sleep(Duration::from_millis(1));
    }
    panic!(
        "did not reach {phase:?}: {:?}",
        app.world().resource::<WorldTransition>()
    );
}

fn press(app: &mut App, key: KeyCode) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(key);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
}

fn request(app: &mut App) {
    assert!(
        app.world_mut()
            .resource_mut::<WorldTransition>()
            .request_destination(
                RuntimeMapId::try_new("destination").unwrap(),
                Position::new(1, 0),
                CardinalDirection::Right
            )
    );
}

fn active_entities(world: &mut World) -> Vec<Entity> {
    let mut entities = world
        .query_filtered::<Entity, Or<(
            With<WorldPlayer>,
            With<WorldNpc>,
            With<WorldSign>,
            With<WorldItemBox>,
            With<WorldMapCamera>,
            With<StaticMapTile>,
            With<WorldEnemy>,
        )>>()
        .iter(world)
        .collect::<Vec<_>>();
    entities.sort();
    entities
}

#[test]
fn late_npc_image_failure_restores_exact_source_entities_session_and_save_then_return_works() {
    let fixture = Fixture::new();
    fixture.write("data/maps/destination.yaml", b"id: destination\nname: Destination\ndismiss_companions: true\nnpcs:\n  - id: broken\n    name: Broken\n    position: [1, 1]\n    sprite: media/sprites/late.tsx\n");
    fixture.write("media/sprites/late.tsx", PLAYER_TSX.as_bytes()); // valid TSX, absent PNG dependency
    let mut app = fixture.app();
    wait_for(&mut app, TransitionPhase::Idle);
    let save = fs::read(fixture.autosave()).unwrap();
    let player = app
        .world_mut()
        .query_filtered::<Entity, With<WorldPlayer>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .get_mut::<WorldPlayerMotion>(player)
        .unwrap()
        .set_top_left(Vec2::new(3.25, 7.75));
    let motion = *app.world().get::<WorldPlayerMotion>(player).unwrap();
    let source = app.world().resource::<GameState>().clone();
    let entities = active_entities(app.world_mut());
    request(&mut app);
    wait_for(&mut app, TransitionPhase::Publishing);
    assert!(
        app.world().get::<WorldPlayer>(player).is_none(),
        "source player must be dormant"
    );
    assert!(
        app.world().get::<Sprite>(player).is_none(),
        "dormant source must not render"
    );
    wait_for(&mut app, TransitionPhase::Failed);
    assert!(
        app.world()
            .resource::<WorldTransition>()
            .failure()
            .unwrap()
            .contains("NPC sprite or image")
    );
    assert_eq!(
        app.world().resource::<GameState>(),
        &source,
        "RNG, party, location, facing and visited history roll back together"
    );
    assert_eq!(
        *app.world().get::<WorldPlayerMotion>(player).unwrap(),
        motion
    );
    assert_eq!(
        active_entities(app.world_mut()),
        entities,
        "no destination or duplicate source entities"
    );
    assert_eq!(fs::read(fixture.autosave()).unwrap(), save);
    assert!(
        app.world_mut()
            .query_filtered::<Entity, With<RecoveryUi>>()
            .iter(app.world())
            .count()
            >= 2,
        "recovery has text and its own camera"
    );
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(
        app.world().resource::<GameState>(),
        &source,
        "failed screen holds simulation still"
    );
    press(&mut app, KeyCode::Escape);
    wait_for(&mut app, TransitionPhase::Idle);
    assert_eq!(fs::read(fixture.autosave()).unwrap(), save);
    assert!(!app.world().resource::<WorldTransition>().input_locked());
}

#[test]
fn explicit_retry_reloads_failed_dependency_and_discards_the_source_after_success() {
    let fixture = Fixture::new();
    fixture.write("data/maps/destination.yaml", b"id: destination\nname: Destination\nnpcs:\n  - id: broken\n    name: Broken\n    position: [1, 1]\n    sprite: media/sprites/late.tsx\n");
    fixture.write("media/sprites/late.tsx", PLAYER_TSX.as_bytes());
    let mut app = fixture.app();
    wait_for(&mut app, TransitionPhase::Idle);
    let source_entities = active_entities(app.world_mut());
    request(&mut app);
    wait_for(&mut app, TransitionPhase::Failed);
    fixture.write("media/sprites/01_aric_walk.png", PLAYER_PNG);
    press(&mut app, KeyCode::KeyR);
    wait_for(&mut app, TransitionPhase::Idle);
    assert_eq!(
        app.world()
            .resource::<GameState>()
            .map()
            .current()
            .unwrap()
            .as_str(),
        "destination"
    );
    for entity in source_entities {
        assert!(
            app.world().get_entity(entity).is_err(),
            "successful publication discards dormant source entities"
        );
    }
    let save = fs::read_to_string(fixture.autosave()).unwrap();
    assert!(save.contains("current: destination"), "{save}");
}

#[test]
fn malformed_destination_metadata_has_a_visible_diagnostic_and_preserves_source() {
    let fixture = Fixture::new();
    fixture.write("data/maps/destination.yaml", b"npcs: [malformed");
    let mut app = fixture.app();
    wait_for(&mut app, TransitionPhase::Idle);
    let source = app.world().resource::<GameState>().clone();
    request(&mut app);
    wait_for(&mut app, TransitionPhase::Failed);
    assert!(
        app.world()
            .resource::<WorldTransition>()
            .failure()
            .unwrap()
            .contains("metadata")
    );
    assert_eq!(app.world().resource::<GameState>(), &source);
    assert!(
        app.world()
            .resource::<TransitionRecovery>()
            .return_available
    );
}

#[test]
fn object_sprite_failure_rolls_back_destination_npc_mutations() {
    let fixture = Fixture::new();
    fixture.write("data/maps/destination.yaml", b"id: destination\nname: Destination\ndismiss_companions: true\nnpcs:\n  - id: farewell\n    name: Farewell\n    position: [1, 1]\n");
    let mut app = fixture.app();
    wait_for(&mut app, TransitionPhase::Idle);
    // Replace only the object sprite path after source publication. Destination NPCs may
    // spawn and consume RNG before the independently loaded item-box atlas fails.
    let handle = app
        .world()
        .resource::<ActiveManifestLoad>()
        .handle()
        .clone();
    app.world_mut()
        .resource_mut::<Assets<Manifest>>()
        .get_mut(&handle)
        .unwrap()
        .item_box
        .sprite =
        crate::scenario_path::ScenarioRelativePath::try_from("media/sprites/absent.tsx").unwrap();
    let source = app.world().resource::<GameState>().clone();
    request(&mut app);
    wait_for(&mut app, TransitionPhase::Failed);
    assert!(
        app.world()
            .resource::<WorldTransition>()
            .failure()
            .unwrap()
            .contains("item-box")
    );
    assert_eq!(app.world().resource::<GameState>(), &source);
}

#[test]
fn missing_protagonist_on_initial_entry_never_unlocks_or_saves_and_title_is_usable() {
    let fixture = Fixture::new();
    fs::remove_file(
        fixture
            .0
            .join("scenarios/fixture/media/sprites/party/01_aric_walk.tsx"),
    )
    .unwrap();
    let mut app = fixture.app();
    wait_for(&mut app, TransitionPhase::Failed);
    assert!(
        app.world()
            .resource::<WorldTransition>()
            .failure()
            .unwrap()
            .contains("protagonist")
    );
    assert!(!fixture.autosave().exists());
    assert!(
        !app.world()
            .resource::<TransitionRecovery>()
            .return_available
    );
    press(&mut app, KeyCode::KeyT);
    app.update();
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::Title
    );
    assert!(active_entities(app.world_mut()).is_empty());
    assert!(!fixture.autosave().exists());
}

#[test]
fn unresolved_publication_has_a_bounded_timeout_and_a_visible_recovery() {
    let fixture = Fixture::new();
    let mut app = fixture.app();
    wait_for(&mut app, TransitionPhase::Idle);
    request(&mut app);
    wait_for(&mut app, TransitionPhase::Publishing);
    // A valid request with no completion is distinguishable from a terminal loader failure.
    // Drive the production barrier alone so the destination cannot resolve in other systems.
    app.world_mut()
        .resource_mut::<WorldTransition>()
        .wait_seconds = LOAD_TIMEOUT_SECONDS;
    drive_transition_loading(app.world_mut());
    update_recovery_ui(app.world_mut());
    assert_eq!(
        app.world().resource::<WorldTransition>().phase,
        TransitionPhase::Failed
    );
    assert!(
        app.world()
            .resource::<WorldTransition>()
            .failure()
            .unwrap()
            .contains("timed out")
    );
    assert!(
        app.world()
            .resource::<TransitionRecovery>()
            .return_available
    );
    assert_eq!(
        app.world()
            .resource::<GameState>()
            .map()
            .current()
            .unwrap()
            .as_str(),
        "source"
    );
}

#[test]
fn initial_retry_restores_rng_and_party_before_repeating_publication() {
    let clean_fixture = Fixture::new();
    let mut clean = clean_fixture.app();
    wait_for(&mut clean, TransitionPhase::FadingIn);
    let expected = clean.world().resource::<GameState>().clone();

    let fixture = Fixture::new();
    fs::remove_file(
        fixture
            .0
            .join("scenarios/fixture/media/sprites/party/01_aric_walk.tsx"),
    )
    .unwrap();
    let mut app = fixture.app();
    wait_for(&mut app, TransitionPhase::Failed);
    assert!(!fixture.autosave().exists());
    fixture.write(
        "media/sprites/party/01_aric_walk.tsx",
        PLAYER_TSX.as_bytes(),
    );
    press(&mut app, KeyCode::KeyR);
    wait_for(&mut app, TransitionPhase::FadingIn);
    assert_eq!(app.world().resource::<GameState>(), &expected);
    assert!(
        !fixture.autosave().exists(),
        "retry must still wait for the fade to finish"
    );
}

#[test]
fn malformed_encounter_zone_is_part_of_the_publication_barrier() {
    let fixture = Fixture::new();
    fixture.write("data/encount/destination.yaml", b"encounters: [broken");
    let mut app = fixture.app();
    wait_for(&mut app, TransitionPhase::Idle);
    let source = app.world().resource::<GameState>().clone();
    let entities = active_entities(app.world_mut());
    request(&mut app);
    wait_for(&mut app, TransitionPhase::Failed);
    assert!(
        app.world()
            .resource::<WorldTransition>()
            .failure()
            .unwrap()
            .contains("encounter zone")
    );
    assert_eq!(app.world().resource::<GameState>(), &source);
    assert_eq!(active_entities(app.world_mut()), entities);
}

#[test]
fn leaving_world_during_publication_disposes_dormant_source_and_partial_destination() {
    let fixture = Fixture::new();
    let mut app = fixture.app();
    wait_for(&mut app, TransitionPhase::Idle);
    let entities = active_entities(app.world_mut());
    request(&mut app);
    wait_for(&mut app, TransitionPhase::Publishing);
    assert!(
        app.world()
            .resource::<TransitionRecovery>()
            .source
            .is_some()
    );
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Title);
    app.update();
    for entity in entities {
        assert!(app.world().get_entity(entity).is_err());
    }
    assert!(active_entities(app.world_mut()).is_empty());
    assert!(
        app.world()
            .resource::<TransitionRecovery>()
            .source
            .is_none()
    );
}

#[test]
fn tmx_dependency_failure_during_preflight_keeps_source_entities_active() {
    let fixture = Fixture::new();
    fixture.write(
        "media/maps/destination.tmx",
        MAP.replace("../sprites/objects/item_box.tsx", "../sprites/late.tsx")
            .as_bytes(),
    );
    fixture.write("media/sprites/late.tsx", BOX_TSX.as_bytes());
    let mut app = fixture.app();
    wait_for(&mut app, TransitionPhase::Idle);
    let source = app.world().resource::<GameState>().clone();
    let entities = active_entities(app.world_mut());
    let save = fs::read(fixture.autosave()).unwrap();
    request(&mut app);
    wait_for(&mut app, TransitionPhase::Failed);
    assert!(
        app.world()
            .resource::<WorldTransition>()
            .failure()
            .unwrap()
            .contains("destination map or image")
    );
    assert_eq!(app.world().resource::<GameState>(), &source);
    assert_eq!(active_entities(app.world_mut()), entities);
    assert_eq!(fs::read(fixture.autosave()).unwrap(), save);
    assert!(
        app.world()
            .resource::<TransitionRecovery>()
            .source
            .is_none()
    );
}

#[test]
fn unresolved_preflight_times_out_without_touching_the_source() {
    let fixture = Fixture::new();
    let mut app = fixture.app();
    wait_for(&mut app, TransitionPhase::Idle);
    let source = app.world().resource::<GameState>().clone();
    let entities = active_entities(app.world_mut());
    request(&mut app);
    let handle = app
        .world()
        .resource::<Assets<TmxGroundAsset>>()
        .reserve_handle();
    app.world_mut()
        .resource_mut::<TransitionDestinationLoad>()
        .handle = Some(handle);
    let mut transition = app.world_mut().resource_mut::<WorldTransition>();
    transition.phase = TransitionPhase::Loading;
    transition.wait_seconds = LOAD_TIMEOUT_SECONDS;
    drive_transition_loading(app.world_mut());
    assert_eq!(
        app.world().resource::<WorldTransition>().phase,
        TransitionPhase::Failed
    );
    assert!(
        app.world()
            .resource::<WorldTransition>()
            .failure()
            .unwrap()
            .contains("timed out")
    );
    assert_eq!(app.world().resource::<GameState>(), &source);
    assert_eq!(active_entities(app.world_mut()), entities);
}

#[test]
fn companion_dismissal_waits_for_the_final_controlled_player_sprite() {
    let fixture = Fixture::new();
    fixture.write(
        "data/maps/destination.yaml",
        b"id: destination\nname: Destination\ndismiss_companions: true\n",
    );
    let mut app = fixture.app();
    app.world_mut()
        .resource_mut::<GameState>()
        .set_controlled_member("elise")
        .unwrap();
    wait_for(&mut app, TransitionPhase::Idle);
    request(&mut app);
    wait_for(&mut app, TransitionPhase::FadingIn);
    assert_eq!(
        app.world().resource::<GameState>().controlled_member_id(),
        "aric"
    );
    assert_eq!(app.world().resource::<GameState>().party().len(), 1);
    assert_eq!(
        app.world()
            .resource::<WorldPlayerSpawnState>()
            .publication_status("destination", "aric"),
        PublicationStatus::Ready
    );
}

fn hot_app(fixture: &Fixture) -> App {
    let mut app = fixture.app();
    app.init_resource::<crate::gameplay_rng::GameplayRng>();
    app.add_plugins(super::super::hot_reload::ScenarioHotReloadPlugin::new(
        fixture.0.clone(),
        true,
    ));
    wait_for(&mut app, TransitionPhase::Idle);
    app
}

fn wait_for_revision(app: &mut App, revision: u64, phase: TransitionPhase) {
    for _ in 0..4_000 {
        app.update();
        if app
            .world()
            .resource::<super::super::hot_reload::HotReload>()
            .revision
            >= revision
            && app.world().resource::<WorldTransition>().phase == phase
        {
            return;
        }
        thread::sleep(Duration::from_millis(1));
    }
    panic!(
        "hot revision {revision} did not reach {phase:?}: {:?}",
        app.world().resource::<WorldTransition>()
    );
}

fn blocked(app: &App) -> bool {
    app.world()
        .resource::<WorldCollision>()
        .occupancy_for("source")
        .unwrap()
        .is_blocked(1, 1)
        .unwrap()
}

fn changed_map() -> String {
    MAP.replace("1,0,\n0,0", "1,1,\n0,0")
        .replace("0,0,\n0,0", "0,0,\n0,1")
}

#[test]
fn watched_same_map_tmx_tsx_and_yaml_publish_new_geometry_collision_actors_and_objects() {
    let fixture = Fixture::new();
    let mut app = hot_app(&fixture);
    assert!(!blocked(&app));
    let original_entities = active_entities(app.world_mut());
    fixture.write("media/maps/source.tmx", changed_map().as_bytes());
    fixture.write(
        "media/sprites/objects/item_box.tsx",
        BOX_TSX
            .replace("tilewidth=\"32\"", "tilewidth=\"16\"")
            .as_bytes(),
    );
    fixture.write("data/maps/source.yaml", b"id: source\nname: Revised\nnpcs:\n  - id: replacement\n    name: Replacement\n    position: [1, 0]\nitem_boxes:\n  - id: replacement_box\n    position: [0, 0]\n");
    wait_for_revision(&mut app, 1, TransitionPhase::Idle);
    assert!(blocked(&app));
    let world = app.world_mut();
    assert_eq!(
        world
            .query_filtered::<Entity, With<StaticMapTile>>()
            .iter(world)
            .count(),
        3 // Two ground tiles and the authored visible collision tile.
    );
    assert_eq!(
        world.query::<&WorldNpc>().single(world).unwrap().name(),
        "Replacement"
    );
    assert_eq!(
        world.query::<&WorldItemBox>().single(world).unwrap().id(),
        "replacement_box"
    );
    let atlas = world
        .resource::<Assets<TsxAtlasAsset>>()
        .iter()
        .find(|(_, atlas)| {
            atlas
                .metadata()
                .image()
                .source()
                .as_str()
                .ends_with("item_box.png")
        })
        .unwrap()
        .1;
    assert_eq!(atlas.frame_rect(0).unwrap().width(), 16.0);
    assert!(
        original_entities
            .iter()
            .all(|&entity| world.get_entity(entity).is_err())
    );
}

#[test]
fn watched_failed_generation_restores_assets_entities_collision_rng_and_retries() {
    let fixture = Fixture::new();
    let mut app = hot_app(&fixture);
    let entities = active_entities(app.world_mut());
    let game = app.world().resource::<GameState>().clone();
    let before = app
        .world()
        .resource::<crate::gameplay_rng::GameplayRng>()
        .clone();
    let image_id = app
        .world()
        .resource::<Assets<TsxAtlasAsset>>()
        .iter()
        .find(|(_, atlas)| {
            atlas
                .metadata()
                .image()
                .source()
                .as_str()
                .ends_with("item_box.png")
        })
        .unwrap()
        .1
        .image()
        .id();
    let original_image = app
        .world()
        .resource::<Assets<Image>>()
        .get(image_id)
        .unwrap()
        .data
        .clone();
    fixture.write("media/maps/source.tmx", changed_map().as_bytes());
    fixture.write(
        "media/sprites/objects/item_box.tsx",
        BOX_TSX
            .replace("tilewidth=\"32\"", "tilewidth=\"16\"")
            .as_bytes(),
    );
    fixture.write("media/sprites/objects/item_box.png", PLAYER_PNG);
    fixture.write(
        "data/maps/source.yaml",
        b"npcs: [this is not valid npc data]\n",
    );
    wait_for_revision(&mut app, 1, TransitionPhase::Failed);
    assert_eq!(active_entities(app.world_mut()), entities);
    assert_eq!(app.world().resource::<GameState>(), &game);
    assert_eq!(
        app.world().resource::<crate::gameplay_rng::GameplayRng>(),
        &before
    );
    assert!(!blocked(&app));
    let atlas = app
        .world()
        .resource::<Assets<TsxAtlasAsset>>()
        .iter()
        .find(|(_, atlas)| {
            atlas
                .metadata()
                .image()
                .source()
                .as_str()
                .ends_with("item_box.png")
        })
        .unwrap()
        .1;
    assert_eq!(atlas.frame_rect(0).unwrap().width(), 32.0);
    assert_eq!(
        app.world()
            .resource::<Assets<Image>>()
            .get(image_id)
            .unwrap()
            .data,
        original_image
    );
    press(&mut app, KeyCode::Escape);
    wait_for(&mut app, TransitionPhase::Idle);
    assert!(!blocked(&app));
    fixture.write("data/maps/source.yaml", b"id: source\nname: Repaired\n");
    wait_for_revision(&mut app, 2, TransitionPhase::Idle);
    // The repaired YAML triggers a transaction, but failed generation's other changed files
    // must also be retried rather than silently keeping their restored cached values.
    assert!(blocked(&app));
}

#[test]
fn watched_removed_dependency_rejects_and_explicit_retry_restores_complete_generation() {
    let fixture = Fixture::new();
    let mut app = hot_app(&fixture);
    let entities = active_entities(app.world_mut());
    fs::remove_file(
        fixture
            .0
            .join("scenarios/fixture/media/sprites/objects/item_box.png"),
    )
    .unwrap();
    wait_for_revision(&mut app, 1, TransitionPhase::Failed);
    assert_eq!(active_entities(app.world_mut()), entities);
    assert!(
        app.world()
            .resource::<WorldTransition>()
            .failure()
            .unwrap()
            .contains("item_box.png")
    );
    fixture.write("media/sprites/objects/item_box.png", BOX_PNG);
    press(&mut app, KeyCode::KeyR);
    wait_for_revision(&mut app, 2, TransitionPhase::Idle);
    assert!(!blocked(&app));
}

#[test]
fn live_scenario_identity_mutation_is_rejected_before_loading_another_package() {
    let fixture = Fixture::new();
    let mut app = hot_app(&fixture);
    let source = app.world().resource::<GameState>().clone();
    let entities = active_entities(app.world_mut());
    app.world_mut()
        .insert_resource(ScenarioRoot::try_for_package_key("another_package").unwrap());
    app.world_mut()
        .insert_resource(ScenarioInventory::default());
    app.update();
    assert_eq!(
        app.world().resource::<ScenarioRoot>().package_key(),
        "fixture"
    );
    assert!(
        app.world()
            .resource::<ScenarioInventory>()
            .tmx_directory
            .is_some()
    );
    assert_eq!(app.world().resource::<GameState>(), &source);
    assert_eq!(active_entities(app.world_mut()), entities);
}

#[test]
fn separately_selected_scenarios_with_same_map_ids_have_distinct_entities_and_assets() {
    let first = Fixture::new();
    let second = Fixture::new();
    second.write("media/maps/source.tmx", changed_map().as_bytes());
    second.write("data/maps/source.yaml", b"id: source\nname: Second scenario\nnpcs:\n  - id: second_witness\n    name: Second Witness\n    position: [1, 0]\n");
    fs::rename(
        second.0.join("scenarios/fixture"),
        second.0.join("scenarios/second"),
    )
    .unwrap();
    let mut first_app = hot_app(&first);
    let mut second_app = second.app_for_package("second");
    second_app.add_plugins(super::super::hot_reload::ScenarioHotReloadPlugin::new(
        second.0.clone(),
        true,
    ));
    wait_for(&mut second_app, TransitionPhase::Idle);
    assert!(!blocked(&first_app));
    assert!(blocked(&second_app));
    for (app, expected, package) in [
        (&mut first_app, "Witness", "fixture"),
        (&mut second_app, "Second Witness", "second"),
    ] {
        let world = app.world_mut();
        assert_eq!(
            world.query::<&WorldNpc>().single(world).unwrap().name(),
            expected
        );
        let active = world.resource::<ActiveManifestLoad>();
        assert_eq!(active.root().package_key(), package);
        let handle = world.resource::<StaticMapRenderState>().asset_ids()[0];
        assert!(
            world
                .resource::<AssetServer>()
                .get_path(handle)
                .unwrap()
                .to_string()
                .starts_with(&format!("scenarios/{package}/"))
        );
    }
}

#[test]
fn watcher_rebuilds_item_catalog_and_rolls_back_late_catalog_validation_failure() {
    use crate::field_menu_domain::{CatalogStatus, FieldMenuDomainPlugin};
    let fixture = Fixture::new();
    let items = include_str!(scenario_file!("data/items/consumables_recovery.yaml"));
    fixture.write("data/items/recovery.yaml", items.as_bytes());
    fixture.write(
        "data/items/field_use.yaml",
        include_bytes!(scenario_file!("data/items/field_use.yaml")),
    );
    fixture.write(
        "data/quests.yaml",
        include_bytes!(scenario_file!("data/quests.yaml")),
    );
    let mut app = fixture.app();
    app.add_plugins(FieldMenuDomainPlugin);
    app.add_plugins(super::super::hot_reload::ScenarioHotReloadPlugin::new(
        fixture.0.clone(),
        true,
    ));
    wait_for(&mut app, TransitionPhase::Idle);
    for _ in 0..2000 {
        app.update();
        if app.world().resource::<FieldMenuCatalog>().status() == CatalogStatus::Ready {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(
        app.world().resource::<FieldMenuCatalog>().status(),
        CatalogStatus::Ready
    );
    fixture.write(
        "data/items/recovery.yaml",
        items
            .replace("name: Potion", "name: Revised Potion")
            .as_bytes(),
    );
    wait_for_revision(&mut app, 1, TransitionPhase::Idle);
    let Some(crate::scenario_item::ItemDefinition::Consumable(potion)) =
        app.world().resource::<FieldMenuCatalog>().item("potion")
    else {
        panic!("missing potion");
    };
    assert_eq!(potion.name, "Revised Potion");
    fixture.write("media/maps/source.tmx", changed_map().as_bytes());
    fixture.write("data/items/recovery.yaml", b"not: an item catalog\n");
    wait_for_revision(&mut app, 2, TransitionPhase::Failed);
    let Some(crate::scenario_item::ItemDefinition::Consumable(potion)) =
        app.world().resource::<FieldMenuCatalog>().item("potion")
    else {
        panic!("missing rolled-back potion");
    };
    assert_eq!(potion.name, "Revised Potion");
    assert!(!blocked(&app));
}

#[test]
fn watcher_rejects_manifest_identity_change_and_does_not_retry_until_another_edit() {
    let fixture = Fixture::new();
    let mut app = hot_app(&fixture);
    let path = fixture.0.join("scenarios/fixture/manifest.yaml");
    let manifest = fs::read_to_string(&path).unwrap();
    fixture.write(
        "manifest.yaml",
        manifest
            .replace("id: my_rpg_story", "id: different_story")
            .as_bytes(),
    );
    wait_for_revision(&mut app, 1, TransitionPhase::Failed);
    assert!(
        app.world()
            .resource::<WorldTransition>()
            .failure()
            .unwrap()
            .contains("identity changed")
    );
    press(&mut app, KeyCode::Escape);
    wait_for(&mut app, TransitionPhase::Idle);
    for _ in 0..120 {
        app.update();
    }
    assert_eq!(
        app.world()
            .resource::<super::super::hot_reload::HotReload>()
            .revision,
        1
    );
    let active = app.world().resource::<ActiveManifestLoad>();
    assert_eq!(
        active
            .manifest(app.world().resource::<Assets<Manifest>>())
            .unwrap()
            .id,
        "my_rpg_story"
    );
    fixture.write(
        "manifest.yaml",
        manifest
            .replace(
                "window_title: \"Rusted Kingdoms\"",
                "window_title: \"Repaired title\"",
            )
            .as_bytes(),
    );
    wait_for_revision(&mut app, 2, TransitionPhase::Idle);
}

#[test]
fn watcher_checks_new_audio_reference_before_publishing_same_map_metadata() {
    let fixture = Fixture::new();
    fixture.write(
        "data/audio/bgm_index.yaml",
        b"world:\n  field: missing.mp3\n",
    );
    let mut app = hot_app(&fixture);
    let entities = active_entities(app.world_mut());
    // The key resolves, but its dependency is missing; this used to enter World and fail later.
    fixture.write(
        "data/maps/source.yaml",
        b"id: source\nname: Sound test\nbgm: world.field\n",
    );
    wait_for_revision(&mut app, 1, TransitionPhase::Failed);
    assert!(
        app.world()
            .resource::<WorldTransition>()
            .failure()
            .unwrap()
            .contains("BGM audio dependency"),
        "{:?}",
        app.world().resource::<WorldTransition>().failure()
    );
    assert_eq!(active_entities(app.world_mut()), entities);
}

#[test]
fn hot_timeout_guards_late_asset_writes_and_refreshes_available_recovery_actions() {
    let fixture = Fixture::new();
    let mut app = hot_app(&fixture);
    let image_id = app
        .world()
        .resource::<Assets<TsxAtlasAsset>>()
        .iter()
        .find(|(_, atlas)| {
            atlas
                .metadata()
                .image()
                .source()
                .as_str()
                .ends_with("item_box.png")
        })
        .unwrap()
        .1
        .image()
        .id();
    let original = app
        .world()
        .resource::<Assets<Image>>()
        .get(image_id)
        .unwrap()
        .clone();
    let source = active_entities(app.world_mut());
    let save = fs::read(fixture.autosave()).unwrap();
    super::super::hot_reload::begin(app.world_mut());
    super::super::hot_reload::hold_completion_for_test(app.world_mut(), image_id.untyped());
    app.world_mut()
        .resource_mut::<WorldTransition>()
        .wait_seconds = LOAD_TIMEOUT_SECONDS;
    drive_transition_loading(app.world_mut());
    app.update();
    assert_eq!(
        app.world().resource::<WorldTransition>().phase,
        TransitionPhase::Failed
    );
    let world = app.world_mut();
    let text = &world
        .query_filtered::<&Text, With<RecoveryText>>()
        .single(world)
        .unwrap()
        .0;
    assert!(text.contains("reads are settling"));
    assert!(!text.contains("R: retry"));
    assert!(text.contains("T: return to title"));
    press(&mut app, KeyCode::Escape);
    assert_eq!(
        app.world().resource::<WorldTransition>().phase,
        TransitionPhase::Failed
    );
    let mut late = original.clone();
    late.data.as_mut().unwrap()[0] ^= 255;
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .insert(image_id, late)
        .unwrap();
    app.update();
    assert_eq!(
        app.world()
            .resource::<Assets<Image>>()
            .get(image_id)
            .unwrap()
            .data,
        original.data
    );
    app.update();
    let world = app.world_mut();
    let text = &world
        .query_filtered::<&Text, With<RecoveryText>>()
        .single(world)
        .unwrap()
        .0;
    assert!(!text.contains("reads are settling"));
    assert!(text.contains("R: retry"));
    assert_eq!(fs::read(fixture.autosave()).unwrap(), save);
    press(&mut app, KeyCode::Escape);
    wait_for(&mut app, TransitionPhase::Idle);
    assert_eq!(active_entities(app.world_mut()), source);
}

#[test]
fn watched_audio_bytes_restart_the_logical_loop_after_publication() {
    use crate::world_audio::LogicalBgmPlayer;
    let fixture = Fixture::new();
    let original = include_bytes!(scenario_file!("media/audio/sfx/ui_menu/001_Hover_01.mp3"));
    let replacement = include_bytes!(scenario_file!("media/audio/sfx/ui_menu/013_Confirm_03.mp3"));
    fixture.write("data/audio/bgm_index.yaml", b"world:\n  field: loop.mp3\n");
    fixture.write("media/audio/loop.mp3", original);
    fixture.write(
        "data/maps/source.yaml",
        b"id: source\nname: Audio fixture\nbgm: world.field\n",
    );
    let mut app = hot_app(&fixture);
    let world = app.world_mut();
    let player = world
        .query_filtered::<Entity, With<LogicalBgmPlayer>>()
        .single(world)
        .unwrap();
    fixture.write("media/audio/loop.mp3", replacement);
    wait_for_revision(&mut app, 1, TransitionPhase::Idle);
    let world = app.world_mut();
    let new_player = world
        .query_filtered::<Entity, With<LogicalBgmPlayer>>()
        .single(world)
        .unwrap();
    assert_ne!(new_player, player);
    let audio = world.get::<AudioPlayer>(new_player).unwrap();
    assert_eq!(
        world
            .resource::<Assets<AudioSource>>()
            .get(&audio.0)
            .unwrap()
            .bytes
            .as_ref(),
        replacement.as_slice()
    );
}

#[test]
fn disabled_watching_leaves_same_map_derived_state_unchanged() {
    let fixture = Fixture::new();
    let mut app = fixture.app();
    app.add_plugins(super::super::hot_reload::ScenarioHotReloadPlugin::new(
        fixture.0.clone(),
        false,
    ));
    wait_for(&mut app, TransitionPhase::Idle);
    let entities = active_entities(app.world_mut());
    fixture.write("media/maps/source.tmx", changed_map().as_bytes());
    for _ in 0..120 {
        app.update();
    }
    assert_eq!(
        app.world()
            .resource::<super::super::hot_reload::HotReload>()
            .revision,
        0
    );
    assert!(!blocked(&app));
    assert_eq!(active_entities(app.world_mut()), entities);
}
