//! Catalog-local duplicate logical-id validation.
//!
//! YAML deserializers intentionally retain authored order, including duplicates.  This module is
//! the validation boundary that rejects duplicate *logical* identifiers without conflating the
//! scenario's deliberately separate namespaces.  A location is stable scenario-relative data:
//! `data/file.yaml#entries[2].id`, rather than an unreliable parser line number.

use std::{collections::BTreeMap, fmt};

use crate::scenario_path::{ScenarioRelativePath, ScenarioRelativePathError};

/// A catalog namespace with one identity policy.
///
/// `Npcs` and `ItemBoxes` are scoped by containing map, while BGM uses qualified
/// `category.event` keys and SFX uses unqualified event keys because that is how the Python
/// managers address them. The manifest and balance file are singletons and therefore have no
/// duplicate-id namespace.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CatalogNamespace {
    Party,
    Classes,
    Abilities,
    Items,
    FieldUseItems,
    Maps,
    Npcs,
    ItemBoxes,
    Dialogue,
    Enemies,
    BossMoveSets,
    Encounters,
    BattleBackgrounds,
    Recipes,
    Quests,
    Bgm,
    Sfx,
}

impl CatalogNamespace {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Party => "party",
            Self::Classes => "classes",
            Self::Abilities => "abilities",
            Self::Items => "items",
            Self::FieldUseItems => "field_use_items",
            Self::Maps => "maps",
            Self::Npcs => "npcs",
            Self::ItemBoxes => "item_boxes",
            Self::Dialogue => "dialogue",
            Self::Enemies => "enemies",
            Self::BossMoveSets => "boss_move_sets",
            Self::Encounters => "encounters",
            Self::BattleBackgrounds => "battle_backgrounds",
            Self::Recipes => "recipes",
            Self::Quests => "quests",
            Self::Bgm => "bgm",
            Self::Sfx => "sfx",
        }
    }
}

/// Stable authored location of one identifier. `path` is scenario-relative and `field_path`
/// identifies the document/list/category position where that id was read.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CatalogIdLocation {
    path: ScenarioRelativePath,
    field_path: String,
}

impl CatalogIdLocation {
    /// Creates a stable scenario-relative diagnostic location.
    pub fn new(
        path: impl AsRef<str>,
        field_path: impl Into<String>,
    ) -> Result<Self, CatalogIdLocationError> {
        let field_path = field_path.into();
        if field_path.is_empty() {
            return Err(CatalogIdLocationError::EmptyFieldPath);
        }
        Ok(Self {
            path: ScenarioRelativePath::try_from(path.as_ref())
                .map_err(CatalogIdLocationError::InvalidPath)?,
            field_path,
        })
    }

    pub fn path(&self) -> &ScenarioRelativePath {
        &self.path
    }

    pub fn field_path(&self) -> &str {
        &self.field_path
    }
}

/// Why a diagnostic location cannot be constructed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CatalogIdLocationError {
    InvalidPath(ScenarioRelativePathError),
    EmptyFieldPath,
}

impl fmt::Display for CatalogIdLocationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPath(error) => write!(
                formatter,
                "invalid scenario-relative location path: {error}"
            ),
            Self::EmptyFieldPath => {
                formatter.write_str("catalog diagnostic field path must not be empty")
            }
        }
    }
}

impl std::error::Error for CatalogIdLocationError {}

impl fmt::Display for CatalogIdLocation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}#{}", self.path.as_str(), self.field_path)
    }
}

/// One logical id supplied by a catalog adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CatalogId {
    pub namespace: CatalogNamespace,
    /// Optional scope within a namespace, for example a map id for item boxes. Empty means the
    /// namespace is scenario-wide.
    pub scope: String,
    pub id: String,
    pub location: CatalogIdLocation,
}

impl CatalogId {
    pub fn scenario_wide(
        namespace: CatalogNamespace,
        id: impl Into<String>,
        location: CatalogIdLocation,
    ) -> Self {
        Self {
            namespace,
            scope: String::new(),
            id: id.into(),
            location,
        }
    }

