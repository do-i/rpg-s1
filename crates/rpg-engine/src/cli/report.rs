//! Output formatting for validation, baseline gating, reports, sweeps, and debug launches.

use std::{
    io::{self, Write},
    path::Path,
};

use super::args::battle_fixture_label;
use crate::{
    debug_launch::{DebugLaunchConfig, DebugPartyPreset},
    dialogue_sweep::DialogueTraversalReport,
    encounter_sweep::EncounterSweepReport,
    runtime_map_sweep::RuntimeMapSweepReport,
    scenario_cross_reference::{
        DiagnosticSeverity, ScenarioCatalogCounts, ScenarioValidationReport,
    },
    scenario_dialogue_report::{
        DialogueReachability, DialogueReport, DialogueReportEntry, DialogueReportShape,
    },
    scenario_map_report::MapReport,
    scenario_map_sweep::{MapSweepReport, SweepCategory},
    scenario_root::ScenarioRoot,
    scenario_validation_baseline::BaselineComparison,
};

pub(super) fn write_debug_log(output: &mut impl Write, debug: Option<&DebugLaunchConfig>) {
    let Some(debug) = debug else { return };
    if let (Some(map), Some(position)) = (&debug.start_map, debug.start_position) {
        let _ = writeln!(
            output,
            "Debug start: {map} at [{}, {}]",
            position.x, position.y
        );
    }
    if let Some(preset) = debug.party_preset {
        let label = match preset {
            DebugPartyPreset::Solo => "solo",
            DebugPartyPreset::Full => "full",
        };
        let _ = writeln!(output, "Debug party preset: {label}");
    }
    if let Some(fixture) = debug.battle_fixture {
        let _ = writeln!(
            output,
            "Debug battle fixture: {}",
            battle_fixture_label(fixture)
        );
    }
    for (flag, value) in &debug.flag_overrides {
        let _ = writeln!(
            output,
            "Debug flag override: {flag}={}",
            if *value { "set" } else { "unset" }
        );
    }
    let _ = writeln!(
        output,
        "Debug overrides are session-only until an explicit manual save"
    );
}

pub(super) fn write_validation_report(
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

    for line in validation_report_lines(report) {
        writeln!(output, "{line}")?;
    }
    Ok(())
}

/// Renders one report line per diagnostic, in the display order the report is printed in.
///
/// The baseline compares these exact strings, so the accepted-diagnostics file reads as a slice of
/// the validator's own output rather than a second format to learn.
pub(super) fn validation_report_lines(report: &ScenarioValidationReport) -> Vec<String> {
    let mut diagnostics = report.diagnostics.iter().collect::<Vec<_>>();
    diagnostics.sort_by(|left, right| {
        left.severity
            .cmp(&right.severity)
            .then_with(|| left.location.cmp(&right.location))
            .then_with(|| left.code.cmp(right.code))
            .then_with(|| left.message.cmp(&right.message))
    });
    diagnostics
        .into_iter()
        .map(|diagnostic| {
            format!(
                "{} {}:{} [{}] {}",
                severity_name(diagnostic.severity),
                report.package_key,
                diagnostic.location,
                diagnostic.code,
                diagnostic.message
            )
        })
        .collect()
}

