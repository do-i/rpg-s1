//! Source-authored encounter-zone schemas.
//!
//! The pinned `0897035` scenario contains 16 mapping-root files beneath `data/encount/`.
//! Together they define 147 weighted formations, ten bosses, six barrier-enemy rules, and two
//! zone-level spawn-frequency overrides. Encounter files are selected by map filename stem. The
//! source currently authors an identical `id`, while the Python loader falls back to that stem
//! when `id` is absent.
//!
//! Missing names and battle-background ids become empty strings, missing formation/barrier
//! lists become empty, boss names default to their enemy id, bosses default to once-only, and a
//! missing or null boss completion block has no flag. A null boss or spawn frequency is also
//! absent. Formation selection, boss completion, barrier resolution, and spawning are runtime
//! concerns reserved for later milestones.

use bevy::{asset::Asset, reflect::TypePath};

use crate::scenario_class::{PositiveFinite, UnitInterval};
use crate::scenario_yaml::{deserialize_string, deserialize_strings};
use serde::{Deserialize, Deserializer};

/// One map-keyed encounter-zone document beneath `data/encount/`.
#[derive(Asset, Clone, Debug, Deserialize, PartialEq, TypePath)]
#[serde(deny_unknown_fields)]
pub struct EncounterZone {
    /// Authored identity. Missing values use the containing filename stem.
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub id: Option<String>,
    #[serde(default, deserialize_with = "deserialize_string")]
    pub name: String,
    /// Per-tick spawn probability, constrained by the source DTO's documented `0..=1` range.
    pub density: UnitInterval,
    #[serde(default)]
    pub entries: Vec<EncounterFormation>,
    /// Missing and explicit null both mean that the zone has no boss.
    #[serde(default)]
    pub boss: Option<EncounterBoss>,
    #[serde(default)]
    pub barrier_enemies: Vec<BarrierEnemy>,
    /// Battle-background catalog id, not a scenario-relative filesystem path.
    #[serde(default, deserialize_with = "deserialize_string")]
    pub background: String,
    /// Positive seconds between reactivation ticks. Missing and null use later fallback layers.
    #[serde(default)]
    pub spawn_frequency: Option<PositiveFinite>,
}

impl EncounterZone {
    /// Returns the authored id, or the same-stem filename identity when `id` is absent.
    pub fn effective_id<'a>(&'a self, filename_stem: &'a str) -> &'a str {
        self.id.as_deref().unwrap_or(filename_stem)
    }

    /// Preserves the authored integer weights while exposing their widened total.
    pub fn total_weight(&self) -> u64 {
        self.entries
            .iter()
            .map(|entry| u64::from(entry.weight))
            .sum()
    }
}

/// One weighted, ordered enemy formation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EncounterFormation {
    #[serde(rename = "formation", deserialize_with = "deserialize_strings")]
    pub enemy_ids: Vec<String>,
    /// Zero is retained because the pinned Python resolver explicitly treats it as unselectable.
    pub weight: u32,
    /// Required by the source loader even though the Python DTO has a constructor default.
    pub chase_range: u32,
}

/// Effective boss configuration after applying source-loader defaults.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncounterBoss {
    pub enemy_id: String,
    pub name: String,
    pub once: bool,
    pub completion: BossCompletion,
}

impl<'de> Deserialize<'de> for EncounterBoss {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let document = BossDocument::deserialize(deserializer)?;
        Ok(Self {
            name: document.name.unwrap_or_else(|| document.enemy_id.clone()),
            enemy_id: document.enemy_id,
            once: document.once,
            completion: document.on_complete.unwrap_or_default(),
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BossDocument {
    #[serde(rename = "id", deserialize_with = "deserialize_string")]
    enemy_id: String,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    name: Option<String>,
    #[serde(default = "default_true")]
    once: bool,
    /// The Python loader explicitly normalizes a missing or null mapping to an empty mapping.
    #[serde(default)]
    on_complete: Option<BossCompletion>,
}

/// Boss effects authored beneath `on_complete`.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BossCompletion {
    /// Empty means there is no completion flag, matching the Python loader default.
    #[serde(default, deserialize_with = "deserialize_string")]
    pub set_flag: String,
}

/// An enemy omitted from battle unless its required inventory item is present.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BarrierEnemy {
    #[serde(rename = "id", deserialize_with = "deserialize_string")]
    pub enemy_id: String,
    /// Missing becomes empty because that is the source loader's effective default.
    #[serde(default, deserialize_with = "deserialize_string")]
    pub requires_item: String,
    /// Required by the source loader despite the Python DTO's constructor default.
    #[serde(deserialize_with = "deserialize_string")]
    pub blocked_message: String,
}

fn deserialize_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_string(deserializer).map(Some)
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::{EncounterBoss, EncounterFormation, EncounterZone};
    use crate::scenario_yaml;
    use std::fs;

