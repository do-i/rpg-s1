//! Half-transparent enemy minimap in the World, cycled with the Enemy Map action (`B`, for bird's-eye view).
//!
//! Visual only: it reads enemy and player positions and writes nothing back, so it never touches
//! saves, replay records or gameplay. The toggle is session-only and survives battles; it resets
//! when a new game begins.

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    action_input::ActionState,
    app_state::AppState,
    game_state::GameState,
    input_bindings::AppAction,
    tmx_ground_asset::{StaticMapRenderState, TmxGroundAsset},
    world_encounter::WorldEnemy,
    world_pause::WorldOverlays,
    world_player::{WorldPlayer, WorldPlayerMotion},
    world_transition::runtime_portals,
};

const MAP_FLAG_ELISE: &str = "npc_elise_joined";
const MAP_FLAG_REIYA: &str = "npc_reiya_joined";
const ENEMY_DOTS_FLAG: &str = "npc_jep_joined";
/// The largest box the panel may fill; the panel shrinks inside it to the map's own aspect.
const PANEL_MAX: Vec2 = Vec2::new(256.0, 192.0);
const PANEL_MARGIN: f32 = 16.0;
const PANEL_Z: i32 = 4_000;
const BORDER_WIDTH: f32 = 1.0;
const ENEMY_DOT: f32 = 6.0;
const BOSS_DOT: f32 = 8.0;
const PLAYER_DOT: f32 = 6.0;
const PORTAL_DOT: f32 = 5.0;
/// Which look the map has, or off. The key steps Off -> Dark -> Light -> Off.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum MapMode {
    #[default]
    Off,
    Dark,
    Light,
}

impl MapMode {
    const fn next(self) -> Self {
        match self {
            Self::Off => Self::Dark,
            Self::Dark => Self::Light,
            Self::Light => Self::Off,
        }
    }

    const fn theme(self) -> Option<MapTheme> {
        match self {
            Self::Off => None,
            Self::Dark => Some(MapTheme {
                // Dark glass with bright marks, for pale maps.
                panel: Color::srgba(0.0, 0.0, 0.0, 0.55),
                border: Color::srgba(1.0, 1.0, 1.0, 0.6),
                enemy: Color::srgb(1.0, 0.35, 0.3),
                boss: Color::srgb(1.0, 0.82, 0.1),
                player: Color::WHITE,
                portal: Color::srgb(0.2, 0.85, 1.0),
            }),
            Self::Light => Some(MapTheme {
                // Pale glass with deep marks, for dark maps.
                panel: Color::srgba(1.0, 1.0, 1.0, 0.6),
                border: Color::srgba(0.0, 0.0, 0.0, 0.6),
                enemy: Color::srgb(0.75, 0.05, 0.05),
                boss: Color::srgb(0.7, 0.45, 0.0),
                player: Color::srgb(0.05, 0.05, 0.1),
                portal: Color::srgb(0.0, 0.4, 0.75),
            }),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct MapTheme {
    panel: Color,
    border: Color,
    enemy: Color,
    boss: Color,
    player: Color,
    portal: Color,
}

/// The enemy map's current mode. Never saved.
#[derive(Clone, Copy, Debug, Default, Resource)]
pub(crate) struct EnemyMapOverlay {
    mode: MapMode,
}

#[derive(Component)]
struct EnemyMapRoot;

#[derive(Component)]
struct EnemyMapDot;

pub(crate) struct WorldEnemyMapPlugin;

impl Plugin for WorldEnemyMapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EnemyMapOverlay>()
            .add_systems(OnEnter(AppState::NameEntry), reset_overlay)
            .add_systems(OnExit(AppState::World), despawn_panel)
            .add_systems(
                Update,
                (toggle_overlay, sync_panel)
                    .chain()
                    .run_if(in_state(AppState::World)),
            );
    }
}

fn reset_overlay(mut overlay: ResMut<EnemyMapOverlay>) {
    overlay.mode = MapMode::Off;
}

/// The map is unavailable until Elise or Reiya has joined. Joined flags persist when a member is
/// benched, so the party cap never takes the map away.
fn map_available(game: Option<&GameState>) -> bool {
    game.is_some_and(|game| {
        game.flags().is_set(MAP_FLAG_ELISE) || game.flags().is_set(MAP_FLAG_REIYA)
    })
}

/// Enemy and boss dots are drawn only once Jep has joined.
fn enemy_dots_available(game: Option<&GameState>) -> bool {
    game.is_some_and(|game| game.flags().is_set(ENEMY_DOTS_FLAG))
}

/// The enemies that get dots: none until Jep has joined.
fn dotted_enemies<'a>(
    game: Option<&GameState>,
    enemies: impl Iterator<Item = &'a WorldEnemy>,
) -> Vec<&'a WorldEnemy> {
    if enemy_dots_available(game) {
        enemies.collect()
    } else {
        Vec::new()
    }
}

