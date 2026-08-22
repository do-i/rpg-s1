//! Source-authored quest-board registry schema.
//!
//! The pinned `0897035` Rusted Kingdoms scenario has one list-root quest file,
//! `data/quests.yaml`, containing sixteen ordered entries. Each entry has exactly seven required
//! string-valued fields. Four entries are `main` quests and twelve are `sub` quests.
//!
//! Despite the milestone's earlier generic wording, the pinned quest format has no objective or
//! reward records, conditions, quantities, or optional fields. Progress is represented only by a
//! required started-flag id and completed-flag id; dialogue actions author the actual flag changes
//! and item grants. This module therefore does not invent objective/reward wire shapes or apply
//! runtime status rules. Flag and quest ids remain logical identifiers until later cross-reference
//! validation. Entry identity always comes from the required `id`; the fixed catalog filename does
//! not supply a fallback id.

use bevy::{asset::Asset, reflect::TypePath};
use serde::{Deserialize, Deserializer, de};

use crate::scenario_yaml::deserialize_string;

/// The non-empty list-root `data/quests.yaml` document.
#[derive(Asset, Clone, Debug, Eq, PartialEq, TypePath)]
pub struct QuestCatalogFile(Vec<QuestDefinition>);

impl QuestCatalogFile {
    /// Returns entries in their authored board order.
    pub fn entries(&self) -> &[QuestDefinition] {
        &self.0
    }
}

impl<'de> Deserialize<'de> for QuestCatalogFile {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let entries = Vec::<QuestDefinition>::deserialize(deserializer)?;
        if entries.is_empty() {
            return Err(de::Error::custom(
                "expected a non-empty list of quest entries",
            ));
        }
        Ok(Self(entries))
    }
}

/// One source-authored quest-board row.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct QuestDefinition {
    #[serde(deserialize_with = "deserialize_string")]
    pub id: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub name: String,
    #[serde(rename = "type")]
    pub kind: QuestKind,
    /// Authored display text, not a map identifier.
    #[serde(deserialize_with = "deserialize_string")]
    pub location: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub description: String,
    /// Logical flag id whose presence means the quest has started.
    #[serde(deserialize_with = "deserialize_string")]
    pub started_flag: String,
    /// Logical flag id whose presence takes precedence and means the quest is complete.
    #[serde(deserialize_with = "deserialize_string")]
    pub completed_flag: String,
}

/// The complete quest-kind vocabulary in the pinned corpus.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum QuestKind {
    Main,
    Sub,
}

#[cfg(test)]
mod tests {
    use super::{QuestCatalogFile, QuestKind};
    use crate::scenario_yaml;

    const FIXTURE: &str = include_str!("../tests/fixtures/quest-catalog-shapes.yaml");

    #[test]
    fn loads_every_current_quest_shape_and_preserves_order() {
        let catalog: QuestCatalogFile = scenario_yaml::from_str(FIXTURE)
            .expect("source-shaped quest fixture should deserialize");

        assert_eq!(catalog.entries().len(), 2);
        let main = &catalog.entries()[0];
        assert_eq!(main.id, "main_invented_act");
        assert_eq!(main.name, "An Invented Main Road");
        assert_eq!(main.kind, QuestKind::Main);
        assert_eq!(main.location, "Invented Keep - Invented Shore");
        assert_eq!(main.description, "Follow the invented road.");
        assert_eq!(main.started_flag, "story_invented_started");
        assert_eq!(main.completed_flag, "story_invented_completed");

        let sub = &catalog.entries()[1];
        assert_eq!(sub.id, "sq_invented_errand");
        assert_eq!(sub.kind, QuestKind::Sub);
    }

    #[test]
    fn requires_the_non_empty_list_root_and_all_seven_fields() {
        for document in [
            "[]\n".to_owned(),
            "{}\n".to_owned(),
            FIXTURE.replacen("- id: main_invented_act\n", "-", 1),
            FIXTURE.replacen("  name: An Invented Main Road\n", "", 1),
            FIXTURE.replacen("  type: main\n", "", 1),
            FIXTURE.replacen("  location: Invented Keep - Invented Shore\n", "", 1),
            FIXTURE.replacen("  description: Follow the invented road.\n", "", 1),
            FIXTURE.replacen("  started_flag: story_invented_started\n", "", 1),
            FIXTURE.replacen("  completed_flag: story_invented_completed\n", "", 1),
        ] {
            assert!(
                scenario_yaml::from_str::<QuestCatalogFile>(&document).is_err(),
                "accepted:\n{document}"
            );
        }
    }