    #[test]
    fn loads_regular_zone_formations_barrier_and_spawn_metadata() {
        let zone: EncounterZone = scenario_yaml::from_str(include_str!(
            "../../../tests/fixtures/encounter-regular-zone.yaml"
        ))
        .expect("source-shaped regular encounter zone should deserialize");

        assert_eq!(zone.effective_id("ignored_stem"), "zone_mossy_track");
        assert_eq!(zone.name, "Mossy Track");
        assert_eq!(zone.density.get(), 0.35);
        assert_eq!(zone.background, "moss-track-bg-1280x468");
        assert_eq!(zone.spawn_frequency.unwrap().get(), 18.5);
        assert!(zone.boss.is_none());
        assert_eq!(zone.entries.len(), 3);
        assert_eq!(zone.total_weight(), 100);
        assert_eq!(zone.entries[0].enemy_ids, ["moss_hare"]);
        assert_eq!(zone.entries[1].enemy_ids, ["moss_hare", "reed_wisp"]);
        assert_eq!(zone.entries[2].weight, 0);
        assert_eq!(zone.entries[0].chase_range, 4);
        assert_eq!(zone.barrier_enemies.len(), 1);
        assert_eq!(zone.barrier_enemies[0].enemy_id, "reed_wisp");
        assert_eq!(zone.barrier_enemies[0].requires_item, "invented_reed_charm");
        assert_eq!(
            zone.barrier_enemies[0].blocked_message,
            "The reeds turn aside your strike."
        );
    }

    #[test]
    fn loads_boss_zone_and_completion_flag() {
        let zone: EncounterZone = scenario_yaml::from_str(include_str!(
            "../../../tests/fixtures/encounter-boss-zone.yaml"
        ))
        .expect("source-shaped boss encounter zone should deserialize");

        assert_eq!(zone.effective_id("ignored_stem"), "zone_clockwork_keep");
        let boss = zone.boss.expect("boss fixture should retain its boss");
        assert_eq!(boss.enemy_id, "invented_clockwork_tyrant");
        assert_eq!(boss.name, "Clockwork Tyrant");
        assert!(boss.once);
        assert_eq!(boss.completion.set_flag, "boss_clockwork_keep_defeated");
    }

    #[test]
    fn applies_only_source_loader_defaults_and_filename_identity() {
        let zone: EncounterZone =
            scenario_yaml::from_str("density: 0.0\nboss: null\nspawn_frequency: null\n")
                .expect("documented missing/null defaults should deserialize");

        assert_eq!(
            zone.effective_id("zone_filename_identity"),
            "zone_filename_identity"
        );
        assert_eq!(zone.name, "");
        assert!(zone.entries.is_empty());
        assert!(zone.boss.is_none());
        assert!(zone.barrier_enemies.is_empty());
        assert_eq!(zone.background, "");
        assert!(zone.spawn_frequency.is_none());

        let zone: EncounterZone = scenario_yaml::from_str(
            "density: 1.0\nboss:\n  id: invented_lich\n  on_complete: null\nbarrier_enemies:\n  - id: invented_ward\n    blocked_message: Stop.\n",
        )
        .expect("boss and barrier effective defaults should deserialize");
        assert_eq!(
            zone.boss,
            Some(EncounterBoss {
                enemy_id: "invented_lich".to_owned(),
                name: "invented_lich".to_owned(),
                once: true,
                completion: Default::default(),
            })
        );
        assert_eq!(zone.barrier_enemies[0].requires_item, "");
    }

    #[test]
    fn preserves_order_duplicates_zero_weight_and_once_false() {
        let zone: EncounterZone = scenario_yaml::from_str(
            "density: 1.0\nentries:\n  - formation: [wisp, wisp, hare]\n    weight: 0\n    chase_range: 0\nboss:\n  id: patient_guardian\n  once: false\n",
        )
        .expect("source loader's non-lossy weight and boss boolean shapes should load");

        assert_eq!(
            zone.entries,
            [EncounterFormation {
                enemy_ids: vec!["wisp".to_owned(), "wisp".to_owned(), "hare".to_owned()],
                weight: 0,
                chase_range: 0,
            }]
        );
        assert!(!zone.boss.unwrap().once);
    }

