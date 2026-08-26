//! Per-dialogue reachability and effects report for the `dialogue-report` CLI subcommand.
//!
//! Every source-authored dialogue entry (a `DialogueDocument::Entries` conditional branch) is
//! evaluated against the pinned Python engine's first-match rule from
//! `engine/dialogue/dialogue_engine.py`: entries are tried top to bottom, an entry matches when
//! every `requires` flag is set and no `excludes` flag is set, and the first match wins. An entry
//! is dead when no assignment of the document's referenced flags ever lets it win. This mirrors
//! [`crate::world_dialogue::DialogueSession::resolve`], which only ever selects a `node`-less
//! entry by this same rule; `node` entries are graph targets reached by `next` or a choice, not
//! flag-gated branches, so they are reported separately and never marked dead.
//!
//! One pinned dialogue, `ardel_fisherman`, ends with two flavor entries that are provably dead
//! under this rule. The target keeps that source content deliberately, and this report marks
//! exactly those two entries "documented-accepted" rather than surfacing them as a new finding.
//! `millhaven_carter`, `harborgate_fishwife`, and `ruinwatch_digger` carry the identical pattern
//! and are accepted the same way. ADR 0007 records the inherited-data decision and exact entries.
//!
//! Running this report against the shipped scenario turns up the same shape of dead trailing
//! entry in three more pinned dialogues (`ashenveil_ashgatherer`, `elder_intro`,
//! `frostholm_courtier`) — every one a sub-quest giver whose first four
//! entries already exhaustively partition the relevant `sq_*_started`/`_relayed`/`_done` states
//! (or, for `elder_intro`, a reward entry and its own source-commented "safety fallback"
//! duplicate), leaving the trailing flavor or story-flag entry unreachable under the pinned
//! first-match rule regardless of any other flag. None of these are in
//! [`DOCUMENTED_DEAD_ENTRIES`], so they surface as `DialogueReachability::Dead` findings, not
//! `DeadAccepted` ones; they are accepted only once a wave reaches them and the ledger names
//! them. This module does not alter the pinned YAML — it reports what is there.
//!
//! Like [`crate::scenario_map_report`], this command is informational only: it never fails the
//! process, and it deliberately does not repeat the item/flag reference checks
//! [`crate::scenario_cross_reference::validate_scenario_directory`] already performs at error
//! level — unresolved item ids show up here only as informational notes.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    scenario_dialogue::{DialogueActions, DialogueDocument, DialogueEntry, EntryDialogueKind},
    scenario_item::ItemCatalogFile,
    scenario_manifest::Manifest,
    scenario_yaml,
};

/// Enumeration gives up beyond this many referenced flags per document; the pinned corpus's
/// worst case is 5 flags (32 states), so this leaves ample headroom while keeping the exhaustive
/// search bounded (2^16 = 65536 states per document, times a handful of entries each).
const MAX_ENUMERATED_FLAGS: usize = 16;

/// The dialogue ids whose dead entries are accepted, deliberately-kept source content rather than
/// a migration finding. ADR 0007 records the inherited-data decision and exact entries.
const DOCUMENTED_DEAD_ENTRIES: &[(&str, &[usize])] = &[
    ("ardel_fisherman", &[4, 5]),
    ("millhaven_carter", &[4, 5]),
    ("harborgate_fishwife", &[4]),
    ("ruinwatch_digger", &[4]),
];

/// The complete dialogue report for one scenario package.
pub(crate) struct DialogueReport {
    pub(crate) scenario_id: Option<String>,
    pub(crate) scenario_name: Option<String>,
    pub(crate) documents: Vec<DialogueReportDocument>,
    /// Set only when the manifest (or its referenced directories) could not be read at all.
    pub(crate) load_error: Option<String>,
}

impl DialogueReport {
    pub(crate) fn with_load_error(message: impl Into<String>) -> Self {
        Self {
            scenario_id: None,
            scenario_name: None,
            documents: Vec::new(),
            load_error: Some(message.into()),
        }
    }

