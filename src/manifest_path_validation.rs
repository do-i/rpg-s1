//! Validation of every path-valued field in a scenario manifest.
//!
//! This module deliberately knows neither Bevy's asset server nor the host filesystem.  Callers
//! supply a [`ScenarioPathProbe`] that can inspect one already-validated, scenario-relative path.
//! Keeping the boundary this narrow lets runtime and the future headless validator apply the same
//! manifest rules while their filesystem adapters retain ADR 0002/0004 containment guarantees.

use crate::{
    scenario_manifest::{Manifest, ScenarioDirectoryPath},
    scenario_path::ScenarioRelativePath,
};
use std::fmt;

/// The kind of scenario entry selected by a manifest reference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScenarioEntryKind {
    File,
    Directory,
}

impl fmt::Display for ScenarioEntryKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::File => formatter.write_str("file"),
            Self::Directory => formatter.write_str("directory"),
        }
    }
}

/// The observable result of looking up a validated path within a scenario package.
///
/// This intentionally carries no host path or I/O error.  An adapter may use its own private
/// paths and canonicalization checks, but the shared manifest diagnostic remains rooted in the
/// source-authored scenario-relative value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScenarioPathProbeResult {
    File,
    Directory,
    Missing,
}

impl From<ScenarioEntryKind> for ScenarioPathProbeResult {
    fn from(kind: ScenarioEntryKind) -> Self {
        match kind {
            ScenarioEntryKind::File => Self::File,
            ScenarioEntryKind::Directory => Self::Directory,
        }
    }
}

/// Looks up one validated path in the active scenario package.
///
/// Implementations must not reinterpret the supplied value as an arbitrary host path.  In
/// particular, filesystem-backed implementations are responsible for rejecting symlink escapes
/// before reporting a result, as required by ADR 0002 and ADR 0004.
pub trait ScenarioPathProbe {
    fn probe(&self, path: &ScenarioRelativePath) -> ScenarioPathProbeResult;
}

impl<F> ScenarioPathProbe for F
where
    F: for<'a> Fn(&'a ScenarioRelativePath) -> ScenarioPathProbeResult,
{
    fn probe(&self, path: &ScenarioRelativePath) -> ScenarioPathProbeResult {
        self(path)
    }
}

/// One path-valued manifest field and the kind it must reference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManifestPathReference<'a> {
    /// Source YAML location, relative to the manifest document root.
    pub field_path: &'static str,
    /// The validated, normalized path authored at [`Self::field_path`].
    pub path: &'a ScenarioRelativePath,
    /// Whether the path must name a file or directory.
    pub expected_kind: ScenarioEntryKind,
}

/// A focused validation outcome for one manifest-owned path reference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManifestPathValidation<'a> {
    pub reference: ManifestPathReference<'a>,
    pub result: ManifestPathValidationResult,
}

impl fmt::Display for ManifestPathValidation<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let reference = self.reference;
        match self.result {
            ManifestPathValidationResult::Exists => write!(
                formatter,
                "{}: {}: {} exists",
                reference.field_path, reference.path, reference.expected_kind
            ),
            ManifestPathValidationResult::Missing => write!(
                formatter,
                "{}: {}: expected {} but it is missing",
                reference.field_path, reference.path, reference.expected_kind
            ),
            ManifestPathValidationResult::WrongKind { actual_kind } => write!(
                formatter,
                "{}: {}: expected {} but found {}",
                reference.field_path, reference.path, reference.expected_kind, actual_kind
            ),
        }
    }
}

/// Whether a manifest path reference has the required entry kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManifestPathValidationResult {
    Exists,
    Missing,
    WrongKind { actual_kind: ScenarioEntryKind },
}

fn file_reference<'a>(
    field_path: &'static str,
    path: &'a ScenarioRelativePath,
) -> ManifestPathReference<'a> {
    ManifestPathReference {
        field_path,
        path,
        expected_kind: ScenarioEntryKind::File,
    }
}

