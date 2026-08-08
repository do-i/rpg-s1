//! Whole-scenario loading and cross-reference validation.
//!
//! This is the production validation boundary for the typed M2 scenario schemas. It deliberately
//! keeps host paths private: diagnostics identify the selected [`ScenarioRoot`] package and a
//! stable scenario-relative `file#field` location. The pinned Python validator is the behavioral
//! baseline, while additional checks (AI references, audio, derived assets, and all typed catalog
//! edges) implement the stricter direct-authoring policy recorded by ADR 0002.
//!
//! No catalog is reparsed into generic YAML. TMX portal links and TSX image references are the
//! only relevant source edges not modeled here: ADR 0003 assigns their typed XML parsing to M4,
//! so this milestone checks same-stem TMX presence but does not implement an ad-hoc XML scanner.
//! The pinned quest schema contains flag-backed board rows rather than objective/reward records;
//! those flag edges are therefore the complete current quest reference surface.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt, fs, io,
    path::{Path, PathBuf},
};

use serde::de::DeserializeOwned;

use crate::{
    manifest_path_validation::{
        ManifestPathValidationResult, ScenarioPathProbeResult, validate_manifest_paths,
    },
    scenario_audio::{AUDIO_ASSET_ROOT, BGM_INDEX_PATH, BgmIndex, SFX_INDEX_PATH, SfxIndex},
    scenario_balance::BalanceData,
    scenario_battle_background::BattleBackgroundCatalog,
    scenario_class::{AbilityElement, AbilityKind, ClassDefinition},
    scenario_condition::FlagConditions,
    scenario_dialogue::{DialogueActions, DialogueDocument},
    scenario_duplicate_id::{CatalogId, CatalogIdLocation, CatalogNamespace, find_duplicate_ids},
    scenario_encounter::EncounterZone,
    scenario_enemy::{
        BossMoveSet, EnemyAi, EnemyBehavior, EnemyCatalogFile, EnemyMove, EnemyTargetOverride,
    },
    scenario_item::{AccessoryStats, FieldUseCatalogFile, ItemCatalogFile, ItemDefinition},
    scenario_manifest::Manifest,
    scenario_map::MapMetadata,
    scenario_party::PartyCatalog,
    scenario_path::ScenarioRelativePath,
    scenario_quest::QuestCatalogFile,
    scenario_recipe::RecipeCatalogFile,
    scenario_root::{SCENARIO_MANIFEST_PATH, ScenarioRoot},
    scenario_yaml,
};

/// Whether a validation finding prevents a strictly valid scenario.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

/// A stable source location. No machine-specific filesystem path is retained.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ScenarioLocation {
    pub path: ScenarioRelativePath,
    pub field_path: String,
}

impl ScenarioLocation {
    fn new(path: &str, field_path: impl Into<String>) -> Self {
        Self {
            path: ScenarioRelativePath::try_from(path)
                .expect("validator-created paths must remain scenario-relative"),
            field_path: field_path.into(),
        }
    }
}

impl fmt::Display for ScenarioLocation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}#{}", self.path, self.field_path)
    }
}

/// One actionable validation finding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScenarioDiagnostic {
    pub severity: DiagnosticSeverity,
    pub code: &'static str,
    pub location: ScenarioLocation,
    pub message: String,
}

impl fmt::Display for ScenarioDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}:{}: {}: {}",
            self.location.path, self.location.field_path, self.code, self.message
        )
    }
}

/// Meaningful corpus totals returned even when cross-reference findings exist.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ScenarioCatalogCounts {
    pub party_members: usize,
    pub classes: usize,
    pub abilities: usize,
    pub items: usize,
    pub field_use_items: usize,
    pub maps: usize,
    pub dialogue_documents: usize,
    pub enemies: usize,
    pub boss_move_sets: usize,
    pub encounters: usize,
    pub battle_backgrounds: usize,
    pub recipes: usize,
    pub quests: usize,
    pub bgm_keys: usize,
    pub sfx_keys: usize,
}

/// Complete validation result. Diagnostics are sorted by severity, location, code, and message.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ScenarioValidationReport {
    pub package_key: String,
    pub counts: ScenarioCatalogCounts,
    pub checked_references: usize,
    pub diagnostics: Vec<ScenarioDiagnostic>,
}

impl ScenarioValidationReport {
    pub fn is_valid(&self) -> bool {
        !self
            .diagnostics
            .iter()
            .any(|finding| finding.severity == DiagnosticSeverity::Error)
    }

    pub fn errors(&self) -> impl Iterator<Item = &ScenarioDiagnostic> {
        self.diagnostics
            .iter()
            .filter(|finding| finding.severity == DiagnosticSeverity::Error)
    }

    pub fn warnings(&self) -> impl Iterator<Item = &ScenarioDiagnostic> {
        self.diagnostics
            .iter()
            .filter(|finding| finding.severity == DiagnosticSeverity::Warning)
    }

    fn finish(&mut self) {
        self.diagnostics.sort_by(|left, right| {
            left.severity
                .cmp(&right.severity)
                .then_with(|| left.location.cmp(&right.location))
                .then_with(|| left.code.cmp(right.code))
                .then_with(|| left.message.cmp(&right.message))
        });
    }
}

#[derive(Clone, Debug)]
struct Located<T> {
    path: String,
    stem: String,
    value: T,
}

#[derive(Default)]
struct ScenarioCatalogs {
    manifest: Option<Manifest>,
    party: Option<Located<PartyCatalog>>,
    classes: Vec<Located<ClassDefinition>>,
    items: Vec<Located<ItemCatalogFile>>,
    field_use: Option<Located<FieldUseCatalogFile>>,
    maps: Vec<Located<MapMetadata>>,
    dialogue: Vec<Located<DialogueDocument>>,
    enemies: Vec<Located<EnemyCatalogFile>>,
    boss_move_sets: Vec<Located<BossMoveSet>>,
    encounters: Vec<Located<EncounterZone>>,
    recipes: Vec<Located<RecipeCatalogFile>>,
    quests: Option<Located<QuestCatalogFile>>,
    balance: Option<Located<BalanceData>>,
    backgrounds: Option<Located<BattleBackgroundCatalog>>,
    bgm: Option<Located<BgmIndex>>,
    sfx: Option<Located<SfxIndex>>,
}

/// Loads every manifest-selected typed catalog and validates the whole scenario.
///
/// `physical_root` is the host directory containing `manifest.yaml`; it is never placed in a
/// diagnostic. [`ScenarioRoot`] supplies the selected package identity and future AssetServer
/// resolution context. Missing and malformed files are accumulated where possible.
pub fn validate_scenario_directory(
    scenario_root: &ScenarioRoot,
    physical_root: impl AsRef<Path>,
) -> ScenarioValidationReport {
    let mut validator = Validator::new(scenario_root, physical_root.as_ref());
    validator.load();
    validator.validate();
    validator.report.finish();
    validator.report
}

struct Validator<'a> {
    physical_root: &'a Path,
    canonical_root: Option<PathBuf>,
    report: ScenarioValidationReport,
    catalogs: ScenarioCatalogs,
}

impl<'a> Validator<'a> {
    fn new(scenario_root: &ScenarioRoot, physical_root: &'a Path) -> Self {
        Self {
            physical_root,
            canonical_root: physical_root.canonicalize().ok(),
            report: ScenarioValidationReport {
                package_key: scenario_root.package_key().to_owned(),
                ..Default::default()
            },
            catalogs: ScenarioCatalogs::default(),
        }
    }

    fn diagnostic(
        &mut self,
        severity: DiagnosticSeverity,
        code: &'static str,
        path: &str,
        field_path: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.report.diagnostics.push(ScenarioDiagnostic {
            severity,
            code,
            location: ScenarioLocation::new(path, field_path),
            message: message.into(),
        });
    }

    fn error(
        &mut self,
        code: &'static str,
        path: &str,
        field_path: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.diagnostic(DiagnosticSeverity::Error, code, path, field_path, message);
    }

    fn warning(
        &mut self,
        code: &'static str,
        path: &str,
        field_path: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.diagnostic(DiagnosticSeverity::Warning, code, path, field_path, message);
    }

    fn load(&mut self) {
        let Some(manifest) = self.read_yaml::<Manifest>(SCENARIO_MANIFEST_PATH) else {
            return;
        };
        self.validate_manifest_paths(&manifest);

        let party_path = manifest.refs.party.as_str().to_owned();
        self.catalogs.party = self
            .read_yaml::<PartyCatalog>(&party_path)
            .map(|value| located(&party_path, value));
        self.catalogs.classes = self.load_yaml_directory(manifest.refs.classes.as_str());
        self.load_items(manifest.refs.items.as_str());
        self.catalogs.maps = self.load_yaml_directory(manifest.refs.maps.as_str());
        self.catalogs.dialogue = self.load_yaml_directory(manifest.refs.dialogue.as_str());
        self.load_enemies(manifest.refs.enemies.as_str());
        self.catalogs.encounters = self.load_yaml_directory(manifest.refs.encount.as_str());
        self.catalogs.recipes = self.load_yaml_directory(manifest.refs.recipe.as_str());

        let quests_path = manifest.refs.quests.as_str().to_owned();
        self.catalogs.quests = self
            .read_yaml::<QuestCatalogFile>(&quests_path)
            .map(|value| located(&quests_path, value));
        let balance_path = manifest.refs.balance.as_str().to_owned();
        self.catalogs.balance = self
            .read_yaml::<BalanceData>(&balance_path)
            .map(|value| located(&balance_path, value));
        let backgrounds_path = manifest.refs.battle_backgrounds.as_str().to_owned();
        self.catalogs.backgrounds = self
            .read_yaml::<BattleBackgroundCatalog>(&backgrounds_path)
            .map(|value| located(&backgrounds_path, value));
        self.catalogs.bgm = self
            .read_yaml::<BgmIndex>(BGM_INDEX_PATH)
            .map(|value| located(BGM_INDEX_PATH, value));
        self.catalogs.sfx = self
            .read_yaml::<SfxIndex>(SFX_INDEX_PATH)
            .map(|value| located(SFX_INDEX_PATH, value));
        self.catalogs.manifest = Some(manifest);
        self.update_counts();
    }

