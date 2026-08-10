//! Source-authored background-music and sound-effect index schemas.
//!
//! The pinned Rusted Kingdoms scenario keeps its two indexes at
//! `data/audio/bgm_index.yaml` and `data/audio/sfx_index.yaml`. Both are mapping-root
//! documents whose category and event names are authored strings and whose values are paths
//! relative to the scenario's `assets/audio` directory. This module validates and resolves that
//! data only; audio playback, Bevy loading, volume, looping, and existence checks remain in
//! later tasks.

use bevy::{asset::Asset, reflect::TypePath};
use serde::{Deserialize, Deserializer, de::MapAccess};

use crate::{scenario_path::ScenarioRelativePath, scenario_root::ScenarioRoot, scenario_yaml};

/// Scenario-relative location of the background-music index.
pub const BGM_INDEX_PATH: &str = "data/audio/bgm_index.yaml";
/// Scenario-relative location of the sound-effect index.
pub const SFX_INDEX_PATH: &str = "data/audio/sfx_index.yaml";
/// Base directory for the paths contained by both source index files.
pub const AUDIO_ASSET_ROOT: &str = "assets/audio";

/// A lossless category-indexed BGM catalog.
///
/// Categories and entries retain YAML encounter order. BGM logical keys are category-qualified
/// (`battle.normal`), exactly as Python's `BgmManager` creates them.
#[derive(Asset, Clone, Debug, Eq, PartialEq, TypePath)]
pub struct BgmIndex {
    pub categories: Vec<AudioCategory>,
}

/// A lossless category-indexed SFX catalog.
///
/// Python's `SfxManager` looks up SFX by the unqualified event name. If a future source catalog
/// repeats a name in a later category, its last occurrence wins, matching Python assignment
/// order. Duplicate reporting belongs to M2.24.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SfxIndex {
    pub categories: Vec<AudioCategory>,
}

/// One named audio category and all of its authored entries.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AudioCategory {
    pub id: String,
    pub entries: Vec<AudioEntry>,
}

/// One logical audio event and its path below [`AUDIO_ASSET_ROOT`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AudioEntry {
    pub id: String,
    pub path: ScenarioRelativePath,
}

impl BgmIndex {
    /// Finds a BGM path by its Python-compatible `category.event` logical key.
    pub fn path_for_key(&self, key: &str) -> Option<&ScenarioRelativePath> {
        let (category_id, entry_id) = key.split_once('.')?;
        self.categories
            .iter()
            .rev()
            .find(|category| category.id == category_id)
            .and_then(|category| {
                category
                    .entries
                    .iter()
                    .rev()
                    .find(|entry| entry.id == entry_id)
                    .map(|entry| &entry.path)
            })
    }

    /// Resolves a BGM logical key to the active scenario's AssetServer path.
    pub fn resolve_key(&self, root: &ScenarioRoot, key: &str) -> Option<String> {
        self.path_for_key(key)
            .map(|path| resolve_audio_path(root, path))
    }
}

impl SfxIndex {
    /// Finds an SFX path by its unqualified Python-compatible event key.
    pub fn path_for_key(&self, key: &str) -> Option<&ScenarioRelativePath> {
        let mut effective_categories = Vec::new();
        for category in &self.categories {
            if let Some(position) = effective_categories
                .iter()
                .position(|effective: &&AudioCategory| effective.id == category.id)
            {
                // PyYAML's mapping construction retains a key's original position while its
                // later value replaces the whole nested mapping.
                effective_categories[position] = category;
            } else {
                effective_categories.push(category);
            }
        }

        let mut resolved = None;
        for category in effective_categories {
            let mut effective_entries = Vec::new();
            for entry in &category.entries {
                if let Some(position) = effective_entries
                    .iter()
                    .position(|effective: &&AudioEntry| effective.id == entry.id)
                {
                    // Nested PyYAML mappings use the same replace-without-reordering rule.
                    effective_entries[position] = entry;
                } else {
                    effective_entries.push(entry);
                }
            }
            for entry in effective_entries {
                if entry.id == key {
                    // SfxManager assigns one unqualified key at a time while visiting the
                    // effective top-level category order, so later categories win globally.
                    resolved = Some(&entry.path);
                }
            }
        }
        resolved
    }

    /// Resolves an SFX event key to the active scenario's AssetServer path.
    pub fn resolve_key(&self, root: &ScenarioRoot, key: &str) -> Option<String> {
        self.path_for_key(key)
            .map(|path| resolve_audio_path(root, path))
    }
}

/// Resolves one index value beneath `assets/audio` and the active scenario package.
pub fn resolve_audio_path(
    root: &ScenarioRoot,
    audio_relative_path: &ScenarioRelativePath,
) -> String {
    let scenario_path = ScenarioRelativePath::try_from(format!(
        "{AUDIO_ASSET_ROOT}/{}",
        audio_relative_path.as_str()
    ))
    .expect("the validated audio base and relative path must remain scenario-relative");
    root.resolve(&scenario_path)
}

impl<'de> Deserialize<'de> for BgmIndex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_categories(deserializer).map(|categories| Self { categories })
    }
}

