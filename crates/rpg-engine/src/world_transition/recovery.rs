//! Publication rollback owns dormant source bundles until the destination is complete.

use bevy::{
    asset::{AssetPath, UntypedAssetId, VisitAssetDependencies},
    camera::{ClearColorConfig, visibility::RenderLayers},
    ecs::query::QueryFilter,
    prelude::*,
};

use super::*;
use crate::{
    encounter::EnemyCatalog,
    gameplay_canvas::{
        camera_follow::{CameraFollowTarget, MapCameraFollow},
        fixed_gameplay_camera,
    },
    scenario_manifest_asset::ActiveManifestLoad,
    scenario_spatial::{collision_occupancy::CollisionOccupancy, world_collision::WorldCollision},
    tmx_ground_asset::{StaticMapTile, WorldMapCamera},
    tsx_atlas_asset::TsxAtlasAsset,
    world_actor::{WorldActorState, WorldNpc},
    world_encounter::{WorldEncounterRestore, WorldEncounterState, WorldEnemy},
    world_object::{WorldItemBox, WorldObjectState, WorldSign},
    world_player::{WorldPlayerAnimation, WorldPlayerSpawnState},
    world_publication::{PublicationStatus, asset_failed},
};

const LOAD_TIMEOUT_SECONDS: f32 = 30.0;

/// A stored bundle has no live Sprite, camera, collision or gameplay marker. Its Entity ID
/// remains reserved so failure can restore the exact source rather than rerun RNG or spawns.
struct Stored<B: Bundle>(Vec<(Entity, B)>);

impl<B: Bundle + bevy::ecs::bundle::BundleFromComponents> Stored<B> {
    fn take<F: QueryFilter>(world: &mut World) -> Self {
        let entities = world
            .query_filtered::<Entity, F>()
            .iter(world)
            .collect::<Vec<_>>();
        Self(
            entities
                .into_iter()
                .map(|entity| {
                    let bundle = world
                        .entity_mut(entity)
                        .take::<B>()
                        .expect("complete World entity bundle");
                    (entity, bundle)
                })
                .collect(),
        )
    }

    fn restore(self, world: &mut World) {
        for (entity, bundle) in self.0 {
            world.entity_mut(entity).insert(bundle);
        }
    }

    fn discard(self, world: &mut World) {
        for (entity, _) in self.0 {
            world.despawn(entity);
        }
    }
}

type Tiles = Stored<(StaticMapTile, Sprite, Transform)>;
type Cameras = Stored<(WorldMapCamera, Camera, MapCameraFollow, Transform)>;
type Players = Stored<(
    WorldPlayer,
    WorldPlayerMotion,
    WorldPlayerAnimation,
    CameraFollowTarget,
    Sprite,
    Transform,
)>;
type Actors = Stored<(WorldNpc, Sprite, Transform)>;
type Signs = Stored<(WorldSign, Transform)>;
type Boxes = Stored<(WorldItemBox, Sprite, Transform)>;
type Enemies = Stored<(WorldEnemy, Sprite, Transform)>;

struct SourceWorld {
    reload: Option<super::hot_reload::ReloadSnapshot>,
    game: GameState,
    render: StaticMapRenderState,
    player: WorldPlayerSpawnState,
    actors: WorldActorState,
    objects: WorldObjectState,
    collision: WorldCollision,
    encounters: Option<WorldEncounterState>,
    catalog: Option<EnemyCatalog>,
    encounter_restore: Option<WorldEncounterRestore>,
    tiles: Tiles,
    cameras: Cameras,
    players: Players,
    npcs: Actors,
    signs: Signs,
    boxes: Boxes,
    enemies: Enemies,
}

impl SourceWorld {
    fn take(world: &mut World) -> Self {
        Self {
            reload: None,
            game: world.resource::<GameState>().clone(),
            render: world.resource::<StaticMapRenderState>().clone(),
            player: world.resource::<WorldPlayerSpawnState>().clone(),
            actors: world.resource::<WorldActorState>().clone(),
            objects: world.resource::<WorldObjectState>().clone(),
            collision: world.resource::<WorldCollision>().clone(),
            encounters: world.get_resource::<WorldEncounterState>().cloned(),
            catalog: world.get_resource::<EnemyCatalog>().cloned(),
            encounter_restore: world.get_resource::<WorldEncounterRestore>().cloned(),
            tiles: Stored::take::<With<StaticMapTile>>(world),
            cameras: Stored::take::<With<WorldMapCamera>>(world),
            players: Stored::take::<With<WorldPlayer>>(world),
            npcs: Stored::take::<With<WorldNpc>>(world),
            signs: Stored::take::<With<WorldSign>>(world),
            boxes: Stored::take::<With<WorldItemBox>>(world),
            enemies: Stored::take::<With<WorldEnemy>>(world),
        }
    }