    fn validate_manifest_paths(&mut self, manifest: &Manifest) {
        let root = self.physical_root;
        let canonical_root = self.canonical_root.clone();
        let probe = |path: &ScenarioRelativePath| probe_path(root, canonical_root.as_deref(), path);
        for finding in validate_manifest_paths(manifest, &probe) {
            self.report.checked_references += 1;
            match finding.result {
                ManifestPathValidationResult::Exists => {}
                ManifestPathValidationResult::Missing => self.error(
                    "path.missing",
                    SCENARIO_MANIFEST_PATH,
                    finding.reference.field_path,
                    format!(
                        "referenced {} is missing: {}",
                        finding.reference.expected_kind, finding.reference.path
                    ),
                ),
                ManifestPathValidationResult::WrongKind { actual_kind } => self.error(
                    "path.wrong_kind",
                    SCENARIO_MANIFEST_PATH,
                    finding.reference.field_path,
                    format!(
                        "expected {} but found {actual_kind}: {}",
                        finding.reference.expected_kind, finding.reference.path
                    ),
                ),
            }
        }
    }

    fn read_yaml<T: DeserializeOwned>(&mut self, relative: &str) -> Option<T> {
        let text = self.read_text(relative)?;
        match scenario_yaml::from_str(&text) {
            Ok(value) => Some(value),
            Err(error) => {
                self.error("yaml.invalid", relative, error.path(), error.to_string());
                None
            }
        }
    }

    fn read_enemy_stream(&mut self, relative: &str) -> Option<EnemyCatalogFile> {
        let text = self.read_text(relative)?;
        match EnemyCatalogFile::from_yaml_stream(&text) {
            Ok(value) => Some(value),
            Err(error) => {
                self.error("yaml.invalid", relative, error.path(), error.to_string());
                None
            }
        }
    }

    fn read_text(&mut self, relative: &str) -> Option<String> {
        let Ok(relative_path) = ScenarioRelativePath::try_from(relative) else {
            self.error(
                "path.invalid",
                SCENARIO_MANIFEST_PATH,
                "$loader",
                "loader received an invalid scenario-relative path",
            );
            return None;
        };
        let path = self.physical_root.join(relative);
        let Some(canonical_root) = self.canonical_root.as_deref() else {
            self.error(
                "io.root",
                relative_path.as_str(),
                "$",
                "scenario root cannot be resolved",
            );
            return None;
        };
        let Ok(canonical_path) = path.canonicalize() else {
            self.error(
                "io.read",
                relative_path.as_str(),
                "$",
                "scenario file cannot be resolved",
            );
            return None;
        };
        if !canonical_path.starts_with(canonical_root) {
            self.error(
                "path.escape",
                relative_path.as_str(),
                "$",
                "scenario file resolves outside the scenario root",
            );
            return None;
        }
        if !canonical_path.is_file() {
            self.error(
                "io.read",
                relative_path.as_str(),
                "$",
                "scenario path is not a readable file",
            );
            return None;
        }
        match fs::read_to_string(&canonical_path) {
            Ok(text) => Some(text),
            Err(_) => {
                self.error(
                    "io.read",
                    relative_path.as_str(),
                    "$",
                    "scenario file could not be read as UTF-8 text",
                );
                None
            }
        }
    }

    fn load_yaml_directory<T: DeserializeOwned>(&mut self, directory: &str) -> Vec<Located<T>> {
        self.yaml_files(directory)
            .into_iter()
            .filter_map(|path| {
                self.read_yaml(&path)
                    .map(|value| located(path.as_str(), value))
            })
            .collect()
    }

    fn load_items(&mut self, directory: &str) {
        for path in self.yaml_files(directory) {
            if path.ends_with("/field_use.yaml") {
                self.catalogs.field_use = self
                    .read_yaml::<FieldUseCatalogFile>(&path)
                    .map(|value| located(&path, value));
            } else if let Some(value) = self.read_yaml::<ItemCatalogFile>(&path) {
                self.catalogs.items.push(located(&path, value));
            }
        }
    }

    fn load_enemies(&mut self, directory: &str) {
        for path in self.yaml_files(directory) {
            let filename = Path::new(&path)
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or_default();
            if filename.starts_with("enemies_rank_") {
                if let Some(value) = self.read_enemy_stream(&path) {
                    self.catalogs.enemies.push(located(&path, value));
                }
            } else if let Some(value) = self.read_yaml::<BossMoveSet>(&path) {
                self.catalogs.boss_move_sets.push(located(&path, value));
            }
        }
    }

    fn yaml_files(&mut self, directory: &str) -> Vec<String> {
        let mut paths = Vec::new();
        let mut visited_directories = BTreeSet::new();
        let directory_path = self.physical_root.join(directory);
        if let Err(error) = collect_yaml_files(
            self.physical_root,
            self.canonical_root.as_deref(),
            &directory_path,
            &mut visited_directories,
            &mut paths,
        ) {
            self.error("io.walk", directory, "$", error.to_string());
        }
        paths.sort();
        paths
    }

    fn update_counts(&mut self) {
        let counts = &mut self.report.counts;
        if let Some(party) = self.catalogs.party.clone() {
            counts.party_members = party.value.party.len();
        }
        counts.classes = self.catalogs.classes.len();
        counts.abilities = self
            .catalogs
            .classes
            .iter()
            .map(|class| class.value.abilities.len())
            .sum();
        counts.items = self
            .catalogs
            .items
            .iter()
            .map(|file| file.value.entries().len())
            .sum();
        counts.field_use_items = self
            .catalogs
            .field_use
            .as_ref()
            .map_or(0, |file| file.value.entries().len());
        counts.maps = self.catalogs.maps.len();
        counts.dialogue_documents = self.catalogs.dialogue.len();
        counts.enemies = self
            .catalogs
            .enemies
            .iter()
            .map(|file| file.value.entries().len())
            .sum();
        counts.boss_move_sets = self.catalogs.boss_move_sets.len();
        counts.encounters = self.catalogs.encounters.len();
        counts.recipes = self
            .catalogs
            .recipes
            .iter()
            .map(|file| file.value.entries().len())
            .sum();
        counts.quests = self
            .catalogs
            .quests
            .as_ref()
            .map_or(0, |file| file.value.entries().len());
        counts.battle_backgrounds = self
            .catalogs
            .backgrounds
            .as_ref()
            .map_or(0, |file| file.value.0.len());
        counts.bgm_keys = self.catalogs.bgm.as_ref().map_or(0, |index| {
            index
                .value
                .categories
                .iter()
                .map(|category| category.entries.len())
                .sum()
        });
        counts.sfx_keys = self.catalogs.sfx.as_ref().map_or(0, |index| {
            index
                .value
                .categories
                .iter()
                .map(|category| category.entries.len())
                .sum()
        });
    }

    fn validate(&mut self) {
        let Some(manifest) = self.catalogs.manifest.clone() else {
            return;
        };
        let mut index = ReferenceIndex::from_catalogs(&self.catalogs);
        self.add_tmx_map_ids(&mut index);
        self.validate_duplicates(&index);
        self.validate_manifest_ids(&manifest, &index);
        self.validate_party(&index);
        self.validate_items(&index);
        self.validate_maps(&index);
        self.validate_dialogue(&index);
        self.validate_enemies(&index);
        self.validate_encounters(&index);
        self.validate_recipes(&index);
        self.validate_audio(&index);
        self.validate_flags(&index);
    }

    fn add_tmx_map_ids(&mut self, index: &mut ReferenceIndex) {
        let directory = self.physical_root.join(&index.tmx_root);
        let Some(canonical_root) = self.canonical_root.as_deref() else {
            return;
        };
        let Ok(canonical_directory) = directory.canonicalize() else {
            return;
        };
        if !canonical_directory.starts_with(canonical_root) || !canonical_directory.is_dir() {
            return;
        }
        let Ok(entries) = fs::read_dir(directory) else {
            return;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let contained_file = path
                .strip_prefix(self.physical_root)
                .ok()
                .and_then(|relative| relative.to_str())
                .and_then(|relative| ScenarioRelativePath::try_from(relative).ok())
                .is_some_and(|relative| {
                    probe_path(
                        self.physical_root,
                        self.canonical_root.as_deref(),
                        &relative,
                    ) == ScenarioPathProbeResult::File
                });
            if path.extension().is_some_and(|extension| extension == "tmx")
                && contained_file
                && let Some(stem) = path.file_stem().and_then(|stem| stem.to_str())
            {
                index.maps.insert(stem.to_owned());
            }
        }
    }

    fn checked(
        &mut self,
        namespace: &'static str,
        known: &BTreeSet<String>,
        id: &str,
        path: &str,
        field: impl Into<String>,
    ) {
        if id.is_empty() {
            return;
        }
        self.report.checked_references += 1;
        if !known.contains(id) {
            self.error(
                "reference.missing",
                path,
                field,
                format!("unknown {namespace} id `{id}`"),
            );
        }
    }

    fn checked_path(&mut self, path: &str, field: impl Into<String>, referenced: &str) {
        self.report.checked_references += 1;
        let Ok(relative) = ScenarioRelativePath::try_from(referenced) else {
            self.error(
                "path.invalid",
                path,
                field,
                format!("invalid scenario-relative path `{referenced}`"),
            );
            return;
        };
        if probe_path(
            self.physical_root,
            self.canonical_root.as_deref(),
            &relative,
        ) != ScenarioPathProbeResult::File
        {
            self.error(
                "path.missing",
                path,
                field,
                format!("referenced file is missing: {referenced}"),
            );
        }
    }

