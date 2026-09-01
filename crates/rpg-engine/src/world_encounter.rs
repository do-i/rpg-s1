//! Visible World enemies and the transactional World-to-Battle handoff.

use std::collections::BTreeMap;

use bevy::{
    asset::{AssetServer, Assets, Handle, LoadState},
    audio::{PlaybackMode, Volume},
    ecs::{schedule::ApplyDeferred, system::SystemParam},
    prelude::*,
};

use crate::{
    app_state::{AppState, AppStateTransitionRequest},
    encounter::{
        BattleEntry, EnemyCatalog, PreBattleReturnContext, SpawnCadence, WorldEnemyReturnState,
        build_battle_entry, build_scripted_battle_entry, party_encounter_modifiers,
        pick_weighted_formation,
    },
    field_menu_domain::{CatalogStatus, FieldMenuCatalog},
    game_state::GameState,
    gameplay_canvas::fixed_gameplay_camera,
    scenario_audio::{BGM_INDEX_PATH, BgmIndex, SFX_INDEX_PATH, SfxIndex},
    scenario_balance::BalanceData,
    scenario_battle_background::BattleBackgroundCatalog,
    scenario_encounter::EncounterZone,
    scenario_enemy::{BossMoveSet, EnemyCatalogFile},
    scenario_inventory::ScenarioInventory,
    scenario_map::{MapMetadata, optional_scenario_asset_is_missing},
    scenario_path::ScenarioRelativePath,
    scenario_root::ScenarioRoot,
    scenario_spatial::{
        CardinalDirection, Position, collision_occupancy::CollisionOccupancy,
        world_collision::WorldCollision,
    },
    tmx_ground_asset::{StaticMapRenderState, TmxGroundAsset, world_entity_y_z},
    tsx_atlas_asset::TsxAtlasAsset,
    world_audio::LogicalBgmPlayer,
    world_player::{
        CHARACTER_COLLISION_HEIGHT, CHARACTER_COLLISION_OFFSET_X, CHARACTER_COLLISION_OFFSET_Y,
        CHARACTER_COLLISION_WIDTH, CHARACTER_SPRITE_SIZE, CharacterCollisionRect, WorldPlayer,
        WorldPlayerMotion,
    },
};

const TILE_SIZE: u32 = 32;
const ENEMY_SPRITE_HALF_HEIGHT: f32 = 32.0;
const ENEMY_WALK_ROW_OFFSET: u32 = 8;
const ENEMY_ATLAS_COLUMNS: u32 = 9;
const ENEMY_MOVE_SPEED_PIXELS_PER_SECOND: f32 = TILE_SIZE as f32 * 3.5;
const ENEMY_FRAME_SECONDS: f32 = 0.15;
const MAX_ENEMY_DELTA_SECONDS: f32 = 0.1;
const DEFAULT_RESPAWN_SECONDS: f32 = 30.0;
const BATTLE_FLASH_SECONDS: f32 = 0.55;

pub(crate) struct WorldEncounterPlugin;

impl Plugin for WorldEncounterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldEncounterState>()
            .init_resource::<BattleTransition>()
            .add_systems(
                OnEnter(AppState::World),
                (reset_world_encounters, spawn_battle_flash_overlay).chain(),
            )
            .add_systems(
                Update,
                (
                    request_active_encounter_assets,
                    ApplyDeferred,
                    drive_active_encounter_assets,
                    ApplyDeferred,
                    // The ambient World holds still while any overlay owns the
                    // screen; asset loading and the battle hand-off below do not.
                    (update_world_enemies, detect_enemy_contact)
                        .chain()
                        .run_if(crate::world_pause::world_simulation_running),
                    start_scripted_battle,
                    advance_battle_transition,
                    update_battle_flash_overlay,
                )
                    .chain()
                    .run_if(in_state(AppState::World)),
            )
            .add_systems(
                OnExit(AppState::World),
                (cleanup_world_encounters, cleanup_battle_flash_overlay),
            );
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum WorldEncounterStatus {
    #[default]
    Idle,
    Loading,
    NoEncounters,
    Spawned,
    Failed,
}

#[derive(Debug, Default, Resource)]
pub(crate) struct WorldEncounterState {
    map_id: Option<String>,
    metadata: Option<Handle<MapMetadata>>,
    zone: Option<Handle<EncounterZone>>,
    enemy_files: Vec<Handle<EnemyCatalogFile>>,
    boss_move_sets: Vec<(String, Handle<BossMoveSet>)>,
    backgrounds: Option<Handle<BattleBackgroundCatalog>>,
    sfx_index: Option<Handle<SfxIndex>>,
    encounter_sfx: Option<String>,
    pending: Vec<PendingWorldEnemy>,
    sprite_handles: BTreeMap<String, Handle<TsxAtlasAsset>>,
    cadence: Option<SpawnCadence>,
    status: WorldEncounterStatus,
    failure: Option<String>,
}

impl WorldEncounterState {
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "runtime diagnostics are test-facing")
    )]
    pub(crate) const fn status(&self) -> WorldEncounterStatus {
        self.status
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "runtime diagnostics are test-facing")
    )]
    pub(crate) fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }
}

#[derive(Clone, Debug)]
struct PendingWorldEnemy {
    encounter_id: String,
    formation: Vec<String>,
    origin: Position,
    boss: bool,
    chase_range: u32,
    position: Position,
    facing: CardinalDirection,
    active: bool,
}

/// One-shot encounter pool restored after a battle outcome.
#[derive(Clone, Debug, Resource)]
pub(crate) struct WorldEncounterRestore {
    pub(crate) map_id: String,
    pub(crate) enemies: Vec<WorldEnemyReturnState>,
}

#[derive(Component, Clone, Debug, PartialEq)]
pub(crate) struct WorldEnemy {
    encounter_id: String,
    formation: Vec<String>,
    origin: Position,
    position: Position,
    top_left: Vec2,
    boss: bool,
    chase_range: u32,
    active: bool,
    engaged: bool,
    facing: CardinalDirection,
    frame: u32,
    frame_elapsed: f32,
    wander_pause: f32,
    wander_target: Option<Vec2>,
}