    fn restore(self, world: &mut World) {
        clear_destination(world);
        if let Some(snapshot) = self.reload {
            snapshot.restore(world);
        }
        world.insert_resource(self.game);
        world.insert_resource(self.render);
        world.insert_resource(self.player);
        world.insert_resource(self.actors);
        world.insert_resource(self.objects);
        world.insert_resource(self.collision);
        if let Some(state) = self.encounters {
            world.insert_resource(state);
        }
        if let Some(catalog) = self.catalog {
            world.insert_resource(catalog);
        }
        if let Some(restore) = self.encounter_restore {
            world.insert_resource(restore);
        }
        self.tiles.restore(world);
        self.cameras.restore(world);
        self.players.restore(world);
        self.npcs.restore(world);
        self.signs.restore(world);
        self.boxes.restore(world);
        self.enemies.restore(world);
    }

    fn discard(self, world: &mut World) {
        self.tiles.discard(world);
        self.cameras.discard(world);
        self.players.discard(world);
        self.npcs.discard(world);
        self.signs.discard(world);
        self.boxes.discard(world);
        self.enemies.discard(world);
    }
}

#[derive(Default, Resource)]
pub(super) struct TransitionRecovery {
    source: Option<SourceWorld>,
    pub(super) initial_game: Option<GameState>,
    pub(super) initial_encounters: Option<WorldEncounterRestore>,
    return_available: bool,
    hot_reload: bool,
    retry_paths: Vec<AssetPath<'static>>,
    // AssetServer::reload queues asynchronous work; allow it to replace cached failure states.
    retry_grace: f32,
}

pub(super) fn loaders_enabled(
    transition: Res<WorldTransition>,
    recovery: Res<TransitionRecovery>,
    reload: Option<Res<super::hot_reload::HotReload>>,
) -> bool {
    transition.phase != TransitionPhase::Failed
        && recovery.retry_grace <= 0.0
        && super::hot_reload::loaders_ready(reload)
}

pub(super) fn begin_hot_reload(world: &mut World, snapshot: super::hot_reload::ReloadSnapshot) {
    let mut source = SourceWorld::take(world);
    source.reload = Some(snapshot);
    clear_destination(world);
    let mut recovery = world.resource_mut::<TransitionRecovery>();
    recovery.source = Some(source);
    recovery.hot_reload = true;
    recovery.return_available = false;
    let mut transition = world.resource_mut::<WorldTransition>();
    transition.phase = TransitionPhase::Publishing;
    transition.alpha = 1.0;
    transition.wait_seconds = 0.0;
    transition.failure = None;
}

fn clear_destination(world: &mut World) {
    let entities = world
        .query_filtered::<Entity, Or<(
            With<StaticMapTile>,
            With<WorldMapCamera>,
            With<WorldPlayer>,
            With<WorldNpc>,
            With<WorldSign>,
            With<WorldItemBox>,
            With<WorldEnemy>,
        )>>()
        .iter(world)
        .collect::<Vec<_>>();
    for entity in entities {
        world.despawn(entity);
    }
    world.insert_resource(StaticMapRenderState::default());
    world.insert_resource(WorldPlayerSpawnState::default());
    world.insert_resource(WorldActorState::default());
    world.insert_resource(WorldObjectState::default());
    world.insert_resource(WorldCollision::default());
    if world.contains_resource::<WorldEncounterState>() {
        world.insert_resource(WorldEncounterState::default());
    }
    world.remove_resource::<EnemyCatalog>();
    world.remove_resource::<WorldEncounterRestore>();
}