fn toggle_overlay(
    actions: Res<ActionState>,
    overlays: WorldOverlays,
    game: Option<Res<GameState>>,
    mut overlay: ResMut<EnemyMapOverlay>,
) {
    if actions.just_pressed(AppAction::EnemyMap)
        && !overlays.any_active()
        && map_available(game.as_deref())
    {
        overlay.mode = overlay.mode.next();
    }
}

fn despawn_panel(mut commands: Commands, roots: Query<Entity, With<EnemyMapRoot>>) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}

/// Where `point_px` lands inside a panel of `panel_px` that shows a map of `map_px`, with the map
/// fitted to the panel at its own aspect ratio and centred. Relative to the panel's top-left.
fn dot_offset(point_px: Vec2, map_px: Vec2, panel_px: Vec2) -> Vec2 {
    let scale = (panel_px.x / map_px.x).min(panel_px.y / map_px.y);
    let letterbox = (panel_px - map_px * scale) / 2.0;
    letterbox + point_px * scale
}

/// The panel size that frames the whole map and nothing else: `PANEL_MAX` fitted to the map's
/// aspect ratio, plus the border, so the panel's inner edge is the map's edge.
fn panel_size(map_px: Vec2) -> Vec2 {
    let scale = (PANEL_MAX.x / map_px.x).min(PANEL_MAX.y / map_px.y);
    (map_px * scale).round() + Vec2::splat(BORDER_WIDTH * 2.0)
}

fn spawn_root(commands: &mut Commands, theme: MapTheme, size: Vec2) -> Entity {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(PANEL_MARGIN),
                bottom: px(PANEL_MARGIN),
                width: px(size.x),
                height: px(size.y),
                border: UiRect::all(px(BORDER_WIDTH)),
                ..default()
            },
            BackgroundColor(theme.panel),
            BorderColor::all(theme.border),
            GlobalZIndex(PANEL_Z),
            Pickable::IGNORE,
            Name::new("Enemy map"),
            EnemyMapRoot,
        ))
        .id()
}

fn dot_bundle(centre: Vec2, size: f32, color: Color) -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            left: px(centre.x - size / 2.0),
            top: px(centre.y - size / 2.0),
            width: px(size),
            height: px(size),
            ..default()
        },
        BackgroundColor(color),
        Pickable::IGNORE,
        EnemyMapDot,
    )
}

/// What the panel reads each frame: the map's size and where everything stands on it.
#[derive(SystemParam)]
struct MapSources<'w, 's> {
    maps: Res<'w, Assets<TmxGroundAsset>>,
    render: Res<'w, StaticMapRenderState>,
    game: Option<Res<'w, GameState>>,
    enemies: Query<'w, 's, &'static WorldEnemy>,
    players: Query<'w, 's, &'static WorldPlayerMotion, With<WorldPlayer>>,
}

/// Portal centres for the active map. They change only when a different map asset is active or
/// when the map assets are modified in place (hot reload keeps the asset id), so the cache is keyed
/// on the asset ids and dropped whenever the `Assets` collection is marked changed.
#[derive(Default)]
struct PortalCache {
    entry: Option<(Vec<bevy::asset::UntypedAssetId>, Vec<Vec2>)>,
}

impl PortalCache {
    fn invalidate(&mut self) {
        self.entry = None;
    }

    fn get(
        &mut self,
        key: &[bevy::asset::UntypedAssetId],
        compute: impl FnOnce() -> Vec<Vec2>,
    ) -> &[Vec2] {
        if self.entry.as_ref().is_none_or(|(k, _)| k != key) {
            self.entry = Some((key.to_vec(), compute()));
        }
        self.entry.as_ref().map_or(&[], |(_, p)| p.as_slice())
    }
}