fn directory_reference<'a>(
    field_path: &'static str,
    path: &'a ScenarioDirectoryPath,
) -> ManifestPathReference<'a> {
    ManifestPathReference {
        field_path,
        path: path.as_relative_path(),
        expected_kind: ScenarioEntryKind::Directory,
    }
}

/// Enumerates every currently known manifest path reference.
///
/// Identifier fields such as `protagonist.id`, `protagonist.class`, and `start.map` deliberately
/// do not appear here: they name records, not scenario files.  Their cross-catalog validation is
/// owned by later milestones.
pub fn manifest_path_references(manifest: &Manifest) -> [ManifestPathReference<'_>; 28] {
    [
        file_reference("title.image", &manifest.title.image),
        file_reference("title.cursor_icon", &manifest.title.cursor_icon),
        file_reference("font.path", &manifest.font.path),
        file_reference("ui.menu_backdrop", &manifest.ui.menu_backdrop),
        file_reference("apothecary.sprite", &manifest.apothecary.sprite),
        file_reference("apothecary.icons.locked", &manifest.apothecary.icons.locked),
        file_reference("apothecary.icons.ready", &manifest.apothecary.icons.ready),
        file_reference(
            "apothecary.icons.missing",
            &manifest.apothecary.icons.missing,
        ),
        file_reference("inn.sprite", &manifest.inn.sprite),
        file_reference("item_shop.sprite", &manifest.item_shop.sprite),
        file_reference("weapon_shop.sprite", &manifest.weapon_shop.sprite),
        file_reference("armor_shop.sprite", &manifest.armor_shop.sprite),
        file_reference("item_box.sprite", &manifest.item_box.sprite),
        file_reference("protagonist.sprite", &manifest.protagonist.sprite),
        file_reference("start.intro_dialogue", &manifest.start.intro_dialogue),
        file_reference("refs.party", &manifest.refs.party),
        directory_reference("refs.classes", &manifest.refs.classes),
        directory_reference("refs.maps", &manifest.refs.maps),
        directory_reference("refs.dialogue", &manifest.refs.dialogue),
        directory_reference("refs.items", &manifest.refs.items),
        directory_reference("refs.enemies", &manifest.refs.enemies),
        directory_reference("refs.encount", &manifest.refs.encount),
        directory_reference("refs.recipe", &manifest.refs.recipe),
        file_reference("refs.quests", &manifest.refs.quests),
        file_reference("refs.balance", &manifest.refs.balance),
        file_reference("refs.battle_backgrounds", &manifest.refs.battle_backgrounds),
        directory_reference("refs.assets", &manifest.refs.assets),
        directory_reference("refs.tmx", &manifest.refs.tmx),
    ]
}

/// Validates every manifest path reference with a caller-supplied package probe.
///
/// The returned array preserves manifest-schema order and contains a result for every reference,
/// including successes, so callers may aggregate diagnostics without losing coverage.
pub fn validate_manifest_paths<'a>(
    manifest: &'a Manifest,
    probe: &impl ScenarioPathProbe,
) -> [ManifestPathValidation<'a>; 28] {
    manifest_path_references(manifest).map(|reference| {
        let result = match probe.probe(reference.path) {
            ScenarioPathProbeResult::Missing => ManifestPathValidationResult::Missing,
            ScenarioPathProbeResult::File if reference.expected_kind == ScenarioEntryKind::File => {
                ManifestPathValidationResult::Exists
            }
            ScenarioPathProbeResult::Directory
                if reference.expected_kind == ScenarioEntryKind::Directory =>
            {
                ManifestPathValidationResult::Exists
            }
            ScenarioPathProbeResult::File => ManifestPathValidationResult::WrongKind {
                actual_kind: ScenarioEntryKind::File,
            },
            ScenarioPathProbeResult::Directory => ManifestPathValidationResult::WrongKind {
                actual_kind: ScenarioEntryKind::Directory,
            },
        };
        ManifestPathValidation { reference, result }
    })
}

