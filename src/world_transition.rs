//! Portal extraction and the transactional World-map transition lifecycle.

use std::{collections::BTreeSet, fmt};

use bevy::{asset::LoadState, prelude::*};

use crate::{
    app_state::AppState,
    game_state::GameState,
    runtime_map::RuntimeMapId,
    scenario_path::ScenarioRelativePath,
    scenario_root::ScenarioRoot,
    scenario_spatial::collision_occupancy::CollisionOccupancy,
    scenario_spatial::{CardinalDirection, Position},
    tmx_ground_asset::{StaticMapRenderState, TmxGroundAsset},
    tmx_header::{TmxMapDocument, TmxPropertyValue},
    tsx_atlas_asset::TsxAtlasAsset,
    world_actor::WorldActorState,
    world_object::WorldObjectState,
    world_player::{CharacterCollisionRect, WorldPlayer, WorldPlayerMotion, WorldPlayerSpawnState},
};

const FADE_ALPHA_PER_SECOND: f32 = 300.0 / 255.0;

/// Installs the transition state now consumed by movement and later driven by loaded map assets.
pub(crate) struct WorldTransitionPlugin;

impl Plugin for WorldTransitionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldTransition>()
            .init_resource::<TransitionDestinationLoad>()
            .add_systems(
                OnEnter(AppState::World),
                (reset_transition, spawn_fade_overlay).chain(),
            )
            .add_systems(
                Update,
                (
                    detect_portal_entry,
                    prepare_destination_load,
                    advance_transition_fade,
                    drive_transition_loading,
                    update_fade_overlay,
                )
                    .chain()
                    .run_if(in_state(AppState::World)),
            )
            .add_systems(
                OnExit(AppState::World),
                (reset_transition, cleanup_fade_overlay),
            );
    }
}

/// A source-authored rectangle portal with a validated destination.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RuntimePortal {
    object_id: u32,
    name: Option<String>,
    bounds: PortalBounds,
    target_map: RuntimeMapId,
    target_position: Position,
}

impl RuntimePortal {
    pub(crate) const fn object_id(&self) -> u32 {
        self.object_id
    }

    #[cfg(test)]
    pub(crate) fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The portal's source-pixel rectangle, e.g. for the `RPG_S1_DEBUG_COLLISION` overlay
    /// (`world_debug_overlay`), which mirrors the source's `_render_portal_debug`.
    pub(crate) const fn bounds(&self) -> PortalBounds {
        self.bounds
    }

    pub(crate) fn target_map(&self) -> &RuntimeMapId {
        &self.target_map
    }

    pub(crate) const fn target_position(&self) -> Position {
        self.target_position
    }
}

/// Positive finite source-pixel rectangle, using Tiled's top-left coordinate space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PortalBounds {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
}

impl PortalBounds {
    /// Mirrors the source's `Portal.is_triggered_by` (`engine/world/portal_data.py`): the player's
    /// live pixel collision rect against this portal's pixel rect, inclusive on all four edges —
    /// so a zero-size point portal (`width`/`height` both 0) still triggers on exact touch.
    fn is_triggered_by(self, rect: CharacterCollisionRect) -> bool {
        let rect_x = f64::from(rect.x);
        let rect_y = f64::from(rect.y);
        let rect_w = f64::from(rect.width);
        let rect_h = f64::from(rect.height);
        rect_x <= self.x + self.width
            && rect_x + rect_w >= self.x
            && rect_y <= self.y + self.height
            && rect_y + rect_h >= self.y
    }
}

/// Converts only property-bearing objects in the reserved `portals` group.
pub(crate) fn runtime_portals(
    document: &TmxMapDocument,
) -> Result<Vec<RuntimePortal>, RuntimePortalError> {
    let Some(group) = document
        .object_groups()
        .iter()
        .find(|group| group.name() == "portals")
    else {
        return Ok(Vec::new());
    };

    group
        .objects()
        .iter()
        .filter(|object| !object.properties().is_empty())
        .map(|object| {
            let property = |name: &str| {
                object
                    .properties()
                    .iter()
                    .find(|property| property.name() == name)
                    .map(|property| property.value())
            };
            let Some(TmxPropertyValue::String(target_map)) = property("target_map") else {
                return Err(RuntimePortalError::InvalidProperties(object.id()));
            };
            let Some(TmxPropertyValue::Integer(target_x)) = property("target_position_x") else {
                return Err(RuntimePortalError::InvalidProperties(object.id()));
            };
            let Some(TmxPropertyValue::Integer(target_y)) = property("target_position_y") else {
                return Err(RuntimePortalError::InvalidProperties(object.id()));
            };
            let target_x = i32::try_from(*target_x)
                .map_err(|_| RuntimePortalError::TargetOutOfRange(object.id()))?;
            let target_y = i32::try_from(*target_y)
                .map_err(|_| RuntimePortalError::TargetOutOfRange(object.id()))?;
            let target_map = RuntimeMapId::try_new(target_map.clone())
                .map_err(|_| RuntimePortalError::EmptyTargetMap(object.id()))?;

            Ok(RuntimePortal {
                object_id: object.id(),
                name: object.name().map(str::to_owned),
                bounds: PortalBounds {
                    x: object.x(),
                    y: object.y(),
                    width: object.width(),
                    height: object.height(),
                },
                target_map,
                target_position: Position::new(target_x, target_y),
            })
        })
        .collect()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RuntimePortalError {
    InvalidProperties(u32),
    TargetOutOfRange(u32),
    EmptyTargetMap(u32),
}

impl fmt::Display for RuntimePortalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProperties(id) => {
                write!(formatter, "portal object {id} has invalid properties")
            }
            Self::TargetOutOfRange(id) => write!(
                formatter,
                "portal object {id} target is outside the i32 tile domain"
            ),
            Self::EmptyTargetMap(id) => {
                write!(formatter, "portal object {id} has an empty target map")
            }
        }
    }
}

