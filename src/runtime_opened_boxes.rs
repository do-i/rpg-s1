//! Mutable opened-item-box membership state.
//!
//! Source box ids are unique only inside their map, so each opened identity is a typed
//! `(map_id, box_id)` pair rather than a bare id. This resource records membership only:
//! map loading, presence conditions, interaction, loot granting, presentation, catalog checks,
//! and serialization are deliberately owned by later milestones.

use bevy::prelude::Resource;
use std::{collections::BTreeSet, fmt};

use crate::runtime_map::RuntimeMapId;

/// A map-scoped item-box identity.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OpenedBoxKey {
    map_id: RuntimeMapId,
    box_id: String,
}

impl OpenedBoxKey {
    /// Creates an identity from an already validated nonempty map id and a nonempty box id.
    pub fn try_new(
        map_id: RuntimeMapId,
        box_id: impl Into<String>,
    ) -> Result<Self, OpenedBoxKeyError> {
        let box_id = box_id.into();
        if box_id.is_empty() {
            return Err(OpenedBoxKeyError);
        }
        Ok(Self { map_id, box_id })
    }

    /// Returns the map scope that owns this box id.
    pub fn map_id(&self) -> &RuntimeMapId {
        &self.map_id
    }

    /// Returns the exact authored box-id spelling.
    pub fn box_id(&self) -> &str {
        &self.box_id
    }
}

/// Rejects an empty item-box id from runtime construction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OpenedBoxKeyError;

impl fmt::Display for OpenedBoxKeyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("item-box id must not be empty")
    }
}

impl std::error::Error for OpenedBoxKeyError {}

/// Opened item boxes for one runtime session.
#[derive(Clone, Debug, Default, Eq, PartialEq, Resource)]
pub struct RuntimeOpenedBoxes {
    opened: BTreeSet<OpenedBoxKey>,
}

impl RuntimeOpenedBoxes {
    /// Records a completed opening, returning whether this was the first record for that key.
    pub fn record(&mut self, key: OpenedBoxKey) -> bool {
        self.opened.insert(key)
    }

    /// Returns whether this exact map-scoped box has been recorded as opened.
    pub fn contains(&self, key: &OpenedBoxKey) -> bool {
        self.opened.contains(key)
    }

    /// Iterates map-scoped box keys in stable lexical `(map_id, box_id)` order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &OpenedBoxKey> {
        self.opened.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::{OpenedBoxKey, RuntimeOpenedBoxes};
    use crate::runtime_map::RuntimeMapId;

    fn map(id: &str) -> RuntimeMapId {
        RuntimeMapId::try_new(id).expect("test map id should be nonempty")
    }

    fn key(map_id: &str, box_id: &str) -> OpenedBoxKey {
        OpenedBoxKey::try_new(map(map_id), box_id).expect("test box key should be valid")
    }

    #[test]
    fn default_state_is_empty() {
        let opened = RuntimeOpenedBoxes::default();

        assert!(!opened.contains(&key("map_a", "box_1")));
        assert_eq!(opened.iter().count(), 0);
    }

    #[test]
    fn first_record_changes_state_and_a_duplicate_is_idempotent() {
        let key = key("map_a", "box_1");
        let mut opened = RuntimeOpenedBoxes::default();

        assert!(opened.record(key.clone()));
        assert!(!opened.record(key.clone()));
        assert!(opened.contains(&key));
        assert_eq!(opened.iter().count(), 1);
    }

    #[test]
    fn membership_is_scoped_by_map_as_in_the_source_contract() {
        let mut opened = RuntimeOpenedBoxes::default();
        let map_a = key("map_a", "chest");
        let map_b = key("map_b", "chest");

        assert!(opened.record(map_a.clone()));
        assert!(opened.contains(&map_a));
        assert!(!opened.contains(&map_b));
    }

    #[test]
    fn iteration_order_is_stable_by_map_then_box_id() {
        let mut opened = RuntimeOpenedBoxes::default();
        for key in [
            key("zone_02", "b"),
            key("zone_01", "z"),
            key("zone_01", "a"),
        ] {
            assert!(opened.record(key));
        }

        assert_eq!(
            opened
                .iter()
                .map(|key| (key.map_id().as_str(), key.box_id()))
                .collect::<Vec<_>>(),
            [("zone_01", "a"), ("zone_01", "z"), ("zone_02", "b")]
        );
    }

    #[test]
    fn construction_rejects_empty_box_ids_and_owns_source_values() {
        let mut source_box_id = String::from("treasure_01");
        let opened_key = OpenedBoxKey::try_new(map("zone_01"), source_box_id.clone()).unwrap();
        source_box_id.clear();

        assert_eq!(opened_key.map_id().as_str(), "zone_01");
        assert_eq!(opened_key.box_id(), "treasure_01");
        assert!(OpenedBoxKey::try_new(map("zone_01"), "").is_err());
    }
}
