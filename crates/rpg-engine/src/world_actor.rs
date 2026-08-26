//! Source-authored World NPC loading, presence, occupancy, animation, and bounded wandering.

use bevy::{asset::LoadState, ecs::schedule::ApplyDeferred, prelude::*};

use crate::{
    app_state::AppState,
    game_state::GameState,
    gameplay_rng::GameplayRng,
    scenario_map::{
        MapMetadata, NpcAnimationMode, NpcMetadata, optional_scenario_asset_is_missing,
    },
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
///
/// # Placement convention
///
/// Unlike the player and world-map enemies (`world_player`, `world_encounter`), which place
/// their sprite's collision-box *center* on the authored tile's center (source
/// `engine/world/player.py` / `engine/encounter/enemy_sprite.py`, both computing
/// `tile*ts + ts/2 - collision_offset - collision_size/2`), a source NPC (`engine/world/npc.py`)
/// stores its pixel origin as the *plain* authored tile corner: `_origin_px = tile_x * tile_size`
/// with no collision centering at all. `top_left` mirrors that: see `character_top_left`.
///
/// One consequence the source embraces: an NPC's `collision_rect` (`_px + 22, _py + 41, 20, 18`)
/// sits with its center near `(tile+1, tile+1)`, not on the authored tile itself — the NPC's feet
/// are drawn at the authored tile's corner, so its collision box lands roughly a tile
/// southeast. `origin` and `position` below give the two tiles that distinction implies:
///
/// - `origin` is the *authored anchor* tile (`npc.position` from the map YAML, unconverted). It
///   never changes after spawn and is what wander target selection re-centers on, matching the
///   source's `_origin_px/_origin_py` (see `pick_wander_target`).
/// - `position` is the tile the NPC's collision box *currently occupies* — i.e.
///   `tile_from_top_left(top_left)`, recomputed the same way at spawn and after every wander
///   step, so the two call sites can never disagree. For an unmoved NPC this is `origin + (1,
///   1)`, by the same geometry as `collision_rect` above; it is intentionally not "the authored
///   tile" once occupancy or interaction distance ties need the box the player can actually
///   bump into.
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

    /// The pixel position the source uses for interaction facing/range checks
    /// (`Npc.pixel_position` in `engine/world/npc.py`, i.e. `(self._px, self._py)`). Since
    /// `top_left` already stores that same uncentered origin (see the type doc), this is just
    /// `top_left` — no compensating offset needed.
    pub(crate) fn source_pixel_position(&self) -> Vec2 {
        self.top_left
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
            state.status = WorldActorStatus::Failed;
            return;
        }
        _ => {
            let Some(metadata) = metadata_assets.get(metadata_handle) else {
                return;
            };
            metadata
        }
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
            position: tile_from_top_left(top_left),
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

/// Tile whose center the NPC's collision box currently sits closest to. Used both at spawn and
/// after every wander step (see the `WorldNpc` type doc for why these must — and do — agree).
fn tile_from_top_left(top_left: Vec2) -> Position {
    let rect = character_collision_rect(top_left);
    Position::new(
        ((rect.x + rect.width / 2.0) / TILE_SIZE as f32).floor() as i32,
        ((rect.y + rect.height / 2.0) / TILE_SIZE as f32).floor() as i32,
    )
}

/// An NPC's sprite top-left for an authored tile: the *plain* tile corner, `tile * TILE_SIZE`,
/// with no collision centering. This matches the source's `Npc.__init__` (`engine/world/npc.py`):
/// `_origin_px = tile_x * tile_size; _origin_py = tile_y * tile_size`, blitted directly as the
/// sprite's top-left in `Npc.render`. This is deliberately *not* the player/enemy formula
/// (`world_player::character_top_left`, `world_encounter::enemy_top_left`), which centers the
/// collision box on the tile instead — the source's player and NPC classes use two different
/// placement conventions, and porting the player's formula onto NPCs was the original bug (NPCs
/// rendering a tile too high/left).
fn character_top_left(position: Position) -> Vec2 {
    Vec2::new(
        position.x as f32 * TILE_SIZE as f32,
        position.y as f32 * TILE_SIZE as f32,
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
            "../../../assets/scenarios/rusted_kingdoms/data/maps/town_01_ardel.yaml"
        ))
        .unwrap()
    }

    fn millhaven_metadata() -> MapMetadata {
        scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/data/maps/town_02_millhaven.yaml"
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
        let top_left = character_top_left(Position::new(4, 4));
        WorldNpc {
            map_id: "invented".into(),
            name: "Npc".into(),
            dialogue_id: "npc".into(),
            origin: Position::new(4, 4),
            // Occupied tile, derived the same way spawn does — see the `WorldNpc` type doc. For
            // an unmoved NPC this is `origin + (1, 1)`, not `origin` itself.
            position: tile_from_top_left(top_left),
            top_left,
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
    fn presence_conditions_select_exact_millhaven_reiya_state() {
        let metadata = millhaven_metadata();
        assert_eq!(metadata.npcs.len(), 6);

        // Fresh flags: Reiya's `present` clause only excludes the joined flag, so she is out
        // wandering town at [12, 7] with no story gate, unlike the ledger's initial assumption.
        let fresh = RuntimeFlags::default();
        let fresh_present = present_npcs(&metadata, &fresh);
        assert_eq!(fresh_present.len(), 6);
        let reiya = fresh_present
            .iter()
            .find(|npc| npc.id == "reiya")
            .expect("reiya is present under fresh flags");
        assert_eq!(reiya.position, Position::new(12, 7));

        // Starting Act II does not itself gate Reiya's presence; she stays until recruited.
        let act2 = RuntimeFlags::from_bootstrap(["story_act2_started"]);
        assert!(
            present_npcs(&metadata, &act2)
                .iter()
                .any(|npc| npc.id == "reiya")
        );
        assert_eq!(present_npcs(&metadata, &act2).len(), 6);

        // Once recruited, Reiya is the only NPC to disappear from the spawn set.
        let joined = RuntimeFlags::from_bootstrap(["story_act2_started", "npc_reiya_joined"]);
        let joined_present = present_npcs(&metadata, &joined);
        assert_eq!(joined_present.len(), 5);
        assert!(!joined_present.iter().any(|npc| npc.id == "reiya"));
        assert_eq!(
            joined_present
                .iter()
                .map(|npc| npc.id.as_str())
                .collect::<Vec<_>>(),
            [
                "millhaven_elder",
                "millhaven_baker",
                "millhaven_granary",
                "millhaven_carter",
                "millhaven_gossip",
            ]
        );
    }

    fn harborgate_metadata() -> MapMetadata {
        scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/data/maps/port_town_harborgate.yaml"
        ))
        .unwrap()
    }

    fn harborgate_quarantine_metadata() -> MapMetadata {
        scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/data/maps/port_town_harborgate_quarantine.yaml"
        ))
        .unwrap()
    }

    fn harborgate_shop_metadata() -> MapMetadata {
        scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/data/maps/port_town_harborgate_shop.yaml"
        ))
        .unwrap()
    }

    /// Unlike Millhaven's Reiya (`excludes: [npc_reiya_joined]`), none of the five authored
    /// Harborgate town NPCs carry a `present:` clause at all — the production spawn set is the
    /// full authored roster under every flag state. This fixture proves the exact ids, positions,
    /// and dialogue references `world_actor::present_npcs` resolves, and that the set is stable
    /// across fresh, Act II, and Act III flags (i.e. no accidental gating slipped into the port).
    #[test]
    fn presence_conditions_keep_the_full_harborgate_town_roster_present_across_story_flags() {
        let metadata = harborgate_metadata();
        assert_eq!(metadata.npcs.len(), 5);

        let expected = [
            (
                "harborgate_dockhand",
                Position::new(21, 14),
                "harborgate_dockhand",
            ),
            ("harborgate_clerk", Position::new(12, 7), "harborgate_clerk"),
            (
                "harborgate_stevedore",
                Position::new(30, 9),
                "harborgate_stevedore",
            ),
            (
                "harborgate_sailor",
                Position::new(33, 17),
                "harborgate_sailor",
            ),
            (
                "harborgate_fishwife",
                Position::new(5, 16),
                "harborgate_fishwife",
            ),
        ];

        for flags in [
            RuntimeFlags::default(),
            RuntimeFlags::from_bootstrap(["story_act2_started"]),
            RuntimeFlags::from_bootstrap([
                "story_act2_started",
                "story_act3_started",
                "transport_sail_unlocked",
                "sq_manifest_done",
                "sq_catch_done",
            ]),
        ] {
            let present = present_npcs(&metadata, &flags);
            assert_eq!(present.len(), 5);
            let observed = present
                .iter()
                .map(|npc| {
                    (
                        npc.id.as_str(),
                        npc.position,
                        npc.dialogue.as_deref().unwrap_or_default(),
                    )
                })
                .collect::<Vec<_>>();
            assert_eq!(observed, expected);
        }
    }

    /// The quarantine interior's two NPCs (priestess and patient) are likewise ungated: both are
    /// present under fresh flags and remain present once the story flags they narrate over
    /// (Act II/III) are set.
    #[test]
    fn presence_conditions_keep_both_quarantine_npcs_present_across_story_flags() {
        let metadata = harborgate_quarantine_metadata();
        assert_eq!(metadata.npcs.len(), 2);

        let expected = [
            (
                "quarantine_priestess",
                Position::new(8, 3),
                "harborgate_priestess",
            ),
            (
                "quarantine_patient",
                Position::new(14, 5),
                "harborgate_patient",
            ),
        ];

        for flags in [
            RuntimeFlags::default(),
            RuntimeFlags::from_bootstrap(["story_act2_started"]),
            RuntimeFlags::from_bootstrap(["story_act3_started"]),
        ] {
            let present = present_npcs(&metadata, &flags);
            assert_eq!(present.len(), 2);
            let observed = present
                .iter()
                .map(|npc| {
                    (
                        npc.id.as_str(),
                        npc.position,
                        npc.dialogue.as_deref().unwrap_or_default(),
                    )
                })
                .collect::<Vec<_>>();
            assert_eq!(observed, expected);
        }
    }

    /// The shop interior's four keepers (item, magic-core, weapon, armor) are likewise ungated —
    /// only their shop inventories carry `unlock_flag` gates (exercised via `open_shop`/service
    /// routing, not NPC presence). All four spawn under every flag state.
    #[test]
    fn presence_conditions_keep_all_four_shop_keepers_present_across_story_flags() {
        let metadata = harborgate_shop_metadata();
        assert_eq!(metadata.npcs.len(), 4);

        let expected = [
            (
                "item_shop_keeper",
                Position::new(6, 3),
                "item_shop_harborgate",
            ),
            ("mc_shop_keeper", Position::new(10, 3), "mc_shop_intro"),
            (
                "weapon_shop_keeper",
                Position::new(4, 3),
                "weapon_shop_harborgate",
            ),
            (
                "armor_shop_keeper",
                Position::new(12, 3),
                "armor_shop_harborgate",
            ),
        ];

        for flags in [
            RuntimeFlags::default(),
            RuntimeFlags::from_bootstrap(["story_quest_started"]),
            RuntimeFlags::from_bootstrap(["story_act2_started", "story_act3_started"]),
        ] {
            let present = present_npcs(&metadata, &flags);
            assert_eq!(present.len(), 4);
            let observed = present
                .iter()
                .map(|npc| {
                    (
                        npc.id.as_str(),
                        npc.position,
                        npc.dialogue.as_deref().unwrap_or_default(),
                    )
                })
                .collect::<Vec<_>>();
            assert_eq!(observed, expected);
        }
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
        // Occupied tile is `origin + (1, 1)` under the source-matching convention (see the
        // `WorldNpc` type doc); the tiny sub-tile move here doesn't cross into a new one.
        assert_eq!(actor.position, Position::new(5, 5));
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

    /// Pins the source placement convention documented on `WorldNpc`
    /// (`engine/world/npc.py::Npc.__init__`/`render`/`collision_rect`) against the exact pixel
    /// values the Python engine computes for an authored NPC tile, verifying render, collision,
    /// and interaction geometry all agree. This is the regression test for the "NPCs render one
    /// cell too high" bug: `character_top_left` used to apply the player's collision-centered
    /// formula to NPCs (`tile*32 - 16, tile*32 - 34`), which shifted the drawn sprite up/left of
    /// the source.
    #[test]
    fn npc_top_left_render_and_collision_match_source_pixel_values() {
        let tile = Position::new(6, 9);
        let top_left = character_top_left(tile);

        // `Npc._origin_px = tile_x * tile_size; _origin_py = tile_y * tile_size` — the plain tile
        // corner, no collision centering.
        assert_eq!(top_left, Vec2::new(6.0 * 32.0, 9.0 * 32.0));

        // `Npc.render` blits the scaled 64x64 sprite's top-left at `(_px, _py)`, so the sprite
        // center — what the port's `Transform` uses — is the top-left plus half the sprite size.
        let render_center = top_left + Vec2::splat(CHARACTER_SPRITE_SIZE / 2.0);
        assert_eq!(render_center, top_left + Vec2::new(32.0, 32.0));

        // `Npc.collision_rect = (_px + 22, _py + 41, 20, 18)`.
        let collision_rect = character_collision_rect(top_left);
        assert_eq!(collision_rect.x, top_left.x + 22.0);
        assert_eq!(collision_rect.y, top_left.y + 41.0);
        assert_eq!(collision_rect.width, 20.0);
        assert_eq!(collision_rect.height, 18.0);

        // `Npc.pixel_position` (used for interaction facing/range checks, e.g.
        // `world_map_logic.try_interact`) is `(_px, _py)` — i.e. exactly `top_left`. This is also
        // the value the *old*, buggy `character_top_left` produced after
        // `source_pixel_position`'s now-deleted compensating offset (`+16, +34`) was added back
        // on, so interaction selection is unchanged by this fix even though rendering moved.
        let npc = WorldNpc {
            map_id: "invented".into(),
            name: "Npc".into(),
            dialogue_id: "npc".into(),
            origin: tile,
            position: tile_from_top_left(top_left),
            top_left,
            facing: CardinalDirection::Down,
            default_facing: CardinalDirection::Down,
            mode: NpcAnimationMode::Still,
            speed: 1.0,
            range: 2,
            interaction_range: 1.5,
            frame: 0,
            frame_elapsed: 0.0,
            wander_pause: 0.0,
            wander_target: None,
        };
        assert_eq!(npc.source_pixel_position(), top_left);

        // `tile_from_top_left` — the tile the collision box occupies — lands one tile southeast
        // of the authored anchor for an unmoved NPC, matching the source's own geometry (the
        // collision rect's center sits near `_px + 32, _py + 50`). `WorldNpc::position` is seeded
        // with exactly this value at spawn, so it agrees with what wander recomputes later.
        assert_eq!(npc.tile_position(), Position::new(tile.x + 1, tile.y + 1));
    }
}
