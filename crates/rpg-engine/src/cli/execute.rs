//! Runs a parsed command: launches play, record, and replay, and drives the windowless tools.

use std::{ffi::OsString, fs, io::Write, path::Path};

use super::{
    EXIT_SUCCESS, EXIT_USAGE, EXIT_VALIDATION_FAILED,
    args::{Command, PlayArguments, USAGE, apply_environment_debug_defaults, parse_command},
    packages::{select_scenario_package, selection_failure},
    report::{
        validation_report_lines, write_baseline_comparison, write_debug_log, write_dialogue_report,
        write_dialogue_traversal_report, write_encounter_sweep_report, write_map_report,
        write_map_sweep_report, write_validation_report,
    },
};
use crate::{
    debug_launch::validate_debug_launch,
    dialogue_sweep::{DialogueTraversalReport, build_dialogue_traversal_sweep},
    encounter_sweep::{EncounterSweepReport, build_encounter_sweep},
    input_record::{InputAutomation, InputRecord, InputScript, RecordSource},
    runtime_map_sweep::{RuntimeMapSweepReport, build_runtime_map_sweep},
    scenario_cross_reference::ScenarioValidationReport,
    scenario_dialogue_report::{DialogueReport, build_dialogue_report},
    scenario_manifest::ManifestIdentityWindow,
    scenario_map_report::{MapReport, build_map_report},
    scenario_map_sweep::{MapSweepReport, build_map_sweep},
    scenario_root::{SCENARIO_MANIFEST_PATH, ScenarioRoot},
    scenario_validation_baseline::{compare_to_baseline, parse_baseline},
    scenario_yaml,
};

