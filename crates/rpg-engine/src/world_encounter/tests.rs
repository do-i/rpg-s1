use std::{thread, time::Duration};

use bevy::{
    asset::AssetApp,
    image::{CompressedImageFormats, ImageLoader, ImagePlugin},
    time::TimeUpdateStrategy,
};

use super::{state::*, *};

use bevy::{asset::Assets, prelude::*};

use crate::{
    app_state::AppState,
    encounter::{
        BattleEntry, BattleSide, EnemyCatalog, PreBattleReturnContext, WorldEnemyReturnState,
        build_battle_entry,
    },
    field_menu_domain::{CatalogStatus, FieldMenuCatalog},
    game_state::GameState,
    scenario_audio::SfxIndex,
    scenario_balance::BalanceData,
    scenario_encounter::EncounterZone,
    scenario_root::ScenarioRoot,
    scenario_spatial::{CardinalDirection, Position},
};
use crate::{
    encounter::BuildBattleError,
    encounter_assets::EncounterAssetPlugin,
    field_menu_domain::{FieldMenuDomainPlugin, derived_stats},
    name_entry::NameEntryConfirmed,
    runtime_map::RuntimeMapId,
    scenario_new_game_assets::{ActiveNewGameInputs, ActiveNewGameInputsStatus},
    scenario_yaml_asset::ScenarioYamlLoader,
    test_support::headless_title_app_with_asset_base,
    tmx_ground_asset::TmxGroundAssetPlugin,
    tsx_atlas_asset::TsxAtlasAssetPlugin,
    world_actor::{WorldActorPlugin, WorldActorState},
    world_audio::WorldAudioPlugin,
    world_object::{WorldObjectPlugin, WorldObjectState},
};