    fn validate_manifest_ids(&mut self, manifest: &Manifest, index: &ReferenceIndex) {
        self.checked(
            "protagonist party member",
            &index.protagonists,
            &manifest.protagonist.id,
            SCENARIO_MANIFEST_PATH,
            "protagonist.id",
        );
        self.checked(
            "class",
            &index.classes,
            &manifest.protagonist.class,
            SCENARIO_MANIFEST_PATH,
            "protagonist.class",
        );
        self.checked(
            "map",
            &index.maps,
            &manifest.start.map,
            SCENARIO_MANIFEST_PATH,
            "start.map",
        );
        if let Some(party) = self.catalogs.party.clone() {
            if index.protagonists.len() != 1 {
                self.error(
                    "party.protagonist_count",
                    &party.path,
                    "party",
                    format!(
                        "expected exactly one protagonist but found {}",
                        index.protagonists.len()
                    ),
                );
            }
            if let Some(member) = party
                .value
                .party
                .iter()
                .find(|member| member.data().id == manifest.protagonist.id)
                && member.data().class_id != manifest.protagonist.class
            {
                self.error(
                    "reference.disagreement",
                    SCENARIO_MANIFEST_PATH,
                    "protagonist.class",
                    format!(
                        "manifest class `{}` disagrees with party member `{}` class `{}`",
                        manifest.protagonist.class,
                        manifest.protagonist.id,
                        member.data().class_id
                    ),
                );
            }
        }
        let intro = self
            .catalogs
            .dialogue
            .iter()
            .find(|document| document.path == manifest.start.intro_dialogue.as_str());
        self.report.checked_references += 1;
        if intro.is_none() {
            self.error(
                "reference.missing",
                SCENARIO_MANIFEST_PATH,
                "start.intro_dialogue",
                "intro path does not identify a loaded dialogue document",
            );
        }
    }

    fn validate_party(&mut self, index: &ReferenceIndex) {
        let Some(file) = self.catalogs.party.clone() else {
            return;
        };
        for (member_index, member) in file.value.party.iter().enumerate() {
            let data = member.data();
            let base = format!("party[{member_index}]");
            self.checked(
                "class",
                &index.classes,
                &data.class_id,
                &file.path,
                format!("{base}.class"),
            );
            self.checked(
                "map",
                &index.maps,
                &data.join.map,
                &file.path,
                format!("{base}.join.map"),
            );
            self.checked_path(
                &file.path,
                format!("{base}.portrait"),
                data.portrait.as_str(),
            );
            for (slot, namespace, known, id) in [
                ("weapon", "weapon", &index.weapons, &data.equipped.weapon),
                ("shield", "shield", &index.shields, &data.equipped.shield),
                ("helmet", "helmet", &index.helmets, &data.equipped.helmet),
                ("body", "body armor", &index.bodies, &data.equipped.body),
                (
                    "accessory",
                    "accessory",
                    &index.accessories,
                    &data.equipped.accessory,
                ),
            ] {
                self.checked(
                    namespace,
                    known,
                    id,
                    &file.path,
                    format!("{base}.equipped.{slot}"),
                );
            }
            let no_abilities = BTreeSet::new();
            let class_abilities = index
                .class_abilities
                .get(&data.class_id)
                .unwrap_or(&no_abilities);
            for (ability_index, ability) in data.abilities_unlocked.iter().enumerate() {
                self.checked(
                    "ability owned by the member class",
                    class_abilities,
                    ability,
                    &file.path,
                    format!("{base}.abilities_unlocked[{ability_index}]"),
                );
            }
            if let Some(recruit) = member.recruit() {
                self.checked(
                    "dialogue",
                    &index.dialogue,
                    &recruit.dialogue,
                    &file.path,
                    format!("{base}.recruit.dialogue"),
                );
                self.report.checked_references += 1;
                if !index
                    .npcs
                    .get(&data.join.map)
                    .is_some_and(|ids| ids.contains(&recruit.npc))
                {
                    self.error(
                        "reference.missing",
                        &file.path,
                        format!("{base}.recruit.npc"),
                        format!(
                            "unknown NPC id `{}` on join map `{}`",
                            recruit.npc, data.join.map
                        ),
                    );
                }
            }
        }
    }

    fn validate_items(&mut self, index: &ReferenceIndex) {
        let items = self.catalogs.items.clone();
        for file in items {
            for (item_index, item) in file.value.entries().iter().enumerate() {
                let base = format!("[{item_index}]");
                match item {
                    ItemDefinition::Key(_) => {}
                    ItemDefinition::Accessory(item) => {
                        for (class_index, class_id) in item.equippable.iter().enumerate() {
                            if class_id == "all" {
                                continue;
                            }
                            self.checked(
                                "class",
                                &index.classes,
                                class_id,
                                &file.path,
                                format!("{base}.equippable[{class_index}]"),
                            );
                        }
                        if let AccessoryStats::AbilityBlock(stats) = &item.stats {
                            self.checked(
                                "enemy ability",
                                &index.enemy_abilities,
                                &stats.blocks_ability,
                                &file.path,
                                format!("{base}.stats.blocks_ability"),
                            );
                        }
                    }
                    _ => {}
                }
            }
        }
        if let Some(field_use) = self.catalogs.field_use.clone() {
            for (item_index, item) in field_use.value.entries().iter().enumerate() {
                self.checked(
                    "item",
                    &index.items,
                    item.id(),
                    &field_use.path,
                    format!("[{item_index}].id"),
                );
            }
        }
    }

    fn validate_maps(&mut self, index: &ReferenceIndex) {
        let maps = self.catalogs.maps.clone();
        for file in maps {
            if let Some(bgm) = &file.value.bgm {
                self.checked("BGM", &index.bgm, bgm, &file.path, "bgm");
            }
            let tmx_path = format!("{}/{}.tmx", index.tmx_root, file.stem);
            self.report.checked_references += 1;
            let tmx_missing = ScenarioRelativePath::try_from(tmx_path.as_str()).is_ok_and(|path| {
                probe_path(self.physical_root, self.canonical_root.as_deref(), &path)
                    != ScenarioPathProbeResult::File
            });
            if tmx_missing {
                self.warning(
                    "source.unmatched_map_metadata",
                    &file.path,
                    "$same_stem_tmx",
                    format!("no same-stem TMX file exists at `{tmx_path}`"),
                );
            }
            for (section, shop) in [
                ("shop", file.value.shop.as_ref()),
                ("weapon_shop", file.value.weapon_shop.as_ref()),
                ("armor_shop", file.value.armor_shop.as_ref()),
            ] {
                if let Some(shop) = shop {
                    let (namespace, known) = match section {
                        "shop" => ("consumable item", &index.consumables),
                        "weapon_shop" => ("weapon", &index.weapons),
                        "armor_shop" => ("armor equipment", &index.armor_equipment),
                        _ => unreachable!("the map schema has exactly three shop sections"),
                    };
                    for (item_index, item) in shop.items.iter().enumerate() {
                        self.checked(
                            namespace,
                            known,
                            item.id(),
                            &file.path,
                            format!("{section}.items[{item_index}].id"),
                        );
                    }
                }
            }
            for (npc_index, npc) in file.value.npcs.iter().enumerate() {
                self.checked(
                    "dialogue",
                    &index.dialogue,
                    npc.effective_dialogue_id(),
                    &file.path,
                    format!("npcs[{npc_index}].dialogue"),
                );
                if let Some(excuses) = &npc.excuses {
                    self.checked(
                        "dialogue",
                        &index.dialogue,
                        excuses,
                        &file.path,
                        format!("npcs[{npc_index}].excuses"),
                    );
                }
                if let Some(sprite) = &npc.sprite {
                    self.checked_path(
                        &file.path,
                        format!("npcs[{npc_index}].sprite"),
                        sprite.as_str(),
                    );
                }
            }
            for (box_index, item_box) in file.value.item_boxes.iter().enumerate() {
                if item_box.loot.items.is_empty() && item_box.loot.magic_cores.is_empty() {
                    self.warning(
                        "content.empty_loot",
                        &file.path,
                        format!("item_boxes[{box_index}].loot"),
                        format!("item box `{}` has empty loot", item_box.id),
                    );
                }
                for (item_index, item) in item_box.loot.items.iter().enumerate() {
                    self.checked(
                        "item",
                        &index.items,
                        &item.id,
                        &file.path,
                        format!("item_boxes[{box_index}].loot.items[{item_index}].id"),
                    );
                }
            }
        }
    }

    fn validate_dialogue(&mut self, index: &ReferenceIndex) {
        let documents = self.catalogs.dialogue.clone();
        for file in documents {
            match &file.value {
                DialogueDocument::Cutscene(document) => self.validate_dialogue_actions(
                    &document.on_complete,
                    &file.path,
                    "on_complete",
                    index,
                ),
                DialogueDocument::Entries(document) => {
                    for (entry_index, entry) in document.entries.iter().enumerate() {
                        self.validate_dialogue_actions(
                            &entry.on_complete,
                            &file.path,
                            &format!("entries[{entry_index}].on_complete"),
                            index,
                        );
                    }
                }
                DialogueDocument::LinePool(_) => {}
            }
        }
    }

    fn validate_dialogue_actions(
        &mut self,
        actions: &DialogueActions,
        path: &str,
        base: &str,
        index: &ReferenceIndex,
    ) {
        for (item_index, item) in actions.give_items.iter().enumerate() {
            self.checked(
                "item",
                &index.items,
                &item.id,
                path,
                format!("{base}.give_items[{item_index}].id"),
            );
        }
        if let Some(member) = &actions.join_party {
            self.checked(
                "party member",
                &index.party,
                member,
                path,
                format!("{base}.join_party"),
            );
        }
        if let Some(transition) = &actions.transition {
            self.checked(
                "map",
                &index.maps,
                &transition.map,
                path,
                format!("{base}.transition.map"),
            );
        }
    }

