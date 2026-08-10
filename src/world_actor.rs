//! Source-authored World NPC loading, presence, occupancy, animation, and bounded wandering.

use bevy::{asset::LoadState, ecs::schedule::ApplyDeferred, prelude::*};

use crate::{
    app_state::AppState,
    game_state::GameState,
    gameplay_rng::GameplayRng,
    scenario_map::{MapMetadata, NpcAnimationMode, NpcMetadata},
    scenario_path::ScenarioRelativePath,
    scenario_root::ScenarioRoot,
    scenario_spatial::{CardinalDirection, Position, collision_occupancy::CollisionOccupancy},
    tile_coordinates::tmx_tile_center,
    tmx_ground_asset::{StaticMapRenderState, TmxGroundAsset, world_entity_y_z},
    tsx_atlas_asset::TsxAtlasAsset,
};

const TILE_SIZE: u32 = 32;
const SPRITE_HALF_HEIGHT: f32 = 32.0;
const BASE_FRAME_SECONDS: f32 = 0.15;
const WANDER_PAUSE_MIN: f32 = 1.0;
const WANDER_PAUSE_SPAN: f32 = 2.5;

pub(crate) struct WorldActorPlugin;

impl Plugin for WorldActorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldActorState>()
            .add_systems(OnEnter(AppState::World), reset_world_actors)
            .add_systems(
                Update,
                (
                    sync_world_actor_request,
                    ApplyDeferred,
                    drive_world_actor_load,
                )
                    .chain()
                    .run_if(in_state(AppState::World)),
            )
            .add_systems(Update, update_world_npcs.run_if(in_state(AppState::World)))
            .add_systems(OnExit(AppState::World), cleanup_world_actors);
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum WorldActorStatus {
    #[default]
    Idle,
    LoadingMetadata,
    LoadingSprites,
    Spawned,
    Failed,
}

#[derive(Debug, Default, Resource)]
pub(crate) struct WorldActorState {
    map_id: Option<String>,
    flags: Vec<String>,
    metadata: Option<Handle<MapMetadata>>,
    sprites: Vec<(String, Handle<TsxAtlasAsset>)>,
    status: WorldActorStatus,
}

impl WorldActorState {
    pub(crate) fn is_spawned_for(&self, map_id: &str) -> bool {
        self.map_id.as_deref() == Some(map_id) && self.status == WorldActorStatus::Spawned
    }
}

/// One live, present NPC. Tile occupancy is authoritative for grid interaction and collision.
#[derive(Component, Debug)]
#[allow(dead_code, reason = "M5.11 interaction consumes these fields next")]
pub(crate) struct WorldNpc {
    map_id: String,
    id: String,
    name: String,
    dialogue_id: String,
    origin: Position,
    position: Position,
    facing: CardinalDirection,
    default_facing: CardinalDirection,
    mode: NpcAnimationMode,
    speed: f32,
    range: i32,
    interaction_range: f32,
    frame: u32,
    frame_elapsed: f32,
    wander_pause: f32,
    wander_target: Option<Position>,
}

#[allow(dead_code, reason = "M5.11 interaction consumes these accessors next")]
impl WorldNpc {
    pub(crate) fn map_id(&self) -> &str {
        &self.map_id
    }

    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn dialogue_id(&self) -> &str {
        &self.dialogue_id
    }

    pub(crate) const fn tile_position(&self) -> Position {
        self.position
    }

    pub(crate) const fn facing(&self) -> CardinalDirection {
        self.facing
    }

    pub(crate) fn interaction_range(&self) -> f32 {
        self.interaction_range
    }
}

fn reset_world_actors(mut state: ResMut<WorldActorState>) {
    *state = WorldActorState::default();
}

fn sync_world_actor_request(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    scenario_root: Res<ScenarioRoot>,
    game: Option<Res<GameState>>,
    actors: Query<Entity, With<WorldNpc>>,
    mut state: ResMut<WorldActorState>,
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

    for entity in &actors {
        commands.entity(entity).despawn();
    }
    *state = WorldActorState::default();
    let Some(map_id) = current else {
        return;
    };
    let logical = format!("data/maps/{map_id}.yaml");
    let Ok(logical) = ScenarioRelativePath::try_from(logical.as_str()) else {
        state.status = WorldActorStatus::Failed;
        return;
    };
    state.map_id = Some(map_id.to_owned());
    state.flags = flags;
    state.metadata = Some(asset_server.load(scenario_root.resolve(&logical)));
    state.status = WorldActorStatus::LoadingMetadata;
}

