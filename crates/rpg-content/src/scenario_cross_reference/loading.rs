//! Loads every manifest-selected typed catalog and records corpus totals.

use std::{
    collections::BTreeSet,
    fs, io,
    path::{Path, PathBuf},
};

use serde::de::DeserializeOwned;

use super::context::{Located, Validator, located, probe_path};
use crate::{
    manifest_path_validation::{ManifestPathValidationResult, validate_manifest_paths},
    scenario_audio::{BGM_INDEX_PATH, BgmIndex, SFX_INDEX_PATH, SfxIndex},
    scenario_balance::BalanceData,
    scenario_battle_background::BattleBackgroundCatalog,
    scenario_enemy::{BossMoveSet, EnemyCatalogFile},
    scenario_item::{FieldUseCatalogFile, ItemCatalogFile},
    scenario_manifest::Manifest,
    scenario_party::PartyCatalog,
    scenario_path::ScenarioRelativePath,
    scenario_quest::QuestCatalogFile,
    scenario_root::SCENARIO_MANIFEST_PATH,
    scenario_transport::TransportCatalog,
    scenario_yaml,
};

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

impl<'a> Validator<'a> {
    pub(super) fn load(&mut self) {
        let Some(manifest) = self.read_yaml::<Manifest>(SCENARIO_MANIFEST_PATH) else {
            return;
        };
        self.report.scenario_id = Some(manifest.id.clone());
        self.report.scenario_name = Some(manifest.name.clone());
        self.report.scenario_version = Some(manifest.version.clone());
        self.validate_manifest_paths(&manifest);

        let party_path = manifest.refs.party.as_str().to_owned();
        if let Some(transport) = &manifest.refs.transport {
            let transport_path = transport.as_str().to_owned();
            self.catalogs.transport = self
                .read_yaml::<TransportCatalog>(&transport_path)
                .map(|value| located(&transport_path, value));
        }
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

    pub(super) fn read_text(&mut self, relative: &str) -> Option<String> {
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
}