type DotFilter = (With<EnemyMapDot>, Without<EnemyMapRoot>);

/// The panel's existing dots, for moving in place.
#[derive(SystemParam)]
struct PanelDots<'w, 's> {
    children: Query<'w, 's, &'static Children, With<EnemyMapRoot>>,
    dots: Query<'w, 's, (&'static mut Node, &'static mut BackgroundColor), DotFilter>,
}

fn sync_panel(
    mut commands: Commands,
    overlay: Res<EnemyMapOverlay>,
    overlays: WorldOverlays,
    sources: MapSources,
    mut roots: Query<
        (
            Entity,
            &mut Visibility,
            &mut BackgroundColor,
            &mut BorderColor,
            &mut Node,
        ),
        With<EnemyMapRoot>,
    >,
    mut panel_dots: PanelDots,
    mut portal_cache: Local<PortalCache>,
) {
    // Checked before any early return, so a reload while the overlay is off is still noticed.
    if sources.maps.is_changed() {
        portal_cache.invalidate();
    }
    let theme = if map_available(sources.game.as_deref()) {
        overlay.mode.theme()
    } else {
        None
    };
    let Some(theme) = theme else {
        for (root, ..) in &roots {
            commands.entity(root).despawn();
        }
        return;
    };
    let Some(map) = sources.render.map(&sources.maps) else {
        return;
    };
    let bounds = map.pixel_bounds();
    let map_px = bounds.max() - bounds.min();
    let size = panel_size(map_px);
    let inner = size - Vec2::splat(BORDER_WIDTH * 2.0);

    let root = match roots.single_mut() {
        Ok((root, mut visibility, mut background, mut border, mut node)) => {
            node.width = px(size.x);
            node.height = px(size.y);
            background.0 = theme.panel;
            *border = BorderColor::all(theme.border);
            *visibility = if overlays.any_active() {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };
            root
        }
        Err(_) => spawn_root(&mut commands, theme, size),
    };

    let enemies = dotted_enemies(sources.game.as_deref(), sources.enemies.iter());
    let player = sources.players.iter().next().map(|motion| {
        let rect = motion.collision_rect();
        Vec2::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
    });
    let portals = portal_cache.get(&sources.render.asset_ids(), || {
        runtime_portals(map.document())
            .unwrap_or_default()
            .iter()
            .map(|portal| {
                let bounds = portal.bounds();
                Vec2::new(
                    (bounds.x + bounds.width / 2.0) as f32,
                    (bounds.y + bounds.height / 2.0) as f32,
                )
            })
            .collect()
    });
    let wanted = map_dots(theme, portals, &enemies, player, map_px, inner);
    reconcile_dots(&mut commands, root, &mut panel_dots, &wanted);
}

/// Brings the panel's dots to `wanted`, which is in paint order. While the count is unchanged the
/// existing entities are moved and recoloured in place; when it changes they are replaced, so
/// the paint order (portals, enemies, player) always holds.
fn reconcile_dots(
    commands: &mut Commands,
    root: Entity,
    panel_dots: &mut PanelDots,
    wanted: &[(Vec2, f32, Color)],
) {
    let existing: Vec<Entity> = panel_dots
        .children
        .get(root)
        .map(|c| c.iter().filter(|e| panel_dots.dots.contains(*e)).collect())
        .unwrap_or_default();
    if existing.len() == wanted.len() {
        for (entity, (centre, size, color)) in existing.iter().zip(wanted) {
            if let Ok((mut node, mut background)) = panel_dots.dots.get_mut(*entity) {
                // Compare through the shared deref so an unchanged dot is not marked changed
                // and not re-laid-out.
                let (left, top) = (px(centre.x - size / 2.0), px(centre.y - size / 2.0));
                if node.left != left {
                    node.left = left;
                }
                if node.top != top {
                    node.top = top;
                }
                if node.width != px(*size) {
                    node.width = px(*size);
                    node.height = px(*size);
                }
                if background.0 != *color {
                    background.0 = *color;
                }
            }
        }
        return;
    }
    for dot in existing {
        commands.entity(dot).despawn();
    }
    commands.entity(root).with_children(|panel| {
        for (centre, size, color) in wanted {
            panel.spawn(dot_bundle(*centre, *size, *color));
        }
    });
}

