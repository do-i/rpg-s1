//! Mutable runtime story-flag state.
//!
//! Scenario catalogs own authored flag identifiers and [`FlagConditions`]; this resource owns
//! only the active membership set for one game session. A `BTreeSet` makes observable ordering
//! deterministic for later save and replay work without assigning any quest or dialogue rules
//! here. The source bootstrap list can seed this state, while new-game construction itself stays
//! with M3.09.

use bevy::prelude::Resource;
use std::collections::BTreeSet;

use crate::scenario_condition::FlagConditions;

/// The active mutable flags for one runtime session.
#[derive(Clone, Debug, Default, Eq, PartialEq, Resource)]
pub struct RuntimeFlags {
    active: BTreeSet<String>,
}

impl RuntimeFlags {
    /// Builds session state from source-authored bootstrap flag identifiers.
    ///
    /// Repeated bootstrap entries have set semantics and therefore collapse to one active flag.
    pub fn from_bootstrap<I, S>(flags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            active: flags.into_iter().map(Into::into).collect(),
        }
    }

    /// Activates a flag, returning whether the state changed.
    pub fn set(&mut self, flag: impl Into<String>) -> bool {
        self.active.insert(flag.into())
    }

    /// Deactivates a flag, returning whether the state changed.
    pub fn unset(&mut self, flag: &str) -> bool {
        self.active.remove(flag)
    }

    /// Returns whether a flag is active.
    pub fn is_set(&self, flag: &str) -> bool {
        self.active.contains(flag)
    }

    /// Evaluates shared source-authored `requires`/`excludes` conditions against this session.
    pub fn satisfies(&self, conditions: &FlagConditions) -> bool {
        conditions.is_satisfied_by(|flag| self.is_set(flag))
    }

    /// Iterates over active flags in stable lexical order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &str> {
        self.active.iter().map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::RuntimeFlags;
    use crate::scenario_condition::FlagConditions;

    #[test]
    fn default_state_is_empty_and_matches_empty_conditions() {
        let flags = RuntimeFlags::default();

        assert!(!flags.is_set("story_started"));
        assert_eq!(flags.iter().count(), 0);
        assert!(flags.satisfies(&FlagConditions::default()));
    }

    #[test]
    fn setting_is_idempotent() {
        let mut flags = RuntimeFlags::default();

        assert!(flags.set("story_started"));
        assert!(!flags.set("story_started"));
        assert!(flags.is_set("story_started"));
        assert_eq!(flags.iter().collect::<Vec<_>>(), ["story_started"]);
    }

    #[test]
    fn unsetting_removes_only_an_active_flag() {
        let mut flags = RuntimeFlags::from_bootstrap(["story_started", "gate_open"]);

        assert!(flags.unset("story_started"));
        assert!(!flags.is_set("story_started"));
        assert!(flags.is_set("gate_open"));
        assert!(!flags.unset("story_started"));
    }

    #[test]
    fn requires_all_and_excludes_any_use_the_shared_condition_type() {
        let conditions = FlagConditions {
            requires: vec!["story_started".to_owned(), "key_found".to_owned()],
            excludes: vec!["quest_finished".to_owned(), "world_destroyed".to_owned()],
        };
        let mut flags = RuntimeFlags::from_bootstrap(["story_started"]);

        assert!(!flags.satisfies(&conditions));
        assert!(flags.set("key_found"));
        assert!(flags.satisfies(&conditions));
        assert!(flags.set("quest_finished"));
        assert!(!flags.satisfies(&conditions));
        assert!(flags.unset("quest_finished"));
        assert!(flags.satisfies(&conditions));
    }

    #[test]
    fn bootstrap_initialization_deduplicates_and_sorts_deterministically() {
        let flags = RuntimeFlags::from_bootstrap(["zeta", "alpha", "zeta", "middle"]);

        assert_eq!(
            flags.iter().collect::<Vec<_>>(),
            ["alpha", "middle", "zeta"]
        );
    }
}