#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "runtime inspection accessors support Gate 8 diagnostics"
    )
)]
impl WorldEnemy {
    pub(crate) fn encounter_id(&self) -> &str {
        &self.encounter_id
    }

    pub(crate) const fn tile_position(&self) -> Position {
        self.position
    }

    pub(crate) fn formation(&self) -> &[String] {
        &self.formation
    }

    pub(crate) const fn is_boss(&self) -> bool {
        self.boss
    }

    fn collision_rect(&self) -> CharacterCollisionRect {
        enemy_collision_rect(self.top_left)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum BattleTransitionPhase {
    #[default]
    Idle,
    Flashing,
    Requested,
}

#[derive(Debug, Default, Resource)]
pub(crate) struct BattleTransition {
    phase: BattleTransitionPhase,
    elapsed: f32,
}

impl BattleTransition {
    pub(crate) const fn input_locked(&self) -> bool {
        !matches!(self.phase, BattleTransitionPhase::Idle)
    }

    fn request(&mut self) -> bool {
        if self.phase != BattleTransitionPhase::Idle {
            return false;
        }
        self.phase = BattleTransitionPhase::Flashing;
        self.elapsed = 0.0;
        true
    }
}

#[derive(Component)]
struct BattleFlashOverlay;

fn reset_world_encounters(
    mut state: ResMut<WorldEncounterState>,
    mut battle: ResMut<BattleTransition>,
) {
    *state = WorldEncounterState::default();
    *battle = BattleTransition::default();
}

fn spawn_battle_flash_overlay(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::ZERO,
            top: Val::ZERO,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::NONE),
        Visibility::Hidden,
        GlobalZIndex(11_000),
        Pickable::IGNORE,
        BattleFlashOverlay,
    ));
}

fn cleanup_battle_flash_overlay(
    mut commands: Commands,
    overlays: Query<Entity, With<BattleFlashOverlay>>,
) {
    for entity in &overlays {
        commands.entity(entity).despawn();
    }
}

fn request_active_encounter_assets(
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
        state.status = WorldEncounterStatus::Failed;
        state.failure = Some(format!("invalid encounter map id `{map_id}`"));
        return;
    };
    state.map_id = Some(map_id.to_owned());
    state.metadata = Some(asset_server.load(root.resolve(&metadata_path)));
    state.status = WorldEncounterStatus::Loading;
}

#[derive(SystemParam)]
struct EncounterScenarioAssets<'w> {
    asset_server: Res<'w, AssetServer>,
    root: Res<'w, ScenarioRoot>,
    inventory: Res<'w, ScenarioInventory>,
}

