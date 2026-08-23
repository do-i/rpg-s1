//! Source-authored World NPC loading, presence, occupancy, animation, and bounded wandering.

use bevy::{asset::LoadState, ecs::schedule::ApplyDeferred, prelude::*};

use crate::{
    app_state::AppState,
    game_state::GameState,
    gameplay_rng::GameplayRng,
    scenario_map::{MapMetadata, NpcAnimationMode, NpcMetadata},
    scenario_path::ScenarioRelativePath,
    scenario_root::ScenarioRoot,
    scenario_spatial::{
        CardinalDirection, Position, collision_occupancy::CollisionOccupancy,
        world_collision::WorldCollision,
    },
    tmx_ground_asset::world_entity_y_z,
    tsx_atlas_asset::TsxAtlasAsset,
    world_player::{
        CHARACTER_COLLISION_HEIGHT, CHARACTER_COLLISION_OFFSET_X, CHARACTER_COLLISION_OFFSET_Y,
        CHARACTER_COLLISION_WIDTH, CHARACTER_SPRITE_SIZE, CharacterCollisionRect,
        WorldPlayerMotion,
    },
};

const TILE_SIZE: u32 = 32;
const SPRITE_HALF_HEIGHT: f32 = 32.0;
const BASE_FRAME_SECONDS: f32 = 0.15;
const WANDER_PIXELS_PER_SECOND: f32 = 60.0;
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
pub(crate) struct WorldNpc {
    map_id: String,
    name: String,
    dialogue_id: String,
    origin: Position,
    position: Position,
    top_left: Vec2,
    facing: CardinalDirection,
    default_facing: CardinalDirection,
    mode: NpcAnimationMode,
    speed: f32,
    range: i32,
    interaction_range: f32,
    frame: u32,
    frame_elapsed: f32,
    wander_pause: f32,
    wander_target: Option<Vec2>,
}

impl WorldNpc {
    pub(crate) fn map_id(&self) -> &str {
        &self.map_id
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

    pub(crate) fn source_pixel_position(&self) -> Vec2 {
        self.top_left
            + Vec2::new(
                CHARACTER_COLLISION_OFFSET_X + CHARACTER_COLLISION_WIDTH / 2.0
                    - TILE_SIZE as f32 / 2.0,
                CHARACTER_COLLISION_OFFSET_Y + CHARACTER_COLLISION_HEIGHT / 2.0
                    - TILE_SIZE as f32 / 2.0,
            )
    }

    pub(crate) fn interaction_range_pixels(&self) -> f32 {
        self.interaction_range * TILE_SIZE as f32
    }

    pub(crate) fn collision_rect(&self) -> CharacterCollisionRect {
        CharacterCollisionRect {
            x: self.top_left.x + CHARACTER_COLLISION_OFFSET_X,
            y: self.top_left.y + CHARACTER_COLLISION_OFFSET_Y,
            width: CHARACTER_COLLISION_WIDTH,
            height: CHARACTER_COLLISION_HEIGHT,
        }
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
        let top_left = character_top_left(npc.position);
        let center = top_left + Vec2::splat(CHARACTER_SPRITE_SIZE / 2.0);
        let actor = WorldNpc {
            map_id: map_id.to_owned(),
            name: npc.name.clone(),
            dialogue_id: npc.effective_dialogue_id().to_owned(),
            origin: npc.position,
            position: npc.position,
            top_left,
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
            Transform::from_translation(Vec3::new(
                center.x,
                -center.y,
                world_entity_y_z(-center.y, SPRITE_HALF_HEIGHT),
            )),
            actor,
        ));
    }
    state.status = WorldActorStatus::Spawned;
}

