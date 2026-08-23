//! Per-map reachability report for the `map-report` CLI subcommand.
//!
//! Unlike [`crate::scenario_cross_reference::validate_scenario_directory`], this walks each
//! map's TMX `portals` object layer (ADR 0003 leaves that XML edge outside the strict M2
//! validator) so the report can describe what a map actually connects to: its same-stem TMX (or
//! numbered-segment TMX family, mirroring the pinned Python `_is_submap`/multi-segment naming
//! convention from `engine/world/warp_logic.py`), its NPCs and their dialogue ids, and the
//! portal targets authored in its TMX. Every finding here is informational: this module never
//! fails the process, it only describes what is present and what is dangling.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use crate::{
    scenario_dialogue::DialogueDocument,
    scenario_manifest::Manifest,
    scenario_map::MapMetadata,
    scenario_path::ScenarioRelativePath,
    scenario_spatial::Position,
    scenario_yaml,
    tmx_header::parse_tmx_map_document,
    world_transition::runtime_portals,
};

/// True when `candidate` names a numbered TMX segment of `parent`: `parent` followed by `_` and
/// one or more ASCII digits, e.g. `zone_05_mountain_foothills_02` under
/// `zone_05_mountain_foothills`. Deliberately narrower than Python's general `_is_submap` (which
/// also matches worded interiors like `..._shop_01`) so only the multi-segment convention this
/// report exists to describe is recognized.
pub(crate) fn is_numeric_tmx_segment(parent: &str, candidate: &str) -> bool {
    candidate
        .strip_prefix(parent)
        .and_then(|rest| rest.strip_prefix('_'))
        .is_some_and(|suffix| !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit()))
}

/// True when `id` resolves to playable map content: either its own same-stem TMX, or a numbered
/// segment TMX family naming it as their parent.
fn map_id_is_resolvable(id: &str, tmx_stems: &BTreeSet<String>) -> bool {
    tmx_stems.contains(id) || tmx_stems.iter().any(|candidate| is_numeric_tmx_segment(id, candidate))
}

/// The complete per-map reachability report for one scenario package.
pub(crate) struct MapReport {
    pub(crate) scenario_id: Option<String>,
    pub(crate) scenario_name: Option<String>,
    pub(crate) entries: Vec<MapReportEntry>,
    /// Set only when the manifest (or its referenced directories) could not be read at all.
    pub(crate) load_error: Option<String>,
}

impl MapReport {
    pub(crate) fn resolvable_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.findings().is_empty())
            .count()
    }

    /// An empty report describing why nothing could be loaded, used when the manifest or a
    /// package selection step fails before any map can be examined.
    pub(crate) fn with_load_error(message: impl Into<String>) -> Self {
        Self {
            scenario_id: None,
            scenario_name: None,
            entries: Vec::new(),
            load_error: Some(message.into()),
        }
    }
}

/// One map identity: either a `data/maps/*.yaml` stem, an `assets/maps/*.tmx` stem, or both.
pub(crate) struct MapReportEntry {
    pub(crate) id: String,
    pub(crate) name: Option<String>,
    pub(crate) has_same_stem_tmx: bool,
    /// Numbered segment TMX stems naming this id as their parent, present only when
    /// `has_same_stem_tmx` is false and at least one segment TMX exists (the "parent metadata"
    /// convention).
    pub(crate) segments: Vec<String>,
    pub(crate) npcs: Vec<MapReportNpc>,
    pub(crate) portals: Vec<MapReportPortal>,
    /// Set when this map has a same-stem TMX but it could not be read or parsed.
    pub(crate) tmx_parse_error: Option<String>,
}

impl MapReportEntry {
    /// True when this id resolves to no TMX content at all: not directly, and not through a
    /// numbered-segment parent relationship.
    pub(crate) fn missing_tmx(&self) -> bool {
        !self.has_same_stem_tmx && self.segments.is_empty()
    }

