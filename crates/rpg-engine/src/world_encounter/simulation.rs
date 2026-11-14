//! Moves World enemies: chasing, wandering, respawning, and their sprite geometry.

use bevy::{asset::Assets, ecs::system::SystemParam, prelude::*};

use crate::{
    encounter::party_encounter_modifiers,
    game_state::GameState,
    scenario_balance::{BalanceData, WorldEnemyBalance},
    scenario_new_game_assets::ActiveNewGameInputs,
    scenario_spatial::{
        CardinalDirection, Position,
        collision_occupancy::CollisionOccupancy,
        world_collision::{WorldCollision, character_collision_rect},
    },
    tile_coordinates::TILE_SIZE,
    tmx_ground_asset::world_entity_y_z,
    world_player::{
        CHARACTER_COLLISION_HEIGHT, CHARACTER_COLLISION_OFFSET_X, CHARACTER_COLLISION_OFFSET_Y,
        CHARACTER_COLLISION_WIDTH, CHARACTER_SPRITE_SIZE, CharacterCollisionRect, WorldPlayer,
        WorldPlayerMotion,
    },
};

use super::state::{WorldEncounterState, WorldEncounterStatus, WorldEnemy};

pub(super) const ENEMY_SPRITE_HALF_HEIGHT: f32 = 32.0;
pub(super) const ENEMY_WALK_ROW_OFFSET: u32 = 8;
pub(super) const ENEMY_ATLAS_COLUMNS: u32 = 9;
pub(super) const ENEMY_FRAME_SECONDS: f32 = 0.15;
pub(super) const MAX_ENEMY_DELTA_SECONDS: f32 = 0.1;

/// World pixels per second an enemy covers while chasing or wandering.
fn enemy_move_speed_pixels(tuning: WorldEnemyBalance) -> f32 {
    TILE_SIZE as f32 * tuning.move_speed_tiles_per_second.get() as f32
}

#[derive(SystemParam)]
pub(super) struct EnemySimulationAssets<'w> {
    collision: Option<Res<'w, WorldCollision>>,
    balances: Res<'w, Assets<BalanceData>>,
    active: Res<'w, ActiveNewGameInputs>,
}

