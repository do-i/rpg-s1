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
    python_save_import::{PythonImportCatalog, convert_python_save, install_python_import},
    save_store::{SaveStore, resolve_save_directory, unix_timestamp_now},
    scenario_cross_reference::{
        DiagnosticSeverity, ScenarioCatalogCounts, ScenarioDiagnostic, ScenarioLocation,
        ScenarioValidationReport, validate_scenario_directory,
    },
    scenario_dialogue_report::{
        DialogueReachability, DialogueReport, DialogueReportEntry, DialogueReportShape,
        build_dialogue_report,
    },
    scenario_map_report::{MapReport, build_map_report},
    scenario_map_sweep::{MapSweepReport, SweepCategory, build_map_sweep},
    scenario_path::ScenarioRelativePath,
    scenario_root::{DEFAULT_SCENARIO_PACKAGE_KEY, SCENARIO_MANIFEST_PATH, ScenarioRoot},
};

pub(crate) const EXIT_SUCCESS: u8 = 0;
pub(crate) const EXIT_VALIDATION_FAILED: u8 = 1;
pub(crate) const EXIT_USAGE: u8 = 2;
const USAGE: &str = "Usage:\n  rpg-s1\n  rpg-s1 play [PACKAGE_KEY]\n  rpg-s1 validate-scenario [PACKAGE_KEY]\n  rpg-s1 map-report [PACKAGE_KEY]\n  rpg-s1 map-sweep [PACKAGE_KEY]\n  rpg-s1 dialogue-report [PACKAGE_KEY]\n  rpg-s1 import-python-save INPUT --slot 0..100 [--package PACKAGE_KEY] [--allow-unchecked] [--replace]\n\nScenario commands default to package `rusted_kingdoms`. PACKAGE_KEY is a portable package name, not a path. Python import is explicit, one-way, and never scans for legacy saves.";