    /// A document is clean when it has no undocumented dead entries, no zero-line reachable
    /// entries, and no informational notes at all.
    pub(crate) fn clean_count(&self) -> usize {
        self.documents.iter().filter(|doc| doc.is_clean()).count()
    }

    /// Documents containing at least one dead entry not already accepted as documented.
    pub(crate) fn documents_with_new_dead_entries(&self) -> usize {
        self.documents
            .iter()
            .filter(|doc| doc.entries.iter().any(DialogueReportEntry::is_new_dead))
            .count()
    }

    /// Documents containing only known-accepted dead flavor entries (see
    /// [`DOCUMENTED_DEAD_ENTRIES`]).
    pub(crate) fn documents_with_only_accepted_dead_entries(&self) -> usize {
        self.documents
            .iter()
            .filter(|doc| {
                let dead = doc
                    .entries
                    .iter()
                    .filter(|entry| entry.reachability.is_dead())
                    .count();
                dead > 0 && !doc.entries.iter().any(DialogueReportEntry::is_new_dead)
            })
            .count()
    }

    pub(crate) fn documents_with_zero_line_entries(&self) -> usize {
        self.documents
            .iter()
            .filter(|doc| doc.entries.iter().any(|entry| entry.line_count == 0))
            .count()
    }

    pub(crate) fn documents_with_notes(&self) -> usize {
        self.documents
            .iter()
            .filter(|doc| !doc.notes().is_empty())
            .count()
    }
}

/// One `data/dialogue/*.yaml` document.
pub(crate) struct DialogueReportDocument {
    pub(crate) id: String,
    pub(crate) file_stem: String,
    pub(crate) shape: DialogueReportShape,
    /// Empty for cutscenes and line pools, which have no condition-selected entries.
    pub(crate) entries: Vec<DialogueReportEntry>,
    pub(crate) referenced_flag_count: usize,
    pub(crate) too_many_flags: bool,
    /// A cutscene's or line pool's line count; entry line counts live on `entries` instead.
    pub(crate) pool_line_count: Option<usize>,
}

impl DialogueReportDocument {
    pub(crate) fn is_clean(&self) -> bool {
        !self.too_many_flags
            && self.notes().is_empty()
            && self
                .entries
                .iter()
                .all(|entry| !entry.is_new_dead() && entry.line_count > 0)
    }

    /// Every informational note for this document, in a stable order: entry-level notes first in
    /// entry order, document-level notes last.
    pub(crate) fn notes(&self) -> Vec<String> {
        let mut notes = Vec::new();
        for entry in &self.entries {
            notes.extend(entry.notes.iter().cloned());
        }
        if self.too_many_flags {
            notes.push(format!(
                "too many flags to enumerate ({} referenced, cap is {MAX_ENUMERATED_FLAGS})",
                self.referenced_flag_count
            ));
        }
        if self.id != self.file_stem {
            notes.push(format!(
                "authored id `{}` differs from filename stem `{}`",
                self.id, self.file_stem
            ));
        }
        notes
    }
}

pub(crate) enum DialogueReportShape {
    Cutscene,
    Entries(Option<EntryDialogueKind>),
    LinePool,
}

/// One condition-branch entry, or one `node` graph entry reported for completeness.
pub(crate) struct DialogueReportEntry {
    pub(crate) index: usize,
    pub(crate) node: Option<String>,
    pub(crate) requires: Vec<String>,
    pub(crate) excludes: Vec<String>,
    pub(crate) line_count: usize,
    pub(crate) reachability: DialogueReachability,
    pub(crate) effects: EffectsSummary,
    /// Informational notes scoped to this entry (unresolved item ids, etc).
    pub(crate) notes: Vec<String>,
}

