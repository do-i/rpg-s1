//! Loads a map's encounter assets and publishes its World enemies.

use bevy::{
    asset::{AssetServer, Assets, LoadState},
    ecs::system::SystemParam,
    prelude::*,
};

use crate::{
    encounter::{EnemyCatalog, SpawnCadence, pick_weighted_formation},
    game_state::GameState,
    scenario_audio::{SFX_INDEX_PATH, SfxIndex},
    scenario_battle_background::BattleBackgroundCatalog,
    scenario_encounter::EncounterZone,
    scenario_enemy::{BossMoveSet, EnemyCatalogFile},
    scenario_inventory::ScenarioInventory,
    scenario_map::{
        MapMetadata, OptionalMapMetadata, optional_map_metadata, optional_scenario_asset_is_missing,
    },
    scenario_path::ScenarioRelativePath,
    scenario_root::ScenarioRoot,
    scenario_spatial::{CardinalDirection, Position},
    tile_coordinates::TILE_SIZE,
    tmx_ground_asset::{StaticMapRenderState, TmxGroundAsset},
    world_publication::WorldLayerState,
};

use super::{
    simulation::{
        enemy_standee_translation, enemy_top_left, enemy_world_translation, random_pause,
    },
    state::{
        EnemyStandee, PendingWorldEnemy, WorldEncounterRestore, WorldEncounterState,
        WorldEncounterStatus, WorldEnemy, fail_encounter,
    },
};

pub(super) const STANDEE_SHADOW_WIDTH: f32 = 58.0;
const STANDEE_SHADOW_FEET_Y: f32 = 3.0;

const DEFAULT_RESPAWN_SECONDS: f32 = 30.0;

/// The encounter-load failure for the first pending spawn whose formation names no enemy.
///
/// An authored `formation: []` parses and stays drawable, so it is refused here, after the
/// formations are picked and before any sprite is requested, without changing the draws.
fn empty_formation_failure(pending: &[PendingWorldEnemy]) -> Option<String> {
    pending
        .iter()
        .find(|enemy| enemy.formation.is_empty())
        .map(|enemy| format!("spawn `{}` has an empty formation", enemy.encounter_id))
}

pub(super) fn request_active_encounter_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
    game: Option<Res<GameState>>,
    enemies: Query<Entity, With<WorldEnemy>>,
    mut state: ResMut<WorldEncounterState>,
) {
    let current = game
        .as_deref()
        .and_then(|game| game.map().current())
        .map(|map| map.as_str());
    if current == state.map_id.as_deref() {
        return;
    }
    for entity in &enemies {
        commands.entity(entity).despawn();
    }
    *state = WorldEncounterState::default();
    let Some(map_id) = current else {
        return;
    };
    let Some(metadata_path) = inventory.map_metadata_path(map_id) else {
        fail_encounter(&mut state, format!("invalid encounter map id `{map_id}`"));
        return;
    };
    state.map_id = Some(map_id.to_owned());
    state.metadata = Some(asset_server.load(root.resolve(&metadata_path)));
    state.status = WorldEncounterStatus::Loading;
}

#[derive(SystemParam)]
pub(super) struct EncounterScenarioAssets<'w> {
    asset_server: Res<'w, AssetServer>,
    root: Res<'w, ScenarioRoot>,
    inventory: Res<'w, ScenarioInventory>,
}

