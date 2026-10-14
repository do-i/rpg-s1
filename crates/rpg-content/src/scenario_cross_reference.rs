//! Whole-scenario loading and cross-reference validation.
//!
//! This is the production validation boundary for the typed M2 scenario schemas. It deliberately
//! keeps host paths private: diagnostics identify the selected [`ScenarioRoot`] package and a
//! stable scenario-relative `file#field` location. The pinned Python validator is the behavioral
//! baseline, while additional checks (AI references, audio, derived assets, and all typed catalog
//! edges) implement the strict direct-authoring policy shared by loading and validation.
//!
//! No catalog is reparsed into generic YAML. TMX portal links and TSX image references are the
//! only relevant source edges not modeled here: their typed XML parsing belongs to the TMX/TSX
//! adapter, so this layer checks same-stem TMX presence without an ad-hoc XML scanner.
//! The pinned quest schema contains flag-backed board rows rather than objective/reward records;
//! those flag edges are therefore the complete current quest reference surface.

mod audio;
mod combat;
mod context;
mod dialogue;
mod index;
mod items;
mod loading;
mod manifest;
mod maps;
mod report;
#[cfg(test)]
mod tests;

use std::path::Path;

use self::context::Validator;
pub use self::report::{
    DiagnosticSeverity, ScenarioCatalogCounts, ScenarioDiagnostic, ScenarioLocation,
    ScenarioValidationReport,
};
use crate::scenario_root::ScenarioRoot;

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
