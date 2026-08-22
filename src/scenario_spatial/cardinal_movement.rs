//! Continuous eight-way movement against the active TMX collision layer.

use std::time::Duration;

use bevy::prelude::*;

use crate::{
    action_input::ActionState,
    app_state::AppState,
    field_menu::FieldMenuState,
    game_state::GameState,
    scenario_spatial::{
        CardinalDirection, EightWayDirection, collision_occupancy::CollisionOccupancy,
        world_collision::WorldCollision,
    },
    service_ui::ServiceUiState,
    world_actor::WorldNpc,
    world_encounter::BattleTransition,
    world_interaction::WorldInteractionState,
    world_object::WorldItemBox,
    world_player::{
        CHARACTER_COLLISION_HEIGHT, CHARACTER_COLLISION_OFFSET_X, CHARACTER_COLLISION_OFFSET_Y,
        CHARACTER_COLLISION_WIDTH, CharacterCollisionRect, WorldPlayer, WorldPlayerAnimation,
        WorldPlayerMotion,
    },
    world_transition::WorldTransition,
};

const RUSTED_KINGDOMS_TILE_WIDTH: u32 = 32;
const RUSTED_KINGDOMS_TILE_HEIGHT: u32 = 32;
const PLAYER_SPEED_PIXELS_PER_SECOND: f32 = 5.0 * 60.0;
const MAX_COLLISION_STEP_PIXELS: f32 = 5.0;
const MAX_MOVEMENT_DELTA_SECONDS: f32 = 0.1;

/// Applies held eight-way input as smooth source-pixel movement while the app is in the world.
pub(crate) struct CardinalMovementPlugin;

impl Plugin for CardinalMovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, move_world_player.run_if(in_state(AppState::World)));
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "the movement system consumes independent Bevy resources and collision queries"
)]
fn move_world_player(
    actions: Option<Res<ActionState>>,
    time: Option<Res<Time>>,
    collision: Res<WorldCollision>,
    transition: Option<Res<WorldTransition>>,
    battle_transition: Option<Res<BattleTransition>>,
    interaction: Option<Res<WorldInteractionState>>,
    field_menu: Option<Res<FieldMenuState>>,
    service: Option<Res<ServiceUiState>>,
    npcs: Query<&WorldNpc>,
    boxes: Query<&WorldItemBox>,
    game: Option<ResMut<GameState>>,
    mut players: Query<
        (
            &mut Transform,
            &mut Sprite,
            &mut WorldPlayerAnimation,
            &mut WorldPlayerMotion,
        ),
        With<WorldPlayer>,
    >,
) {
    let Some(actions) = actions else {
        return;
    };
    let Some(mut game) = game else {
        return;
    };
    let Ok((mut player_transform, mut player_sprite, mut player_animation, mut motion)) =
        players.single_mut()
    else {
        return;
    };
    let delta_time = time.as_deref().map(Time::delta).unwrap_or(Duration::ZERO);
    if transition
        .as_deref()
        .is_some_and(WorldTransition::input_locked)
        || battle_transition
            .as_deref()
            .is_some_and(BattleTransition::input_locked)
        || interaction
            .as_deref()
            .is_some_and(WorldInteractionState::input_locked)
        || field_menu
            .as_deref()
            .is_some_and(FieldMenuState::input_locked)
        || service.as_deref().is_some_and(ServiceUiState::input_locked)
    {
        let tile_id = player_animation.update(None, delta_time);
        set_atlas_tile(&mut player_sprite, tile_id);
        return;
    }
    let Some(direction) = actions.movement() else {
        let tile_id = player_animation.update(None, delta_time);
        set_atlas_tile(&mut player_sprite, tile_id);
        return;
    };

    let facing = movement_facing(direction);
    game.map_mut().set_facing(facing);
    let tile_id = player_animation.update(Some(facing), delta_time);
    set_atlas_tile(&mut player_sprite, tile_id);

    let Some(map_id) = game.map().current().map(|map| map.as_str()) else {
        return;
    };
    let Some(occupancy) = collision.occupancy_for(map_id) else {
        return;
    };
    let direction = movement_vector(direction);
    let seconds = delta_time.as_secs_f32().min(MAX_MOVEMENT_DELTA_SECONDS);
    let mut remaining = direction * PLAYER_SPEED_PIXELS_PER_SECOND * seconds;
    while remaining.abs().max_element() > f32::EPSILON {
        let fraction = (MAX_COLLISION_STEP_PIXELS / remaining.abs().max_element()).min(1.0);
        let step = remaining * fraction;
        let before = motion.top_left();
        let after = smooth_step(before, step, occupancy, &npcs, &boxes);
        motion.set_top_left(after);
        remaining -= step;
        if after == before {
            break;
        }
    }

    let tile = motion.tile_position();
    game.map_mut().set_position(tile);
    let visual_y = -(motion.top_left().y + crate::world_player::CHARACTER_SPRITE_SIZE / 2.0);
    let z = crate::tmx_ground_asset::world_entity_y_z(
        visual_y,
        crate::world_player::CHARACTER_SPRITE_SIZE / 2.0,
    );
    player_transform.translation = motion.sprite_center_world(z);
}