impl<'de> Deserialize<'de> for SfxIndex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_categories(deserializer).map(|categories| Self { categories })
    }
}

fn deserialize_categories<'de, D>(deserializer: D) -> Result<Vec<AudioCategory>, D::Error>
where
    D: Deserializer<'de>,
{
    struct CategoriesVisitor;

    impl<'de> serde::de::Visitor<'de> for CategoriesVisitor {
        type Value = Vec<AudioCategory>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a mapping of audio categories to event-to-path mappings")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut categories = Vec::new();
            while let Some((id, entries)) = map.next_entry::<StrictString, AudioEntries>()? {
                categories.push(AudioCategory {
                    id: id.0,
                    entries: entries.0,
                });
            }
            Ok(categories)
        }
    }

    deserializer.deserialize_map(CategoriesVisitor)
}

struct AudioEntries(Vec<AudioEntry>);

impl<'de> Deserialize<'de> for AudioEntries {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct EntriesVisitor;

        impl<'de> serde::de::Visitor<'de> for EntriesVisitor {
            type Value = AudioEntries;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a mapping of audio event names to scenario-relative paths")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut entries = Vec::new();
                while let Some((id, path)) =
                    map.next_entry::<StrictString, ScenarioRelativePath>()?
                {
                    entries.push(AudioEntry { id: id.0, path });
                }
                Ok(AudioEntries(entries))
            }
        }

        deserializer.deserialize_map(EntriesVisitor)
    }
}

struct StrictString(String);

