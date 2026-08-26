//! Headless production-pipeline sweep for the `map-sweep` CLI subcommand.
//!
//! Unlike [`crate::scenario_map_report`], which describes what each map's authored source
//! *says*, this module actually drives the production loaders over every TMX in the scenario:
//! the same TMX/TSX parsing (`tmx_header`, `tsx_metadata`), the same ground-layer and
//! visible-atlas selection rules (`tmx_ground_asset`), the same collision projection
//! (`scenario_spatial::collision_occupancy`), the same portal extraction
//! (`world_transition::runtime_portals`), the same NPC spawn-set filter
//! (`world_actor::present_npcs`), and the same sign-tile discovery (`world_object::sign_tiles`).
//! None of this needs a window, a GPU, or a running Bevy `App`: every one of those functions is
//! a plain function over already-read file bytes, so this module calls them directly against
//! bytes read from disk instead of driving `AssetServer` loads.
//!
//! Every finding here is informational: this module never fails the process, it only describes
//! what production code did when actually run against the map. `sample_01.tmx` is a known
//! placeholder using an inline tileset outside the M4 external-tileset profile (ADR 0007's
//! sibling parity-plan notes); it is expected to surface a TMX finding here, not to be treated
//! as a bug.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    runtime_flags::RuntimeFlags,
    runtime_map::RuntimeMapId,
    runtime_opened_boxes::OpenedBoxKey,
    scenario_manifest::Manifest,
    scenario_map::MapMetadata,
    scenario_map_report::is_numeric_tmx_segment,
    scenario_path::ScenarioRelativePath,
    scenario_spatial::{Position, collision_occupancy::CollisionOccupancy},
    scenario_yaml,
    tmx_ground_asset::{
        unique_ground_layer_index, visible_atlas_reference_indices, visible_layer_indices,
    },
    tmx_header::{TmxMapDocument, TmxTilesetRanges, parse_tmx_map_document},
    tsx_metadata::parse_tsx_tileset_metadata,
    world_actor::present_npcs,
    world_object::sign_tiles,
    world_transition::runtime_portals,
};

/// One production pipeline this sweep exercises, used to tally the summary footer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SweepCategory {
    /// TMX parsing, ground/visible-layer selection, and visible-atlas GID resolution.
    Tmx,
    /// Collision-occupancy projection from the TMX `collision` layer.
    Collision,
    /// NPC spawn-set derivation across the evaluated flag states.
    Npc,
    /// Portal extraction and target-map resolvability.
    Portal,
    /// Signs and item boxes.
    Object,
}

impl SweepCategory {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Tmx => "tmx",
            Self::Collision => "collision",
            Self::Npc => "npc",
            Self::Portal => "portal",
            Self::Object => "object",
        }
    }
}

/// One actionable observation from actually running a production loader against a map.
pub(crate) struct SweepFinding {
    pub(crate) category: SweepCategory,
    pub(crate) message: String,
}

impl SweepFinding {
    fn new(category: SweepCategory, message: impl Into<String>) -> Self {
        Self {
            category,
            message: message.into(),
        }
    }
}

/// The complete map sweep for one scenario package.
pub(crate) struct MapSweepReport {
    pub(crate) scenario_id: Option<String>,
    pub(crate) scenario_name: Option<String>,
    pub(crate) entries: Vec<MapSweepEntry>,
    /// Set only when the manifest (or its referenced directories) could not be read at all.
    pub(crate) load_error: Option<String>,
}

impl MapSweepReport {
    /// An empty report describing why nothing could be swept, used when the manifest or a
    /// package selection step fails before any map can be examined.
    pub(crate) fn with_load_error(message: impl Into<String>) -> Self {
        Self {
            scenario_id: None,
            scenario_name: None,
            entries: Vec::new(),
            load_error: Some(message.into()),
        }
    }