    pub fn scoped(
        namespace: CatalogNamespace,
        scope: impl Into<String>,
        id: impl Into<String>,
        location: CatalogIdLocation,
    ) -> Self {
        Self {
            namespace,
            scope: scope.into(),
            id: id.into(),
            location,
        }
    }
}

/// A duplicate logical id and its first two authored locations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DuplicateCatalogId {
    pub namespace: CatalogNamespace,
    pub scope: String,
    pub id: String,
    pub first: CatalogIdLocation,
    pub duplicate: CatalogIdLocation,
}

impl fmt::Display for DuplicateCatalogId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let scope = if self.scope.is_empty() {
            String::new()
        } else {
            format!(" in scope `{}`", self.scope)
        };
        write!(
            formatter,
            "duplicate {} id `{}`{}: first at {}; duplicated at {}",
            self.namespace.as_str(),
            self.id,
            scope,
            self.first,
            self.duplicate
        )
    }
}

/// Finds every later occurrence of an id. The first authored occurrence remains the diagnostic
/// anchor, so three duplicates produce two actionable reports.
pub fn find_duplicate_ids(ids: impl IntoIterator<Item = CatalogId>) -> Vec<DuplicateCatalogId> {
    let mut first = BTreeMap::<(CatalogNamespace, String, String), CatalogIdLocation>::new();
    let mut duplicates = Vec::new();
    for entry in ids {
        let key = (entry.namespace, entry.scope.clone(), entry.id.clone());
        if let Some(original) = first.get(&key) {
            duplicates.push(DuplicateCatalogId {
                namespace: entry.namespace,
                scope: entry.scope,
                id: entry.id,
                first: original.clone(),
                duplicate: entry.location,
            });
        } else {
            first.insert(key, entry.location);
        }
    }
    duplicates
}