#[expect(
    clippy::too_many_arguments,
    reason = "transactional encounter publication reads each independently typed asset store"
)]
pub(super) fn drive_active_encounter_assets(
    mut commands: Commands,
    scenario: EncounterScenarioAssets,
    metadata_assets: Res<Assets<MapMetadata>>,
    zone_assets: Res<Assets<EncounterZone>>,
    enemy_file_assets: Res<Assets<EnemyCatalogFile>>,
    boss_move_set_assets: Res<Assets<BossMoveSet>>,
    background_assets: Res<Assets<BattleBackgroundCatalog>>,
    sfx_assets: Res<Assets<SfxIndex>>,
    mut image_assets: ResMut<Assets<Image>>,
    mut shadow: Local<Option<Handle<Image>>>,
    maps: Res<Assets<TmxGroundAsset>>,
    render: Res<StaticMapRenderState>,
    restore: Option<Res<WorldEncounterRestore>>,
    game: Option<ResMut<GameState>>,
    existing: Query<(), With<WorldEnemy>>,
    mut state: ResMut<WorldEncounterState>,
) {
    let asset_server = &scenario.asset_server;
    let root = &scenario.root;
    let inventory = &scenario.inventory;
    if !matches!(state.status, WorldEncounterStatus::Loading) || !existing.is_empty() {
        return;
    }
    let Some(map_id) = state.map_id.clone() else {
        return;
    };
    let Some(metadata_handle) = state.metadata.as_ref() else {
        return;
    };
    let metadata = match optional_map_metadata(asset_server, &metadata_assets, metadata_handle) {
        OptionalMapMetadata::Ready(metadata) => metadata,
        OptionalMapMetadata::Loading => return,
        OptionalMapMetadata::Failed => {
            fail_encounter(
                &mut state,
                format!("map metadata for `{map_id}` failed to load"),
            );
            return;
        }
    };
    if !render.is_spawned_for(&map_id) {
        return;
    }
    let Some(map) = render.map(&maps) else {
        return;
    };
    let regular_spawns = spawn_tiles(map);
    let boss_spawn = boss_spawn_tile(map);
    // A map with no spawn tiles has no wandering enemies, but it can still host a
    // dialogue-scripted battle (`on_complete.start_battle`), which needs this map's zone for its
    // battle background and the enemy catalog to build its formation from. So the load runs to
    // completion and only the *spawning* is skipped, below. Nothing ambient turns on as a result:
    // `detect_enemy_contact` gates on `Spawned`, which a spawn-tile-free map never reaches.
    let has_spawn_tiles = !regular_spawns.is_empty() || boss_spawn.is_some();

    if state.zone.is_none() {
        if let Some(failure) = inventory.failure.as_ref() {
            fail_encounter(&mut state, failure.clone());
            return;
        }
        if !inventory.has_encounter_zone(&map_id) {
            // A map with no
            // `data/encount/<id>.yaml` has encounters disabled ("towns, inns — encounters
            // disabled") even when spawn tiles are painted.
            // Deciding from the inventory avoids requesting a file the asset server would log as
            // missing on every town and inn arrival.
            state.status = WorldEncounterStatus::NoEncounters;
            commands.remove_resource::<EnemyCatalog>();
            return;
        }
        let Some(zone_path) = inventory.encounter_path(&map_id) else {
            fail_encounter(
                &mut state,
                format!("invalid encounter-zone path for `{map_id}`"),
            );
            return;
        };
        state.zone = Some(asset_server.load(root.resolve(&zone_path)));
        state.enemy_files = inventory
            .enemy_catalogs
            .iter()
            .map(|path| asset_server.load(root.resolve(path)))
            .collect();
        state.boss_move_sets = inventory
            .boss_move_sets
            .iter()
            .map(|(logical, path)| (logical.clone(), asset_server.load(root.resolve(path))))
            .collect();
        let Some(background_path) = inventory.battle_backgrounds.as_ref() else {
            fail_encounter(
                &mut state,
                "scenario has no battle background catalog".to_owned(),
            );
            return;
        };
        state.backgrounds = Some(asset_server.load(root.resolve(background_path)));
        let sfx_path = ScenarioRelativePath::try_from(SFX_INDEX_PATH)
            .expect("canonical SFX index path is scenario-relative");
        state.sfx_index = Some(asset_server.load(root.resolve(&sfx_path)));
        return;
    }

    let zone_handle = state
        .zone
        .clone()
        .expect("set on an earlier frame; cleared only by a full state reset");
    if let LoadState::Failed(error) = asset_server.load_state(zone_handle.id()) {
        if optional_scenario_asset_is_missing(&error) {
            // The zone vanished after inventory discovery; treat it like an absent zone.
            state.status = WorldEncounterStatus::NoEncounters;
            commands.remove_resource::<EnemyCatalog>();
            return;
        }
        fail_encounter(
            &mut state,
            format!("encounter zone for `{map_id}` failed to load"),
        );
        return;
    }
    if state
        .enemy_files
        .iter()
        .any(|handle| crate::world_publication::asset_failed(asset_server, handle.id()))
    {
        fail_encounter(&mut state, "enemy catalog failed to load".to_owned());
        return;
    }
    if state
        .boss_move_sets
        .iter()
        .any(|(_, handle)| crate::world_publication::asset_failed(asset_server, handle.id()))
    {
        fail_encounter(
            &mut state,
            "boss move-set catalog failed to load".to_owned(),
        );
        return;
    }
    let Some(background_handle) = state.backgrounds.clone() else {
        return;
    };
    if matches!(
        asset_server.load_state(background_handle.id()),
        LoadState::Failed(_)
    ) {
        fail_encounter(
            &mut state,
            "battle-background catalog failed to load".to_owned(),
        );
        return;
    }
    let Some(sfx_handle) = state.sfx_index.clone() else {
        return;
    };
    if matches!(
        asset_server.load_state(sfx_handle.id()),
        LoadState::Failed(_)
    ) {
        fail_encounter(&mut state, "scenario SFX index failed to load".to_owned());
        return;
    }
    let Some(sfx_index) = sfx_assets.get(&sfx_handle) else {
        return;
    };
    let Some(encounter_sfx) = sfx_index.resolve_key(root, "encounter") else {
        fail_encounter(
            &mut state,
            "scenario SFX index has no `encounter` event".to_owned(),
        );
        return;
    };
    state.encounter_sfx = Some(encounter_sfx);
    let Some(zone) = zone_assets.get(&zone_handle) else {
        return;
    };
    if zone.effective_id(&map_id) != map_id {
        fail_encounter(
            &mut state,
            format!("encounter-zone id does not match active map `{map_id}`"),
        );
        return;
    }
    let Some(backgrounds) = background_assets.get(&background_handle) else {
        return;
    };
    if !backgrounds
        .0
        .iter()
        .any(|background| background.id == zone.background)
    {
        fail_encounter(
            &mut state,
            format!(
                "encounter zone references unknown background `{}`",
                zone.background
            ),
        );
        return;
    }
    let Some(files) = state
        .enemy_files
        .iter()
        .map(|handle| enemy_file_assets.get(handle))
        .collect::<Option<Vec<_>>>()
    else {
        return;
    };
    let Some(move_sets) = state
        .boss_move_sets
        .iter()
        .map(|(path, handle)| {
            boss_move_set_assets
                .get(handle)
                .map(|value| (path.as_str(), value))
        })
        .collect::<Option<Vec<_>>>()
    else {
        return;
    };
    let mut catalog = match EnemyCatalog::try_from_definitions(
        files.iter().flat_map(|file| file.entries().iter().cloned()),
    ) {
        Ok(catalog) => catalog,
        Err(error) => {
            fail_encounter(&mut state, error.to_string());
            return;
        }
    };
    if let Err(error) = catalog.resolve_boss_move_sets(move_sets) {
        fail_encounter(&mut state, error.to_string());
        return;
    }
    if !has_spawn_tiles {
        // The zone and catalog stay resident for a scripted battle; the status keeps the ambient
        // encounter loop off and stops this system from re-entering.
        commands.insert_resource(catalog);
        state.status = WorldEncounterStatus::NoEncounters;
        return;
    }
    let Some(mut game) = game else {
        return;
    };

    if state.pending.is_empty() {
        let restored = restore
            .as_deref()
            .filter(|restore| restore.map_id == map_id)
            .map(|restore| restore.enemies.clone());
        let restored_pool = restored.is_some();
        if let Some(restored) = restored {
            state
                .pending
                .extend(restored.into_iter().map(|enemy| PendingWorldEnemy {
                    encounter_id: enemy.encounter_id,
                    formation: enemy.formation,
                    origin: enemy.origin,
                    boss: enemy.boss,
                    chase_range: enemy.chase_range,
                    position: enemy.position,
                    facing: enemy.facing,
                    active: enemy.active,
                }));
            commands.remove_resource::<WorldEncounterRestore>();
        } else if let (Some(boss), Some(origin)) = (&zone.boss, boss_spawn)
            && !(boss.once
                && !boss.completion.set_flag.is_empty()
                && game.flags().is_set(&boss.completion.set_flag))
        {
            state.pending.push(PendingWorldEnemy {
                encounter_id: format!("{map_id}:boss"),
                formation: vec![boss.enemy_id.clone()],
                origin,
                boss: true,
                chase_range: 0,
                position: origin,
                facing: CardinalDirection::Down,
                active: true,
            });
        }
        if !restored_pool {
            for (index, origin) in regular_spawns.into_iter().enumerate() {
                if let Some(formation) = pick_weighted_formation(&zone.entries, game.rng_mut()) {
                    state.pending.push(PendingWorldEnemy {
                        encounter_id: format!("{map_id}:spawn:{index}"),
                        formation: formation.enemy_ids.clone(),
                        origin,
                        boss: false,
                        chase_range: formation.chase_range,
                        position: origin,
                        facing: CardinalDirection::Down,
                        active: true,
                    });
                }
            }
        }
        if let Some(failure) = empty_formation_failure(&state.pending) {
            fail_encounter(&mut state, failure);
            return;
        }
        let sprite_ids = state
            .pending
            .iter()
            .filter_map(|enemy| enemy.formation.first().cloned())
            .collect::<Vec<_>>();
        let boss_ids = state
            .pending
            .iter()
            .filter(|enemy| enemy.boss)
            .filter_map(|enemy| enemy.formation.first().cloned())
            .collect::<std::collections::BTreeSet<_>>();
        for sprite_id in sprite_ids {
            if catalog.enemy(&sprite_id).is_none() {
                fail_encounter(
                    &mut state,
                    format!("spawn references unknown enemy `{sprite_id}`"),
                );
                return;
            }
            // Only a boss with a standee file on disk asks for one; the listing comes from the
            // inventory so an absent file is never requested.
            if boss_ids.contains(&sprite_id)
                && let Some(standee) = inventory.enemy_standee(&sprite_id)
            {
                state
                    .standee_handles
                    .entry(sprite_id.clone())
                    .or_insert_with(|| asset_server.load(root.resolve(&standee)));
            }
        }
        commands.insert_resource(catalog);
        return;
    }

    if state
        .standee_handles
        .values()
        .any(|handle| crate::world_publication::asset_failed(asset_server, handle.id()))
    {
        fail_encounter(&mut state, "boss field standee failed to load".to_owned());
        return;
    }
    if state
        .standee_handles
        .values()
        .any(|handle| !image_assets.contains(handle))
    {
        return;
    }

    for pending in &state.pending {
        let top_left = enemy_top_left(pending.position);
        // An inactive enemy (the one just beaten, restored after its battle) must spawn hidden.
        // `update_world_enemies` hides it too, but only from its next run, which let the defeated
        // sprite flash on the map for a frame or two after returning from battle.
        let scale = if pending.active {
            Vec3::ONE
        } else {
            Vec3::ZERO
        };
        // Field enemies are never drawn and load no sheet: a hidden default sprite keeps the
        // entity's shape, so movement and contact are untouched.
        // The one exception is a boss whose optional standee image ships: that single frame is
        // drawn instead, feet on the collision footprint. Everything else stays hidden.
        let standee = pending
            .boss
            .then(|| state.standee_handles.get(&pending.formation[0]))
            .flatten();
        let is_standee = standee.is_some();
        let (sprite, translation, visibility) = match standee {
            Some(image) => (
                Sprite::from_image(image.clone()),
                enemy_standee_translation(top_left),
                Visibility::Inherited,
            ),
            None => (
                Sprite::default(),
                enemy_world_translation(top_left),
                Visibility::Hidden,
            ),
        };
        let mut entity = commands.spawn((
            sprite,
            Transform::from_translation(translation).with_scale(scale),
            visibility,
            WorldEnemy {
                encounter_id: pending.encounter_id.clone(),
                formation: pending.formation.clone(),
                origin: pending.origin,
                position: pending.position,
                top_left,
                boss: pending.boss,
                chase_range: pending.chase_range,
                active: pending.active,
                engaged: false,
                facing: pending.facing,
                wander_pause: random_pause(game.rng_mut()),
                wander_target: None,
                chasing: false,
            },
        ));
        if is_standee {
            entity.insert((bevy::sprite::Anchor::BOTTOM_CENTER, EnemyStandee));
            // The standee's feet are its origin, so the shadow sits just above it.
            entity.with_child(crate::blob_shadow::field_shadow(
                crate::blob_shadow::shared_handle(&mut image_assets, &mut shadow),
                STANDEE_SHADOW_WIDTH,
                STANDEE_SHADOW_FEET_Y,
            ));
        }
    }
    let interval = metadata
        .enemy_spawn
        .as_ref()
        .map(|spawn| spawn.interval.get() as f32)
        .or_else(|| zone.spawn_frequency.map(|frequency| frequency.get() as f32))
        .unwrap_or(DEFAULT_RESPAWN_SECONDS);
    state.cadence = Some(SpawnCadence::new(interval));
    state.status = WorldEncounterStatus::Spawned;
}