impl std::error::Error for RuntimePortalError {}

/// Remembers contact until the player exits, so one rectangle emits only once per entry.
#[derive(Debug, Default)]
pub(crate) struct PortalEntryDetector {
    overlapping: BTreeSet<u32>,
}

impl PortalEntryDetector {
    pub(crate) fn entered<'a>(
        &mut self,
        portals: &'a [RuntimePortal],
        player_rect: CharacterCollisionRect,
    ) -> Option<&'a RuntimePortal> {
        let now = portals
            .iter()
            .filter(|portal| portal.bounds.is_triggered_by(player_rect))
            .map(RuntimePortal::object_id)
            .collect::<BTreeSet<_>>();
        let entered = portals.iter().find(|portal| {
            now.contains(&portal.object_id) && !self.overlapping.contains(&portal.object_id)
        });
        self.overlapping = now;
        entered
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum TransitionPhase {
    /// Initial World entry and every successful load starts black and unlocks after fading in.
    #[default]
    FadingIn,
    Idle,
    FadingOut,
    Loading,
    Publishing,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PendingTransition {
    pub(crate) target_map: RuntimeMapId,
    pub(crate) target_position: Position,
    pub(crate) facing: CardinalDirection,
}

/// One transition transaction. Location is committed only after destination publication succeeds.
#[derive(Debug, Resource)]
pub(crate) struct WorldTransition {
    phase: TransitionPhase,
    alpha: f32,
    pending: Option<PendingTransition>,
    detector: PortalEntryDetector,
    suppress_entry_until_exit: bool,
    failure: Option<String>,
}

impl Default for WorldTransition {
    fn default() -> Self {
        Self {
            phase: TransitionPhase::FadingIn,
            alpha: 1.0,
            pending: None,
            detector: PortalEntryDetector::default(),
            suppress_entry_until_exit: false,
            failure: None,
        }
    }
}

impl WorldTransition {
    /// An already-settled transition (`Idle`, fully unlocked), for tests that exercise
    /// interaction/movement systems without running the fade-in this resource's `Default` starts
    /// in — `Default` intentionally begins `FadingIn`/input-locked to match every real World
    /// entry, which would otherwise leave input permanently locked in a fixture that never drives
    /// `WorldTransitionPlugin`'s own fade-decay system.
    #[cfg(test)]
    pub(crate) fn idle_for_test() -> Self {
        Self {
            phase: TransitionPhase::Idle,
            alpha: 0.0,
            pending: None,
            detector: PortalEntryDetector::default(),
            suppress_entry_until_exit: false,
            failure: None,
        }
    }

    #[cfg(test)]
    pub(crate) const fn phase(&self) -> TransitionPhase {
        self.phase
    }

    pub(crate) const fn alpha(&self) -> f32 {
        self.alpha
    }

    pub(crate) const fn input_locked(&self) -> bool {
        !matches!(self.phase, TransitionPhase::Idle)
    }

    pub(crate) fn pending(&self) -> Option<&PendingTransition> {
        self.pending.as_ref()
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "transition diagnostics are exposed before the World error UI"
        )
    )]
    pub(crate) fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }

    pub(crate) fn request(&mut self, portal: &RuntimePortal, facing: CardinalDirection) -> bool {
        self.request_destination(portal.target_map.clone(), portal.target_position, facing)
    }

    /// Requests the same transactional fade/load/publish path for a validated non-portal move.
    pub(crate) fn request_destination(
        &mut self,
        target_map: RuntimeMapId,
        target_position: Position,
        facing: CardinalDirection,
    ) -> bool {
        if self.phase != TransitionPhase::Idle {
            return false;
        }
        self.pending = Some(PendingTransition {
            target_map,
            target_position,
            facing,
        });
        self.failure = None;
        self.phase = TransitionPhase::FadingOut;
        true
    }

    pub(crate) fn advance_fade(&mut self, seconds: f32) {
        let delta = (seconds.max(0.0) * FADE_ALPHA_PER_SECOND).min(1.0);
        match self.phase {
            TransitionPhase::FadingIn => {
                self.alpha = (self.alpha - delta).max(0.0);
                if self.alpha == 0.0 {
                    self.phase = TransitionPhase::Idle;
                }
            }
            TransitionPhase::FadingOut => {
                self.alpha = (self.alpha + delta).min(1.0);
                if self.alpha == 1.0 {
                    self.phase = TransitionPhase::Loading;
                }
            }
            _ => {}
        }
    }

    pub(crate) fn destination_published(&mut self) {
        debug_assert_eq!(self.phase, TransitionPhase::Publishing);
        self.pending = None;
        self.suppress_entry_until_exit = true;
        self.phase = TransitionPhase::FadingIn;
        self.alpha = 1.0;
    }

    fn destination_committed(&mut self) {
        debug_assert_eq!(self.phase, TransitionPhase::Loading);
        self.phase = TransitionPhase::Publishing;
    }

    pub(crate) fn destination_failed(&mut self, reason: impl Into<String>) {
        debug_assert_eq!(self.phase, TransitionPhase::Loading);
        self.pending = None;
        self.failure = Some(reason.into());
        self.phase = TransitionPhase::FadingIn;
        self.alpha = 1.0;
    }

    fn suppress_destination_overlap(
        &mut self,
        portals: &[RuntimePortal],
        player_rect: CharacterCollisionRect,
    ) -> bool {
        if !self.suppress_entry_until_exit {
            return false;
        }
        let _ignored_entry = self.detector.entered(portals, player_rect);
        if self.detector.overlapping.is_empty() {
            self.suppress_entry_until_exit = false;
        }
        true
    }
}