fn publication_status(world: &World, map_id: &str) -> PublicationStatus {
    let mut statuses = vec![
        world
            .resource::<StaticMapRenderState>()
            .publication_status(map_id),
        world
            .resource::<WorldPlayerSpawnState>()
            .publication_status(map_id, world.resource::<GameState>().controlled_member_id()),
        world
            .resource::<WorldActorState>()
            .publication_status(map_id),
        world
            .resource::<WorldObjectState>()
            .publication_status(map_id),
        world
            .resource::<WorldCollision>()
            .publication_status(map_id),
    ];
    if let Some(encounters) = world.get_resource::<WorldEncounterState>() {
        statuses.push(encounters.publication_status(map_id));
    }
    if let Some(index) = statuses
        .iter()
        .position(|status| matches!(status, PublicationStatus::Failed(_)))
    {
        return statuses.swap_remove(index);
    }
    if statuses
        .iter()
        .all(|status| *status == PublicationStatus::Ready)
    {
        PublicationStatus::Ready
    } else {
        PublicationStatus::Pending
    }
}

fn current_publication_status(world: &World) -> PublicationStatus {
    world
        .get_resource::<GameState>()
        .and_then(|game| game.map().current())
        .map(|id| publication_status(world, id.as_str()))
        .unwrap_or_else(|| PublicationStatus::Failed("game session has no current map".into()))
}

pub(super) fn drive_transition_loading(world: &mut World) {
    let phase = world.resource::<WorldTransition>().phase;
    if !matches!(
        phase,
        TransitionPhase::Loading | TransitionPhase::Publishing
    ) {
        return;
    }
    let delta = world.resource::<Time<Real>>().delta_secs();
    let grace = &mut world.resource_mut::<TransitionRecovery>().retry_grace;
    if *grace > 0.0 {
        *grace = (*grace - delta).max(0.0);
        return;
    }
    world.resource_mut::<WorldTransition>().wait_seconds += delta;
    if phase == TransitionPhase::Publishing {
        if world
            .resource::<TransitionRecovery>()
            .initial_game
            .is_none()
            && world.resource::<TransitionRecovery>().source.is_none()
        {
            let game = world.get_resource::<GameState>().cloned();
            world.resource_mut::<TransitionRecovery>().initial_game = game;
        }
        let status = match super::hot_reload::prepare(world) {
            PublicationStatus::Ready => current_publication_status(world),
            other => other,
        };
        match status {
            PublicationStatus::Ready => {
                if let Some(source) = world.resource_mut::<TransitionRecovery>().source.take() {
                    source.discard(world);
                }
                super::hot_reload::finish(world, true);
                *world.resource_mut::<TransitionRecovery>() = TransitionRecovery::default();
                world
                    .resource_mut::<WorldTransition>()
                    .destination_published();
                return;
            }
            PublicationStatus::Failed(reason) => {
                fail_publication(world, reason);
                return;
            }
            PublicationStatus::Pending => {}
        }
    } else {
        // TMX, collision and portal preflight runs with every source entity still active.
        let result = match preflight(world) {
            PublicationStatus::Ready => current_publication_status(world),
            other => other,
        };
        match result {
            PublicationStatus::Ready => {
                let Some(pending) = world.resource::<WorldTransition>().pending.clone() else {
                    return;
                };
                if !world.contains_resource::<GameState>() {
                    fail_publication(world, "game session disappeared during transition".into());
                    return;
                }
                let source = SourceWorld::take(world);
                clear_destination(world);
                world.resource_mut::<TransitionRecovery>().source = Some(source);
                world.resource_mut::<TransitionRecovery>().return_available = false;
                world.resource_mut::<GameState>().map_mut().move_to(
                    pending.target_map,
                    pending.target_position,
                    pending.facing,
                );
                world
                    .resource_mut::<WorldTransition>()
                    .destination_committed();
                return;
            }
            PublicationStatus::Failed(reason) => {
                fail_publication(world, reason);
                return;
            }
            PublicationStatus::Pending => {}
        }
    }
    if world.resource::<WorldTransition>().wait_seconds >= LOAD_TIMEOUT_SECONDS {
        fail_publication(world, "map loading timed out after 30 seconds".into());
    }
}

