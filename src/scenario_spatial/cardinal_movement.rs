//! Fresh-action eight-way grid movement against the active TMX collision layer.

use std::time::Duration;

use bevy::prelude::*;

use crate::{
    action_input::ActionState,
    app_state::AppState,
    game_state::GameState,
    scenario_path::ScenarioRelativePath,
    scenario_root::ScenarioRoot,
    scenario_spatial::{
        CardinalDirection, EightWayDirection, Position, collision_occupancy::CollisionOccupancy,
    },
    tile_coordinates::tmx_tile_center,
    tmx_ground_asset::TmxGroundAsset,
    world_actor::WorldNpc,
    world_interaction::WorldInteractionState,
    world_player::{WorldPlayer, WorldPlayerAnimation},
    world_transition::WorldTransition,
};

const RUSTED_KINGDOMS_TILE_WIDTH: u32 = 32;
const RUSTED_KINGDOMS_TILE_HEIGHT: u32 = 32;

/// Applies one accepted fresh eight-way action while the app is in the world state.
pub(crate) struct CardinalMovementPlugin;

impl Plugin for CardinalMovementPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveMapCollision>()
            .add_systems(
                Update,
                (load_active_map_collision, move_world_player)
                    .chain()
                    .run_if(in_state(AppState::World)),
            )
            .add_systems(OnExit(AppState::World), clear_active_map_collision);
    }
}

/// Collision data paired with the logical map id that produced it.
///
/// The cache is deliberately private to movement. Rendering and collision share the parsed
/// [`TmxGroundAsset`], while an input can never reuse occupancy from a previously active map.
#[derive(Resource, Default)]
struct ActiveMapCollision {
    map_id: Option<String>,
    handle: Option<Handle<TmxGroundAsset>>,
    occupancy: Option<CollisionOccupancy>,
    failed: bool,
}

impl ActiveMapCollision {
    fn reset_for_map(
        &mut self,
        map_id: &str,
        asset_server: &AssetServer,
        scenario_root: &ScenarioRoot,
    ) {
        self.map_id = Some(map_id.to_owned());
        self.handle = None;
        self.occupancy = None;
        self.failed = false;

        let logical = format!("assets/maps/{map_id}.tmx");
        let Ok(logical) = ScenarioRelativePath::try_from(logical.as_str()) else {
            self.failed = true;
            return;
        };
        self.handle = Some(asset_server.load(scenario_root.resolve(&logical)));
    }

    fn is_open(&self, map_id: &str, position: Position) -> bool {
        self.map_id.as_deref() == Some(map_id)
            && !self.failed
            && self
                .occupancy
                .as_ref()
                .and_then(|occupancy| occupancy.is_open(position.x, position.y))
                == Some(true)
    }
}

fn load_active_map_collision(
    asset_server: Option<Res<AssetServer>>,
    scenario_root: Option<Res<ScenarioRoot>>,
    maps: Option<Res<Assets<TmxGroundAsset>>>,
    game: Option<Res<GameState>>,
    mut collision: ResMut<ActiveMapCollision>,
) {
    let Some(game) = game else {
        *collision = ActiveMapCollision::default();
        return;
    };
    let Some(current) = game.map().current() else {
        *collision = ActiveMapCollision::default();
        return;
    };
    let map_id = current.as_str();
    let asset_server = asset_server.as_deref();
    let scenario_root = scenario_root.as_deref();
    let maps = maps.as_deref();

    if collision.map_id.as_deref() != Some(map_id) {
        let (Some(asset_server), Some(scenario_root), Some(_)) =
            (asset_server, scenario_root, maps)
        else {
            // Headless callers may install collision data directly. Production fails closed until
            // all asset resources exist and the active TMX can be requested.
            *collision = ActiveMapCollision {
                map_id: Some(map_id.to_owned()),
                ..default()
            };
            return;
        };
        collision.reset_for_map(map_id, asset_server, scenario_root);
    }
    if collision.failed || collision.occupancy.is_some() {
        return;
    }

    let Some(handle) = collision.handle.as_ref() else {
        return;
    };
    let Some(asset_server) = asset_server else {
        return;
    };
    if matches!(
        asset_server.load_state(handle.id()),
        bevy::asset::LoadState::Failed(_)
    ) {
        collision.failed = true;
        return;
    }
    let Some(maps) = maps else {
        return;
    };
    let Some(map) = maps.get(handle) else {
        return;
    };
    match CollisionOccupancy::from_tmx_document(map.document()) {
        Ok(occupancy) => collision.occupancy = Some(occupancy),
        Err(_) => collision.failed = true,
    }
}