#[expect(
    clippy::too_many_arguments,
    reason = "transactional encounter publication reads each independently typed asset store"
)]
fn drive_active_encounter_assets(
    mut commands: Commands,
    scenario: EncounterScenarioAssets,
    metadata_assets: Res<Assets<MapMetadata>>,
    zone_assets: Res<Assets<EncounterZone>>,
    enemy_file_assets: Res<Assets<EnemyCatalogFile>>,
    boss_move_set_assets: Res<Assets<BossMoveSet>>,
    background_assets: Res<Assets<BattleBackgroundCatalog>>,
    sfx_assets: Res<Assets<SfxIndex>>,
    atlas_assets: Res<Assets<TsxAtlasAsset>>,
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
    // A missing `data/maps/<id>.yaml` is a valid runtime state for a TMX-only map (the pinned
    // engine's `load_yaml_optional`, see `MapMetadata::empty`), not a load failure; only a real
    // reader/parse error is fatal here.
    let empty_metadata;
    let metadata = match asset_server.load_state(metadata_handle.id()) {
        LoadState::Failed(error) if optional_scenario_asset_is_missing(&error) => {
            empty_metadata = MapMetadata::empty();
            &empty_metadata
        }
        LoadState::Failed(_) => {
            fail_encounter(
                &mut state,
                format!("map metadata for `{map_id}` failed to load"),
            );
            return;
        }
        _ => {
            let Some(metadata) = metadata_assets.get(metadata_handle) else {
                return;
            };
            metadata
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

    let zone_handle = state.zone.clone().expect("requested above");
    if let LoadState::Failed(error) = asset_server.load_state(zone_handle.id()) {
        if optional_scenario_asset_is_missing(&error) {
            // Matches the pinned engine's `EncounterManager.set_zone`: a missing
            // `data/encount/<id>.yaml` disables encounters for this map ("towns, inns —
            // encounters disabled") even when spawn tiles are painted (e.g.
            // `zone_02_open_plains_cave_02`), rather than failing to load.
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
        .any(|handle| matches!(asset_server.load_state(handle.id()), LoadState::Failed(_)))
    {
        fail_encounter(&mut state, "enemy catalog failed to load".to_owned());
        return;
    }
    if state
        .boss_move_sets
        .iter()
        .any(|(_, handle)| matches!(asset_server.load_state(handle.id()), LoadState::Failed(_)))
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
        let sprite_ids = state
            .pending
            .iter()
            .filter_map(|enemy| enemy.formation.first().cloned())
            .collect::<Vec<_>>();
        for sprite_id in sprite_ids {
            if catalog.enemy(&sprite_id).is_none() {
                fail_encounter(
                    &mut state,
                    format!("spawn references unknown enemy `{sprite_id}`"),
                );
                return;
            }
            let path = ScenarioRelativePath::try_from(
                format!("assets/sprites/enemies/{sprite_id}.tsx").as_str(),
            )
            .expect("validated enemy id produced a scenario-relative sprite path");
            state
                .sprite_handles
                .entry(sprite_id.clone())
                .or_insert_with(|| asset_server.load(root.resolve(&path)));
        }
        commands.insert_resource(catalog);
        return;
    }

    if state
        .sprite_handles
        .values()
        .any(|handle| matches!(asset_server.load_state(handle.id()), LoadState::Failed(_)))
    {
        fail_encounter(&mut state, "world-enemy sprite failed to load".to_owned());
        return;
    }
    if state
        .sprite_handles
        .values()
        .any(|handle| !asset_server.is_loaded_with_dependencies(handle.id()))
    {
        return;
    }

    for pending in &state.pending {
        let sprite_id = pending
            .formation
            .first()
            .expect("only nonempty formations are pending");
        let sprite = state
            .sprite_handles
            .get(sprite_id)
            .and_then(|handle| atlas_assets.get(handle))
            .and_then(|atlas| {
                atlas
                    .sprite_for_tile(enemy_tile(CardinalDirection::Down, 0))
                    .ok()
            })
            .unwrap_or_else(|| Sprite::from_color(Color::srgb(0.8, 0.2, 0.2), Vec2::splat(64.0)));
        let top_left = enemy_top_left(pending.position);
        commands.spawn((
            sprite,
            Transform::from_translation(enemy_world_translation(top_left)),
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
                frame: 0,
                frame_elapsed: 0.0,
                wander_pause: random_pause(game.rng_mut()),
                wander_target: None,
            },
        ));
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

fn fail_encounter(state: &mut WorldEncounterState, failure: String) {
    state.status = WorldEncounterStatus::Failed;
    state.failure = Some(failure);
}

fn spawn_tiles(map: &TmxGroundAsset) -> Vec<Position> {
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

fn boss_spawn_tile(map: &TmxGroundAsset) -> Option<Position> {
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

#[derive(SystemParam)]
struct EnemySimulationAssets<'w> {
    collision: Option<Res<'w, WorldCollision>>,
    balances: Res<'w, Assets<BalanceData>>,
}

#[expect(
    clippy::too_many_arguments,
    reason = "world enemy simulation reads independent timing, map, party, and collision state"
)]
fn update_world_enemies(
    time: Res<Time>,
    assets: EnemySimulationAssets,
    game: Option<ResMut<GameState>>,
    players: Query<&WorldPlayerMotion, With<WorldPlayer>>,
    mut state: ResMut<WorldEncounterState>,
    mut enemies: Query<(Entity, &mut WorldEnemy, &mut Sprite, &mut Transform)>,
    mut snapshot: Local<Vec<(Entity, CharacterCollisionRect, bool)>>,
    mut timings: Option<ResMut<crate::frame_timing::FrameTimings>>,
) {
    let _measurement = timings
        .as_deref_mut()
        .map(|timings| timings.measure("world.enemies"));
    if state.status != WorldEncounterStatus::Spawned {
        return;
    }
    let Some(mut game) = game else {
        return;
    };
    let Some(collision) = assets.collision.as_deref() else {
        return;
    };
    let Some(map_id) = game.map().current().map(|map| map.as_str()) else {
        return;
    };
    let Some(collision) = collision.occupancy_for(map_id) else {
        return;
    };
    let modifiers = assets
        .balances
        .iter()
        .next()
        .map(|(_, balance)| party_encounter_modifiers(game.party(), &balance.spawner))
        .unwrap_or_else(|| {
            party_encounter_modifiers(game.party(), &BalanceData::default().spawner)
        });
    let player_top_left = players
        .single()
        .map(|motion| motion.top_left())
        .unwrap_or_else(|_| enemy_top_left(game.map().position()));
    snapshot.clear();
    snapshot.extend(
        enemies
            .iter()
            .map(|(entity, enemy, _, _)| (entity, enemy.collision_rect(), enemy.active)),
    );
    let delta = time.delta_secs().min(MAX_ENEMY_DELTA_SECONDS);

    if let Some(cadence) = state.cadence.as_mut() {
        let has_inactive = snapshot.iter().any(|(_, _, active)| !active);
        if cadence.advance(delta, modifiers.interval_multiplier, has_inactive) {
            let inactive = snapshot
                .iter()
                .filter(|(_, _, active)| !active)
                .map(|(entity, _, _)| *entity)
                .collect::<Vec<_>>();
            if !inactive.is_empty() {
                let index = (game.rng_mut().next_u64() % inactive.len() as u64) as usize;
                if let Ok((_, mut enemy, _, _)) = enemies.get_mut(inactive[index]) {
                    enemy.active = true;
                    enemy.engaged = false;
                    enemy.position = enemy.origin;
                    enemy.top_left = enemy_top_left(enemy.origin);
                    enemy.wander_target = None;
                    enemy.wander_pause = random_pause(game.rng_mut());
                }
            }
        }
    }

    for (entity, mut enemy, mut sprite, mut transform) in &mut enemies {
        if !enemy.active {
            transform.scale = Vec3::ZERO;
            continue;
        }
        transform.scale = Vec3::ONE;
        if !enemy.boss {
            let effective_chase = enemy
                .chase_range
                .saturating_sub(modifiers.chase_range_reduction);
            let distance_tiles =
                (player_top_left - enemy.top_left).abs().max_element() / TILE_SIZE as f32;
            let moved = if effective_chase > 0 && distance_tiles <= effective_chase as f32 {
                chase_enemy(
                    &mut enemy,
                    entity,
                    player_top_left,
                    delta,
                    snapshot.as_slice(),
                    collision,
                )
            } else {
                wander_enemy(
                    &mut enemy,
                    entity,
                    delta,
                    snapshot.as_slice(),
                    collision,
                    game.rng_mut(),
                )
            };
            if moved {
                advance_enemy_frame(&mut enemy, delta);
            }
        }
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = enemy_tile(enemy.facing, enemy.frame) as usize;
        }
        transform.translation = enemy_world_translation(enemy.top_left);
    }
}

fn chase_enemy(
    enemy: &mut WorldEnemy,
    entity: Entity,
    player_top_left: Vec2,
    delta: f32,
    snapshot: &[(Entity, CharacterCollisionRect, bool)],
    collision: &CollisionOccupancy,
) -> bool {
    let toward_player = player_top_left - enemy.top_left;
    let distance = toward_player.length();
    if distance < 1.0 {
        return false;
    }
    enemy.facing = direction_toward(enemy.top_left, player_top_left);
    let step = (ENEMY_MOVE_SPEED_PIXELS_PER_SECOND * delta).min(distance);
    try_move_enemy(
        enemy,
        entity,
        enemy.top_left + toward_player / distance * step,
        snapshot,
        collision,
    )
}

fn wander_enemy(
    enemy: &mut WorldEnemy,
    entity: Entity,
    delta: f32,
    snapshot: &[(Entity, CharacterCollisionRect, bool)],
    collision: &CollisionOccupancy,
    rng: &mut crate::gameplay_rng::GameplayRng,
) -> bool {
    if enemy.wander_target.is_none() {
        enemy.wander_pause -= delta;
        enemy.frame = 0;
        if enemy.wander_pause <= 0.0 {
            enemy.wander_target = pick_wander_target(enemy, entity, snapshot, collision, rng);
            if enemy.wander_target.is_none() {
                enemy.wander_pause = random_pause(rng);
            }
        }
        return false;
    }
    let target = enemy.wander_target.expect("checked above");
    let toward_target = target - enemy.top_left;
    let distance = toward_target.abs().max_element();
    let move_distance = ENEMY_MOVE_SPEED_PIXELS_PER_SECOND * delta;
    if distance <= move_distance {
        let moved = try_move_enemy(enemy, entity, target, snapshot, collision);
        enemy.wander_target = None;
        enemy.wander_pause = random_pause(rng);
        enemy.frame = 0;
        return moved;
    }
    enemy.facing = direction_toward(enemy.top_left, target);
    let step = Vec2::new(
        toward_target.x.signum() * move_distance.min(toward_target.x.abs()),
        toward_target.y.signum() * move_distance.min(toward_target.y.abs()),
    );
    if !try_move_enemy(enemy, entity, enemy.top_left + step, snapshot, collision) {
        enemy.wander_target = None;
        enemy.wander_pause = random_pause(rng);
        enemy.frame = 0;
        return false;
    }
    true
}

fn pick_wander_target(
    enemy: &WorldEnemy,
    entity: Entity,
    snapshot: &[(Entity, CharacterCollisionRect, bool)],
    collision: &CollisionOccupancy,
    rng: &mut crate::gameplay_rng::GameplayRng,
) -> Option<Vec2> {
    const RANGE_PIXELS: i32 = 4 * TILE_SIZE as i32;
    const SPAN: u64 = (RANGE_PIXELS as u64 * 2) + 1;
    let origin = enemy_top_left(enemy.origin);
    for _ in 0..8 {
        let x = i32::try_from(rng.next_u64() % SPAN).ok()? - RANGE_PIXELS;
        let y = i32::try_from(rng.next_u64() % SPAN).ok()? - RANGE_PIXELS;
        let target = origin + Vec2::new(x as f32, y as f32);
        if !enemy_blocked(target, entity, snapshot, collision) {
            return Some(target);
        }
    }
    None
}

fn try_move_enemy(
    enemy: &mut WorldEnemy,
    entity: Entity,
    target: Vec2,
    snapshot: &[(Entity, CharacterCollisionRect, bool)],
    collision: &CollisionOccupancy,
) -> bool {
    if enemy_blocked(target, entity, snapshot, collision) {
        return false;
    }
    enemy.top_left = target;
    enemy.position = enemy_tile_position(target);
    true
}

fn enemy_blocked(
    top_left: Vec2,
    entity: Entity,
    snapshot: &[(Entity, CharacterCollisionRect, bool)],
    collision: &CollisionOccupancy,
) -> bool {
    let rect = enemy_collision_rect(top_left);
    collision.is_rect_blocked(rect.x, rect.y, rect.width, rect.height)
        || snapshot.iter().any(|(other, occupied, active)| {
            *other != entity && *active && rect.overlaps(*occupied)
        })
}

fn advance_enemy_frame(enemy: &mut WorldEnemy, delta: f32) {
    enemy.frame_elapsed += delta;
    while enemy.frame_elapsed >= ENEMY_FRAME_SECONDS {
        enemy.frame_elapsed -= ENEMY_FRAME_SECONDS;
        enemy.frame = next_walk_frame(enemy.frame);
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "contact atomically captures world, data, audio, and transition state"
)]
fn detect_enemy_contact(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    game: Option<Res<GameState>>,
    enemy_catalog: Option<Res<EnemyCatalog>>,
    item_catalog: Res<FieldMenuCatalog>,
    zones: Res<Assets<EncounterZone>>,
    metadata_assets: Res<Assets<MapMetadata>>,
    players: Query<&WorldPlayerMotion, With<WorldPlayer>>,
    mut state: ResMut<WorldEncounterState>,
    mut battle_transition: ResMut<BattleTransition>,
    mut enemies: Query<(Entity, &mut WorldEnemy)>,
) {
    if state.status != WorldEncounterStatus::Spawned || battle_transition.input_locked() {
        return;
    }
    let (Some(game), Some(enemy_catalog)) = (game, enemy_catalog) else {
        return;
    };
    if item_catalog.status() != CatalogStatus::Ready {
        if item_catalog.status() == CatalogStatus::Failed {
            let failure = item_catalog
                .failure()
                .unwrap_or("field item catalog failed to load")
                .to_owned();
            fail_encounter(&mut state, failure);
        }
        return;
    }
    let player = game.map().position();
    let player_rect = players
        .single()
        .map(|motion| motion.collision_rect())
        .unwrap_or_else(|_| enemy_collision_rect(enemy_top_left(player)));
    let Some((contact_entity, contact)) = enemies
        .iter()
        .find(|(_, enemy)| {
            enemy.active && !enemy.engaged && enemy.collision_rect().overlaps(player_rect)
        })
        .map(|(entity, enemy)| (entity, enemy.clone()))
    else {
        return;
    };
    let (Some(zone_handle), Some(metadata_handle)) = (state.zone.as_ref(), state.metadata.as_ref())
    else {
        return;
    };
    let (Some(zone), Some(metadata)) =
        (zones.get(zone_handle), metadata_assets.get(metadata_handle))
    else {
        return;
    };
    let Some(map_id) = game.map().current().map(|map| map.as_str().to_owned()) else {
        return;
    };
    let world_enemies = enemies
        .iter()
        .map(|(_, enemy)| WorldEnemyReturnState {
            encounter_id: enemy.encounter_id.clone(),
            formation: enemy.formation.clone(),
            origin: enemy.origin,
            position: enemy.position,
            facing: enemy.facing,
            boss: enemy.boss,
            chase_range: enemy.chase_range,
            active: enemy.active && enemy.encounter_id != contact.encounter_id,
        })
        .collect();
    let context = PreBattleReturnContext {
        map_id,
        position: player,
        facing: game.map().facing(),
        world_bgm_key: metadata.bgm.clone(),
        world_enemies,
    };
    let entry = match build_battle_entry(
        &contact.encounter_id,
        &contact.formation,
        zone,
        &enemy_catalog,
        &item_catalog,
        game.party(),
        game.repository(),
        game.flags(),
        contact.boss,
        context,
    ) {
        Ok(entry) => entry,
        Err(error) => {
            state.failure = Some(error.to_string());
            return;
        }
    };
    let Some(encounter_sfx) = state.encounter_sfx.clone() else {
        state.failure = Some("encounter SFX was not resolved before contact".to_owned());
        return;
    };
    if !battle_transition.request() {
        return;
    }
    commands.spawn((
        AudioPlayer::new(asset_server.load(encounter_sfx)),
        PlaybackSettings::DESPAWN,
    ));
    let Ok((_, mut enemy)) = enemies.get_mut(contact_entity) else {
        return;
    };
    enemy.engaged = true;
    enemy.active = false;
    if let Some(cadence) = state.cadence.as_mut() {
        cadence.reset();
    }
    commands.insert_resource(entry);
}

/// A battle a dialogue asked for with `on_complete.start_battle`.
///
/// Inserted by `world_interaction` only once the dialogue that requested it has closed, so the
/// fight starts after the last line rather than under it.
#[derive(Clone, Debug, Resource)]
pub(crate) struct ScriptedBattleRequest {
    pub(crate) enemy_id: String,
}

/// Hands a scripted battle to the same entry pipeline a wandering encounter uses.
#[expect(
    clippy::too_many_arguments,
    reason = "assembling a battle entry reads every catalog the encounter path reads"
)]
fn start_scripted_battle(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    request: Option<Res<ScriptedBattleRequest>>,
    game: Option<Res<GameState>>,
    enemy_catalog: Option<Res<EnemyCatalog>>,
    item_catalog: Res<FieldMenuCatalog>,
    zones: Res<Assets<EncounterZone>>,
    metadata_assets: Res<Assets<MapMetadata>>,
    mut state: ResMut<WorldEncounterState>,
    mut battle_transition: ResMut<BattleTransition>,
    enemies: Query<&WorldEnemy>,
) {
    let Some(request) = request else {
        return;
    };
    if battle_transition.input_locked() {
        return;
    }
    match state.status {
        // The map's encounter assets are still settling; the request waits rather than failing.
        WorldEncounterStatus::Idle | WorldEncounterStatus::Loading => return,
        WorldEncounterStatus::Failed => {
            commands.remove_resource::<ScriptedBattleRequest>();
            return;
        }
        WorldEncounterStatus::NoEncounters | WorldEncounterStatus::Spawned => {}
    }
    let enemy_id = request.enemy_id.clone();
    let (Some(game), Some(enemy_catalog)) = (game, enemy_catalog) else {
        fail_scripted_battle(
            &mut commands,
            &mut state,
            format!("scripted battle `{enemy_id}` has no enemy catalog on this map"),
        );
        return;
    };
    if item_catalog.status() != CatalogStatus::Ready {
        return;
    }
    let Some(zone) = state.zone.as_ref().and_then(|handle| zones.get(handle)) else {
        fail_scripted_battle(
            &mut commands,
            &mut state,
            format!(
                "scripted battle `{enemy_id}` needs this map's `data/encount/<map_id>.yaml` for \
                 its battle background"
            ),
        );
        return;
    };
    let Some(map_id) = game.map().current().map(|map| map.as_str().to_owned()) else {
        return;
    };
    let world_bgm_key = state
        .metadata
        .as_ref()
        .and_then(|handle| metadata_assets.get(handle))
        .and_then(|metadata| metadata.bgm.clone());
    let context = PreBattleReturnContext {
        map_id,
        position: game.map().position(),
        facing: game.map().facing(),
        world_bgm_key,
        world_enemies: enemies
            .iter()
            .map(|enemy| WorldEnemyReturnState {
                encounter_id: enemy.encounter_id.clone(),
                formation: enemy.formation.clone(),
                origin: enemy.origin,
                position: enemy.position,
                facing: enemy.facing,
                boss: enemy.boss,
                chase_range: enemy.chase_range,
                active: enemy.active,
            })
            .collect(),
    };
    let entry = match build_scripted_battle_entry(
        &enemy_id,
        zone,
        &enemy_catalog,
        &item_catalog,
        game.party(),
        game.repository(),
        game.flags(),
        context,
    ) {
        Ok(entry) => entry,
        Err(error) => {
            fail_scripted_battle(&mut commands, &mut state, error.to_string());
            return;
        }
    };
    if !battle_transition.request() {
        return;
    }
    if let Some(encounter_sfx) = state.encounter_sfx.clone() {
        commands.spawn((
            AudioPlayer::new(asset_server.load(encounter_sfx)),
            PlaybackSettings::DESPAWN,
        ));
    }
    if let Some(cadence) = state.cadence.as_mut() {
        cadence.reset();
    }
    commands.remove_resource::<ScriptedBattleRequest>();
    commands.insert_resource(entry);
}