/// Prints the baseline comparison and reports whether the run may pass.
///
/// Both directions fail: a diagnostic the baseline does not list is a regression, and a baseline
/// entry the report no longer produces is debt that was paid without deleting its line.
pub(super) fn write_baseline_comparison(
    output: &mut impl Write,
    baseline_path: &Path,
    comparison: &BaselineComparison,
) -> io::Result<()> {
    writeln!(output, "Baseline: {}", baseline_path.display())?;
    writeln!(
        output,
        "Baseline status: {} ({} accepted, {} new, {} no longer reported)",
        if comparison.is_clean() {
            "PASS"
        } else {
            "FAIL"
        },
        comparison.matched,
        comparison.unexpected.len(),
        comparison.resolved.len()
    )?;
    for line in &comparison.unexpected {
        writeln!(output, "new {line}")?;
    }
    for line in &comparison.resolved {
        writeln!(output, "no-longer-reported {line}")?;
    }
    if !comparison.unexpected.is_empty() {
        writeln!(
            output,
            "Fix the new diagnostics, or record them in {} with the decision that accepts them.",
            baseline_path.display()
        )?;
    }
    if !comparison.resolved.is_empty() {
        writeln!(
            output,
            "Delete the no-longer-reported lines from {}.",
            baseline_path.display()
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
pub(super) fn write_map_report(
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
pub(super) fn write_map_sweep_report(
    output: &mut impl Write,
    root: &ScenarioRoot,
    report: &MapSweepReport,
    runtime: &RuntimeMapSweepReport,
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
    writeln!(output)?;
    writeln!(output, "Runtime map-load sweep")?;
    if let Some(error) = &runtime.load_error {
        writeln!(output, "Load error: {error}")?;
    }
    for entry in &runtime.entries {
        match &entry.failure {
            Some(error) => writeln!(output, "  FAIL {}: {error}", entry.id)?,
            None => writeln!(
                output,
                "  PASS {} ({} headless frames)",
                entry.id, entry.frames
            )?,
        }
    }
    if !runtime.skipped_fixtures.is_empty() {
        writeln!(
            output,
            "Skipped non-migrated fixtures: {}",
            runtime.skipped_fixtures.join(", ")
        )?;
    }
    writeln!(
        output,
        "Runtime status: {} ({} passed, {} failed)",
        if runtime.is_valid() { "PASS" } else { "FAIL" },
        runtime.passed(),
        runtime.failed()
    )?;
    Ok(())
}

/// Renders the informational `dialogue-report` output: every dialogue's entries with their
/// requires/excludes condition, reachability under the pinned Python engine's first-match rule,
/// line count, and side-effect inventory, followed by a summary footer. The two documented-dead
/// `ardel_fisherman` flavor entries are called out as accepted rather than as findings.
pub(super) fn write_dialogue_report(
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

pub(super) fn write_dialogue_traversal_report(
    output: &mut impl Write,
    root: &ScenarioRoot,
    report: &DialogueTraversalReport,
) -> io::Result<()> {
    writeln!(output, "Dialogue traversal sweep")?;
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
        writeln!(
            output,
            "Dialogue {}: roots={} terminating_paths={} cycles={} errors={}",
            document.id,
            document.roots,
            document.terminating_paths,
            document.cycles.len(),
            document.errors.len()
        )?;
        for cycle in &document.cycles {
            writeln!(output, "  CYCLE {cycle}")?;
        }
        for error in &document.errors {
            writeln!(output, "  ERROR {error}")?;
        }
    }
    writeln!(output)?;
    writeln!(output, "Summary")?;
    writeln!(output, "Documents traversed: {}", report.documents.len())?;
    writeln!(output, "Root branches: {}", report.root_branches())?;
    writeln!(output, "Terminating paths: {}", report.terminating_paths())?;
    writeln!(output, "Cycles reported: {}", report.cycle_count())?;
    writeln!(output, "Errors: {}", report.error_count())?;
    writeln!(
        output,
        "Status: {}",
        if report.is_valid() { "PASS" } else { "FAIL" }
    )?;
    Ok(())
}

pub(super) fn write_encounter_sweep_report(
    output: &mut impl Write,
    root: &ScenarioRoot,
    report: &EncounterSweepReport,
) -> io::Result<()> {
    writeln!(output, "Encounter construction sweep")?;
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
    for zone in &report.zones {
        let failures = zone
            .constructions
            .iter()
            .filter(|construction| construction.failure.is_some())
            .count();
        writeln!(
            output,
            "{} {}: formations={} bosses={} constructions={} failures={}",
            if failures == 0 { "PASS" } else { "FAIL" },
            zone.id,
            zone.formations,
            zone.bosses,
            zone.constructions.len(),
            failures
        )?;
        for construction in &zone.constructions {
            if let Some(error) = &construction.failure {
                writeln!(
                    output,
                    "  ERROR {} ({} enemies): {error}",
                    construction.label, construction.enemies
                )?;
            }
        }
    }
    for failure in &report.asset_failures {
        writeln!(output, "ASSET ERROR {failure}")?;
    }
    writeln!(output)?;
    writeln!(output, "Summary")?;
    writeln!(output, "Zones swept: {}", report.zones.len())?;
    writeln!(output, "Encounters constructed: {}", report.constructions())?;
    writeln!(
        output,
        "Construction failures: {}",
        report.construction_failures()
    )?;
    writeln!(output, "Assets loaded: {}", report.assets_checked)?;
    writeln!(output, "Asset failures: {}", report.asset_failures.len())?;
    writeln!(
        output,
        "Status: {}",
        if report.is_valid() { "PASS" } else { "FAIL" }
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