impl DialogueReportEntry {
    /// True for a dead entry that is not one of the documented-accepted pairs (see
    /// [`DOCUMENTED_DEAD_ENTRIES`]).
    pub(crate) fn is_new_dead(&self) -> bool {
        matches!(self.reachability, DialogueReachability::Dead)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DialogueReachability {
    /// Reached by some assignment of the document's referenced flags under first-match order.
    Reachable,
    /// Never wins under any assignment: a genuine finding.
    Dead,
    /// Never wins under any assignment, but accepted as deliberately-kept source content.
    DeadAccepted,
    /// A `node` entry: reached by graph edge (`next` or a choice target), not by flag condition.
    GraphNode,
    /// The document exceeded the enumeration cap; reachability could not be determined.
    Unknown,
}

impl DialogueReachability {
    fn is_dead(self) -> bool {
        matches!(self, Self::Dead | Self::DeadAccepted)
    }
}

/// The side effects an entry's (or cutscene's) `on_complete` can carry, as modeled by
/// [`crate::scenario_dialogue::DialogueActions`].
#[derive(Default)]
pub(crate) struct EffectsSummary {
    pub(crate) set_flags: Vec<String>,
    pub(crate) unset_flags: Vec<String>,
    /// Flags among `set_flags`/`unset_flags` that follow the source's `unlock` sugar convention
    /// `<kind>_<id>_unlocked` for kind in {recipe, spell, location, transport}: reported as
    /// `(kind, id)`. The pinned corpus always spells these as a direct `set_flag`; the schema has
    /// no separate raw `unlock` action because no pinned document uses one (confirmed against
    /// both the Rust and Python corpora), so this is purely an informational classification of
    /// `set_flag` values that happen to follow the convention Python's `unlock` sugar expands to.
    pub(crate) unlock_flags: Vec<(String, String)>,
    pub(crate) give_items: Vec<(String, u32)>,
    pub(crate) join_party: Option<String>,
    pub(crate) transition_map: Option<String>,
    pub(crate) open_shop: Option<&'static str>,
    pub(crate) open_inn: bool,
    pub(crate) open_apothecary: bool,
}

impl EffectsSummary {
    pub(crate) fn is_empty(&self) -> bool {
        self.set_flags.is_empty()
            && self.unset_flags.is_empty()
            && self.give_items.is_empty()
            && self.join_party.is_none()
            && self.transition_map.is_none()
            && self.open_shop.is_none()
            && !self.open_inn
            && !self.open_apothecary
    }
}

/// The four naming-convention unlock kinds recognized by the pinned Python `unlock` sugar
/// (`engine/dialogue/dialogue_engine.py`'s `UNLOCK_KINDS`).
const UNLOCK_KINDS: &[&str] = &["recipe", "spell", "location", "transport"];

fn classify_unlock_flag(flag: &str) -> Option<(String, String)> {
    let id = flag.strip_suffix("_unlocked")?;
    for kind in UNLOCK_KINDS {
        if let Some(rest) = id
            .strip_prefix(kind)
            .and_then(|rest| rest.strip_prefix('_'))
            && !rest.is_empty()
        {
            return Some(((*kind).to_owned(), rest.to_owned()));
        }
    }
    None
}

fn effects_summary(actions: &DialogueActions) -> EffectsSummary {
    let mut summary = EffectsSummary {
        set_flags: actions
            .set_flag
            .as_ref()
            .map(|flags| flags.as_slice().to_vec())
            .unwrap_or_default(),
        unset_flags: actions
            .unset_flag
            .as_ref()
            .map(|flags| flags.as_slice().to_vec())
            .unwrap_or_default(),
        give_items: actions
            .give_items
            .iter()
            .map(|grant| (grant.id.clone(), grant.qty.get()))
            .collect(),
        join_party: actions.join_party.clone(),
        transition_map: actions.transition.as_ref().map(|t| t.map.clone()),
        open_shop: actions.open_shop.map(|kind| match kind {
            crate::scenario_dialogue::DialogueShopKind::Item => "item",
            crate::scenario_dialogue::DialogueShopKind::Weapon => "weapon",
            crate::scenario_dialogue::DialogueShopKind::Armor => "armor",
            crate::scenario_dialogue::DialogueShopKind::MagicCore => "magic_core",
        }),
        open_inn: actions.open_inn.is_some(),
        open_apothecary: actions.open_apothecary.is_some(),
        unlock_flags: Vec::new(),
    };
    summary.unlock_flags = summary
        .set_flags
        .iter()
        .chain(summary.unset_flags.iter())
        .filter_map(|flag| classify_unlock_flag(flag))
        .collect();
    summary
}

/// Notes unresolved `give_items` ids informationally; does not duplicate error-level validation.
fn item_notes(effects: &EffectsSummary, known_items: &BTreeSet<String>) -> Vec<String> {
    effects
        .give_items
        .iter()
        .filter(|(id, _)| !known_items.contains(id))
        .map(|(id, _)| format!("give_items references unknown item id `{id}`"))
        .collect()
}

/// Builds the complete report by reading the manifest and its dialogue and item directories
/// directly from `physical_root`. Never fails: an unreadable manifest yields an empty, clearly
/// marked report rather than a process error, since this command is informational only.
pub(crate) fn build_dialogue_report(physical_root: &Path) -> DialogueReport {
    let canonical_root = physical_root.canonicalize().ok();
    let Some(manifest) = read_manifest(physical_root, canonical_root.as_deref()) else {
        return DialogueReport::with_load_error("manifest.yaml is unavailable or invalid");
    };

    let dialogue_dir = physical_root.join(manifest.refs.dialogue.as_str());
    let items_dir = physical_root.join(manifest.refs.items.as_str());
    let known_items = read_known_item_ids(&items_dir, canonical_root.as_deref());

    let Some(mut paths) = safe_read_dir(&dialogue_dir, canonical_root.as_deref()) else {
        return DialogueReport {
            scenario_id: Some(manifest.id.clone()),
            scenario_name: Some(manifest.name.clone()),
            documents: Vec::new(),
            load_error: Some("dialogue directory is unavailable".to_owned()),
        };
    };
    paths.retain(|path| {
        path.extension()
            .is_some_and(|extension| extension == "yaml")
    });
    paths.sort();

    let documents = paths
        .iter()
        .filter_map(|path| build_document(path, &known_items))
        .collect();

    DialogueReport {
        scenario_id: Some(manifest.id.clone()),
        scenario_name: Some(manifest.name.clone()),
        documents,
        load_error: None,
    }
}

fn build_document(path: &Path, known_items: &BTreeSet<String>) -> Option<DialogueReportDocument> {
    let stem = path.file_stem().and_then(|stem| stem.to_str())?.to_owned();
    let text = fs::read_to_string(path).ok()?;
    let document: DialogueDocument = scenario_yaml::from_str(&text).ok()?;
    let id = document.effective_id(&stem).to_owned();

    Some(match &document {
        DialogueDocument::Cutscene(cutscene) => {
            let effects = effects_summary(&cutscene.on_complete);
            let notes = item_notes(&effects, known_items);
            DialogueReportDocument {
                id,
                file_stem: stem,
                shape: DialogueReportShape::Cutscene,
                entries: vec![DialogueReportEntry {
                    index: 0,
                    node: None,
                    requires: Vec::new(),
                    excludes: Vec::new(),
                    line_count: cutscene.lines.len(),
                    reachability: DialogueReachability::Reachable,
                    effects,
                    notes,
                }],
                referenced_flag_count: 0,
                too_many_flags: false,
                pool_line_count: None,
            }
        }
        DialogueDocument::LinePool(pool) => DialogueReportDocument {
            id,
            file_stem: stem,
            shape: DialogueReportShape::LinePool,
            entries: Vec::new(),
            referenced_flag_count: 0,
            too_many_flags: false,
            pool_line_count: Some(pool.lines.len()),
        },
        DialogueDocument::Entries(entries) => {
            let (entries, referenced_flag_count, too_many_flags) =
                report_entries(&entries.entries, known_items);
            DialogueReportDocument {
                id: id.clone(),
                file_stem: stem,
                shape: DialogueReportShape::Entries(entries_kind(&document)),
                entries: mark_documented_dead(&id, entries),
                referenced_flag_count,
                too_many_flags,
                pool_line_count: None,
            }
        }
    })
}

fn entries_kind(document: &DialogueDocument) -> Option<EntryDialogueKind> {
    match document {
        DialogueDocument::Entries(entries) => entries.kind,
        _ => None,
    }
}

/// Computes per-entry line counts, effects, and reachability. `node`-bearing entries are reported
/// as [`DialogueReachability::GraphNode`] and excluded from the first-match enumeration, exactly
/// matching [`crate::world_dialogue::DialogueSession::resolve`]'s `node.is_none()` guard.
fn report_entries(
    entries: &[DialogueEntry],
    known_items: &BTreeSet<String>,
) -> (Vec<DialogueReportEntry>, usize, bool) {
    let referenced_flags = referenced_flags(entries);
    let too_many_flags = referenced_flags.len() > MAX_ENUMERATED_FLAGS;

    let candidate_indices = entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.node.is_none())
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let dead_candidates = if too_many_flags {
        BTreeSet::new()
    } else {
        dead_entry_indices(entries, &candidate_indices, &referenced_flags)
    };

    let report_entries = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let effects = effects_summary(&entry.on_complete);
            let mut notes = item_notes(&effects, known_items);
            let reachability = if entry.node.is_some() {
                DialogueReachability::GraphNode
            } else if too_many_flags {
                DialogueReachability::Unknown
            } else if dead_candidates.contains(&index) {
                DialogueReachability::Dead
            } else {
                DialogueReachability::Reachable
            };
            if reachability == DialogueReachability::Dead {
                notes.push(format!(
                    "entry [{index}] is dead: no assignment of the document's referenced flags \
                     ever lets it win first-match"
                ));
            }
            DialogueReportEntry {
                index,
                node: entry.node.clone(),
                requires: entry.condition.requires.clone(),
                excludes: entry.condition.excludes.clone(),
                line_count: entry.lines.len(),
                reachability,
                effects,
                notes,
            }
        })
        .collect();

    (report_entries, referenced_flags.len(), too_many_flags)
}