pub(super) fn spawn_tiles(map: &TmxGroundAsset) -> Vec<Position> {
    let Some(layer) = map
        .document()
        .tile_layers()
        .iter()
        .find(|layer| layer.name() == "spawn_tile")
    else {
        return Vec::new();
    };
    layer
        .gids()
        .iter()
        .enumerate()
        .filter(|(_, gid)| !gid.is_empty())
        .filter_map(|(index, _)| {
            let index = u32::try_from(index).ok()?;
            let x = i32::try_from(index % layer.width()).ok()?;
            let y = i32::try_from(index / layer.width()).ok()?;
            Some(Position::new(x, y))
        })
        .collect()
}

pub(super) fn boss_spawn_tile(map: &TmxGroundAsset) -> Option<Position> {
    let object = map
        .document()
        .object_groups()
        .iter()
        .find(|group| group.name() == "boss_enemy")?
        .objects()
        .first()?;
    Some(Position::new(
        (object.x() / f64::from(TILE_SIZE)).floor() as i32,
        (object.y() / f64::from(TILE_SIZE)).floor() as i32,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gameplay_rng::GameplayRng;

    const ZONE: &str = "id: fixture_zone\nname: Fixture\ndensity: 0.5\nbackground: fixture-bg\nentries:\n  - formation: []\n    weight: 1\n    chase_range: 0\n";

    fn pending(encounter_id: &str, formation: Vec<String>) -> PendingWorldEnemy {
        PendingWorldEnemy {
            encounter_id: encounter_id.to_owned(),
            formation,
            origin: Position::new(0, 0),
            boss: false,
            chase_range: 0,
            position: Position::new(0, 0),
            facing: CardinalDirection::Down,
            active: true,
        }
    }

    /// `formation: []` with a positive weight parses and is drawn like any other entry; the load
    /// must fail on it before sprites are requested instead of panicking when it publishes.
    #[test]
    fn an_empty_authored_formation_fails_the_encounter_load_and_valid_ones_pass() {
        let zone: EncounterZone = crate::scenario_yaml::from_str(ZONE).unwrap();
        let mut rng = GameplayRng::from_seed(7);
        let drawn = pick_weighted_formation(&zone.entries, &mut rng)
            .expect("the only entry has a positive weight");
        let spawns = vec![
            pending("fixture_zone:spawn:0", vec!["moss_hare".to_owned()]),
            pending("fixture_zone:spawn:1", drawn.enemy_ids.clone()),
        ];

        assert_eq!(
            empty_formation_failure(&spawns).as_deref(),
            Some("spawn `fixture_zone:spawn:1` has an empty formation")
        );
        assert_eq!(empty_formation_failure(&spawns[..1]), None);
    }
}