fn set_atlas_tile(sprite: &mut Sprite, tile_id: u32) {
    if let Some(atlas) = sprite.texture_atlas.as_mut() {
        atlas.index = tile_id as usize;
    }
}

fn smooth_step(
    current: Vec2,
    delta: Vec2,
    collision: &CollisionOccupancy,
    npcs: &Query<&WorldNpc>,
    boxes: &Query<&WorldItemBox>,
) -> Vec2 {
    let full = clamp_top_left(current + delta, collision);
    if dynamic_collision(full, npcs, boxes) {
        return current;
    }
    if !tile_collision(full, collision) {
        return full;
    }

    let horizontal = clamp_top_left(current + Vec2::new(delta.x, 0.0), collision);
    if !dynamic_collision(horizontal, npcs, boxes) && !tile_collision(horizontal, collision) {
        return horizontal;
    }
    let vertical = clamp_top_left(current + Vec2::new(0.0, delta.y), collision);
    if !dynamic_collision(vertical, npcs, boxes) && !tile_collision(vertical, collision) {
        return vertical;
    }
    current
}

fn clamp_top_left(position: Vec2, collision: &CollisionOccupancy) -> Vec2 {
    Vec2::new(
        position.x.clamp(
            -CHARACTER_COLLISION_OFFSET_X,
            collision.width() as f32 * RUSTED_KINGDOMS_TILE_WIDTH as f32
                - CHARACTER_COLLISION_OFFSET_X
                - CHARACTER_COLLISION_WIDTH,
        ),
        position.y.clamp(
            -CHARACTER_COLLISION_OFFSET_Y,
            collision.height() as f32 * RUSTED_KINGDOMS_TILE_HEIGHT as f32
                - CHARACTER_COLLISION_OFFSET_Y
                - CHARACTER_COLLISION_HEIGHT,
        ),
    )
}

fn tile_collision(top_left: Vec2, collision: &CollisionOccupancy) -> bool {
    let rect = player_rect(top_left);
    collision.is_rect_blocked(rect.x, rect.y, rect.width, rect.height)
}

fn dynamic_collision(
    top_left: Vec2,
    npcs: &Query<&WorldNpc>,
    boxes: &Query<&WorldItemBox>,
) -> bool {
    let rect = player_rect(top_left);
    npcs.iter().any(|npc| rect.overlaps(npc.collision_rect()))
        || boxes.iter().any(|item_box| {
            let tile = item_box.tile_position();
            rect.overlaps(CharacterCollisionRect {
                x: tile.x as f32 * RUSTED_KINGDOMS_TILE_WIDTH as f32,
                y: tile.y as f32 * RUSTED_KINGDOMS_TILE_HEIGHT as f32,
                width: RUSTED_KINGDOMS_TILE_WIDTH as f32,
                height: RUSTED_KINGDOMS_TILE_HEIGHT as f32,
            })
        })
}

