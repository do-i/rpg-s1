//! Mutable runtime map-session state.
//!
//! This resource holds only the current map location and a deterministic visited-map history.
//! It neither proves that an id names a loaded scenario map nor performs a transition: catalog
//! validation, portal success, movement, rendering, new-game assembly, and save envelopes each
//! have later owners. A map id is nevertheless nonempty at this boundary, while `None` expresses
//! the legitimate pre-new-game state without using Python's ambiguous empty-string sentinel.

use bevy::prelude::Resource;
use std::{collections::BTreeSet, fmt};

use crate::scenario_spatial::{CardinalDirection, Position};

/// A nonempty logical map identifier selected by runtime state.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RuntimeMapId(String);

impl RuntimeMapId {
    /// Creates an identifier without normalizing or catalog-validating its authored spelling.
    pub fn try_new(value: impl Into<String>) -> Result<Self, RuntimeMapIdError> {
        let value = value.into();
        if value.is_empty() {
            return Err(RuntimeMapIdError);
        }
        Ok(Self(value))
    }

    /// Returns the exact logical identifier spelling.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Rejects the empty map-id sentinel from runtime construction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeMapIdError;

impl fmt::Display for RuntimeMapIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("map id must not be empty")
    }
}

impl std::error::Error for RuntimeMapIdError {}

/// Current world location and previously departed maps for one runtime session.
#[derive(Clone, Debug, Eq, PartialEq, Resource)]
pub struct RuntimeMapState {
    current: Option<RuntimeMapId>,
    position: Position,
    facing: CardinalDirection,
    visited: BTreeSet<RuntimeMapId>,
}

impl Default for RuntimeMapState {
    fn default() -> Self {
        Self {
            current: None,
            position: Position::new(0, 0),
            facing: CardinalDirection::Down,
            visited: BTreeSet::new(),
        }
    }
}

impl RuntimeMapState {
    /// Creates initialized state for a selected map without adding that arrival to visited history.
    pub fn new(map_id: RuntimeMapId, position: Position, facing: CardinalDirection) -> Self {
        Self {
            current: Some(map_id),
            position,
            facing,
            visited: BTreeSet::new(),
        }
    }

    /// Restores a complete native save location without inventing transition history.
    pub(crate) fn try_from_saved(
        map_id: RuntimeMapId,
        position: Position,
        facing: CardinalDirection,
        visited: impl IntoIterator<Item = RuntimeMapId>,
    ) -> Result<Self, RuntimeMapStateError> {
        let mut restored = BTreeSet::new();
        for id in visited {
            if !restored.insert(id.clone()) {
                return Err(RuntimeMapStateError::DuplicateVisitedMap(id));
            }
        }
        Ok(Self {
            current: Some(map_id),
            position,
            facing,
            visited: restored,
        })
    }

    /// The currently selected map, or `None` before new-game state has been assembled.
    pub fn current(&self) -> Option<&RuntimeMapId> {
        self.current.as_ref()
    }

    /// The current player tile position.
    pub const fn position(&self) -> Position {
        self.position
    }

    /// The player-facing direction for the current position or a future arrival.
    pub const fn facing(&self) -> CardinalDirection {
        self.facing
    }

    /// Commits a completed map transition.
    ///
    /// The previous current map becomes visited, including a same-id transition, matching the
    /// pinned Python `MapState.move_to` behavior. The arrival itself is not visited until it is
    /// later departed. Repeated history insertions are idempotent.
    pub fn move_to(&mut self, map_id: RuntimeMapId, position: Position, facing: CardinalDirection) {
        if let Some(previous) = self.current.replace(map_id) {
            self.visited.insert(previous);
        }
        self.position = position;
        self.facing = facing;
    }

    /// Updates only the in-map tile position; collision and movement rules remain external.
    pub fn set_position(&mut self, position: Position) {
        self.position = position;
    }

    /// Updates only the current facing direction.
    pub fn set_facing(&mut self, facing: CardinalDirection) {
        self.facing = facing;
    }