pub(super) fn run_with_version<V>(
    arguments: impl IntoIterator<Item = OsString>,
    game_version: &str,
    collection_root: &Path,
    validator: V,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
    launch_bevy_app: impl FnOnce(PlayArguments),
) -> u8
where
    V: FnOnce(&ScenarioRoot, &Path) -> ScenarioValidationReport,
{
    let mut command = match parse_command(arguments) {
        Ok(command) => command,
        Err(error) => {
            let _ = writeln!(stderr, "error: {error}\n\n{USAGE}");
            return EXIT_USAGE;
        }
    };
    if let Err(error) =
        apply_environment_debug_defaults(&mut command, |name| std::env::var_os(name))
    {
        let _ = writeln!(stderr, "error: {error}\n\n{USAGE}");
        return EXIT_USAGE;
    }

    match command {
        Command::Play(mut arguments) => {
            if let Err(error) = prepare_debug_launch(collection_root, &arguments)
                .and_then(|()| prepare_perf_script(&mut arguments))
            {
                let _ = writeln!(stderr, "error: {error}");
                return EXIT_VALIDATION_FAILED;
            }
            let _ = writeln!(stdout, "Gameplay seed: {}", arguments.seed);
            if arguments.timings {
                let _ = writeln!(stdout, "Per-system frame timing: enabled");
            }
            if let Some(perf) = arguments.perf.as_ref() {
                let _ = writeln!(
                    stdout,
                    "Perf measurement: script {}, report {}",
                    perf.script_path.display(),
                    perf.report.display()
                );
            }
            write_debug_log(stdout, arguments.debug.as_ref());
            launch_bevy_app(arguments);
            EXIT_SUCCESS
        }
        Command::Record { output, mut play } => {
            let record = match prepare_record(collection_root, game_version, &play, &output) {
                Ok(record) => record,
                Err(error) => {
                    let _ = writeln!(stderr, "error: {error}");
                    return EXIT_VALIDATION_FAILED;
                }
            };
            let _ = writeln!(stdout, "Gameplay seed: {}", play.seed);
            if play.timings {
                let _ = writeln!(stdout, "Per-system frame timing: enabled");
            }
            write_debug_log(stdout, play.debug.as_ref());
            let _ = writeln!(
                stdout,
                "Recording normalized actions to {}",
                output.display()
            );
            play.automation = Some(InputAutomation::record(output, record));
            launch_bevy_app(play);
            EXIT_SUCCESS
        }
        Command::Replay(input) => {
            let play = match prepare_replay(collection_root, game_version, &input) {
                Ok(play) => play,
                Err(error) => {
                    let _ = writeln!(stderr, "error: {error}");
                    return EXIT_VALIDATION_FAILED;
                }
            };
            let _ = writeln!(stdout, "Gameplay seed: {}", play.seed);
            write_debug_log(stdout, play.debug.as_ref());
            let _ = writeln!(
                stdout,
                "Replaying normalized actions from {}",
                input.display()
            );
            launch_bevy_app(play);
            EXIT_SUCCESS
        }
        Command::Help => match writeln!(stdout, "{USAGE}") {
            Ok(()) => EXIT_SUCCESS,
            Err(_) => EXIT_USAGE,
        },
        Command::Validate(arguments) => {
            let report = validate_selected_scenario(collection_root, &arguments.root, validator);
            if write_validation_report(stdout, &report).is_err() {
                return EXIT_USAGE;
            }
            let Some(baseline_path) = arguments.baseline else {
                return if report.is_valid() {
                    EXIT_SUCCESS
                } else {
                    EXIT_VALIDATION_FAILED
                };
            };
            let contents = match fs::read_to_string(&baseline_path) {
                Ok(contents) => contents,
                Err(error) => {
                    let _ = writeln!(
                        stderr,
                        "cannot read baseline {}: {error}",
                        baseline_path.display()
                    );
                    return EXIT_USAGE;
                }
            };
            let comparison = compare_to_baseline(
                &validation_report_lines(&report),
                &parse_baseline(&contents),
            );
            if write_baseline_comparison(stdout, &baseline_path, &comparison).is_err() {
                EXIT_USAGE
            } else if comparison.is_clean() {
                EXIT_SUCCESS
            } else {
                EXIT_VALIDATION_FAILED
            }
        }
        Command::MapReport(root) => {
            let report = match select_scenario_package(collection_root, &root) {
                Ok(selected) => build_map_report(&selected),
                Err(error) => MapReport::with_load_error(error.message),
            };
            // Informational: findings are described in the body, not through the exit code.
            if write_map_report(stdout, &root, &report).is_err() {
                EXIT_USAGE
            } else {
                EXIT_SUCCESS
            }
        }
        Command::MapSweep(root) => {
            let (report, runtime) = match select_scenario_package(collection_root, &root) {
                Ok(selected) => {
                    let asset_base = collection_root.parent().unwrap_or(collection_root);
                    (
                        build_map_sweep(&selected),
                        build_runtime_map_sweep(asset_base, &root, &selected),
                    )
                }
                Err(error) => (
                    MapSweepReport::with_load_error(error.message),
                    RuntimeMapSweepReport::with_load_error(error.message),
                ),
            };
            let result = if runtime.is_valid() {
                EXIT_SUCCESS
            } else {
                EXIT_VALIDATION_FAILED
            };
            if write_map_sweep_report(stdout, &root, &report, &runtime).is_err() {
                EXIT_USAGE
            } else {
                result
            }
        }
        Command::DialogueReport(root) => {
            let report = match select_scenario_package(collection_root, &root) {
                Ok(selected) => build_dialogue_report(&selected),
                Err(error) => DialogueReport::with_load_error(error.message),
            };
            // Informational: findings are described in the body, not through the exit code.
            if write_dialogue_report(stdout, &root, &report).is_err() {
                EXIT_USAGE
            } else {
                EXIT_SUCCESS
            }
        }
        Command::DialogueSweep(root) => {
            let report = match select_scenario_package(collection_root, &root) {
                Ok(selected) => build_dialogue_traversal_sweep(&selected),
                Err(error) => DialogueTraversalReport::with_load_error(error.message),
            };
            let result = if report.is_valid() {
                EXIT_SUCCESS
            } else {
                EXIT_VALIDATION_FAILED
            };
            if write_dialogue_traversal_report(stdout, &root, &report).is_err() {
                EXIT_USAGE
            } else {
                result
            }
        }
        Command::EncounterSweep(root) => {
            let report = match select_scenario_package(collection_root, &root) {
                Ok(selected) => {
                    let asset_base = collection_root.parent().unwrap_or(collection_root);
                    build_encounter_sweep(asset_base, &root, &selected)
                }
                Err(error) => EncounterSweepReport::with_load_error(error.message),
            };
            let result = if report.is_valid() {
                EXIT_SUCCESS
            } else {
                EXIT_VALIDATION_FAILED
            };
            if write_encounter_sweep_report(stdout, &root, &report).is_err() {
                EXIT_USAGE
            } else {
                result
            }
        }
    }
}