    fn validate_enemies(&mut self, index: &ReferenceIndex) {
        let enemy_files = self.catalogs.enemies.clone();
        for file in enemy_files {
            for (enemy_index, enemy) in file.value.entries().iter().enumerate() {
                let base = format!("documents[{enemy_index}]");
                if let Some(barrier) = &enemy.barrier {
                    self.checked(
                        "item",
                        &index.items,
                        &barrier.requires_item,
                        &file.path,
                        format!("{base}.requires_item"),
                    );
                }
                for (pool_index, pool) in enemy.drops.loot.iter().enumerate() {
                    for (item_index, item) in pool.pool.iter().enumerate() {
                        self.checked(
                            "item",
                            &index.items,
                            &item.item,
                            &file.path,
                            format!("{base}.drops.loot[{pool_index}].pool[{item_index}].item"),
                        );
                    }
                }
                match &enemy.behavior {
                    EnemyBehavior::Inline { ai, targeting } => self.validate_enemy_ai(
                        ai,
                        &targeting.overrides,
                        &file.path,
                        &format!("{base}.ai"),
                        index,
                    ),
                    EnemyBehavior::Referenced { ai_ref } => {
                        let referenced = format!("{}/{}", index.enemy_root, ai_ref.as_str());
                        self.checked_path(&file.path, format!("{base}.ai_ref"), &referenced);
                        self.checked(
                            "boss move set",
                            &index.boss_move_sets,
                            file_stem(ai_ref.as_str()),
                            &file.path,
                            format!("{base}.ai_ref"),
                        );
                    }
                }
                self.validate_enemy_sprite(&file.path, &base, enemy.sprite_id());
            }
        }
        let boss_files = self.catalogs.boss_move_sets.clone();
        for file in boss_files {
            self.validate_enemy_ai(
                &file.value.ai,
                &file.value.targeting.overrides,
                &file.path,
                "ai",
                index,
            );
        }
    }

    fn validate_enemy_ai(
        &mut self,
        ai: &EnemyAi,
        overrides: &[EnemyTargetOverride],
        path: &str,
        base: &str,
        index: &ReferenceIndex,
    ) {
        let local_abilities = ai
            .moves
            .iter()
            .filter_map(|move_| match move_ {
                EnemyMove::Ability { id, .. } => Some(id.as_str()),
                EnemyMove::Attack { .. } => None,
            })
            .collect::<BTreeSet<_>>();
        for (override_index, override_) in overrides.iter().enumerate() {
            let ability = match override_ {
                EnemyTargetOverride::Standard(value) => &value.ability,
                EnemyTargetOverride::AccessoryBlocked(value) => {
                    self.checked(
                        "accessory",
                        &index.accessories,
                        &value.blocked_by_accessory,
                        path,
                        format!(
                            "{base}.targeting.overrides[{override_index}].blocked_by_accessory"
                        ),
                    );
                    &value.ability
                }
            };
            self.report.checked_references += 1;
            if !local_abilities.contains(ability.as_str()) {
                self.error(
                    "reference.missing",
                    path,
                    format!("{base}.targeting.overrides[{override_index}].ability"),
                    format!(
                        "target override names ability `{ability}` absent from this AI move set"
                    ),
                );
            }
        }
    }

    fn validate_enemy_sprite(&mut self, owner_path: &str, base: &str, enemy_id: &str) {
        self.report.checked_references += 1;
        let ordinary = format!("assets/sprites/enemies/{enemy_id}.tsx");
        let battle = format!("assets/sprites/enemies/{enemy_id}_battle.tsx");
        let exists = [ordinary.as_str(), battle.as_str()]
            .iter()
            .any(|candidate| {
                ScenarioRelativePath::try_from(*candidate).is_ok_and(|path| {
                    probe_path(self.physical_root, self.canonical_root.as_deref(), &path)
                        == ScenarioPathProbeResult::File
                })
            });
        if !exists {
            self.error(
                "path.missing",
                owner_path,
                format!("{base}.$derived_sprite"),
                format!("enemy `{enemy_id}` has neither `{ordinary}` nor `{battle}`"),
            );
        }
    }

    fn validate_encounters(&mut self, index: &ReferenceIndex) {
        let encounters = self.catalogs.encounters.clone();
        for file in encounters {
            self.checked(
                "map",
                &index.maps,
                file.value.effective_id(&file.stem),
                &file.path,
                "id",
            );
            if !file.value.background.is_empty() {
                self.checked(
                    "battle background",
                    &index.backgrounds,
                    &file.value.background,
                    &file.path,
                    "background",
                );
            }
            for (formation_index, formation) in file.value.entries.iter().enumerate() {
                for (enemy_index, enemy) in formation.enemy_ids.iter().enumerate() {
                    self.checked(
                        "enemy",
                        &index.enemies,
                        enemy,
                        &file.path,
                        format!("entries[{formation_index}].formation[{enemy_index}]"),
                    );
                }
            }
            if let Some(boss) = &file.value.boss {
                self.checked(
                    "boss enemy",
                    &index.boss_enemies,
                    &boss.enemy_id,
                    &file.path,
                    "boss.id",
                );
            }
            for (barrier_index, barrier) in file.value.barrier_enemies.iter().enumerate() {
                self.checked(
                    "enemy",
                    &index.enemies,
                    &barrier.enemy_id,
                    &file.path,
                    format!("barrier_enemies[{barrier_index}].id"),
                );
                self.checked(
                    "item",
                    &index.items,
                    &barrier.requires_item,
                    &file.path,
                    format!("barrier_enemies[{barrier_index}].requires_item"),
                );
            }
        }
        let encounter_ids = self
            .catalogs
            .encounters
            .iter()
            .map(|file| file.value.effective_id(&file.stem).to_owned())
            .collect::<BTreeSet<_>>();
        let maps = self.catalogs.maps.clone();
        for file in maps {
            if file.value.enemy_spawn.is_some()
                && index.maps.contains(file.value.effective_id(&file.stem))
            {
                self.checked(
                    "encounter",
                    &encounter_ids,
                    file.value.effective_id(&file.stem),
                    &file.path,
                    "enemy_spawn",
                );
            }
        }
        if let Some(backgrounds) = self.catalogs.backgrounds.clone() {
            for (background_index, background) in backgrounds.value.0.iter().enumerate() {
                let asset = format!("assets/images/battle_bg/{}.webp", background.id);
                self.checked_path(
                    &backgrounds.path,
                    format!("[{background_index}].$derived_image"),
                    &asset,
                );
            }
        }
    }

    fn validate_recipes(&mut self, index: &ReferenceIndex) {
        let files = self.catalogs.recipes.clone();
        for file in files {
            for (recipe_index, recipe) in file.value.entries().iter().enumerate() {
                self.checked(
                    "item",
                    &index.items,
                    &recipe.output.item,
                    &file.path,
                    format!("[{recipe_index}].output.item"),
                );
                for (item_index, item) in recipe.inputs.items.iter().enumerate() {
                    self.checked(
                        "item",
                        &index.items,
                        &item.id,
                        &file.path,
                        format!("[{recipe_index}].inputs.items[{item_index}].id"),
                    );
                }
            }
        }
    }

    fn validate_audio(&mut self, index: &ReferenceIndex) {
        if let Some(bgm) = self.catalogs.bgm.clone() {
            for (category_index, category) in bgm.value.categories.iter().enumerate() {
                for (entry_index, entry) in category.entries.iter().enumerate() {
                    let asset = format!("{AUDIO_ASSET_ROOT}/{}", entry.path);
                    self.checked_path(
                        &bgm.path,
                        format!("categories[{category_index}].entries[{entry_index}]"),
                        &asset,
                    );
                }
            }
        }
        if let Some(sfx) = self.catalogs.sfx.clone() {
            for (category_index, category) in sfx.value.categories.iter().enumerate() {
                for (entry_index, entry) in category.entries.iter().enumerate() {
                    let asset = format!("{AUDIO_ASSET_ROOT}/{}", entry.path);
                    self.checked_path(
                        &sfx.path,
                        format!("categories[{category_index}].entries[{entry_index}]"),
                        &asset,
                    );
                }
            }
        }
        for key in ["title.default", "battle.normal", "battle.boss"] {
            self.checked(
                "BGM",
                &index.bgm,
                key,
                BGM_INDEX_PATH,
                format!("$runtime.{key}"),
            );
        }
        for key in [
            "confirm",
            "cancel",
            "hover",
            "atk_impact",
            "party_hit",
            "enemy_death",
            "flee",
            "denied",
            "encounter",
            "atk_slash",
            "defend",
            "use_item",
            "heal",
            "revive",
            "debuff",
            "atk_buff",
            "def_buff",
        ] {
            self.checked(
                "SFX",
                &index.sfx,
                key,
                SFX_INDEX_PATH,
                format!("$runtime.{key}"),
            );
        }
        let classes = self.catalogs.classes.clone();
        for file in classes {
            for (ability_index, ability) in file.value.abilities.iter().enumerate() {
                if let AbilityKind::Spell(spell) = &ability.kind {
                    let key = format!("spell_{}", ability_element_id(spell.element));
                    self.checked(
                        "SFX",
                        &index.sfx,
                        &key,
                        &file.path,
                        format!("abilities[{ability_index}].element"),
                    );
                }
            }
        }
    }

    fn validate_flags(&mut self, _index: &ReferenceIndex) {
        // Implemented after all structural checks by a pure inventory walk, so producers and
        // consumers are order-independent and diagnostics point at the first stable location.
        let (defined, consumed, engine_managed) = collect_flag_edges(&self.catalogs);
        for (flag, locations) in &consumed {
            self.report.checked_references += locations.len();
            if !defined.contains_key(flag) {
                for location in locations {
                    self.report.diagnostics.push(ScenarioDiagnostic {
                        severity: DiagnosticSeverity::Error,
                        code: "flag.undefined",
                        location: location.clone(),
                        message: format!("flag `{flag}` is consumed but never produced"),
                    });
                }
            }
        }
        for (flag, locations) in &defined {
            if !consumed.contains_key(flag) && !engine_managed.contains(flag) {
                self.report.diagnostics.push(ScenarioDiagnostic {
                    severity: DiagnosticSeverity::Warning,
                    code: "flag.orphan",
                    location: locations[0].clone(),
                    message: format!("flag `{flag}` is produced but never consumed"),
                });
            }
        }
    }

    fn validate_duplicates(&mut self, index: &ReferenceIndex) {
        for duplicate in find_duplicate_ids(index.catalog_ids.clone()) {
            self.error(
                "id.duplicate",
                duplicate.duplicate.path().as_str(),
                duplicate.duplicate.field_path(),
                duplicate.to_string(),
            );
        }
    }
}

