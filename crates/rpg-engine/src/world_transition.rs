//! Portal extraction and the transactional World-map transition lifecycle.

use std::{collections::BTreeSet, fmt};

use bevy::prelude::*;

pub(crate) mod hot_reload;
mod recovery;
use recovery::{
    TransitionRecovery, drive_transition_loading, handle_recovery_input, update_recovery_ui,
};

use crate::{
    app_state::AppState,
    game_state::GameState,
    runtime_map::RuntimeMapId,
    scenario_inventory::ScenarioInventory,
    scenario_root::ScenarioRoot,
    scenario_spatial::{CardinalDirection, Position},
    sfx_cue::{PlaySfx, cue},
    tmx_ground_asset::{StaticMapRenderState, TmxGroundAsset},
    tmx_header::{TmxMapDocument, TmxPropertyValue},
    world_player::{CharacterCollisionRect, WorldPlayer, WorldPlayerMotion},
};

const FADE_ALPHA_PER_SECOND: f32 = 300.0 / 255.0;

/// Installs the transition state now consumed by movement and later driven by loaded map assets.
pub(crate) struct WorldTransitionPlugin;

impl Plugin for WorldTransitionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldTransition>()
            .init_resource::<TransitionDestinationLoad>()
            .init_resource::<TransitionRecovery>()
            .configure_sets(
                Update,
                crate::world_publication::WorldPublicationSet::Load
                    .run_if(recovery::loaders_enabled),
            )
            .configure_sets(
                Update,
                (
                    crate::world_publication::WorldPublicationSet::Load,
                    crate::world_publication::WorldPublicationSet::Barrier,
                )
                    .chain(),
            )
            .add_systems(
                OnEnter(AppState::World),
                (reset_transition, spawn_fade_overlay).chain(),
            )
            .add_systems(
                Update,
                (
                    handle_recovery_input,
                    detect_portal_entry,
                    prepare_destination_load,
                    advance_transition_fade,
                    drive_transition_loading,
                    update_fade_overlay,
                    update_recovery_ui,
                )
                    .chain()
                    .in_set(crate::world_publication::WorldPublicationSet::Barrier)
                    .run_if(in_state(AppState::World)),
            )
            .add_systems(
                OnExit(AppState::World),
                (
                    recovery::cleanup_recovery,
                    reset_transition,
                    cleanup_fade_overlay,
                )
                    .chain(),
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
    /// A successfully published World fades from black before input unlocks.
    #[default]
    FadingIn,
    Idle,
    FadingOut,
    Loading,
    Publishing,
    Failed,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PendingTransition {
    pub(crate) target_map: RuntimeMapId,
    pub(crate) target_position: Position,
    pub(crate) facing: CardinalDirection,
}

/// A locked map transaction; recovery retains the complete source until publication succeeds.
#[derive(Debug, Resource)]
pub(crate) struct WorldTransition {
    phase: TransitionPhase,
    alpha: f32,
    pending: Option<PendingTransition>,
    detector: PortalEntryDetector,
    suppress_entry_until_exit: bool,
    failure: Option<String>,
    wait_seconds: f32,
}

impl Default for WorldTransition {
    fn default() -> Self {
        Self {
            phase: TransitionPhase::Publishing,
            alpha: 1.0,
            pending: None,
            detector: PortalEntryDetector::default(),
            suppress_entry_until_exit: false,
            failure: None,
            wait_seconds: 0.0,
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
            wait_seconds: 0.0,
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
        self.wait_seconds = 0.0;
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
        self.wait_seconds = 0.0;
    }

    pub(crate) fn destination_failed(&mut self, reason: impl Into<String>) {
        self.failure = Some(reason.into());
        self.phase = TransitionPhase::Failed;
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
    mut sfx: MessageWriter<PlaySfx>,
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
    if let Some(portal) = entered
        && transition.request(&portal, game.map().facing())
    {
        // Walking through a doorway was silent; the fade carried it alone.
        sfx.write(PlaySfx::new(cue::DOOR));
    }
}

pub(crate) fn transition_settled(transition: Option<Res<WorldTransition>>) -> bool {
    transition.is_none_or(|transition| !transition.input_locked())
}

pub(crate) fn map_audio_ready(transition: Option<Res<WorldTransition>>) -> bool {
    transition.is_none_or(|transition| {
        !matches!(
            transition.phase,
            TransitionPhase::Publishing | TransitionPhase::Failed
        )
    })
}

fn prepare_destination_load(
    asset_server: Res<AssetServer>,
    scenario_root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
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
    let Some(logical) = inventory.tmx_path(map_id) else {
        return;
    };
    destination.map_id = Some(map_id.to_owned());
    destination.handle = Some(asset_server.load(scenario_root.resolve(&logical)));
}

fn advance_transition_fade(time: Res<Time>, mut transition: ResMut<WorldTransition>) {
    transition.advance_fade(time.delta_secs());
}

fn reset_transition(
    game: Option<Res<GameState>>,
    encounter_restore: Option<Res<crate::world_encounter::WorldEncounterRestore>>,
    mut recovery: ResMut<TransitionRecovery>,
    mut transition: ResMut<WorldTransition>,
    mut destination: ResMut<TransitionDestinationLoad>,
) {
    recovery.initial_game = game.as_deref().cloned();
    recovery.initial_encounters = encounter_restore.as_deref().cloned();
    *transition = WorldTransition::default();
    *destination = TransitionDestinationLoad::default();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{scenario_path::ScenarioRelativePath, tmx_header::parse_tmx_map_document};

    #[test]
    fn editor_hidden_portal_objects_and_groups_remain_active() {
        let xml = r#"<map orientation="orthogonal" width="4" height="4" tilewidth="32" tileheight="32">
            <objectgroup id="1" name="portals" visible="0">
                <object id="1" x="32" y="0" width="32" height="16" visible="0">
                    <properties>
                        <property name="target_map" value="town"/>
                        <property name="target_position_x" type="int" value="3"/>
                        <property name="target_position_y" type="int" value="4"/>
                    </properties>
                </object>
            </objectgroup>
        </map>"#;
        let owner = ScenarioRelativePath::try_from("media/maps/shrine.tmx").unwrap();
        let document = parse_tmx_map_document(xml, &owner).unwrap();
        let portals = runtime_portals(&document).unwrap();
        assert_eq!(portals.len(), 1);
        assert_eq!(portals[0].target_map().as_str(), "town");
        assert_eq!(portals[0].target_position(), Position::new(3, 4));
        assert!(
            portals[0]
                .bounds()
                .is_triggered_by(rect(40.0, 0.0, 20.0, 18.0))
        );
    }

    #[test]
    fn python_editor_rectangles_points_and_authored_edges_round_trip_runtime_extraction() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let python =
            std::env::var_os("RPG_S1_EDITOR_TEST_PYTHON").unwrap_or_else(|| "python3".into());
        let output = std::process::Command::new(python)
            .current_dir(root)
            .args([
                "-c",
                r#"
from pathlib import Path
from tempfile import TemporaryDirectory
from tools.map_editor.edit.tmx_writer import create_portal, save_portal_target

with TemporaryDirectory(prefix="rpg-editor-runtime-test-") as directory:
    path = Path(directory) / "source.tmx"
    xml = Path("tools/map_editor/tests/fixtures/source.tmx").read_text()
    xml = xml.replace('tilewidth="32" tileheight="32"', 'tilewidth="16" tileheight="24"')
    xml = xml.replace('x="32" y="32" width="32" height="32"',
                      'x="-3.5" y="30.25" width="9.75" height="32.125"')
    path.write_text(xml)
    create_portal(path, (16, 24, 24, 24), "target", (2, 1))
    print(path.read_text(), end="\0")
    point = create_portal(path, (48, 48, 0, 0), "target", (3, 3))
    print(path.read_text(), end="\0")
    save_portal_target(path, point, "source", (1, 2), None)
    print(path.read_text(), end="\0")
    save_portal_target(path, point, "target", (0, 0), (64, 96, 0, 0))
    print(path.read_text(), end="\0")
"#,
            ])
            .output()
            .expect(
                "python3 (or RPG_S1_EDITOR_TEST_PYTHON) must run the stdlib-only editor writer",
            );
        assert!(
            output.status.success(),
            "Python writer failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let xml = String::from_utf8(output.stdout).unwrap();
        let owner = ScenarioRelativePath::try_from("media/maps/source.tmx").unwrap();
        let documents = xml
            .split_terminator('\0')
            .map(|xml| parse_tmx_map_document(xml, &owner).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(documents.len(), 4);
        for (index, document) in documents.iter().enumerate() {
            assert_eq!(document.header().tile_width(), 16);
            assert_eq!(document.header().tile_height(), 24);
            let portals = runtime_portals(document).unwrap();
            assert_eq!(portals.len(), if index == 0 { 2 } else { 3 });
            assert_eq!(
                portals[0].bounds(),
                PortalBounds {
                    x: -3.5,
                    y: 30.25,
                    width: 9.75,
                    height: 32.125
                }
            );
            assert_eq!(portals[1].object_id(), 2);
            assert_eq!(
                portals[1].bounds(),
                PortalBounds {
                    x: 16.0,
                    y: 24.0,
                    width: 24.0,
                    height: 24.0
                }
            );
            assert_eq!(portals[1].target_position(), Position::new(2, 1));
            if index == 0 {
                continue;
            }
            let point = &portals[2];
            assert_eq!(point.object_id(), 3);
            let (x, y, target, tile) = match index {
                1 => (48.0, 48.0, "target", Position::new(3, 3)),
                2 => (48.0, 48.0, "source", Position::new(1, 2)),
                _ => (64.0, 96.0, "target", Position::new(0, 0)),
            };
            assert_eq!(
                point.bounds(),
                PortalBounds {
                    x,
                    y,
                    width: 0.0,
                    height: 0.0
                }
            );
            assert_eq!(point.target_map().as_str(), target);
            assert_eq!(point.target_position(), tile);
            assert!(point.bounds().is_triggered_by(rect(
                x as f32 - 10.0,
                y as f32 - 10.0,
                10.0,
                10.0
            )));
            assert!(!point.bounds().is_triggered_by(rect(
                x as f32 - 11.0,
                y as f32 - 11.0,
                10.0,
                10.0
            )));
        }
    }

    fn portals_for(map_id: &str, document: &str) -> Vec<RuntimePortal> {
        let owner = ScenarioRelativePath::try_from(format!("media/maps/{map_id}.tmx")).unwrap();
        let document = parse_tmx_map_document(document, &owner).unwrap();
        runtime_portals(&document).unwrap()
    }

    fn ardel_portals() -> Vec<RuntimePortal> {
        portals_for(
            "town_01_ardel",
            include_str!(scenario_file!("media/maps/town_01_ardel.tmx")),
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
    fn ardel_runtime_portals_use_inset_doorway_bounds_and_preserve_destinations() {
        let portals = ardel_portals();
        assert_eq!(portals.len(), 10);
        let house = portals
            .iter()
            .find(|portal| portal.target_map().as_str() == "town_01_ardel_house_01")
            .unwrap();
        assert_eq!(house.object_id(), 7);
        assert_eq!(house.name(), Some("house_cheif"));
        assert_eq!(
            house.bounds(),
            PortalBounds {
                x: 68.0,
                y: 152.0,
                width: 56.0,
                height: 6.0
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
                "town_01_ardel_shop_weapon",
                "town_01_ardel_shop_item",
                "town_01_ardel_shop_mc",
                "town_01_ardel_shop_armor",
            ]
        );

        let return_documents = [
            (
                "town_01_ardel_shop_weapon",
                include_str!(scenario_file!("media/maps/town_01_ardel_shop_weapon.tmx")),
            ),
            (
                "town_01_ardel_shop_armor",
                include_str!(scenario_file!("media/maps/town_01_ardel_shop_armor.tmx")),
            ),
            (
                "town_01_ardel_shop_item",
                include_str!(scenario_file!("media/maps/town_01_ardel_shop_item.tmx")),
            ),
            (
                "town_01_ardel_shop_mc",
                include_str!(scenario_file!("media/maps/town_01_ardel_shop_mc.tmx")),
            ),
            (
                "town_01_ardel_house_01",
                include_str!(scenario_file!("media/maps/town_01_ardel_house_01.tmx")),
            ),
            (
                "town_01_ardel_shop_01",
                include_str!(scenario_file!("media/maps/town_01_ardel_shop_01.tmx")),
            ),
            (
                "town_01_ardel_inn_01",
                include_str!(scenario_file!("media/maps/town_01_ardel_inn_01.tmx")),
            ),
            (
                "town_01_ardel_shrine",
                include_str!(scenario_file!("media/maps/town_01_ardel_shrine.tmx")),
            ),
            (
                "zone_01_starting_forest",
                include_str!(scenario_file!("media/maps/zone_01_starting_forest.tmx")),
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
            include_str!(scenario_file!("media/maps/town_01_ardel.tmx")),
            "town_01_ardel_house_01",
            include_str!(scenario_file!("media/maps/town_01_ardel_house_01.tmx")),
        );
        let returns = portals_for(
            "town_01_ardel_house_01",
            include_str!(scenario_file!("media/maps/town_01_ardel_house_01.tmx")),
        );
        assert_eq!(returns.len(), 1);
        assert_eq!(returns[0].target_position(), Position::new(3, 5));
    }

    #[test]
    fn ardel_shop_portal_is_a_reversible_link() {
        assert_reversible_link(
            "town_01_ardel",
            include_str!(scenario_file!("media/maps/town_01_ardel.tmx")),
            "town_01_ardel_shop_01",
            include_str!(scenario_file!("media/maps/town_01_ardel_shop_01.tmx")),
        );
        let returns = portals_for(
            "town_01_ardel_shop_01",
            include_str!(scenario_file!("media/maps/town_01_ardel_shop_01.tmx")),
        );
        assert_eq!(returns.len(), 1);
        assert_eq!(returns[0].target_position(), Position::new(15, 4));
    }

    #[test]
    fn ardel_inn_portal_is_a_reversible_link() {
        assert_reversible_link(
            "town_01_ardel",
            include_str!(scenario_file!("media/maps/town_01_ardel.tmx")),
            "town_01_ardel_inn_01",
            include_str!(scenario_file!("media/maps/town_01_ardel_inn_01.tmx")),
        );
        let returns = portals_for(
            "town_01_ardel_inn_01",
            include_str!(scenario_file!("media/maps/town_01_ardel_inn_01.tmx")),
        );
        assert_eq!(returns.len(), 1);
        assert_eq!(returns[0].target_position(), Position::new(24, 4));
    }

    /// Roadmap B1.2. The camp is not optional scenery: it was inserted *into* the road, so
    /// zone 9 no longer reaches the stronghold directly and the stronghold no longer reaches
    /// zone 9. Every run now passes the Cinder Marshal.
    #[test]
    fn the_marshal_camp_sits_between_the_volcanic_region_and_the_final_stronghold() {
        const VOLCANIC: &str =
            include_str!(scenario_file!("media/maps/zone_09_volcanic_region.tmx"));
        const CAMP: &str = include_str!(scenario_file!("media/maps/zone_09_marshal_camp.tmx"));
        const STRONGHOLD: &str =
            include_str!(scenario_file!("media/maps/zone_10_final_stronghold.tmx"));

        assert_reversible_link(
            "zone_09_volcanic_region",
            VOLCANIC,
            "zone_09_marshal_camp",
            CAMP,
        );
        assert_reversible_link(
            "zone_09_marshal_camp",
            CAMP,
            "zone_10_final_stronghold",
            STRONGHOLD,
        );

        let volcanic = portals_for("zone_09_volcanic_region", VOLCANIC);
        assert!(
            !volcanic
                .iter()
                .any(|portal| portal.target_map().as_str() == "zone_10_final_stronghold"),
            "the old direct road must be gone, or the camp can be walked past"
        );
        let stronghold = portals_for("zone_10_final_stronghold", STRONGHOLD);
        assert!(
            !stronghold
                .iter()
                .any(|portal| portal.target_map().as_str() == "zone_09_volcanic_region"),
            "and gone in the returning direction too"
        );

        // Two ways in and out, and the eastern one is the single tile the Marshal stands in.
        let camp = portals_for("zone_09_marshal_camp", CAMP);
        assert_eq!(camp.len(), 2);
    }

    #[test]
    fn ardel_shrine_portal_is_a_reversible_link() {
        assert_reversible_link(
            "town_01_ardel",
            include_str!(scenario_file!("media/maps/town_01_ardel.tmx")),
            "town_01_ardel_shrine",
            include_str!(scenario_file!("media/maps/town_01_ardel_shrine.tmx")),
        );
        let returns = portals_for(
            "town_01_ardel_shrine",
            include_str!(scenario_file!("media/maps/town_01_ardel_shrine.tmx")),
        );
        assert_eq!(returns.len(), 1);
        assert_eq!(returns[0].target_position(), Position::new(52, 5));
    }

    #[test]
    fn starting_forest_portals_cover_the_ardel_return_and_reversible_plains_boundary() {
        let forest_document =
            include_str!(scenario_file!("media/maps/zone_01_starting_forest.tmx"));
        let forest = portals_for("zone_01_starting_forest", forest_document);
        assert_eq!(forest.len(), 2);
        let destinations = forest
            .iter()
            .map(|portal| (portal.target_map().as_str(), portal.target_position()))
            .collect::<Vec<_>>();
        assert_eq!(
            destinations,
            [
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
            include_str!(scenario_file!("media/maps/zone_02_open_plains.tmx")),
        );
    }

    fn open_plains_document() -> &'static str {
        include_str!(scenario_file!("media/maps/zone_02_open_plains.tmx"))
    }

    fn open_plains_portals() -> Vec<RuntimePortal> {
        portals_for("zone_02_open_plains", open_plains_document())
    }

    fn millhaven_document() -> &'static str {
        include_str!(scenario_file!("media/maps/town_02_millhaven.tmx"))
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
                ("zone_03_marshland", Position::new(40, 1)),
            ]
        );
    }

    #[test]
    fn open_plains_cave_chain_links_plains_and_cave_02_and_cave_01_with_no_direct_plains_cave_01_link()
     {
        let cave_01_document =
            include_str!(scenario_file!("media/maps/zone_02_open_plains_cave_01.tmx"));
        let cave_02_document =
            include_str!(scenario_file!("media/maps/zone_02_open_plains_cave_02.tmx"));
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
    fn millhaven_portals_cover_every_interior_with_reversible_return_links() {
        let destinations = millhaven_portals()
            .iter()
            .map(|portal| {
                (
                    portal.target_map().as_str().to_owned(),
                    portal.target_position(),
                )
            })
            .collect::<Vec<_>>();
        let expected = [
            ("town_02_millhaven_mill", Position::new(10, 11)),
            ("town_02_millhaven_inn", Position::new(5, 9)),
            ("zone_02_open_plains", Position::new(2, 1)),
            ("town_02_millhaven_shop_weapon", Position::new(7, 10)),
            ("town_02_millhaven_shop_armor", Position::new(7, 10)),
            ("town_02_millhaven_shop", Position::new(7, 10)),
            ("town_02_millhaven_shop_mc", Position::new(7, 10)),
            ("town_02_millhaven_house_01", Position::new(7, 9)),
            ("town_02_millhaven_house_02", Position::new(4, 7)),
        ]
        .map(|(id, position)| (id.to_owned(), position));
        assert_eq!(destinations, expected);
        let interiors = [
            (
                "town_02_millhaven_mill",
                include_str!(scenario_file!("media/maps/town_02_millhaven_mill.tmx")),
                Position::new(20, 16),
            ),
            (
                "town_02_millhaven_inn",
                include_str!(scenario_file!("media/maps/town_02_millhaven_inn.tmx")),
                Position::new(9, 35),
            ),
            (
                "town_02_millhaven_shop_weapon",
                include_str!(scenario_file!(
                    "media/maps/town_02_millhaven_shop_weapon.tmx"
                )),
                Position::new(9, 6),
            ),
            (
                "town_02_millhaven_shop_armor",
                include_str!(scenario_file!(
                    "media/maps/town_02_millhaven_shop_armor.tmx"
                )),
                Position::new(18, 6),
            ),
            (
                "town_02_millhaven_shop",
                include_str!(scenario_file!("media/maps/town_02_millhaven_shop.tmx")),
                Position::new(27, 6),
            ),
            (
                "town_02_millhaven_shop_mc",
                include_str!(scenario_file!("media/maps/town_02_millhaven_shop_mc.tmx")),
                Position::new(36, 6),
            ),
            (
                "town_02_millhaven_house_01",
                include_str!(scenario_file!("media/maps/town_02_millhaven_house_01.tmx")),
                Position::new(36, 16),
            ),
            (
                "town_02_millhaven_house_02",
                include_str!(scenario_file!("media/maps/town_02_millhaven_house_02.tmx")),
                Position::new(35, 36),
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
        include_str!(scenario_file!("media/maps/zone_03_marshland.tmx"))
    }

    fn marshland_portals() -> Vec<RuntimePortal> {
        portals_for("zone_03_marshland", marshland_document())
    }

    fn harborgate_document() -> &'static str {
        include_str!(scenario_file!("media/maps/town_03_harborgate.tmx"))
    }

    fn harborgate_portals() -> Vec<RuntimePortal> {
        portals_for("town_03_harborgate", harborgate_document())
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
                ("zone_04_ancient_ruins_01_gate", Position::new(70, 29)),
                ("town_03_harborgate", Position::new(4, 1)),
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
        assert_eq!(plains_to_marshland.target_position(), Position::new(40, 1));

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
    fn harborgate_authored_doors_have_reversible_interior_links() {
        let destinations = harborgate_portals()
            .iter()
            .map(|portal| {
                (
                    portal.target_map().as_str().to_owned(),
                    portal.target_position(),
                )
            })
            .collect::<Vec<_>>();
        let expected = [
            ("town_03_harborgate_inn", Position::new(5, 9)),
            ("town_03_harborgate_harbormaster", Position::new(10, 11)),
            ("zone_03_marshland", Position::new(57, 58)),
            ("town_03_harborgate_shop_weapon", Position::new(7, 10)),
            ("town_03_harborgate_shop_armor", Position::new(7, 10)),
            ("town_03_harborgate_shop_item", Position::new(7, 10)),
            ("town_03_harborgate_shop_mc", Position::new(7, 10)),
            ("town_03_harborgate_shop", Position::new(7, 9)),
            ("town_03_harborgate_house_02", Position::new(4, 7)),
            ("town_03_harborgate_storage", Position::new(7, 11)),
        ]
        .map(|(id, position)| (id.to_owned(), position));
        assert_eq!(destinations, expected);
        let interiors = [
            (
                "town_03_harborgate_inn",
                include_str!(scenario_file!("media/maps/town_03_harborgate_inn.tmx")),
                Position::new(9, 25),
            ),
            (
                "town_03_harborgate_harbormaster",
                include_str!(scenario_file!(
                    "media/maps/town_03_harborgate_harbormaster.tmx"
                )),
                Position::new(35, 26),
            ),
            (
                "town_03_harborgate_shop_weapon",
                include_str!(scenario_file!(
                    "media/maps/town_03_harborgate_shop_weapon.tmx"
                )),
                Position::new(6, 37),
            ),
            (
                "town_03_harborgate_shop_armor",
                include_str!(scenario_file!(
                    "media/maps/town_03_harborgate_shop_armor.tmx"
                )),
                Position::new(15, 37),
            ),
            (
                "town_03_harborgate_shop_item",
                include_str!(scenario_file!(
                    "media/maps/town_03_harborgate_shop_item.tmx"
                )),
                Position::new(24, 37),
            ),
            (
                "town_03_harborgate_shop_mc",
                include_str!(scenario_file!("media/maps/town_03_harborgate_shop_mc.tmx")),
                Position::new(33, 37),
            ),
            (
                "town_03_harborgate_shop",
                include_str!(scenario_file!("media/maps/town_03_harborgate_shop.tmx")),
                Position::new(20, 6),
            ),
            (
                "town_03_harborgate_house_02",
                include_str!(scenario_file!("media/maps/town_03_harborgate_house_02.tmx")),
                Position::new(36, 6),
            ),
            (
                "town_03_harborgate_storage",
                include_str!(scenario_file!("media/maps/town_03_harborgate_storage.tmx")),
                Position::new(7, 16),
            ),
        ];
        for (map_id, document, expected_return_position) in interiors {
            assert_reversible_link(
                "town_03_harborgate",
                harborgate_document(),
                map_id,
                document,
            );
            let returns = portals_for(map_id, document);
            assert_eq!(returns.len(), 1);
            assert_eq!(returns[0].target_map().as_str(), "town_03_harborgate");
            assert_eq!(returns[0].target_position(), expected_return_position);
        }
        assert_reversible_link(
            "town_03_harborgate",
            harborgate_document(),
            "zone_03_marshland",
            marshland_document(),
        );
    }

    fn ancient_ruins_gate_document() -> &'static str {
        include_str!(scenario_file!(
            "media/maps/zone_04_ancient_ruins_01_gate.tmx"
        ))
    }

    fn ancient_ruins_courtyard_document() -> &'static str {
        include_str!(scenario_file!(
            "media/maps/zone_04_ancient_ruins_02_courtyard.tmx"
        ))
    }

    fn ancient_ruins_sanctum_document() -> &'static str {
        include_str!(scenario_file!(
            "media/maps/zone_04_ancient_ruins_03_sanctum.tmx"
        ))
    }

    fn ruinwatch_document() -> &'static str {
        include_str!(scenario_file!("media/maps/town_03_ruinwatch.tmx"))
    }

    #[test]
    fn ancient_ruins_portals_close_the_marshland_gate_courtyard_and_sanctum_graph() {
        let gate = portals_for(
            "zone_04_ancient_ruins_01_gate",
            ancient_ruins_gate_document(),
        );
        assert_eq!(
            gate.iter()
                .map(|portal| (portal.target_map().as_str(), portal.target_position()))
                .collect::<Vec<_>>(),
            [
                ("zone_04_ancient_ruins_02_courtyard", Position::new(66, 24),),
                ("zone_03_marshland", Position::new(1, 43)),
                ("town_03_ruinwatch", Position::new(19, 29)),
            ]
        );
        assert_reversible_link(
            "zone_04_ancient_ruins_01_gate",
            ancient_ruins_gate_document(),
            "zone_03_marshland",
            marshland_document(),
        );
        assert_reversible_link(
            "zone_04_ancient_ruins_01_gate",
            ancient_ruins_gate_document(),
            "zone_04_ancient_ruins_02_courtyard",
            ancient_ruins_courtyard_document(),
        );

        let courtyard = portals_for(
            "zone_04_ancient_ruins_02_courtyard",
            ancient_ruins_courtyard_document(),
        );
        assert_eq!(
            courtyard
                .iter()
                .map(|portal| (portal.target_map().as_str(), portal.target_position()))
                .collect::<Vec<_>>(),
            [
                ("zone_04_ancient_ruins_01_gate", Position::new(1, 29),),
                ("zone_04_ancient_ruins_03_sanctum", Position::new(58, 34),),
            ]
        );
        assert_reversible_link(
            "zone_04_ancient_ruins_02_courtyard",
            ancient_ruins_courtyard_document(),
            "zone_04_ancient_ruins_03_sanctum",
            ancient_ruins_sanctum_document(),
        );

        let sanctum = portals_for(
            "zone_04_ancient_ruins_03_sanctum",
            ancient_ruins_sanctum_document(),
        );
        assert_eq!(
            sanctum
                .iter()
                .map(|portal| (portal.target_map().as_str(), portal.target_position()))
                .collect::<Vec<_>>(),
            [
                ("zone_04_ancient_ruins_02_courtyard", Position::new(1, 24),),
                ("zone_05_mountain_foothills_01", Position::new(12, 56),),
            ]
        );
    }

    #[test]
    fn ruinwatch_portals_cover_every_interior_with_reversible_return_links() {
        let portals = portals_for("town_03_ruinwatch", ruinwatch_document());
        assert_eq!(
            portals
                .iter()
                .map(|portal| (portal.target_map().as_str(), portal.target_position()))
                .collect::<Vec<_>>(),
            [
                ("town_03_ruinwatch_shop_weapon", Position::new(7, 9)),
                ("town_03_ruinwatch_shop_armor", Position::new(7, 9)),
                ("town_03_ruinwatch_inn", Position::new(7, 5)),
                ("town_03_ruinwatch_shop_mc", Position::new(7, 9)),
                ("zone_04_ancient_ruins_01_gate", Position::new(53, 1),),
                ("town_03_ruinwatch_monastery_vaults", Position::new(11, 20)),
                ("town_03_ruinwatch_storage", Position::new(4, 8)),
                ("town_03_ruinwatch_shop", Position::new(7, 9)),
                ("town_03_ruinwatch_shop_apothecary", Position::new(7, 9)),
            ]
        );

        let interiors = [
            (
                "town_03_ruinwatch_inn",
                include_str!(scenario_file!("media/maps/town_03_ruinwatch_inn.tmx")),
                Position::new(9, 25),
            ),
            (
                "town_03_ruinwatch_shop",
                include_str!(scenario_file!("media/maps/town_03_ruinwatch_shop.tmx")),
                Position::new(54, 7),
            ),
            (
                "town_03_ruinwatch_monastery_vaults",
                include_str!(scenario_file!(
                    "media/maps/town_03_ruinwatch_monastery_vaults.tmx"
                )),
                Position::new(45, 16),
            ),
            (
                "town_03_ruinwatch_shop_weapon",
                include_str!(scenario_file!(
                    "media/maps/town_03_ruinwatch_shop_weapon.tmx"
                )),
                Position::new(20, 6),
            ),
            (
                "town_03_ruinwatch_shop_armor",
                include_str!(scenario_file!(
                    "media/maps/town_03_ruinwatch_shop_armor.tmx"
                )),
                Position::new(36, 6),
            ),
            (
                "town_03_ruinwatch_shop_mc",
                include_str!(scenario_file!("media/maps/town_03_ruinwatch_shop_mc.tmx")),
                Position::new(35, 26),
            ),
            (
                "town_03_ruinwatch_storage",
                include_str!(scenario_file!("media/maps/town_03_ruinwatch_storage.tmx")),
                Position::new(55, 25),
            ),
            (
                "town_03_ruinwatch_shop_apothecary",
                include_str!(scenario_file!(
                    "media/maps/town_03_ruinwatch_shop_apothecary.tmx"
                )),
                Position::new(45, 6),
            ),
        ];
        for (map_id, document, expected_return_position) in interiors {
            assert_reversible_link("town_03_ruinwatch", ruinwatch_document(), map_id, document);
            let returns = portals_for(map_id, document);
            assert_eq!(returns.len(), 1);
            assert_eq!(returns[0].target_map().as_str(), "town_03_ruinwatch");
            assert_eq!(returns[0].target_position(), expected_return_position);
        }

        assert_reversible_link(
            "town_03_ruinwatch",
            ruinwatch_document(),
            "zone_04_ancient_ruins_01_gate",
            ancient_ruins_gate_document(),
        );
    }

    #[test]
    fn town_building_doorways_allow_sideways_passing_and_trigger_on_walking_in() {
        let towns = [
            (
                "town_01_ardel",
                include_str!(scenario_file!("media/maps/town_01_ardel.tmx")),
                8,
                9,
            ),
            (
                "town_02_millhaven",
                include_str!(scenario_file!("media/maps/town_02_millhaven.tmx")),
                8,
                8,
            ),
            (
                "town_03_harborgate",
                include_str!(scenario_file!("media/maps/town_03_harborgate.tmx")),
                9,
                9,
            ),
            (
                "town_03_ruinwatch",
                include_str!(scenario_file!("media/maps/town_03_ruinwatch.tmx")),
                8,
                8,
            ),
            (
                "town_04_frostholm",
                include_str!(scenario_file!("media/maps/town_04_frostholm.tmx")),
                8,
                8,
            ),
            (
                "town_05_ashenveil",
                include_str!(scenario_file!("media/maps/town_05_ashenveil.tmx")),
                11,
                12,
            ),
        ];
        for (map_id, xml, expected_entrances, expected_door_tiles) in towns {
            assert_town_doorway_passage(map_id, xml, expected_entrances, expected_door_tiles);
        }
    }

    fn assert_town_doorway_passage(
        map_id: &str,
        xml: &str,
        expected_entrances: usize,
        expected_door_tiles: usize,
    ) {
        use crate::scenario_spatial::collision_occupancy::CollisionOccupancy;

        let owner = ScenarioRelativePath::try_from(format!("media/maps/{map_id}.tmx")).unwrap();
        let document = parse_tmx_map_document(xml, &owner).unwrap();
        let occupancy = CollisionOccupancy::from_tmx_document(&document).unwrap();
        let portals = runtime_portals(&document).unwrap();
        let entrances = portals
            .iter()
            .filter(|portal| {
                portal
                    .target_map()
                    .as_str()
                    .strip_prefix(map_id)
                    .is_some_and(|suffix| suffix.starts_with('_'))
            })
            .collect::<Vec<_>>();
        assert_eq!(entrances.len(), expected_entrances, "{map_id}");
        let door_first_gid = document
            .external_tilesets()
            .iter()
            .find(|tileset| tileset.source().as_str().ends_with("/door.tsx"))
            .unwrap()
            .first_gid();
        let mut door_count = 0;
        for layer in document.tile_layers() {
            if matches!(layer.name(), "collision" | "spawn_tile") {
                continue;
            }
            for (index, gid) in layer.gids().iter().enumerate() {
                let Some(local_id) = gid.global_id().checked_sub(door_first_gid) else {
                    continue;
                };
                if !matches!(local_id, 8..=15 | 24) {
                    continue;
                }
                door_count += 1;
                let column = index as u32 % layer.width();
                let row = index as u32 / layer.width();
                let left = (column * occupancy.tile_width()) as f32;
                let bottom = ((row + 1) * occupancy.tile_height()) as f32;
                assert_eq!(occupancy.is_open(column as i32, row as i32), Some(true));

                // Pass in either direction along the pavement, including exact wall contact.
                for y_offset in [0.0, 0.5, 1.0, 7.0, 16.0, 32.0] {
                    for x_offset in -32..=64 {
                        let feet = collision_rect(left + x_offset as f32, bottom + y_offset);
                        assert!(
                            entrances
                                .iter()
                                .all(|portal| !portal.bounds().is_triggered_by(feet)),
                            "{map_id}: sideways passage triggered a portal at door {column},{row}"
                        );
                    }
                }

                // Walk north from the pavement using the live player's foot rectangle.
                let mut motion =
                    WorldPlayerMotion::from_tile(Position::new(column as i32, row as i32 + 1));
                let mut entered = false;
                for _ in 0..8 {
                    let feet = motion.collision_rect();
                    assert!(!occupancy.is_rect_blocked(feet.x, feet.y, feet.width, feet.height));
                    if entrances
                        .iter()
                        .any(|portal| portal.bounds().is_triggered_by(feet))
                    {
                        assert!(
                            (bottom - 8.0..bottom).contains(&feet.y),
                            "{map_id}: entrance must trigger just inside the bottom of the door"
                        );
                        entered = true;
                        break;
                    }
                    motion.set_top_left(motion.top_left() - Vec2::new(0.0, 5.0));
                }
                assert!(
                    entered,
                    "{map_id}: door {column},{row} cannot be entered from the pavement"
                );
            }
        }
        assert_eq!(
            door_count, expected_door_tiles,
            "{map_id}: include every door leaf"
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
        // This rect overlaps only the relocated house doorway among Ardel's ten portals.
        let on_house = collision_rect(100.0, 150.0);
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
            .find(|portal| portal.target_map().as_str() == "town_01_ardel_house_01")
            .unwrap();
        let mut transition = WorldTransition {
            phase: TransitionPhase::Publishing,
            pending: Some(PendingTransition {
                target_map: RuntimeMapId::try_new("town_01_ardel").unwrap(),
                target_position: Position::new(3, 5),
                facing: CardinalDirection::Down,
            }),
            ..default()
        };

        transition.destination_published();
        transition.phase = TransitionPhase::Idle;
        let on_house = collision_rect(100.0, 150.0);
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
        transition.destination_published();
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
            .find(|portal| portal.target_map().as_str() == "town_01_ardel_house_01")
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