impl<'de> Deserialize<'de> for StrictString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        scenario_yaml::deserialize_string(deserializer).map(Self)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{BGM_INDEX_PATH, BgmIndex, SFX_INDEX_PATH, SfxIndex};
    use crate::{scenario_root::ScenarioRoot, scenario_yaml};

    #[test]
    fn loads_source_shaped_indexes_and_resolves_their_distinct_logical_keys() {
        let bgm: BgmIndex =
            scenario_yaml::from_str(include_str!("../tests/fixtures/audio-bgm-index.yaml"))
                .expect("invented BGM index should load");
        let sfx: SfxIndex =
            scenario_yaml::from_str(include_str!("../tests/fixtures/audio-sfx-index.yaml"))
                .expect("invented SFX index should load");
        let root = ScenarioRoot::default();

        assert_eq!(bgm.categories.len(), 2);
        assert_eq!(bgm.categories[0].id, "battle");
        assert_eq!(bgm.categories[0].entries[1].id, "boss");
        assert_eq!(
            bgm.path_for_key("battle.normal").unwrap().as_str(),
            "bgm/Invented_March.mp3"
        );
        assert_eq!(
            bgm.resolve_key(&root, "battle.normal"),
            Some("scenarios/rusted_kingdoms/assets/audio/bgm/Invented_March.mp3".to_owned())
        );
        assert_eq!(sfx.categories.len(), 2);
        assert_eq!(
            sfx.path_for_key("confirm").unwrap().as_str(),
            "sfx/ui/confirm.mp3"
        );
        assert_eq!(
            sfx.resolve_key(&root, "confirm"),
            Some("scenarios/rusted_kingdoms/assets/audio/sfx/ui/confirm.mp3".to_owned())
        );
        assert_eq!(bgm.resolve_key(&root, "battle"), None);
        assert_eq!(bgm.resolve_key(&root, "battle.unknown"), None);
        assert_eq!(sfx.resolve_key(&root, "unknown"), None);
        assert_eq!(BGM_INDEX_PATH, "data/audio/bgm_index.yaml");
        assert_eq!(SFX_INDEX_PATH, "data/audio/sfx_index.yaml");
    }

    #[test]
    fn preserves_the_python_sfx_later_category_overwrite_behavior() {
        let index: SfxIndex = scenario_yaml::from_str(
            "first:\n  shared: sfx/first.mp3\nsecond:\n  shared: sfx/second.mp3\n",
        )
        .expect("source-shaped duplicate SFX keys should remain representable before M2.24");
        assert_eq!(index.categories.len(), 2);
        assert_eq!(
            index.path_for_key("shared").unwrap().as_str(),
            "sfx/second.mp3"
        );
    }

    #[test]
    fn sfx_duplicate_categories_replace_entries_without_reordering_categories() {
        let index: SfxIndex = scenario_yaml::from_str(
            "first:\n  shared: sfx/first.mp3\n  removed: sfx/removed.mp3\nsecond:\n  shared: sfx/second.mp3\nfirst:\n  replacement: sfx/replacement.mp3\n",
        )
        .expect("duplicate YAML categories should retain raw authored data before M2.24");

        // PyYAML keeps `first` before `second`, replaces its whole nested mapping, and then
        // SfxManager assigns `shared` from `second` after `first` no longer supplies it.
        assert_eq!(index.categories.len(), 3);
        assert_eq!(
            index.path_for_key("shared").unwrap().as_str(),
            "sfx/second.mp3"
        );
        assert_eq!(
            index.path_for_key("replacement").unwrap().as_str(),
            "sfx/replacement.mp3"
        );
        assert_eq!(index.path_for_key("removed"), None);
    }

    #[test]
    fn sfx_duplicate_events_replace_paths_without_reordering_events() {
        let index: SfxIndex = scenario_yaml::from_str(
            "ui:\n  shared: sfx/first.mp3\n  other: sfx/other.mp3\n  shared: sfx/replacement.mp3\n",
        )
        .expect("duplicate YAML event names should remain representable before M2.24");
        assert_eq!(
            index.path_for_key("shared").unwrap().as_str(),
            "sfx/replacement.mp3"
        );
        assert_eq!(
            index.path_for_key("other").unwrap().as_str(),
            "sfx/other.mp3"
        );
    }

    #[test]
    fn preserves_the_python_bgm_later_mapping_overwrite_behavior() {
        let index: BgmIndex = scenario_yaml::from_str(
            "battle:\n  normal: bgm/first.mp3\nbattle:\n  normal: bgm/second.mp3\n",
        )
        .expect("duplicate YAML mappings should remain representable before M2.24");
        assert_eq!(index.categories.len(), 2);
        assert_eq!(
            index.path_for_key("battle.normal").unwrap().as_str(),
            "bgm/second.mp3"
        );
    }

    #[test]
    fn rejects_non_mapping_null_coerced_unknown_and_escaping_shapes() {
        for document in [
            "- battle\n",
            "battle: null\n",
            "battle: sfx/battle.mp3\n",
            "battle:\n  encounter: null\n",
            "battle:\n  encounter: 42\n",
            "battle:\n  encounter: true\n",
            "42:\n  encounter: sfx/battle.mp3\n",
            "battle:\n  42: sfx/battle.mp3\n",
            "battle:\n  encounter: ../outside.mp3\n",
            "battle:\n  encounter: /outside.mp3\n",
            "battle:\n  encounter: sfx\\\\outside.mp3\n",
        ] {
            assert!(
                scenario_yaml::from_str::<BgmIndex>(document).is_err(),
                "{document}"
            );
            assert!(
                scenario_yaml::from_str::<SfxIndex>(document).is_err(),
                "{document}"
            );
        }
    }

    #[test]
    fn accepts_empty_mapping_without_inventing_defaults_or_metadata() {
        let bgm: BgmIndex = scenario_yaml::from_str("{}\n").unwrap();
        let sfx: SfxIndex = scenario_yaml::from_str("{}\n").unwrap();
        assert!(bgm.categories.is_empty());
        assert!(sfx.categories.is_empty());
    }

    #[test]
    #[ignore = "requires the separately licensed pinned Python source checkout"]
    fn audits_the_complete_pinned_audio_indexes_when_requested() {
        let root = std::env::var_os("RPG_S1_PINNED_AUDIO_DIR")
            .map(std::path::PathBuf::from)
            .expect("RPG_S1_PINNED_AUDIO_DIR must name the pinned data/audio directory");
        let bgm_document = fs::read_to_string(root.join("bgm_index.yaml"))
            .expect("pinned BGM index should be readable");
        let sfx_document = fs::read_to_string(root.join("sfx_index.yaml"))
            .expect("pinned SFX index should be readable");
        let bgm: BgmIndex =
            scenario_yaml::from_str(&bgm_document).expect("pinned BGM index should load");
        let sfx: SfxIndex =
            scenario_yaml::from_str(&sfx_document).expect("pinned SFX index should load");
        let scenario_root = ScenarioRoot::default();

        assert_eq!(bgm.categories.len(), 5);
        assert_eq!(
            bgm.categories
                .iter()
                .map(|category| category.entries.len())
                .sum::<usize>(),
            12
        );
        assert_eq!(sfx.categories.len(), 2);
        assert_eq!(
            sfx.categories
                .iter()
                .map(|category| category.entries.len())
                .sum::<usize>(),
            23
        );
        assert_eq!(
            bgm.path_for_key("title.default").unwrap().as_str(),
            "bgm/Chronicles_of_the_Lost_Flame_Title.mp3"
        );
        assert_eq!(
            bgm.path_for_key("battle.boss").unwrap().as_str(),
            "bgm/Crimson_Storm_s_Echo.mp3"
        );
        assert_eq!(
            sfx.path_for_key("confirm").unwrap().as_str(),
            "sfx/ui_menu/013_Confirm_03.mp3"
        );
        assert!(
            bgm.categories
                .iter()
                .flat_map(|category| &category.entries)
                .all(|entry| entry.path.as_str().ends_with(".mp3"))
        );
        assert!(
            sfx.categories
                .iter()
                .flat_map(|category| &category.entries)
                .all(|entry| entry.path.as_str().ends_with(".mp3"))
        );
        assert_eq!(
            bgm.resolve_key(&scenario_root, "town.default"),
            Some("scenarios/rusted_kingdoms/assets/audio/bgm/Whiteveil_Streets.mp3".to_owned())
        );
        assert_eq!(
            sfx.resolve_key(&scenario_root, "use_item"),
            Some(
                "scenarios/rusted_kingdoms/assets/audio/sfx/ui_menu/051_use_item_01.mp3".to_owned()
            )
        );
    }
}