#[expect(
    clippy::too_many_arguments,
    reason = "world enemy simulation reads independent timing, map, party, and collision state"
)]
pub(super) fn update_world_enemies(
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
        .active
        .balance(&assets.balances)
        .map(|balance| party_encounter_modifiers(game.party(), &balance.spawner))
        .unwrap_or_else(|| {
            party_encounter_modifiers(game.party(), &BalanceData::default().spawner)
        });
    let tuning = assets
        .active
        .balance(&assets.balances)
        .map_or_else(WorldEnemyBalance::default, |balance| balance.world_enemy);
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
                let index = (game.rng_mut().below(inactive.len() as u64)) as usize;
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
                    tuning,
                )
            } else {
                wander_enemy(
                    &mut enemy,
                    entity,
                    delta,
                    snapshot.as_slice(),
                    collision,
                    game.rng_mut(),
                    tuning,
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

pub(super) fn chase_enemy(
    enemy: &mut WorldEnemy,
    entity: Entity,
    player_top_left: Vec2,
    delta: f32,
    snapshot: &[(Entity, CharacterCollisionRect, bool)],
    collision: &CollisionOccupancy,
    tuning: WorldEnemyBalance,
) -> bool {
    let toward_player = player_top_left - enemy.top_left;
    let distance = toward_player.length();
    if distance < 1.0 {
        return false;
    }
    enemy.facing = direction_toward(enemy.top_left, player_top_left);
    let step = (enemy_move_speed_pixels(tuning) * delta).min(distance);
    try_move_enemy(
        enemy,
        entity,
        enemy.top_left + toward_player / distance * step,
        snapshot,
        collision,
    )
}

pub(super) fn wander_enemy(
    enemy: &mut WorldEnemy,
    entity: Entity,
    delta: f32,
    snapshot: &[(Entity, CharacterCollisionRect, bool)],
    collision: &CollisionOccupancy,
    rng: &mut crate::gameplay_rng::GameplayRng,
    tuning: WorldEnemyBalance,
) -> bool {
    if enemy.wander_target.is_none() {
        enemy.wander_pause -= delta;
        enemy.frame = 0;
        if enemy.wander_pause <= 0.0 {
            enemy.wander_target =
                pick_wander_target(enemy, entity, snapshot, collision, rng, tuning);
            if enemy.wander_target.is_none() {
                enemy.wander_pause = random_pause(rng);
            }
        }
        return false;
    }
    let target = enemy
        .wander_target
        .expect("the wander_target.is_none() branch returned");
    let toward_target = target - enemy.top_left;
    let distance = toward_target.abs().max_element();
    let move_distance = enemy_move_speed_pixels(tuning) * delta;
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

pub(super) fn pick_wander_target(
    enemy: &WorldEnemy,
    entity: Entity,
    snapshot: &[(Entity, CharacterCollisionRect, bool)],
    collision: &CollisionOccupancy,
    rng: &mut crate::gameplay_rng::GameplayRng,
    tuning: WorldEnemyBalance,
) -> Option<Vec2> {
    // `wander_range_tiles` is validated to at most 1024 when the balance loads, so this cannot
    // overflow.
    let range_pixels: i32 = tuning.wander_range_tiles.get() as i32 * TILE_SIZE as i32;
    let span: u64 = (range_pixels as u64 * 2) + 1;
    let origin = enemy_top_left(enemy.origin);
    for _ in 0..8 {
        let x = i32::try_from(rng.below(span)).ok()? - range_pixels;
        let y = i32::try_from(rng.below(span)).ok()? - range_pixels;
        let target = origin + Vec2::new(x as f32, y as f32);
        if !enemy_blocked(target, entity, snapshot, collision) {
            return Some(target);
        }
    }
    None
}

pub(super) fn try_move_enemy(
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

pub(super) fn enemy_blocked(
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

pub(super) fn advance_enemy_frame(enemy: &mut WorldEnemy, delta: f32) {
    enemy.frame_elapsed += delta;
    while enemy.frame_elapsed >= ENEMY_FRAME_SECONDS {
        enemy.frame_elapsed -= ENEMY_FRAME_SECONDS;
        enemy.frame = next_walk_frame(enemy.frame);
    }
}

pub(super) fn enemy_top_left(position: Position) -> Vec2 {
    Vec2::new(
        position.x as f32 * TILE_SIZE as f32 + TILE_SIZE as f32 / 2.0
            - (CHARACTER_COLLISION_OFFSET_X + CHARACTER_COLLISION_WIDTH / 2.0),
        position.y as f32 * TILE_SIZE as f32 + TILE_SIZE as f32 / 2.0
            - (CHARACTER_COLLISION_OFFSET_Y + CHARACTER_COLLISION_HEIGHT / 2.0),
    )
}

pub(super) fn enemy_collision_rect(top_left: Vec2) -> CharacterCollisionRect {
    character_collision_rect(top_left)
}

pub(super) fn enemy_tile_position(top_left: Vec2) -> Position {
    let rect = enemy_collision_rect(top_left);
    Position::new(
        ((rect.x + rect.width / 2.0) / TILE_SIZE as f32).floor() as i32,
        ((rect.y + rect.height / 2.0) / TILE_SIZE as f32).floor() as i32,
    )
}

pub(super) fn enemy_world_translation(top_left: Vec2) -> Vec3 {
    let center = top_left + Vec2::splat(CHARACTER_SPRITE_SIZE / 2.0);
    let world_y = -center.y;
    Vec3::new(
        center.x,
        world_y,
        world_entity_y_z(world_y, ENEMY_SPRITE_HALF_HEIGHT),
    )
}

pub(super) fn direction_toward(from: Vec2, to: Vec2) -> CardinalDirection {
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

pub(super) const fn enemy_tile(direction: CardinalDirection, frame: u32) -> u32 {
    let direction_row = match direction {
        CardinalDirection::Up => 0,
        CardinalDirection::Left => 1,
        CardinalDirection::Down => 2,
        CardinalDirection::Right => 3,
    };
    (ENEMY_WALK_ROW_OFFSET + direction_row) * ENEMY_ATLAS_COLUMNS + frame
}

pub(super) const fn next_walk_frame(frame: u32) -> u32 {
    if frame >= 8 { 1 } else { frame + 1 }
}

pub(super) fn random_pause(rng: &mut crate::gameplay_rng::GameplayRng) -> f32 {
    1.0 + (rng.next_u64() as f64 / u64::MAX as f64 * 2.5) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario_path::ScenarioRelativePath;

    #[test]
    fn enemy_walk_tiles_use_the_lpc_rows_eight_through_eleven() {
        assert_eq!(enemy_tile(CardinalDirection::Up, 0), 72);
        assert_eq!(enemy_tile(CardinalDirection::Left, 0), 81);
        assert_eq!(enemy_tile(CardinalDirection::Down, 0), 90);
        assert_eq!(enemy_tile(CardinalDirection::Right, 8), 107);
    }

    #[test]
    fn deterministic_chase_stays_open_and_avoids_other_enemies() {
        let owner = ScenarioRelativePath::try_from("media/maps/test.tmx").unwrap();
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
            WorldEnemyBalance::default(),
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
            WorldEnemyBalance::default(),
        ));
        assert!((enemy.top_left.x - origin.x - 11.2).abs() < 0.001);
    }

    #[test]
    fn deterministic_wander_target_stays_bounded_open_and_unoccupied() {
        let owner = ScenarioRelativePath::try_from("media/maps/test.tmx").unwrap();
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
        let a = pick_wander_target(
            &enemy,
            entity,
            &snapshot,
            &collision,
            &mut first,
            WorldEnemyBalance::default(),
        )
        .unwrap();
        let b = pick_wander_target(
            &enemy,
            entity,
            &snapshot,
            &collision,
            &mut second,
            WorldEnemyBalance::default(),
        )
        .unwrap();
        assert_eq!(a, b);
        assert!((a.x - enemy.top_left.x).abs() <= 4.0 * TILE_SIZE as f32);
        assert!((a.y - enemy.top_left.y).abs() <= 4.0 * TILE_SIZE as f32);
        assert!(!enemy_collision_rect(a).overlaps(snapshot[1].1));
        let rect = enemy_collision_rect(a);
        assert!(!collision.is_rect_blocked(rect.x, rect.y, rect.width, rect.height));
    }

    fn open_field() -> CollisionOccupancy {
        let owner = ScenarioRelativePath::try_from("media/maps/test.tmx").unwrap();
        let row = ["0"; 12].join(",");
        let rows = [row.as_str(); 12].join(",\n");
        let document = crate::tmx_header::parse_tmx_map_document(
            &format!(
                "<map version=\"1.10\" orientation=\"orthogonal\" width=\"12\" height=\"12\" tilewidth=\"32\" tileheight=\"32\" infinite=\"0\"><layer id=\"1\" name=\"collision\" width=\"12\" height=\"12\"><data encoding=\"csv\">{}</data></layer></map>",
                rows
            ),
            &owner,
        )
        .unwrap();
        CollisionOccupancy::from_tmx_document(&document).unwrap()
    }

    fn open_field_enemy(origin: Position) -> WorldEnemy {
        WorldEnemy {
            encounter_id: "spawn".to_owned(),
            formation: vec!["goblin".to_owned()],
            origin,
            position: origin,
            top_left: enemy_top_left(origin),
            boss: false,
            chase_range: 0,
            active: true,
            engaged: false,
            facing: CardinalDirection::Down,
            frame: 0,
            frame_elapsed: 0.0,
            wander_pause: 0.0,
            wander_target: None,
        }
    }

    fn tuning(speed_tiles: f64, range_tiles: u32) -> WorldEnemyBalance {
        WorldEnemyBalance {
            move_speed_tiles_per_second: rpg_content::scenario_class::PositiveFinite::new(
                speed_tiles,
            )
            .unwrap(),
            wander_range_tiles: rpg_content::scenario_balance::PositiveInteger::new(range_tiles)
                .unwrap(),
        }
    }

    #[test]
    fn enemy_speed_follows_the_authored_tiles_per_second() {
        assert_eq!(
            enemy_move_speed_pixels(WorldEnemyBalance::default()),
            TILE_SIZE as f32 * 3.5
        );
        let collision = open_field();
        let entity = Entity::from_bits(1);
        let player = enemy_top_left(Position::new(10, 5));
        let mut distances = Vec::new();
        for speed in [3.5, 7.0] {
            let mut enemy = open_field_enemy(Position::new(0, 5));
            let start = enemy.top_left;
            let snapshot = [(entity, enemy.collision_rect(), true)];
            assert!(chase_enemy(
                &mut enemy,
                entity,
                player,
                0.1,
                &snapshot,
                &collision,
                tuning(speed, 4),
            ));
            distances.push(enemy.top_left.x - start.x);
        }
        assert!((distances[0] - 11.2).abs() < 0.001);
        assert!((distances[1] - 22.4).abs() < 0.001);
    }

    #[test]
    fn wander_targets_stay_within_the_authored_range() {
        let collision = open_field();
        let entity = Entity::from_bits(1);
        let enemy = open_field_enemy(Position::new(5, 5));
        let snapshot = [(entity, enemy.collision_rect(), true)];
        let mut widest = [0.0_f32; 2];
        for (slot, range) in [2_u32, 4].into_iter().enumerate() {
            for seed in 0..64 {
                let mut rng = crate::gameplay_rng::GameplayRng::from_seed(seed);
                let target = pick_wander_target(
                    &enemy,
                    entity,
                    &snapshot,
                    &collision,
                    &mut rng,
                    tuning(3.5, range),
                )
                .unwrap();
                let reach = (target.x - enemy.top_left.x)
                    .abs()
                    .max((target.y - enemy.top_left.y).abs());
                assert!(
                    reach <= (range * TILE_SIZE) as f32,
                    "range {range}: {reach}"
                );
                widest[slot] = widest[slot].max(reach);
            }
        }
        // The wider setting really does reach past the narrower one's limit.
        assert!(widest[1] > (2 * TILE_SIZE) as f32);
    }
}