#[expect(
    clippy::too_many_arguments,
    reason = "transactional NPC publication needs each independently loaded Bevy asset resource"
)]
fn drive_world_actor_load(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    metadata_assets: Res<Assets<MapMetadata>>,
    atlas_assets: Res<Assets<TsxAtlasAsset>>,
    scenario_root: Res<ScenarioRoot>,
    game: Option<ResMut<GameState>>,
    actors: Query<(), With<WorldNpc>>,
    mut state: ResMut<WorldActorState>,
) {
    if matches!(
        state.status,
        WorldActorStatus::Idle | WorldActorStatus::Spawned | WorldActorStatus::Failed
    ) {
        return;
    }
    let Some(metadata_handle) = state.metadata.as_ref() else {
        state.status = WorldActorStatus::Failed;
        return;
    };
    if matches!(
        asset_server.load_state(metadata_handle.id()),
        LoadState::Failed(_)
    ) {
        state.status = WorldActorStatus::Failed;
        return;
    }
    let Some(metadata) = metadata_assets.get(metadata_handle) else {
        return;
    };
    let Some(mut game) = game else {
        return;
    };
    let Some(map_id) = state.map_id.as_deref() else {
        return;
    };
    if metadata.effective_id(map_id) != map_id {
        state.status = WorldActorStatus::Failed;
        return;
    }
    let present = present_npcs(metadata, game.flags());

    if state.status == WorldActorStatus::LoadingMetadata {
        state.sprites = present
            .iter()
            .filter_map(|npc| {
                npc.sprite.as_ref().map(|sprite| {
                    (
                        npc.id.clone(),
                        asset_server.load(scenario_root.resolve(sprite)),
                    )
                })
            })
            .collect();
        state.status = WorldActorStatus::LoadingSprites;
        return;
    }
    if state
        .sprites
        .iter()
        .any(|(_, handle)| matches!(asset_server.load_state(handle.id()), LoadState::Failed(_)))
    {
        state.status = WorldActorStatus::Failed;
        return;
    }
    if state
        .sprites
        .iter()
        .any(|(_, handle)| !asset_server.is_loaded_with_dependencies(handle.id()))
    {
        return;
    }
    if !actors.is_empty() {
        state.status = WorldActorStatus::Spawned;
        return;
    }

    for npc in present {
        let sprite = state
            .sprites
            .iter()
            .find(|(id, _)| id == &npc.id)
            .and_then(|(_, handle)| atlas_assets.get(handle))
            .and_then(|atlas| {
                atlas
                    .sprite_for_tile(direction_frame(npc.default_facing, 0))
                    .ok()
            })
            .unwrap_or_else(|| {
                Sprite::from_color(Color::srgb(0.31, 0.63, 0.86), Vec2::splat(64.0))
            });
        let center = tile_center(npc.position);
        let actor = WorldNpc {
            map_id: map_id.to_owned(),
            id: npc.id.clone(),
            name: npc.name.clone(),
            dialogue_id: npc.effective_dialogue_id().to_owned(),
            origin: npc.position,
            position: npc.position,
            facing: npc.default_facing,
            default_facing: npc.default_facing,
            mode: npc.animation.mode,
            speed: npc.animation.speed.get() as f32,
            range: i32::try_from(npc.animation.range.get()).unwrap_or(i32::MAX),
            interaction_range: npc.interaction_range.get() as f32,
            frame: 0,
            frame_elapsed: 0.0,
            wander_pause: random_pause(game.rng_mut()),
            wander_target: None,
        };
        commands.spawn((
            sprite,
            Transform::from_translation(
                center.extend(world_entity_y_z(center.y, SPRITE_HALF_HEIGHT)),
            ),
            actor,
        ));
    }
    state.status = WorldActorStatus::Spawned;
}

fn present_npcs<'a>(
    metadata: &'a MapMetadata,
    flags: &crate::runtime_flags::RuntimeFlags,
) -> Vec<&'a NpcMetadata> {
    metadata
        .npcs
        .iter()
        .filter(|npc| flags.satisfies(&npc.present))
        .collect()
}

fn update_world_npcs(
    time: Res<Time>,
    maps: Res<Assets<TmxGroundAsset>>,
    render: Res<StaticMapRenderState>,
    game: Option<ResMut<GameState>>,
    mut actors: Query<(Entity, &mut WorldNpc, &mut Sprite, &mut Transform)>,
) {
    let Some(mut game) = game else {
        return;
    };
    let Some(map) = render.map(&maps) else {
        return;
    };
    let Ok(collision) = CollisionOccupancy::from_tmx_document(map.document()) else {
        return;
    };
    let player = game.map().position();
    let snapshot = actors
        .iter()
        .map(|(entity, actor, _, _)| (entity, actor.position))
        .collect::<Vec<_>>();
    let delta = time.delta_secs();

    for (entity, mut actor, mut sprite, mut transform) in &mut actors {
        let near = is_near(actor.position, player, actor.interaction_range);
        if near {
            actor.facing = direction_toward(actor.position, player);
            actor.frame = 0;
            actor.wander_target = None;
            actor.wander_pause = random_pause(game.rng_mut());
        } else {
            match actor.mode {
                NpcAnimationMode::Still => {
                    actor.facing = actor.default_facing;
                    actor.frame = 0;
                }
                NpcAnimationMode::Step => advance_frame(&mut actor, delta),
                NpcAnimationMode::Wander => update_wander(
                    &mut actor,
                    delta,
                    entity,
                    player,
                    &snapshot,
                    &collision,
                    game.rng_mut(),
                ),
            }
        }
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = direction_frame(actor.facing, actor.frame) as usize;
        }
        let center = tile_center(actor.position);
        transform.translation = center.extend(world_entity_y_z(center.y, SPRITE_HALF_HEIGHT));
    }
}