/// Every flag referenced by a `node`-less entry's condition, in first-appearance order. Only
/// `node`-less entries participate: their conditions are the only ones the runtime ever tests
/// (see the module doc comment).
fn referenced_flags(entries: &[DialogueEntry]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut flags = Vec::new();
    for entry in entries.iter().filter(|entry| entry.node.is_none()) {
        for flag in entry
            .condition
            .requires
            .iter()
            .chain(&entry.condition.excludes)
        {
            if seen.insert(flag.clone()) {
                flags.push(flag.clone());
            }
        }
    }
    flags
}

/// Exhaustively enumerates every assignment of `flags` and records which `node`-less entry index
/// (from `candidate_indices`, in document order) wins first-match at each assignment. Any
/// candidate index that never wins is dead.
fn dead_entry_indices(
    entries: &[DialogueEntry],
    candidate_indices: &[usize],
    flags: &[String],
) -> BTreeSet<usize> {
    let mut reachable = BTreeSet::new();
    let state_count = 1usize << flags.len();
    for mask in 0..state_count {
        let has_flag = |flag: &str| -> bool {
            flags
                .iter()
                .position(|candidate| candidate == flag)
                .is_some_and(|bit| mask & (1 << bit) != 0)
        };
        if let Some(&winner) = candidate_indices
            .iter()
            .find(|&&index| entries[index].condition.is_satisfied_by(has_flag))
        {
            reachable.insert(winner);
        }
    }
    candidate_indices
        .iter()
        .copied()
        .filter(|index| !reachable.contains(index))
        .collect()
}