fn prepare_record(
    collection_root: &Path,
    game_version: &str,
    play: &PlayArguments,
    output: &Path,
) -> Result<InputRecord, String> {
    if output.exists() {
        return Err(format!(
            "record output `{}` already exists; choose a new path",
            output.display()
        ));
    }
    prepare_debug_launch(collection_root, play)?;
    let source = record_source(collection_root, &play.root)?;
    let record = InputRecord::new(game_version, source, play.seed).with_debug(play.debug.clone());
    let encoded = record.encode().map_err(|error| error.to_string())?;
    fs::write(output, encoded)
        .map_err(|error| format!("could not create record `{}`: {error}", output.display()))?;
    Ok(record)
}

fn prepare_replay(
    collection_root: &Path,
    game_version: &str,
    input: &Path,
) -> Result<PlayArguments, String> {
    let bytes = fs::read(input)
        .map_err(|error| format!("could not read replay `{}`: {error}", input.display()))?;
    let record = InputRecord::decode(&bytes).map_err(|error| error.to_string())?;
    if record.game_version != game_version {
        return Err(format!(
            "replay game version `{}` does not match running game version `{game_version}`",
            record.game_version
        ));
    }
    let root = ScenarioRoot::try_for_package_key(record.source.package_key.clone())
        .map_err(|error| format!("invalid replay package key: {error}"))?;
    let current = record_source(collection_root, &root)?;
    if current != record.source {
        return Err(format!(
            "replay source {} {} does not match installed source {} {}",
            record.source.scenario_id,
            record.source.scenario_version,
            current.scenario_id,
            current.scenario_version
        ));
    }
    let play = PlayArguments {
        root,
        seed: record.seed,
        timings: false,
        debug: record.debug.clone(),
        automation: Some(InputAutomation::replay(input.to_owned(), record)),
        perf: None,
    };
    prepare_debug_launch(collection_root, &play)?;
    Ok(play)
}

fn prepare_perf_script(play: &mut PlayArguments) -> Result<(), String> {
    let Some(perf) = play.perf.as_mut() else {
        return Ok(());
    };
    let text = fs::read_to_string(&perf.script_path).map_err(|error| {
        format!(
            "could not read perf script `{}`: {error}",
            perf.script_path.display()
        )
    })?;
    perf.script = InputScript::parse(&text)
        .map_err(|error| format!("perf script `{}`: {error}", perf.script_path.display()))?;
    Ok(())
}

fn prepare_debug_launch(collection_root: &Path, play: &PlayArguments) -> Result<(), String> {
    let Some(debug) = play.debug.as_ref() else {
        return Ok(());
    };
    let selected =
        select_scenario_package(collection_root, &play.root).map_err(|error| error.message)?;
    validate_debug_launch(&selected, debug)
}

fn record_source(collection_root: &Path, root: &ScenarioRoot) -> Result<RecordSource, String> {
    let selected = select_scenario_package(collection_root, root).map_err(|error| error.message)?;
    let manifest_path = selected.join(SCENARIO_MANIFEST_PATH);
    let document = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("could not read selected scenario manifest: {error}"))?;
    let identity: ManifestIdentityWindow = scenario_yaml::from_str(&document)
        .map_err(|error| format!("selected scenario manifest is invalid: {error}"))?;
    Ok(RecordSource {
        package_key: root.package_key().to_owned(),
        scenario_id: identity.id,
        scenario_version: identity.version,
    })
}

fn validate_selected_scenario<V>(
    collection_root: &Path,
    root: &ScenarioRoot,
    validator: V,
) -> ScenarioValidationReport
where
    V: FnOnce(&ScenarioRoot, &Path) -> ScenarioValidationReport,
{
    match select_scenario_package(collection_root, root) {
        Ok(selected) => validator(root, &selected),
        Err(error) => selection_failure(root, error.code, error.message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::tests::TempCollection;

    #[cfg(unix)]
    #[test]
    fn selected_package_symlink_is_rejected_before_validation() {
        use std::os::unix::fs::symlink;

        let collection = TempCollection::new("present");
        let outside = collection.0.with_extension("outside");
        fs::create_dir_all(&outside).expect("outside package should be creatable");
        symlink(&outside, collection.0.join("linked"))
            .expect("package symlink should be creatable");
        let root = ScenarioRoot::try_for_package_key("linked").unwrap();

        let report = validate_selected_scenario(&collection.0, &root, |_, _| {
            panic!("a symlinked package must not reach validation")
        });
        assert_eq!(
            report.errors().next().unwrap().code,
            "scenario.package_escape"
        );
        assert!(
            !report.diagnostics[0]
                .to_string()
                .contains(outside.to_string_lossy().as_ref())
        );

        fs::remove_dir_all(outside).expect("outside package should be removable");
    }
}
