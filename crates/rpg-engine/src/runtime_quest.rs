//! Flag-derived runtime quest status.
//!
//! The pinned Python game has no quest-progress store or objective counters. Its read-only quest
//! board derives status directly from each immutable [`QuestDefinition`]'s two flag ids. Keeping
//! that query pure leaves [`RuntimeFlags`] as the only mutable source of truth.

use crate::{runtime_flags::RuntimeFlags, scenario_quest::QuestDefinition};

/// The three quest-board lifecycle states exposed by the pinned game.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuestStatus {
    Inactive,
    Active,
    Completed,
}

/// Derives one quest's current status without changing flags or catalog data.
///
/// Completion takes precedence, matching Python when only the completion flag or both flags are
/// active.
pub fn quest_status(quest: &QuestDefinition, flags: &RuntimeFlags) -> QuestStatus {
    if flags.is_set(&quest.completed_flag) {
        QuestStatus::Completed
    } else if flags.is_set(&quest.started_flag) {
        QuestStatus::Active
    } else {
        QuestStatus::Inactive
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario_quest::QuestKind;

    fn quest(id: &str, started_flag: &str, completed_flag: &str) -> QuestDefinition {
        QuestDefinition {
            id: id.to_owned(),
            name: format!("Invented {id}"),
            kind: QuestKind::Sub,
            location: "Invented Place".to_owned(),
            description: "An invented quest used only by this test.".to_owned(),
            started_flag: started_flag.to_owned(),
            completed_flag: completed_flag.to_owned(),
        }
    }

    #[test]
    fn neither_flag_is_inactive_and_started_flag_is_active() {
        let quest = quest("errand", "errand_started", "errand_done");
        let mut flags = RuntimeFlags::default();

        assert_eq!(quest_status(&quest, &flags), QuestStatus::Inactive);
        assert!(flags.set("errand_started"));
        assert_eq!(quest_status(&quest, &flags), QuestStatus::Active);
    }

    #[test]
    fn completed_flag_alone_or_with_started_flag_takes_precedence() {
        let quest = quest("errand", "errand_started", "errand_done");
        let mut flags = RuntimeFlags::from_bootstrap(["errand_done"]);

        assert_eq!(quest_status(&quest, &flags), QuestStatus::Completed);
        assert!(flags.set("errand_started"));
        assert_eq!(quest_status(&quest, &flags), QuestStatus::Completed);
    }

    #[test]
    fn independent_queries_do_not_mutate_flags_or_catalog_definitions() {
        let first = quest("first", "first_started", "first_done");
        let second = quest("second", "second_started", "second_done");
        let first_before = first.clone();
        let second_before = second.clone();
        let flags = RuntimeFlags::from_bootstrap(["first_started", "second_done"]);
        let flags_before = flags.clone();

        assert_eq!(quest_status(&first, &flags), QuestStatus::Active);
        assert_eq!(quest_status(&second, &flags), QuestStatus::Completed);
        assert_eq!(first, first_before);
        assert_eq!(second, second_before);
        assert_eq!(flags, flags_before);
    }
}