#[derive(Debug, Default, Resource)]
struct TransitionDestinationLoad {
    map_id: Option<String>,
    handle: Option<Handle<TmxGroundAsset>>,
}

#[derive(Component)]
struct WorldFadeOverlay;

fn spawn_fade_overlay(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::ZERO,
            top: Val::ZERO,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::BLACK),
        GlobalZIndex(10_000),
        Pickable::IGNORE,
        WorldFadeOverlay,
    ));
}

fn cleanup_fade_overlay(mut commands: Commands, overlays: Query<Entity, With<WorldFadeOverlay>>) {
    for entity in &overlays {
        commands.entity(entity).despawn();
    }
}

fn update_fade_overlay(
    transition: Res<WorldTransition>,
    mut overlays: Query<&mut BackgroundColor, With<WorldFadeOverlay>>,
) {
    for mut background in &mut overlays {
        background.0 = Color::srgba(0.0, 0.0, 0.0, transition.alpha());
    }
}

fn detect_portal_entry(
    maps: Res<Assets<TmxGroundAsset>>,
    render: Res<StaticMapRenderState>,
    game: Option<Res<GameState>>,
    players: Query<&WorldPlayerMotion, With<WorldPlayer>>,
    mut transition: ResMut<WorldTransition>,
) {
    if transition.phase != TransitionPhase::Idle {
        return;
    }
    let Some(game) = game else {
        return;
    };
    let Some(map) = render.map(&maps) else {
        return;
    };
    let Ok(portals) = runtime_portals(map.document()) else {
        return;
    };
    // Prefer the player's live smoothly-moved pixel collision rect, matching the source's
    // per-frame `Portal.is_triggered_by(col.x, col.y, COLLISION_W, COLLISION_H)` check against the
    // player's actual position. The tile-derived fallback (same formula as
    // `WorldPlayerMotion::from_tile`) only applies in the narrow window before the World player
    // entity has spawned, which `TransitionPhase::Idle` above should already rule out in practice.
    let player_rect = players
        .single()
        .map(|motion| motion.collision_rect())
        .unwrap_or_else(|_| WorldPlayerMotion::from_tile(game.map().position()).collision_rect());
    if transition.suppress_destination_overlap(&portals, player_rect) {
        return;
    }
    let entered = transition.detector.entered(&portals, player_rect).cloned();
    if let Some(portal) = entered {
        transition.request(&portal, game.map().facing());
    }
}

fn prepare_destination_load(
    asset_server: Res<AssetServer>,
    scenario_root: Res<ScenarioRoot>,
    transition: Res<WorldTransition>,
    mut destination: ResMut<TransitionDestinationLoad>,
) {
    let Some(pending) = transition.pending() else {
        *destination = TransitionDestinationLoad::default();
        return;
    };
    let map_id = pending.target_map.as_str();
    if destination.map_id.as_deref() == Some(map_id) {
        return;
    }
    *destination = TransitionDestinationLoad::default();
    let logical = format!("assets/maps/{map_id}.tmx");
    let Ok(logical) = ScenarioRelativePath::try_from(logical.as_str()) else {
        return;
    };
    destination.map_id = Some(map_id.to_owned());
    destination.handle = Some(asset_server.load(scenario_root.resolve(&logical)));
}

fn advance_transition_fade(time: Res<Time>, mut transition: ResMut<WorldTransition>) {
    transition.advance_fade(time.delta_secs());
}

