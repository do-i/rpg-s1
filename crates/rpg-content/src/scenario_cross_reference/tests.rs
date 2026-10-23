use super::{report::*, *};
use std::{collections::BTreeSet, fs, path::PathBuf};

use crate::test_support::scenario_package_dir;
use crate::{
    scenario_audio::{BGM_INDEX_PATH, SFX_INDEX_PATH},
    scenario_root::ScenarioRoot,
};
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
  tmx: media/maps/
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
        self.write(
            "data/maps/village.yaml",
            "name: Invented Village\nbgm: zone.village\n",
        );
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
            "title: {default: title.ogg}\nbattle: {normal: battle.ogg, boss: boss.ogg}\n\
                 zone: {village: village.ogg}\n",
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
            "media/maps/village.tmx",
            "media/audio/title.ogg",
            "media/audio/battle.ogg",
            "media/audio/boss.ogg",
            "media/audio/village.ogg",
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
            self.touch(&format!("media/audio/{key}.ogg"));
        }
    }
}

impl Drop for InventedScenario {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("temporary scenario should be removable");
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ValidationOutcome {
    Pass,
    Fail,
}

impl ValidationOutcome {
    fn from_success(success: bool) -> Self {
        if success { Self::Pass } else { Self::Fail }
    }
}

struct PythonParityCase {
    name: &'static str,
    mutate: fn(&InventedScenario),
    rust: ValidationOutcome,
    rust_code: Option<&'static str>,
    rust_field_path: Option<&'static str>,
}

fn no_mutation(_: &InventedScenario) {}

fn remove_intro_dialogue(fixture: &InventedScenario) {
    fs::remove_file(fixture.0.join("data/dialogue/intro.yaml"))
        .expect("invented intro should exist before removal");
}

fn add_shop_with_unknown_item(fixture: &InventedScenario) {
    fixture.write(
        "data/maps/village.yaml",
        r#"name: Invented Village
shop:
  items:
    - {id: absent_blade, buy_price: 10, unlock_flag: quest_started}
"#,
    );
}

fn add_encounter_with_unknown_background(fixture: &InventedScenario) {
    fixture.write(
        "data/encount/village.yaml",
        "density: 0.0\nbackground: absent_backdrop\n",
    );
}

fn add_dialogue_with_unknown_character(fixture: &InventedScenario) {
    fixture.write(
        "data/dialogue/recruiter.yaml",
        r#"id: recruiter
type: npc
entries:
  - lines: [Join the invented expedition.]
    on_complete: {join_party: absent_companion}
"#,
    );
}

fn add_recipe_with_unknown_output(fixture: &InventedScenario) {
    fixture.write(
        "data/recipe/recipes.yaml",
        r#"- id: absent_output_recipe
  scroll_name: An Invented Recipe
  output: {item: absent_tonic, qty: 1}
  inputs: {}
  gp_cost: 0
"#,
    );
}

fn remove_defined_quest_completion_flag(fixture: &InventedScenario) {
    let manifest = fs::read_to_string(fixture.0.join("manifest.yaml"))
        .expect("invented manifest should be readable")
        .replace(
            "bootstrap_flags: [quest_started, quest_done]",
            "bootstrap_flags: [quest_started]",
        );
    fixture.write("manifest.yaml", &manifest);
}

fn remove_party_portrait(fixture: &InventedScenario) {
    fs::remove_file(fixture.0.join("assets/maker_portrait.webp"))
        .expect("invented portrait should exist before removal");
}

fn remove_manifest_cursor(fixture: &InventedScenario) {
    fs::remove_file(fixture.0.join("assets/cursor.webp"))
        .expect("invented cursor should exist before removal");
}

fn remove_bgm_asset(fixture: &InventedScenario) {
    fs::remove_file(fixture.0.join("media/audio/title.ogg"))
        .expect("invented BGM asset should exist before removal");
}

fn add_encounter_with_unknown_enemy(fixture: &InventedScenario) {
    fixture.write(
        "data/encount/village.yaml",
        r#"density: 0.0
entries:
  - formation: [absent_slime]
    weight: 1
    chase_range: 1
"#,
    );
}

fn python_parity_cases() -> [PythonParityCase; 11] {
    [
        PythonParityCase {
            name: "accepted_compact_scenario",
            mutate: no_mutation,
            rust: ValidationOutcome::Pass,
            rust_code: None,
            rust_field_path: None,
        },
        PythonParityCase {
            name: "missing_manifest_intro_path",
            mutate: remove_intro_dialogue,
            rust: ValidationOutcome::Fail,
            rust_code: Some("path.missing"),
            rust_field_path: Some("start.intro_dialogue"),
        },
        PythonParityCase {
            name: "map_shop_unknown_item",
            mutate: add_shop_with_unknown_item,
            rust: ValidationOutcome::Fail,
            rust_code: Some("reference.missing"),
            rust_field_path: Some("shop.items[0].id"),
        },
        PythonParityCase {
            name: "encounter_unknown_background",
            mutate: add_encounter_with_unknown_background,
            rust: ValidationOutcome::Fail,
            rust_code: Some("reference.missing"),
            rust_field_path: Some("background"),
        },
        PythonParityCase {
            name: "dialogue_join_unknown_character",
            mutate: add_dialogue_with_unknown_character,
            rust: ValidationOutcome::Fail,
            rust_code: Some("reference.missing"),
            rust_field_path: Some("entries[0].on_complete.join_party"),
        },
        PythonParityCase {
            name: "recipe_unknown_output_item",
            mutate: add_recipe_with_unknown_output,
            rust: ValidationOutcome::Fail,
            rust_code: Some("reference.missing"),
            rust_field_path: Some("[0].output.item"),
        },
        PythonParityCase {
            name: "quest_consumes_undefined_flag",
            mutate: remove_defined_quest_completion_flag,
            rust: ValidationOutcome::Fail,
            rust_code: Some("flag.undefined"),
            rust_field_path: Some("[0].completed_flag"),
        },
        PythonParityCase {
            name: "party_portrait_asset_missing",
            mutate: remove_party_portrait,
            rust: ValidationOutcome::Fail,
            rust_code: Some("path.missing"),
            rust_field_path: Some("party[0].portrait"),
        },
        PythonParityCase {
            name: "manifest_cursor_missing_is_rust_strict",
            mutate: remove_manifest_cursor,
            rust: ValidationOutcome::Fail,
            rust_code: Some("path.missing"),
            rust_field_path: Some("title.cursor_icon"),
        },
        PythonParityCase {
            name: "bgm_asset_missing_is_rust_strict",
            mutate: remove_bgm_asset,
            rust: ValidationOutcome::Fail,
            rust_code: Some("path.missing"),
            rust_field_path: Some("categories[0].entries[0]"),
        },
        PythonParityCase {
            name: "encounter_enemy_missing_is_rust_strict",
            mutate: add_encounter_with_unknown_enemy,
            rust: ValidationOutcome::Fail,
            rust_code: Some("reference.missing"),
            rust_field_path: Some("entries[0].formation[0]"),
        },
    ]
}

fn validate_rust_parity_case(case: &PythonParityCase, fixture: &InventedScenario) {
    let report = validate_scenario_directory(&ScenarioRoot::default(), &fixture.0);
    assert_eq!(
        ValidationOutcome::from_success(report.is_valid()),
        case.rust,
        "Rust outcome changed for {}: {:#?}",
        case.name,
        report.diagnostics
    );
    if let Some(code) = case.rust_code {
        assert!(
            report.errors().any(|finding| {
                finding.code == code
                    && case
                        .rust_field_path
                        .is_none_or(|field_path| finding.location.field_path == field_path)
            }),
            "Rust evidence changed for {}: {:#?}",
            case.name,
            report.diagnostics
        );
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
    assert_eq!(report.scenario_id.as_deref(), Some("invented_story"));
    assert_eq!(report.scenario_name.as_deref(), Some("Invented Story"));
    assert_eq!(report.scenario_version.as_deref(), Some("1.0"));
    assert_eq!(report.counts.party_members, 1);
    assert_eq!(report.counts.classes, 1);
    assert_eq!(report.counts.items, 1);
    assert_eq!(report.counts.maps, 1);
    assert_eq!(report.counts.dialogue_documents, 1);
    assert_eq!(report.counts.quests, 1);
    assert!(report.checked_references > 60);
}

#[test]
fn rust_validator_preserves_the_python_parity_matrix_without_python_installed() {
    for case in python_parity_cases() {
        let fixture = InventedScenario::new();
        (case.mutate)(&fixture);
        validate_rust_parity_case(&case, &fixture);
    }
}

// `compares_invented_parity_cases_with_the_pinned_python_validator` lived here until the
// Python checkout was retired. It ran the source's own validator over each invented parity
// case and compared verdicts. The cases themselves are unaffected and still run above in
// `validate_rust_parity_case`; only the second opinion is gone, which is the point of no
// longer depending on the Python engine.

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
    assert!(errors.iter().any(
        |finding| finding.code == "flag.undefined" && finding.message.contains("missing_flag")
    ));
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
        .find(|finding| finding.code == "io.walk" && finding.location.path.as_str() == "data/maps")
        .expect("contained directory cycles should terminate with a stable diagnostic");
    assert!(cycle.message.contains("cycle or repeated alias"));
    assert!(
        !cycle
            .to_string()
            .contains(fixture.0.to_string_lossy().as_ref())
    );
}

/// Adds one ordinary enemy plus its battler sprite, so a `start_battle` can resolve.
fn add_duel_enemy(fixture: &InventedScenario) {
    fixture.write(
        "data/enemies/enemies_rank_a.yaml",
        r#"id: cinder_marshal
name: Cinder Marshal
type: humanoid
rank: A
hp: 900
atk: 40
def: 30
mres: 25
dex: 20
exp: 500
size: large
ai:
  pattern: random
  moves:
    - action: attack
      weight: 100
targeting:
  default: random_alive
"#,
    );
    fixture.touch("media/sprites/enemies/cinder_marshal.tsx");
}

#[test]
fn a_branch_can_start_a_battle_against_a_catalogued_enemy() {
    let fixture = InventedScenario::new();
    add_duel_enemy(&fixture);
    fixture.write(
        "data/dialogue/marshal_duel.yaml",
        r#"id: marshal_duel
type: npc
entries:
  - lines: [Then draw.]
    end: true
    on_complete:
      start_battle: cinder_marshal
"#,
    );

    let report = validate_scenario_directory(&ScenarioRoot::default(), &fixture.0);
    assert!(
        !report
            .diagnostics
            .iter()
            .any(|finding| finding.location.path.as_str() == "data/dialogue/marshal_duel.yaml"),
        "a resolvable scripted battle should be clean: {:#?}",
        report.diagnostics
    );
}

#[test]
fn a_start_battle_against_an_absent_enemy_is_a_missing_reference() {
    let fixture = InventedScenario::new();
    fixture.write(
        "data/dialogue/marshal_duel.yaml",
        r#"id: marshal_duel
type: npc
entries:
  - lines: [Then draw.]
    choices:
      - text: Draw.
        target: fight
        on_select:
          start_battle: absent_marshal
  - node: fight
    lines: [Steel meets steel.]
    end: true
"#,
    );

    let report = validate_scenario_directory(&ScenarioRoot::default(), &fixture.0);
    let finding = report
        .errors()
        .find(|finding| {
            finding.location.path.as_str() == "data/dialogue/marshal_duel.yaml"
                && finding.code == "reference.missing"
        })
        .expect("an unknown scripted-battle enemy must be reported");
    assert_eq!(
        finding.location.field_path,
        "entries[0].choices[0].on_select.start_battle"
    );
    assert_eq!(finding.message, "unknown enemy id `absent_marshal`");
}

/// The source documents `start_battle` as `npc`-only: a cutscene plays with no world behind
/// it, so there is nowhere to return the player to when the fight ends.
#[test]
fn a_start_battle_inside_a_cutscene_is_rejected_even_when_the_enemy_exists() {
    let fixture = InventedScenario::new();
    add_duel_enemy(&fixture);
    fixture.write(
        "data/dialogue/intro.yaml",
        r#"id: intro
type: cutscene
lines: [An invented beginning.]
on_complete:
  start_battle: cinder_marshal
"#,
    );

    let report = validate_scenario_directory(&ScenarioRoot::default(), &fixture.0);
    let finding = report
        .errors()
        .find(|finding| finding.code == "dialogue.start_battle_in_cutscene")
        .expect("a cutscene must not be able to start a battle");
    assert_eq!(finding.location.path.as_str(), "data/dialogue/intro.yaml");
    assert_eq!(finding.location.field_path, "on_complete.start_battle");
}

/// Roadmap B1.3: `zone_03_marshland.yaml` and all three `zone_04_ancient_ruins_*.yaml` were
/// two lines each and declared no BGM, so the marshland and the whole Ancient Ruins arc
/// played whichever town track the player walked in from. Nothing failed; it just sounded
/// wrong, for four maps, for the length of an act.
#[test]
fn a_playable_map_that_declares_no_bgm_is_reported() {
    let fixture = InventedScenario::new();
    fixture.write("data/maps/village.yaml", "name: Invented Village\n");

    let report = validate_scenario_directory(&ScenarioRoot::default(), &fixture.0);
    let finding = report
        .warnings()
        .find(|finding| finding.code == "map.missing_bgm")
        .expect("an unscored playable map must be reported");
    assert_eq!(finding.location.path.as_str(), "data/maps/village.yaml");
    assert_eq!(finding.location.field_path, "bgm");
}

/// A metadata file with no TMX of its own is not playable, and already has its own louder
/// diagnostic, so it must not also be nagged about a track nothing would ever play.
#[test]
fn an_unplayable_map_is_not_also_reported_for_its_missing_bgm() {
    let fixture = InventedScenario::new();
    fixture.write("data/maps/orphan.yaml", "name: Orphan\n");

    let report = validate_scenario_directory(&ScenarioRoot::default(), &fixture.0);
    assert!(
        report.warnings().any(|finding| {
            finding.code == "source.unmatched_map_metadata"
                && finding.location.path.as_str() == "data/maps/orphan.yaml"
        }),
        "the TMX-less metadata file should still be reported as unmatched"
    );
    assert!(
        !report.diagnostics.iter().any(|finding| {
            finding.code == "map.missing_bgm"
                && finding.location.path.as_str() == "data/maps/orphan.yaml"
        }),
        "an unplayable map must not be held to the BGM check: {:#?}",
        report.diagnostics
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

/// The exact shape of roadmap B2.1: `<stem>.yaml` describing a map whose TMX files are
/// `<stem>_01.tmx`, `<stem>_02.tmx`. Neither engine has a parent-to-segment metadata
/// fallback, so the file's BGM, `warp_order`, chests, and NPCs load for nobody. This used to
/// be suppressed as a multi-segment "convention" and shipped two unreachable chests.
#[test]
fn parent_metadata_naming_only_segment_tmx_files_is_an_error_not_a_convention() {
    let fixture = InventedScenario::new();
    fixture.write("data/maps/vale.yaml", "name: Vale\nwarp_order: 10\n");
    fixture.touch("media/maps/vale_01.tmx");
    fixture.touch("media/maps/vale_02.tmx");

    let report = validate_scenario_directory(&ScenarioRoot::default(), &fixture.0);
    let finding = report
        .errors()
        .find(|finding| finding.code == "source.unreadable_map_metadata")
        .expect("parent metadata with only segment TMX files must be an error");
    assert_eq!(finding.location.path.as_str(), "data/maps/vale.yaml");
    assert_eq!(finding.location.field_path, "$same_stem_tmx");
    assert!(
        finding.message.contains("vale_01, vale_02"),
        "message should name the segments: {}",
        finding.message
    );
    // The softer warning is for the no-segment case and must not double-report here.
    assert!(
        !report
            .warnings()
            .any(|finding| finding.code == "source.unmatched_map_metadata"
                && finding.location.path.as_str() == "data/maps/vale.yaml")
    );
}

#[test]
fn tmx_only_map_id_satisfies_a_runtime_map_reference() {
    let fixture = InventedScenario::new();
    fixture.touch("media/maps/tmx_only.tmx");
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

/// Pins the shipped scenario's exact diagnostics rather than asserting zero: four Sorcerer
/// ultimate flags are accepted inherited debt. A new or vanished diagnostic fails CI.
#[test]
fn the_shipped_scenario_matches_the_known_diagnostic_inventory() {
    let physical_root = crate::test_support::scenario_package_dir();
    let report = validate_scenario_directory(&ScenarioRoot::default(), &physical_root);

    let errors = report.errors().map(ToString::to_string).collect::<Vec<_>>();
    let warnings = report
        .warnings()
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    // The four accepted Sorcerer ultimate diagnostics, and nothing else.
    assert_eq!(errors.len(), 4, "shipped errors changed:\n{errors:#?}");
    assert_eq!(
        report
            .errors()
            .filter(|finding| finding.code == "flag.undefined")
            .count(),
        4
    );
    assert_eq!(
        report
            .errors()
            .filter_map(|finding| finding.message.split('`').nth(1))
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            "story_ultimate_earth",
            "story_ultimate_fire",
            "story_ultimate_water",
            "story_ultimate_wind",
        ])
    );

    // B4.1 consumes the Sky Crystal's Fly unlock through the manifest-selected transport
    // catalog, so the former orphan warning is intentionally gone.
    assert!(
        warnings.is_empty(),
        "shipped warnings changed:\n{warnings:#?}"
    );

    // No unresolvable asset path or dangling id may ship again.
    for code in ["path.missing", "reference.missing"] {
        assert_eq!(
            report.errors().filter(|f| f.code == code).count(),
            0,
            "a `{code}` diagnostic shipped:\n{errors:#?}"
        );
    }
}

#[test]
fn audits_the_complete_shipped_scenario_with_the_production_validator() {
    let report = validate_scenario_directory(&ScenarioRoot::default(), scenario_package_dir());
    // The authored id, not the package directory name: the package directory and the id its
    // manifest declares are allowed to differ, and here they do -- the id is inherited from
    // the source. Pinned so that changing it becomes a deliberate act rather than a silent
    // one.
    assert_eq!(report.scenario_id.as_deref(), Some("my_rpg_story"));
    assert_eq!(
        report.scenario_name.as_deref(),
        Some("Chronicles of the Lost Flame")
    );
    assert_eq!(report.scenario_version.as_deref(), Some("1.0.0"));
    assert_eq!(report.counts.party_members, 5);
    assert_eq!(report.counts.classes, 5);
    assert_eq!(report.counts.abilities, 42);
    assert_eq!(report.counts.items, 210);
    assert_eq!(report.counts.field_use_items, 14);
    assert_eq!(report.counts.maps, 74);
    assert_eq!(report.counts.dialogue_documents, 146);
    assert_eq!(report.counts.enemies, 108);
    assert_eq!(report.counts.boss_move_sets, 11);
    assert_eq!(report.counts.encounters, 21);
    assert_eq!(report.counts.battle_backgrounds, 13);
    assert_eq!(report.counts.recipes, 15);
    assert_eq!(report.counts.quests, 18);
    assert_eq!(report.counts.bgm_keys, 13);
    assert_eq!(report.counts.sfx_keys, 64);
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
    // Four, where the pinned scenario produced thirty-eight. The port repaired thirty-four of
    // them -- missing drop items, the stranded `zone_05_mountain_foothills` metadata, the
    // undefined `transport_warp_unlocked`, the `title.cursor_icon` and `zone.open_plains`
    // references -- and the detailed accounting of each pinned defect that used to live here
    // went with the checkout it described.
    //
    // What survives is inherited and deliberate: the Sorcerer's four ultimate-ability unlock
    // flags are consumed by `data/classes/sorcerer.yaml` and produced by nothing. Repairing
    // that is a content decision, not a port defect, so it is pinned here and mirrored by
    // the package's own `validation-baseline.txt`, which CI gates against.
    assert_eq!(errors.len(), 4, "shipped diagnostics changed:\n{errors:#?}");
    assert_eq!(
        warnings.len(),
        0,
        "shipped warnings changed:\n{warnings:#?}"
    );
    assert_eq!(
        report
            .errors()
            .filter(|finding| finding.code == "flag.undefined")
            .count(),
        4,
        "every remaining error should be an undefined flag"
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
        ])
    );
}
