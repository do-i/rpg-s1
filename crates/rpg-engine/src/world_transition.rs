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
    /// (`world_debug_overlay`), which draws the same rectangle.
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
    /// The player's
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
    // Prefer the player's live smoothly-moved pixel collision rect, checked every frame against
    // the player's actual position. The tile-derived fallback (same formula as
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
mod tests;