    /// Every actionable observation for this map, in a stable, human-readable order.
    pub(crate) fn findings(&self) -> Vec<String> {
        let mut findings = Vec::new();
        if self.missing_tmx() {
            findings.push(format!(
                "no same-stem or segment TMX resolves map id `{}`",
                self.id
            ));
        }
        if let Some(error) = &self.tmx_parse_error {
            findings.push(format!("TMX could not be parsed: {error}"));
        }
        for portal in &self.portals {
            if !portal.target_resolvable {
                findings.push(format!(
                    "portal targets map id `{}` which has no same-stem or segment TMX",
                    portal.target_map
                ));
            }
        }
        for npc in &self.npcs {
            if npc.dialogue_missing {
                findings.push(format!(
                    "NPC `{}` references unknown dialogue id `{}`",
                    npc.id, npc.dialogue_id
                ));
            }
            if let Some(excuses) = &npc.excuses
                && npc.excuses_missing
            {
                findings.push(format!(
                    "NPC `{}` references unknown excuses dialogue id `{}`",
                    npc.id, excuses
                ));
            }
        }
        findings
    }

    /// Every dialogue id referenced by this map's NPCs, deduplicated (`dialogue` and `excuses`).
    pub(crate) fn dialogue_ids(&self) -> BTreeSet<String> {
        let mut ids = BTreeSet::new();
        for npc in &self.npcs {
            ids.insert(npc.dialogue_id.clone());
            if let Some(excuses) = &npc.excuses {
                ids.insert(excuses.clone());
            }
        }
        ids
    }
}

pub(crate) struct MapReportNpc {
    pub(crate) id: String,
    pub(crate) position: Position,
    pub(crate) dialogue_id: String,
    pub(crate) dialogue_missing: bool,
    pub(crate) excuses: Option<String>,
    pub(crate) excuses_missing: bool,
    /// True when `present` carries at least one `requires` or `excludes` flag.
    pub(crate) gated: bool,
}

pub(crate) struct MapReportPortal {
    pub(crate) target_map: String,
    pub(crate) target_position: Position,
    pub(crate) target_resolvable: bool,
}

/// Builds the complete report by reading the manifest and its three flat map-adjacent
/// directories directly from `physical_root`. Never fails: an unreadable manifest or directory
/// yields an empty, clearly-marked report rather than a process error, since this command is
/// informational only.
pub(crate) fn build_map_report(physical_root: &Path) -> MapReport {
    let canonical_root = physical_root.canonicalize().ok();
    let Some(manifest) = read_manifest(physical_root, canonical_root.as_deref()) else {
        return MapReport::with_load_error("manifest.yaml is unavailable or invalid");
    };

    let maps_dir = physical_root.join(manifest.refs.maps.as_str());
    let tmx_dir = physical_root.join(manifest.refs.tmx.as_str());
    let dialogue_dir = physical_root.join(manifest.refs.dialogue.as_str());

    let map_yaml = read_map_yaml_directory(&maps_dir, canonical_root.as_deref());
    let tmx_stems = read_stem_set(&tmx_dir, canonical_root.as_deref(), "tmx");
    let dialogue_ids = read_dialogue_ids(&dialogue_dir, canonical_root.as_deref());

    let mut ids: BTreeSet<String> = map_yaml.keys().cloned().collect();
    ids.extend(tmx_stems.iter().cloned());

    let entries = ids
        .into_iter()
        .map(|stem| {
            build_entry(
                &stem,
                physical_root,
                canonical_root.as_deref(),
                manifest.refs.tmx.as_str(),
                &map_yaml,
                &tmx_stems,
                &dialogue_ids,
            )
        })
        .collect();

    MapReport {
        scenario_id: Some(manifest.id.clone()),
        scenario_name: Some(manifest.name.clone()),
        entries,
        load_error: None,
    }
}

