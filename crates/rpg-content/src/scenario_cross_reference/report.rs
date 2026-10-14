//! The validation report: diagnostics, locations, catalog counts, and their stable order.

use std::fmt;

use crate::scenario_path::ScenarioRelativePath;

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
    pub(super) fn new(path: &str, field_path: impl Into<String>) -> Self {
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
    pub scenario_id: Option<String>,
    pub scenario_name: Option<String>,
    pub scenario_version: Option<String>,
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

    pub(super) fn finish(&mut self) {
        self.diagnostics.sort_by(|left, right| {
            left.severity
                .cmp(&right.severity)
                .then_with(|| left.location.cmp(&right.location))
                .then_with(|| left.code.cmp(right.code))
                .then_with(|| left.message.cmp(&right.message))
        });
    }
}