fn preflight(world: &World) -> PublicationStatus {
    let Some(handle) = world
        .resource::<TransitionDestinationLoad>()
        .handle
        .as_ref()
    else {
        return PublicationStatus::Failed("destination map path is invalid".into());
    };
    let server = world.resource::<AssetServer>();
    if asset_failed(server, handle.id()) {
        return PublicationStatus::Failed(
            "destination map or image dependency failed to load".into(),
        );
    }
    if !server.is_loaded_with_dependencies(handle.id()) {
        return PublicationStatus::Pending;
    }
    let Some(map) = world.resource::<Assets<TmxGroundAsset>>().get(handle) else {
        return PublicationStatus::Pending;
    };
    if let Err(error) = map.visible_bundles(world.resource::<Assets<TsxAtlasAsset>>()) {
        return PublicationStatus::Failed(format!("destination map cannot render: {error}"));
    }
    if let Err(error) = CollisionOccupancy::from_tmx_document(map.document()) {
        return PublicationStatus::Failed(format!("destination collision is invalid: {error}"));
    }
    if let Err(error) = runtime_portals(map.document()) {
        return PublicationStatus::Failed(format!("destination portals are invalid: {error}"));
    }
    PublicationStatus::Ready
}

fn retry_paths(world: &World) -> Vec<AssetPath<'static>> {
    let mut ids: Vec<UntypedAssetId> = world.resource::<StaticMapRenderState>().asset_ids();
    ids.extend(world.resource::<WorldPlayerSpawnState>().asset_ids());
    ids.extend(world.resource::<WorldActorState>().asset_ids());
    ids.extend(world.resource::<WorldObjectState>().asset_ids());
    if let Some(state) = world.get_resource::<WorldEncounterState>() {
        ids.extend(state.asset_ids());
    }
    if let Some(manifest) = world.get_resource::<ActiveManifestLoad>() {
        ids.push(manifest.handle().id().untyped());
    }
    if let Some(handle) = &world.resource::<TransitionDestinationLoad>().handle {
        ids.push(handle.id().untyped());
    }
    // Include failed dependencies, which are absent from their Assets store but retained by
    // their successfully parsed parent. Reloading only the TMX/TSX cannot retry a cached PNG failure.
    let mut dependencies = std::collections::HashMap::new();
    for (id, map) in world.resource::<Assets<TmxGroundAsset>>().iter() {
        let mut children = Vec::new();
        map.visit_dependencies(&mut |id| children.push(id));
        dependencies.insert(id.untyped(), children);
    }
    for (id, atlas) in world.resource::<Assets<TsxAtlasAsset>>().iter() {
        let mut children = Vec::new();
        atlas.visit_dependencies(&mut |id| children.push(id));
        dependencies.insert(id.untyped(), children);
    }
    let mut seen = std::collections::HashSet::new();
    let mut cursor = 0;
    while let Some(&id) = ids.get(cursor) {
        cursor += 1;
        if seen.insert(id)
            && let Some(children) = dependencies.get(&id)
        {
            ids.extend(children);
        }
    }
    let server = world.resource::<AssetServer>();
    let mut paths = ids
        .into_iter()
        .filter_map(|id| server.get_path(id))
        .map(AssetPath::into_owned)
        .collect::<Vec<_>>();
    paths.sort_by_key(ToString::to_string);
    paths.dedup();
    paths
}

fn fail_publication(world: &mut World, reason: String) {
    let paths = retry_paths(world);
    let was_loading = world.resource::<WorldTransition>().phase == TransitionPhase::Loading;
    let source = world.resource_mut::<TransitionRecovery>().source.take();
    let can_return = (was_loading && current_publication_status(world) == PublicationStatus::Ready)
        || source.is_some();
    if !can_return
        && world
            .resource::<TransitionRecovery>()
            .initial_game
            .is_none()
    {
        let initial_game = world.get_resource::<GameState>().cloned();
        world.resource_mut::<TransitionRecovery>().initial_game = initial_game;
    }
    if let Some(source) = source {
        source.restore(world);
    }
    super::hot_reload::finish(world, false);
    let mut recovery = world.resource_mut::<TransitionRecovery>();
    recovery.retry_paths = paths;
    recovery.return_available = can_return;
    world
        .resource_mut::<WorldTransition>()
        .destination_failed(reason);
}