/// Drops the request so a broken one cannot retry every frame, and records why.
fn fail_scripted_battle(commands: &mut Commands, state: &mut WorldEncounterState, failure: String) {
    commands.remove_resource::<ScriptedBattleRequest>();
    fail_encounter(state, failure);
}

fn advance_battle_transition(
    time: Res<Time>,
    mut transition: ResMut<BattleTransition>,
    mut requests: MessageWriter<AppStateTransitionRequest>,
) {
    if transition.phase != BattleTransitionPhase::Flashing {
        return;
    }
    transition.elapsed += time.delta_secs();
    if transition.elapsed >= BATTLE_FLASH_SECONDS {
        transition.phase = BattleTransitionPhase::Requested;
        requests.write(AppStateTransitionRequest::new(AppState::Battle));
    }
}

fn update_battle_flash_overlay(
    transition: Res<BattleTransition>,
    mut overlays: Query<(&mut BackgroundColor, &mut Visibility), With<BattleFlashOverlay>>,
) {
    let alpha = if transition.phase == BattleTransitionPhase::Flashing {
        let progress = (transition.elapsed / BATTLE_FLASH_SECONDS).clamp(0.0, 1.0);
        (progress * std::f32::consts::PI * 3.0).sin().abs()
    } else if transition.phase == BattleTransitionPhase::Requested {
        1.0
    } else {
        0.0
    };
    for (mut color, mut visibility) in &mut overlays {
        color.0 = Color::srgba(1.0, 1.0, 1.0, alpha);
        *visibility = if transition.phase == BattleTransitionPhase::Idle {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }
}

fn cleanup_world_encounters(
    mut commands: Commands,
    enemies: Query<Entity, With<WorldEnemy>>,
    mut state: ResMut<WorldEncounterState>,
) {
    for entity in &enemies {
        commands.entity(entity).despawn();
    }
    state.pending.clear();
    state.sprite_handles.clear();
}

fn enemy_top_left(position: Position) -> Vec2 {
    Vec2::new(
        position.x as f32 * TILE_SIZE as f32 + TILE_SIZE as f32 / 2.0
            - (CHARACTER_COLLISION_OFFSET_X + CHARACTER_COLLISION_WIDTH / 2.0),
        position.y as f32 * TILE_SIZE as f32 + TILE_SIZE as f32 / 2.0
            - (CHARACTER_COLLISION_OFFSET_Y + CHARACTER_COLLISION_HEIGHT / 2.0),
    )
}

fn enemy_collision_rect(top_left: Vec2) -> CharacterCollisionRect {
    CharacterCollisionRect {
        x: top_left.x + CHARACTER_COLLISION_OFFSET_X,
        y: top_left.y + CHARACTER_COLLISION_OFFSET_Y,
        width: CHARACTER_COLLISION_WIDTH,
        height: CHARACTER_COLLISION_HEIGHT,
    }
}

fn enemy_tile_position(top_left: Vec2) -> Position {
    let rect = enemy_collision_rect(top_left);
    Position::new(
        ((rect.x + rect.width / 2.0) / TILE_SIZE as f32).floor() as i32,
        ((rect.y + rect.height / 2.0) / TILE_SIZE as f32).floor() as i32,
    )
}

fn enemy_world_translation(top_left: Vec2) -> Vec3 {
    let center = top_left + Vec2::splat(CHARACTER_SPRITE_SIZE / 2.0);
    let world_y = -center.y;
    Vec3::new(
        center.x,
        world_y,
        world_entity_y_z(world_y, ENEMY_SPRITE_HALF_HEIGHT),
    )
}

fn direction_toward(from: Vec2, to: Vec2) -> CardinalDirection {
    let delta = to - from;
    if delta.x.abs() >= delta.y.abs() {
        if delta.x < 0.0 {
            CardinalDirection::Left
        } else {
            CardinalDirection::Right
        }
    } else if delta.y < 0.0 {
        CardinalDirection::Up
    } else {
        CardinalDirection::Down
    }
}

const fn enemy_tile(direction: CardinalDirection, frame: u32) -> u32 {
    let direction_row = match direction {
        CardinalDirection::Up => 0,
        CardinalDirection::Left => 1,
        CardinalDirection::Down => 2,
        CardinalDirection::Right => 3,
    };
    (ENEMY_WALK_ROW_OFFSET + direction_row) * ENEMY_ATLAS_COLUMNS + frame
}

const fn next_walk_frame(frame: u32) -> u32 {
    if frame >= 8 { 1 } else { frame + 1 }
}

fn random_pause(rng: &mut crate::gameplay_rng::GameplayRng) -> f32 {
    1.0 + (rng.next_u64() as f64 / u64::MAX as f64 * 2.5) as f32
}

/// Minimal M8 presentation proving background and battle-BGM selection before M9 owns the loop.
pub(crate) struct BattleEntryPlugin;

impl Plugin for BattleEntryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BattlePresentationState>()
            .add_systems(OnEnter(AppState::Battle), begin_battle_presentation)
            .add_systems(
                Update,
                drive_battle_audio.run_if(in_state(AppState::Battle)),
            )
            .add_systems(OnExit(AppState::Battle), cleanup_battle_presentation);
    }
}