enum Command {
    Play(ScenarioRoot),
    Validate(ScenarioRoot),
    MapReport(ScenarioRoot),
    MapSweep(ScenarioRoot),
    DialogueReport(ScenarioRoot),
    ImportPython(ImportPythonArguments),
    Help,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ImportPythonArguments {
    input: PathBuf,
    slot: usize,
    root: ScenarioRoot,
    allow_unchecked: bool,
    replace: bool,
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
    launch_bevy_app: impl FnOnce(ScenarioRoot),
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
    launch_bevy_app: impl FnOnce(ScenarioRoot),
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
        Command::Play(root) => {
            launch_bevy_app(root);
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
            let report = match select_scenario_package(collection_root, &root) {
                Ok(selected) => build_map_sweep(&selected),
                Err(error) => MapSweepReport::with_load_error(error.message),
            };
            // Informational: findings are described in the body, not through the exit code.
            if write_map_sweep_report(stdout, &root, &report).is_err() {
                EXIT_USAGE
            } else {
                EXIT_SUCCESS
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
        Command::ImportPython(arguments) => match run_python_import(collection_root, &arguments) {
            Ok(result) => {
                let _ = writeln!(
                    stdout,
                    "Imported Python save into {}",
                    result
                        .destination
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("native slot")
                );
                if let Some(backup) = result.backup {
                    let _ = writeln!(
                        stdout,
                        "Preserved previous slot as import-backups/{}",
                        backup
                            .file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or("verified backup")
                    );
                }
                EXIT_SUCCESS
            }
            Err(error) => {
                let _ = writeln!(stderr, "error: {error}");
                EXIT_VALIDATION_FAILED
            }
        },
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
        [] => Ok(Command::Play(
            ScenarioRoot::try_for_package_key(DEFAULT_SCENARIO_PACKAGE_KEY)
                .expect("the default package key is valid"),
        )),
        [command] if command == "play" => Ok(Command::Play(
            ScenarioRoot::try_for_package_key(DEFAULT_SCENARIO_PACKAGE_KEY)
                .expect("the default package key is valid"),
        )),
        [command, flag] if command == "play" && (flag == "-h" || flag == "--help") => {
            Ok(Command::Help)
        }
        [command, package_key] if command == "play" => {
            ScenarioRoot::try_for_package_key(package_key.clone())
                .map(Command::Play)
                .map_err(|error| {
                    UsageError(format!("invalid package key `{package_key}`: {error}"))
                })
        }
        [command, ..] if command == "play" => Err(UsageError(
            "play accepts at most one package key".to_owned(),
        )),
        [flag] if flag == "-h" || flag == "--help" => Ok(Command::Help),
        [command, flag] if command == "validate-scenario" && (flag == "-h" || flag == "--help") => {
            Ok(Command::Help)
        }
        [command, flag] if command == "map-report" && (flag == "-h" || flag == "--help") => {
            Ok(Command::Help)
        }
        [command, flag] if command == "dialogue-report" && (flag == "-h" || flag == "--help") => {
            Ok(Command::Help)
        }
        [command, flag] if command == "map-sweep" && (flag == "-h" || flag == "--help") => {
            Ok(Command::Help)
        }
        [command, flag]
            if command == "import-python-save" && (flag == "-h" || flag == "--help") =>
        {
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
        [command] if command == "map-report" => Ok(Command::MapReport(
            ScenarioRoot::try_for_package_key(DEFAULT_SCENARIO_PACKAGE_KEY)
                .expect("the default package key is valid"),
        )),
        [command, package_key] if command == "map-report" => {
            ScenarioRoot::try_for_package_key(package_key.clone())
                .map(Command::MapReport)
                .map_err(|error| {
                    UsageError(format!("invalid package key `{package_key}`: {error}"))
                })
        }
        [command, ..] if command == "map-report" => Err(UsageError(
            "map-report accepts at most one package key".to_owned(),
        )),
        [command] if command == "map-sweep" => Ok(Command::MapSweep(
            ScenarioRoot::try_for_package_key(DEFAULT_SCENARIO_PACKAGE_KEY)
                .expect("the default package key is valid"),
        )),
        [command, package_key] if command == "map-sweep" => {
            ScenarioRoot::try_for_package_key(package_key.clone())
                .map(Command::MapSweep)
                .map_err(|error| {
                    UsageError(format!("invalid package key `{package_key}`: {error}"))
                })
        }
        [command, ..] if command == "map-sweep" => Err(UsageError(
            "map-sweep accepts at most one package key".to_owned(),
        )),
        [command] if command == "dialogue-report" => Ok(Command::DialogueReport(
            ScenarioRoot::try_for_package_key(DEFAULT_SCENARIO_PACKAGE_KEY)
                .expect("the default package key is valid"),
        )),
        [command, package_key] if command == "dialogue-report" => {
            ScenarioRoot::try_for_package_key(package_key.clone())
                .map(Command::DialogueReport)
                .map_err(|error| {
                    UsageError(format!("invalid package key `{package_key}`: {error}"))
                })
        }
        [command, ..] if command == "dialogue-report" => Err(UsageError(
            "dialogue-report accepts at most one package key".to_owned(),
        )),
        [command, rest @ ..] if command == "import-python-save" => {
            parse_import_python_arguments(rest).map(Command::ImportPython)
        }
        [command, ..] => Err(UsageError(format!("unknown command `{command}`"))),
    }
}

fn parse_import_python_arguments(
    arguments: &[String],
) -> Result<ImportPythonArguments, UsageError> {
    let Some(input) = arguments.first() else {
        return Err(UsageError("import-python-save requires INPUT".to_owned()));
    };
    if input.starts_with('-') {
        return Err(UsageError(
            "import-python-save INPUT must come first".to_owned(),
        ));
    }
    let mut slot = None;
    let mut package = DEFAULT_SCENARIO_PACKAGE_KEY.to_owned();
    let mut allow_unchecked = false;
    let mut replace = false;
    let mut index = 1;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--slot" => {
                index += 1;
                let value = arguments
                    .get(index)
                    .ok_or_else(|| UsageError("--slot requires a value".to_owned()))?;
                let parsed = value
                    .parse::<usize>()
                    .map_err(|_| UsageError("--slot must be an integer in 0..=100".to_owned()))?;
                if parsed > 100 || slot.replace(parsed).is_some() {
                    return Err(UsageError(
                        "--slot must appear once with a value in 0..=100".to_owned(),
                    ));
                }
            }
            "--package" => {
                index += 1;
                package = arguments
                    .get(index)
                    .ok_or_else(|| UsageError("--package requires a value".to_owned()))?
                    .clone();
            }
            "--allow-unchecked" if !allow_unchecked => allow_unchecked = true,
            "--replace" if !replace => replace = true,
            option => {
                return Err(UsageError(format!(
                    "unknown or repeated import-python-save option `{option}`"
                )));
            }
        }
        index += 1;
    }
    let slot = slot.ok_or_else(|| UsageError("import-python-save requires --slot".to_owned()))?;
    let root = ScenarioRoot::try_for_package_key(package.clone())
        .map_err(|error| UsageError(format!("invalid package key `{package}`: {error}")))?;
    Ok(ImportPythonArguments {
        input: PathBuf::from(input),
        slot,
        root,
        allow_unchecked,
        replace,
    })
}

fn run_python_import(
    collection_root: &Path,
    arguments: &ImportPythonArguments,
) -> Result<crate::python_save_import::ImportInstallResult, String> {
    let canonical_collection = collection_root
        .canonicalize()
        .map_err(|_| "scenario collection is unavailable".to_owned())?;
    let selected = collection_root.join(arguments.root.package_key());
    let metadata = fs::symlink_metadata(&selected)
        .map_err(|_| "selected scenario package is unavailable".to_owned())?;
    if metadata.file_type().is_symlink() {
        return Err("selected scenario package must not be a symbolic link".to_owned());
    }
    let package = selected
        .canonicalize()
        .map_err(|_| "selected scenario package cannot be resolved".to_owned())?;
    if !package.starts_with(&canonical_collection) || !package.is_dir() {
        return Err("selected scenario package resolves outside its collection".to_owned());
    }
    let source = fs::read(&arguments.input)
        .map_err(|error| format!("could not read Python save input: {error}"))?;
    let catalog = PythonImportCatalog::load(&package).map_err(|error| error.to_string())?;
    let timestamp = unix_timestamp_now().map_err(|error| error.to_string())?;
    let envelope = convert_python_save(&source, arguments.allow_unchecked, &catalog, timestamp)
        .map_err(|error| error.to_string())?;
    let save_root =
        resolve_save_directory(|name| std::env::var_os(name)).map_err(|error| error.to_string())?;
    let store = SaveStore::new(save_root);
    let destination = store
        .slot_path(arguments.slot)
        .map_err(|error| error.to_string())?;
    ensure_distinct_import_paths(&arguments.input, &destination)?;
    install_python_import(
        &store,
        arguments.slot,
        &envelope,
        arguments.replace,
        &catalog.balance,
    )
    .map_err(|error| error.to_string())
}