fn build_entry(
    stem: &str,
    physical_root: &Path,
    canonical_root: Option<&Path>,
    tmx_root: &str,
    map_yaml: &BTreeMap<String, Result<MapMetadata, String>>,
    tmx_stems: &BTreeSet<String>,
    dialogue_ids: &BTreeSet<String>,
) -> MapReportEntry {
    let has_same_stem_tmx = tmx_stems.contains(stem);
    let segments = if has_same_stem_tmx {
        Vec::new()
    } else {
        tmx_stems
            .iter()
            .filter(|candidate| is_numeric_tmx_segment(stem, candidate))
            .cloned()
            .collect()
    };

    let (name, npcs) = match map_yaml.get(stem) {
        Some(Ok(metadata)) => {
            let npcs = metadata
                .npcs
                .iter()
                .map(|npc| {
                    let dialogue_id = npc.effective_dialogue_id().to_owned();
                    let dialogue_missing = !dialogue_ids.contains(&dialogue_id);
                    let excuses = npc.excuses.clone();
                    let excuses_missing = excuses
                        .as_ref()
                        .is_some_and(|excuses| !dialogue_ids.contains(excuses));
                    MapReportNpc {
                        id: npc.id.clone(),
                        position: npc.position,
                        dialogue_id,
                        dialogue_missing,
                        excuses,
                        excuses_missing,
                        gated: !npc.present.requires.is_empty() || !npc.present.excludes.is_empty(),
                    }
                })
                .collect();
            (Some(metadata.name.clone()), npcs)
        }
        Some(Err(_)) | None => (None, Vec::new()),
    };

    let (portals, tmx_parse_error) = if has_same_stem_tmx {
        parse_portals(stem, physical_root, canonical_root, tmx_root, tmx_stems)
    } else {
        (Vec::new(), None)
    };

    MapReportEntry {
        id: stem.to_owned(),
        name,
        has_same_stem_tmx,
        segments,
        npcs,
        portals,
        tmx_parse_error,
    }
}

fn parse_portals(
    stem: &str,
    physical_root: &Path,
    canonical_root: Option<&Path>,
    tmx_root: &str,
    tmx_stems: &BTreeSet<String>,
) -> (Vec<MapReportPortal>, Option<String>) {
    let Ok(logical_path) = ScenarioRelativePath::try_from(format!("{tmx_root}/{stem}.tmx")) else {
        return (Vec::new(), Some("map TMX path is invalid".to_owned()));
    };
    let Some(text) = read_file_safely(physical_root, canonical_root, logical_path.as_str()) else {
        return (Vec::new(), Some("TMX file could not be read".to_owned()));
    };
    let document = match parse_tmx_map_document(&text, &logical_path) {
        Ok(document) => document,
        Err(error) => return (Vec::new(), Some(error.to_string())),
    };
    let portals = match runtime_portals(&document) {
        Ok(portals) => portals,
        Err(error) => return (Vec::new(), Some(error.to_string())),
    };
    let entries = portals
        .into_iter()
        .map(|portal| {
            let target_map = portal.target_map().as_str().to_owned();
            let target_resolvable = map_id_is_resolvable(&target_map, tmx_stems);
            MapReportPortal {
                target_map,
                target_position: portal.target_position(),
                target_resolvable,
            }
        })
        .collect();
    (entries, None)
}

fn read_manifest(physical_root: &Path, canonical_root: Option<&Path>) -> Option<Manifest> {
    let text = read_file_safely(physical_root, canonical_root, "manifest.yaml")?;
    scenario_yaml::from_str(&text).ok()
}

fn read_map_yaml_directory(
    dir: &Path,
    canonical_root: Option<&Path>,
) -> BTreeMap<String, Result<MapMetadata, String>> {
    let mut out = BTreeMap::new();
    let Some(paths) = safe_read_dir(dir, canonical_root) else {
        return out;
    };
    for path in paths {
        if path.extension().is_some_and(|extension| extension == "yaml")
            && let Some(stem) = path.file_stem().and_then(|stem| stem.to_str())
        {
            let value = fs::read_to_string(&path)
                .map_err(|error| error.to_string())
                .and_then(|text| {
                    scenario_yaml::from_str::<MapMetadata>(&text).map_err(|error| error.to_string())
                });
            out.insert(stem.to_owned(), value);
        }
    }
    out
}

fn read_stem_set(dir: &Path, canonical_root: Option<&Path>, extension: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let Some(paths) = safe_read_dir(dir, canonical_root) else {
        return out;
    };
    for path in paths {
        if path.extension().is_some_and(|found| found == extension)
            && let Some(stem) = path.file_stem().and_then(|stem| stem.to_str())
        {
            out.insert(stem.to_owned());
        }
    }
    out
}