#[derive(Debug, Default, Resource)]
struct BattlePresentationState {
    bgm_index: Option<Handle<BgmIndex>>,
    audio_started: bool,
}

#[derive(Component)]
struct BattlePresentation;

#[derive(Component)]
struct BattleBgm;

fn begin_battle_presentation(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    entry: Option<Res<BattleEntry>>,
    mut state: ResMut<BattlePresentationState>,
    logical_bgm: Query<Entity, With<LogicalBgmPlayer>>,
) {
    *state = BattlePresentationState::default();
    for entity in &logical_bgm {
        commands.entity(entity).despawn();
    }
    let Some(_entry) = entry else {
        return;
    };
    commands.spawn((fixed_gameplay_camera(), BattlePresentation));
    let bgm_index = ScenarioRelativePath::try_from(BGM_INDEX_PATH)
        .expect("canonical BGM index path is scenario-relative");
    state.bgm_index = Some(asset_server.load(root.resolve(&bgm_index)));
}

fn drive_battle_audio(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    indexes: Res<Assets<BgmIndex>>,
    entry: Option<Res<BattleEntry>>,
    mut state: ResMut<BattlePresentationState>,
) {
    if state.audio_started {
        return;
    }
    let (Some(entry), Some(handle)) = (entry, state.bgm_index.as_ref()) else {
        return;
    };
    if matches!(asset_server.load_state(handle.id()), LoadState::Failed(_)) {
        return;
    }
    let Some(index) = indexes.get(handle) else {
        return;
    };
    let Some(path) = index.resolve_key(&root, &entry.bgm_key) else {
        return;
    };
    commands.spawn((
        AudioPlayer::new(asset_server.load(path)),
        PlaybackSettings {
            mode: PlaybackMode::Loop,
            volume: Volume::Linear(0.3),
            ..default()
        },
        LogicalBgmPlayer,
        BattleBgm,
        BattlePresentation,
    ));
    state.audio_started = true;
}