#[derive(Default)]
struct ReferenceIndex {
    party: BTreeSet<String>,
    protagonists: BTreeSet<String>,
    classes: BTreeSet<String>,
    class_abilities: BTreeMap<String, BTreeSet<String>>,
    items: BTreeSet<String>,
    consumables: BTreeSet<String>,
    weapons: BTreeSet<String>,
    shields: BTreeSet<String>,
    helmets: BTreeSet<String>,
    bodies: BTreeSet<String>,
    accessories: BTreeSet<String>,
    armor_equipment: BTreeSet<String>,
    maps: BTreeSet<String>,
    npcs: BTreeMap<String, BTreeSet<String>>,
    dialogue: BTreeSet<String>,
    enemies: BTreeSet<String>,
    boss_enemies: BTreeSet<String>,
    enemy_abilities: BTreeSet<String>,
    boss_move_sets: BTreeSet<String>,
    backgrounds: BTreeSet<String>,
    bgm: BTreeSet<String>,
    sfx: BTreeSet<String>,
    enemy_root: String,
    tmx_root: String,
    catalog_ids: Vec<CatalogId>,
}

impl ReferenceIndex {
    fn from_catalogs(catalogs: &ScenarioCatalogs) -> Self {
        let mut index = Self::default();
        if let Some(manifest) = &catalogs.manifest {
            index.enemy_root = manifest.refs.enemies.as_str().to_owned();
            index.tmx_root = manifest.refs.tmx.as_str().to_owned();
        }
        if let Some(file) = &catalogs.party {
            for (position, member) in file.value.party.iter().enumerate() {
                add_id(
                    &mut index.party,
                    &mut index.catalog_ids,
                    CatalogNamespace::Party,
                    "",
                    &member.data().id,
                    &file.path,
                    format!("party[{position}].id"),
                );
                if member.is_protagonist() {
                    index.protagonists.insert(member.data().id.clone());
                }
            }
        }
        for file in &catalogs.classes {
            add_id(
                &mut index.classes,
                &mut index.catalog_ids,
                CatalogNamespace::Classes,
                "",
                &file.value.class_id,
                &file.path,
                "class",
            );
            for (position, ability) in file.value.abilities.iter().enumerate() {
                index
                    .class_abilities
                    .entry(file.value.class_id.clone())
                    .or_default()
                    .insert(ability.id.clone());
                add_catalog_id(
                    &mut index.catalog_ids,
                    CatalogNamespace::Abilities,
                    "",
                    &ability.id,
                    &file.path,
                    format!("abilities[{position}].id"),
                );
            }
        }
        for file in &catalogs.items {
            for (position, item) in file.value.entries().iter().enumerate() {
                add_id(
                    &mut index.items,
                    &mut index.catalog_ids,
                    CatalogNamespace::Items,
                    "",
                    item.id(),
                    &file.path,
                    format!("[{position}].id"),
                );
                match item {
                    ItemDefinition::Consumable(item) => {
                        index.consumables.insert(item.id.clone());
                    }
                    ItemDefinition::Weapon(item) => {
                        index.weapons.insert(item.id.clone());
                    }
                    ItemDefinition::Shield(item) => {
                        index.shields.insert(item.id.clone());
                        index.armor_equipment.insert(item.id.clone());
                    }
                    ItemDefinition::Helmet(item) => {
                        index.helmets.insert(item.id.clone());
                        index.armor_equipment.insert(item.id.clone());
                    }
                    ItemDefinition::Body(item) => {
                        index.bodies.insert(item.id.clone());
                        index.armor_equipment.insert(item.id.clone());
                    }
                    ItemDefinition::Accessory(item) => {
                        index.accessories.insert(item.id.clone());
                        index.armor_equipment.insert(item.id.clone());
                    }
                    ItemDefinition::Material(_)
                    | ItemDefinition::Key(_)
                    | ItemDefinition::MagicCore(_) => {}
                }
            }
        }
        if let Some(file) = &catalogs.field_use {
            for (position, item) in file.value.entries().iter().enumerate() {
                add_catalog_id(
                    &mut index.catalog_ids,
                    CatalogNamespace::FieldUseItems,
                    "",
                    item.id(),
                    &file.path,
                    format!("[{position}].id"),
                );
            }
        }
        for file in &catalogs.maps {
            let id = file.value.effective_id(&file.stem);
            add_catalog_id(
                &mut index.catalog_ids,
                CatalogNamespace::Maps,
                "",
                id,
                &file.path,
                "id",
            );
            // Runtime map identity comes from the TMX stem. Keep the authored metadata id in
            // duplicate diagnostics above, but scope children to the map they can inhabit.
            let scope = file.stem.clone();
            for (position, npc) in file.value.npcs.iter().enumerate() {
                index
                    .npcs
                    .entry(scope.clone())
                    .or_default()
                    .insert(npc.id.clone());
                add_catalog_id(
                    &mut index.catalog_ids,
                    CatalogNamespace::Npcs,
                    &scope,
                    &npc.id,
                    &file.path,
                    format!("npcs[{position}].id"),
                );
            }
            for (position, item_box) in file.value.item_boxes.iter().enumerate() {
                add_catalog_id(
                    &mut index.catalog_ids,
                    CatalogNamespace::ItemBoxes,
                    &scope,
                    &item_box.id,
                    &file.path,
                    format!("item_boxes[{position}].id"),
                );
            }
        }
        for file in &catalogs.dialogue {
            let id = file.value.effective_id(&file.stem);
            add_id(
                &mut index.dialogue,
                &mut index.catalog_ids,
                CatalogNamespace::Dialogue,
                "",
                id,
                &file.path,
                "id",
            );
        }
        for file in &catalogs.enemies {
            for (position, enemy) in file.value.entries().iter().enumerate() {
                add_id(
                    &mut index.enemies,
                    &mut index.catalog_ids,
                    CatalogNamespace::Enemies,
                    "",
                    &enemy.id,
                    &file.path,
                    format!("documents[{position}].id"),
                );
                if enemy.boss {
                    index.boss_enemies.insert(enemy.id.clone());
                }
                if let EnemyBehavior::Inline { ai, .. } = &enemy.behavior {
                    collect_enemy_abilities(ai, &mut index.enemy_abilities);
                }
            }
        }
        for file in &catalogs.boss_move_sets {
            add_id(
                &mut index.boss_move_sets,
                &mut index.catalog_ids,
                CatalogNamespace::BossMoveSets,
                "",
                &file.stem,
                &file.path,
                "$filename_stem",
            );
            collect_enemy_abilities(&file.value.ai, &mut index.enemy_abilities);
        }
        for file in &catalogs.encounters {
            add_catalog_id(
                &mut index.catalog_ids,
                CatalogNamespace::Encounters,
                "",
                file.value.effective_id(&file.stem),
                &file.path,
                "id",
            );
        }
        if let Some(file) = &catalogs.backgrounds {
            for (position, background) in file.value.0.iter().enumerate() {
                add_id(
                    &mut index.backgrounds,
                    &mut index.catalog_ids,
                    CatalogNamespace::BattleBackgrounds,
                    "",
                    &background.id,
                    &file.path,
                    format!("[{position}].id"),
                );
            }
        }
        for file in &catalogs.recipes {
            for (position, recipe) in file.value.entries().iter().enumerate() {
                add_catalog_id(
                    &mut index.catalog_ids,
                    CatalogNamespace::Recipes,
                    "",
                    &recipe.id,
                    &file.path,
                    format!("[{position}].id"),
                );
            }
        }
        if let Some(file) = &catalogs.quests {
            for (position, quest) in file.value.entries().iter().enumerate() {
                add_catalog_id(
                    &mut index.catalog_ids,
                    CatalogNamespace::Quests,
                    "",
                    &quest.id,
                    &file.path,
                    format!("[{position}].id"),
                );
            }
        }
        if let Some(file) = &catalogs.bgm {
            for (category_position, category) in file.value.categories.iter().enumerate() {
                for (entry_position, entry) in category.entries.iter().enumerate() {
                    let id = format!("{}.{}", category.id, entry.id);
                    add_id(
                        &mut index.bgm,
                        &mut index.catalog_ids,
                        CatalogNamespace::Bgm,
                        "",
                        &id,
                        &file.path,
                        format!("categories[{category_position}].entries[{entry_position}]"),
                    );
                }
            }
        }
        if let Some(file) = &catalogs.sfx {
            for (category_position, category) in file.value.categories.iter().enumerate() {
                for (entry_position, entry) in category.entries.iter().enumerate() {
                    index.sfx.insert(entry.id.clone());
                    add_catalog_id(
                        &mut index.catalog_ids,
                        CatalogNamespace::Sfx,
                        "",
                        &entry.id,
                        &file.path,
                        format!("categories[{category_position}].entries[{entry_position}]"),
                    );
                }
            }
        }
        index
    }
}

fn collect_enemy_abilities(ai: &EnemyAi, target: &mut BTreeSet<String>) {
    for move_ in &ai.moves {
        if let EnemyMove::Ability { id, .. } = move_ {
            target.insert(id.clone());
        }
    }
}

fn ability_element_id(element: AbilityElement) -> &'static str {
    match element {
        AbilityElement::Fire => "fire",
        AbilityElement::Water => "water",
        AbilityElement::Wind => "wind",
        AbilityElement::Earth => "earth",
        AbilityElement::Holy => "holy",
    }
}

fn add_id(
    set: &mut BTreeSet<String>,
    ids: &mut Vec<CatalogId>,
    namespace: CatalogNamespace,
    scope: &str,
    id: &str,
    path: &str,
    field: impl Into<String>,
) {
    set.insert(id.to_owned());
    add_catalog_id(ids, namespace, scope, id, path, field);
}

fn add_catalog_id(
    ids: &mut Vec<CatalogId>,
    namespace: CatalogNamespace,
    scope: &str,
    id: &str,
    path: &str,
    field: impl Into<String>,
) {
    let location = CatalogIdLocation::new(path, field.into())
        .expect("loaded paths and fields are valid diagnostic locations");
    ids.push(if scope.is_empty() {
        CatalogId::scenario_wide(namespace, id, location)
    } else {
        CatalogId::scoped(namespace, scope, id, location)
    });
}