/// Converts the reusable report into a single validation result.
pub fn validate_unique_ids(
    ids: impl IntoIterator<Item = CatalogId>,
) -> Result<(), Vec<DuplicateCatalogId>> {
    let duplicates = find_duplicate_ids(ids);
    if duplicates.is_empty() {
        Ok(())
    } else {
        Err(duplicates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        scenario_audio::{BgmIndex, SfxIndex},
        scenario_battle_background::BattleBackgroundCatalog,
        scenario_class::ClassDefinition,
        scenario_dialogue::DialogueDocument,
        scenario_encounter::EncounterZone,
        scenario_enemy::{BossMoveSet, EnemyCatalogFile},
        scenario_item::{FieldUseCatalogFile, ItemCatalogFile},
        scenario_map::MapMetadata,
        scenario_party::PartyCatalog,
        scenario_quest::QuestCatalogFile,
        scenario_recipe::RecipeCatalogFile,
        scenario_yaml,
    };
    use std::{
        collections::BTreeMap,
        fs,
        path::{Path, PathBuf},
    };

    fn at(path: &str, field: &str) -> CatalogIdLocation {
        CatalogIdLocation::new(path, field).expect("test location should be valid")
    }

    #[test]
    fn locations_reject_escaping_absolute_and_empty_field_paths() {
        for path in ["/outside.yaml", "../outside.yaml"] {
            assert!(matches!(
                CatalogIdLocation::new(path, "[0].id"),
                Err(CatalogIdLocationError::InvalidPath(_))
            ));
        }
        assert_eq!(
            CatalogIdLocation::new("data/items.yaml", ""),
            Err(CatalogIdLocationError::EmptyFieldPath)
        );
        let location = at("data/items.yaml", "[0].id");
        assert_eq!(location.path().as_str(), "data/items.yaml");
        assert_eq!(location.field_path(), "[0].id");
        assert_eq!(location.to_string(), "data/items.yaml#[0].id");

        let validated = ScenarioRelativePath::try_from("data/quests.yaml").unwrap();
        assert_eq!(
            CatalogIdLocation::new(validated, "[1].id")
                .unwrap()
                .to_string(),
            "data/quests.yaml#[1].id"
        );
    }

    #[test]
    fn duplicate_fixture_reports_id_and_both_stable_locations() {
        let report = find_duplicate_ids([
            CatalogId::scenario_wide(
                CatalogNamespace::Items,
                "invented_ether",
                at("data/items/consumables.yaml", "[0].id"),
            ),
            CatalogId::scenario_wide(
                CatalogNamespace::Items,
                "invented_ether",
                at("data/items/materials.yaml", "[3].id"),
            ),
        ]);
        assert_eq!(report.len(), 1);
        assert_eq!(report[0].id, "invented_ether");
        assert_eq!(
            report[0].first.to_string(),
            "data/items/consumables.yaml#[0].id"
        );
        assert_eq!(
            report[0].duplicate.to_string(),
            "data/items/materials.yaml#[3].id"
        );
        assert_eq!(
            report[0].to_string(),
            "duplicate items id `invented_ether`: first at data/items/consumables.yaml#[0].id; duplicated at data/items/materials.yaml#[3].id"
        );
    }

    #[test]
    fn intentionally_separate_namespaces_are_not_conflated() {
        let ids = [
            CatalogId::scenario_wide(
                CatalogNamespace::Items,
                "key",
                at("data/items/key_items.yaml", "[0].id"),
            ),
            CatalogId::scenario_wide(
                CatalogNamespace::Abilities,
                "key",
                at("data/classes/rogue.yaml", "abilities[0].id"),
            ),
            CatalogId::scenario_wide(
                CatalogNamespace::Bgm,
                "battle.normal",
                at("data/audio/bgm_index.yaml", "battle.normal"),
            ),
            CatalogId::scenario_wide(
                CatalogNamespace::Sfx,
                "normal",
                at("data/audio/sfx_index.yaml", "ui.normal"),
            ),
        ];
        assert_eq!(validate_unique_ids(ids), Ok(()));
    }

    #[test]
    fn item_box_ids_are_unique_per_map_not_globally() {
        let ids = [
            CatalogId::scoped(
                CatalogNamespace::ItemBoxes,
                "town_ardel",
                "chest",
                at("data/maps/town_ardel.yaml", "item_boxes[0].id"),
            ),
            CatalogId::scoped(
                CatalogNamespace::ItemBoxes,
                "forest",
                "chest",
                at("data/maps/forest.yaml", "item_boxes[0].id"),
            ),
        ];
        assert_eq!(validate_unique_ids(ids), Ok(()));
    }

    #[test]
    #[ignore = "requires the separately licensed pinned Python source checkout"]
    fn audits_every_modeled_pinned_catalog_namespace_for_duplicate_ids() {
        let root = std::env::var_os("RPG_S1_PINNED_SCENARIO_DIR")
            .map(PathBuf::from)
            .expect("RPG_S1_PINNED_SCENARIO_DIR must name the pinned rusted_kingdoms directory");
        let data = root.join("data");
        let mut ids = Vec::new();
        let mut counts = BTreeMap::<CatalogNamespace, usize>::new();
        let mut add = |entry: CatalogId| {
            *counts.entry(entry.namespace).or_default() += 1;
            ids.push(entry);
        };
        let location = |path: &Path, field: String| {
            let relative = path
                .strip_prefix(&root)
                .expect("catalog must be below scenario root");
            CatalogIdLocation::new(relative.to_string_lossy().as_ref(), field)
                .expect("pinned catalog location should be scenario-relative and non-empty")
        };

        let party_path = data.join("party.yaml");
        let party: PartyCatalog = load(&party_path);
        for (index, member) in party.party.iter().enumerate() {
            add(CatalogId::scenario_wide(
                CatalogNamespace::Party,
                member.data().id.clone(),
                location(&party_path, format!("party[{index}].id")),
            ));
        }

        for path in yaml_files(&data.join("classes")) {
            let class: ClassDefinition = load(&path);
            add(CatalogId::scenario_wide(
                CatalogNamespace::Classes,
                class.class_id.clone(),
                location(&path, "class".to_owned()),
            ));
            for (index, ability) in class.abilities.iter().enumerate() {
                add(CatalogId::scenario_wide(
                    CatalogNamespace::Abilities,
                    ability.id.clone(),
                    location(&path, format!("abilities[{index}].id")),
                ));
            }
        }

        for path in yaml_files(&data.join("items")) {
            if path
                .file_name()
                .is_some_and(|name| name == "field_use.yaml")
            {
                let catalog: FieldUseCatalogFile = load(&path);
                for (index, entry) in catalog.entries().iter().enumerate() {
                    add(CatalogId::scenario_wide(
                        CatalogNamespace::FieldUseItems,
                        entry.id().to_owned(),
                        location(&path, format!("[{index}].id")),
                    ));
                }
            } else {
                let catalog: ItemCatalogFile = load(&path);
                for (index, entry) in catalog.entries().iter().enumerate() {
                    add(CatalogId::scenario_wide(
                        CatalogNamespace::Items,
                        entry.id().to_owned(),
                        location(&path, format!("[{index}].id")),
                    ));
                }
            }
        }

        for path in yaml_files(&data.join("maps")) {
            let map: MapMetadata = load(&path);
            let stem = file_stem(&path);
            let map_id = map.effective_id(stem).to_owned();
            add(CatalogId::scenario_wide(
                CatalogNamespace::Maps,
                map_id.clone(),
                location(&path, "id (or filename stem)".to_owned()),
            ));
            for (index, npc) in map.npcs.iter().enumerate() {
                add(CatalogId::scoped(
                    CatalogNamespace::Npcs,
                    map_id.clone(),
                    npc.id.clone(),
                    location(&path, format!("npcs[{index}].id")),
                ));
            }
            for (index, item_box) in map.item_boxes.iter().enumerate() {
                add(CatalogId::scoped(
                    CatalogNamespace::ItemBoxes,
                    map_id.clone(),
                    item_box.id.clone(),
                    location(&path, format!("item_boxes[{index}].id")),
                ));
            }
        }

        for path in yaml_files(&data.join("dialogue")) {
            let document: DialogueDocument = load(&path);
            add(CatalogId::scenario_wide(
                CatalogNamespace::Dialogue,
                document.effective_id(file_stem(&path)).to_owned(),
                location(&path, "id (or filename stem)".to_owned()),
            ));
        }

        let enemies = data.join("enemies");
        for path in yaml_files(&enemies) {
            if path
                .parent()
                .is_some_and(|parent| parent.ends_with("boss_move_sets"))
            {
                continue;
            }
            let stream = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            let catalog = EnemyCatalogFile::from_yaml_stream(&stream)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            for (index, enemy) in catalog.entries().iter().enumerate() {
                add(CatalogId::scenario_wide(
                    CatalogNamespace::Enemies,
                    enemy.id.clone(),
                    location(&path, format!("document[{index}].id")),
                ));
            }
        }
        for path in yaml_files(&enemies.join("boss_move_sets")) {
            let moves: BossMoveSet = load(&path);
            add(CatalogId::scenario_wide(
                CatalogNamespace::BossMoveSets,
                moves.effective_id(file_stem(&path)).to_owned(),
                location(&path, "filename stem".to_owned()),
            ));
        }

        for path in yaml_files(&data.join("encount")) {
            let zone: EncounterZone = load(&path);
            add(CatalogId::scenario_wide(
                CatalogNamespace::Encounters,
                zone.effective_id(file_stem(&path)).to_owned(),
                location(&path, "id (or filename stem)".to_owned()),
            ));
        }

        let backgrounds_path = data.join("battle_backgrounds.yaml");
        let backgrounds: BattleBackgroundCatalog = load(&backgrounds_path);
        for (index, background) in backgrounds.0.iter().enumerate() {
            add(CatalogId::scenario_wide(
                CatalogNamespace::BattleBackgrounds,
                background.id.clone(),
                location(&backgrounds_path, format!("[{index}].id")),
            ));
        }
        let recipes_path = data.join("recipe/all_recipe.yaml");
        let recipes: RecipeCatalogFile = load(&recipes_path);
        for (index, recipe) in recipes.entries().iter().enumerate() {
            add(CatalogId::scenario_wide(
                CatalogNamespace::Recipes,
                recipe.id.clone(),
                location(&recipes_path, format!("[{index}].id")),
            ));
        }
        let quests_path = data.join("quests.yaml");
        let quests: QuestCatalogFile = load(&quests_path);
        for (index, quest) in quests.entries().iter().enumerate() {
            add(CatalogId::scenario_wide(
                CatalogNamespace::Quests,
                quest.id.clone(),
                location(&quests_path, format!("[{index}].id")),
            ));
        }

        let bgm_path = data.join("audio/bgm_index.yaml");
        let bgm: BgmIndex = load(&bgm_path);
        for (category_index, category) in bgm.categories.iter().enumerate() {
            for (entry_index, entry) in category.entries.iter().enumerate() {
                add(CatalogId::scenario_wide(
                    CatalogNamespace::Bgm,
                    format!("{}.{}", category.id, entry.id),
                    location(
                        &bgm_path,
                        format!(
                            "{}[{category_index}].{}[{entry_index}]",
                            category.id, entry.id
                        ),
                    ),
                ));
            }
        }
        let sfx_path = data.join("audio/sfx_index.yaml");
        let sfx: SfxIndex = load(&sfx_path);
        for (category_index, category) in sfx.categories.iter().enumerate() {
            for (entry_index, entry) in category.entries.iter().enumerate() {
                add(CatalogId::scenario_wide(
                    CatalogNamespace::Sfx,
                    entry.id.clone(),
                    location(
                        &sfx_path,
                        format!(
                            "{}[{category_index}].{}[{entry_index}]",
                            category.id, entry.id
                        ),
                    ),
                ));
            }
        }

        assert_eq!(
            counts,
            BTreeMap::from([
                (CatalogNamespace::Party, 5),
                (CatalogNamespace::Classes, 5),
                (CatalogNamespace::Abilities, 42),
                (CatalogNamespace::Items, 172),
                (CatalogNamespace::FieldUseItems, 13),
                (CatalogNamespace::Maps, 43),
                (CatalogNamespace::Npcs, 77),
                (CatalogNamespace::ItemBoxes, 17),
                (CatalogNamespace::Dialogue, 91),
                (CatalogNamespace::Enemies, 106),
                (CatalogNamespace::BossMoveSets, 9),
                (CatalogNamespace::Encounters, 16),
                (CatalogNamespace::BattleBackgrounds, 13),
                (CatalogNamespace::Recipes, 11),
                (CatalogNamespace::Quests, 16),
                (CatalogNamespace::Bgm, 12),
                (CatalogNamespace::Sfx, 23),
            ])
        );
        assert_eq!(
            validate_unique_ids(ids),
            Ok(()),
            "pinned catalog identities must be unique within their documented namespaces"
        );
    }

    fn load<T: serde::de::DeserializeOwned>(path: &Path) -> T {
        let document =
            fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        scenario_yaml::from_str(&document)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
    }

    fn yaml_files(directory: &Path) -> Vec<PathBuf> {
        let mut files = fs::read_dir(directory)
            .unwrap_or_else(|error| panic!("{}: {error}", directory.display()))
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "yaml")
            })
            .collect::<Vec<_>>();
        files.sort();
        files
    }

    fn file_stem(path: &Path) -> &str {
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .expect("YAML path has UTF-8 stem")
    }
}