#[cfg(test)]
mod tests {
    use super::{
        ManifestPathValidationResult, ScenarioEntryKind, ScenarioPathProbe,
        ScenarioPathProbeResult, manifest_path_references, validate_manifest_paths,
    };
    use crate::{scenario_manifest::Manifest, scenario_path::ScenarioRelativePath, scenario_yaml};
    use std::{cell::RefCell, collections::BTreeMap};

    fn manifest() -> Manifest {
        scenario_yaml::from_str(include_str!(
            "../tests/fixtures/rusted-kingdoms-manifest-complete.yaml"
        ))
        .expect("the complete pinned-shaped manifest should deserialize")
    }

    struct Probe(BTreeMap<String, ScenarioPathProbeResult>);

    impl ScenarioPathProbe for Probe {
        fn probe(&self, path: &ScenarioRelativePath) -> ScenarioPathProbeResult {
            *self
                .0
                .get(path.as_str())
                .unwrap_or(&ScenarioPathProbeResult::Missing)
        }
    }

    fn successful_probe(manifest: &Manifest) -> Probe {
        Probe(
            manifest_path_references(manifest)
                .into_iter()
                .map(|reference| {
                    (
                        reference.path.as_str().to_owned(),
                        reference.expected_kind.into(),
                    )
                })
                .collect(),
        )
    }