#[test]
fn the_enemy_just_defeated_is_restored_hidden_from_its_first_frame() {
    let mut app = headless_title_app_with_asset_base(
        AppState::NameEntry,
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets").to_owned(),
        ScenarioRoot::default(),
    );
    app.add_plugins(ImagePlugin::default_nearest())
        .register_asset_loader(ImageLoader::new(CompressedImageFormats::empty()))
        .add_plugins(TsxAtlasAssetPlugin)
        .add_plugins(TmxGroundAssetPlugin)
        .add_plugins(EncounterAssetPlugin)
        .add_plugins(FieldMenuDomainPlugin)
        .add_plugins(WorldAudioPlugin)
        .init_asset::<SfxIndex>()
        .register_asset_loader(ScenarioYamlLoader::<SfxIndex>::new("SFX index"))
        .add_plugins(WorldEncounterPlugin)
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            100,
        )));
    for _ in 0..5_000 {
        app.update();
        if app.world().resource::<ActiveNewGameInputs>().status()
            == ActiveNewGameInputsStatus::Ready
        {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
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
    app.world_mut()
        .resource_mut::<GameState>()
        .map_mut()
        .move_to(
            RuntimeMapId::try_new("zone_01_starting_forest").unwrap(),
            Position::new(9, 8),
            CardinalDirection::Down,
        );
    let returning = |id: &str, position: Position, active: bool| WorldEnemyReturnState {
        encounter_id: id.to_owned(),
        formation: vec!["goblin".to_owned()],
        origin: position,
        position,
        facing: CardinalDirection::Down,
        boss: false,
        chase_range: 0,
        active,
    };
    app.insert_resource(WorldEncounterRestore {
        map_id: "zone_01_starting_forest".to_owned(),
        enemies: vec![
            returning("defeated", Position::new(14, 10), false),
            returning("waiting", Position::new(4, 12), true),
        ],
    });
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::World);

    let mut checked = false;
    for _ in 0..5_000 {
        app.update();
        let world = app.world_mut();
        let spawned = world
            .query::<(&WorldEnemy, &Transform)>()
            .iter(world)
            .map(|(enemy, transform)| (enemy.encounter_id.clone(), transform.scale))
            .collect::<Vec<_>>();
        if !spawned.is_empty() {
            // The very first frame the enemies exist is the one that used to flash.
            assert!(
                spawned.contains(&("defeated".to_owned(), Vec3::ZERO)),
                "{spawned:?}"
            );
            assert!(
                spawned.contains(&("waiting".to_owned(), Vec3::ONE)),
                "{spawned:?}"
            );
            checked = true;
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert!(checked, "the restored enemies never spawned");
}

#[test]
fn production_first_zone_spawns_and_contact_builds_one_complete_battle_handoff() {
    let mut app = headless_title_app_with_asset_base(
        AppState::NameEntry,
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets").to_owned(),
        ScenarioRoot::default(),
    );
    app.add_plugins(ImagePlugin::default_nearest())
        .register_asset_loader(ImageLoader::new(CompressedImageFormats::empty()))
        .add_plugins(TsxAtlasAssetPlugin)
        .add_plugins(TmxGroundAssetPlugin)
        .add_plugins(EncounterAssetPlugin)
        .add_plugins(FieldMenuDomainPlugin)
        .add_plugins(WorldAudioPlugin)
        .init_asset::<SfxIndex>()
        .register_asset_loader(ScenarioYamlLoader::<SfxIndex>::new("SFX index"))
        .add_plugins(WorldEncounterPlugin)
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            100,
        )));

    for _ in 0..5_000 {
        app.update();
        if app.world().resource::<ActiveNewGameInputs>().status()
            == ActiveNewGameInputsStatus::Ready
        {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(
        app.world().resource::<ActiveNewGameInputs>().status(),
        ActiveNewGameInputsStatus::Ready
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
    let mut game = app.world_mut().resource_mut::<GameState>();
    game.map_mut().move_to(
        RuntimeMapId::try_new("zone_01_starting_forest").unwrap(),
        Position::new(9, 8),
        CardinalDirection::Down,
    );
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::World);

    for _ in 0..5_000 {
        app.update();
        if app.world().resource::<WorldEncounterState>().status() == WorldEncounterStatus::Spawned
            && app.world().resource::<FieldMenuCatalog>().status() == CatalogStatus::Ready
        {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(
        app.world().resource::<WorldEncounterState>().status(),
        WorldEncounterStatus::Spawned,
        "{:?}",
        app.world().resource::<WorldEncounterState>().failure()
    );
    // 107 before roadmap B1.2 added the Cinder Marshal.
    assert_eq!(app.world().resource::<EnemyCatalog>().len(), 108);
    let expected_party_stats = {
        let world = app.world();
        let member = world
            .resource::<GameState>()
            .party()
            .members()
            .next()
            .unwrap();
        derived_stats(member, world.resource::<FieldMenuCatalog>())
    };
    // B3.1: battle DEF is no longer the CON total. It is equipment CON plus a divided
    // innate share, so armor -- not the level curve -- drives survivability.
    let expected_party_defense = {
        let world = app.world();
        let member = world
            .resource::<GameState>()
            .party()
            .members()
            .next()
            .unwrap();
        crate::field_menu_domain::battle_defense(
            member,
            world.resource::<FieldMenuCatalog>(),
            BalanceData::default()
                .battle
                .party_defense_con_divisor
                .get(),
        )
    };
    let mut query = app.world_mut().query::<&WorldEnemy>();
    let enemies = query.iter(app.world()).cloned().collect::<Vec<_>>();
    assert_eq!(enemies.len(), 11);
    let boss = enemies.iter().find(|enemy| enemy.is_boss()).unwrap();
    assert_eq!(boss.encounter_id(), "zone_01_starting_forest:boss");
    assert_eq!(boss.formation(), ["grik_the_grin"]);
    assert_eq!(boss.tile_position(), Position::new(0, 25));
    assert!(enemies.iter().all(|enemy| !enemy.formation().is_empty()));

    let boss_position = boss.tile_position();
    let boss_encounter_id = boss.encounter_id().to_owned();
    app.world_mut()
        .resource_mut::<GameState>()
        .map_mut()
        .move_to(
            RuntimeMapId::try_new("town_01_ardel").unwrap(),
            Position::new(14, 5),
            CardinalDirection::Up,
        );
    for _ in 0..5_000 {
        app.update();
        let enemy_count = {
            let world = app.world_mut();
            world.query::<&WorldEnemy>().iter(world).count()
        };
        if app.world().resource::<WorldEncounterState>().status()
            == WorldEncounterStatus::NoEncounters
            && enemy_count == 0
        {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(
        app.world().resource::<WorldEncounterState>().status(),
        WorldEncounterStatus::NoEncounters
    );
    let mut query = app.world_mut().query::<&WorldEnemy>();
    assert_eq!(query.iter(app.world()).count(), 0);

    app.world_mut()
        .resource_mut::<GameState>()
        .map_mut()
        .move_to(
            RuntimeMapId::try_new("zone_01_starting_forest").unwrap(),
            Position::new(9, 8),
            CardinalDirection::Down,
        );
    for _ in 0..5_000 {
        app.update();
        if app.world().resource::<WorldEncounterState>().status() == WorldEncounterStatus::Spawned {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    let mut query = app.world_mut().query::<&WorldEnemy>();
    assert_eq!(query.iter(app.world()).count(), 11);

    app.world_mut()
        .resource_mut::<GameState>()
        .map_mut()
        .set_position(boss_position);
    for _ in 0..10 {
        app.update();
        if app.world().resource::<State<AppState>>().get() == &AppState::Battle {
            break;
        }
    }
    assert_eq!(
        app.world().resource::<State<AppState>>().get(),
        &AppState::Battle
    );
    let entry = app.world().resource::<BattleEntry>();
    let party = entry
        .participants
        .iter()
        .find(|participant| participant.side == crate::encounter::BattleSide::Party)
        .unwrap();
    assert_eq!(party.attack, i64::from(expected_party_stats.strength));
    assert_eq!(party.defense, i64::from(expected_party_defense));
    assert_eq!(
        party.magic_resistance,
        i64::from(expected_party_stats.intelligence)
    );
    assert_eq!(party.dexterity, i64::from(expected_party_stats.dexterity));
    assert_eq!(entry.encounter_id, boss_encounter_id);
    assert_eq!(entry.background_id, "zone1-bg-1280x468");
    assert_eq!(entry.bgm_key, "battle.boss");
    assert_eq!(
        entry.boss_completion_flag.as_deref(),
        Some("boss_zone01_defeated")
    );
    assert_eq!(entry.return_context.map_id, "zone_01_starting_forest");
    assert_eq!(entry.return_context.position, boss_position);
    assert_eq!(
        entry.return_context.world_bgm_key.as_deref(),
        Some("zone.starting_forest")
    );
}

/// Loads Open Plains and both caves through the encounter, actor and object plugins.
/// Cave markers must produce enemies and lootable chests, and changing maps must replace
/// the previous map's entities without leaving any publication participant loading.
#[test]
fn production_open_plains_and_caves_spawn_encounters_and_treasure() {
    let mut app = headless_title_app_with_asset_base(
        AppState::NameEntry,
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets").to_owned(),
        ScenarioRoot::default(),
    );
    app.add_plugins(ImagePlugin::default_nearest())
        .register_asset_loader(ImageLoader::new(CompressedImageFormats::empty()))
        .add_plugins(TsxAtlasAssetPlugin)
        .add_plugins(TmxGroundAssetPlugin)
        .add_plugins(EncounterAssetPlugin)
        .add_plugins(FieldMenuDomainPlugin)
        .add_plugins(WorldAudioPlugin)
        .init_asset::<SfxIndex>()
        .register_asset_loader(ScenarioYamlLoader::<SfxIndex>::new("SFX index"))
        .add_plugins(WorldEncounterPlugin)
        .add_plugins(WorldActorPlugin)
        .add_plugins(WorldObjectPlugin)
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            100,
        )));

    for _ in 0..5_000 {
        app.update();
        if app.world().resource::<ActiveNewGameInputs>().status()
            == ActiveNewGameInputsStatus::Ready
        {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(
        app.world().resource::<ActiveNewGameInputs>().status(),
        ActiveNewGameInputsStatus::Ready
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

    app.world_mut()
        .resource_mut::<GameState>()
        .map_mut()
        .move_to(
            RuntimeMapId::try_new("zone_02_open_plains").unwrap(),
            Position::new(2, 2),
            CardinalDirection::Down,
        );
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::World);
    for _ in 0..5_000 {
        app.update();
        if app.world().resource::<WorldEncounterState>().status() == WorldEncounterStatus::Spawned
            && app
                .world()
                .resource::<WorldActorState>()
                .is_spawned_for("zone_02_open_plains")
            && app
                .world()
                .resource::<WorldObjectState>()
                .is_spawned_for("zone_02_open_plains")
        {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(
        app.world().resource::<WorldEncounterState>().status(),
        WorldEncounterStatus::Spawned,
        "{:?}",
        app.world().resource::<WorldEncounterState>().failure()
    );
    let mut query = app.world_mut().query::<&WorldEnemy>();
    let enemies = query.iter(app.world()).cloned().collect::<Vec<_>>();
    assert_eq!(enemies.len(), 7, "6 spawn_tile entries plus the boss");
    assert!(enemies.iter().all(|enemy| !enemy.formation().is_empty()));
    let boss = enemies.iter().find(|enemy| enemy.is_boss()).unwrap();
    assert_eq!(boss.encounter_id(), "zone_02_open_plains:boss");
    assert_eq!(boss.formation(), ["wolf_beast_black_fur"]);
    assert_eq!(boss.tile_position(), Position::new(3, 18));

    for (map_id, entry, expected_enemies, expected_boxes) in [
        ("zone_02_open_plains_cave_01", Position::new(1, 3), 6, 4),
        ("zone_02_open_plains_cave_02", Position::new(8, 27), 9, 3),
        ("zone_02_open_plains_cave_01", Position::new(1, 3), 6, 4),
    ] {
        app.world_mut()
            .resource_mut::<GameState>()
            .map_mut()
            .move_to(
                RuntimeMapId::try_new(map_id).unwrap(),
                entry,
                CardinalDirection::Down,
            );
        for _ in 0..5_000 {
            app.update();
            if app.world().resource::<WorldEncounterState>().status()
                == WorldEncounterStatus::Spawned
                && app
                    .world()
                    .resource::<WorldActorState>()
                    .is_spawned_for(map_id)
                && app
                    .world()
                    .resource::<WorldObjectState>()
                    .is_spawned_for(map_id)
            {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(
            app.world().resource::<WorldEncounterState>().status(),
            WorldEncounterStatus::Spawned,
            "{map_id}: {:?}",
            app.world().resource::<WorldEncounterState>().failure()
        );
        assert!(
            app.world()
                .resource::<WorldActorState>()
                .is_spawned_for(map_id)
        );
        assert!(
            app.world()
                .resource::<WorldObjectState>()
                .is_spawned_for(map_id)
        );
        let mut enemies = app.world_mut().query::<&WorldEnemy>();
        assert_eq!(
            enemies.iter(app.world()).count(),
            expected_enemies,
            "{map_id}"
        );
        assert!(enemies.iter(app.world()).all(|enemy| {
            enemy.encounter_id().starts_with(map_id)
                && !enemy.is_boss()
                && !enemy.formation().is_empty()
        }));
        let mut boxes = app
            .world_mut()
            .query::<&crate::world_object::WorldItemBox>();
        assert_eq!(boxes.iter(app.world()).count(), expected_boxes, "{map_id}");
        assert!(
            boxes
                .iter(app.world())
                .all(|item_box| !item_box.loot().items.is_empty())
        );
        if map_id == "zone_02_open_plains_cave_01" {
            let key = crate::runtime_opened_boxes::OpenedBoxKey::try_new(
                RuntimeMapId::try_new(map_id).unwrap(),
                "plains_cave_01_chest_01",
            )
            .unwrap();
            let already_open = app
                .world()
                .resource::<GameState>()
                .opened_boxes()
                .contains(&key);
            let mut query =
                app.world_mut()
                    .query::<(&crate::world_object::WorldItemBox, &Sprite, &Transform)>();
            let rows = query.iter(app.world()).collect::<Vec<_>>();
            let one = rows
                .iter()
                .find(|(b, _, _)| b.id() == "plains_cave_01_chest_01")
                .unwrap();
            let shared = rows
                .iter()
                .find(|(b, _, _)| b.id() == "plains_cave_01_chest_03")
                .unwrap();
            assert_eq!(one.0.loot(), shared.0.loot());
            assert_ne!(one.0.key(), shared.0.key());
            assert_eq!(
                one.1.texture_atlas.as_ref().unwrap().index,
                usize::from(already_open)
            );
            assert_eq!(shared.1.texture_atlas.as_ref().unwrap().index, 0);
            let moved = rows
                .iter()
                .find(|(b, _, _)| b.id() == "plains_cave_01_chest_02")
                .unwrap();
            assert_eq!(moved.0.tile_position(), Position::new(12, 19));
            assert!((moved.2.translation.x - 401.5).abs() < 0.01);
            assert!((moved.2.translation.y + 624.5).abs() < 0.01);
            app.world_mut()
                .resource_mut::<GameState>()
                .opened_boxes_mut()
                .record(key);
            app.update();
            let mut query = app
                .world_mut()
                .query::<(&crate::world_object::WorldItemBox, &Sprite)>();
            for (chest, sprite) in query.iter(app.world()) {
                assert_eq!(
                    sprite.texture_atlas.as_ref().unwrap().index,
                    usize::from(chest.id() == "plains_cave_01_chest_01")
                );
            }
        }
    }
}

/// Covers W12.3's `zone_03_marshland` encounters end to end: the 18 authored `spawn_tile`
/// entries plus the boss spawn from `data/encount/zone_03_marshland.yaml` and the TMX
/// `spawn_tile`/`boss_enemy` layers, a full respawn round-trip through a neighboring town with
/// no encounter zone at all (`town_03_harborgate`, which has no `data/encount/*.yaml`), and
/// the boss contact battle handoff. Marshland is also the first map in wave order with no
/// `bgm:` field at all (`data/maps/zone_03_marshland.yaml` is two lines: `name` and
/// `warp_order`, byte-identical to the pinned source) — `return_context.world_bgm_key` must
/// come back `None`, matching `world_encounter::build_battle_entry`'s direct
/// `metadata.bgm.clone()` and `world_audio`'s "no authored bgm leaves the previous track
/// playing" behavior (see `world_audio::tests`), not a `zone.marshland` key that exists in
/// neither the map metadata nor the BGM index.
#[test]
fn production_marshland_spawns_and_contact_builds_one_complete_boss_battle_handoff() {
    let mut app = headless_title_app_with_asset_base(
        AppState::NameEntry,
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets").to_owned(),
        ScenarioRoot::default(),
    );
    app.add_plugins(ImagePlugin::default_nearest())
        .register_asset_loader(ImageLoader::new(CompressedImageFormats::empty()))
        .add_plugins(TsxAtlasAssetPlugin)
        .add_plugins(TmxGroundAssetPlugin)
        .add_plugins(EncounterAssetPlugin)
        .add_plugins(FieldMenuDomainPlugin)
        .add_plugins(WorldAudioPlugin)
        .init_asset::<SfxIndex>()
        .register_asset_loader(ScenarioYamlLoader::<SfxIndex>::new("SFX index"))
        .add_plugins(WorldEncounterPlugin)
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            100,
        )));

    for _ in 0..5_000 {
        app.update();
        if app.world().resource::<ActiveNewGameInputs>().status()
            == ActiveNewGameInputsStatus::Ready
        {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(
        app.world().resource::<ActiveNewGameInputs>().status(),
        ActiveNewGameInputsStatus::Ready
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
    let mut game = app.world_mut().resource_mut::<GameState>();
    game.map_mut().move_to(
        RuntimeMapId::try_new("zone_03_marshland").unwrap(),
        // The production arrival tile for the Open Plains -> Marshland portal
        // (`world_transition::tests::marshland_portals_...`).
        Position::new(27, 1),
        CardinalDirection::Down,
    );
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::World);

    for _ in 0..5_000 {
        app.update();
        if app.world().resource::<WorldEncounterState>().status() == WorldEncounterStatus::Spawned
            && app.world().resource::<FieldMenuCatalog>().status() == CatalogStatus::Ready
        {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(
        app.world().resource::<WorldEncounterState>().status(),
        WorldEncounterStatus::Spawned,
        "{:?}",
        app.world().resource::<WorldEncounterState>().failure()
    );
    let expected_party_stats = {
        let world = app.world();
        let member = world
            .resource::<GameState>()
            .party()
            .members()
            .next()
            .unwrap();
        derived_stats(member, world.resource::<FieldMenuCatalog>())
    };
    // B3.1: battle DEF is no longer the CON total. It is equipment CON plus a divided
    // innate share, so armor -- not the level curve -- drives survivability.
    let expected_party_defense = {
        let world = app.world();
        let member = world
            .resource::<GameState>()
            .party()
            .members()
            .next()
            .unwrap();
        crate::field_menu_domain::battle_defense(
            member,
            world.resource::<FieldMenuCatalog>(),
            BalanceData::default()
                .battle
                .party_defense_con_divisor
                .get(),
        )
    };
    let mut query = app.world_mut().query::<&WorldEnemy>();
    let enemies = query.iter(app.world()).cloned().collect::<Vec<_>>();
    assert_eq!(enemies.len(), 19, "18 spawn_tile entries plus the boss");
    let boss = enemies.iter().find(|enemy| enemy.is_boss()).unwrap();
    assert_eq!(boss.encounter_id(), "zone_03_marshland:boss");
    assert_eq!(boss.formation(), ["ratkin_plague_doctor_black_mask_doctor"]);
    assert_eq!(boss.tile_position(), Position::new(0, 31));
    assert!(enemies.iter().all(|enemy| !enemy.formation().is_empty()));

    let boss_position = boss.tile_position();
    let boss_encounter_id = boss.encounter_id().to_owned();

    // Walk into a neighboring map with no `data/encount/*.yaml` at all
    // (`town_03_harborgate`) and confirm every enemy despawns and the zone goes to
    // `NoEncounters`, exactly like the W12.1/W12.2 town fixtures.
    app.world_mut()
        .resource_mut::<GameState>()
        .map_mut()
        .move_to(
            RuntimeMapId::try_new("town_03_harborgate").unwrap(),
            Position::new(21, 37),
            CardinalDirection::Up,
        );
    for _ in 0..5_000 {
        app.update();
        let enemy_count = {
            let world = app.world_mut();
            world.query::<&WorldEnemy>().iter(world).count()
        };
        if app.world().resource::<WorldEncounterState>().status()
            == WorldEncounterStatus::NoEncounters
            && enemy_count == 0
        {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(
        app.world().resource::<WorldEncounterState>().status(),
        WorldEncounterStatus::NoEncounters
    );
    let mut query = app.world_mut().query::<&WorldEnemy>();
    assert_eq!(query.iter(app.world()).count(), 0);

    // Return to Marshland: the full 19-enemy spawn set respawns.
    app.world_mut()
        .resource_mut::<GameState>()
        .map_mut()
        .move_to(
            RuntimeMapId::try_new("zone_03_marshland").unwrap(),
            Position::new(21, 37),
            CardinalDirection::Down,
        );
    for _ in 0..5_000 {
        app.update();
        if app.world().resource::<WorldEncounterState>().status() == WorldEncounterStatus::Spawned {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    let mut query = app.world_mut().query::<&WorldEnemy>();
    assert_eq!(query.iter(app.world()).count(), 19);

    // Walk onto the boss tile and confirm the full battle handoff.
    app.world_mut()
        .resource_mut::<GameState>()
        .map_mut()
        .set_position(boss_position);
    for _ in 0..10 {
        app.update();
        if app.world().resource::<State<AppState>>().get() == &AppState::Battle {
            break;
        }
    }
    assert_eq!(
        app.world().resource::<State<AppState>>().get(),
        &AppState::Battle
    );
    let entry = app.world().resource::<BattleEntry>();
    let party = entry
        .participants
        .iter()
        .find(|participant| participant.side == crate::encounter::BattleSide::Party)
        .unwrap();
    assert_eq!(party.attack, i64::from(expected_party_stats.strength));
    assert_eq!(party.defense, i64::from(expected_party_defense));
    assert_eq!(
        party.magic_resistance,
        i64::from(expected_party_stats.intelligence)
    );
    assert_eq!(party.dexterity, i64::from(expected_party_stats.dexterity));
    assert_eq!(entry.encounter_id, boss_encounter_id);
    assert_eq!(entry.background_id, "zone3-bg-1280x468");
    assert_eq!(entry.bgm_key, "battle.boss");
    assert_eq!(
        entry.boss_completion_flag.as_deref(),
        Some("boss_zone03_defeated")
    );
    assert_eq!(entry.return_context.map_id, "zone_03_marshland");
    assert_eq!(entry.return_context.position, boss_position);
    // This was `None` until roadmap B1.3: the marshland declared no BGM, so returning from a
    // battle restored nothing and the map went on playing whichever town track the player
    // had walked in from.
    assert_eq!(
        entry.return_context.world_bgm_key.as_deref(),
        Some("zone.marsh")
    );
}

/// Covers W12.4's three Ancient Ruins maps through the production loaders. The map changes
/// prove each TMX and optional metadata document publishes into the actor/object/encounter
/// systems, while the sanctum checks the first production barrier rule and the complete
/// Skeleton Knight boss handoff.
#[test]
fn production_ancient_ruins_maps_spawn_barriers_and_build_the_sanctum_boss_handoff() {
    let mut app = headless_title_app_with_asset_base(
        AppState::NameEntry,
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets").to_owned(),
        ScenarioRoot::default(),
    );
    app.add_plugins(ImagePlugin::default_nearest())
        .register_asset_loader(ImageLoader::new(CompressedImageFormats::empty()))
        .add_plugins(TsxAtlasAssetPlugin)
        .add_plugins(TmxGroundAssetPlugin)
        .add_plugins(EncounterAssetPlugin)
        .add_plugins(FieldMenuDomainPlugin)
        .add_plugins(WorldAudioPlugin)
        .init_asset::<SfxIndex>()
        .register_asset_loader(ScenarioYamlLoader::<SfxIndex>::new("SFX index"))
        .add_plugins(WorldEncounterPlugin)
        .add_plugins(WorldActorPlugin)
        .add_plugins(WorldObjectPlugin)
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            100,
        )));

    for _ in 0..5_000 {
        app.update();
        if app.world().resource::<ActiveNewGameInputs>().status()
            == ActiveNewGameInputsStatus::Ready
        {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(
        app.world().resource::<ActiveNewGameInputs>().status(),
        ActiveNewGameInputsStatus::Ready
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

    let maps = [
        ("zone_04_ancient_ruins_01_gate", Position::new(1, 31), 7),
        (
            "zone_04_ancient_ruins_02_courtyard",
            Position::new(1, 12),
            13,
        ),
        ("zone_04_ancient_ruins_03_sanctum", Position::new(1, 16), 9),
    ];
    for (index, (map_id, position, expected_enemies)) in maps.into_iter().enumerate() {
        app.world_mut()
            .resource_mut::<GameState>()
            .map_mut()
            .move_to(
                RuntimeMapId::try_new(map_id).unwrap(),
                position,
                CardinalDirection::Down,
            );
        if index == 0 {
            app.world_mut()
                .resource_mut::<NextState<AppState>>()
                .set(AppState::World);
        }

        for _ in 0..5_000 {
            app.update();
            if app.world().resource::<WorldEncounterState>().status()
                == WorldEncounterStatus::Spawned
                && app
                    .world()
                    .resource::<WorldActorState>()
                    .is_spawned_for(map_id)
                && app
                    .world()
                    .resource::<WorldObjectState>()
                    .is_spawned_for(map_id)
                && app.world().resource::<FieldMenuCatalog>().status() == CatalogStatus::Ready
            {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(
            app.world().resource::<WorldEncounterState>().status(),
            WorldEncounterStatus::Spawned,
            "{map_id}: {:?}",
            app.world().resource::<WorldEncounterState>().failure()
        );
        assert!(
            app.world()
                .resource::<WorldActorState>()
                .is_spawned_for(map_id)
        );
        assert!(
            app.world()
                .resource::<WorldObjectState>()
                .is_spawned_for(map_id)
        );
        let mut query = app.world_mut().query::<&WorldEnemy>();
        let enemies = query.iter(app.world()).cloned().collect::<Vec<_>>();
        assert_eq!(enemies.len(), expected_enemies, "{map_id}");
        assert!(enemies.iter().all(|enemy| !enemy.formation().is_empty()));
    }

    let boss = {
        let mut query = app.world_mut().query::<&WorldEnemy>();
        query
            .iter(app.world())
            .find(|enemy| enemy.is_boss())
            .cloned()
            .expect("sanctum publishes its authored boss")
    };
    assert_eq!(boss.encounter_id(), "zone_04_ancient_ruins_03_sanctum:boss");
    assert_eq!(boss.formation(), ["skeleton_knight_base"]);
    assert_eq!(boss.tile_position(), Position::new(19, 14));

    let barrier_context = PreBattleReturnContext {
        map_id: "zone_04_ancient_ruins_03_sanctum".to_owned(),
        position: Position::new(1, 16),
        facing: CardinalDirection::Down,
        world_bgm_key: None,
        world_enemies: Vec::new(),
    };
    let blocked = {
        let world = app.world();
        let encounter_state = world.resource::<WorldEncounterState>();
        let zones = world.resource::<Assets<EncounterZone>>();
        let zone = zones
            .get(encounter_state.zone.as_ref().unwrap())
            .expect("active sanctum encounter zone is loaded");
        let game = world.resource::<GameState>();
        build_battle_entry(
            "sanctum:barrier",
            &["bat_demon_red_wing_fiend".to_owned()],
            zone,
            world.resource::<EnemyCatalog>(),
            world.resource::<FieldMenuCatalog>(),
            game.party(),
            game.repository(),
            game.flags(),
            &BalanceData::default().battle,
            false,
            barrier_context.clone(),
        )
    };
    assert_eq!(blocked, Err(BuildBattleError::EmptyFormation));

    assert_eq!(
        app.world_mut()
            .resource_mut::<GameState>()
            .repository_mut()
            .add_item("veil_breaker", 1)
            .unwrap()
            .added(),
        1
    );
    let unblocked = {
        let world = app.world();
        let encounter_state = world.resource::<WorldEncounterState>();
        let zones = world.resource::<Assets<EncounterZone>>();
        let zone = zones
            .get(encounter_state.zone.as_ref().unwrap())
            .expect("active sanctum encounter zone is loaded");
        let game = world.resource::<GameState>();
        build_battle_entry(
            "sanctum:barrier",
            &["bat_demon_red_wing_fiend".to_owned()],
            zone,
            world.resource::<EnemyCatalog>(),
            world.resource::<FieldMenuCatalog>(),
            game.party(),
            game.repository(),
            game.flags(),
            &BalanceData::default().battle,
            false,
            barrier_context,
        )
        .unwrap()
    };
    assert!(unblocked.barrier_messages.is_empty());
    assert_eq!(
        unblocked
            .participants
            .iter()
            .filter(|participant| participant.side == BattleSide::Enemy)
            .map(|participant| participant.id.as_str())
            .collect::<Vec<_>>(),
        ["bat_demon_red_wing_fiend"]
    );

    let boss_position = boss.tile_position();
    app.world_mut()
        .resource_mut::<GameState>()
        .map_mut()
        .set_position(boss_position);
    for _ in 0..10 {
        app.update();
        if app.world().resource::<State<AppState>>().get() == &AppState::Battle {
            break;
        }
    }
    assert_eq!(
        app.world().resource::<State<AppState>>().get(),
        &AppState::Battle
    );
    let entry = app.world().resource::<BattleEntry>();
    assert_eq!(entry.encounter_id, boss.encounter_id());
    assert_eq!(entry.background_id, "zone4-sanctum-bg-1280x468");
    assert_eq!(entry.bgm_key, "battle.boss");
    assert_eq!(
        entry.boss_completion_flag.as_deref(),
        Some("boss_zone04_defeated")
    );
    assert_eq!(
        entry.return_context.map_id,
        "zone_04_ancient_ruins_03_sanctum"
    );
    assert_eq!(entry.return_context.position, boss_position);
    // The whole Ancient Ruins arc declared no BGM until roadmap B1.3, so all three of its
    // maps returned from battle with nothing to restore.
    assert_eq!(
        entry.return_context.world_bgm_key.as_deref(),
        Some("zone.ruins")
    );
}