fn located<T>(path: &str, value: T) -> Located<T> {
    Located {
        path: path.to_owned(),
        stem: file_stem(path).to_owned(),
        value,
    }
}

fn file_stem(path: &str) -> &str {
    Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(path)
}

fn probe_path(
    root: &Path,
    canonical_root: Option<&Path>,
    path: &ScenarioRelativePath,
) -> ScenarioPathProbeResult {
    let candidate = root.join(path.as_str());
    let Ok(metadata) = fs::metadata(&candidate) else {
        return ScenarioPathProbeResult::Missing;
    };
    if let (Some(canonical_root), Ok(canonical_candidate)) =
        (canonical_root, candidate.canonicalize())
        && !canonical_candidate.starts_with(canonical_root)
    {
        return ScenarioPathProbeResult::Missing;
    }
    if metadata.is_file() {
        ScenarioPathProbeResult::File
    } else if metadata.is_dir() {
        ScenarioPathProbeResult::Directory
    } else {
        ScenarioPathProbeResult::Missing
    }
}

fn collect_yaml_files(
    root: &Path,
    canonical_root: Option<&Path>,
    directory: &Path,
    visited_directories: &mut BTreeSet<PathBuf>,
    target: &mut Vec<String>,
) -> io::Result<()> {
    let canonical_root = canonical_root.ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "scenario root cannot be resolved")
    })?;
    let canonical_directory = directory.canonicalize().map_err(|_| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "scenario catalog directory cannot be resolved",
        )
    })?;
    if !canonical_directory.starts_with(canonical_root) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "scenario catalog directory resolves outside the scenario root",
        ));
    }
    if !canonical_directory.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "scenario catalog path is not a directory",
        ));
    }
    if !visited_directories.insert(canonical_directory) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "scenario catalog directory cycle or repeated alias detected",
        ));
    }
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = fs::metadata(&path)?;
        let canonical_path = path.canonicalize().map_err(|_| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "scenario catalog entry cannot be resolved",
            )
        })?;
        if !canonical_path.starts_with(canonical_root) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "scenario catalog entry resolves outside the scenario root",
            ));
        }
        if metadata.is_dir() {
            collect_yaml_files(
                root,
                Some(canonical_root),
                &path,
                visited_directories,
                target,
            )?;
        } else if metadata.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension == "yaml")
            && let Ok(relative) = path.strip_prefix(root)
        {
            target.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(())
}

type FlagLocations = BTreeMap<String, Vec<ScenarioLocation>>;

fn collect_flag_edges(
    catalogs: &ScenarioCatalogs,
) -> (FlagLocations, FlagLocations, BTreeSet<String>) {
    let mut defined = FlagLocations::new();
    let mut consumed = FlagLocations::new();
    let mut engine_managed = BTreeSet::new();
    let add = |map: &mut FlagLocations, flag: &str, path: &str, field: String| {
        if !flag.is_empty() {
            map.entry(flag.to_owned())
                .or_default()
                .push(ScenarioLocation::new(path, field));
        }
    };
    if let Some(manifest) = &catalogs.manifest {
        for (index, flag) in manifest.bootstrap_flags.iter().enumerate() {
            add(
                &mut defined,
                flag,
                SCENARIO_MANIFEST_PATH,
                format!("bootstrap_flags[{index}]"),
            );
        }
        for (index, flag) in manifest.engine_managed_flags.iter().enumerate() {
            add(
                &mut defined,
                flag,
                SCENARIO_MANIFEST_PATH,
                format!("engine_managed_flags[{index}]"),
            );
            engine_managed.insert(flag.clone());
        }
    }
    if let Some(file) = &catalogs.party {
        for (index, member) in file.value.party.iter().enumerate() {
            add(
                &mut consumed,
                &member.data().join.condition,
                &file.path,
                format!("party[{index}].join.condition"),
            );
            if let Some(recruit) = member.recruit() {
                add(
                    &mut consumed,
                    &recruit.joined_flag,
                    &file.path,
                    format!("party[{index}].recruit.joined_flag"),
                );
            }
        }
    }
    for file in &catalogs.classes {
        for (index, ability) in file.value.abilities.iter().enumerate() {
            if let Some(flag) = &ability.unlock_flag {
                add(
                    &mut consumed,
                    flag,
                    &file.path,
                    format!("abilities[{index}].unlock_flag"),
                );
            }
        }
    }
    for file in &catalogs.items {
        for (index, item) in file.value.entries().iter().enumerate() {
            if let ItemDefinition::Key(item) = item
                && let crate::scenario_item::KeyItemEffect::Unlock(effect) = &item.effect
            {
                add(
                    &mut defined,
                    &effect.unlock_flag,
                    &file.path,
                    format!("[{index}].effect.unlock_flag"),
                );
            }
        }
    }
    for file in &catalogs.maps {
        for (i, npc) in file.value.npcs.iter().enumerate() {
            add_conditions(
                &mut consumed,
                &npc.present,
                &file.path,
                format!("npcs[{i}].present"),
            );
        }
        for (i, b) in file.value.item_boxes.iter().enumerate() {
            add_conditions(
                &mut consumed,
                &b.present,
                &file.path,
                format!("item_boxes[{i}].present"),
            );
        }
        for (section, shop) in [
            ("shop", file.value.shop.as_ref()),
            ("weapon_shop", file.value.weapon_shop.as_ref()),
            ("armor_shop", file.value.armor_shop.as_ref()),
        ] {
            if let Some(shop) = shop {
                for (i, item) in shop.items.iter().enumerate() {
                    add(
                        &mut consumed,
                        item.unlock_flag(),
                        &file.path,
                        format!("{section}.items[{i}].unlock_flag"),
                    );
                }
            }
        }
        if let Some(t) = &file.value.transport {
            for (mode, data) in [("sail", &t.sail), ("fly", &t.fly), ("warp", &t.warp)] {
                add(
                    &mut consumed,
                    &data.unlock_flag,
                    &file.path,
                    format!("transport.{mode}.unlock_flag"),
                );
            }
        }
    }
    for file in &catalogs.dialogue {
        match &file.value {
            DialogueDocument::Cutscene(d) => {
                add_actions(&mut defined, &d.on_complete, &file.path, "on_complete")
            }
            DialogueDocument::Entries(d) => {
                for (i, e) in d.entries.iter().enumerate() {
                    add_conditions(
                        &mut consumed,
                        &e.condition,
                        &file.path,
                        format!("entries[{i}].condition"),
                    );
                    add_actions(
                        &mut defined,
                        &e.on_complete,
                        &file.path,
                        &format!("entries[{i}].on_complete"),
                    );
                }
            }
            DialogueDocument::LinePool(_) => {}
        }
    }
    for file in &catalogs.encounters {
        if let Some(b) = &file.value.boss {
            add(
                &mut defined,
                &b.completion.set_flag,
                &file.path,
                "boss.on_complete.set_flag".to_owned(),
            );
        }
    }
    for file in &catalogs.recipes {
        for (i, r) in file.value.entries().iter().enumerate() {
            if let Some(flag) = &r.unlock_flag {
                add(
                    &mut consumed,
                    flag,
                    &file.path,
                    format!("[{i}].unlock_flag"),
                );
            }
        }
    }
    if let Some(file) = &catalogs.quests {
        for (i, q) in file.value.entries().iter().enumerate() {
            add(
                &mut consumed,
                &q.started_flag,
                &file.path,
                format!("[{i}].started_flag"),
            );
            add(
                &mut consumed,
                &q.completed_flag,
                &file.path,
                format!("[{i}].completed_flag"),
            );
        }
    }
    (defined, consumed, engine_managed)
}