fn ensure_distinct_import_paths(input: &Path, destination: &Path) -> Result<(), String> {
    if destination.exists()
        && let (Ok(input), Ok(destination)) = (input.canonicalize(), destination.canonicalize())
        && input == destination
    {
        return Err(
            "Python input and native destination must be different files; choose another slot"
                .to_owned(),
        );
    }
    Ok(())
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

/// Why a package key could not be resolved to a usable scenario package directory.
struct PackageSelectionError {
    code: &'static str,
    message: &'static str,
}

/// Resolves `root`'s package key to a scenario package directory beneath `collection_root`,
/// rejecting anything that escapes the collection (a symlinked or `..`-traversing package key)
/// before any command reads from it. Shared by every scenario command so `validate-scenario` and
/// `map-report` reject the same package layouts the same way.
fn select_scenario_package(
    collection_root: &Path,
    root: &ScenarioRoot,
) -> Result<PathBuf, PackageSelectionError> {
    let Ok(canonical_collection) = collection_root.canonicalize() else {
        return Err(PackageSelectionError {
            code: "scenario.collection_unavailable",
            message: "scenario collection is unavailable",
        });
    };
    if !canonical_collection.is_dir() {
        return Err(PackageSelectionError {
            code: "scenario.collection_unavailable",
            message: "scenario collection is not a directory",
        });
    }

    let selected = collection_root.join(root.package_key());
    let Ok(metadata) = fs::symlink_metadata(&selected) else {
        return Err(PackageSelectionError {
            code: "scenario.package_unavailable",
            message: "selected scenario package is unavailable",
        });
    };
    if metadata.file_type().is_symlink() {
        return Err(PackageSelectionError {
            code: "scenario.package_escape",
            message: "selected scenario package must not be a symbolic link",
        });
    }
    let Ok(canonical_selected) = selected.canonicalize() else {
        return Err(PackageSelectionError {
            code: "scenario.package_unavailable",
            message: "selected scenario package cannot be resolved",
        });
    };
    if !canonical_selected.starts_with(&canonical_collection) {
        return Err(PackageSelectionError {
            code: "scenario.package_escape",
            message: "selected scenario package resolves outside the scenario collection",
        });
    }
    if !canonical_selected.is_dir() {
        return Err(PackageSelectionError {
            code: "scenario.package_unavailable",
            message: "selected scenario package is not a directory",
        });
    }

    Ok(canonical_selected)
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

/// Renders the informational `map-report` output: every map's TMX pairing (including the
/// numbered-segment "parent metadata" convention), NPCs, portal targets, referenced dialogue
/// ids, and any dangling findings, followed by a summary footer.
fn write_map_report(
    output: &mut impl Write,
    root: &ScenarioRoot,
    report: &MapReport,
) -> io::Result<()> {
    writeln!(output, "Map report")?;
    writeln!(output, "Package: {}", root.package_key())?;
    match (
        report.scenario_id.as_deref(),
        report.scenario_name.as_deref(),
    ) {
        (Some(id), Some(name)) => writeln!(output, "Scenario: {id} ({name})")?,
        _ => writeln!(output, "Scenario: unavailable")?,
    }
    if let Some(error) = &report.load_error {
        writeln!(output, "Load error: {error}")?;
    }
    writeln!(output)?;

    let mut maps_with_findings = 0usize;
    for entry in &report.entries {
        let heading = match &entry.name {
            Some(name) => format!("Map {} ({name})", entry.id),
            None => format!("Map {} (no metadata YAML)", entry.id),
        };
        writeln!(output, "{heading}")?;

        if entry.has_same_stem_tmx {
            writeln!(output, "  same-stem TMX: yes")?;
        } else if entry.segments.is_empty() {
            writeln!(output, "  same-stem TMX: no")?;
        } else {
            writeln!(
                output,
                "  same-stem TMX: no (parent metadata; segments: {})",
                entry.segments.join(", ")
            )?;
        }

        if entry.npcs.is_empty() {
            writeln!(output, "  NPCs: none")?;
        } else {
            writeln!(output, "  NPCs:")?;
            for npc in &entry.npcs {
                let gated = if npc.gated { " [gated]" } else { "" };
                let missing = if npc.dialogue_missing {
                    " [MISSING]"
                } else {
                    ""
                };
                writeln!(
                    output,
                    "    {} at ({}, {}) -> dialogue `{}`{missing}{gated}",
                    npc.id, npc.position.x, npc.position.y, npc.dialogue_id
                )?;
                if let Some(excuses) = &npc.excuses {
                    let excuses_missing = if npc.excuses_missing {
                        " [MISSING]"
                    } else {
                        ""
                    };
                    writeln!(
                        output,
                        "      excuses dialogue `{excuses}`{excuses_missing}"
                    )?;
                }
            }
        }

        if entry.portals.is_empty() {
            writeln!(output, "  Portals: none")?;
        } else {
            writeln!(output, "  Portals:")?;
            for portal in &entry.portals {
                let status = if portal.target_resolvable {
                    ""
                } else {
                    " [DANGLING]"
                };
                writeln!(
                    output,
                    "    -> {} at ({}, {}){status}",
                    portal.target_map, portal.target_position.x, portal.target_position.y
                )?;
            }
        }

        let dialogue_ids = entry.dialogue_ids();
        if dialogue_ids.is_empty() {
            writeln!(output, "  Dialogue refs: none")?;
        } else {
            writeln!(
                output,
                "  Dialogue refs: {}",
                dialogue_ids.into_iter().collect::<Vec<_>>().join(", ")
            )?;
        }

        let findings = entry.findings();
        if findings.is_empty() {
            writeln!(output, "  Findings: none")?;
        } else {
            maps_with_findings += 1;
            writeln!(output, "  Findings:")?;
            for finding in &findings {
                writeln!(output, "    - {finding}")?;
            }
        }
        writeln!(output)?;
    }

    writeln!(output, "Summary")?;
    writeln!(output, "Total maps: {}", report.entries.len())?;
    writeln!(output, "Fully resolvable: {}", report.resolvable_count())?;
    writeln!(output, "Maps with findings: {maps_with_findings}")?;
    Ok(())
}

/// Renders the informational `map-sweep` output: every TMX actually run through the production
/// TMX/TSX pipeline, collision projection, portal extraction, NPC spawn-set derivation, and
/// sign/item-box structural load, followed by a per-category finding summary footer.
fn write_map_sweep_report(
    output: &mut impl Write,
    root: &ScenarioRoot,
    report: &MapSweepReport,
) -> io::Result<()> {
    writeln!(output, "Map sweep")?;
    writeln!(output, "Package: {}", root.package_key())?;
    match (
        report.scenario_id.as_deref(),
        report.scenario_name.as_deref(),
    ) {
        (Some(id), Some(name)) => writeln!(output, "Scenario: {id} ({name})")?,
        _ => writeln!(output, "Scenario: unavailable")?,
    }
    if let Some(error) = &report.load_error {
        writeln!(output, "Load error: {error}")?;
    }
    writeln!(output)?;

    for entry in &report.entries {
        writeln!(output, "Map {}", entry.id)?;
        writeln!(
            output,
            "  portals={} signs={} item_boxes={} npc_flag_states={}",
            entry.portal_count, entry.sign_count, entry.item_box_count, entry.npc_flag_states
        )?;
        if entry.findings.is_empty() {
            writeln!(output, "  Findings: none")?;
        } else {
            writeln!(output, "  Findings:")?;
            for finding in &entry.findings {
                writeln!(
                    output,
                    "    - [{}] {}",
                    finding.category.label(),
                    finding.message
                )?;
            }
        }
        writeln!(output)?;
    }

    writeln!(output, "Summary")?;
    writeln!(output, "Total maps swept: {}", report.entries.len())?;
    writeln!(output, "Fully clean: {}", report.clean_count())?;
    writeln!(
        output,
        "Maps with findings: {}",
        report.maps_with_findings()
    )?;
    for category in [
        SweepCategory::Tmx,
        SweepCategory::Collision,
        SweepCategory::Npc,
        SweepCategory::Portal,
        SweepCategory::Object,
    ] {
        writeln!(
            output,
            "Findings ({}): {}",
            category.label(),
            report.category_count(category)
        )?;
    }
    Ok(())
}

/// Renders the informational `dialogue-report` output: every dialogue's entries with their
/// requires/excludes condition, reachability under the pinned Python engine's first-match rule,
/// line count, and side-effect inventory, followed by a summary footer. The two documented-dead
/// `ardel_fisherman` flavor entries are called out as accepted rather than as findings.
fn write_dialogue_report(
    output: &mut impl Write,
    root: &ScenarioRoot,
    report: &DialogueReport,
) -> io::Result<()> {
    writeln!(output, "Dialogue report")?;
    writeln!(output, "Package: {}", root.package_key())?;
    match (
        report.scenario_id.as_deref(),
        report.scenario_name.as_deref(),
    ) {
        (Some(id), Some(name)) => writeln!(output, "Scenario: {id} ({name})")?,
        _ => writeln!(output, "Scenario: unavailable")?,
    }
    if let Some(error) = &report.load_error {
        writeln!(output, "Load error: {error}")?;
    }
    writeln!(output)?;

    for document in &report.documents {
        let shape = match &document.shape {
            DialogueReportShape::Cutscene => "cutscene".to_owned(),
            DialogueReportShape::Entries(Some(kind)) => format!("{kind:?}").to_lowercase(),
            DialogueReportShape::Entries(None) => "entries (untyped)".to_owned(),
            DialogueReportShape::LinePool => "line pool".to_owned(),
        };
        writeln!(output, "Dialogue {} ({shape})", document.id)?;

        if let Some(pool_lines) = document.pool_line_count {
            writeln!(output, "  Lines: {pool_lines}")?;
        } else {
            writeln!(
                output,
                "  Entries: {} ({} flag(s) referenced{})",
                document.entries.len(),
                document.referenced_flag_count,
                if document.too_many_flags {
                    ", too many to enumerate"
                } else {
                    ""
                }
            )?;
            for entry in &document.entries {
                write_dialogue_entry(output, entry)?;
            }
        }

        let notes = document.notes();
        if notes.is_empty() {
            writeln!(output, "  Notes: none")?;
        } else {
            writeln!(output, "  Notes:")?;
            for note in &notes {
                writeln!(output, "    - {note}")?;
            }
        }
        writeln!(output)?;
    }

    writeln!(output, "Summary")?;
    writeln!(output, "Total dialogues: {}", report.documents.len())?;
    writeln!(output, "Fully clean: {}", report.clean_count())?;
    writeln!(
        output,
        "Dialogues with dead entries (documented-accepted): {}",
        report.documents_with_only_accepted_dead_entries()
    )?;
    writeln!(
        output,
        "Dialogues with dead entries (new finding): {}",
        report.documents_with_new_dead_entries()
    )?;
    writeln!(
        output,
        "Dialogues with zero-line entries: {}",
        report.documents_with_zero_line_entries()
    )?;
    writeln!(
        output,
        "Dialogues with informational notes: {}",
        report.documents_with_notes()
    )?;
    Ok(())
}

fn write_dialogue_entry(output: &mut impl Write, entry: &DialogueReportEntry) -> io::Result<()> {
    let status = match entry.reachability {
        DialogueReachability::Reachable => "reachable".to_owned(),
        DialogueReachability::Dead => "DEAD".to_owned(),
        DialogueReachability::DeadAccepted => "dead (documented-accepted)".to_owned(),
        DialogueReachability::GraphNode => "graph node".to_owned(),
        DialogueReachability::Unknown => "unknown (too many flags)".to_owned(),
    };
    let node = entry
        .node
        .as_deref()
        .map(|node| format!(" node=`{node}`"))
        .unwrap_or_default();
    writeln!(
        output,
        "    [{}]{node} requires={:?} excludes={:?} lines={} {status}",
        entry.index, entry.requires, entry.excludes, entry.line_count
    )?;

    let effects = &entry.effects;
    if !effects.is_empty() {
        let mut parts = Vec::new();
        if !effects.set_flags.is_empty() {
            parts.push(format!("set_flag={}", effects.set_flags.join(",")));
        }
        if !effects.unset_flags.is_empty() {
            parts.push(format!("unset_flag={}", effects.unset_flags.join(",")));
        }
        if !effects.unlock_flags.is_empty() {
            let unlocks = effects
                .unlock_flags
                .iter()
                .map(|(kind, id)| format!("{kind}:{id}"))
                .collect::<Vec<_>>()
                .join(",");
            parts.push(format!("unlock={unlocks}"));
        }
        if !effects.give_items.is_empty() {
            let items = effects
                .give_items
                .iter()
                .map(|(id, qty)| format!("{id}x{qty}"))
                .collect::<Vec<_>>()
                .join(",");
            parts.push(format!("give_items={items}"));
        }
        if let Some(member) = &effects.join_party {
            parts.push(format!("join_party={member}"));
        }
        if let Some(map) = &effects.transition_map {
            parts.push(format!("transition={map}"));
        }
        if let Some(shop) = effects.open_shop {
            parts.push(format!("open_shop={shop}"));
        }
        if effects.open_inn {
            parts.push("open_inn".to_owned());
        }
        if effects.open_apothecary {
            parts.push("open_apothecary".to_owned());
        }
        writeln!(output, "      effects: {}", parts.join("; "))?;
    }
    Ok(())
}

pub(crate) fn production_asset_base() -> PathBuf {
    let manifest_directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    match std::env::current_exe() {
        Ok(executable) => asset_base_for_executable(&executable, &manifest_directory),
        Err(_) => manifest_directory.join("assets"),
    }
}

fn production_scenario_collection() -> PathBuf {
    production_asset_base().join("scenarios")
}

/// Resolves the Bevy asset base through the same development/installed layout decision as the
/// filesystem validator's scenario collection.
///
/// The repository target directory may resolve outside the repository when it is a symlink. Its
/// canonical location still identifies that executable as a development build without consulting
/// the process working directory. A packaged executable copied elsewhere continues to use only
/// the `assets/` directory beside itself.
fn asset_base_for_executable(executable: &Path, manifest_directory: &Path) -> PathBuf {
    let development_target_directory = manifest_directory.join("target");
    let is_development_executable = path_is_within(executable, manifest_directory)
        || path_is_within(executable, &development_target_directory);

    if is_development_executable {
        manifest_directory.join("assets")
    } else {
        executable
            .parent()
            .unwrap_or(manifest_directory)
            .join("assets")
    }
}

fn path_is_within(path: &Path, directory: &Path) -> bool {
    path.starts_with(directory)
        || fs::canonicalize(path)
            .ok()
            .zip(fs::canonicalize(directory).ok())
            .is_some_and(|(path, directory)| path.starts_with(directory))
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
            asset_base_for_executable(&executable, &workspace).join("scenarios"),
            workspace.join("assets/scenarios")
        );
        assert_eq!(
            asset_base_for_executable(&executable, &workspace),
            workspace.join("assets")
        );
    }

    #[cfg(unix)]
    #[test]
    fn development_executable_behind_target_symlink_uses_manifest_assets() {
        use std::os::unix::fs::symlink;

        let temporary = TempCollection::new("source/rpg-s1");
        let manifest_directory = temporary.0.join("source/rpg-s1");
        let external_target = temporary.0.join("cache/target");
        let external_executable = external_target.join("debug/rpg-s1");
        fs::create_dir_all(external_executable.parent().unwrap())
            .expect("external target should be creatable");
        fs::write(&external_executable, b"invented executable")
            .expect("external executable should be creatable");
        symlink(&external_target, manifest_directory.join("target"))
            .expect("repository target symlink should be creatable");
        let resolved_executable = fs::canonicalize(manifest_directory.join("target/debug/rpg-s1"))
            .expect("target executable should resolve through the symlink");

        assert_eq!(
            asset_base_for_executable(&resolved_executable, &manifest_directory),
            manifest_directory.join("assets")
        );
    }

    #[test]
    fn installed_executable_uses_only_assets_beside_its_package_binary() {
        let base = std::env::temp_dir().join("invented-installed-layout");
        let manifest_directory = base.join("source/rpg-s1");
        let executable = base.join("package/rpg-s1");

        assert_eq!(
            asset_base_for_executable(&executable, &manifest_directory).join("scenarios"),
            base.join("package/assets/scenarios")
        );
        assert_eq!(
            asset_base_for_executable(&executable, &manifest_directory),
            base.join("package/assets")
        );
    }

    #[test]
    fn collection_resolution_is_absolute_and_has_no_working_directory_input() {
        let base = std::env::temp_dir().join("invented-cwd-independent-layout");
        let manifest_directory = base.join("source/rpg-s1");
        let executable = base.join("package/rpg-s1");

        let collection =
            asset_base_for_executable(&executable, &manifest_directory).join("scenarios");

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
            asset_base_for_executable(&executable, &manifest_directory).join("scenarios"),
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
            |_| launched.set(true),
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
            |_| launched.set(true),
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

    /// A manifest-only scenario, sufficient for `build_map_report` to load a scenario identity
    /// with zero map entries. Deep per-map report content (segments, portals, NPCs, dialogue) is
    /// covered by `scenario_map_report`'s own tests; these CLI tests only check the command is
    /// wired up and always exits successfully.
    fn write_manifest_only_scenario(collection: &Path, package_key: &str) {
        let manifest = r#"id: invented_story
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
bootstrap_flags: []
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
"#;
        fs::write(collection.join(package_key).join("manifest.yaml"), manifest)
            .expect("temporary manifest should be writable");
    }

    #[test]
    fn map_report_command_does_not_invoke_the_bevy_app_launcher() {
        let collection = TempCollection::new("fixture");
        write_manifest_only_scenario(&collection.0, "fixture");
        let launched = Cell::new(false);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run_with(
            ["map-report".into(), "fixture".into()],
            &collection.0,
            valid_report,
            &mut stdout,
            &mut stderr,
            |_| launched.set(true),
        );

        assert_eq!(exit, EXIT_SUCCESS);
        assert!(
            !launched.get(),
            "map-report must not construct the Bevy app"
        );
        assert!(stderr.is_empty());
        let output = String::from_utf8(stdout).unwrap();
        assert!(output.contains("Map report\n"));
        assert!(output.contains("Package: fixture\n"));
        assert!(output.contains("Scenario: invented_story (Invented Story)\n"));
        assert!(output.contains("Total maps: 0\n"));
        assert!(output.contains("Fully resolvable: 0\n"));
        assert!(output.contains("Maps with findings: 0\n"));
    }

    /// A TMX-only map (a same-stem TMX with no `data/maps/<id>.yaml`, exactly the shape of
    /// `zone_02_open_plains_cave_01`/`_cave_02`) must render exactly one "Dialogue refs:" line
    /// and exactly one "Findings:" line, not a duplicate pair.
    #[test]
    fn map_report_renders_a_single_dialogue_refs_and_findings_line_for_a_tmx_only_map() {
        let collection = TempCollection::new("caves");
        write_manifest_only_scenario(&collection.0, "caves");
        fs::create_dir_all(collection.0.join("caves/assets/maps"))
            .expect("temporary tmx directory should be creatable");
        fs::write(
            collection.0.join("caves/assets/maps/cave_only.tmx"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<map version="1.10" tiledversion="1.10.2" orientation="orthogonal" renderorder="right-down" width="1" height="1" tilewidth="32" tileheight="32" infinite="0" nextlayerid="1" nextobjectid="1">
</map>
"#,
        )
        .expect("temporary tmx-only map should be writable");
        let root = ScenarioRoot::try_for_package_key("caves").unwrap();
        let report = build_map_report(&collection.0.join("caves"));

        let mut stdout = Vec::new();
        write_map_report(&mut stdout, &root, &report).expect("map report should render");
        let output = String::from_utf8(stdout).unwrap();

        let block_start = output
            .find("Map cave_only (no metadata YAML)\n")
            .expect("tmx-only map heading should be present");
        let block = &output[block_start..];
        let block_end = block.find("\n\n").map_or(block.len(), |offset| offset + 1);
        let block = &block[..block_end];

        assert_eq!(
            block.matches("  Dialogue refs:").count(),
            1,
            "expected exactly one Dialogue refs line in:\n{block}"
        );
        assert_eq!(
            block.matches("  Findings:").count(),
            1,
            "expected exactly one Findings line in:\n{block}"
        );
        assert!(block.contains("  Dialogue refs: none\n"));
        assert!(block.contains("  Findings: none\n"));
    }

    #[test]
    fn map_report_default_selects_the_default_package_key() {
        let collection = TempCollection::new(DEFAULT_SCENARIO_PACKAGE_KEY);
        write_manifest_only_scenario(&collection.0, DEFAULT_SCENARIO_PACKAGE_KEY);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run_with(
            ["map-report".into()],
            &collection.0,
            valid_report,
            &mut stdout,
            &mut stderr,
            |_| panic!("map-report must not launch Bevy"),
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
    fn map_report_on_an_unavailable_package_still_exits_successfully() {
        let collection = TempCollection::new("present");
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run_with(
            ["map-report".into(), "missing".into()],
            &collection.0,
            |_, _| panic!("an unavailable package must not reach the validator"),
            &mut stdout,
            &mut stderr,
            |_| panic!("map-report must not launch Bevy"),
        );

        assert_eq!(
            exit, EXIT_SUCCESS,
            "map-report is informational and always exits zero"
        );
        let output = String::from_utf8(stdout).unwrap();
        assert!(output.contains("Load error: selected scenario package is unavailable\n"));
        assert!(output.contains("Total maps: 0\n"));
        assert!(!output.contains(collection.0.to_string_lossy().as_ref()));
        assert!(stderr.is_empty());
    }

    #[test]
    fn map_sweep_command_does_not_invoke_the_bevy_app_launcher() {
        let collection = TempCollection::new("fixture");
        write_manifest_only_scenario(&collection.0, "fixture");
        let launched = Cell::new(false);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run_with(
            ["map-sweep".into(), "fixture".into()],
            &collection.0,
            valid_report,
            &mut stdout,
            &mut stderr,
            |_| launched.set(true),
        );

        assert_eq!(exit, EXIT_SUCCESS);
        assert!(!launched.get(), "map-sweep must not construct the Bevy app");
        assert!(stderr.is_empty());
        let output = String::from_utf8(stdout).unwrap();
        assert!(output.contains("Map sweep\n"));
        assert!(output.contains("Package: fixture\n"));
        assert!(output.contains("Scenario: invented_story (Invented Story)\n"));
        assert!(output.contains("Total maps swept: 0\n"));
        assert!(output.contains("Fully clean: 0\n"));
        assert!(output.contains("Maps with findings: 0\n"));
        assert!(output.contains("Findings (tmx): 0\n"));
        assert!(output.contains("Findings (collision): 0\n"));
        assert!(output.contains("Findings (npc): 0\n"));
        assert!(output.contains("Findings (portal): 0\n"));
        assert!(output.contains("Findings (object): 0\n"));
    }

    #[test]
    fn map_sweep_default_selects_the_default_package_key() {
        let collection = TempCollection::new(DEFAULT_SCENARIO_PACKAGE_KEY);
        write_manifest_only_scenario(&collection.0, DEFAULT_SCENARIO_PACKAGE_KEY);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run_with(
            ["map-sweep".into()],
            &collection.0,
            valid_report,
            &mut stdout,
            &mut stderr,
            |_| panic!("map-sweep must not launch Bevy"),
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
    fn map_sweep_on_an_unavailable_package_still_exits_successfully() {
        let collection = TempCollection::new("present");
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run_with(
            ["map-sweep".into(), "missing".into()],
            &collection.0,
            |_, _| panic!("an unavailable package must not reach the validator"),
            &mut stdout,
            &mut stderr,
            |_| panic!("map-sweep must not launch Bevy"),
        );

        assert_eq!(
            exit, EXIT_SUCCESS,
            "map-sweep is informational and always exits zero"
        );
        let output = String::from_utf8(stdout).unwrap();
        assert!(output.contains("Load error: selected scenario package is unavailable\n"));
        assert!(output.contains("Total maps swept: 0\n"));
        assert!(!output.contains(collection.0.to_string_lossy().as_ref()));
        assert!(stderr.is_empty());
    }

    #[test]
    fn dialogue_report_command_does_not_invoke_the_bevy_app_launcher() {
        let collection = TempCollection::new("fixture");
        write_manifest_only_scenario(&collection.0, "fixture");
        fs::create_dir_all(collection.0.join("fixture/data/dialogue"))
            .expect("temporary dialogue directory should be creatable");
        fs::write(
            collection.0.join("fixture/data/dialogue/guide.yaml"),
            "id: guide\ntype: npc\nentries:\n  - lines: [\"Hello.\"]\n",
        )
        .expect("temporary dialogue fixture should be writable");
        let launched = Cell::new(false);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run_with(
            ["dialogue-report".into(), "fixture".into()],
            &collection.0,
            valid_report,
            &mut stdout,
            &mut stderr,
            |_| launched.set(true),
        );

        assert_eq!(exit, EXIT_SUCCESS);
        assert!(
            !launched.get(),
            "dialogue-report must not construct the Bevy app"
        );
        assert!(stderr.is_empty());
        let output = String::from_utf8(stdout).unwrap();
        assert!(output.contains("Dialogue report\n"));
        assert!(output.contains("Package: fixture\n"));
        assert!(output.contains("Scenario: invented_story (Invented Story)\n"));
        assert!(output.contains("Dialogue guide (npc)\n"));
        assert!(output.contains("Total dialogues: 1\n"));
        assert!(output.contains("Fully clean: 1\n"));
        assert!(output.contains("Dialogues with dead entries (documented-accepted): 0\n"));
        assert!(output.contains("Dialogues with dead entries (new finding): 0\n"));
    }

    #[test]
    fn dialogue_report_default_selects_the_default_package_key() {
        let collection = TempCollection::new(DEFAULT_SCENARIO_PACKAGE_KEY);
        write_manifest_only_scenario(&collection.0, DEFAULT_SCENARIO_PACKAGE_KEY);
        fs::create_dir_all(
            collection
                .0
                .join(format!("{DEFAULT_SCENARIO_PACKAGE_KEY}/data/dialogue")),
        )
        .expect("temporary dialogue directory should be creatable");
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run_with(
            ["dialogue-report".into()],
            &collection.0,
            valid_report,
            &mut stdout,
            &mut stderr,
            |_| panic!("dialogue-report must not launch Bevy"),
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
    fn dialogue_report_on_an_unavailable_package_still_exits_successfully() {
        let collection = TempCollection::new("present");
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run_with(
            ["dialogue-report".into(), "missing".into()],
            &collection.0,
            |_, _| panic!("an unavailable package must not reach the validator"),
            &mut stdout,
            &mut stderr,
            |_| panic!("dialogue-report must not launch Bevy"),
        );

        assert_eq!(
            exit, EXIT_SUCCESS,
            "dialogue-report is informational and always exits zero"
        );
        let output = String::from_utf8(stdout).unwrap();
        assert!(output.contains("Load error: selected scenario package is unavailable\n"));
        assert!(output.contains("Total dialogues: 0\n"));
        assert!(!output.contains(collection.0.to_string_lossy().as_ref()));
        assert!(stderr.is_empty());
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
            |_| panic!("validation must not launch Bevy"),
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
            |_| panic!("validation must not launch Bevy"),
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
            |_| panic!("shared validation must not launch Bevy"),
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
            vec!["map-report".into(), "../escape".into()],
            vec!["map-report".into(), "one".into(), "two".into()],
            vec!["map-sweep".into(), "../escape".into()],
            vec!["map-sweep".into(), "one".into(), "two".into()],
            vec!["dialogue-report".into(), "../escape".into()],
            vec!["dialogue-report".into(), "one".into(), "two".into()],
            vec!["import-python-save".into()],
            vec!["import-python-save".into(), "save.yaml".into()],
            vec![
                "import-python-save".into(),
                "save.yaml".into(),
                "--slot".into(),
                "101".into(),
            ],
        ] {
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            let exit = run_with(
                arguments,
                Path::new("unused"),
                |_, _| panic!("misuse must not validate"),
                &mut stdout,
                &mut stderr,
                |_| panic!("misuse must not launch Bevy"),
            );
            assert_eq!(exit, EXIT_USAGE);
            assert!(stdout.is_empty());
            assert!(String::from_utf8(stderr).unwrap().contains("Usage:"));
        }
    }

    #[test]
    fn python_import_arguments_are_explicit_and_bounded() {
        let Command::ImportPython(arguments) = parse_command([
            "import-python-save".into(),
            "legacy/007.yaml".into(),
            "--slot".into(),
            "7".into(),
            "--package".into(),
            "rusted_kingdoms".into(),
            "--allow-unchecked".into(),
            "--replace".into(),
        ])
        .unwrap() else {
            panic!("import command must select the converter");
        };
        assert_eq!(arguments.input, Path::new("legacy/007.yaml"));
        assert_eq!(arguments.slot, 7);
        assert_eq!(arguments.root.package_key(), "rusted_kingdoms");
        assert!(arguments.allow_unchecked);
        assert!(arguments.replace);
    }

    #[test]
    fn python_import_cannot_replace_its_own_input_path() {
        let temporary = TempCollection::new("paths");
        let input = temporary.0.join("007.yaml");
        let other = temporary.0.join("008.yaml");
        fs::write(&input, b"legacy").unwrap();
        fs::write(&other, b"native").unwrap();

        assert!(ensure_distinct_import_paths(&input, &input).is_err());
        assert_eq!(ensure_distinct_import_paths(&input, &other), Ok(()));
    }

    #[test]
    fn no_subcommand_launches_the_default_scenario() {
        let launched = Cell::new(false);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let exit = run_with(
            [],
            Path::new("unused"),
            |_, _| panic!("play mode must not validate"),
            &mut stdout,
            &mut stderr,
            |root| {
                assert_eq!(root.package_key(), DEFAULT_SCENARIO_PACKAGE_KEY);
                launched.set(true);
            },
        );

        assert_eq!(exit, EXIT_SUCCESS);
        assert!(launched.get());
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
    }

    #[test]
    fn play_subcommand_launches_the_selected_scenario_without_validation() {
        let launched = Cell::new(false);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let exit = run_with(
            ["play".into(), "invented_campaign".into()],
            Path::new("unused"),
            |_, _| panic!("play mode must not validate"),
            &mut stdout,
            &mut stderr,
            |root| {
                assert_eq!(root.package_key(), "invented_campaign");
                launched.set(true);
            },
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