    pub(crate) fn clean_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.findings.is_empty())
            .count()
    }

    pub(crate) fn maps_with_findings(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| !entry.findings.is_empty())
            .count()
    }

    /// The number of findings in one category, across every swept map.
    pub(crate) fn category_count(&self, category: SweepCategory) -> usize {
        self.entries
            .iter()
            .flat_map(|entry| &entry.findings)
            .filter(|finding| finding.category == category)
            .count()
    }
}

/// One TMX file actually run through the production loading pipeline.
pub(crate) struct MapSweepEntry {
    pub(crate) id: String,
    pub(crate) portal_count: usize,
    pub(crate) sign_count: usize,
    pub(crate) item_box_count: usize,
    /// Number of flag states the NPC spawn set was evaluated under: 1 (empty) plus one for each
    /// distinct `requires`/`excludes` flag referenced by this map's NPCs. Zero when this TMX has
    /// no same-stem metadata YAML to spawn NPCs from.
    pub(crate) npc_flag_states: usize,
    pub(crate) findings: Vec<SweepFinding>,
}

/// Sweeps every TMX file beneath `physical_root`'s configured `tmx` directory through the
/// production loading pipeline. Never fails: an unreadable manifest yields an empty, clearly
/// marked report rather than a process error, since this command is informational only.
pub(crate) fn build_map_sweep(physical_root: &Path) -> MapSweepReport {
    let canonical_root = physical_root.canonicalize().ok();
    let Some(manifest) = read_manifest(physical_root, canonical_root.as_deref()) else {
        return MapSweepReport::with_load_error("manifest.yaml is unavailable or invalid");
    };

    let tmx_dir = physical_root.join(manifest.refs.tmx.as_str());
    let maps_dir = physical_root.join(manifest.refs.maps.as_str());
    let dialogue_dir = physical_root.join(manifest.refs.dialogue.as_str());

    let tmx_stems = read_stem_set(&tmx_dir, canonical_root.as_deref(), "tmx");
    let dialogue_ids = read_dialogue_ids(&dialogue_dir, canonical_root.as_deref());

    let entries = tmx_stems
        .iter()
        .map(|stem| {
            build_entry(
                stem,
                physical_root,
                canonical_root.as_deref(),
                &manifest,
                &tmx_stems,
                &dialogue_ids,
                &maps_dir,
            )
        })
        .collect();

    MapSweepReport {
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
    manifest: &Manifest,
    tmx_stems: &BTreeSet<String>,
    dialogue_ids: &BTreeSet<String>,
    maps_dir: &Path,
) -> MapSweepEntry {
    let mut findings = Vec::new();
    let mut portal_count = 0;
    let mut sign_count = 0;
    let mut item_box_count = 0;
    let mut npc_flag_states = 0;

    let tmx_root = manifest.refs.tmx.as_str();
    let logical_tmx = format!("{tmx_root}/{stem}.tmx");
    let document = ScenarioRelativePath::try_from(logical_tmx.as_str())
        .ok()
        .and_then(|logical| {
            read_file_safely(physical_root, canonical_root, logical.as_str())
                .map(|text| (logical, text))
        });

    let document = match document {
        Some((logical, text)) => match parse_tmx_map_document(&text, &logical) {
            Ok(document) => Some(document),
            Err(error) => {
                findings.push(SweepFinding::new(
                    SweepCategory::Tmx,
                    format!("TMX could not be parsed: {error}"),
                ));
                None
            }
        },
        None => {
            findings.push(SweepFinding::new(
                SweepCategory::Tmx,
                "TMX file could not be read",
            ));
            None
        }
    };

    if let Some(document) = &document {
        check_visible_pipeline(document, physical_root, canonical_root, &mut findings);
        check_collision(document, &mut findings);
        portal_count = check_portals(document, tmx_stems, &mut findings);
        sign_count = check_signs(document, stem, manifest, dialogue_ids, &mut findings);
        let (states, boxes) = check_npcs_and_boxes(
            document,
            stem,
            maps_dir,
            canonical_root,
            dialogue_ids,
            &mut findings,
        );
        npc_flag_states = states;
        item_box_count = boxes;
    }

    MapSweepEntry {
        id: stem.to_owned(),
        portal_count,
        sign_count,
        item_box_count,
        npc_flag_states,
        findings,
    }
}

/// Replays [`crate::tmx_ground_asset::TmxGroundAsset`]'s visible-tile-layer pipeline: the same
/// ground-layer and visible-atlas selection, the same ordered GID-range resolution, and the same
/// atlas-tilecount boundary [`crate::tsx_atlas_asset::TsxAtlasAsset::sprite_for_tile`] enforces —
/// all without needing a Bevy `AssetServer` or decoded images, since every step through GID
/// resolution is plain data over already-parsed TSX metadata.
fn check_visible_pipeline(
    document: &TmxMapDocument,
    physical_root: &Path,
    canonical_root: Option<&Path>,
    findings: &mut Vec<SweepFinding>,
) {
    if let Err(error) = unique_ground_layer_index(document) {
        findings.push(SweepFinding::new(
            SweepCategory::Tmx,
            format!("ground layer: {error}"),
        ));
    }

    let visible = visible_layer_indices(document);
    let atlas_references = match visible_atlas_reference_indices(document, &visible) {
        Ok(references) => references,
        Err(error) => {
            findings.push(SweepFinding::new(
                SweepCategory::Tmx,
                format!("visible atlas references: {error}"),
            ));
            return;
        }
    };

    let mut metadata = Vec::with_capacity(atlas_references.len());
    for &reference_index in &atlas_references {
        let reference = &document.external_tilesets()[reference_index];
        let Some(text) =
            read_file_safely(physical_root, canonical_root, reference.source().as_str())
        else {
            findings.push(SweepFinding::new(
                SweepCategory::Tmx,
                format!("tileset `{}` could not be read", reference.source()),
            ));
            return;
        };
        match parse_tsx_tileset_metadata(&text, reference.source()) {
            Ok(parsed) => metadata.push(parsed),
            Err(error) => {
                findings.push(SweepFinding::new(
                    SweepCategory::Tmx,
                    format!("tileset `{}` is invalid: {error}", reference.source()),
                ));
                return;
            }
        }
    }

    let pairs = atlas_references
        .iter()
        .map(|&index| &document.external_tilesets()[index])
        .zip(metadata.iter());
    let ranges = match TmxTilesetRanges::try_new(pairs) {
        Ok(ranges) => ranges,
        Err(error) => {
            findings.push(SweepFinding::new(
                SweepCategory::Tmx,
                format!("visible tileset ranges: {error}"),
            ));
            return;
        }
    };

    for &layer_index in &visible {
        for gid in document.tile_layers()[layer_index]
            .gids()
            .iter()
            .copied()
            .filter(|gid| !gid.is_empty())
        {
            match ranges.resolve(gid) {
                Ok(Some(resolved)) => {
                    let tile_count = atlas_references
                        .iter()
                        .position(|&index| {
                            std::ptr::eq(&document.external_tilesets()[index], resolved.tileset())
                        })
                        .and_then(|position| metadata.get(position))
                        .map(|metadata| metadata.tile_count());
                    if let Some(tile_count) = tile_count
                        && resolved.local_id() >= tile_count
                    {
                        findings.push(SweepFinding::new(
                            SweepCategory::Tmx,
                            format!(
                                "visible tile local id {} is outside atlas tilecount {tile_count}",
                                resolved.local_id()
                            ),
                        ));
                        return;
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    findings.push(SweepFinding::new(
                        SweepCategory::Tmx,
                        format!("visible GID is invalid: {error}"),
                    ));
                    return;
                }
            }
        }
    }
}

fn check_collision(document: &TmxMapDocument, findings: &mut Vec<SweepFinding>) {
    if let Err(error) = CollisionOccupancy::from_tmx_document(document) {
        findings.push(SweepFinding::new(
            SweepCategory::Collision,
            error.to_string(),
        ));
    }
}

fn check_portals(
    document: &TmxMapDocument,
    tmx_stems: &BTreeSet<String>,
    findings: &mut Vec<SweepFinding>,
) -> usize {
    match runtime_portals(document) {
        Ok(portals) => {
            for portal in &portals {
                let target = portal.target_map().as_str();
                if !map_id_is_resolvable(target, tmx_stems) {
                    findings.push(SweepFinding::new(
                        SweepCategory::Portal,
                        format!(
                            "portal targets map id `{target}` which has no same-stem or segment TMX"
                        ),
                    ));
                }
            }
            portals.len()
        }
        Err(error) => {
            findings.push(SweepFinding::new(
                SweepCategory::Portal,
                format!("portals could not be parsed: {error}"),
            ));
            0
        }
    }
}

/// True when `id` resolves to playable map content: either its own same-stem TMX, or a numbered
/// segment TMX family naming it as their parent. Mirrors
/// [`crate::scenario_map_report::map_id_is_resolvable`], reusing its shared segment-naming rule.
fn map_id_is_resolvable(id: &str, tmx_stems: &BTreeSet<String>) -> bool {
    tmx_stems.contains(id)
        || tmx_stems
            .iter()
            .any(|candidate| is_numeric_tmx_segment(id, candidate))
}

/// Runs the production sign-tile scan ([`crate::world_object::sign_tiles`]) and checks that a
/// map with any discovered signs has the dialogue id the production sign-interaction path would
/// look up (`sign_{map_id}`, the exact convention `world_object::drive_world_object_load` uses).
fn check_signs(
    document: &TmxMapDocument,
    stem: &str,
    manifest: &Manifest,
    dialogue_ids: &BTreeSet<String>,
    findings: &mut Vec<SweepFinding>,
) -> usize {
    let positions = sign_tiles(document, &manifest.signs);
    if !positions.is_empty() {
        let dialogue_id = format!("sign_{stem}");
        if !dialogue_ids.contains(&dialogue_id) {
            findings.push(SweepFinding::new(
                SweepCategory::Object,
                format!(
                    "{} sign(s) present but dialogue `{dialogue_id}` is missing",
                    positions.len()
                ),
            ));
        }
    }
    positions.len()
}

/// Loads this TMX's same-stem metadata YAML (when present) and, using the exact production
/// [`present_npcs`] filter, evaluates the would-spawn NPC set under the empty flag state plus one
/// state per distinct flag referenced by this map's NPC `present` conditions. Also structurally
/// walks the map's item boxes (no interaction simulation).
///
/// A TMX with no same-stem metadata YAML has no NPCs or item boxes to sweep; that is expected for
/// the pinned corpus's numbered cave/segment/placeholder maps and is not itself a finding.
fn check_npcs_and_boxes(
    document: &TmxMapDocument,
    stem: &str,
    maps_dir: &Path,
    canonical_root: Option<&Path>,
    dialogue_ids: &BTreeSet<String>,
    findings: &mut Vec<SweepFinding>,
) -> (usize, usize) {
    let metadata_filename = format!("{stem}.yaml");
    if !maps_dir.join(&metadata_filename).is_file() {
        return (0, 0);
    }
    let Some(text) = read_file_safely(maps_dir, canonical_root, &metadata_filename) else {
        findings.push(SweepFinding::new(
            SweepCategory::Npc,
            "map metadata YAML could not be read",
        ));
        return (0, 0);
    };
    let metadata: MapMetadata = match scenario_yaml::from_str(&text) {
        Ok(metadata) => metadata,
        Err(error) => {
            findings.push(SweepFinding::new(
                SweepCategory::Npc,
                format!("map metadata YAML could not be parsed: {error}"),
            ));
            return (0, 0);
        }
    };
    if metadata.effective_id(stem) != stem {
        findings.push(SweepFinding::new(
            SweepCategory::Npc,
            format!(
                "map metadata id `{}` does not match filename stem `{stem}`",
                metadata.effective_id(stem)
            ),
        ));
    }

    let header = document.header();
    let (width, height) = (header.width() as i32, header.height() as i32);
    let in_bounds = |position: Position| {
        position.x >= 0 && position.x < width && position.y >= 0 && position.y < height
    };

    let mut seen_ids = BTreeSet::new();
    for npc in &metadata.npcs {
        if !seen_ids.insert(npc.id.as_str()) {
            findings.push(SweepFinding::new(
                SweepCategory::Npc,
                format!("NPC id `{}` is declared more than once", npc.id),
            ));
        }
    }

    let mut referenced_flags = BTreeSet::new();
    for npc in &metadata.npcs {
        referenced_flags.extend(npc.present.requires.iter().cloned());
        referenced_flags.extend(npc.present.excludes.iter().cloned());
    }
    let mut states = vec![RuntimeFlags::default()];
    for flag in &referenced_flags {
        states.push(RuntimeFlags::from_bootstrap([flag.clone()]));
    }

    let mut reported_dialogue_missing = BTreeSet::new();
    let mut reported_out_of_bounds = BTreeSet::new();
    for flags in &states {
        for npc in present_npcs(&metadata, flags) {
            let dialogue_id = npc.effective_dialogue_id();
            if !dialogue_ids.contains(dialogue_id)
                && reported_dialogue_missing.insert(npc.id.clone())
            {
                findings.push(SweepFinding::new(
                    SweepCategory::Npc,
                    format!(
                        "NPC `{}` references unknown dialogue id `{dialogue_id}`",
                        npc.id
                    ),
                ));
            }
            if let Some(excuses) = &npc.excuses
                && !dialogue_ids.contains(excuses)
                && reported_dialogue_missing.insert(format!("{}#excuses", npc.id))
            {
                findings.push(SweepFinding::new(
                    SweepCategory::Npc,
                    format!(
                        "NPC `{}` references unknown excuses dialogue id `{excuses}`",
                        npc.id
                    ),
                ));
            }
            if !in_bounds(npc.position) && reported_out_of_bounds.insert(npc.id.clone()) {
                findings.push(SweepFinding::new(
                    SweepCategory::Npc,
                    format!(
                        "NPC `{}` at ({}, {}) is outside map bounds {width}x{height}",
                        npc.id, npc.position.x, npc.position.y
                    ),
                ));
            }
        }
    }

    let map_id =
        RuntimeMapId::try_new(stem.to_owned()).expect("a scanned TMX stem is a nonempty map id");
    for item_box in &metadata.item_boxes {
        if let Err(error) = OpenedBoxKey::try_new(map_id.clone(), item_box.id.clone()) {
            findings.push(SweepFinding::new(
                SweepCategory::Object,
                format!("item box `{}` has an invalid key: {error}", item_box.id),
            ));
        }
        if !in_bounds(item_box.position) {
            findings.push(SweepFinding::new(
                SweepCategory::Object,
                format!(
                    "item box `{}` at ({}, {}) is outside map bounds {width}x{height}",
                    item_box.id, item_box.position.x, item_box.position.y
                ),
            ));
        }
        if item_box.loot.items.is_empty() && item_box.loot.magic_cores.is_empty() {
            findings.push(SweepFinding::new(
                SweepCategory::Object,
                format!("item box `{}` has no loot entries", item_box.id),
            ));
        }
    }

    (states.len(), metadata.item_boxes.len())
}

fn read_manifest(physical_root: &Path, canonical_root: Option<&Path>) -> Option<Manifest> {
    let text = read_file_safely(physical_root, canonical_root, "manifest.yaml")?;
    scenario_yaml::from_str(&text).ok()
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
        if path
            .extension()
            .is_some_and(|extension| extension == "yaml")
            && let Some(stem) = path.file_stem().and_then(|stem| stem.to_str())
            && let Ok(text) = fs::read_to_string(&path)
            && let Ok(document) =
                scenario_yaml::from_str::<crate::scenario_dialogue::DialogueDocument>(&text)
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
                "rpg-s1-map-sweep-{}-{nonce}-{}",
                std::process::id(),
                NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(root.join("data/maps")).unwrap();
            fs::create_dir_all(root.join("data/dialogue")).unwrap();
            fs::create_dir_all(root.join("assets/maps")).unwrap();
            fs::create_dir_all(root.join("assets/tilesets")).unwrap();
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
signs: {tileset: signboard, tile_ids: [1]}
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
    }

    impl Drop for TempScenario {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("temporary scenario should be removable");
        }
    }

    fn minimal_map_xml(collision_row: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<map version="1.10" orientation="orthogonal" renderorder="right-down" width="2" height="2" tilewidth="32" tileheight="32" infinite="0" nextlayerid="3" nextobjectid="1">
  <layer id="1" name="ground" width="2" height="2">
    <data encoding="csv">0,0,
    0,0</data>
  </layer>
  <layer id="2" name="collision" width="2" height="2">
    <data encoding="csv">{collision_row}</data>
  </layer>
</map>
"#
        )
    }

    fn entry<'a>(report: &'a MapSweepReport, id: &str) -> &'a MapSweepEntry {
        report.entries.iter().find(|entry| entry.id == id).unwrap()
    }

    fn findings(entry: &MapSweepEntry, category: SweepCategory) -> Vec<&str> {
        entry
            .findings
            .iter()
            .filter(|finding| finding.category == category)
            .map(|finding| finding.message.as_str())
            .collect()
    }

    #[test]
    fn unreadable_manifest_yields_an_empty_report_with_a_load_error() {
        let report = build_map_sweep(Path::new("/nonexistent/rpg-s1-scenario"));
        assert!(report.load_error.is_some());
        assert!(report.entries.is_empty());
    }

    #[test]
    fn every_tmx_is_visited_even_when_one_fails_to_parse() {
        let fixture = TempScenario::new();
        fixture.write("assets/maps/broken.tmx", "not xml at all");
        fixture.write("assets/maps/village.tmx", &minimal_map_xml("0,0,\n0,0"));

        let report = build_map_sweep(&fixture.0);

        assert_eq!(report.entries.len(), 2);
        let broken = entry(&report, "broken");
        assert_eq!(findings(broken, SweepCategory::Tmx).len(), 1);
        assert!(broken.findings[0].message.contains("could not be parsed"));
        let village = entry(&report, "village");
        assert!(village.findings.is_empty());
    }

    #[test]
    fn missing_visible_tileset_is_a_tmx_finding() {
        let fixture = TempScenario::new();
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<map version="1.10" orientation="orthogonal" renderorder="right-down" width="1" height="1" tilewidth="32" tileheight="32" infinite="0" nextlayerid="3" nextobjectid="1">
  <tileset firstgid="1" source="../tilesets/missing.tsx"/>
  <layer id="1" name="ground" width="1" height="1">
    <data encoding="csv">1</data>
  </layer>
  <layer id="2" name="collision" width="1" height="1">
    <data encoding="csv">0</data>
  </layer>
</map>
"#;
        fixture.write("assets/maps/ghost_tileset.tmx", xml);

        let report = build_map_sweep(&fixture.0);
        let mapped = entry(&report, "ghost_tileset");
        let tmx_findings = findings(mapped, SweepCategory::Tmx);
        assert_eq!(tmx_findings.len(), 1);
        assert!(
            tmx_findings[0].contains("could not be read"),
            "{tmx_findings:?}"
        );
    }

    #[test]
    fn missing_collision_layer_is_a_collision_finding() {
        let fixture = TempScenario::new();
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<map version="1.10" orientation="orthogonal" renderorder="right-down" width="1" height="1" tilewidth="32" tileheight="32" infinite="0" nextlayerid="2" nextobjectid="1">
  <layer id="1" name="ground" width="1" height="1">
    <data encoding="csv">0</data>
  </layer>
</map>
"#;
        fixture.write("assets/maps/no_collision.tmx", xml);

        let report = build_map_sweep(&fixture.0);
        let mapped = entry(&report, "no_collision");
        let collision_findings = findings(mapped, SweepCategory::Collision);
        assert_eq!(collision_findings.len(), 1);
        assert!(collision_findings[0].contains("collision"));
    }

    #[test]
    fn dangling_portal_target_is_a_portal_finding_and_valid_target_resolves() {
        let fixture = TempScenario::new();
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<map version="1.10" orientation="orthogonal" renderorder="right-down" width="2" height="2" tilewidth="32" tileheight="32" infinite="0" nextlayerid="3" nextobjectid="3">
  <layer id="1" name="ground" width="2" height="2"><data encoding="csv">0,0,
  0,0</data></layer>
  <layer id="2" name="collision" width="2" height="2"><data encoding="csv">0,0,
  0,0</data></layer>
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
        <property name="target_map" value="destination"/>
        <property name="target_position_x" type="int" value="1"/>
        <property name="target_position_y" type="int" value="1"/>
      </properties>
    </object>
  </objectgroup>
</map>
"#;
        fixture.write("assets/maps/origin.tmx", xml);
        fixture.write("assets/maps/destination.tmx", &minimal_map_xml("0,0,\n0,0"));

        let report = build_map_sweep(&fixture.0);
        let origin = entry(&report, "origin");
        assert_eq!(origin.portal_count, 2);
        let portal_findings = findings(origin, SweepCategory::Portal);
        assert_eq!(portal_findings.len(), 1);
        assert!(portal_findings[0].contains("`nowhere`"));
    }

    #[test]
    fn npc_flag_states_cover_empty_and_each_referenced_flag() {
        let fixture = TempScenario::new();
        fixture.write("assets/maps/village.tmx", &minimal_map_xml("0,0,\n0,0"));
        fixture.write(
            "data/maps/village.yaml",
            r#"name: Village
npcs:
  - id: guide
    name: Guide
    position: [0, 0]
    dialogue: guide_line
    present: { requires: [story_started], excludes: [story_done] }
"#,
        );
        fixture.write(
            "data/dialogue/guide_line.yaml",
            "id: guide_line\nentries: [{lines: [Hi.]}]\n",
        );

        let report = build_map_sweep(&fixture.0);
        let village = entry(&report, "village");
        assert_eq!(village.npc_flag_states, 3);
        assert!(village.findings.is_empty());
    }

    #[test]
    fn npc_out_of_bounds_position_and_unknown_dialogue_are_npc_findings() {
        let fixture = TempScenario::new();
        fixture.write("assets/maps/village.tmx", &minimal_map_xml("0,0,\n0,0"));
        fixture.write(
            "data/maps/village.yaml",
            r#"name: Village
npcs:
  - id: ghost
    name: Ghost
    position: [50, 50]
    dialogue: unknown_line
"#,
        );

        let report = build_map_sweep(&fixture.0);
        let village = entry(&report, "village");
        let npc_findings = findings(village, SweepCategory::Npc);
        assert!(
            npc_findings
                .iter()
                .any(|finding| finding.contains("outside map bounds")),
            "{npc_findings:?}"
        );
        assert!(
            npc_findings
                .iter()
                .any(|finding| finding.contains("unknown dialogue id `unknown_line`")),
            "{npc_findings:?}"
        );
    }

    #[test]
    fn tmx_without_metadata_yaml_sweeps_npc_free_with_zero_flag_states() {
        let fixture = TempScenario::new();
        fixture.write("assets/maps/cave.tmx", &minimal_map_xml("0,0,\n0,0"));

        let report = build_map_sweep(&fixture.0);
        let cave = entry(&report, "cave");
        assert_eq!(cave.npc_flag_states, 0);
        assert!(cave.findings.is_empty());
    }

    #[test]
    fn item_box_out_of_bounds_and_empty_loot_are_object_findings() {
        let fixture = TempScenario::new();
        fixture.write("assets/maps/village.tmx", &minimal_map_xml("0,0,\n0,0"));
        fixture.write(
            "data/maps/village.yaml",
            r#"name: Village
item_boxes:
  - id: chest_01
    position: [99, 99]
"#,
        );

        let report = build_map_sweep(&fixture.0);
        let village = entry(&report, "village");
        assert_eq!(village.item_box_count, 1);
        let object_findings = findings(village, SweepCategory::Object);
        assert!(
            object_findings
                .iter()
                .any(|finding| finding.contains("outside map bounds")),
            "{object_findings:?}"
        );
        assert!(
            object_findings
                .iter()
                .any(|finding| finding.contains("no loot entries")),
            "{object_findings:?}"
        );
    }

    #[test]
    fn signs_present_without_matching_dialogue_is_an_object_finding() {
        let fixture = TempScenario::new();
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<map version="1.10" orientation="orthogonal" renderorder="right-down" width="1" height="1" tilewidth="32" tileheight="32" infinite="0" nextlayerid="2" nextobjectid="1">
  <tileset firstgid="1" source="../tilesets/signboard.tsx"/>
  <layer id="1" name="ground" width="1" height="1"><data encoding="csv">2</data></layer>
  <layer id="2" name="collision" width="1" height="1"><data encoding="csv">0</data></layer>
</map>
"#;
        fixture.write("assets/maps/village.tmx", xml);
        fixture.write(
            "assets/tilesets/signboard.tsx",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<tileset name="signboard" tilewidth="32" tileheight="32" tilecount="2" columns="2">
  <image source="signboard.png" width="64" height="32"/>
</tileset>
"#,
        );

        let report = build_map_sweep(&fixture.0);
        let village = entry(&report, "village");
        assert_eq!(village.sign_count, 1);
        let object_findings = findings(village, SweepCategory::Object);
        assert_eq!(object_findings.len(), 1);
        assert!(object_findings[0].contains("sign_village"));
    }

    #[test]
    fn category_count_tallies_findings_by_category_across_maps() {
        let fixture = TempScenario::new();
        fixture.write("assets/maps/broken.tmx", "not xml");
        fixture.write("assets/maps/village.tmx", &minimal_map_xml("0,0,\n0,0"));

        let report = build_map_sweep(&fixture.0);
        assert_eq!(report.category_count(SweepCategory::Tmx), 1);
        assert_eq!(report.category_count(SweepCategory::Collision), 0);
        assert_eq!(report.maps_with_findings(), 1);
        assert_eq!(report.clean_count(), 1);
    }

    #[test]
    fn the_shipped_scenario_sweeps_every_tmx_and_visits_the_known_placeholder() {
        let root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/scenarios/rusted_kingdoms");
        let report = build_map_sweep(&root);

        assert!(report.load_error.is_none());
        assert_eq!(report.entries.len(), 47);

        let sample_01 = entry(&report, "sample_01");
        assert!(
            findings(sample_01, SweepCategory::Tmx)
                .iter()
                .any(|finding| finding.contains("inline")),
            "sample_01.tmx is a known placeholder using an inline tileset"
        );
    }

    /// Pins the exact finding inventory against the shipped scenario. This test exists to catch
    /// regressions in the sweep pipeline itself, not to bless the count — a real change in either
    /// direction (new findings, or findings that silently stop firing) is worth investigating.
    #[test]
    fn the_shipped_scenario_matches_the_known_finding_inventory() {
        let root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/scenarios/rusted_kingdoms");
        let report = build_map_sweep(&root);

        assert_eq!(report.entries.len(), 47);
        assert_eq!(report.clean_count(), 44);
        assert_eq!(report.maps_with_findings(), 3);
        assert_eq!(report.category_count(SweepCategory::Tmx), 1);
        assert_eq!(report.category_count(SweepCategory::Collision), 0);
        assert_eq!(report.category_count(SweepCategory::Npc), 0);
        assert_eq!(report.category_count(SweepCategory::Portal), 0);
        assert_eq!(report.category_count(SweepCategory::Object), 2);

        let marshland = entry(&report, "zone_03_marshland");
        assert_eq!(
            findings(marshland, SweepCategory::Object),
            ["2 sign(s) present but dialogue `sign_zone_03_marshland` is missing"]
        );
        let mountain_pass_01 = entry(&report, "zone_06_mountain_pass_01");
        assert_eq!(
            findings(mountain_pass_01, SweepCategory::Object),
            ["1 sign(s) present but dialogue `sign_zone_06_mountain_pass_01` is missing"]
        );
    }
}