/// Filters a map's authored NPCs to the ones the given flag state would spawn.
///
/// `pub(crate)` so [`crate::scenario_map_sweep`] can headlessly replay the exact production
/// spawn-set derivation across several flag states, instead of duplicating this filter.
pub(crate) fn present_npcs<'a>(
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
    collision: Option<Res<WorldCollision>>,
    game: Option<ResMut<GameState>>,
    players: Query<&WorldPlayerMotion>,
    mut actors: Query<(Entity, &mut WorldNpc, &mut Sprite, &mut Transform)>,
    mut snapshot: Local<Vec<(Entity, CharacterCollisionRect)>>,
) {
    let Some(mut game) = game else {
        return;
    };
    let Some(collision) = collision.as_deref() else {
        return;
    };
    let player_tile = game.map().position();
    let player_facing = game.map().facing();
    let Some(map_id) = game.map().current().map(|map| map.as_str()) else {
        return;
    };
    let Some(collision) = collision.occupancy_for(map_id) else {
        return;
    };
    let player_motion = players
        .single()
        .copied()
        .unwrap_or_else(|_| WorldPlayerMotion::from_tile(player_tile));
    let player_rect = player_motion.collision_rect();
    snapshot.clear();
    snapshot.extend(
        actors
            .iter()
            .map(|(entity, actor, _, _)| (entity, actor.collision_rect())),
    );
    let delta = time.delta_secs();

    for (entity, mut actor, mut sprite, mut transform) in &mut actors {
        let notices_player = notices_player(
            actor.top_left,
            actor.facing,
            player_motion.top_left(),
            player_facing,
            actor.interaction_range * TILE_SIZE as f32,
        );
        if notices_player {
            actor.facing = direction_toward(actor.top_left, player_motion.top_left());
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
                    player_rect,
                    snapshot.as_slice(),
                    collision,
                    game.rng_mut(),
                ),
            }
        }
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = direction_frame(actor.facing, actor.frame) as usize;
        }
        let center = actor.top_left + Vec2::splat(CHARACTER_SPRITE_SIZE / 2.0);
        transform.translation = Vec3::new(
            center.x,
            -center.y,
            world_entity_y_z(-center.y, SPRITE_HALF_HEIGHT),
        );
    }
}

fn update_wander(
    actor: &mut WorldNpc,
    delta: f32,
    entity: Entity,
    player: CharacterCollisionRect,
    snapshot: &[(Entity, CharacterCollisionRect)],
    collision: &CollisionOccupancy,
    rng: &mut GameplayRng,
) {
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
    let target = actor.wander_target.expect("checked above");
    let delta_to_target = target - actor.top_left;
    let distance = delta_to_target.abs().max_element();
    let next = if distance <= WANDER_PIXELS_PER_SECOND * delta {
        target
    } else {
        let step = WANDER_PIXELS_PER_SECOND * delta;
        actor.top_left
            + Vec2::new(
                if delta_to_target.x == 0.0 {
                    0.0
                } else {
                    delta_to_target.x.signum() * step
                },
                if delta_to_target.y == 0.0 {
                    0.0
                } else {
                    delta_to_target.y.signum() * step
                },
            )
    };
    actor.facing = if delta_to_target.x.abs() >= delta_to_target.y.abs() && delta_to_target.x != 0.0
    {
        if delta_to_target.x < 0.0 {
            CardinalDirection::Left
        } else {
            CardinalDirection::Right
        }
    } else if delta_to_target.y < 0.0 {
        CardinalDirection::Up
    } else {
        CardinalDirection::Down
    };
    let next_rect = character_collision_rect(next);
    if collision.is_rect_blocked(next_rect.x, next_rect.y, next_rect.width, next_rect.height)
        || occupied(next_rect, entity, player, snapshot)
    {
        actor.wander_target = None;
        actor.wander_pause = random_pause(rng);
        actor.frame = 0;
        return;
    }
    actor.top_left = next;
    actor.position = tile_from_top_left(next);
    advance_frame(actor, delta);
    if next == target {
        actor.wander_target = None;
        actor.wander_pause = random_pause(rng);
        actor.frame = 0;
    }
}

fn pick_wander_target(
    actor: &WorldNpc,
    entity: Entity,
    player: CharacterCollisionRect,
    snapshot: &[(Entity, CharacterCollisionRect)],
    collision: &CollisionOccupancy,
    rng: &mut GameplayRng,
) -> Option<Vec2> {
    let max_offset = actor.range.saturating_mul(TILE_SIZE as i32);
    let span = u64::try_from(max_offset.saturating_mul(2).saturating_add(1)).ok()?;
    for _ in 0..8 {
        let x = i32::try_from(rng.next_u64() % span).ok()? - max_offset;
        let y = i32::try_from(rng.next_u64() % span).ok()? - max_offset;
        let target = character_top_left(actor.origin) + Vec2::new(x as f32, y as f32);
        let target_rect = character_collision_rect(target);
        if !collision.is_rect_blocked(
            target_rect.x,
            target_rect.y,
            target_rect.width,
            target_rect.height,
        ) && !occupied(target_rect, entity, player, snapshot)
        {
            return Some(target);
        }
    }
    None
}