    #[test]
    fn rejects_unobserved_kinds_unknown_fields_nulls_and_coerced_strings() {
        for document in [
            FIXTURE.replacen("type: main", "type: side", 1),
            FIXTURE.replacen("- id: main_invented_act", "- id: 42", 1),
            FIXTURE.replacen("  name: An Invented Main Road", "  name: 42", 1),
            FIXTURE.replacen("type: main", "type: null", 1),
            FIXTURE.replacen(
                "  location: Invented Keep - Invented Shore",
                "  location: true",
                1,
            ),
            FIXTURE.replacen(
                "  description: Follow the invented road.",
                "  description: null",
                1,
            ),
            FIXTURE.replacen(
                "  started_flag: story_invented_started",
                "  started_flag: false",
                1,
            ),
            FIXTURE.replacen(
                "  completed_flag: story_invented_completed",
                "  completed_flag: 100",
                1,
            ),
            FIXTURE.replacen(
                "  completed_flag: story_invented_completed",
                "  completed_flag: story_invented_completed\n  objective: invented_target",
                1,
            ),
            FIXTURE.replacen(
                "  completed_flag: story_invented_completed",
                "  completed_flag: story_invented_completed\n  rewards: []",
                1,
            ),
        ] {
            assert!(
                scenario_yaml::from_str::<QuestCatalogFile>(&document).is_err(),
                "accepted:\n{document}"
            );
        }
    }

    #[test]
    fn keeps_logical_ids_and_display_text_losslessly_without_resolving_them() {
        let catalog: QuestCatalogFile = scenario_yaml::from_str(
            "- id: ''\n  name: ''\n  type: sub\n  location: ''\n  description: ''\n  started_flag: ''\n  completed_flag: ''\n",
        )
        .expect("the Python loader rejects null but does not reject empty strings");

        let quest = &catalog.entries()[0];
        assert_eq!(
            (
                quest.id.as_str(),
                quest.name.as_str(),
                quest.location.as_str(),
                quest.description.as_str(),
                quest.started_flag.as_str(),
                quest.completed_flag.as_str(),
            ),
            ("", "", "", "", "", "")
        );
    }

    #[test]
    #[ignore = "requires the separately licensed pinned Python source checkout"]
    fn audits_the_complete_pinned_quest_catalog_when_requested() {
        let path = std::env::var_os("RPG_S1_PINNED_QUESTS_FILE")
            .map(std::path::PathBuf::from)
            .expect("RPG_S1_PINNED_QUESTS_FILE must name the pinned data/quests.yaml file");
        let document = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{} should be readable: {error}", path.display()));
        let catalog: QuestCatalogFile = scenario_yaml::from_str(&document)
            .unwrap_or_else(|error| panic!("{} should load: {error}", path.display()));

        assert_eq!(catalog.entries().len(), 16);
        assert_eq!(
            catalog
                .entries()
                .iter()
                .filter(|quest| quest.kind == QuestKind::Main)
                .count(),
            4
        );
        assert_eq!(
            catalog
                .entries()
                .iter()
                .filter(|quest| quest.kind == QuestKind::Sub)
                .count(),
            12
        );
        assert!(
            catalog.entries()[..4]
                .iter()
                .all(|quest| quest.kind == QuestKind::Main)
        );
        assert!(
            catalog.entries()[4..]
                .iter()
                .all(|quest| quest.kind == QuestKind::Sub)
        );
        assert!(catalog.entries().iter().all(|quest| {
            [
                quest.id.as_str(),
                quest.name.as_str(),
                quest.location.as_str(),
                quest.description.as_str(),
                quest.started_flag.as_str(),
                quest.completed_flag.as_str(),
            ]
            .into_iter()
            .all(|field| !field.is_empty())
        }));
    }
}