fn player_rect(top_left: Vec2) -> CharacterCollisionRect {
    CharacterCollisionRect {
        x: top_left.x + CHARACTER_COLLISION_OFFSET_X,
        y: top_left.y + CHARACTER_COLLISION_OFFSET_Y,
        width: CHARACTER_COLLISION_WIDTH,
        height: CHARACTER_COLLISION_HEIGHT,
    }
}

fn movement_vector(direction: EightWayDirection) -> Vec2 {
    match direction {
        EightWayDirection::Up => Vec2::new(0.0, -1.0),
        EightWayDirection::UpRight => Vec2::new(
            std::f32::consts::FRAC_1_SQRT_2,
            -std::f32::consts::FRAC_1_SQRT_2,
        ),
        EightWayDirection::Right => Vec2::new(1.0, 0.0),
        EightWayDirection::DownRight => Vec2::splat(std::f32::consts::FRAC_1_SQRT_2),
        EightWayDirection::Down => Vec2::new(0.0, 1.0),
        EightWayDirection::DownLeft => Vec2::new(
            -std::f32::consts::FRAC_1_SQRT_2,
            std::f32::consts::FRAC_1_SQRT_2,
        ),
        EightWayDirection::Left => Vec2::new(-1.0, 0.0),
        EightWayDirection::UpLeft => Vec2::splat(-std::f32::consts::FRAC_1_SQRT_2),
    }
}

