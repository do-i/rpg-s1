//! Process entrypoint routing for the game and its windowless scenario validator.
//!
//! Argument parsing happens before the game launcher is called. The `validate-scenario` command
//! accepts a package key, never a filesystem path, and resolves it beneath the bounded production
//! collection selected by ADR 0004. Tests inject a separate collection root through the same
//! boundary rather than adding fixture-only command-line options.

use std::{
    ffi::OsString,
    fmt, fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use crate::{
    scenario_cross_reference::{
        DiagnosticSeverity, ScenarioCatalogCounts, ScenarioDiagnostic, ScenarioLocation,
        ScenarioValidationReport, validate_scenario_directory,
    },
    scenario_path::ScenarioRelativePath,
    scenario_root::{DEFAULT_SCENARIO_PACKAGE_KEY, SCENARIO_MANIFEST_PATH, ScenarioRoot},
};

pub(crate) const EXIT_SUCCESS: u8 = 0;
pub(crate) const EXIT_VALIDATION_FAILED: u8 = 1;
pub(crate) const EXIT_USAGE: u8 = 2;
const PRODUCTION_SCENARIO_COLLECTION: &str = "assets/scenarios";
const USAGE: &str = "Usage:\n  rpg-s1\n  rpg-s1 validate-scenario [PACKAGE_KEY]\n\nValidate defaults to package `rusted_kingdoms`. PACKAGE_KEY is a portable package name, not a path.";

enum Command {
    Play,
    Validate(ScenarioRoot),
    Help,
}

#[derive(Debug, Eq, PartialEq)]
struct UsageError(String);

impl fmt::Display for UsageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Routes one process invocation. Validation is completed before the Bevy launcher is considered.
pub(crate) fn run(
    arguments: impl IntoIterator<Item = OsString>,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
    launch_bevy_app: impl FnOnce(),
) -> u8 {
    let collection_root = production_scenario_collection();
    run_with(
        arguments,
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

fn run_with<V>(
    arguments: impl IntoIterator<Item = OsString>,
    collection_root: &Path,
    validator: V,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
    launch_bevy_app: impl FnOnce(),
) -> u8
where
    V: FnOnce(&ScenarioRoot, &Path) -> ScenarioValidationReport,
{
    let command = match parse_command(arguments) {
        Ok(command) => command,
        Err(error) => {
            let _ = writeln!(stderr, "error: {error}\n\n{USAGE}");
            return EXIT_USAGE;
        }
    };

    match command {
        Command::Play => {
            launch_bevy_app();
            EXIT_SUCCESS
        }
        Command::Help => match writeln!(stdout, "{USAGE}") {
            Ok(()) => EXIT_SUCCESS,
            Err(_) => EXIT_USAGE,
        },
        Command::Validate(root) => {
            let report = validate_selected_scenario(collection_root, &root, validator);
            let result = if report.is_valid() {
                EXIT_SUCCESS
            } else {
                EXIT_VALIDATION_FAILED
            };
            if write_validation_report(stdout, &report).is_err() {
                EXIT_USAGE
            } else {
                result
            }
        }
    }
}

fn parse_command(arguments: impl IntoIterator<Item = OsString>) -> Result<Command, UsageError> {
    let arguments = arguments
        .into_iter()
        .map(|argument| {
            argument
                .into_string()
                .map_err(|_| UsageError("arguments must be valid UTF-8".to_owned()))
        })
        .collect::<Result<Vec<_>, _>>()?;

    match arguments.as_slice() {
        [] => Ok(Command::Play),
        [flag] if flag == "-h" || flag == "--help" => Ok(Command::Help),
        [command, flag] if command == "validate-scenario" && (flag == "-h" || flag == "--help") => {
            Ok(Command::Help)
        }
        [command] if command == "validate-scenario" => Ok(Command::Validate(
            ScenarioRoot::try_for_package_key(DEFAULT_SCENARIO_PACKAGE_KEY)
                .expect("the default package key is valid"),
        )),
        [command, package_key] if command == "validate-scenario" => {
            ScenarioRoot::try_for_package_key(package_key.clone())
                .map(Command::Validate)
                .map_err(|error| {
                    UsageError(format!("invalid package key `{package_key}`: {error}"))
                })
        }
        [command, ..] if command == "validate-scenario" => Err(UsageError(
            "validate-scenario accepts at most one package key".to_owned(),
        )),
        [command, ..] => Err(UsageError(format!("unknown command `{command}`"))),
    }
}

fn validate_selected_scenario<V>(
    collection_root: &Path,
    root: &ScenarioRoot,
    validator: V,
) -> ScenarioValidationReport
where
    V: FnOnce(&ScenarioRoot, &Path) -> ScenarioValidationReport,
{
    let Ok(canonical_collection) = collection_root.canonicalize() else {
        return selection_failure(
            root,
            "scenario.collection_unavailable",
            "scenario collection is unavailable",
        );
    };
    if !canonical_collection.is_dir() {
        return selection_failure(
            root,
            "scenario.collection_unavailable",
            "scenario collection is not a directory",
        );
    }

    let selected = collection_root.join(root.package_key());
    let Ok(metadata) = fs::symlink_metadata(&selected) else {
        return selection_failure(
            root,
            "scenario.package_unavailable",
            "selected scenario package is unavailable",
        );
    };
    if metadata.file_type().is_symlink() {
        return selection_failure(
            root,
            "scenario.package_escape",
            "selected scenario package must not be a symbolic link",
        );
    }
    let Ok(canonical_selected) = selected.canonicalize() else {
        return selection_failure(
            root,
            "scenario.package_unavailable",
            "selected scenario package cannot be resolved",
        );
    };
    if !canonical_selected.starts_with(&canonical_collection) {
        return selection_failure(
            root,
            "scenario.package_escape",
            "selected scenario package resolves outside the scenario collection",
        );
    }
    if !canonical_selected.is_dir() {
        return selection_failure(
            root,
            "scenario.package_unavailable",
            "selected scenario package is not a directory",
        );
    }

    validator(root, &canonical_selected)
}

fn selection_failure(
    root: &ScenarioRoot,
    code: &'static str,
    message: &'static str,
) -> ScenarioValidationReport {
    ScenarioValidationReport {
        package_key: root.package_key().to_owned(),
        diagnostics: vec![ScenarioDiagnostic {
            severity: DiagnosticSeverity::Error,
            code,
            location: ScenarioLocation {
                path: ScenarioRelativePath::try_from(SCENARIO_MANIFEST_PATH)
                    .expect("the manifest path is valid"),
                field_path: "$selection".to_owned(),
            },
            message: message.to_owned(),
        }],
        ..Default::default()
    }
}

fn write_validation_report(
    output: &mut impl Write,
    report: &ScenarioValidationReport,
) -> io::Result<()> {
    let errors = report.errors().count();
    let warnings = report.warnings().count();
    writeln!(output, "Scenario validation")?;
    writeln!(output, "Package: {}", report.package_key)?;
    match (
        report.scenario_id.as_deref(),
        report.scenario_name.as_deref(),
        report.scenario_version.as_deref(),
    ) {
        (Some(id), Some(name), Some(version)) => {
            writeln!(output, "Scenario: {id} ({name}), version {version}")?;
        }
        _ => writeln!(output, "Scenario: unavailable")?,
    }
    writeln!(
        output,
        "Status: {}",
        if report.is_valid() { "PASS" } else { "FAIL" }
    )?;
    write_counts(output, &report.counts)?;
    writeln!(output, "References checked: {}", report.checked_references)?;
    writeln!(
        output,
        "Diagnostics: {errors} error(s), {warnings} warning(s)"
    )?;

    let mut diagnostics = report.diagnostics.iter().collect::<Vec<_>>();
    diagnostics.sort_by(|left, right| {
        left.severity
            .cmp(&right.severity)
            .then_with(|| left.location.cmp(&right.location))
            .then_with(|| left.code.cmp(right.code))
            .then_with(|| left.message.cmp(&right.message))
    });
    for diagnostic in diagnostics {
        writeln!(
            output,
            "{} {}:{} [{}] {}",
            severity_name(diagnostic.severity),
            report.package_key,
            diagnostic.location,
            diagnostic.code,
            diagnostic.message
        )?;
    }
    Ok(())
}

fn write_counts(output: &mut impl Write, counts: &ScenarioCatalogCounts) -> io::Result<()> {
    writeln!(
        output,
        "Counts: party={}, classes={}, abilities={}, items={}, field_use={}, maps={}, dialogue={}, enemies={}, boss_move_sets={}, encounters={}, backgrounds={}, recipes={}, quests={}, bgm={}, sfx={}",
        counts.party_members,
        counts.classes,
        counts.abilities,
        counts.items,
        counts.field_use_items,
        counts.maps,
        counts.dialogue_documents,
        counts.enemies,
        counts.boss_move_sets,
        counts.encounters,
        counts.battle_backgrounds,
        counts.recipes,
        counts.quests,
        counts.bgm_keys,
        counts.sfx_keys,
    )
}

fn severity_name(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Error => "error",
        DiagnosticSeverity::Warning => "warning",
    }
}

fn production_scenario_collection() -> PathBuf {
    let manifest_directory = Path::new(env!("CARGO_MANIFEST_DIR"));
    match std::env::current_exe() {
        Ok(executable) => scenario_collection_for_executable(&executable, manifest_directory),
        Err(_) => manifest_directory.join(PRODUCTION_SCENARIO_COLLECTION),
    }
}

/// Resolves exactly the two layouts owned by ADR 0004 without inspecting the current directory.
///
/// Cargo-built development binaries live below the repository and use its collection. A packaged
/// executable outside that tree owns an `assets/` directory beside itself. The returned path does
/// not need to exist: selection reports the stable collection-unavailable diagnostic later.
fn scenario_collection_for_executable(executable: &Path, manifest_directory: &Path) -> PathBuf {
    if executable.starts_with(manifest_directory) {
        manifest_directory.join(PRODUCTION_SCENARIO_COLLECTION)
    } else {
        executable
            .parent()
            .unwrap_or(manifest_directory)
            .join(PRODUCTION_SCENARIO_COLLECTION)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        cell::Cell,
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

    struct TempCollection(PathBuf);

    impl TempCollection {
        fn new(package_key: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should follow the Unix epoch")
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "rpg-s1-cli-{}-{nonce}-{}",
                std::process::id(),
                NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(root.join(package_key))
                .expect("temporary scenario collection should be creatable");
            Self(root)
        }
    }

    impl Drop for TempCollection {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("temporary scenario collection should be removable");
        }
    }

    fn valid_report(root: &ScenarioRoot, selected: &Path) -> ScenarioValidationReport {
        assert!(selected.is_dir());
        assert_eq!(
            selected.file_name().and_then(|name| name.to_str()),
            Some(root.package_key())
        );
        ScenarioValidationReport {
            package_key: root.package_key().to_owned(),
            scenario_id: Some("invented_story".to_owned()),
            scenario_name: Some("Invented Story".to_owned()),
            scenario_version: Some("1.0".to_owned()),
            counts: ScenarioCatalogCounts {
                party_members: 1,
                classes: 1,
                maps: 1,
                ..Default::default()
            },
            checked_references: 73,
            ..Default::default()
        }
    }

    fn invalid_report(root: &ScenarioRoot, _: &Path) -> ScenarioValidationReport {
        ScenarioValidationReport {
            package_key: root.package_key().to_owned(),
            scenario_id: Some("broken_story".to_owned()),
            scenario_name: Some("Broken Story".to_owned()),
            scenario_version: Some("1.0".to_owned()),
            checked_references: 2,
            diagnostics: vec![
                ScenarioDiagnostic {
                    severity: DiagnosticSeverity::Warning,
                    code: "content.warning",
                    location: ScenarioLocation {
                        path: ScenarioRelativePath::try_from("data/maps/z.yaml").unwrap(),
                        field_path: "name".to_owned(),
                    },
                    message: "later warning".to_owned(),
                },
                ScenarioDiagnostic {
                    severity: DiagnosticSeverity::Error,
                    code: "reference.missing",
                    location: ScenarioLocation {
                        path: ScenarioRelativePath::try_from("data/maps/a.yaml").unwrap(),
                        field_path: "npcs[0].dialogue".to_owned(),
                    },
                    message: "unknown dialogue id `lost`".to_owned(),
                },
            ],
            ..Default::default()
        }
    }

    #[test]
    fn development_executable_uses_the_manifest_repository_collection() {
        let workspace = std::env::temp_dir().join("invented-workspace/rpg-s1");
        let executable = workspace.join("target/debug/rpg-s1");

        assert_eq!(
            scenario_collection_for_executable(&executable, &workspace),
            workspace.join("assets/scenarios")
        );
    }

    #[test]
    fn installed_executable_uses_only_assets_beside_its_package_binary() {
        let base = std::env::temp_dir().join("invented-installed-layout");
        let manifest_directory = base.join("source/rpg-s1");
        let executable = base.join("package/rpg-s1");

        assert_eq!(
            scenario_collection_for_executable(&executable, &manifest_directory),
            base.join("package/assets/scenarios")
        );
    }

    #[test]
    fn collection_resolution_is_absolute_and_has_no_working_directory_input() {
        let base = std::env::temp_dir().join("invented-cwd-independent-layout");
        let manifest_directory = base.join("source/rpg-s1");
        let executable = base.join("package/rpg-s1");

        let collection = scenario_collection_for_executable(&executable, &manifest_directory);

        assert!(collection.is_absolute());
        assert_eq!(
            collection,
            executable.parent().unwrap().join("assets/scenarios")
        );
    }

    #[test]
    fn missing_installed_collection_does_not_trigger_an_ancestor_search() {
        let temporary = TempCollection::new("unrelated");
        let manifest_directory = temporary.0.join("source/rpg-s1");
        let executable = temporary.0.join("package/bin/rpg-s1");
        let unrelated_ancestor_collection = temporary.0.join("package/assets/scenarios");
        fs::create_dir_all(&unrelated_ancestor_collection)
            .expect("unrelated ancestor collection should be creatable");
        let intended = temporary.0.join("package/bin/assets/scenarios");
        assert!(!intended.exists());

        assert_eq!(
            scenario_collection_for_executable(&executable, &manifest_directory),
            intended
        );
    }

    #[test]
    fn validation_command_does_not_invoke_the_bevy_app_launcher() {
        let collection = TempCollection::new("fixture");
        let launched = Cell::new(false);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run_with(
            ["validate-scenario".into(), "fixture".into()],
            &collection.0,
            valid_report,
            &mut stdout,
            &mut stderr,
            || launched.set(true),
        );

        assert_eq!(exit, EXIT_SUCCESS);
        assert!(
            !launched.get(),
            "validation must not construct the Bevy app"
        );
        assert!(stderr.is_empty());
        let output = String::from_utf8(stdout).unwrap();
        assert!(output.contains("Package: fixture\n"));
        assert!(output.contains("Scenario: invented_story (Invented Story), version 1.0\n"));
        assert!(output.contains("Status: PASS\n"));
        assert!(output.contains("References checked: 73\n"));
        assert!(output.contains("Diagnostics: 0 error(s), 0 warning(s)\n"));
    }

    #[test]
    fn validation_errors_exit_one_and_render_diagnostics_deterministically() {
        let collection = TempCollection::new("broken");
        let launched = Cell::new(false);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run_with(
            ["validate-scenario".into(), "broken".into()],
            &collection.0,
            invalid_report,
            &mut stdout,
            &mut stderr,
            || launched.set(true),
        );

        assert_eq!(exit, EXIT_VALIDATION_FAILED);
        assert!(!launched.get());
        assert!(stderr.is_empty());
        let output = String::from_utf8(stdout).unwrap();
        assert!(output.contains("Status: FAIL\n"));
        assert!(output.contains("Diagnostics: 1 error(s), 1 warning(s)\n"));
        let error = output.find("error broken:data/maps/a.yaml").unwrap();
        let warning = output.find("warning broken:data/maps/z.yaml").unwrap();
        assert!(
            error < warning,
            "errors must sort before warnings:\n{output}"
        );
    }

    #[test]
    fn default_validation_selects_the_default_package_key() {
        let collection = TempCollection::new(DEFAULT_SCENARIO_PACKAGE_KEY);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run_with(
            ["validate-scenario".into()],
            &collection.0,
            valid_report,
            &mut stdout,
            &mut stderr,
            || panic!("validation must not launch Bevy"),
        );

        assert_eq!(exit, EXIT_SUCCESS);
        assert!(
            String::from_utf8(stdout)
                .unwrap()
                .contains("Package: rusted_kingdoms\n")
        );
        assert!(stderr.is_empty());
    }

    #[test]
    fn missing_package_is_a_validation_failure_without_host_path_leakage() {
        let collection = TempCollection::new("present");
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run_with(
            ["validate-scenario".into(), "missing".into()],
            &collection.0,
            |_, _| panic!("an unavailable package must not reach the validator"),
            &mut stdout,
            &mut stderr,
            || panic!("validation must not launch Bevy"),
        );

        assert_eq!(exit, EXIT_VALIDATION_FAILED);
        let output = String::from_utf8(stdout).unwrap();
        assert!(output.contains("[scenario.package_unavailable]"));
        assert!(!output.contains(collection.0.to_string_lossy().as_ref()));
        assert!(stderr.is_empty());
    }

    #[test]
    fn command_path_calls_the_shared_production_validator_without_launching_bevy() {
        let collection = TempCollection::new("empty");
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run_with(
            ["validate-scenario".into(), "empty".into()],
            &collection.0,
            production_validate,
            &mut stdout,
            &mut stderr,
            || panic!("shared validation must not launch Bevy"),
        );

        assert_eq!(exit, EXIT_VALIDATION_FAILED);
        let output = String::from_utf8(stdout).unwrap();
        assert!(output.contains("error empty:manifest.yaml#$ [io.read]"));
        assert!(!output.contains(collection.0.to_string_lossy().as_ref()));
        assert!(stderr.is_empty());
    }

    #[test]
    fn argument_misuse_exits_two_without_launching_or_validating() {
        for arguments in [
            vec!["unknown".into()],
            vec!["validate-scenario".into(), "../escape".into()],
            vec!["validate-scenario".into(), "one".into(), "two".into()],
        ] {
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            let exit = run_with(
                arguments,
                Path::new("unused"),
                |_, _| panic!("misuse must not validate"),
                &mut stdout,
                &mut stderr,
                || panic!("misuse must not launch Bevy"),
            );
            assert_eq!(exit, EXIT_USAGE);
            assert!(stdout.is_empty());
            assert!(String::from_utf8(stderr).unwrap().contains("Usage:"));
        }
    }

    #[test]
    fn no_subcommand_is_the_only_path_that_invokes_the_bevy_launcher() {
        let launched = Cell::new(false);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let exit = run_with(
            [],
            Path::new("unused"),
            |_, _| panic!("play mode must not validate"),
            &mut stdout,
            &mut stderr,
            || launched.set(true),
        );

        assert_eq!(exit, EXIT_SUCCESS);
        assert!(launched.get());
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
    }

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