fn move_world_player(
    actions: Option<Res<ActionState>>,
    time: Option<Res<Time>>,
    collision: Res<ActiveMapCollision>,
    transition: Option<Res<WorldTransition>>,
    interaction: Option<Res<WorldInteractionState>>,
    npcs: Query<&WorldNpc>,
    game: Option<ResMut<GameState>>,
    mut players: Query<(&mut Transform, &mut Sprite, &mut WorldPlayerAnimation), With<WorldPlayer>>,
) {
    let Some(actions) = actions else {
        return;
    };
    let Some(mut game) = game else {
        return;
    };
    let Ok((mut player_transform, mut player_sprite, mut player_animation)) = players.single_mut()
    else {
        return;
    };
    let delta_time = time.as_deref().map(Time::delta).unwrap_or(Duration::ZERO);
    if transition
        .as_deref()
        .is_some_and(WorldTransition::input_locked)
        || interaction
            .as_deref()
            .is_some_and(WorldInteractionState::input_locked)
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

    let current = game.map().position();
    let delta = movement_delta(direction);
    let facing = movement_facing(direction);
    game.map_mut().set_facing(facing);
    let tile_id = player_animation.update(Some(facing), delta_time);
    set_atlas_tile(&mut player_sprite, tile_id);

    let Some(map_id) = game.map().current().map(|map| map.as_str()) else {
        return;
    };
    let Some(next) = accepted_destination(current, delta, map_id, &collision) else {
        return;
    };
    if npcs.iter().any(|npc| npc.tile_position() == next) {
        return;
    }
    let (Ok(column), Ok(row)) = (u32::try_from(next.x), u32::try_from(next.y)) else {
        return;
    };
    let world_center = tmx_tile_center(
        column,
        row,
        RUSTED_KINGDOMS_TILE_WIDTH,
        RUSTED_KINGDOMS_TILE_HEIGHT,
    );

    game.map_mut().set_position(next);
    player_transform.translation = world_center.extend(player_transform.translation.z);
}

fn set_atlas_tile(sprite: &mut Sprite, tile_id: u32) {
    if let Some(atlas) = sprite.texture_atlas.as_mut() {
        atlas.index = tile_id as usize;
    }
}

fn clear_active_map_collision(mut collision: ResMut<ActiveMapCollision>) {
    *collision = ActiveMapCollision::default();
}

/// Applies Python's smooth-collision order to one grid action.
///
/// Continuous Python movement tests the full diagonal rectangle, then X-only, then Y-only. A
/// whole-tile jump would otherwise tunnel through a closed corner, so the grid adaptation accepts
/// the full diagonal only when its destination and both side-adjacent cells are open. If not, it
/// preserves the source's horizontal-then-vertical slide order.
fn accepted_destination(
    current: Position,
    delta: Position,
    map_id: &str,
    collision: &ActiveMapCollision,
) -> Option<Position> {
    let horizontal = offset_position(current, delta.x, 0);
    let vertical = offset_position(current, 0, delta.y);
    if delta.x != 0 && delta.y != 0 {
        let diagonal = offset_position(current, delta.x, delta.y);
        if horizontal.is_some_and(|position| collision.is_open(map_id, position))
            && vertical.is_some_and(|position| collision.is_open(map_id, position))
            && diagonal.is_some_and(|position| collision.is_open(map_id, position))
        {
            return diagonal;
        }
        if horizontal.is_some_and(|position| collision.is_open(map_id, position)) {
            return horizontal;
        }
        if vertical.is_some_and(|position| collision.is_open(map_id, position)) {
            return vertical;
        }
        return None;
    }

    let destination = if delta.x != 0 { horizontal } else { vertical };
    destination.filter(|position| collision.is_open(map_id, *position))
}

const fn offset_position(current: Position, dx: i32, dy: i32) -> Option<Position> {
    let Some(x) = current.x.checked_add(dx) else {
        return None;
    };
    let Some(y) = current.y.checked_add(dy) else {
        return None;
    };
    Some(Position::new(x, y))
}