fn read_dialogue_ids(dir: &Path, canonical_root: Option<&Path>) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let Some(paths) = safe_read_dir(dir, canonical_root) else {
        return out;
    };
    for path in paths {
        if path.extension().is_some_and(|extension| extension == "yaml")
            && let Some(stem) = path.file_stem().and_then(|stem| stem.to_str())
            && let Ok(text) = fs::read_to_string(&path)
            && let Ok(document) = scenario_yaml::from_str::<DialogueDocument>(&text)
        {
            out.insert(document.effective_id(stem).to_owned());
        }
    }
    out
}

/// Lists the files directly within `dir`, refusing to follow it outside `canonical_root`.
/// Every directory this module reads (`data/maps`, `assets/maps`, `data/dialogue`) is flat in
/// the pinned scenario corpus, so this deliberately does not recurse.
fn safe_read_dir(dir: &Path, canonical_root: Option<&Path>) -> Option<Vec<PathBuf>> {
    let canonical_root = canonical_root?;
    let canonical_dir = dir.canonicalize().ok()?;
    if !canonical_dir.starts_with(canonical_root) || !canonical_dir.is_dir() {
        return None;
    }
    let entries = fs::read_dir(dir).ok()?;
    Some(
        entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .collect(),
    )
}

fn read_file_safely(root: &Path, canonical_root: Option<&Path>, relative: &str) -> Option<String> {
    let canonical_root = canonical_root?;
    let candidate = root.join(relative);
    let canonical_candidate = candidate.canonicalize().ok()?;
    if !canonical_candidate.starts_with(canonical_root) {
        return None;
    }
    fs::read_to_string(candidate).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

    struct TempScenario(PathBuf);

    impl TempScenario {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should follow the Unix epoch")
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "rpg-s1-map-report-{}-{nonce}-{}",
                std::process::id(),
                NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(root.join("data/maps")).unwrap();
            fs::create_dir_all(root.join("data/dialogue")).unwrap();
            fs::create_dir_all(root.join("assets/maps")).unwrap();
            let scenario = Self(root);
            scenario.write(
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
"#,
            );
            scenario
        }

        fn write(&self, relative: &str, content: &str) {
            let path = self.0.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, content).unwrap();
        }

        fn touch_tmx(&self, stem: &str) {
            self.write(
                &format!("assets/maps/{stem}.tmx"),
                r#"<?xml version="1.0" encoding="UTF-8"?>
<map version="1.10" tiledversion="1.10.2" orientation="orthogonal" renderorder="right-down" width="1" height="1" tilewidth="32" tileheight="32" infinite="0" nextlayerid="1" nextobjectid="1">
</map>
"#,
            );
        }
    }

    impl Drop for TempScenario {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("temporary scenario should be removable");
        }
    }

    #[test]
    fn is_numeric_tmx_segment_matches_digits_only_and_rejects_worded_interiors() {
        assert!(is_numeric_tmx_segment(
            "zone_05_mountain_foothills",
            "zone_05_mountain_foothills_02"
        ));
        assert!(!is_numeric_tmx_segment(
            "town_01_ardel",
            "town_01_ardel_shop_01"
        ));
        assert!(!is_numeric_tmx_segment("zone_05", "zone_05_alt_02"));
        assert!(!is_numeric_tmx_segment(
            "zone_05_mountain_foothills",
            "zone_05_mountain_foothills"
        ));
    }

    #[test]
    fn parent_metadata_map_lists_its_numeric_segments_and_is_not_missing() {
        let fixture = TempScenario::new();
        fixture.write("data/maps/zone_05_mountain_foothills.yaml", "name: Foothills\n");
        fixture.touch_tmx("zone_05_mountain_foothills_01");
        fixture.touch_tmx("zone_05_mountain_foothills_02");

        let report = build_map_report(&fixture.0);
        let parent = report
            .entries
            .iter()
            .find(|entry| entry.id == "zone_05_mountain_foothills")
            .expect("parent metadata entry should be present");

        assert!(!parent.has_same_stem_tmx);
        assert!(!parent.missing_tmx());
        assert_eq!(
            parent.segments,
            vec![
                "zone_05_mountain_foothills_01".to_owned(),
                "zone_05_mountain_foothills_02".to_owned()
            ]
        );
        assert!(parent.findings().is_empty());
    }

    #[test]
    fn metadata_only_map_without_segments_is_a_missing_tmx_finding() {
        let fixture = TempScenario::new();
        fixture.write("data/maps/orphan.yaml", "name: Orphan\n");

        let report = build_map_report(&fixture.0);
        let orphan = report
            .entries
            .iter()
            .find(|entry| entry.id == "orphan")
            .unwrap();

        assert!(orphan.missing_tmx());
        assert!(
            orphan
                .findings()
                .iter()
                .any(|finding| finding.contains("no same-stem or segment TMX"))
        );
    }

    #[test]
    fn npc_dialogue_and_excuses_are_checked_against_the_dialogue_catalog() {
        let fixture = TempScenario::new();
        fixture.write(
            "data/maps/village.yaml",
            r#"name: Village
npcs:
  - id: guide
    name: Guide
    position: [1, 2]
    dialogue: guide_line
    excuses: missing_excuse
    type: guide
"#,
        );
        fixture.write(
            "data/dialogue/guide_line.yaml",
            "id: guide_line\nentries: [{lines: [Hi.]}]\n",
        );
        fixture.touch_tmx("village");

        let report = build_map_report(&fixture.0);
        let village = report
            .entries
            .iter()
            .find(|entry| entry.id == "village")
            .unwrap();
        let npc = &village.npcs[0];
        assert!(!npc.dialogue_missing);
        assert_eq!(npc.excuses.as_deref(), Some("missing_excuse"));
        assert!(npc.excuses_missing);
        assert!(
            village
                .findings()
                .iter()
                .any(|finding| finding.contains("missing_excuse"))
        );
    }

    #[test]
    fn dangling_portal_target_is_flagged_and_segment_target_is_resolvable() {
        let fixture = TempScenario::new();
        fixture.write(
            "assets/maps/village.tmx",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<map version="1.10" tiledversion="1.10.2" orientation="orthogonal" renderorder="right-down" width="1" height="1" tilewidth="32" tileheight="32" infinite="0" nextlayerid="2" nextobjectid="3">
  <objectgroup id="1" name="portals">
    <object id="1" x="0" y="0" width="32" height="32">
      <properties>
        <property name="target_map" value="nowhere"/>
        <property name="target_position_x" type="int" value="1"/>
        <property name="target_position_y" type="int" value="1"/>
      </properties>
    </object>
    <object id="2" x="32" y="0" width="32" height="32">
      <properties>
        <property name="target_map" value="zone_05_mountain_foothills"/>
        <property name="target_position_x" type="int" value="2"/>
        <property name="target_position_y" type="int" value="2"/>
      </properties>
    </object>
  </objectgroup>
</map>
"#,
        );
        fixture.touch_tmx("zone_05_mountain_foothills_01");

        let report = build_map_report(&fixture.0);
        let village = report
            .entries
            .iter()
            .find(|entry| entry.id == "village")
            .unwrap();
        assert_eq!(village.portals.len(), 2);
        let nowhere = village
            .portals
            .iter()
            .find(|portal| portal.target_map == "nowhere")
            .unwrap();
        assert!(!nowhere.target_resolvable);
        let segment_parent = village
            .portals
            .iter()
            .find(|portal| portal.target_map == "zone_05_mountain_foothills")
            .unwrap();
        assert!(segment_parent.target_resolvable);
        assert!(
            village
                .findings()
                .iter()
                .any(|finding| finding.contains("`nowhere`"))
        );
    }

    #[test]
    fn unreadable_manifest_yields_an_empty_report_with_a_load_error() {
        let report = build_map_report(Path::new("/nonexistent/rpg-s1-scenario"));
        assert!(report.load_error.is_some());
        assert!(report.entries.is_empty());
    }

    #[test]
    fn resolvable_count_excludes_entries_with_findings() {
        let fixture = TempScenario::new();
        fixture.write("data/maps/orphan.yaml", "name: Orphan\n");
        fixture.write("data/maps/fine.yaml", "name: Fine\n");
        fixture.touch_tmx("fine");

        let report = build_map_report(&fixture.0);
        assert_eq!(report.entries.len(), 2);
        assert_eq!(report.resolvable_count(), 1);
    }
}