fn update_wander(
    actor: &mut WorldNpc,
    delta: f32,
    entity: Entity,
    player: Position,
    snapshot: &[(Entity, Position)],
    collision: &CollisionOccupancy,
    rng: &mut GameplayRng,
) {
    actor.frame_elapsed += delta;
    if actor.wander_target.is_none() {
        actor.wander_pause -= delta;
        actor.frame = 0;
        actor.facing = actor.default_facing;
        if actor.wander_pause <= 0.0 {
            actor.wander_target =
                pick_wander_target(actor, entity, player, snapshot, collision, rng);
            if actor.wander_target.is_none() {
                actor.wander_pause = random_pause(rng);
            }
        }
        return;
    }
    let step_seconds = BASE_FRAME_SECONDS / actor.speed.max(0.1);
    if actor.frame_elapsed < step_seconds {
        return;
    }
    actor.frame_elapsed -= step_seconds;
    let target = actor.wander_target.expect("checked above");
    if actor.position == target {
        actor.wander_target = None;
        actor.wander_pause = random_pause(rng);
        actor.frame = 0;
        return;
    }
    let dx = target.x - actor.position.x;
    let dy = target.y - actor.position.y;
    let (step, facing) = if dx.abs() >= dy.abs() && dx != 0 {
        (
            Position::new(dx.signum(), 0),
            if dx < 0 {
                CardinalDirection::Left
            } else {
                CardinalDirection::Right
            },
        )
    } else {
        (
            Position::new(0, dy.signum()),
            if dy < 0 {
                CardinalDirection::Up
            } else {
                CardinalDirection::Down
            },
        )
    };
    actor.facing = facing;
    let next = Position::new(actor.position.x + step.x, actor.position.y + step.y);
    if occupied(next, entity, player, snapshot) || collision.is_open(next.x, next.y) != Some(true) {
        actor.wander_target = None;
        actor.wander_pause = random_pause(rng);
        actor.frame = 0;
        return;
    }
    actor.position = next;
    actor.frame = if actor.frame >= 8 { 1 } else { actor.frame + 1 };
}

fn pick_wander_target(
    actor: &WorldNpc,
    entity: Entity,
    player: Position,
    snapshot: &[(Entity, Position)],
    collision: &CollisionOccupancy,
    rng: &mut GameplayRng,
) -> Option<Position> {
    let span = u64::try_from(actor.range.saturating_mul(2).saturating_add(1)).ok()?;
    for _ in 0..8 {
        let x = i32::try_from(rng.next_u64() % span).ok()? - actor.range;
        let y = i32::try_from(rng.next_u64() % span).ok()? - actor.range;
        let target = Position::new(actor.origin.x + x, actor.origin.y + y);
        if collision.is_open(target.x, target.y) == Some(true)
            && !occupied(target, entity, player, snapshot)
        {
            return Some(target);
        }
    }
    None
}

fn occupied(
    position: Position,
    entity: Entity,
    player: Position,
    snapshot: &[(Entity, Position)],
) -> bool {
    position == player
        || snapshot
            .iter()
            .any(|(other, occupied)| *other != entity && *occupied == position)
}

fn advance_frame(actor: &mut WorldNpc, delta: f32) {
    actor.frame_elapsed += delta;
    let duration = BASE_FRAME_SECONDS / actor.speed.max(0.1);
    while actor.frame_elapsed >= duration {
        actor.frame_elapsed -= duration;
        actor.frame = if actor.frame >= 8 { 1 } else { actor.frame + 1 };
    }
}

fn random_pause(rng: &mut GameplayRng) -> f32 {
    let fraction = (rng.next_u64() >> 11) as f64 / ((1_u64 << 53) as f64);
    WANDER_PAUSE_MIN + fraction as f32 * WANDER_PAUSE_SPAN
}

