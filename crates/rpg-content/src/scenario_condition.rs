//! Shared source-authored flag conditions.
//!
//! The pinned `0897035` corpus contains these fields in dialogue-entry conditions and NPC
//! `present` mappings. Both consumers use the same rule: every `requires` flag must be set and
//! no `excludes` flag may be set. Missing fields are empty lists. The source uses string lists
//! only; scalar shorthand and null are not accepted compatibility forms.
//!
//! Enemy move mappings also use the key `condition`, but their HP/turn rules are a distinct
//! schema and deliberately do not use this type.

use serde::Deserialize;

/// A conjunction of required flags and excluded flags.
///
/// Vectors preserve source ordering and duplicate entries exactly. The pinned corpus has no
/// duplicates, and duplicates do not change membership-based evaluation semantics.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FlagConditions {
    /// Flag identifiers that must all be set.
    #[serde(
        default,
        deserialize_with = "crate::scenario_yaml::deserialize_strings"
    )]
    pub requires: Vec<String>,
    /// Flag identifiers that must all be absent.
    #[serde(
        default,
        deserialize_with = "crate::scenario_yaml::deserialize_strings"
    )]
    pub excludes: Vec<String>,
}

impl FlagConditions {
    /// Returns whether this condition matches the supplied flag-membership lookup.
    ///
    /// Required flags use AND semantics. A single set excluded flag rejects the condition.
    /// Empty lists therefore make no restriction, matching Python's `all`/`not any` rules.
    pub fn is_satisfied_by(&self, mut has_flag: impl FnMut(&str) -> bool) -> bool {
        self.requires.iter().all(|flag| has_flag(flag))
            && self.excludes.iter().all(|flag| !has_flag(flag))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use serde::Deserialize;

    use super::FlagConditions;
    use crate::scenario_yaml;

    #[derive(Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct ConditionFixture {
        cases: Vec<ConditionCase>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct ConditionCase {
        id: String,
        #[serde(default)]
        condition: FlagConditions,
    }

    #[test]
    fn loads_every_pinned_flag_condition_shape_from_a_fixture() {
        let fixture: ConditionFixture = scenario_yaml::from_str(include_str!(
            "../tests/fixtures/shared-flag-conditions.yaml"
        ))
        .expect("shared condition fixture should deserialize");

        let cases = fixture
            .cases
            .into_iter()
            .map(|case| (case.id, case.condition))
            .collect::<std::collections::HashMap<_, _>>();

        assert_eq!(cases["missing"], FlagConditions::default());
        assert_eq!(cases["requires_only"].requires, ["story_quest_started"]);
        assert!(cases["requires_only"].excludes.is_empty());
        assert!(cases["excludes_only"].requires.is_empty());
        assert_eq!(cases["excludes_only"].excludes, ["npc_elise_joined"]);
        assert_eq!(
            cases["two_requires_empty_excludes"].requires,
            ["story_act3_started", "boss_zone03_defeated"]
        );
        assert!(cases["two_requires_empty_excludes"].excludes.is_empty());
        assert_eq!(cases["both"].requires, ["sq_stream_started"]);
        assert_eq!(cases["both"].excludes, ["sq_stream_relayed"]);
    }

    #[test]
    fn preserves_authored_order_and_duplicates() {
        let conditions: FlagConditions = scenario_yaml::from_str(
            "requires: [second, first, second]\nexcludes: [blocked, blocked]\n",
        )
        .expect("string lists should deserialize without normalization");

        assert_eq!(conditions.requires, ["second", "first", "second"]);
        assert_eq!(conditions.excludes, ["blocked", "blocked"]);
    }

    #[test]
    fn requires_every_required_flag_and_rejects_any_excluded_flag() {
        let conditions = FlagConditions {
            requires: vec!["quest_started".to_owned(), "key_found".to_owned()],
            excludes: vec!["quest_done".to_owned(), "gate_destroyed".to_owned()],
        };

        for (active, expected) in [
            (&[][..], false),
            (&["quest_started"][..], false),
            (&["quest_started", "key_found"][..], true),
            (&["quest_started", "key_found", "quest_done"][..], false),
            (&["quest_started", "key_found", "gate_destroyed"][..], false),
        ] {
            let active = active.iter().copied().collect::<HashSet<_>>();
            assert_eq!(
                conditions.is_satisfied_by(|flag| active.contains(flag)),
                expected
            );
        }
    }

    #[test]
    fn empty_conditions_match_every_flag_set() {
        assert!(FlagConditions::default().is_satisfied_by(|_| false));
        assert!(FlagConditions::default().is_satisfied_by(|_| true));
    }

    #[test]
    fn rejects_unobserved_scalar_null_and_unknown_forms() {
        for document in [
            "requires: story_quest_started\n",
            "requires: null\n",
            "requires: [42]\n",
            "excludes: story_act2_started\n",
            "excludes: null\n",
            "excludes: [true]\n",
            "requires_any: [story_quest_started]\n",
        ] {
            assert!(scenario_yaml::from_str::<FlagConditions>(document).is_err());
        }
    }
}