#[expect(
    clippy::too_many_arguments,
    reason = "the transition publish barrier observes every map-scoped World subsystem"
)]
fn drive_transition_loading(
    asset_server: Res<AssetServer>,
    maps: Res<Assets<TmxGroundAsset>>,
    atlases: Res<Assets<TsxAtlasAsset>>,
    destination: Res<TransitionDestinationLoad>,
    render: Res<StaticMapRenderState>,
    player: Res<WorldPlayerSpawnState>,
    actors: Res<WorldActorState>,
    objects: Res<WorldObjectState>,
    game: Option<ResMut<GameState>>,
    mut transition: ResMut<WorldTransition>,
) {
    if transition.phase == TransitionPhase::Publishing {
        let Some(map_id) = transition
            .pending()
            .map(|pending| pending.target_map.as_str())
        else {
            return;
        };
        if render.is_spawned_for(map_id)
            && player.is_spawned_for(map_id)
            && actors.is_spawned_for(map_id)
            && objects.is_spawned_for(map_id)
        {
            transition.destination_published();
        }
        return;
    }
    if transition.phase != TransitionPhase::Loading {
        return;
    }

    let Some(handle) = destination.handle.as_ref() else {
        transition.destination_failed("destination map path is invalid");
        return;
    };
    if matches!(asset_server.load_state(handle.id()), LoadState::Failed(_)) {
        transition.destination_failed("destination map failed to load");
        return;
    }
    if !asset_server.is_loaded_with_dependencies(handle.id()) {
        return;
    }
    let Some(map) = maps.get(handle) else {
        return;
    };
    if let Err(error) = map.visible_bundles(&atlases) {
        transition.destination_failed(format!("destination map cannot render: {error}"));
        return;
    }
    if let Err(error) = CollisionOccupancy::from_tmx_document(map.document()) {
        transition.destination_failed(format!("destination collision is invalid: {error}"));
        return;
    }
    if let Err(error) = runtime_portals(map.document()) {
        transition.destination_failed(format!("destination portals are invalid: {error}"));
        return;
    }

    let Some(mut game) = game else {
        transition.destination_failed("game session disappeared during transition");
        return;
    };
    let Some(pending) = transition.pending().cloned() else {
        return;
    };
    game.map_mut()
        .move_to(pending.target_map, pending.target_position, pending.facing);
    transition.destination_committed();
}