const fn movement_delta(direction: EightWayDirection) -> Position {
    match direction {
        EightWayDirection::Up => Position::new(0, -1),
        EightWayDirection::UpRight => Position::new(1, -1),
        EightWayDirection::Right => Position::new(1, 0),
        EightWayDirection::DownRight => Position::new(1, 1),
        EightWayDirection::Down => Position::new(0, 1),
        EightWayDirection::DownLeft => Position::new(-1, 1),
        EightWayDirection::Left => Position::new(-1, 0),
        EightWayDirection::UpLeft => Position::new(-1, -1),
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

    use bevy::{image::TextureAtlas, state::app::StatesPlugin};

    use super::*;
    use crate::{
        action_input::ActionInputPlugin,
        new_game::{NewGameScenario, build_new_game_state},
        scenario_balance::BalanceData,
        scenario_manifest::Manifest,
        scenario_party::{PartyCatalog, PartyMember},
        scenario_path::ScenarioRelativePath,
        scenario_spatial::aric_atlas::AricAtlasLayout,
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
            .add_plugins(CardinalMovementPlugin);
        if with_session {
            let mut game = game_state();
            game.map_mut().set_position(Position::new(2, 2));
            app.insert_resource(game);
            app.insert_resource(ActiveMapCollision {
                map_id: Some("town_01_ardel".to_owned()),
                occupancy: Some(occupancy(blocked)),
                ..default()
            });
        }
        for _ in 0..player_count {
            let center = tmx_tile_center(2, 2, 32, 32);
            app.world_mut().spawn((
                WorldPlayer,
                Transform::from_translation(center.extend(PLAYER_Z)),
                player_sprite(),
                player_animation(),
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
    fn each_fresh_cardinal_action_moves_exactly_one_tile_and_updates_facing() {
        for (key, direction, expected_position, expected_frame) in [
            (
                KeyCode::ArrowUp,
                CardinalDirection::Up,
                Position::new(2, 1),
                1,
            ),
            (
                KeyCode::ArrowLeft,
                CardinalDirection::Left,
                Position::new(1, 2),
                10,
            ),
            (
                KeyCode::ArrowDown,
                CardinalDirection::Down,
                Position::new(2, 3),
                19,
            ),
            (
                KeyCode::ArrowRight,
                CardinalDirection::Right,
                Position::new(3, 2),
                28,
            ),
        ] {
            let mut app = movement_app(AppState::World, true, 1, &[]);
            press(&mut app, &[key]);
            app.update();

            let game = app.world().resource::<GameState>();
            assert_eq!(game.map().position(), expected_position);
            assert_eq!(game.map().facing(), direction);
            assert_eq!(
                player_translation(&mut app),
                tmx_tile_center(
                    expected_position.x as u32,
                    expected_position.y as u32,
                    32,
                    32,
                )
                .extend(PLAYER_Z)
            );
            assert_eq!(player_frame(&mut app), expected_frame);
        }
    }

    #[test]
    fn all_four_diagonals_move_one_grid_step_and_use_vertical_facing() {
        for (keys, direction, expected_position, expected_frame) in [
            (
                [KeyCode::ArrowUp, KeyCode::ArrowRight],
                CardinalDirection::Up,
                Position::new(3, 1),
                1,
            ),
            (
                [KeyCode::ArrowDown, KeyCode::ArrowRight],
                CardinalDirection::Down,
                Position::new(3, 3),
                19,
            ),
            (
                [KeyCode::ArrowDown, KeyCode::ArrowLeft],
                CardinalDirection::Down,
                Position::new(1, 3),
                19,
            ),
            (
                [KeyCode::ArrowUp, KeyCode::ArrowLeft],
                CardinalDirection::Up,
                Position::new(1, 1),
                1,
            ),
        ] {
            let mut app = movement_app(AppState::World, true, 1, &[]);
            press(&mut app, &keys);
            app.update();

            let game = app.world().resource::<GameState>();
            assert_eq!(game.map().position(), expected_position);
            assert_eq!(game.map().facing(), direction);
            assert_eq!(
                player_translation(&mut app),
                tmx_tile_center(
                    expected_position.x as u32,
                    expected_position.y as u32,
                    32,
                    32,
                )
                .extend(PLAYER_Z)
            );
            assert_eq!(player_frame(&mut app), expected_frame);
        }
    }

    #[test]
    fn diagonal_grid_collision_preserves_python_full_then_x_then_y_order() {
        // The grid adaptation forbids tunneling through a corner: full diagonal requires both
        // side cells and the destination. When full movement is blocked, Python smooth collision
        // tries X-only before Y-only, which these cases preserve exactly.
        for (blocked, expected_position) in [
            (vec![Position::new(3, 1)], Position::new(3, 2)),
            (vec![Position::new(3, 2)], Position::new(2, 1)),
            (vec![Position::new(2, 1)], Position::new(3, 2)),
            (
                vec![Position::new(3, 2), Position::new(2, 1)],
                Position::new(2, 2),
            ),
        ] {
            let mut app = movement_app(AppState::World, true, 1, &blocked);
            press(&mut app, &[KeyCode::ArrowUp, KeyCode::ArrowRight]);
            app.update();

            let game = app.world().resource::<GameState>();
            assert_eq!(game.map().position(), expected_position);
            assert_eq!(game.map().facing(), CardinalDirection::Up);
            assert_eq!(player_frame(&mut app), 1);
        }
    }

    #[test]
    fn authored_walk_frame_stays_visible_for_its_tsx_duration_then_idles() {
        let mut animation = player_animation();

        assert_eq!(
            animation.update(Some(CardinalDirection::Right), Duration::ZERO),
            28
        );
        assert_eq!(animation.update(None, Duration::from_millis(99)), 28);
        assert_eq!(animation.update(None, Duration::from_millis(1)), 27);

        // A later grid action continues the walk cycle instead of flashing frame one repeatedly.
        assert_eq!(
            animation.update(Some(CardinalDirection::Right), Duration::ZERO),
            29
        );
        assert_eq!(animation.update(None, Duration::from_millis(100)), 27);

        // Direction changes select the same next walk column in the new authored row, and the
        // eventual idle frame retains that last facing.
        assert_eq!(
            animation.update(Some(CardinalDirection::Up), Duration::ZERO),
            3
        );
        assert_eq!(animation.update(None, Duration::from_millis(100)), 0);
    }

    #[test]
    fn held_key_does_not_retrigger_until_released_and_pressed_again() {
        let mut app = movement_app(AppState::World, true, 1, &[]);
        press(&mut app, &[KeyCode::ArrowRight]);
        app.update();
        assert_eq!(
            app.world().resource::<GameState>().map().position(),
            Position::new(3, 2)
        );

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.update();
        assert_eq!(
            app.world().resource::<GameState>().map().position(),
            Position::new(3, 2)
        );

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::ArrowRight);
        app.update();
        press(&mut app, &[KeyCode::ArrowRight]);
        app.update();
        assert_eq!(
            app.world().resource::<GameState>().map().position(),
            Position::new(4, 2)
        );
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
    fn collision_rejects_each_cardinal_destination_and_only_updates_facing() {
        for (key, direction, blocked) in [
            (KeyCode::ArrowUp, CardinalDirection::Up, Position::new(2, 1)),
            (
                KeyCode::ArrowLeft,
                CardinalDirection::Left,
                Position::new(1, 2),
            ),
            (
                KeyCode::ArrowDown,
                CardinalDirection::Down,
                Position::new(2, 3),
            ),
            (
                KeyCode::ArrowRight,
                CardinalDirection::Right,
                Position::new(3, 2),
            ),
        ] {
            let mut app = movement_app(AppState::World, true, 1, &[blocked]);
            let initial_transform = player_translation(&mut app);
            press(&mut app, &[key]);
            app.update();

            let game = app.world().resource::<GameState>();
            assert_eq!(game.map().position(), Position::new(2, 2));
            assert_eq!(game.map().facing(), direction);
            assert_eq!(player_translation(&mut app), initial_transform);
        }
    }

    #[test]
    fn missing_stale_or_out_of_bounds_collision_data_fails_closed() {
        for collision in [
            ActiveMapCollision::default(),
            ActiveMapCollision {
                map_id: Some("different_map".to_owned()),
                occupancy: Some(occupancy(&[])),
                ..default()
            },
            ActiveMapCollision {
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

        let mut app = movement_app(AppState::World, true, 1, &[]);
        app.world_mut()
            .resource_mut::<GameState>()
            .map_mut()
            .set_position(Position::new(4, 2));
        press(&mut app, &[KeyCode::ArrowRight]);
        app.update();
        assert_eq!(
            app.world().resource::<GameState>().map().position(),
            Position::new(4, 2)
        );
    }
}