/// Downgrades entries at documented dead-entry positions from [`DialogueReachability::Dead`] to
/// [`DialogueReachability::DeadAccepted`] and adjusts their note accordingly.
fn mark_documented_dead(id: &str, entries: Vec<DialogueReportEntry>) -> Vec<DialogueReportEntry> {
    let accepted_indices: &[usize] = DOCUMENTED_DEAD_ENTRIES
        .iter()
        .find(|entry| entry.0 == id)
        .map(|entry| entry.1)
        .unwrap_or(&[]);
    entries
        .into_iter()
        .map(|mut entry| {
            if accepted_indices.contains(&entry.index)
                && entry.reachability == DialogueReachability::Dead
            {
                entry.reachability = DialogueReachability::DeadAccepted;
                entry.notes.retain(|note| !note.contains("is dead"));
                entry.notes.push(format!(
                    "entry [{}] is dead but documented-accepted: pinned `{id}` flavor content \
                     kept deliberately (docs/adr/0007-inherited-scenario-data-debt.md)",
                    entry.index
                ));
            }
            entry
        })
        .collect()
}

fn read_manifest(physical_root: &Path, canonical_root: Option<&Path>) -> Option<Manifest> {
    let text = read_file_safely(physical_root, canonical_root, "manifest.yaml")?;
    scenario_yaml::from_str(&text).ok()
}