pub(super) fn handle_recovery_input(world: &mut World) {
    if world.resource::<WorldTransition>().phase != TransitionPhase::Failed {
        return;
    }
    let Some(keys) = world.get_resource::<ButtonInput<KeyCode>>() else {
        return;
    };
    let retry = keys.just_pressed(KeyCode::KeyR);
    let back = keys.just_pressed(KeyCode::Escape);
    let title = keys.just_pressed(KeyCode::KeyT);
    if title {
        world.write_message(crate::app_state::AppStateTransitionRequest::new(
            AppState::Title,
        ));
    } else if super::hot_reload::recovering_late_assets(world) {
        // Title remains available while outstanding IO is prevented from changing the source.
    } else if back && world.resource::<TransitionRecovery>().return_available {
        let mut transition = world.resource_mut::<WorldTransition>();
        transition.pending = None;
        transition.failure = None;
        transition.suppress_entry_until_exit = true;
        transition.phase = TransitionPhase::FadingIn;
        *world.resource_mut::<TransitionRecovery>() = TransitionRecovery::default();
    } else if retry {
        if world.resource::<TransitionRecovery>().hot_reload {
            super::hot_reload::begin(world);
            return;
        }
        let paths = world.resource::<TransitionRecovery>().retry_paths.clone();
        for path in paths {
            world.resource::<AssetServer>().reload(path);
        }
        if world.resource::<TransitionRecovery>().return_available {
            // Keep the restored source bundles until preflight has succeeded again.
            world.resource_mut::<WorldTransition>().phase = TransitionPhase::Loading;
        } else {
            clear_destination(world);
            if let Some(game) = world.resource::<TransitionRecovery>().initial_game.clone() {
                world.insert_resource(game);
            }
            if let Some(restore) = world
                .resource::<TransitionRecovery>()
                .initial_encounters
                .clone()
            {
                world.insert_resource(restore);
            }
            world.resource_mut::<WorldTransition>().phase = TransitionPhase::Publishing;
        }
        let mut transition = world.resource_mut::<WorldTransition>();
        transition.failure = None;
        transition.wait_seconds = 0.0;
        world.resource_mut::<TransitionRecovery>().retry_grace = 0.25;
    }
}

#[derive(Component)]
struct RecoveryUi;

#[derive(Component)]
struct RecoveryText;

pub(super) fn update_recovery_ui(world: &mut World) {
    let existing = world
        .query_filtered::<Entity, With<RecoveryUi>>()
        .iter(world)
        .collect::<Vec<_>>();
    let transition = world.resource::<WorldTransition>();
    if transition.phase != TransitionPhase::Failed {
        for entity in existing {
            world.despawn(entity);
        }
        return;
    }
    let back = if world.resource::<TransitionRecovery>().return_available {
        "\nEsc: return to the previous map"
    } else {
        ""
    };
    let actions = if super::hot_reload::recovering_late_assets(world) {
        "Asset reads are settling; retry and source return resume afterwards.\nT: return to title"
            .to_owned()
    } else {
        format!("R: retry{back}\nT: return to title")
    };
    let text = format!(
        "Unable to enter the map\n{}\n\n{actions}",
        transition.failure().unwrap_or("map publication failed")
    );
    if !existing.is_empty() {
        for mut label in world
            .query_filtered::<&mut Text, With<RecoveryText>>()
            .iter_mut(world)
        {
            if label.0 != text {
                label.0.clone_from(&text);
            }
        }
        return;
    }
    // Initial map failure may leave no map camera. A dedicated UI camera makes recovery visible
    // even when the TMX or protagonist fails before any World render entity has been published.
    world.spawn((
        fixed_gameplay_camera(),
        Camera {
            order: 100,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        RenderLayers::none(),
        IsDefaultUiCamera,
        RecoveryUi,
    ));
    world
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::ZERO,
                top: Val::ZERO,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::BLACK),
            GlobalZIndex(10_001),
            RecoveryUi,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(text),
                RecoveryText,
                TextFont {
                    font_size: 24.0.into(),
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    max_width: Val::Percent(90.0),
                    ..default()
                },
            ));
        });
}

pub(super) fn cleanup_recovery(world: &mut World) {
    if let Some(mut source) = world.resource_mut::<TransitionRecovery>().source.take() {
        if let Some(snapshot) = source.reload.take() {
            snapshot.restore(world);
            super::hot_reload::finish(world, false);
        }
        source.discard(world);
    }
    *world.resource_mut::<TransitionRecovery>() = TransitionRecovery::default();
    let entities = world
        .query_filtered::<Entity, With<RecoveryUi>>()
        .iter(world)
        .collect::<Vec<_>>();
    for entity in entities {
        world.despawn(entity);
    }
}

#[cfg(test)]
mod tests;