    #[test]
    fn enumerates_every_path_valued_manifest_field_with_its_kind() {
        let manifest = manifest();
        let references = manifest_path_references(&manifest);

        assert_eq!(references.len(), 28);
        assert_eq!(
            references
                .iter()
                .filter(|reference| reference.expected_kind == ScenarioEntryKind::File)
                .count(),
            19
        );
        assert_eq!(
            references
                .iter()
                .filter(|reference| reference.expected_kind == ScenarioEntryKind::Directory)
                .count(),
            9
        );
        assert_eq!(
            references
                .iter()
                .map(|reference| (reference.field_path, reference.path.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (
                    "title.image",
                    "assets/images/title_bg/title_lost_flame.webp"
                ),
                (
                    "title.cursor_icon",
                    "assets/images/icons/arrow-head-right.webp"
                ),
                ("font.path", "assets/fonts/Philosopher-Regular.ttf"),
                (
                    "ui.menu_backdrop",
                    "assets/images/battle_bg/zone4-sanctum-bg-1280x468.webp"
                ),
                ("apothecary.sprite", "assets/sprites/npc/female_wiz_01.tsx"),
                (
                    "apothecary.icons.locked",
                    "assets/images/icons/lock-locked-red-small.webp"
                ),
                (
                    "apothecary.icons.ready",
                    "assets/images/icons/lock-unlocked-green-small.webp"
                ),
                (
                    "apothecary.icons.missing",
                    "assets/images/icons/lock-unlocked-yellow-small.webp"
                ),
                ("inn.sprite", "assets/sprites/npc/female_blue_01.tsx"),
                (
                    "item_shop.sprite",
                    "assets/sprites/npc/teen_halfmessy_01.tsx"
                ),
                (
                    "weapon_shop.sprite",
                    "assets/sprites/npc/male_sword_fighter_axe_fighter.tsx"
                ),
                (
                    "armor_shop.sprite",
                    "assets/sprites/npc/plate_knight_base.tsx"
                ),
                ("item_box.sprite", "assets/sprites/objects/item_box.tsx"),
                (
                    "protagonist.sprite",
                    "assets/sprites/party/01_aric_walk.tsx"
                ),
                ("start.intro_dialogue", "data/dialogue/intro_cutscene.yaml"),
                ("refs.party", "data/party.yaml"),
                ("refs.classes", "data/classes"),
                ("refs.maps", "data/maps"),
                ("refs.dialogue", "data/dialogue"),
                ("refs.items", "data/items"),
                ("refs.enemies", "data/enemies"),
                ("refs.encount", "data/encount"),
                ("refs.recipe", "data/recipe"),
                ("refs.quests", "data/quests.yaml"),
                ("refs.balance", "data/balance.yaml"),
                ("refs.battle_backgrounds", "data/battle_backgrounds.yaml"),
                ("refs.assets", "assets"),
                ("refs.tmx", "assets/maps"),
            ]
        );
    }

    #[test]
    fn validates_each_reference_when_probe_reports_its_expected_kind() {
        let manifest = manifest();
        let probe = successful_probe(&manifest);
        let results = validate_manifest_paths(&manifest, &probe);

        assert_eq!(results.len(), 28);
        assert!(
            results
                .iter()
                .all(|result| { result.result == ManifestPathValidationResult::Exists })
        );
    }

    #[test]
    fn reports_a_missing_file_with_its_owner_and_scenario_relative_path() {
        let manifest = manifest();
        let mut probe = successful_probe(&manifest);
        probe.0.insert(
            "assets/images/icons/arrow-head-right.webp".to_owned(),
            ScenarioPathProbeResult::Missing,
        );
        let results = validate_manifest_paths(&manifest, &probe);
        let missing = results
            .iter()
            .find(|result| result.reference.field_path == "title.cursor_icon")
            .expect("the cursor reference should be validated");

        assert_eq!(missing.result, ManifestPathValidationResult::Missing);
        assert_eq!(
            missing.to_string(),
            "title.cursor_icon: assets/images/icons/arrow-head-right.webp: expected file but it is missing"
        );
    }

    #[test]
    fn reports_a_missing_directory_with_its_owner_and_scenario_relative_path() {
        let manifest = manifest();
        let mut probe = successful_probe(&manifest);
        probe
            .0
            .insert("data/encount".to_owned(), ScenarioPathProbeResult::Missing);
        let results = validate_manifest_paths(&manifest, &probe);
        let missing = results
            .iter()
            .find(|result| result.reference.field_path == "refs.encount")
            .expect("the encount root should be validated");

        assert_eq!(missing.result, ManifestPathValidationResult::Missing);
        assert_eq!(
            missing.to_string(),
            "refs.encount: data/encount: expected directory but it is missing"
        );
    }

    #[test]
    fn reports_wrong_entry_kind_without_losing_the_reference_context() {
        let manifest = manifest();
        let mut probe = successful_probe(&manifest);
        probe
            .0
            .insert("data/classes".to_owned(), ScenarioPathProbeResult::File);
        let results = validate_manifest_paths(&manifest, &probe);
        let wrong_kind = results
            .iter()
            .find(|result| result.reference.field_path == "refs.classes")
            .expect("the classes root should be validated");

        assert_eq!(
            wrong_kind.result,
            ManifestPathValidationResult::WrongKind {
                actual_kind: ScenarioEntryKind::File
            }
        );
        assert_eq!(
            wrong_kind.to_string(),
            "refs.classes: data/classes: expected directory but found file"
        );
    }

    #[test]
    fn probe_only_receives_scenario_relative_values() {
        let manifest = manifest();
        struct RecordingProbe(RefCell<BTreeMap<String, ()>>);

        impl ScenarioPathProbe for RecordingProbe {
            fn probe(&self, path: &ScenarioRelativePath) -> ScenarioPathProbeResult {
                self.0.borrow_mut().insert(path.as_str().to_owned(), ());
                ScenarioPathProbeResult::Missing
            }
        }

        let probe = RecordingProbe(RefCell::new(BTreeMap::new()));
        let results = validate_manifest_paths(&manifest, &probe);

        let seen = probe.0.into_inner();
        assert_eq!(seen.len(), 28);
        assert!(seen.keys().all(|path| !path.starts_with('/')));
        assert!(
            results
                .iter()
                .all(|result| result.result == ManifestPathValidationResult::Missing)
        );
    }
}
