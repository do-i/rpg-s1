use super::{args::*, execute::*, report::*, *};
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    debug_launch::{DebugBattleFixture, DebugPartyPreset},
    gameplay_rng::DEFAULT_GAMEPLAY_SEED,
    input_record::{InputAutomation, InputRecord, RecordSource},
    scenario_cross_reference::{
        DiagnosticSeverity, ScenarioCatalogCounts, ScenarioDiagnostic, ScenarioLocation,
        ScenarioValidationReport,
    },
    scenario_map_report::build_map_report,
    scenario_path::ScenarioRelativePath,
    scenario_root::{DEFAULT_SCENARIO_PACKAGE_KEY, ScenarioRoot},
};
use std::{
    cell::Cell,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

pub(super) struct TempCollection(pub(super) PathBuf);

impl TempCollection {
    pub(super) fn new(package_key: &str) -> Self {
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

/// Runs `validate-scenario` against `invalid_report` with a baseline file of the given text.
fn run_validation_against_baseline(contents: &str) -> (u8, String, String) {
    let collection = TempCollection::new("broken");
    let baseline = collection.0.join("baseline.txt");
    fs::write(&baseline, contents).expect("baseline file should be writable");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    let exit = run_with(
        [
            "validate-scenario".into(),
            "broken".into(),
            "--baseline".into(),
            baseline.clone().into(),
        ],
        &collection.0,
        invalid_report,
        &mut stdout,
        &mut stderr,
        |_| panic!("validation must never launch the app"),
    );

    (
        exit,
        String::from_utf8(stdout).unwrap(),
        String::from_utf8(stderr).unwrap(),
    )
}

const ACCEPTED_DIAGNOSTICS: &str = "\
error broken:data/maps/a.yaml#npcs[0].dialogue [reference.missing] unknown dialogue id `lost`
warning broken:data/maps/z.yaml#name [content.warning] later warning
";

#[test]
fn a_report_matching_its_baseline_exits_zero_despite_standing_errors() {
    let (exit, output, errors) = run_validation_against_baseline(ACCEPTED_DIAGNOSTICS);

    // The point of the gate: the report itself still says FAIL, because these diagnostics are
    // real and accepted inherited debt, yet CI can run the command and get a usable signal.
    assert_eq!(exit, EXIT_SUCCESS);
    assert!(output.contains("Status: FAIL\n"));
    assert!(
        output.contains("Baseline status: PASS (2 accepted, 0 new, 0 no longer reported)\n"),
        "{output}"
    );
    assert!(errors.is_empty());
}

#[test]
fn a_diagnostic_the_baseline_omits_fails_the_gate() {
    let (exit, output, _) = run_validation_against_baseline(
        "warning broken:data/maps/z.yaml#name [content.warning] later warning\n",
    );

    assert_eq!(exit, EXIT_VALIDATION_FAILED);
    assert!(
        output.contains("Baseline status: FAIL (1 accepted, 1 new, 0 no longer reported)\n"),
        "{output}"
    );
    assert!(
            output.contains(
                "new error broken:data/maps/a.yaml#npcs[0].dialogue [reference.missing] unknown dialogue id `lost`\n"
            ),
            "{output}"
        );
}

#[test]
fn a_baseline_line_the_report_no_longer_produces_fails_the_gate() {
    let stale = format!(
        "{ACCEPTED_DIAGNOSTICS}error broken:data/maps/gone.yaml#id [reference.missing] already repaired\n"
    );

    let (exit, output, _) = run_validation_against_baseline(&stale);

    assert_eq!(exit, EXIT_VALIDATION_FAILED);
    assert!(
        output.contains("Baseline status: FAIL (2 accepted, 0 new, 1 no longer reported)\n"),
        "{output}"
    );
    assert!(
            output.contains(
                "no-longer-reported error broken:data/maps/gone.yaml#id [reference.missing] already repaired\n"
            ),
            "{output}"
        );
}

#[test]
fn baseline_comments_and_blank_lines_are_not_accepted_diagnostics() {
    let annotated = format!("# why these are accepted\n\n{ACCEPTED_DIAGNOSTICS}\n");

    let (exit, output, _) = run_validation_against_baseline(&annotated);

    assert_eq!(exit, EXIT_SUCCESS);
    assert!(
        output.contains("Baseline status: PASS (2 accepted"),
        "{output}"
    );
}

#[test]
fn an_unreadable_baseline_is_a_usage_error_not_a_silent_pass() {
    let collection = TempCollection::new("broken");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    let exit = run_with(
        [
            "validate-scenario".into(),
            "broken".into(),
            "--baseline".into(),
            collection.0.join("absent.txt").into(),
        ],
        &collection.0,
        invalid_report,
        &mut stdout,
        &mut stderr,
        |_| panic!("validation must never launch the app"),
    );

    assert_eq!(exit, EXIT_USAGE);
    assert!(
        String::from_utf8(stderr)
            .unwrap()
            .contains("cannot read baseline"),
        "a missing baseline must be reported, never treated as an empty accepted set"
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
  tmx: media/maps/
"#;
    fs::write(collection.join(package_key).join("manifest.yaml"), manifest)
        .expect("temporary manifest should be writable");
    fs::create_dir_all(collection.join(package_key).join("media/maps"))
        .expect("temporary empty TMX directory should be creatable");
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
    fs::create_dir_all(collection.0.join("caves/media/maps"))
        .expect("temporary tmx directory should be creatable");
    fs::write(
            collection.0.join("caves/media/maps/cave_only.tmx"),
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
fn map_sweep_on_an_unavailable_package_fails_the_runtime_phase() {
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

    assert_eq!(exit, EXIT_VALIDATION_FAILED);
    let output = String::from_utf8(stdout).unwrap();
    assert!(output.contains("Load error: selected scenario package is unavailable\n"));
    assert!(output.contains("Total maps swept: 0\n"));
    assert!(output.contains("Runtime status: FAIL (0 passed, 0 failed)\n"));
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
        vec!["record".into()],
        vec!["record".into(), "--seed".into(), "1".into()],
        vec!["replay".into()],
        vec!["replay".into(), "one".into(), "two".into()],
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
        |arguments| {
            assert_eq!(arguments.root.package_key(), DEFAULT_SCENARIO_PACKAGE_KEY);
            assert_eq!(arguments.seed, DEFAULT_GAMEPLAY_SEED);
            launched.set(true);
        },
    );

    assert_eq!(exit, EXIT_SUCCESS);
    assert!(launched.get());
    assert_eq!(String::from_utf8(stdout).unwrap(), "Gameplay seed: 1\n");
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
        |arguments| {
            assert_eq!(arguments.root.package_key(), "invented_campaign");
            assert_eq!(arguments.seed, DEFAULT_GAMEPLAY_SEED);
            launched.set(true);
        },
    );

    assert_eq!(exit, EXIT_SUCCESS);
    assert!(launched.get());
    assert_eq!(String::from_utf8(stdout).unwrap(), "Gameplay seed: 1\n");
    assert!(stderr.is_empty());
}

#[test]
fn play_seed_is_bounded_to_u64_and_reaches_the_launcher() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit = run_with(
        ["play".into(), "--seed".into(), u64::MAX.to_string().into()],
        Path::new("unused"),
        |_, _| panic!("play mode must not validate"),
        &mut stdout,
        &mut stderr,
        |arguments| {
            assert_eq!(arguments.root.package_key(), DEFAULT_SCENARIO_PACKAGE_KEY);
            assert_eq!(arguments.seed, u64::MAX);
        },
    );

    assert_eq!(exit, EXIT_SUCCESS);
    assert_eq!(
        String::from_utf8(stdout).unwrap(),
        format!("Gameplay seed: {}\n", u64::MAX)
    );
    assert!(stderr.is_empty());

    for arguments in [
        vec!["play".into(), "--seed".into()],
        vec!["play".into(), "--seed".into(), "-1".into()],
        vec!["play".into(), "--seed".into(), "not-a-seed".into()],
        vec![
            "play".into(),
            "--seed".into(),
            "1".into(),
            "--seed".into(),
            "2".into(),
        ],
    ] {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        assert_eq!(
            run_with(
                arguments,
                Path::new("unused"),
                |_, _| panic!("misuse must not validate"),
                &mut stdout,
                &mut stderr,
                |_| panic!("misuse must not launch Bevy"),
            ),
            EXIT_USAGE
        );
        assert!(stdout.is_empty());
        assert!(String::from_utf8(stderr).unwrap().contains("Usage:"));
    }
}

/// Builds the environment lookup the production call site injects, without
/// touching the real process environment.
fn fake_environment(value: Option<&str>) -> impl Fn(&str) -> Option<OsString> + use<> {
    let value = value.map(OsString::from);
    move |name| {
        (name == PARTY_PRESET_ENV_VAR)
            .then(|| value.clone())
            .flatten()
    }
}

#[test]
fn the_party_preset_env_var_starts_a_bare_launch_directly_in_the_world() {
    // A bare `rpg-s1` has no debug config at all until the environment supplies one.
    let mut command = parse_command(Vec::<OsString>::new()).unwrap();
    let Command::Play(before) = &command else {
        unreachable!()
    };
    assert!(before.debug.is_none());

    apply_environment_debug_defaults(&mut command, fake_environment(Some("full"))).unwrap();
    let Command::Play(after) = &command else {
        unreachable!()
    };
    let debug = after.debug.as_ref().expect("the env var activates debug");
    assert_eq!(debug.party_preset, Some(DebugPartyPreset::Full));
    // `run_game` keys "skip the title screen" off `debug.is_some()`.
    assert!(debug.is_active());
}

#[test]
fn an_explicit_party_preset_flag_beats_the_env_var() {
    let mut command =
        parse_command(["play".into(), "--party-preset".into(), "solo".into()]).unwrap();
    apply_environment_debug_defaults(&mut command, fake_environment(Some("full"))).unwrap();
    let Command::Play(play) = &command else {
        unreachable!()
    };
    assert_eq!(
        play.debug.as_ref().unwrap().party_preset,
        Some(DebugPartyPreset::Solo)
    );
}

#[test]
fn battle_fixtures_supply_an_isolated_map_and_required_party_shape() {
    for (name, fixture, preset) in [
        (
            "feedback",
            DebugBattleFixture::Feedback,
            DebugPartyPreset::Full,
        ),
        ("status", DebugBattleFixture::Status, DebugPartyPreset::Full),
        (
            "enemy-ai",
            DebugBattleFixture::EnemyAi,
            DebugPartyPreset::Solo,
        ),
        (
            "rewards",
            DebugBattleFixture::Rewards,
            DebugPartyPreset::Solo,
        ),
    ] {
        let Command::Play(play) =
            parse_command(["play".into(), "--battle-fixture".into(), name.into()]).unwrap()
        else {
            unreachable!()
        };
        let debug = play.debug.unwrap();
        assert_eq!(debug.battle_fixture, Some(fixture));
        assert_eq!(debug.party_preset, Some(preset));
        assert_eq!(debug.start_map.as_deref(), Some("zone_01_starting_forest"));
        assert_eq!(
            debug.start_position,
            Some(crate::scenario_spatial::Position::new(29, 1))
        );
    }
}

#[test]
fn a_battle_fixture_rejects_a_party_shape_that_would_invalidate_it() {
    let error = match parse_command([
        "play".into(),
        "--battle-fixture".into(),
        "rewards".into(),
        "--party-preset".into(),
        "full".into(),
    ]) {
        Ok(_) => panic!("an incompatible explicit party shape should fail"),
        Err(error) => error,
    };
    assert!(error.0.contains("requires --party-preset solo"));
}

#[test]
fn the_party_preset_env_var_leaves_replay_and_absent_values_alone() {
    // Replay must stay faithful to its record regardless of the environment.
    let mut replay = parse_command(["replay".into(), "trace.yaml".into()]).unwrap();
    apply_environment_debug_defaults(&mut replay, fake_environment(Some("full"))).unwrap();
    let Command::Replay(input) = &replay else {
        panic!("replay must stay a replay command");
    };
    assert_eq!(input, &PathBuf::from("trace.yaml"));

    let mut command = parse_command(["play".into()]).unwrap();
    apply_environment_debug_defaults(&mut command, fake_environment(None)).unwrap();
    let Command::Play(play) = &command else {
        unreachable!()
    };
    assert!(play.debug.is_none(), "an unset env var changes nothing");
}

#[test]
fn a_recorded_session_captures_the_env_var_preset_it_ran_with() {
    let mut command =
        parse_command(["record".into(), "trace.yaml".into(), "invented".into()]).unwrap();
    apply_environment_debug_defaults(&mut command, fake_environment(Some("full"))).unwrap();
    let Command::Record { play, .. } = &command else {
        unreachable!()
    };
    assert_eq!(
        play.debug.as_ref().unwrap().party_preset,
        Some(DebugPartyPreset::Full)
    );
}

#[test]
fn an_unreadable_party_preset_env_var_is_a_usage_error() {
    let mut command = parse_command(["play".into()]).unwrap();
    let error = apply_environment_debug_defaults(&mut command, fake_environment(Some("everyone")))
        .unwrap_err();
    assert!(error.0.contains("requires solo|full"), "got {}", error.0);
    // Whitespace from a shell export is tolerated rather than rejected.
    let mut padded = parse_command(["play".into()]).unwrap();
    apply_environment_debug_defaults(&mut padded, fake_environment(Some(" full "))).unwrap();
    let Command::Play(play) = &padded else {
        unreachable!()
    };
    assert_eq!(
        play.debug.as_ref().unwrap().party_preset,
        Some(DebugPartyPreset::Full)
    );
}

#[test]
fn debug_play_options_are_typed_repeatable_and_isolated_from_normal_play() {
    let Command::Play(arguments) = parse_command([
        "play".into(),
        "invented".into(),
        "--start-map".into(),
        "town_01".into(),
        "--start-position".into(),
        "10,12".into(),
        "--party-preset".into(),
        "full".into(),
        "--timings".into(),
        "--set-flag".into(),
        "quest_ready".into(),
        "--unset-flag".into(),
        "quest_hidden".into(),
    ])
    .unwrap() else {
        panic!("debug options should remain a play command");
    };
    let debug = arguments.debug.unwrap();
    assert_eq!(arguments.root.package_key(), "invented");
    assert_eq!(debug.start_map.as_deref(), Some("town_01"));
    assert_eq!(
        debug.start_position,
        Some(crate::scenario_spatial::Position::new(10, 12))
    );
    assert_eq!(debug.party_preset, Some(DebugPartyPreset::Full));
    assert!(arguments.timings);
    assert_eq!(debug.flag_overrides.get("quest_ready"), Some(&true));
    assert_eq!(debug.flag_overrides.get("quest_hidden"), Some(&false));

    let Command::Play(normal) = parse_command(["play".into()]).unwrap() else {
        unreachable!()
    };
    assert!(normal.debug.is_none());

    for invalid in [
        vec!["play".into(), "--start-map".into(), "town_01".into()],
        vec!["play".into(), "--start-position".into(), "10,12".into()],
        vec!["play".into(), "--party-preset".into(), "unknown".into()],
        vec!["play".into(), "--set-flag".into(), "bad flag".into()],
        vec![
            "play".into(),
            "--set-flag".into(),
            "same".into(),
            "--unset-flag".into(),
            "same".into(),
        ],
    ] {
        assert!(parse_command(invalid).is_err());
    }
}

#[test]
fn record_and_replay_commands_validate_identity_before_launching() {
    let collection = TempCollection::new("invented");
    fs::write(
        collection.0.join("invented/manifest.yaml"),
        "id: invented_story\nname: Invented Story\nversion: 2.3.4\nwindow_title: Invented\n",
    )
    .unwrap();
    let output_path = collection.0.join("trace.yaml");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let recorded = Cell::new(false);
    let exit = run_with_version(
        [
            OsString::from("record"),
            output_path.as_os_str().to_owned(),
            OsString::from("invented"),
            OsString::from("--seed"),
            OsString::from("77"),
        ],
        "2026.8.1",
        &collection.0,
        |_, _| panic!("record must not invoke full validation"),
        &mut stdout,
        &mut stderr,
        |arguments| {
            assert_eq!(arguments.root.package_key(), "invented");
            assert_eq!(arguments.seed, 77);
            let Some(InputAutomation::Record { record, .. }) = arguments.automation else {
                panic!("record mode should reach the launcher");
            };
            assert_eq!(record.game_version, "2026.8.1");
            assert_eq!(record.source.scenario_id, "invented_story");
            assert_eq!(record.source.scenario_version, "2.3.4");
            recorded.set(true);
        },
    );
    assert_eq!(exit, EXIT_SUCCESS);
    assert!(recorded.get());
    assert!(stderr.is_empty());
    assert!(
        String::from_utf8(stdout)
            .unwrap()
            .contains("Recording normalized actions")
    );
    let disk_record = InputRecord::decode(&fs::read(&output_path).unwrap()).unwrap();
    assert_eq!(disk_record.seed, 77);

    let replayed = Cell::new(false);
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit = run_with_version(
        [OsString::from("replay"), output_path.as_os_str().to_owned()],
        "2026.8.1",
        &collection.0,
        |_, _| panic!("replay must not invoke full validation"),
        &mut stdout,
        &mut stderr,
        |arguments| {
            assert_eq!(arguments.root.package_key(), "invented");
            assert_eq!(arguments.seed, 77);
            assert!(matches!(
                arguments.automation,
                Some(InputAutomation::Replay { .. })
            ));
            replayed.set(true);
        },
    );
    assert_eq!(exit, EXIT_SUCCESS);
    assert!(replayed.get());
    assert!(stderr.is_empty());
    assert!(
        String::from_utf8(stdout)
            .unwrap()
            .contains("Replaying normalized actions")
    );
}

#[test]
fn record_refuses_overwrite_and_replay_rejects_version_or_source_drift() {
    let collection = TempCollection::new("invented");
    let manifest_path = collection.0.join("invented/manifest.yaml");
    fs::write(
        &manifest_path,
        "id: invented_story\nname: Invented Story\nversion: 2.3.4\nwindow_title: Invented\n",
    )
    .unwrap();
    let output_path = collection.0.join("trace.yaml");
    let record = InputRecord::new(
        "2026.8.1",
        RecordSource {
            package_key: "invented".to_owned(),
            scenario_id: "invented_story".to_owned(),
            scenario_version: "2.3.4".to_owned(),
        },
        1,
    );
    fs::write(&output_path, record.encode().unwrap()).unwrap();

    for (arguments, game_version) in [
        (
            vec![OsString::from("record"), output_path.as_os_str().to_owned()],
            "2026.8.1",
        ),
        (
            vec![OsString::from("replay"), output_path.as_os_str().to_owned()],
            "2026.9.0",
        ),
    ] {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        assert_eq!(
            run_with_version(
                arguments,
                game_version,
                &collection.0,
                |_, _| panic!("failed preparation must not validate"),
                &mut stdout,
                &mut stderr,
                |_| panic!("failed preparation must not launch Bevy"),
            ),
            EXIT_VALIDATION_FAILED
        );
        assert!(stdout.is_empty());
        assert!(!stderr.is_empty());
    }

    fs::write(
        manifest_path,
        "id: invented_story\nname: Invented Story\nversion: 9.0.0\nwindow_title: Invented\n",
    )
    .unwrap();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    assert_eq!(
        run_with_version(
            [OsString::from("replay"), output_path.as_os_str().to_owned(),],
            "2026.8.1",
            &collection.0,
            |_, _| panic!("failed preparation must not validate"),
            &mut stdout,
            &mut stderr,
            |_| panic!("failed preparation must not launch Bevy"),
        ),
        EXIT_VALIDATION_FAILED
    );
    assert!(stdout.is_empty());
    assert!(
        String::from_utf8(stderr)
            .unwrap()
            .contains("does not match installed source")
    );
}
