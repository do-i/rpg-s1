//! The validator, its typed catalogs, and the shared diagnostic and reference helpers.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use super::{
    index::ReferenceIndex,
    report::{DiagnosticSeverity, ScenarioDiagnostic, ScenarioLocation, ScenarioValidationReport},
};
use crate::{
    manifest_path_validation::ScenarioPathProbeResult,
    scenario_audio::{BgmIndex, SfxIndex},
    scenario_balance::BalanceData,
    scenario_battle_background::BattleBackgroundCatalog,
    scenario_class::ClassDefinition,
    scenario_dialogue::DialogueDocument,
    scenario_encounter::EncounterZone,
    scenario_enemy::{BossMoveSet, EnemyCatalogFile},
    scenario_item::{FieldUseCatalogFile, ItemCatalogFile},
    scenario_manifest::Manifest,
    scenario_map::MapMetadata,
    scenario_party::PartyCatalog,
    scenario_path::ScenarioRelativePath,
    scenario_quest::QuestCatalogFile,
    scenario_recipe::RecipeCatalogFile,
    scenario_root::ScenarioRoot,
    scenario_transport::TransportCatalog,
};

#[derive(Clone, Debug)]
pub(super) struct Located<T> {
    pub(super) path: String,
    pub(super) stem: String,
    pub(super) value: T,
}

#[derive(Default)]
pub(super) struct ScenarioCatalogs {
    pub(super) manifest: Option<Manifest>,
    pub(super) party: Option<Located<PartyCatalog>>,
    pub(super) classes: Vec<Located<ClassDefinition>>,
    pub(super) items: Vec<Located<ItemCatalogFile>>,
    pub(super) field_use: Option<Located<FieldUseCatalogFile>>,
    pub(super) maps: Vec<Located<MapMetadata>>,
    pub(super) item_boxes: Option<Located<crate::scenario_item_box::ItemBoxCatalog>>,
    pub(super) dialogue: Vec<Located<DialogueDocument>>,
    pub(super) enemies: Vec<Located<EnemyCatalogFile>>,
    pub(super) boss_move_sets: Vec<Located<BossMoveSet>>,
    pub(super) encounters: Vec<Located<EncounterZone>>,
    pub(super) recipes: Vec<Located<RecipeCatalogFile>>,
    pub(super) quests: Option<Located<QuestCatalogFile>>,
    pub(super) balance: Option<Located<BalanceData>>,
    pub(super) backgrounds: Option<Located<BattleBackgroundCatalog>>,
    pub(super) bgm: Option<Located<BgmIndex>>,
    pub(super) sfx: Option<Located<SfxIndex>>,
    pub(super) transport: Option<Located<TransportCatalog>>,
}

pub(super) struct Validator<'a> {
    pub(super) physical_root: &'a Path,
    pub(super) canonical_root: Option<PathBuf>,
    pub(super) report: ScenarioValidationReport,
    pub(super) catalogs: ScenarioCatalogs,
}

pub(super) fn located<T>(path: &str, value: T) -> Located<T> {
    Located {
        path: path.to_owned(),
        stem: file_stem(path).to_owned(),
        value,
    }
}

pub(super) fn file_stem(path: &str) -> &str {
    Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(path)
}

pub(super) fn probe_path(
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

impl<'a> Validator<'a> {
    pub(super) fn new(scenario_root: &ScenarioRoot, physical_root: &'a Path) -> Self {
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

    pub(super) fn error(
        &mut self,
        code: &'static str,
        path: &str,
        field_path: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.diagnostic(DiagnosticSeverity::Error, code, path, field_path, message);
    }

    pub(super) fn warning(
        &mut self,
        code: &'static str,
        path: &str,
        field_path: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.diagnostic(DiagnosticSeverity::Warning, code, path, field_path, message);
    }

    pub(super) fn validate(&mut self) {
        let Some(manifest) = self.catalogs.manifest.clone() else {
            return;
        };
        let mut index = ReferenceIndex::from_catalogs(&self.catalogs);
        self.add_tmx_map_ids(&mut index);
        self.validate_duplicates(&index);
        self.validate_manifest_ids(&manifest, &index);
        self.validate_transport(&index);
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

    pub(super) fn checked(
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

    pub(super) fn checked_path(&mut self, path: &str, field: impl Into<String>, referenced: &str) {
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
}