/// Every dot to draw, as panel-relative centre, size and colour: portals first, then one per
/// live, unengaged enemy, then the player so it paints on top.
fn map_dots(
    theme: MapTheme,
    portals: &[Vec2],
    enemies: &[&WorldEnemy],
    player: Option<Vec2>,
    map_px: Vec2,
    panel_px: Vec2,
) -> Vec<(Vec2, f32, Color)> {
    let mut dots: Vec<_> = portals
        .iter()
        .map(|centre| {
            (
                dot_offset(*centre, map_px, panel_px),
                PORTAL_DOT,
                theme.portal,
            )
        })
        .collect();
    dots.extend(
        enemies
            .iter()
            .filter(|enemy| enemy.is_active() && !enemy.is_engaged())
            .map(|enemy| {
                let (size, color) = if enemy.is_boss() {
                    (BOSS_DOT, theme.boss)
                } else {
                    (ENEMY_DOT, theme.enemy)
                };
                (dot_offset(enemy.centre_px(), map_px, panel_px), size, color)
            }),
    );
    dots.extend(player.map(|centre| {
        (
            dot_offset(centre, map_px, panel_px),
            PLAYER_DOT,
            theme.player,
        )
    }));
    dots
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    const PANEL: Vec2 = Vec2::new(256.0, 192.0);

    #[test]
    fn the_panel_takes_the_maps_aspect_so_it_has_no_empty_bands() {
        let border = Vec2::splat(BORDER_WIDTH * 2.0);

        // Wide: width-bound. Tall: height-bound. Matching aspect fills the box.
        assert_eq!(
            panel_size(Vec2::new(1024.0, 384.0)),
            Vec2::new(256.0, 96.0) + border
        );
        assert_eq!(
            panel_size(Vec2::new(384.0, 768.0)),
            Vec2::new(96.0, 192.0) + border
        );
        assert_eq!(panel_size(Vec2::new(512.0, 384.0)), PANEL_MAX + border);
    }

    #[test]
    fn dot_offset_maps_corners_and_centre_of_a_matching_map() {
        let map = Vec2::new(512.0, 384.0);

        assert_eq!(dot_offset(Vec2::ZERO, map, PANEL), Vec2::ZERO);
        assert_eq!(dot_offset(map, map, PANEL), PANEL);
        assert_eq!(dot_offset(map / 2.0, map, PANEL), PANEL / 2.0);
    }

    #[test]
    fn a_wide_map_is_letterboxed_top_and_bottom() {
        let map = Vec2::new(1024.0, 384.0);

        // Scale 0.25 -> 256x96, leaving 48 px above and below.
        assert_eq!(dot_offset(Vec2::ZERO, map, PANEL), Vec2::new(0.0, 48.0));
        assert_eq!(dot_offset(map, map, PANEL), Vec2::new(256.0, 144.0));
    }

    #[test]
    fn a_tall_map_is_letterboxed_left_and_right() {
        let map = Vec2::new(384.0, 768.0);

        // Scale 0.25 -> 96x192, leaving 80 px either side.
        assert_eq!(dot_offset(Vec2::ZERO, map, PANEL), Vec2::new(80.0, 0.0));
        assert_eq!(dot_offset(map, map, PANEL), Vec2::new(176.0, 192.0));
    }

    #[test]
    fn dots_cover_live_unengaged_enemies_and_the_player() {
        use crate::scenario_spatial::Position;
        let tile = Position::new(2, 2);
        let normal = WorldEnemy::for_test(tile, false, true, false);
        let boss = WorldEnemy::for_test(tile, true, true, false);
        let hidden = WorldEnemy::for_test(tile, false, false, false);
        let engaged = WorldEnemy::for_test(tile, false, true, true);

        let theme = MapMode::Dark.theme().unwrap();
        let dots = map_dots(
            theme,
            &[Vec2::new(1.0, 1.0)],
            &[&normal, &boss, &hidden, &engaged],
            Some(Vec2::new(10.0, 10.0)),
            Vec2::new(512.0, 384.0),
            PANEL,
        );

        let summary: Vec<_> = dots
            .iter()
            .map(|(_, size, color)| (*size, *color))
            .collect();
        assert_eq!(
            summary,
            [
                (PORTAL_DOT, theme.portal),
                (ENEMY_DOT, theme.enemy),
                (BOSS_DOT, theme.boss),
                (PLAYER_DOT, theme.player),
            ]
        );
    }

    #[derive(Resource)]
    struct Wanted(Vec<(Vec2, f32, Color)>);

    fn reconcile_app() -> (App, Entity) {
        let mut app = App::new();
        let root = app.world_mut().spawn(EnemyMapRoot).id();
        app.insert_resource(Wanted(Vec::new()));
        app.add_systems(
            Update,
            move |mut commands: Commands, wanted: Res<Wanted>, mut panel_dots: PanelDots| {
                reconcile_dots(&mut commands, root, &mut panel_dots, &wanted.0);
            },
        );
        (app, root)
    }

    fn dot_entities(app: &mut App) -> Vec<Entity> {
        app.world_mut()
            .query_filtered::<Entity, With<EnemyMapDot>>()
            .iter(app.world())
            .collect()
    }

    #[test]
    fn idle_frames_keep_the_same_dot_entities_and_move_them_in_place() {
        let (mut app, _root) = reconcile_app();
        let red = Color::srgb(1.0, 0.0, 0.0);
        app.world_mut().resource_mut::<Wanted>().0 = vec![
            (Vec2::new(10.0, 10.0), 6.0, red),
            (Vec2::new(20.0, 20.0), 6.0, red),
        ];
        app.update();
        let first = dot_entities(&mut app);
        assert_eq!(first.len(), 2);

        // Nothing moves: no dot may have its Node or colour marked changed.
        let mut changed_dots = app
            .world_mut()
            .query_filtered::<(Ref<Node>, Ref<BackgroundColor>), With<EnemyMapDot>>();
        for _ in 0..3 {
            let before = app.world().read_change_tick();
            app.update();
            let now = app.world().read_change_tick();
            let changed = changed_dots
                .iter(app.world())
                .filter(|(node, colour)| {
                    node.last_changed().is_newer_than(before, now)
                        || colour.last_changed().is_newer_than(before, now)
                })
                .count();
            assert_eq!(changed, 0, "idle frame rewrote {changed} dots");
            assert_eq!(dot_entities(&mut app), first);
        }

        // An enemy walks; same count, so nothing is spawned or despawned and only it is touched.
        app.world_mut().resource_mut::<Wanted>().0[0].0 = Vec2::new(40.0, 50.0);
        let before = app.world().read_change_tick();
        app.update();
        let now = app.world().read_change_tick();
        assert_eq!(dot_entities(&mut app), first);
        let moved: Vec<Entity> = changed_dots
            .iter(app.world())
            .zip(&first)
            .filter(|((node, _), _)| node.last_changed().is_newer_than(before, now))
            .map(|(_, e)| *e)
            .collect();
        assert_eq!(moved, vec![first[0]]);
        let node = app.world().get::<Node>(first[0]).unwrap();
        assert_eq!(node.left, px(37.0));
        assert_eq!(node.top, px(47.0));

        // A change in count rebuilds, so paint order stays portals, enemies, player.
        app.world_mut()
            .resource_mut::<Wanted>()
            .0
            .push((Vec2::ZERO, 6.0, red));
        app.update();
        assert_eq!(dot_entities(&mut app).len(), 3);
    }

    #[test]
    fn the_portal_cache_recomputes_on_a_new_map_or_an_invalidation_only() {
        use bevy::asset::UntypedAssetId;
        let key_a = [UntypedAssetId::Uuid {
            type_id: std::any::TypeId::of::<u8>(),
            uuid: bevy::asset::uuid::Uuid::from_u128(1),
        }];
        let key_b = [UntypedAssetId::Uuid {
            type_id: std::any::TypeId::of::<u8>(),
            uuid: bevy::asset::uuid::Uuid::from_u128(2),
        }];
        let mut cache = PortalCache::default();
        let mut parses = 0;
        let mut get = |cache: &mut PortalCache, key: &[UntypedAssetId]| {
            cache.get(key, || {
                parses += 1;
                vec![Vec2::splat(parses as f32)]
            })[0]
        };

        assert_eq!(get(&mut cache, &key_a), Vec2::splat(1.0));
        assert_eq!(get(&mut cache, &key_a), Vec2::splat(1.0));
        assert_eq!(get(&mut cache, &key_b), Vec2::splat(2.0));
        // Same asset id, content modified in place (hot reload): invalidation forces a re-parse.
        cache.invalidate();
        assert_eq!(get(&mut cache, &key_b), Vec2::splat(3.0));
        assert_eq!(get(&mut cache, &key_b), Vec2::splat(3.0));
    }

    #[test]
    fn idle_frames_do_not_mark_the_map_assets_changed() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<TmxGroundAsset>();
        app.update();
        app.update();
        let tick = |app: &App| app.world().read_change_tick();
        let is_changed = |app: &mut App, f: &mut dyn FnMut(&mut App)| {
            let before = tick(app);
            f(app);
            let now = tick(app);
            app.world()
                .resource_ref::<Assets<TmxGroundAsset>>()
                .last_changed()
                .is_newer_than(before, now)
        };
        assert!(!is_changed(&mut app, &mut |a| a.update()));
        assert!(!is_changed(&mut app, &mut |a| a.update()));
    }

    fn toggle_app() -> App {
        let mut app = App::new();
        app.insert_resource(crate::field_menu_domain::tests::game(["npc_elise_joined"]))
            .init_resource::<ActionState>()
            .init_resource::<EnemyMapOverlay>()
            .add_systems(Update, toggle_overlay);
        app
    }

    fn press_enemy_map(app: &mut App) {
        app.world_mut()
            .resource_mut::<ActionState>()
            .press(AppAction::EnemyMap);
        app.update();
        app.world_mut().resource_mut::<ActionState>().clear();
    }

    fn mode(app: &App) -> MapMode {
        app.world().resource::<EnemyMapOverlay>().mode
    }

    #[test]
    fn the_action_cycles_off_dark_light_off_and_survives_a_battle_round_trip() {
        let mut app = toggle_app();
        press_enemy_map(&mut app);
        assert_eq!(mode(&app), MapMode::Dark);

        // Nothing resets it on leaving the World, so a battle round trip keeps it.
        app.world_mut()
            .run_system_once(despawn_panel)
            .expect("the system only queries");
        assert_eq!(mode(&app), MapMode::Dark);

        press_enemy_map(&mut app);
        assert_eq!(mode(&app), MapMode::Light);
        press_enemy_map(&mut app);
        assert_eq!(mode(&app), MapMode::Off);
    }

    #[test]
    fn the_action_is_ignored_while_the_game_is_paused() {
        let mut app = toggle_app();
        app.insert_resource(crate::game_pause::GamePause::paused_for_test());

        press_enemy_map(&mut app);

        assert_eq!(mode(&app), MapMode::Off);
    }

    #[test]
    fn a_new_game_switches_the_overlay_off() {
        let mut app = toggle_app();
        app.add_systems(Update, reset_overlay.after(toggle_overlay));
        app.world_mut().resource_mut::<EnemyMapOverlay>().mode = MapMode::Light;
        app.update();
        assert_eq!(mode(&app), MapMode::Off);
    }

    #[test]
    fn the_action_does_nothing_until_elise_or_reiya_has_joined() {
        for (flags, expected) in [
            (&[][..], MapMode::Off),
            (&["npc_jep_joined"][..], MapMode::Off),
            (&["npc_elise_joined"][..], MapMode::Dark),
            (&["npc_reiya_joined"][..], MapMode::Dark),
        ] {
            let mut app = toggle_app();
            app.insert_resource(crate::field_menu_domain::tests::game([]));
            for flag in flags {
                app.world_mut()
                    .resource_mut::<crate::game_state::GameState>()
                    .flags_mut()
                    .set(*flag);
            }
            press_enemy_map(&mut app);
            assert_eq!(mode(&app), expected, "{flags:?}");
        }
    }

    #[test]
    fn enemy_dots_need_jep_and_appear_as_soon_as_he_joins() {
        use crate::scenario_spatial::Position;
        let normal = WorldEnemy::for_test(Position::new(2, 2), false, true, false);
        let boss = WorldEnemy::for_test(Position::new(3, 3), true, true, false);
        let mut game = crate::field_menu_domain::tests::game(["npc_elise_joined"]);

        assert!(dotted_enemies(None, [&normal, &boss].into_iter()).is_empty());
        assert!(dotted_enemies(Some(&game), [&normal, &boss].into_iter()).is_empty());

        game.flags_mut().set("npc_jep_joined");
        assert_eq!(
            dotted_enemies(Some(&game), [&normal, &boss].into_iter()).len(),
            2
        );
    }
}