fn add_conditions(target: &mut FlagLocations, c: &FlagConditions, path: &str, base: String) {
    for (i, f) in c.requires.iter().enumerate() {
        target
            .entry(f.clone())
            .or_default()
            .push(ScenarioLocation::new(path, format!("{base}.requires[{i}]")));
    }
    for (i, f) in c.excludes.iter().enumerate() {
        target
            .entry(f.clone())
            .or_default()
            .push(ScenarioLocation::new(path, format!("{base}.excludes[{i}]")));
    }
}
fn add_actions(target: &mut FlagLocations, a: &DialogueActions, path: &str, base: &str) {
    if let Some(flags) = &a.set_flag {
        for (i, f) in flags.as_slice().iter().enumerate() {
            target
                .entry(f.clone())
                .or_default()
                .push(ScenarioLocation::new(path, format!("{base}.set_flag[{i}]")));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

    struct InventedScenario(PathBuf);

    impl InventedScenario {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should follow the Unix epoch")
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "rpg-s1-cross-reference-{}-{nonce}-{}",
                std::process::id(),
                NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&root).expect("temporary scenario root should be creatable");
            let scenario = Self(root);
            scenario.populate();
            scenario
        }

        fn write(&self, relative: &str, contents: &str) {
            let path = self.0.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("temporary scenario parent should be creatable");
            }
            fs::write(path, contents).expect("temporary scenario file should be writable");
        }

        fn touch(&self, relative: &str) {
            self.write(relative, "invented fixture asset\n");
        }

        fn populate(&self) {
            self.write(
                "manifest.yaml",
                r#"id: invented_story
name: Invented Story
version: "1.0"
window_title: Invented Window
title:
  image: assets/title.webp
  cursor_icon: assets/cursor.webp
font:
  path: assets/font.ttf
ui:
  menu_backdrop: assets/backdrop.webp
apothecary:
  sprite: assets/apothecary.tsx
  icons:
    locked: assets/locked.webp
    ready: assets/ready.webp
    missing: assets/missing.webp
inn: {sprite: assets/inn.tsx}
item_shop: {sprite: assets/item_shop.tsx}
weapon_shop: {sprite: assets/weapon_shop.tsx}
armor_shop: {sprite: assets/armor_shop.tsx}
item_box: {sprite: assets/item_box.tsx}
protagonist:
  id: maker
  name: Maker
  class: maker
  sprite: assets/maker.tsx
start:
  map: village
  position: [1, 2]
  intro_dialogue: data/dialogue/intro.yaml
bootstrap_flags: [quest_started, quest_done]
engine_managed_flags: []
refs:
  party: data/party.yaml
  classes: data/classes/
  maps: data/maps/
  dialogue: data/dialogue/
  items: data/items/
  enemies: data/enemies/
  encount: data/encount/
  recipe: data/recipe/
  quests: data/quests.yaml
  balance: data/balance.yaml
  battle_backgrounds: data/battle_backgrounds.yaml
  assets: assets/
  tmx: assets/maps/
"#,
            );
            self.write(
                "data/party.yaml",
                r#"party:
  - id: maker
    name: Maker
    class: maker
    protagonist: true
    portrait: assets/maker_portrait.webp
    join: {condition: quest_started, map: village, position: [1, 2]}
    row: front
    age: 20
    gender: female
    level: 1
    exp: 0
    hp: 10
    hp_max: 10
    mp: 4
    mp_max: 4
    stats: {str: 2, dex: 2, con: 2, int: 2}
    equipped: {weapon: "", shield: "", helmet: "", body: "", accessory: ""}
    abilities_unlocked: []
    status_effects: []
"#,
            );
            self.write(
                "data/classes/maker.yaml",
                r#"class: maker
name: Maker
description: An invented class.
base_hp: 10
base_mp: 4
default_row: front
stat_growth:
  str: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
  dex: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
  con: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
  int: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
exp_curve: quadratic
exp_base: 10
exp_factor: 1.0
equipment_slots:
  weapon: [all]
  shield: [all]
  helmet: [all]
  body: [all]
  accessory: [all]
abilities: []
"#,
            );
            self.write(
                "data/items/materials.yaml",
                r#"- id: tonic
  name: Tonic
  type: material
  sell_price: 1
  description: An invented material.
"#,
            );
            self.write("data/items/field_use.yaml", "[]\n");
            self.write("data/maps/village.yaml", "name: Invented Village\n");
            self.write(
                "data/dialogue/intro.yaml",
                "id: intro\ntype: cutscene\nlines: [An invented beginning.]\n",
            );
            fs::create_dir_all(self.0.join("data/enemies"))
                .expect("enemy directory should be creatable");
            fs::create_dir_all(self.0.join("data/encount"))
                .expect("encounter directory should be creatable");
            self.write("data/recipe/recipes.yaml", "[]\n");
            self.write(
                "data/quests.yaml",
                r#"- id: invented_quest
  name: Invented Quest
  type: main
  location: Invented Village
  description: Finish the invented fixture.
  started_flag: quest_started
  completed_flag: quest_done
"#,
            );
            self.write(
                "data/balance.yaml",
                r#"progression: {level_cap: 10, exp_cap: 1000}
economy: {gp_cap: 1000, item_qty_cap: 10, max_tags_per_item: 2}
battle: {flee_base_chance: 0.3, flee_rogue_dex_bonus: 0.02}
spawner: {rogue_chase_reduction: 1, stealth_cloak_reduction: 1, lure_charm_interval_mult: 0.5}
movement: {player_speed: 5}
"#,
            );
            self.write("data/battle_backgrounds.yaml", "[]\n");
            self.write(
                BGM_INDEX_PATH,
                "title: {default: title.ogg}\nbattle: {normal: battle.ogg, boss: boss.ogg}\n",
            );
            self.write(
                SFX_INDEX_PATH,
                r#"runtime:
  confirm: confirm.ogg
  cancel: cancel.ogg
  hover: hover.ogg
  atk_impact: atk_impact.ogg
  party_hit: party_hit.ogg
  enemy_death: enemy_death.ogg
  flee: flee.ogg
  denied: denied.ogg
  encounter: encounter.ogg
  atk_slash: atk_slash.ogg
  defend: defend.ogg
  use_item: use_item.ogg
  heal: heal.ogg
  revive: revive.ogg
  debuff: debuff.ogg
  atk_buff: atk_buff.ogg
  def_buff: def_buff.ogg
"#,
            );
            for asset in [
                "assets/title.webp",
                "assets/cursor.webp",
                "assets/font.ttf",
                "assets/backdrop.webp",
                "assets/apothecary.tsx",
                "assets/locked.webp",
                "assets/ready.webp",
                "assets/missing.webp",
                "assets/inn.tsx",
                "assets/item_shop.tsx",
                "assets/weapon_shop.tsx",
                "assets/armor_shop.tsx",
                "assets/item_box.tsx",
                "assets/maker.tsx",
                "assets/maker_portrait.webp",
                "assets/maps/village.tmx",
                "assets/audio/title.ogg",
                "assets/audio/battle.ogg",
                "assets/audio/boss.ogg",
            ] {
                self.touch(asset);
            }
            for key in [
                "confirm",
                "cancel",
                "hover",
                "atk_impact",
                "party_hit",
                "enemy_death",
                "flee",
                "denied",
                "encounter",
                "atk_slash",
                "defend",
                "use_item",
                "heal",
                "revive",
                "debuff",
                "atk_buff",
                "def_buff",
            ] {
                self.touch(&format!("assets/audio/{key}.ogg"));
            }
        }
    }

    impl Drop for InventedScenario {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("temporary scenario should be removable");
        }
    }

    #[test]
    fn stable_locations_do_not_leak_host_paths() {
        let location = ScenarioLocation::new("data/maps/example.yaml", "npcs[1].dialogue");
        assert_eq!(
            location.to_string(),
            "data/maps/example.yaml#npcs[1].dialogue"
        );
        assert!(!location.to_string().starts_with('/'));
    }

    #[test]
    fn missing_namespace_reference_and_warning_are_distinct() {
        let mut report = ScenarioValidationReport::default();
        report.diagnostics.push(ScenarioDiagnostic {
            severity: DiagnosticSeverity::Error,
            code: "reference.missing",
            location: ScenarioLocation::new("data/maps/example.yaml", "npcs[0].dialogue"),
            message: "unknown dialogue id `missing`".to_owned(),
        });
        report.diagnostics.push(ScenarioDiagnostic {
            severity: DiagnosticSeverity::Warning,
            code: "content.empty_loot",
            location: ScenarioLocation::new("data/maps/example.yaml", "item_boxes[0].loot"),
            message: "item box `empty` has empty loot".to_owned(),
        });
        assert!(!report.is_valid());
        assert_eq!(report.errors().count(), 1);
        assert_eq!(report.warnings().count(), 1);
    }

    #[test]
    fn production_validator_accepts_a_compact_invented_scenario() {
        let fixture = InventedScenario::new();
        let report = validate_scenario_directory(&ScenarioRoot::default(), &fixture.0);

        assert!(report.is_valid(), "{:#?}", report.diagnostics);
        assert!(report.diagnostics.is_empty());
        assert_eq!(report.counts.party_members, 1);
        assert_eq!(report.counts.classes, 1);
        assert_eq!(report.counts.items, 1);
        assert_eq!(report.counts.maps, 1);
        assert_eq!(report.counts.dialogue_documents, 1);
        assert_eq!(report.counts.quests, 1);
        assert!(report.checked_references > 60);
    }

    #[test]
    fn production_validator_aggregates_wrong_namespace_flag_and_path_errors() {
        let fixture = InventedScenario::new();
        fixture.write(
            "data/party.yaml",
            &fs::read_to_string(fixture.0.join("data/party.yaml"))
                .unwrap()
                .replacen("class: maker", "class: tonic", 1),
        );
        fixture.write(
            "data/maps/village.yaml",
            r#"name: Invented Village
bgm: missing.track
npcs:
  - id: watcher
    name: Watcher
    dialogue: intro
    position: [2, 2]
    present: {requires: [missing_flag]}
"#,
        );
        fs::remove_file(fixture.0.join("assets/cursor.webp"))
            .expect("invented cursor should exist before removal");

        let report = validate_scenario_directory(&ScenarioRoot::default(), &fixture.0);
        let errors = report.errors().collect::<Vec<_>>();

        assert_eq!(errors.len(), 5, "{errors:#?}");
        assert!(
            errors
                .iter()
                .any(|finding| finding.message == "unknown class id `tonic`")
        );
        assert!(
            errors
                .iter()
                .any(|finding| finding.message == "unknown BGM id `missing.track`")
        );
        assert!(
            errors.iter().any(|finding| finding.code == "flag.undefined"
                && finding.message.contains("missing_flag"))
        );
        assert!(errors.iter().any(|finding| finding.code == "path.missing"
            && finding.location.field_path == "title.cursor_icon"));
        assert!(
            errors
                .iter()
                .any(|finding| finding.code == "reference.disagreement"
                    && finding.location.field_path == "protagonist.class")
        );
    }

    #[cfg(unix)]
    #[test]
    fn filesystem_probe_rejects_a_symlink_escape_without_leaking_its_target() {
        use std::os::unix::fs::symlink;

        let fixture = InventedScenario::new();
        let outside = fixture.0.with_extension("outside-cursor");
        fs::write(&outside, "outside fixture data\n").expect("outside fixture should be writable");
        fs::remove_file(fixture.0.join("assets/cursor.webp"))
            .expect("invented cursor should exist before replacement");
        symlink(&outside, fixture.0.join("assets/cursor.webp"))
            .expect("fixture symlink should be creatable");

        let report = validate_scenario_directory(&ScenarioRoot::default(), &fixture.0);
        let cursor = report
            .errors()
            .find(|finding| finding.location.field_path == "title.cursor_icon")
            .expect("escaping cursor symlink should be rejected");
        assert_eq!(cursor.code, "path.missing");
        assert!(
            !cursor
                .to_string()
                .contains(outside.to_string_lossy().as_ref())
        );

        fs::remove_file(outside).expect("outside fixture should be removable");
    }

    #[cfg(unix)]
    #[test]
    fn typed_catalog_read_rejects_a_symlink_escape_without_loading_content() {
        use std::os::unix::fs::symlink;

        let fixture = InventedScenario::new();
        let outside = fixture.0.with_extension("outside-party.yaml");
        fs::copy(fixture.0.join("data/party.yaml"), &outside)
            .expect("outside valid party catalog should be writable");
        fs::remove_file(fixture.0.join("data/party.yaml"))
            .expect("invented party catalog should exist before replacement");
        symlink(&outside, fixture.0.join("data/party.yaml"))
            .expect("fixture symlink should be creatable");

        let report = validate_scenario_directory(&ScenarioRoot::default(), &fixture.0);
        assert_eq!(report.counts.party_members, 0, "escaped data was loaded");
        let escape = report
            .errors()
            .find(|finding| {
                finding.code == "path.escape" && finding.location.path.as_str() == "data/party.yaml"
            })
            .expect("escaping typed catalog should be rejected before reading");
        assert_eq!(escape.location.field_path, "$");
        assert!(!report.diagnostics.iter().any(|finding| {
            finding
                .to_string()
                .contains(outside.to_string_lossy().as_ref())
        }));

        fs::remove_file(outside).expect("outside fixture should be removable");
    }

    #[cfg(unix)]
    #[test]
    fn recursive_catalog_walk_rejects_a_contained_symlink_cycle() {
        use std::os::unix::fs::symlink;

        let fixture = InventedScenario::new();
        symlink(
            fixture.0.join("data/maps"),
            fixture.0.join("data/maps/loop"),
        )
        .expect("fixture directory cycle should be creatable");

        let report = validate_scenario_directory(&ScenarioRoot::default(), &fixture.0);
        let cycle = report
            .errors()
            .find(|finding| {
                finding.code == "io.walk" && finding.location.path.as_str() == "data/maps"
            })
            .expect("contained directory cycles should terminate with a stable diagnostic");
        assert!(cycle.message.contains("cycle or repeated alias"));
        assert!(
            !cycle
                .to_string()
                .contains(fixture.0.to_string_lossy().as_ref())
        );
    }

    #[test]
    fn metadata_only_map_id_does_not_satisfy_a_runtime_map_reference() {
        let fixture = InventedScenario::new();
        fixture.write("data/maps/metadata_only.yaml", "name: Metadata Only\n");
        fixture.write(
            "data/dialogue/intro.yaml",
            r#"id: intro
type: cutscene
lines: [An invented beginning.]
on_complete:
  transition: {map: metadata_only, position: [3, 4], fade: in}
"#,
        );

        let report = validate_scenario_directory(&ScenarioRoot::default(), &fixture.0);
        assert!(report.errors().any(|finding| {
            finding.location.path.as_str() == "data/dialogue/intro.yaml"
                && finding.location.field_path == "on_complete.transition.map"
                && finding.message == "unknown map id `metadata_only`"
        }));
        assert!(report.warnings().any(|finding| {
            finding.code == "source.unmatched_map_metadata"
                && finding.location.path.as_str() == "data/maps/metadata_only.yaml"
        }));
    }

    #[test]
    fn tmx_only_map_id_satisfies_a_runtime_map_reference() {
        let fixture = InventedScenario::new();
        fixture.touch("assets/maps/tmx_only.tmx");
        fixture.write(
            "data/dialogue/intro.yaml",
            r#"id: intro
type: cutscene
lines: [An invented beginning.]
on_complete:
  transition: {map: tmx_only, position: [3, 4], fade: in}
"#,
        );

        let report = validate_scenario_directory(&ScenarioRoot::default(), &fixture.0);
        assert!(report.is_valid(), "{:#?}", report.diagnostics);
        assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
        assert_eq!(report.counts.maps, 1, "counts describe metadata files");
    }

    #[test]
    fn recruit_joined_flag_requires_an_actual_producer() {
        let fixture = InventedScenario::new();
        let mut party = fs::read_to_string(fixture.0.join("data/party.yaml"))
            .expect("invented party catalog should be readable");
        party.push_str(
            r#"  - id: scout
    name: Scout
    class: maker
    protagonist: false
    portrait: assets/scout_portrait.webp
    join: {condition: quest_started, map: village, position: [2, 2]}
    recruit: {npc: scout, dialogue: intro, joined_flag: scout_joined}
    row: back
    age: 21
    gender: male
    level: 1
    exp: 0
    hp: 10
    hp_max: 10
    mp: 4
    mp_max: 4
    stats: {str: 2, dex: 2, con: 2, int: 2}
    equipped: {weapon: "", shield: "", helmet: "", body: "", accessory: ""}
    abilities_unlocked: []
    status_effects: []
"#,
        );
        fixture.write("data/party.yaml", &party);
        fixture.write(
            "data/maps/village.yaml",
            r#"name: Invented Village
npcs:
  - id: scout
    name: Scout
    dialogue: intro
    position: [2, 2]
"#,
        );
        fixture.touch("assets/scout_portrait.webp");

        let report = validate_scenario_directory(&ScenarioRoot::default(), &fixture.0);
        let undefined = report
            .errors()
            .filter(|finding| finding.code == "flag.undefined")
            .collect::<Vec<_>>();
        assert_eq!(undefined.len(), 1, "{:#?}", report.diagnostics);
        assert_eq!(undefined[0].location.path.as_str(), "data/party.yaml");
        assert_eq!(
            undefined[0].location.field_path,
            "party[1].recruit.joined_flag"
        );
        assert!(undefined[0].message.contains("scout_joined"));
    }

    #[test]
    fn offensive_spell_requires_its_element_sfx_key() {
        let fixture = InventedScenario::new();
        let class = fs::read_to_string(fixture.0.join("data/classes/maker.yaml"))
            .expect("invented class catalog should be readable")
            .replace(
                "abilities: []",
                r#"abilities:
  - id: spark
    name: Spark
    unlock_level: 1
    type: spell
    element: fire
    description: An invented spell.
    mp_cost: 1
    spell_coeff: 1.0
    target: single_enemy"#,
            );
        fixture.write("data/classes/maker.yaml", &class);

        let report = validate_scenario_directory(&ScenarioRoot::default(), &fixture.0);
        let missing = report
            .errors()
            .find(|finding| {
                finding.location.path.as_str() == "data/classes/maker.yaml"
                    && finding.location.field_path == "abilities[0].element"
            })
            .expect("an offensive spell should require its derived element SFX key");
        assert_eq!(missing.code, "reference.missing");
        assert_eq!(missing.message, "unknown SFX id `spell_fire`");
    }

    #[test]
    #[ignore = "requires RPG_S1_PINNED_SCENARIO_DIR pointing at the pinned source scenario"]
    fn audits_complete_pinned_scenario_with_typed_production_validator() {
        let root =
            std::env::var("RPG_S1_PINNED_SCENARIO_DIR").expect("set RPG_S1_PINNED_SCENARIO_DIR");
        let report = validate_scenario_directory(&ScenarioRoot::default(), root);
        assert_eq!(report.counts.party_members, 5);
        assert_eq!(report.counts.classes, 5);
        assert_eq!(report.counts.abilities, 42);
        assert_eq!(report.counts.items, 172);
        assert_eq!(report.counts.field_use_items, 13);
        assert_eq!(report.counts.maps, 43);
        assert_eq!(report.counts.dialogue_documents, 91);
        assert_eq!(report.counts.enemies, 106);
        assert_eq!(report.counts.boss_move_sets, 9);
        assert_eq!(report.counts.encounters, 16);
        assert_eq!(report.counts.battle_backgrounds, 13);
        assert_eq!(report.counts.recipes, 11);
        assert_eq!(report.counts.quests, 16);
        assert_eq!(report.counts.bgm_keys, 12);
        assert_eq!(report.counts.sfx_keys, 23);
        assert!(
            report.checked_references > 700,
            "only {} references checked",
            report.checked_references
        );
        let errors = report.errors().map(ToString::to_string).collect::<Vec<_>>();
        let warnings = report
            .warnings()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        assert_eq!(
            errors.len(),
            37,
            "pinned disagreements changed:\n{errors:#?}"
        );
        assert_eq!(warnings.len(), 1, "pinned warnings changed:\n{warnings:#?}");
        assert_eq!(
            report
                .errors()
                .filter(|finding| finding.code == "flag.undefined")
                .count(),
            5
        );
        assert_eq!(
            report
                .errors()
                .filter(|finding| finding.code == "reference.missing")
                .count(),
            31
        );
        assert_eq!(
            report
                .errors()
                .filter(|finding| finding.code == "path.missing")
                .count(),
            1
        );
        let undefined_flags = report
            .errors()
            .filter(|finding| finding.code == "flag.undefined")
            .filter_map(|finding| finding.message.split('`').nth(1))
            .collect::<BTreeSet<_>>();
        assert_eq!(
            undefined_flags,
            BTreeSet::from([
                "story_ultimate_earth",
                "story_ultimate_fire",
                "story_ultimate_water",
                "story_ultimate_wind",
                "transport_warp_unlocked",
            ])
        );
        let missing_drop_items = report
            .errors()
            .filter(|finding| finding.location.field_path.contains(".drops.loot["))
            .filter_map(|finding| finding.message.split('`').nth(1))
            .collect::<BTreeSet<_>>();
        assert_eq!(
            missing_drop_items,
            BTreeSet::from([
                "fire_dragon_horn",
                "goblin_ear",
                "goblin_fang",
                "goblin_shield",
                "rusty_blade",
                "stone_dragon_horn",
                "void_core",
            ])
        );
        assert!(
            errors
                .iter()
                .any(|error| error.contains("title.cursor_icon"))
        );
        assert!(
            errors
                .iter()
                .any(|error| error.contains("zone.open_plains"))
        );
        assert!(errors.iter().any(|error| error.contains("goblin_fang")));
        assert!(
            errors
                .iter()
                .any(|error| error.contains("dungeon_ruinwatch"))
        );
        assert_eq!(
            warnings[0],
            "data/maps/zone_05_mountain_foothills.yaml:$same_stem_tmx: source.unmatched_map_metadata: no same-stem TMX file exists at `assets/maps/zone_05_mountain_foothills.tmx`"
        );
    }
}