fn cleanup_battle_presentation(
    mut commands: Commands,
    entities: Query<Entity, With<BattlePresentation>>,
) {
    for entity in &entities {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use bevy::{
        asset::AssetApp,
        image::{CompressedImageFormats, ImageLoader, ImagePlugin},
        time::TimeUpdateStrategy,
    };

    use super::*;
    use crate::{
        encounter::{BattleSide, BuildBattleError},
        encounter_assets::EncounterAssetPlugin,
        field_menu_domain::{FieldMenuDomainPlugin, derived_stats},
        name_entry::NameEntryConfirmed,
        runtime_map::RuntimeMapId,
        scenario_new_game_assets::{ActiveNewGameInputs, ActiveNewGameInputsStatus},
        test_support::headless_title_app_with_asset_base,
        tmx_ground_asset::TmxGroundAssetPlugin,
        tsx_atlas_asset::TsxAtlasAssetPlugin,
        world_actor::{WorldActorPlugin, WorldActorState},
        world_audio::WorldAudioPlugin,
        world_interaction::SfxIndexAssetLoader,
        world_object::{WorldObjectPlugin, WorldObjectState},
    };

    #[test]
    fn transition_accepts_exactly_one_request_and_locks_input() {
        let state = WorldEncounterState::default();
        assert_eq!(state.status(), WorldEncounterStatus::Idle);
        assert!(state.failure().is_none());
        let mut transition = BattleTransition::default();
        assert!(transition.request());
        assert!(!transition.request());
        assert!(transition.input_locked());
    }

    #[test]
    fn enemy_walk_tiles_use_the_lpc_rows_eight_through_eleven() {
        assert_eq!(enemy_tile(CardinalDirection::Up, 0), 72);
        assert_eq!(enemy_tile(CardinalDirection::Left, 0), 81);
        assert_eq!(enemy_tile(CardinalDirection::Down, 0), 90);
        assert_eq!(enemy_tile(CardinalDirection::Right, 8), 107);
    }

    #[test]
    fn deterministic_chase_stays_open_and_avoids_other_enemies() {
        let owner = ScenarioRelativePath::try_from("assets/maps/test.tmx").unwrap();
        let document = crate::tmx_header::parse_tmx_map_document(
            "<map version=\"1.10\" orientation=\"orthogonal\" width=\"4\" height=\"3\" tilewidth=\"32\" tileheight=\"32\" infinite=\"0\"><layer id=\"1\" name=\"collision\" width=\"4\" height=\"3\"><data encoding=\"csv\">0,0,0,0,\n0,0,0,0,\n0,0,0,0</data></layer></map>",
            &owner,
        )
        .unwrap();
        let collision = CollisionOccupancy::from_tmx_document(&document).unwrap();
        let entity = Entity::from_bits(1);
        let mut enemy = WorldEnemy {
            encounter_id: "spawn".to_owned(),
            formation: vec!["goblin".to_owned()],
            origin: Position::new(0, 1),
            position: Position::new(0, 1),
            top_left: enemy_top_left(Position::new(0, 1)),
            boss: false,
            chase_range: 8,
            active: true,
            engaged: false,
            facing: CardinalDirection::Down,
            frame: 0,
            frame_elapsed: 0.0,
            wander_pause: 0.0,
            wander_target: None,
        };
        let origin = enemy.top_left;
        let other_rect = enemy_collision_rect(enemy_top_left(Position::new(1, 1)));
        let snapshot = [
            (entity, enemy.collision_rect(), true),
            (Entity::from_bits(2), other_rect, true),
        ];
        assert!(chase_enemy(
            &mut enemy,
            entity,
            enemy_top_left(Position::new(3, 1)),
            0.1,
            &snapshot,
            &collision,
        ));
        assert!((enemy.top_left.x - origin.x - 11.2).abs() < 0.001);
        assert_eq!(enemy.position, Position::new(0, 1));
        assert_eq!(enemy.facing, CardinalDirection::Right);
        assert!(!chase_enemy(
            &mut enemy,
            entity,
            enemy_top_left(Position::new(3, 1)),
            0.1,
            &snapshot,
            &collision,
        ));
        assert!((enemy.top_left.x - origin.x - 11.2).abs() < 0.001);
    }

    #[test]
    fn deterministic_wander_target_stays_bounded_open_and_unoccupied() {
        let owner = ScenarioRelativePath::try_from("assets/maps/test.tmx").unwrap();
        let document = crate::tmx_header::parse_tmx_map_document(
            "<map version=\"1.10\" orientation=\"orthogonal\" width=\"10\" height=\"10\" tilewidth=\"32\" tileheight=\"32\" infinite=\"0\"><layer id=\"1\" name=\"collision\" width=\"10\" height=\"10\"><data encoding=\"csv\">0,0,0,0,0,0,0,0,0,0,\n0,0,0,0,0,0,0,0,0,0,\n0,0,0,0,0,0,0,0,0,0,\n0,0,0,0,0,0,0,0,0,0,\n0,0,0,0,0,0,0,0,0,0,\n0,0,0,0,0,0,0,0,0,0,\n0,0,0,0,0,0,0,0,0,0,\n0,0,0,0,0,0,0,0,0,0,\n0,0,0,0,0,0,0,0,0,0,\n0,0,0,0,0,0,0,0,0,0</data></layer></map>",
            &owner,
        )
        .unwrap();
        let collision = CollisionOccupancy::from_tmx_document(&document).unwrap();
        let entity = Entity::from_bits(1);
        let other = Entity::from_bits(2);
        let enemy = WorldEnemy {
            encounter_id: "spawn".to_owned(),
            formation: vec!["goblin".to_owned()],
            origin: Position::new(5, 5),
            position: Position::new(5, 5),
            top_left: enemy_top_left(Position::new(5, 5)),
            boss: false,
            chase_range: 0,
            active: true,
            engaged: false,
            facing: CardinalDirection::Down,
            frame: 0,
            frame_elapsed: 0.0,
            wander_pause: 0.0,
            wander_target: None,
        };
        let snapshot = [
            (entity, enemy.collision_rect(), true),
            (
                other,
                enemy_collision_rect(enemy_top_left(Position::new(6, 6))),
                true,
            ),
        ];
        let mut first = crate::gameplay_rng::GameplayRng::from_seed(77);
        let mut second = crate::gameplay_rng::GameplayRng::from_seed(77);
        let a = pick_wander_target(&enemy, entity, &snapshot, &collision, &mut first).unwrap();
        let b = pick_wander_target(&enemy, entity, &snapshot, &collision, &mut second).unwrap();
        assert_eq!(a, b);
        assert!((a.x - enemy.top_left.x).abs() <= 4.0 * TILE_SIZE as f32);
        assert!((a.y - enemy.top_left.y).abs() <= 4.0 * TILE_SIZE as f32);
        assert!(!enemy_collision_rect(a).overlaps(snapshot[1].1));
        let rect = enemy_collision_rect(a);
        assert!(!collision.is_rect_blocked(rect.x, rect.y, rect.width, rect.height));
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
            .init_asset_loader::<SfxIndexAssetLoader>()
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
            if app.world().resource::<WorldEncounterState>().status()
                == WorldEncounterStatus::Spawned
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
        assert_eq!(app.world().resource::<EnemyCatalog>().len(), 107);
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
        let mut query = app.world_mut().query::<&WorldEnemy>();
        let enemies = query.iter(app.world()).cloned().collect::<Vec<_>>();
        assert_eq!(enemies.len(), 6);
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
            if app.world().resource::<WorldEncounterState>().status()
                == WorldEncounterStatus::Spawned
            {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        let mut query = app.world_mut().query::<&WorldEnemy>();
        assert_eq!(query.iter(app.world()).count(), 6);

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
        assert_eq!(party.defense, i64::from(expected_party_stats.constitution));
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

    /// Covers W12.2 encounters end to end: Open Plains' seeded formations and boss spawn from
    /// `data/encount/zone_02_open_plains.yaml` and the TMX `spawn_tile`/`boss_enemy` layers, then
    /// both caves' TMX-only content (no `data/maps/<id>.yaml`). Also runs `WorldActorPlugin` and
    /// `WorldObjectPlugin` alongside the encounter plugin, because all three read the same
    /// optional per-map YAML: this is the regression proof that a missing file resolves as empty
    /// metadata (Python `load_yaml_optional` parity) instead of a permanent `Failed` status that
    /// would soft-lock any future portal transition into these maps at the `Publishing` barrier
    /// (`world_transition::drive_transition_loading`).
    #[test]
    fn production_open_plains_and_caves_spawn_or_gracefully_skip_encounters_without_metadata_soft_lock()
     {
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
            .init_asset_loader::<SfxIndexAssetLoader>()
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
            if app.world().resource::<WorldEncounterState>().status()
                == WorldEncounterStatus::Spawned
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
        assert_eq!(boss.tile_position(), Position::new(12, 11));

        // Cave_01: no `data/maps/...yaml`, and no `spawn_tile`/`boss_enemy` authoring at all.
        app.world_mut()
            .resource_mut::<GameState>()
            .map_mut()
            .move_to(
                RuntimeMapId::try_new("zone_02_open_plains_cave_01").unwrap(),
                Position::new(2, 2),
                CardinalDirection::Down,
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
                && app
                    .world()
                    .resource::<WorldActorState>()
                    .is_spawned_for("zone_02_open_plains_cave_01")
                && app
                    .world()
                    .resource::<WorldObjectState>()
                    .is_spawned_for("zone_02_open_plains_cave_01")
            {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(
            app.world().resource::<WorldEncounterState>().status(),
            WorldEncounterStatus::NoEncounters,
            "{:?}",
            app.world().resource::<WorldEncounterState>().failure()
        );
        assert!(
            app.world()
                .resource::<WorldActorState>()
                .is_spawned_for("zone_02_open_plains_cave_01")
        );
        assert!(
            app.world()
                .resource::<WorldObjectState>()
                .is_spawned_for("zone_02_open_plains_cave_01")
        );

        // Cave_02: no `data/maps/...yaml` either, but its TMX paints 9 `spawn_tile` gids with no
        // matching `data/encount/...yaml`. The pinned engine's `EncounterManager.set_zone`
        // disables encounters whenever the zone file is absent ("towns, inns — encounters
        // disabled"), regardless of authored spawn tiles, so this must resolve the same way as a
        // town: `NoEncounters`, not a load failure.
        app.world_mut()
            .resource_mut::<GameState>()
            .map_mut()
            .move_to(
                RuntimeMapId::try_new("zone_02_open_plains_cave_02").unwrap(),
                Position::new(2, 2),
                CardinalDirection::Down,
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
                && app
                    .world()
                    .resource::<WorldActorState>()
                    .is_spawned_for("zone_02_open_plains_cave_02")
                && app
                    .world()
                    .resource::<WorldObjectState>()
                    .is_spawned_for("zone_02_open_plains_cave_02")
            {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(
            app.world().resource::<WorldEncounterState>().status(),
            WorldEncounterStatus::NoEncounters,
            "{:?}",
            app.world().resource::<WorldEncounterState>().failure()
        );
        assert!(
            app.world()
                .resource::<WorldActorState>()
                .is_spawned_for("zone_02_open_plains_cave_02")
        );
        assert!(
            app.world()
                .resource::<WorldObjectState>()
                .is_spawned_for("zone_02_open_plains_cave_02")
        );
    }

    /// Covers W12.3's `zone_03_marshland` encounters end to end: the 18 authored `spawn_tile`
    /// entries plus the boss spawn from `data/encount/zone_03_marshland.yaml` and the TMX
    /// `spawn_tile`/`boss_enemy` layers, a full respawn round-trip through a neighboring town with
    /// no encounter zone at all (`port_town_harborgate`, which has no `data/encount/*.yaml`), and
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
            .init_asset_loader::<SfxIndexAssetLoader>()
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
            if app.world().resource::<WorldEncounterState>().status()
                == WorldEncounterStatus::Spawned
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
        // (`port_town_harborgate`) and confirm every enemy despawns and the zone goes to
        // `NoEncounters`, exactly like the W12.1/W12.2 town fixtures.
        app.world_mut()
            .resource_mut::<GameState>()
            .map_mut()
            .move_to(
                RuntimeMapId::try_new("port_town_harborgate").unwrap(),
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
            if app.world().resource::<WorldEncounterState>().status()
                == WorldEncounterStatus::Spawned
            {
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
        assert_eq!(party.defense, i64::from(expected_party_stats.constitution));
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
            .init_asset_loader::<SfxIndexAssetLoader>()
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
}
