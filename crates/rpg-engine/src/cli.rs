//! Process entrypoint routing for the game and its windowless scenario validator.
//!
//! Argument parsing happens before the game launcher is called. The `validate-scenario` command
//! accepts a package key, never a filesystem path, and resolves it beneath the bounded production
//! scenario collection. Tests inject a separate collection root through the same
//! boundary rather than adding fixture-only command-line options.

mod args;
mod execute;
mod packages;
mod report;
#[cfg(test)]
mod tests;

use std::{ffi::OsString, io::Write, path::Path};

pub(crate) use self::{args::PlayArguments, packages::production_asset_base};
use self::{execute::run_with_version, packages::production_scenario_collection};
use crate::{
    scenario_cross_reference::{ScenarioValidationReport, validate_scenario_directory},
    scenario_root::ScenarioRoot,
};

pub(crate) const EXIT_SUCCESS: u8 = 0;

pub(crate) const EXIT_VALIDATION_FAILED: u8 = 1;

pub(crate) const EXIT_USAGE: u8 = 2;

/// Routes one process invocation. Validation is completed before the Bevy launcher is considered.
pub(crate) fn run(
    arguments: impl IntoIterator<Item = OsString>,
    game_version: &str,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
    launch_bevy_app: impl FnOnce(PlayArguments),
) -> u8 {
    let collection_root = production_scenario_collection();
    run_with_version(
        arguments,
        game_version,
        &collection_root,
        production_validate,
        stdout,
        stderr,
        launch_bevy_app,
    )
}

fn production_validate(root: &ScenarioRoot, selected: &Path) -> ScenarioValidationReport {
    validate_scenario_directory(root, selected)
}

#[cfg(test)]
fn run_with<V>(
    arguments: impl IntoIterator<Item = OsString>,
    collection_root: &Path,
    validator: V,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
    launch_bevy_app: impl FnOnce(PlayArguments),
) -> u8
where
    V: FnOnce(&ScenarioRoot, &Path) -> ScenarioValidationReport,
{
    run_with_version(
        arguments,
        env!("CARGO_PKG_VERSION"),
        collection_root,
        validator,
        stdout,
        stderr,
        launch_bevy_app,
    )
}
