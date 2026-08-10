//! Portal extraction and the transactional World-map transition lifecycle.

use std::{collections::BTreeSet, fmt};

use bevy::prelude::*;

use crate::{
    app_state::AppState,
    runtime_map::RuntimeMapId,
    scenario_spatial::{CardinalDirection, Position},
    tmx_header::{TmxMapDocument, TmxPropertyValue},
};

const PLAYER_TILE_SIZE: f64 = 32.0;
const FADE_ALPHA_PER_SECOND: f32 = 300.0 / 255.0;

/// Installs the transition state now consumed by movement and later driven by loaded map assets.
pub(crate) struct WorldTransitionPlugin;

impl Plugin for WorldTransitionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldTransition>()
            .add_systems(OnEnter(AppState::World), reset_transition)
            .add_systems(OnExit(AppState::World), reset_transition);
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

    pub(crate) fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

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
    fn overlaps_tile(self, tile: Position) -> bool {
        let tile_x = f64::from(tile.x) * PLAYER_TILE_SIZE;
        let tile_y = f64::from(tile.y) * PLAYER_TILE_SIZE;
        tile_x < self.x + self.width
            && tile_x + PLAYER_TILE_SIZE > self.x
            && tile_y < self.y + self.height
            && tile_y + PLAYER_TILE_SIZE > self.y
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
        player_tile: Position,
    ) -> Option<&'a RuntimePortal> {
        let now = portals
            .iter()
            .filter(|portal| portal.bounds.overlaps_tile(player_tile))
            .map(RuntimePortal::object_id)
            .collect::<BTreeSet<_>>();
        let entered = portals.iter().find(|portal| {
            now.contains(&portal.object_id) && !self.overlapping.contains(&portal.object_id)
        });
        self.overlapping = now;
        entered
    }

    pub(crate) fn clear(&mut self) {
        self.overlapping.clear();
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
    Failed,
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
}

impl Default for WorldTransition {
    fn default() -> Self {
        Self {
            phase: TransitionPhase::FadingIn,
            alpha: 1.0,
            pending: None,
            detector: PortalEntryDetector::default(),
        }
    }
}

impl WorldTransition {
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

    pub(crate) fn request(&mut self, portal: &RuntimePortal, facing: CardinalDirection) -> bool {
        if self.phase != TransitionPhase::Idle {
            return false;
        }
        self.pending = Some(PendingTransition {
            target_map: portal.target_map.clone(),
            target_position: portal.target_position,
            facing,
        });
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
        debug_assert_eq!(self.phase, TransitionPhase::Loading);
        self.pending = None;
        self.detector.clear();
        self.phase = TransitionPhase::FadingIn;
        self.alpha = 1.0;
    }

    pub(crate) fn destination_failed(&mut self) {
        debug_assert_eq!(self.phase, TransitionPhase::Loading);
        self.pending = None;
        self.phase = TransitionPhase::Failed;
        self.alpha = 1.0;
    }
}

fn reset_transition(mut transition: ResMut<WorldTransition>) {
    *transition = WorldTransition::default();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{scenario_path::ScenarioRelativePath, tmx_header::parse_tmx_map_document};

    fn ardel_portals() -> Vec<RuntimePortal> {
        let owner = ScenarioRelativePath::try_from("assets/maps/town_01_ardel.tmx").unwrap();
        let document = parse_tmx_map_document(
            include_str!("../assets/scenarios/rusted_kingdoms/assets/maps/town_01_ardel.tmx"),
            &owner,
        )
        .unwrap();
        runtime_portals(&document).unwrap()
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
    fn entry_detector_emits_once_until_player_leaves_and_reenters() {
        let portals = ardel_portals();
        let mut detector = PortalEntryDetector::default();
        assert_eq!(
            detector
                .entered(&portals, Position::new(2, 3))
                .map(RuntimePortal::object_id),
            Some(7)
        );
        assert!(detector.entered(&portals, Position::new(3, 3)).is_none());
        assert!(detector.entered(&portals, Position::new(4, 4)).is_none());
        assert_eq!(
            detector
                .entered(&portals, Position::new(2, 3))
                .map(RuntimePortal::object_id),
            Some(7)
        );
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
        transition.destination_published();
        assert_eq!(transition.phase(), TransitionPhase::FadingIn);
        transition.advance_fade(1.0);
        assert_eq!(transition.phase(), TransitionPhase::Idle);
        assert!(!transition.input_locked());
    }
}