    /// Returns whether this map has been departed previously.
    pub fn has_visited(&self, map_id: &RuntimeMapId) -> bool {
        self.visited.contains(map_id)
    }

    /// Iterates visited map ids in stable lexical order.
    pub fn visited(&self) -> impl ExactSizeIterator<Item = &RuntimeMapId> {
        self.visited.iter()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RuntimeMapStateError {
    DuplicateVisitedMap(RuntimeMapId),
}

impl fmt::Display for RuntimeMapStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateVisitedMap(id) => {
                write!(
                    formatter,
                    "visited map `{}` appears more than once",
                    id.as_str()
                )
            }
        }
    }
}

impl std::error::Error for RuntimeMapStateError {}

#[cfg(test)]
mod tests {
    use super::{RuntimeMapId, RuntimeMapState};
    use crate::scenario_spatial::{CardinalDirection, Position};

    fn map(id: &str) -> RuntimeMapId {
        RuntimeMapId::try_new(id).expect("test map id should be nonempty")
    }

    #[test]
    fn default_state_has_no_current_map_and_source_compatible_spatial_defaults() {
        let state = RuntimeMapState::default();

        assert!(state.current().is_none());
        assert_eq!(state.position(), Position::new(0, 0));
        assert_eq!(state.facing(), CardinalDirection::Down);
        assert_eq!(state.visited().count(), 0);
    }

    #[test]
    fn move_updates_location_and_records_only_the_departed_map() {
        let zone = map("zone_01");
        let town = map("town_01");
        let mut state =
            RuntimeMapState::new(zone.clone(), Position::new(1, 2), CardinalDirection::Up);

        state.move_to(town.clone(), Position::new(5, 8), CardinalDirection::Right);

        assert_eq!(state.current(), Some(&town));
        assert_eq!(state.position(), Position::new(5, 8));
        assert_eq!(state.facing(), CardinalDirection::Right);
        assert!(state.has_visited(&zone));
        assert!(!state.has_visited(&town));
    }

    #[test]
    fn position_and_facing_updates_do_not_change_the_current_map_or_history() {
        let town = map("town_01");
        let mut state =
            RuntimeMapState::new(town.clone(), Position::new(5, 8), CardinalDirection::Down);

        state.set_position(Position::new(-3, 10));
        state.set_facing(CardinalDirection::Left);

        assert_eq!(state.current(), Some(&town));
        assert_eq!(state.position(), Position::new(-3, 10));
        assert_eq!(state.facing(), CardinalDirection::Left);
        assert_eq!(state.visited().count(), 0);
    }

    #[test]
    fn visited_history_is_idempotent_and_stably_ordered() {
        let zone = map("zone_01");
        let town = map("town_01");
        let keep = map("keep_01");
        let mut state =
            RuntimeMapState::new(zone.clone(), Position::new(0, 0), CardinalDirection::Down);

        state.move_to(town.clone(), Position::new(0, 0), CardinalDirection::Down);
        state.move_to(zone.clone(), Position::new(0, 0), CardinalDirection::Down);
        state.move_to(town.clone(), Position::new(0, 0), CardinalDirection::Down);
        state.move_to(keep, Position::new(0, 0), CardinalDirection::Down);

        assert_eq!(
            state
                .visited()
                .map(RuntimeMapId::as_str)
                .collect::<Vec<_>>(),
            ["town_01", "zone_01"]
        );
    }

    #[test]
    fn state_owns_map_identity_and_position_values_independently_of_their_sources() {
        let mut source_id = String::from("town_01");
        let source_position = Position::new(7, 9);
        let state = RuntimeMapState::new(
            RuntimeMapId::try_new(source_id.clone()).unwrap(),
            source_position,
            CardinalDirection::Down,
        );
        source_id.clear();

        assert_eq!(state.current().unwrap().as_str(), "town_01");
        assert_eq!(state.position(), source_position);
        assert!(RuntimeMapId::try_new("").is_err());
    }
}