fn direction_frame(direction: CardinalDirection, frame: u32) -> u32 {
    let row = match direction {
        CardinalDirection::Up => 0,
        CardinalDirection::Left => 1,
        CardinalDirection::Down => 2,
        CardinalDirection::Right => 3,
    };
    row * 9 + frame
}

fn direction_toward(from: Position, to: Position) -> CardinalDirection {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    if dy.abs() >= dx.abs() {
        if dy < 0 {
            CardinalDirection::Up
        } else {
            CardinalDirection::Down
        }
    } else if dx < 0 {
        CardinalDirection::Left
    } else {
        CardinalDirection::Right
    }
}

fn is_near(npc: Position, player: Position, range: f32) -> bool {
    (npc.x - player.x).abs() as f32 <= range && (npc.y - player.y).abs() as f32 <= range
}

fn tile_center(position: Position) -> Vec2 {
    tmx_tile_center(
        u32::try_from(position.x).unwrap_or_default(),
        u32::try_from(position.y).unwrap_or_default(),
        TILE_SIZE,
        TILE_SIZE,
    )
}

fn cleanup_world_actors(
    mut commands: Commands,
    actors: Query<Entity, With<WorldNpc>>,
    mut state: ResMut<WorldActorState>,
) {
    for entity in &actors {
        commands.entity(entity).despawn();
    }
    *state = WorldActorState::default();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{runtime_flags::RuntimeFlags, scenario_yaml};

    fn ardel_metadata() -> MapMetadata {
        scenario_yaml::from_str(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/maps/town_01_ardel.yaml"
        ))
        .unwrap()
    }

    #[test]
    fn presence_conditions_select_exact_ardel_elise_state() {
        let metadata = ardel_metadata();
        let initial = RuntimeFlags::from_bootstrap(["story_quest_started"]);
        assert!(
            present_npcs(&metadata, &initial)
                .iter()
                .any(|npc| npc.id == "elise")
        );
        let joined = RuntimeFlags::from_bootstrap(["story_quest_started", "npc_elise_joined"]);
        assert!(
            !present_npcs(&metadata, &joined)
                .iter()
                .any(|npc| npc.id == "elise")
        );
        assert_eq!(present_npcs(&metadata, &joined).len(), 5);
    }

    #[test]
    fn static_and_step_frames_keep_authored_facing_rows() {
        assert_eq!(direction_frame(CardinalDirection::Up, 0), 0);
        assert_eq!(direction_frame(CardinalDirection::Left, 1), 10);
        assert_eq!(direction_frame(CardinalDirection::Down, 8), 26);
        assert_eq!(direction_frame(CardinalDirection::Right, 0), 27);
    }

    #[test]
    fn deterministic_wander_targets_stay_bounded_and_avoid_occupants() {
        let xml = r#"<map orientation="orthogonal" width="9" height="9" tilewidth="32" tileheight="32"><layer id="1" name="collision" width="9" height="9"><data encoding="csv">
0,0,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,0
</data></layer></map>"#;
        let owner = ScenarioRelativePath::try_from("assets/maps/invented.tmx").unwrap();
        let document = crate::tmx_header::parse_tmx_map_document(xml, &owner).unwrap();
        let collision = CollisionOccupancy::from_tmx_document(&document).unwrap();
        let mut actor = WorldNpc {
            map_id: "invented".into(),
            id: "npc".into(),
            name: "Npc".into(),
            dialogue_id: "npc".into(),
            origin: Position::new(4, 4),
            position: Position::new(4, 4),
            facing: CardinalDirection::Down,
            default_facing: CardinalDirection::Down,
            mode: NpcAnimationMode::Wander,
            speed: 1.0,
            range: 2,
            interaction_range: 1.5,
            frame: 0,
            frame_elapsed: 0.0,
            wander_pause: 0.0,
            wander_target: None,
        };
        let entity = Entity::from_bits(1);
        let occupied_entity = Entity::from_bits(2);
        let snapshot = [
            (entity, actor.position),
            (occupied_entity, Position::new(5, 5)),
        ];
        let mut first = GameplayRng::from_seed(77);
        let mut second = GameplayRng::from_seed(77);
        let a = pick_wander_target(
            &actor,
            entity,
            Position::new(4, 5),
            &snapshot,
            &collision,
            &mut first,
        )
        .unwrap();
        let b = pick_wander_target(
            &actor,
            entity,
            Position::new(4, 5),
            &snapshot,
            &collision,
            &mut second,
        )
        .unwrap();
        assert_eq!(a, b);
        assert!((a.x - actor.origin.x).abs() <= actor.range);
        assert!((a.y - actor.origin.y).abs() <= actor.range);
        assert_ne!(a, Position::new(4, 5));
        assert_ne!(a, Position::new(5, 5));
        actor.wander_target = Some(a);
    }
}