fn read_known_item_ids(dir: &Path, canonical_root: Option<&Path>) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    let Some(paths) = safe_read_dir(dir, canonical_root) else {
        return ids;
    };
    for path in paths {
        // `field_use.yaml` is a dispatch catalog keyed by item id, not an `ItemCatalogFile`; item
        // identity itself always comes from the other list-root metadata files.
        if path
            .extension()
            .is_some_and(|extension| extension == "yaml")
            && path.file_name().and_then(|name| name.to_str()) != Some("field_use.yaml")
            && let Ok(text) = fs::read_to_string(&path)
            && let Ok(catalog) = scenario_yaml::from_str::<ItemCatalogFile>(&text)
        {
            ids.extend(catalog.entries().iter().map(|item| item.id().to_owned()));
        }
    }
    ids
}

/// Lists the files directly within `dir`, refusing to follow it outside `canonical_root`.
/// `data/dialogue` and `data/items` are flat in the pinned scenario corpus, so this deliberately
/// does not recurse.
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
                "rpg-s1-dialogue-report-{}-{nonce}-{}",
                std::process::id(),
                NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(root.join("data/dialogue")).unwrap();
            fs::create_dir_all(root.join("data/items")).unwrap();
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
    }

    impl Drop for TempScenario {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("temporary scenario should be removable");
        }
    }

    #[test]
    fn classifies_unlock_convention_flags_for_all_four_kinds() {
        assert_eq!(
            classify_unlock_flag("transport_sail_unlocked"),
            Some(("transport".to_owned(), "sail".to_owned()))
        );
        assert_eq!(
            classify_unlock_flag("recipe_heal_potion_unlocked"),
            Some(("recipe".to_owned(), "heal_potion".to_owned()))
        );
        assert_eq!(
            classify_unlock_flag("spell_ward_unlocked"),
            Some(("spell".to_owned(), "ward".to_owned()))
        );
        assert_eq!(
            classify_unlock_flag("location_ashenveil_unlocked"),
            Some(("location".to_owned(), "ashenveil".to_owned()))
        );
        assert_eq!(classify_unlock_flag("npc_mira_joined"), None);
        assert_eq!(classify_unlock_flag("story_quest_started"), None);
    }

    #[test]
    fn linear_partition_leaves_no_entry_dead() {
        let fixture = TempScenario::new();
        fixture.write(
            "data/dialogue/village_elder.yaml",
            r#"id: village_elder
type: npc
entries:
  - condition: { requires: [quest_done] }
    lines: ["Thank you."]
  - condition: { requires: [quest_started], excludes: [quest_done] }
    lines: ["Please hurry."]
  - condition: { excludes: [quest_started] }
    lines: ["Would you help?"]
    on_complete: { set_flag: quest_started }
"#,
        );

        let report = build_dialogue_report(&fixture.0);
        let doc = report
            .documents
            .iter()
            .find(|doc| doc.id == "village_elder")
            .unwrap();
        assert_eq!(doc.referenced_flag_count, 2);
        assert!(!doc.too_many_flags);
        assert!(
            doc.entries
                .iter()
                .all(|entry| entry.reachability == DialogueReachability::Reachable)
        );
        assert!(doc.is_clean());
    }

    #[test]
    fn an_entry_fully_shadowed_by_an_earlier_entry_is_reported_dead() {
        let fixture = TempScenario::new();
        fixture.write(
            "data/dialogue/shadowed.yaml",
            r#"id: shadowed
type: npc
entries:
  - lines: ["Always matches; nothing has a condition."]
  - condition: { requires: [never_reachable_because_first_entry_has_no_condition] }
    lines: ["Can never win."]
"#,
        );

        let report = build_dialogue_report(&fixture.0);
        let doc = report
            .documents
            .iter()
            .find(|doc| doc.id == "shadowed")
            .unwrap();
        assert_eq!(doc.entries[0].reachability, DialogueReachability::Reachable);
        assert_eq!(doc.entries[1].reachability, DialogueReachability::Dead);
        assert!(doc.entries[1].is_new_dead());
        assert!(!doc.is_clean());
        assert_eq!(report.documents_with_new_dead_entries(), 1);
    }

    #[test]
    fn ardel_fisherman_dead_entries_are_marked_documented_accepted_not_a_new_finding() {
        let fixture = TempScenario::new();
        fixture.write(
            "data/dialogue/ardel_fisherman.yaml",
            include_str!(
                "../../../assets/scenarios/rusted_kingdoms/data/dialogue/ardel_fisherman.yaml"
            ),
        );

        let report = build_dialogue_report(&fixture.0);
        let doc = report
            .documents
            .iter()
            .find(|doc| doc.id == "ardel_fisherman")
            .unwrap();
        assert_eq!(
            doc.entries[4].reachability,
            DialogueReachability::DeadAccepted
        );
        assert_eq!(
            doc.entries[5].reachability,
            DialogueReachability::DeadAccepted
        );
        assert!(!doc.entries[4].is_new_dead());
        assert!(!doc.entries[5].is_new_dead());
        assert_eq!(report.documents_with_new_dead_entries(), 0);
        assert_eq!(report.documents_with_only_accepted_dead_entries(), 1);
    }

    #[test]
    fn too_many_referenced_flags_is_reported_rather_than_enumerated() {
        let fixture = TempScenario::new();
        let mut yaml = String::from("id: overloaded\ntype: npc\nentries:\n");
        for index in 0..17 {
            yaml.push_str(&format!(
                "  - condition: {{ requires: [flag_{index}] }}\n    lines: [\"Line {index}.\"]\n"
            ));
        }
        fixture.write("data/dialogue/overloaded.yaml", &yaml);

        let report = build_dialogue_report(&fixture.0);
        let doc = report
            .documents
            .iter()
            .find(|doc| doc.id == "overloaded")
            .unwrap();
        assert!(doc.too_many_flags);
        assert_eq!(doc.referenced_flag_count, 17);
        assert!(
            doc.entries
                .iter()
                .all(|entry| entry.reachability == DialogueReachability::Unknown)
        );
        assert!(
            doc.notes()
                .iter()
                .any(|note| note.contains("too many flags"))
        );
    }

    #[test]
    fn zero_line_entries_are_reported() {
        let fixture = TempScenario::new();
        fixture.write(
            "data/dialogue/empty_lines.yaml",
            "id: empty_lines\ntype: npc\nentries:\n  - lines: []\n",
        );

        let report = build_dialogue_report(&fixture.0);
        let doc = report
            .documents
            .iter()
            .find(|doc| doc.id == "empty_lines")
            .unwrap();
        assert_eq!(doc.entries[0].line_count, 0);
        assert!(!doc.is_clean());
        assert_eq!(report.documents_with_zero_line_entries(), 1);
    }

    #[test]
    fn unresolved_item_grant_is_an_informational_note_not_a_dead_entry() {
        let fixture = TempScenario::new();
        fixture.write(
            "data/dialogue/rewarder.yaml",
            "id: rewarder\ntype: npc\nentries:\n  - lines: [\"Take this.\"]\n    on_complete: { give_items: [{ id: ghost_item, qty: 1 }] }\n",
        );

        let report = build_dialogue_report(&fixture.0);
        let doc = report
            .documents
            .iter()
            .find(|doc| doc.id == "rewarder")
            .unwrap();
        assert_eq!(doc.entries[0].reachability, DialogueReachability::Reachable);
        assert!(
            doc.entries[0]
                .notes
                .iter()
                .any(|note| note.contains("unknown item id `ghost_item`"))
        );
        assert_eq!(report.documents_with_notes(), 1);
    }

    #[test]
    fn cutscenes_and_line_pools_are_reported_without_reachability_analysis() {
        let fixture = TempScenario::new();
        fixture.write(
            "data/dialogue/intro.yaml",
            "id: intro\ntype: cutscene\nlines: [\"Once upon a time.\"]\non_complete: { set_flag: story_started }\n",
        );
        fixture.write(
            "data/dialogue/excuses.yaml",
            "lines: [\"Not now.\", \"Busy.\"]\n",
        );

        let report = build_dialogue_report(&fixture.0);
        let cutscene = report
            .documents
            .iter()
            .find(|doc| doc.id == "intro")
            .unwrap();
        assert!(matches!(cutscene.shape, DialogueReportShape::Cutscene));
        assert_eq!(cutscene.entries.len(), 1);
        assert_eq!(cutscene.entries[0].effects.set_flags, ["story_started"]);

        let pool = report
            .documents
            .iter()
            .find(|doc| doc.id == "excuses")
            .unwrap();
        assert!(matches!(pool.shape, DialogueReportShape::LinePool));
        assert_eq!(pool.pool_line_count, Some(2));
        assert!(pool.entries.is_empty());
        assert!(pool.is_clean());
    }

    #[test]
    fn unreadable_manifest_yields_an_empty_report_with_a_load_error() {
        let report = build_dialogue_report(Path::new("/nonexistent/rpg-s1-scenario"));
        assert!(report.load_error.is_some());
        assert!(report.documents.is_empty());
    }

    /// Pins the exact reachability findings against the shipped scenario. `ardel_fisherman`,
    /// `millhaven_carter`, `harborgate_fishwife`, and `ruinwatch_digger` are the only dialogues
    /// named as accepted in `docs/adr/0007-inherited-scenario-data-debt.md`, so they are the only
    /// ones this report accepts as documented; the other three carry the identical pinned-source dead
    /// trailing-entry pattern (see the module doc comment) and surface as new findings until the
    /// wave that reaches them documents them too. This test exists to catch regressions in the
    /// reachability algorithm itself, not to bless the count — a real drop in findings (fewer
    /// dead entries) is as worth investigating as a rise.
    #[test]
    fn the_shipped_scenario_matches_the_known_dead_entry_inventory() {
        let root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/scenarios/rusted_kingdoms");
        let report = build_dialogue_report(&root);
        assert!(report.load_error.is_none());
        assert_eq!(report.documents.len(), 91);
        assert!(!report.documents.iter().any(|doc| doc.too_many_flags));

        assert_eq!(report.documents_with_only_accepted_dead_entries(), 4);
        let expected_accepted: &[(&str, &[usize])] = &[
            ("ardel_fisherman", &[4, 5]),
            ("millhaven_carter", &[4, 5]),
            ("harborgate_fishwife", &[4]),
            ("ruinwatch_digger", &[4]),
        ];
        for (id, dead_indices) in expected_accepted {
            let document = report.documents.iter().find(|doc| doc.id == *id).unwrap();
            for index in *dead_indices {
                assert_eq!(
                    document.entries[*index].reachability,
                    DialogueReachability::DeadAccepted,
                    "entry [{index}] in `{id}` should be documented-accepted"
                );
            }
        }

        let expected_new_dead: &[(&str, &[usize])] = &[
            ("ashenveil_ashgatherer", &[5]),
            ("elder_intro", &[2]),
            ("frostholm_courtier", &[4, 5]),
        ];
        assert_eq!(
            report.documents_with_new_dead_entries(),
            expected_new_dead.len()
        );
        for (id, dead_indices) in expected_new_dead {
            let document = report.documents.iter().find(|doc| doc.id == *id).unwrap();
            let found = document
                .entries
                .iter()
                .filter(|entry| entry.is_new_dead())
                .map(|entry| entry.index)
                .collect::<Vec<_>>();
            assert_eq!(&found, dead_indices, "unexpected dead entries in `{id}`");
        }
    }
}