const fn movement_facing(direction: EightWayDirection) -> CardinalDirection {
    match direction {
        EightWayDirection::Up | EightWayDirection::UpRight | EightWayDirection::UpLeft => {
            CardinalDirection::Up
        }
        EightWayDirection::Down | EightWayDirection::DownRight | EightWayDirection::DownLeft => {
            CardinalDirection::Down
        }
        EightWayDirection::Left => CardinalDirection::Left,
        EightWayDirection::Right => CardinalDirection::Right,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{image::TextureAtlas, state::app::StatesPlugin, time::TimeUpdateStrategy};

    use super::*;
    use crate::{
        action_input::ActionInputPlugin,
        new_game::{NewGameScenario, build_new_game_state},
        scenario_balance::BalanceData,
        scenario_manifest::Manifest,
        scenario_party::{PartyCatalog, PartyMember},
        scenario_path::ScenarioRelativePath,
        scenario_spatial::{Position, aric_atlas::AricAtlasLayout},
        scenario_yaml,
        tmx_header::parse_tmx_map_document,
        tsx_metadata::parse_tsx_tileset_metadata,
    };

    const PLAYER_Z: f32 = 7.0;
    const COPIED_ARIC_TSX: &str = include_str!(
        "../../assets/scenarios/rusted_kingdoms/assets/sprites/party/01_aric_walk.tsx"
    );

    fn game_state() -> GameState {
        let manifest: Manifest = scenario_yaml::from_str(include_str!(
            "../../tests/fixtures/rusted-kingdoms-manifest-complete.yaml"
        ))
        .unwrap();
        let mut party: PartyCatalog = scenario_yaml::from_str(include_str!(
            "../../tests/fixtures/party-catalog-shapes.yaml"
        ))
        .unwrap();
        let PartyMember::Protagonist(protagonist) = &mut party.party[0] else {
            panic!("invented first party member must be the protagonist");
        };
        protagonist.id = "aric".to_owned();
        protagonist.name = "Aric".to_owned();
        protagonist.class_id = "hero".to_owned();
        let balance: BalanceData =
            scenario_yaml::from_str(include_str!("../../tests/fixtures/balance-complete.yaml"))
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

    fn occupancy(blocked: &[Position]) -> CollisionOccupancy {
        let cells = (0..5)
            .map(|row| {
                (0..5)
                    .map(|column| {
                        let position = Position::new(column, row);
                        u8::from(blocked.contains(&position)).to_string()
                    })
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .collect::<Vec<_>>()
            .join(",\n");
        let xml = format!(
            r#"<map orientation="orthogonal" width="5" height="5" tilewidth="32" tileheight="32">
                <layer id="1" name="collision" width="5" height="5">
                    <data encoding="csv">{cells}</data>
                </layer>
            </map>"#
        );
        let path = ScenarioRelativePath::try_from("assets/maps/invented.tmx").unwrap();
        let document = parse_tmx_map_document(&xml, &path).unwrap();
        CollisionOccupancy::from_tmx_document(&document).unwrap()
    }

    fn player_animation() -> WorldPlayerAnimation {
        let path = ScenarioRelativePath::try_from("assets/sprites/party/01_aric_walk.tsx").unwrap();
        let metadata = parse_tsx_tileset_metadata(COPIED_ARIC_TSX, &path).unwrap();
        let layout = AricAtlasLayout::from_tsx_metadata(&metadata).unwrap();
        WorldPlayerAnimation::new(layout, CardinalDirection::Down)
    }

    fn player_sprite() -> Sprite {
        Sprite {
            texture_atlas: Some(TextureAtlas {
                layout: default(),
                index: 18,
            }),
            ..default()
        }
    }

    fn movement_app(
        state: AppState,
        with_session: bool,
        player_count: usize,
        blocked: &[Position],
    ) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(StatesPlugin)
            .init_resource::<ButtonInput<KeyCode>>()
            .insert_state(state)
            .add_plugins(ActionInputPlugin)
            .add_plugins((
                crate::scenario_spatial::world_collision::WorldCollisionPlugin,
                CardinalMovementPlugin,
            ));
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 60.0,
        )));
        if with_session {
            let mut game = game_state();
            game.map_mut().set_position(Position::new(2, 2));
            app.insert_resource(game);
            app.insert_resource(WorldCollision::loaded_for(
                "town_01_ardel",
                occupancy(blocked),
            ));
        }
        for _ in 0..player_count {
            let motion = WorldPlayerMotion::from_tile(Position::new(2, 2));
            app.world_mut().spawn((
                WorldPlayer,
                Transform::from_translation(motion.sprite_center_world(PLAYER_Z)),
                player_sprite(),
                player_animation(),
                motion,
            ));
        }
        app.update();
        app
    }

    fn press(app: &mut App, keys: &[KeyCode]) {
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        for key in keys {
            input.press(*key);
        }
    }

    fn player_translation(app: &mut App) -> Vec3 {
        app.world_mut()
            .query_filtered::<&Transform, With<WorldPlayer>>()
            .single(app.world())
            .unwrap()
            .translation
    }

    fn player_frame(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<&Sprite, With<WorldPlayer>>()
            .single(app.world())
            .unwrap()
            .texture_atlas
            .as_ref()
            .expect("test World player should use an atlas")
            .index
    }

    #[test]
    fn held_cardinal_input_moves_five_pixels_per_frame_and_updates_facing() {
        for (key, direction, expected_delta, expected_frame) in [
            (
                KeyCode::ArrowUp,
                CardinalDirection::Up,
                Vec2::new(0.0, -5.0),
                1,
            ),
            (
                KeyCode::ArrowLeft,
                CardinalDirection::Left,
                Vec2::new(-5.0, 0.0),
                10,
            ),
            (
                KeyCode::ArrowDown,
                CardinalDirection::Down,
                Vec2::new(0.0, 5.0),
                19,
            ),
            (
                KeyCode::ArrowRight,
                CardinalDirection::Right,
                Vec2::new(5.0, 0.0),
                28,
            ),
        ] {
            let mut app = movement_app(AppState::World, true, 1, &[]);
            let start = app
                .world_mut()
                .query_filtered::<&WorldPlayerMotion, With<WorldPlayer>>()
                .single(app.world())
                .unwrap()
                .top_left();
            press(&mut app, &[key]);
            app.update();

            let game = app.world().resource::<GameState>();
            assert_eq!(game.map().position(), Position::new(2, 2));
            assert_eq!(game.map().facing(), direction);
            let moved = app
                .world_mut()
                .query_filtered::<&WorldPlayerMotion, With<WorldPlayer>>()
                .single(app.world())
                .unwrap()
                .top_left();
            assert!((moved - (start + expected_delta)).length() < 0.01);
            assert_eq!(player_frame(&mut app), expected_frame);
        }
    }

    #[test]
    fn all_four_diagonals_are_normalized_and_use_vertical_facing() {
        for (keys, direction, signs, expected_frame) in [
            (
                [KeyCode::ArrowUp, KeyCode::ArrowRight],
                CardinalDirection::Up,
                Vec2::new(1.0, -1.0),
                1,
            ),
            (
                [KeyCode::ArrowDown, KeyCode::ArrowRight],
                CardinalDirection::Down,
                Vec2::new(1.0, 1.0),
                19,
            ),
            (
                [KeyCode::ArrowDown, KeyCode::ArrowLeft],
                CardinalDirection::Down,
                Vec2::new(-1.0, 1.0),
                19,
            ),
            (
                [KeyCode::ArrowUp, KeyCode::ArrowLeft],
                CardinalDirection::Up,
                Vec2::new(-1.0, -1.0),
                1,
            ),
        ] {
            let mut app = movement_app(AppState::World, true, 1, &[]);
            let start = app
                .world_mut()
                .query_filtered::<&WorldPlayerMotion, With<WorldPlayer>>()
                .single(app.world())
                .unwrap()
                .top_left();
            press(&mut app, &keys);
            app.update();

            let game = app.world().resource::<GameState>();
            assert_eq!(game.map().position(), Position::new(2, 2));
            assert_eq!(game.map().facing(), direction);
            let moved = app
                .world_mut()
                .query_filtered::<&WorldPlayerMotion, With<WorldPlayer>>()
                .single(app.world())
                .unwrap()
                .top_left()
                - start;
            let expected = signs * (5.0 * std::f32::consts::FRAC_1_SQRT_2);
            assert!((moved - expected).length() < 0.01);
            assert_eq!(player_frame(&mut app), expected_frame);
        }
    }

    #[test]
    fn diagonal_collision_slides_along_a_wall_instead_of_stopping_or_teleporting() {
        let mut app = movement_app(
            AppState::World,
            true,
            1,
            &[Position::new(3, 2), Position::new(3, 1)],
        );
        let start = app
            .world_mut()
            .query_filtered::<&WorldPlayerMotion, With<WorldPlayer>>()
            .single(app.world())
            .unwrap()
            .top_left();
        press(&mut app, &[KeyCode::ArrowUp, KeyCode::ArrowRight]);
        for _ in 0..8 {
            app.update();
        }
        let moved = app
            .world_mut()
            .query_filtered::<&WorldPlayerMotion, With<WorldPlayer>>()
            .single(app.world())
            .unwrap()
            .top_left();
        assert!(moved.y < start.y - 20.0);
        assert!(moved.x < start.x + 10.0);
        assert_eq!(
            app.world().resource::<GameState>().map().facing(),
            CardinalDirection::Up
        );
    }

    #[test]
    fn authored_walk_cycle_advances_while_moving_and_idles_on_release() {
        let mut animation = player_animation();

        assert_eq!(
            animation.update(Some(CardinalDirection::Right), Duration::ZERO),
            28
        );
        assert_eq!(
            animation.update(Some(CardinalDirection::Right), Duration::from_millis(99)),
            28
        );
        assert_eq!(
            animation.update(Some(CardinalDirection::Right), Duration::from_millis(1)),
            29
        );
        assert_eq!(animation.update(None, Duration::ZERO), 27);

        // A later movement resumes at the next walk column instead of flashing frame one.
        assert_eq!(
            animation.update(Some(CardinalDirection::Up), Duration::ZERO),
            3
        );
        assert_eq!(animation.update(None, Duration::ZERO), 0);
    }

    #[test]
    fn held_key_continues_moving_until_released() {
        let mut app = movement_app(AppState::World, true, 1, &[]);
        let start = app
            .world_mut()
            .query_filtered::<&WorldPlayerMotion, With<WorldPlayer>>()
            .single(app.world())
            .unwrap()
            .top_left();
        press(&mut app, &[KeyCode::ArrowRight]);
        app.update();
        let first = app
            .world_mut()
            .query_filtered::<&WorldPlayerMotion, With<WorldPlayer>>()
            .single(app.world())
            .unwrap()
            .top_left();
        assert!((first.x - start.x - 5.0).abs() < 0.01);

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.update();
        let second = app
            .world_mut()
            .query_filtered::<&WorldPlayerMotion, With<WorldPlayer>>()
            .single(app.world())
            .unwrap()
            .top_left();
        assert!((second.x - first.x - 5.0).abs() < 0.01);

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::ArrowRight);
        app.update();
        let released = app
            .world_mut()
            .query_filtered::<&WorldPlayerMotion, With<WorldPlayer>>()
            .single(app.world())
            .unwrap()
            .top_left();
        assert_eq!(released, second);
    }

    #[test]
    fn opposite_simultaneous_actions_cancel_without_changing_facing() {
        for keys in [
            [KeyCode::ArrowLeft, KeyCode::ArrowRight],
            [KeyCode::ArrowUp, KeyCode::ArrowDown],
        ] {
            let mut app = movement_app(AppState::World, true, 1, &[]);
            let initial_transform = player_translation(&mut app);
            press(&mut app, &keys);
            app.update();

            let game = app.world().resource::<GameState>();
            assert_eq!(game.map().position(), Position::new(2, 2));
            assert_eq!(game.map().facing(), CardinalDirection::Down);
            assert_eq!(player_translation(&mut app), initial_transform);
        }
    }

    #[test]
    fn movement_requires_world_state_session_and_exactly_one_player() {
        for (state, with_session, player_count) in [
            (AppState::Title, true, 1),
            (AppState::World, false, 1),
            (AppState::World, true, 0),
            (AppState::World, true, 2),
        ] {
            let mut app = movement_app(state, with_session, player_count, &[]);
            let before = app
                .world()
                .get_resource::<GameState>()
                .map(|game| game.map().position());
            press(&mut app, &[KeyCode::ArrowRight]);
            app.update();
            let after = app
                .world()
                .get_resource::<GameState>()
                .map(|game| game.map().position());
            assert_eq!(after, before);
        }
    }

    #[test]
    fn collision_rectangle_can_approach_grass_edge_but_cannot_enter_blocked_tile() {
        let mut app = movement_app(AppState::World, true, 1, &[Position::new(3, 2)]);
        press(&mut app, &[KeyCode::ArrowRight]);
        for _ in 0..12 {
            app.update();
        }
        let motion = *app
            .world_mut()
            .query_filtered::<&WorldPlayerMotion, With<WorldPlayer>>()
            .single(app.world())
            .unwrap();
        let rect = motion.collision_rect();
        assert!(rect.x + rect.width <= 96.0);
        assert!(
            motion.top_left().x
                > WorldPlayerMotion::from_tile(Position::new(2, 2))
                    .top_left()
                    .x
        );
        assert_eq!(
            app.world().resource::<GameState>().map().facing(),
            CardinalDirection::Right
        );
    }

    #[test]
    fn missing_stale_or_out_of_bounds_collision_data_fails_closed() {
        for collision in [
            WorldCollision::default(),
            WorldCollision {
                map_id: Some("different_map".to_owned()),
                occupancy: Some(occupancy(&[])),
                ..default()
            },
            WorldCollision {
                map_id: Some("town_01_ardel".to_owned()),
                failed: true,
                occupancy: Some(occupancy(&[])),
                ..default()
            },
        ] {
            let mut app = movement_app(AppState::World, true, 1, &[]);
            app.insert_resource(collision);
            press(&mut app, &[KeyCode::ArrowRight]);
            app.update();

            assert_eq!(
                app.world().resource::<GameState>().map().position(),
                Position::new(2, 2)
            );
        }
    }
}