    #[test]
    fn rejects_unknown_coerced_incomplete_null_and_out_of_range_shapes() {
        let valid = include_str!("../../../tests/fixtures/encounter-regular-zone.yaml");
        for document in [
            valid.replacen("name: Mossy Track", "name: true", 1),
            valid.replacen("density: 0.35", "density: 1", 1),
            valid.replacen("density: 0.35", "density: 1.1", 1),
            valid.replacen("spawn_frequency: 18.5", "spawn_frequency: 0.0", 1),
            valid.replacen("formation: [moss_hare]", "formation: [42]", 1),
            valid.replacen("weight: 60", "weight: -1", 1),
            valid.replacen("chase_range: 4", "chase_range: -1", 1),
            valid.replacen("chase_range: 4\n", "", 1),
            valid.replacen(
                "blocked_message: The reeds turn aside your strike.",
                "blocked_message: null",
                1,
            ),
            valid.replacen(
                "id: zone_mossy_track",
                "id: zone_mossy_track\nunknown: value",
                1,
            ),
            valid.replacen("weight: 60", "weight: 60\n    unknown: value", 1),
        ] {
            assert!(
                scenario_yaml::from_str::<EncounterZone>(&document).is_err(),
                "unexpectedly accepted:\n{document}"
            );
        }

        for document in [
            "density: 0.5\nentries: null\n",
            "density: 0.5\nbarrier_enemies: null\n",
            "density: 0.5\nid: null\n",
            "density: 0.5\nname: null\n",
            "density: 0.5\nbackground: null\n",
            "density: 0.5\nboss:\n  id: true\n",
            "density: 0.5\nboss:\n  id: lich\n  name: null\n",
            "density: 0.5\nboss:\n  id: lich\n  once: null\n",
            "density: 0.5\nboss:\n  id: lich\n  on_complete: {set_flag: true}\n",
            "density: 0.5\nboss:\n  id: lich\n  on_complete: {set_flag: done, transition: town}\n",
            "density: 0.5\nbarrier_enemies:\n  - id: ward\n    requires_item: key\n",
        ] {
            assert!(
                scenario_yaml::from_str::<EncounterZone>(document).is_err(),
                "unexpectedly accepted:\n{document}"
            );
        }
    }

    #[test]
    #[ignore = "requires the separately licensed pinned Python source checkout"]
    fn audits_the_complete_pinned_encounter_corpus_when_requested() {
        let root = std::env::var_os("RPG_S1_PINNED_ENCOUNTS_DIR")
            .map(std::path::PathBuf::from)
            .expect("RPG_S1_PINNED_ENCOUNTS_DIR must name the pinned data/encount directory");

        let mut files = fs::read_dir(&root)
            .expect("pinned encount directory should be readable")
            .map(|entry| {
                entry
                    .expect("encount directory entry should be readable")
                    .path()
            })
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "yaml")
            })
            .collect::<Vec<_>>();
        files.sort();

        let mut zones = Vec::new();
        for path in &files {
            let document = fs::read_to_string(path)
                .unwrap_or_else(|error| panic!("{} should be readable: {error}", path.display()));
            let zone: EncounterZone = scenario_yaml::from_str(&document)
                .unwrap_or_else(|error| panic!("{} should load: {error}", path.display()));
            let stem = path.file_stem().and_then(|stem| stem.to_str()).unwrap();
            assert_eq!(zone.effective_id(stem), stem);
            assert!(!zone.name.is_empty());
            assert!(!zone.background.is_empty());
            assert!(zone.total_weight() > 0);
            zones.push(zone);
        }

        assert_eq!(files.len(), 16);
        assert_eq!(
            zones.iter().map(|zone| zone.entries.len()).sum::<usize>(),
            147
        );
        assert_eq!(zones.iter().filter(|zone| zone.boss.is_some()).count(), 10);
        assert_eq!(
            zones
                .iter()
                .flat_map(|zone| zone.barrier_enemies.iter())
                .count(),
            6
        );
        assert_eq!(
            zones
                .iter()
                .filter(|zone| zone.spawn_frequency.is_some())
                .count(),
            2
        );
        assert_eq!(
            zones
                .iter()
                .flat_map(|zone| zone.entries.iter())
                .filter(|entry| entry.chase_range > 0)
                .count(),
            15
        );
        assert_eq!(
            zones
                .iter()
                .flat_map(|zone| zone.entries.iter())
                .filter(|entry| entry.enemy_ids.len() == 1)
                .count(),
            65
        );
        assert_eq!(
            zones
                .iter()
                .flat_map(|zone| zone.entries.iter())
                .filter(|entry| entry.enemy_ids.len() == 2)
                .count(),
            80
        );
        assert_eq!(
            zones
                .iter()
                .flat_map(|zone| zone.entries.iter())
                .filter(|entry| entry.enemy_ids.len() == 3)
                .count(),
            2
        );
        assert!(
            zones
                .iter()
                .filter_map(|zone| zone.boss.as_ref())
                .all(|boss| {
                    boss.once && !boss.name.is_empty() && !boss.completion.set_flag.is_empty()
                })
        );
        assert!(
            zones
                .iter()
                .flat_map(|zone| zone.barrier_enemies.iter())
                .all(|barrier| !barrier.requires_item.is_empty()
                    && !barrier.blocked_message.is_empty())
        );
    }
}