fn occupied(
    rect: CharacterCollisionRect,
    entity: Entity,
    player: CharacterCollisionRect,
    snapshot: &[(Entity, CharacterCollisionRect)],
) -> bool {
    rect.overlaps(player)
        || snapshot
            .iter()
            .any(|(other, occupied)| *other != entity && rect.overlaps(*occupied))
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

fn direction_toward(from: Vec2, to: Vec2) -> CardinalDirection {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    if dy.abs() >= dx.abs() {
        if dy < 0.0 {
            CardinalDirection::Up
        } else {
            CardinalDirection::Down
        }
    } else if dx < 0.0 {
        CardinalDirection::Left
    } else {
        CardinalDirection::Right
    }
}

fn is_near(npc: Vec2, player: Vec2, range: f32) -> bool {
    (npc.x - player.x).abs() <= range && (npc.y - player.y).abs() <= range
}

fn notices_player(
    npc: Vec2,
    npc_facing: CardinalDirection,
    player: Vec2,
    player_facing: CardinalDirection,
    range: f32,
) -> bool {
    is_near(npc, player, range)
        && is_facing_toward(npc, player, npc_facing)
        && is_facing_toward(player, npc, player_facing)
}

fn is_facing_toward(from: Vec2, to: Vec2, facing: CardinalDirection) -> bool {
    let delta = to - from;
    match facing {
        CardinalDirection::Up => delta.y < 0.0,
        CardinalDirection::Down => delta.y > 0.0,
        CardinalDirection::Left => delta.x < 0.0,
        CardinalDirection::Right => delta.x > 0.0,
    }
}

fn character_collision_rect(top_left: Vec2) -> CharacterCollisionRect {
    CharacterCollisionRect {
        x: top_left.x + CHARACTER_COLLISION_OFFSET_X,
        y: top_left.y + CHARACTER_COLLISION_OFFSET_Y,
        width: CHARACTER_COLLISION_WIDTH,
        height: CHARACTER_COLLISION_HEIGHT,
    }
}

fn tile_from_top_left(top_left: Vec2) -> Position {
    let rect = character_collision_rect(top_left);
    Position::new(
        ((rect.x + rect.width / 2.0) / TILE_SIZE as f32).floor() as i32,
        ((rect.y + rect.height / 2.0) / TILE_SIZE as f32).floor() as i32,
    )
}

fn character_top_left(position: Position) -> Vec2 {
    Vec2::new(
        position.x as f32 * TILE_SIZE as f32 + TILE_SIZE as f32 / 2.0
            - (CHARACTER_COLLISION_OFFSET_X + CHARACTER_COLLISION_WIDTH / 2.0),
        position.y as f32 * TILE_SIZE as f32 + TILE_SIZE as f32 / 2.0
            - (CHARACTER_COLLISION_OFFSET_Y + CHARACTER_COLLISION_HEIGHT / 2.0),
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

    fn open_collision() -> CollisionOccupancy {
        let rows = std::iter::repeat_n("0,0,0,0,0,0,0,0,0", 9)
            .collect::<Vec<_>>()
            .join(",\n");
        let xml = format!(
            r#"<map orientation="orthogonal" width="9" height="9" tilewidth="32" tileheight="32"><layer id="1" name="collision" width="9" height="9"><data encoding="csv">{rows}</data></layer></map>"#
        );
        let owner = ScenarioRelativePath::try_from("assets/maps/invented.tmx").unwrap();
        let document = crate::tmx_header::parse_tmx_map_document(&xml, &owner).unwrap();
        CollisionOccupancy::from_tmx_document(&document).unwrap()
    }

    fn wandering_actor(speed: f32) -> WorldNpc {
        WorldNpc {
            map_id: "invented".into(),
            name: "Npc".into(),
            dialogue_id: "npc".into(),
            origin: Position::new(4, 4),
            position: Position::new(4, 4),
            top_left: character_top_left(Position::new(4, 4)),
            facing: CardinalDirection::Down,
            default_facing: CardinalDirection::Down,
            mode: NpcAnimationMode::Wander,
            speed,
            range: 2,
            interaction_range: 1.5,
            frame: 0,
            frame_elapsed: 0.0,
            wander_pause: 0.0,
            wander_target: None,
        }
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
        let collision = open_collision();
        let mut actor = wandering_actor(1.0);
        let entity = Entity::from_bits(1);
        let occupied_entity = Entity::from_bits(2);
        let snapshot = [
            (entity, actor.collision_rect()),
            (
                occupied_entity,
                character_collision_rect(character_top_left(Position::new(5, 5))),
            ),
        ];
        let player = character_collision_rect(character_top_left(Position::new(4, 5)));
        let mut first = GameplayRng::from_seed(77);
        let mut second = GameplayRng::from_seed(77);
        let a =
            pick_wander_target(&actor, entity, player, &snapshot, &collision, &mut first).unwrap();
        let b =
            pick_wander_target(&actor, entity, player, &snapshot, &collision, &mut second).unwrap();
        assert_eq!(a, b);
        let origin = character_top_left(actor.origin);
        assert!((a.x - origin.x).abs() <= (actor.range * TILE_SIZE as i32) as f32);
        assert!((a.y - origin.y).abs() <= (actor.range * TILE_SIZE as i32) as f32);
        assert!(!character_collision_rect(a).overlaps(player));
        actor.wander_target = Some(a);
    }

    #[test]
    fn wandering_npc_moves_fractionally_toward_target_instead_of_teleporting() {
        let collision = open_collision();
        let mut actor = wandering_actor(1.0);
        let start = actor.top_left;
        actor.wander_target = Some(start + Vec2::new(TILE_SIZE as f32, 0.0));
        let entity = Entity::from_bits(1);
        let snapshot = [(entity, actor.collision_rect())];
        let player = character_collision_rect(character_top_left(Position::new(8, 8)));
        let mut rng = GameplayRng::from_seed(9);

        update_wander(
            &mut actor,
            1.0 / 60.0,
            entity,
            player,
            &snapshot,
            &collision,
            &mut rng,
        );

        assert!(actor.top_left.x > start.x);
        assert!(actor.top_left.x < start.x + TILE_SIZE as f32);
        assert_eq!(actor.top_left.y, start.y);
        assert_eq!(actor.position, Position::new(4, 4));
        assert_eq!(actor.facing, CardinalDirection::Right);
        assert!(actor.wander_target.is_some());
    }

    #[test]
    fn animation_speed_does_not_accelerate_wander_travel() {
        let collision = open_collision();
        let mut normal = wandering_actor(1.0);
        let mut fast_animation = wandering_actor(2.2);
        let start = normal.top_left;
        let target = start + Vec2::new(100.0, 0.0);
        normal.wander_target = Some(target);
        fast_animation.wander_target = Some(target);
        let entity = Entity::from_bits(1);
        let snapshot = [(entity, normal.collision_rect())];
        let player = character_collision_rect(character_top_left(Position::new(8, 8)));
        let mut normal_rng = GameplayRng::from_seed(9);
        let mut fast_rng = GameplayRng::from_seed(9);

        update_wander(
            &mut normal,
            0.1,
            entity,
            player,
            &snapshot,
            &collision,
            &mut normal_rng,
        );
        update_wander(
            &mut fast_animation,
            0.1,
            entity,
            player,
            &snapshot,
            &collision,
            &mut fast_rng,
        );

        assert_eq!(normal.top_left, start + Vec2::new(6.0, 0.0));
        assert_eq!(fast_animation.top_left, normal.top_left);
        assert_eq!(normal.frame, 0);
        assert_eq!(fast_animation.frame, 1);
    }

    #[test]
    fn npc_only_notices_a_mutually_facing_player() {
        let npc = Vec2::new(100.0, 100.0);
        let player = Vec2::new(100.0, 120.0);

        assert!(notices_player(
            npc,
            CardinalDirection::Down,
            player,
            CardinalDirection::Up,
            48.0,
        ));
        assert!(!notices_player(
            npc,
            CardinalDirection::Up,
            player,
            CardinalDirection::Up,
            48.0,
        ));
        assert!(!notices_player(
            npc,
            CardinalDirection::Down,
            player,
            CardinalDirection::Down,
            48.0,
        ));
        assert!(!notices_player(
            npc,
            CardinalDirection::Down,
            player,
            CardinalDirection::Up,
            19.0,
        ));
    }
}