fn reset_transition(
    mut transition: ResMut<WorldTransition>,
    mut destination: ResMut<TransitionDestinationLoad>,
) {
    *transition = WorldTransition::default();
    *destination = TransitionDestinationLoad::default();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{scenario_path::ScenarioRelativePath, tmx_header::parse_tmx_map_document};

    fn portals_for(map_id: &str, document: &str) -> Vec<RuntimePortal> {
        let owner = ScenarioRelativePath::try_from(format!("assets/maps/{map_id}.tmx")).unwrap();
        let document = parse_tmx_map_document(document, &owner).unwrap();
        runtime_portals(&document).unwrap()
    }

    fn ardel_portals() -> Vec<RuntimePortal> {
        portals_for(
            "town_01_ardel",
            include_str!("../assets/scenarios/rusted_kingdoms/assets/maps/town_01_ardel.tmx"),
        )
    }

    fn assert_reversible_link(
        source_map: &str,
        source_document: &str,
        destination_map: &str,
        destination_document: &str,
    ) {
        let outgoing = portals_for(source_map, source_document);
        assert!(
            outgoing
                .iter()
                .any(|portal| portal.target_map().as_str() == destination_map),
            "{source_map} must link to {destination_map}"
        );
        let returning = portals_for(destination_map, destination_document);
        assert!(
            returning
                .iter()
                .any(|portal| portal.target_map().as_str() == source_map),
            "{destination_map} must link back to {source_map}"
        );
    }

    #[test]
    fn ardel_runtime_portals_preserve_source_bounds_and_destinations() {
        let portals = ardel_portals();
        assert_eq!(portals.len(), 6);
        let house = portals
            .iter()
            .find(|portal| portal.name() == Some("house"))
            .unwrap();
        assert_eq!(house.object_id(), 7);
        assert_eq!(
            house.bounds(),
            PortalBounds {
                x: 69.4546,
                y: 124.818,
                width: 53.9015,
                height: 9.59847
            }
        );
        assert_eq!(house.target_map().as_str(), "town_01_ardel_house_01");
        assert_eq!(house.target_position(), Position::new(10, 11));
    }

    #[test]
    fn every_ardel_outgoing_destination_has_a_return_portal() {
        let portals = ardel_portals();
        let destinations = portals
            .iter()
            .map(|portal| portal.target_map().as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            destinations,
            [
                "zone_01_starting_forest",
                "town_01_ardel_house_01",
                "town_01_ardel_shop_01",
                "town_01_ardel_inn_01",
                "zone_01_starting_forest",
                "town_01_ardel_shrine",
            ]
        );

        let return_documents = [
            (
                "town_01_ardel_house_01",
                include_str!(
                    "../assets/scenarios/rusted_kingdoms/assets/maps/town_01_ardel_house_01.tmx"
                ),
            ),
            (
                "town_01_ardel_shop_01",
                include_str!(
                    "../assets/scenarios/rusted_kingdoms/assets/maps/town_01_ardel_shop_01.tmx"
                ),
            ),
            (
                "town_01_ardel_inn_01",
                include_str!(
                    "../assets/scenarios/rusted_kingdoms/assets/maps/town_01_ardel_inn_01.tmx"
                ),
            ),
            (
                "town_01_ardel_shrine",
                include_str!(
                    "../assets/scenarios/rusted_kingdoms/assets/maps/town_01_ardel_shrine.tmx"
                ),
            ),
            (
                "zone_01_starting_forest",
                include_str!(
                    "../assets/scenarios/rusted_kingdoms/assets/maps/zone_01_starting_forest.tmx"
                ),
            ),
        ];
        for (map_id, document) in return_documents {
            let returns_to_ardel = portals_for(map_id, document)
                .into_iter()
                .filter(|portal| portal.target_map().as_str() == "town_01_ardel")
                .count();
            assert!(
                returns_to_ardel > 0,
                "{map_id} must have a loadable return portal to Ardel"
            );
        }
    }

    #[test]
    fn ardel_house_portal_is_a_reversible_link() {
        assert_reversible_link(
            "town_01_ardel",
            include_str!("../assets/scenarios/rusted_kingdoms/assets/maps/town_01_ardel.tmx"),
            "town_01_ardel_house_01",
            include_str!(
                "../assets/scenarios/rusted_kingdoms/assets/maps/town_01_ardel_house_01.tmx"
            ),
        );
        let returns = portals_for(
            "town_01_ardel_house_01",
            include_str!(
                "../assets/scenarios/rusted_kingdoms/assets/maps/town_01_ardel_house_01.tmx"
            ),
        );
        assert_eq!(returns.len(), 1);
        assert_eq!(returns[0].target_position(), Position::new(3, 4));
    }

    #[test]
    fn ardel_shop_portal_is_a_reversible_link() {
        assert_reversible_link(
            "town_01_ardel",
            include_str!("../assets/scenarios/rusted_kingdoms/assets/maps/town_01_ardel.tmx"),
            "town_01_ardel_shop_01",
            include_str!(
                "../assets/scenarios/rusted_kingdoms/assets/maps/town_01_ardel_shop_01.tmx"
            ),
        );
        let returns = portals_for(
            "town_01_ardel_shop_01",
            include_str!(
                "../assets/scenarios/rusted_kingdoms/assets/maps/town_01_ardel_shop_01.tmx"
            ),
        );
        assert_eq!(returns.len(), 1);
        assert_eq!(returns[0].target_position(), Position::new(15, 4));
    }

    #[test]
    fn ardel_inn_portal_is_a_reversible_link() {
        assert_reversible_link(
            "town_01_ardel",
            include_str!("../assets/scenarios/rusted_kingdoms/assets/maps/town_01_ardel.tmx"),
            "town_01_ardel_inn_01",
            include_str!(
                "../assets/scenarios/rusted_kingdoms/assets/maps/town_01_ardel_inn_01.tmx"
            ),
        );
        let returns = portals_for(
            "town_01_ardel_inn_01",
            include_str!(
                "../assets/scenarios/rusted_kingdoms/assets/maps/town_01_ardel_inn_01.tmx"
            ),
        );
        assert_eq!(returns.len(), 1);
        assert_eq!(returns[0].target_position(), Position::new(24, 4));
    }

    #[test]
    fn ardel_shrine_portal_is_a_reversible_link() {
        assert_reversible_link(
            "town_01_ardel",
            include_str!("../assets/scenarios/rusted_kingdoms/assets/maps/town_01_ardel.tmx"),
            "town_01_ardel_shrine",
            include_str!(
                "../assets/scenarios/rusted_kingdoms/assets/maps/town_01_ardel_shrine.tmx"
            ),
        );
        let returns = portals_for(
            "town_01_ardel_shrine",
            include_str!(
                "../assets/scenarios/rusted_kingdoms/assets/maps/town_01_ardel_shrine.tmx"
            ),
        );
        assert_eq!(returns.len(), 1);
        assert_eq!(returns[0].target_position(), Position::new(3, 18));
    }

    #[test]
    fn starting_forest_portals_cover_both_ardel_routes_and_the_reversible_wave_boundary() {
        let forest_document = include_str!(
            "../assets/scenarios/rusted_kingdoms/assets/maps/zone_01_starting_forest.tmx"
        );
        let forest = portals_for("zone_01_starting_forest", forest_document);
        assert_eq!(forest.len(), 3);
        let destinations = forest
            .iter()
            .map(|portal| (portal.target_map().as_str(), portal.target_position()))
            .collect::<Vec<_>>();
        assert_eq!(
            destinations,
            [
                ("town_01_ardel", Position::new(27, 12)),
                ("zone_02_open_plains", Position::new(28, 6)),
                ("town_01_ardel", Position::new(20, 17)),
            ]
        );
        assert_eq!(
            ardel_portals()
                .iter()
                .filter(|portal| portal.target_map().as_str() == "zone_01_starting_forest")
                .count(),
            2
        );
        assert_reversible_link(
            "zone_01_starting_forest",
            forest_document,
            "zone_02_open_plains",
            include_str!("../assets/scenarios/rusted_kingdoms/assets/maps/zone_02_open_plains.tmx"),
        );
    }

    fn open_plains_document() -> &'static str {
        include_str!("../assets/scenarios/rusted_kingdoms/assets/maps/zone_02_open_plains.tmx")
    }

    fn open_plains_portals() -> Vec<RuntimePortal> {
        portals_for("zone_02_open_plains", open_plains_document())
    }

    fn millhaven_document() -> &'static str {
        include_str!("../assets/scenarios/rusted_kingdoms/assets/maps/town_02_millhaven.tmx")
    }

    fn millhaven_portals() -> Vec<RuntimePortal> {
        portals_for("town_02_millhaven", millhaven_document())
    }

    #[test]
    fn open_plains_portals_cover_all_four_authored_exits() {
        let portals = open_plains_portals();
        let destinations = portals
            .iter()
            .map(|portal| (portal.target_map().as_str(), portal.target_position()))
            .collect::<Vec<_>>();
        assert_eq!(
            destinations,
            [
                ("zone_02_open_plains_cave_02", Position::new(8, 27)),
                ("zone_01_starting_forest", Position::new(1, 26)),
                ("town_02_millhaven", Position::new(19, 29)),
                ("zone_03_marshland", Position::new(27, 1)),
            ]
        );
    }

    #[test]
    fn open_plains_cave_chain_links_plains_and_cave_02_and_cave_01_with_no_direct_plains_cave_01_link()
     {
        let cave_01_document = include_str!(
            "../assets/scenarios/rusted_kingdoms/assets/maps/zone_02_open_plains_cave_01.tmx"
        );
        let cave_02_document = include_str!(
            "../assets/scenarios/rusted_kingdoms/assets/maps/zone_02_open_plains_cave_02.tmx"
        );
        let cave_01 = portals_for("zone_02_open_plains_cave_01", cave_01_document);
        let cave_02 = portals_for("zone_02_open_plains_cave_02", cave_02_document);

        // Cave_01's sole exit leads deeper into cave_02, never directly back to the plains.
        assert_eq!(cave_01.len(), 1);
        assert_eq!(
            cave_01[0].target_map().as_str(),
            "zone_02_open_plains_cave_02"
        );
        assert_eq!(cave_01[0].target_position(), Position::new(58, 19));
        assert!(
            !cave_01
                .iter()
                .any(|portal| portal.target_map().as_str() == "zone_02_open_plains")
        );

        // Cave_02 is the hinge: one exit returns to the plains, the other continues to cave_01.
        assert_eq!(cave_02.len(), 2);
        let cave_02_destinations = cave_02
            .iter()
            .map(|portal| (portal.target_map().as_str(), portal.target_position()))
            .collect::<Vec<_>>();
        assert_eq!(
            cave_02_destinations,
            [
                ("zone_02_open_plains", Position::new(25, 3)),
                ("zone_02_open_plains_cave_01", Position::new(1, 3)),
            ]
        );

        assert_reversible_link(
            "zone_02_open_plains",
            open_plains_document(),
            "zone_02_open_plains_cave_02",
            cave_02_document,
        );
        assert_reversible_link(
            "zone_02_open_plains_cave_01",
            cave_01_document,
            "zone_02_open_plains_cave_02",
            cave_02_document,
        );
    }

    #[test]
    fn millhaven_portals_cover_all_four_authored_exits_with_reversible_return_links() {
        let portals = millhaven_portals();
        let destinations = portals
            .iter()
            .map(|portal| (portal.target_map().as_str(), portal.target_position()))
            .collect::<Vec<_>>();
        assert_eq!(
            destinations,
            [
                ("town_02_millhaven_mill", Position::new(10, 11)),
                ("town_02_millhaven_shop", Position::new(7, 10)),
                ("town_02_millhaven_inn", Position::new(5, 9)),
                ("zone_02_open_plains", Position::new(2, 1)),
            ]
        );

        let interiors = [
            (
                "town_02_millhaven_inn",
                include_str!(
                    "../assets/scenarios/rusted_kingdoms/assets/maps/town_02_millhaven_inn.tmx"
                ),
                Position::new(9, 25),
            ),
            (
                "town_02_millhaven_mill",
                include_str!(
                    "../assets/scenarios/rusted_kingdoms/assets/maps/town_02_millhaven_mill.tmx"
                ),
                Position::new(20, 6),
            ),
            (
                "town_02_millhaven_shop",
                include_str!(
                    "../assets/scenarios/rusted_kingdoms/assets/maps/town_02_millhaven_shop.tmx"
                ),
                Position::new(36, 6),
            ),
        ];
        for (map_id, document, expected_return_position) in interiors {
            assert_reversible_link("town_02_millhaven", millhaven_document(), map_id, document);
            let returns = portals_for(map_id, document);
            assert_eq!(returns.len(), 1);
            assert_eq!(returns[0].target_map().as_str(), "town_02_millhaven");
            assert_eq!(returns[0].target_position(), expected_return_position);
        }

        assert_reversible_link(
            "town_02_millhaven",
            millhaven_document(),
            "zone_02_open_plains",
            open_plains_document(),
        );
    }

    fn marshland_document() -> &'static str {
        include_str!("../assets/scenarios/rusted_kingdoms/assets/maps/zone_03_marshland.tmx")
    }

    fn marshland_portals() -> Vec<RuntimePortal> {
        portals_for("zone_03_marshland", marshland_document())
    }

    fn harborgate_document() -> &'static str {
        include_str!("../assets/scenarios/rusted_kingdoms/assets/maps/port_town_harborgate.tmx")
    }

    fn harborgate_portals() -> Vec<RuntimePortal> {
        portals_for("port_town_harborgate", harborgate_document())
    }

    #[test]
    fn marshland_portals_cover_all_three_authored_exits_including_the_w12_4_boundary() {
        let portals = marshland_portals();
        let destinations = portals
            .iter()
            .map(|portal| (portal.target_map().as_str(), portal.target_position()))
            .collect::<Vec<_>>();
        assert_eq!(
            destinations,
            [
                ("zone_02_open_plains", Position::new(3, 18)),
                ("zone_04_ancient_ruins_01_gate", Position::new(39, 14)),
                ("port_town_harborgate", Position::new(4, 1)),
            ]
        );

        // The W12.2 boundary is reversible: the plains' Marshland exit lands back on
        // this same portal's return position.
        assert_reversible_link(
            "zone_03_marshland",
            marshland_document(),
            "zone_02_open_plains",
            open_plains_document(),
        );
        let plains_to_marshland = open_plains_portals()
            .into_iter()
            .find(|portal| portal.target_map().as_str() == "zone_03_marshland")
            .unwrap();
        assert_eq!(plains_to_marshland.target_position(), Position::new(27, 1));

        // The W12.4 boundary (Ancient Ruins) only needs to parse here; the destination
        // map is out of this wave's scope and asserting a reverse link would be a false
        // claim about content this wave does not own.
        assert!(
            portals
                .iter()
                .any(|portal| portal.target_map().as_str() == "zone_04_ancient_ruins_01_gate")
        );
    }

    #[test]
    fn harborgate_portals_cover_all_five_authored_exits_with_reversible_return_links() {
        let portals = harborgate_portals();
        let destinations = portals
            .iter()
            .map(|portal| (portal.target_map().as_str(), portal.target_position()))
            .collect::<Vec<_>>();
        assert_eq!(
            destinations,
            [
                ("port_town_harborgate_quarantine", Position::new(5, 9)),
                ("port_town_harborgate_shop", Position::new(7, 10)),
                ("port_town_harborgate_inn", Position::new(5, 9)),
                ("port_town_harborgate_harbormaster", Position::new(10, 11)),
                ("zone_03_marshland", Position::new(21, 37)),
            ]
        );

        let interiors = [
            (
                "port_town_harborgate_quarantine",
                include_str!(
                    "../assets/scenarios/rusted_kingdoms/assets/maps/port_town_harborgate_quarantine.tmx"
                ),
                Position::new(20, 6),
            ),
            (
                "port_town_harborgate_shop",
                include_str!(
                    "../assets/scenarios/rusted_kingdoms/assets/maps/port_town_harborgate_shop.tmx"
                ),
                Position::new(36, 6),
            ),
            (
                "port_town_harborgate_inn",
                include_str!(
                    "../assets/scenarios/rusted_kingdoms/assets/maps/port_town_harborgate_inn.tmx"
                ),
                Position::new(9, 25),
            ),
            (
                "port_town_harborgate_harbormaster",
                include_str!(
                    "../assets/scenarios/rusted_kingdoms/assets/maps/port_town_harborgate_harbormaster.tmx"
                ),
                Position::new(35, 26),
            ),
        ];
        for (map_id, document, expected_return_position) in interiors {
            assert_reversible_link(
                "port_town_harborgate",
                harborgate_document(),
                map_id,
                document,
            );
            let returns = portals_for(map_id, document);
            assert_eq!(returns.len(), 1);
            assert_eq!(returns[0].target_map().as_str(), "port_town_harborgate");
            assert_eq!(returns[0].target_position(), expected_return_position);
        }

        assert_reversible_link(
            "port_town_harborgate",
            harborgate_document(),
            "zone_03_marshland",
            marshland_document(),
        );
    }

    /// A player collision rect (the live 20x18 source-pixel size) at an arbitrary top-left.
    fn collision_rect(x: f32, y: f32) -> CharacterCollisionRect {
        rect(x, y, 20.0, 18.0)
    }

    /// A collision rect of an arbitrary size, for boundary-touching assertions that need a size
    /// other than the player's own 20x18.
    fn rect(x: f32, y: f32, width: f32, height: f32) -> CharacterCollisionRect {
        CharacterCollisionRect {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn entry_detector_emits_once_until_player_leaves_and_reenters() {
        let portals = ardel_portals();
        let mut detector = PortalEntryDetector::default();
        // town_01_ardel's "house" portal is the pixel rect x:[69.4546, 123.3561],
        // y:[124.818, 134.416]; this rect overlaps only that portal among Ardel's six.
        let on_house = collision_rect(100.0, 120.0);
        // Far from every authored Ardel portal (shop/inn/hole/shrine/to_zone_01 included).
        let away = collision_rect(300.0, 300.0);
        assert_eq!(
            detector
                .entered(&portals, on_house)
                .map(RuntimePortal::object_id),
            Some(7)
        );
        assert!(detector.entered(&portals, away).is_none());
        assert!(detector.entered(&portals, away).is_none());
        assert_eq!(
            detector
                .entered(&portals, on_house)
                .map(RuntimePortal::object_id),
            Some(7)
        );
    }

    #[test]
    fn destination_portal_contact_is_seeded_until_the_player_exits() {
        let portals = ardel_portals();
        let house = portals
            .iter()
            .find(|portal| portal.name() == Some("house"))
            .unwrap();
        let mut transition = WorldTransition {
            phase: TransitionPhase::Publishing,
            pending: Some(PendingTransition {
                target_map: RuntimeMapId::try_new("town_01_ardel").unwrap(),
                target_position: Position::new(3, 4),
                facing: CardinalDirection::Down,
            }),
            ..default()
        };

        transition.destination_published();
        transition.phase = TransitionPhase::Idle;
        let on_house = collision_rect(100.0, 120.0);
        let away = collision_rect(300.0, 300.0);
        assert!(transition.suppress_destination_overlap(&portals, on_house));
        assert!(transition.suppress_destination_overlap(&portals, on_house));
        assert!(transition.suppress_destination_overlap(&portals, away));
        assert!(!transition.suppress_destination_overlap(&portals, away));
        assert_eq!(
            transition
                .detector
                .entered(&portals, on_house)
                .map(RuntimePortal::object_id),
            Some(house.object_id())
        );
    }

    /// Pins `PortalBounds::is_triggered_by` against the source's `Portal.is_triggered_by`
    /// (`engine/world/portal_data.py`): edges are inclusive, and a zero-size point portal (a
    /// Tiled point object, `width`/`height` both 0) still triggers on exact touch.
    #[test]
    fn portal_trigger_matches_source_inclusive_edge_and_point_portal_semantics() {
        let point = PortalBounds {
            x: 100.0,
            y: 100.0,
            width: 0.0,
            height: 0.0,
        };
        // Rect's bottom-right corner lands exactly on the point: touches, so it triggers.
        assert!(point.is_triggered_by(rect(90.0, 90.0, 10.0, 10.0)));
        // One pixel short on each axis: no longer reaches the point.
        assert!(!point.is_triggered_by(rect(89.0, 89.0, 10.0, 10.0)));

        let portal = PortalBounds {
            x: 50.0,
            y: 50.0,
            width: 20.0,
            height: 20.0,
        };
        // Rect's left edge lands exactly on the portal's right edge (x = 70): the source's
        // `<=`/`>=` comparison counts this as a trigger, unlike a strict-inequality tile-box test.
        assert!(portal.is_triggered_by(rect(70.0, 55.0, 10.0, 10.0)));
        // Symmetric case on the portal's left edge (x = 50).
        assert!(portal.is_triggered_by(rect(30.0, 55.0, 20.0, 10.0)));
        // Half a pixel further out no longer touches either edge.
        assert!(!portal.is_triggered_by(rect(70.5, 55.0, 10.0, 10.0)));
    }

    #[test]
    fn fade_locks_input_and_request_cannot_duplicate() {
        let portal = &ardel_portals()[1];
        let mut transition = WorldTransition::default();
        assert!(transition.input_locked());
        transition.advance_fade(1.0);
        assert_eq!(transition.phase(), TransitionPhase::Idle);
        assert!(!transition.input_locked());
        assert!(transition.request(portal, CardinalDirection::Up));
        assert!(!transition.request(portal, CardinalDirection::Down));
        transition.advance_fade(1.0);
        assert_eq!(transition.phase(), TransitionPhase::Loading);
        assert!(transition.input_locked());
        assert_eq!(transition.pending().unwrap().facing, CardinalDirection::Up);
        transition.destination_committed();
        transition.destination_published();
        assert_eq!(transition.phase(), TransitionPhase::FadingIn);
        transition.advance_fade(1.0);
        assert_eq!(transition.phase(), TransitionPhase::Idle);
        assert!(!transition.input_locked());
    }

    #[test]
    fn direct_destination_uses_the_same_transaction_and_rejects_while_busy() {
        let destination = RuntimeMapId::try_new("zone_01_starting_forest").unwrap();
        let position = Position::new(29, 1);
        let mut transition = WorldTransition {
            phase: TransitionPhase::Idle,
            alpha: 0.0,
            ..default()
        };

        assert!(transition.request_destination(
            destination.clone(),
            position,
            CardinalDirection::Down
        ));
        assert!(!transition.request_destination(
            destination.clone(),
            position,
            CardinalDirection::Up
        ));
        assert_eq!(transition.phase(), TransitionPhase::FadingOut);
        assert_eq!(transition.pending().unwrap().target_map, destination);
        assert_eq!(transition.pending().unwrap().target_position, position);
        assert_eq!(
            transition.pending().unwrap().facing,
            CardinalDirection::Down
        );
    }

    #[test]
    fn failed_destination_keeps_source_location_and_visited_history_unchanged() {
        let source = RuntimeMapId::try_new("town_01_ardel").unwrap();
        let mut map = crate::runtime_map::RuntimeMapState::new(
            source.clone(),
            Position::new(2, 3),
            CardinalDirection::Up,
        );
        let portal = ardel_portals()
            .into_iter()
            .find(|portal| portal.name() == Some("house"))
            .unwrap();
        let mut transition = WorldTransition {
            phase: TransitionPhase::Idle,
            alpha: 0.0,
            ..default()
        };
        assert!(transition.request(&portal, CardinalDirection::Up));
        transition.advance_fade(1.0);
        transition.destination_failed("invented load failure");

        assert_eq!(map.current(), Some(&source));
        assert_eq!(map.position(), Position::new(2, 3));
        assert_eq!(map.visited().count(), 0);
        assert_eq!(transition.failure(), Some("invented load failure"));

        let pending = PendingTransition {
            target_map: RuntimeMapId::try_new("town_01_ardel_house_01").unwrap(),
            target_position: Position::new(10, 11),
            facing: CardinalDirection::Up,
        };
        map.move_to(pending.target_map, pending.target_position, pending.facing);
        assert!(map.has_visited(&source));
        assert_eq!(map.position(), Position::new(10, 11));
    }
}
