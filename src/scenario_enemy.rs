//! Source-authored enemy and boss move-set schemas.
//!
//! The pinned `0897035` scenario has eight YAML multi-document rank streams containing 106
//! enemies and nine mapping-root boss move sets. Rank-stream enemies always carry explicit
//! identity; an enemy's battle sprite identity is that same `id`. Boss move sets are id-less and
//! use their referenced filename stem as identity.
//!
//! Regular enemies and one boss carry inline AI plus targeting. Nine other bosses carry an
//! `ai_ref` relative to `data/enemies/`. Those alternatives are closed here so incomplete or
//! mixed behavior definitions cannot silently become an empty basic-attack policy. Battle AI,
//! damage, loot resolution, and cross-reference checks belong to later milestones.

use crate::scenario_class::UnitInterval;
use crate::scenario_path::ScenarioRelativePath;
use crate::scenario_yaml::{self, deserialize_string};
use serde::{Deserialize, Deserializer};
use std::num::NonZeroU32;

/// One source rank file, represented by its ordered YAML document stream.
#[derive(Clone, Debug, PartialEq)]
pub struct EnemyCatalogFile(pub Vec<EnemyDefinition>);

impl EnemyCatalogFile {
    /// Parses the multi-document source form used by `enemies_rank_*.yaml`.
    pub fn from_yaml_stream(stream: &str) -> Result<Self, scenario_yaml::ScenarioYamlError> {
        scenario_yaml::from_documents(stream).map(Self)
    }

    pub fn entries(&self) -> &[EnemyDefinition] {
        &self.0
    }
}

/// One enemy stat block and its authored behavior source.
#[derive(Clone, Debug, PartialEq)]
pub struct EnemyDefinition {
    pub id: String,
    pub name: String,
    pub enemy_type: EnemyType,
    pub rank: EnemyRank,
    pub boss: bool,
    pub immunities: Vec<EnemyImmunity>,
    pub barrier: Option<EnemyBarrier>,
    pub hp: NonZeroU32,
    pub attack: NonZeroU32,
    pub defense: NonZeroU32,
    pub magic_resistance: NonZeroU32,
    pub dexterity: NonZeroU32,
    pub experience: NonZeroU32,
    pub size: EnemySize,
    pub sprite_scale_percent: NonZeroU32,
    pub drops: EnemyDrops,
    pub behavior: EnemyBehavior,
}

impl EnemyDefinition {
    /// Enemy battler assets are keyed by the source enemy id; there is no separate sprite field.
    pub fn sprite_id(&self) -> &str {
        &self.id
    }
}

impl<'de> Deserialize<'de> for EnemyDefinition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        EnemyDocument::deserialize(deserializer)?
            .into_definition()
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EnemyDocument {
    #[serde(deserialize_with = "deserialize_string")]
    id: String,
    #[serde(deserialize_with = "deserialize_string")]
    name: String,
    #[serde(rename = "type")]
    enemy_type: EnemyType,
    rank: EnemyRank,
    #[serde(default)]
    boss: OptionalField<bool>,
    #[serde(default)]
    immune_to: Vec<EnemyImmunity>,
    #[serde(default)]
    barrier: OptionalField<bool>,
    #[serde(default)]
    requires_item: OptionalField<StrictString>,
    hp: NonZeroU32,
    #[serde(rename = "atk")]
    attack: NonZeroU32,
    #[serde(rename = "def")]
    defense: NonZeroU32,
    #[serde(rename = "mres")]
    magic_resistance: NonZeroU32,
    #[serde(rename = "dex")]
    dexterity: NonZeroU32,
    #[serde(rename = "exp")]
    experience: NonZeroU32,
    size: EnemySize,
    #[serde(default = "default_sprite_scale")]
    sprite_scale: NonZeroU32,
    #[serde(default)]
    drops: EnemyDrops,
    #[serde(default)]
    ai: OptionalField<EnemyAi>,
    #[serde(default)]
    targeting: OptionalField<EnemyTargeting>,
    #[serde(default)]
    ai_ref: OptionalField<ScenarioRelativePath>,
}

impl EnemyDocument {
    fn into_definition(self) -> Result<EnemyDefinition, String> {
        let boss = match self.boss {
            OptionalField::Missing => false,
            OptionalField::Present(true) => true,
            OptionalField::Present(false) => {
                return Err("field `boss` is omitted for ordinary enemies; explicit false is not an authored shape".to_owned());
            }
        };

        let barrier = match (self.barrier, self.requires_item) {
            (OptionalField::Missing, OptionalField::Missing) => None,
            (OptionalField::Present(true), OptionalField::Present(item)) => Some(EnemyBarrier {
                requires_item: item.0,
            }),
            (OptionalField::Present(false), _) => {
                return Err("field `barrier` is omitted for ordinary enemies; explicit false is not an authored shape".to_owned());
            }
            (OptionalField::Present(true), OptionalField::Missing) => {
                return Err("barrier enemy must define `requires_item`".to_owned());
            }
            (OptionalField::Missing, OptionalField::Present(_)) => {
                return Err("field `requires_item` requires `barrier: true`".to_owned());
            }
        };

        let behavior = match (self.ai, self.targeting, self.ai_ref) {
            (
                OptionalField::Present(ai),
                OptionalField::Present(targeting),
                OptionalField::Missing,
            ) => EnemyBehavior::Inline { ai, targeting },
            (OptionalField::Missing, OptionalField::Missing, OptionalField::Present(ai_ref)) => {
                EnemyBehavior::Referenced { ai_ref }
            }
            _ => {
                return Err(
                    "enemy must define exactly one behavior source: both `ai` and `targeting`, or `ai_ref`"
                        .to_owned(),
                );
            }
        };

        if matches!(&behavior, EnemyBehavior::Referenced { .. }) && !boss {
            return Err("only an enemy with `boss: true` may define `ai_ref`".to_owned());
        }

        Ok(EnemyDefinition {
            id: self.id,
            name: self.name,
            enemy_type: self.enemy_type,
            rank: self.rank,
            boss,
            immunities: self.immune_to,
            barrier,
            hp: self.hp,
            attack: self.attack,
            defense: self.defense,
            magic_resistance: self.magic_resistance,
            dexterity: self.dexterity,
            experience: self.experience,
            size: self.size,
            sprite_scale_percent: self.sprite_scale,
            drops: self.drops,
            behavior,
        })
    }
}

/// The source taxonomy used by enemy stat blocks.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EnemyType {
    Beast,
    Construct,
    Demon,
    Humanoid,
    Undead,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum EnemyRank {
    #[serde(rename = "SS")]
    DoubleS,
    #[serde(rename = "S")]
    S,
    #[serde(rename = "A")]
    A,
    #[serde(rename = "B")]
    B,
    #[serde(rename = "C")]
    C,
    #[serde(rename = "D")]
    D,
    #[serde(rename = "E")]
    E,
    #[serde(rename = "F")]
    F,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EnemySize {
    Small,
    Medium,
    Large,
    /// Accepted by the pinned Python validator and battle layout table, though current boss
    /// records author `large`.
    Boss,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EnemyImmunity {
    InstantKill,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyBarrier {
    pub requires_item: String,
}

/// Inline behavior or a source path relative to `data/enemies/`.
#[derive(Clone, Debug, PartialEq)]
pub enum EnemyBehavior {
    Inline {
        ai: EnemyAi,
        targeting: EnemyTargeting,
    },
    Referenced {
        ai_ref: ScenarioRelativePath,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EnemyAi {
    pub pattern: EnemyAiPattern,
    pub moves: Vec<EnemyMove>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EnemyAiPattern {
    Random,
    Conditional,
}

/// The two authored action signatures. Ability identity is required only for ability moves.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum EnemyMove {
    Attack {
        weight: NonZeroU32,
        #[serde(default, deserialize_with = "deserialize_present_option")]
        condition: Option<EnemyMoveCondition>,
    },
    Ability {
        #[serde(deserialize_with = "deserialize_string")]
        id: String,
        weight: NonZeroU32,
        #[serde(default, deserialize_with = "deserialize_present_option")]
        condition: Option<EnemyMoveCondition>,
        #[serde(default)]
        once: bool,
    },
}

/// Enemy move conditions are HP/turn rules, not story [`crate::scenario_condition::FlagConditions`].
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum EnemyMoveCondition {
    HpBelow(HpBelowCondition),
    TurnModulo(TurnModuloCondition),
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HpBelowCondition {
    pub hp_pct_below: UnitInterval,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TurnModuloCondition {
    pub turn_mod: TurnModulo,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TurnModulo {
    pub every: NonZeroU32,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EnemyTargeting {
    pub default: EnemyDefaultTarget,
    #[serde(default)]
    pub overrides: Vec<EnemyTargetOverride>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EnemyDefaultTarget {
    RandomAlive,
    LowestHp,
    HighestHp,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(untagged)]
pub enum EnemyTargetOverride {
    Standard(StandardTargetOverride),
    AccessoryBlocked(AccessoryBlockedTargetOverride),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StandardTargetOverride {
    #[serde(deserialize_with = "deserialize_string")]
    pub ability: String,
    pub target: EnemyOverrideTarget,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AccessoryBlockedTargetOverride {
    #[serde(deserialize_with = "deserialize_string")]
    pub ability: String,
    pub target: EnemyOverrideTarget,
    #[serde(deserialize_with = "deserialize_string")]
    pub blocked_by_accessory: String,
    pub on_blocked: BlockedAbilityOutcome,
    pub on_hit: AbilityHitOutcome,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EnemyOverrideTarget {
    RandomAlive,
    LowestHp,
    HighestHp,
    HighestDex,
    AllParty,
    #[serde(rename = "self")]
    SelfTarget,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BlockedAbilityOutcome {
    NoEffect,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AbilityHitOutcome {
    #[serde(rename = "hp_to_1")]
    HpTo1,
}

/// The source default for a wholly absent `drops` mapping is no drops. If the mapping is
/// authored, both current list fields remain required rather than receiving blanket defaults.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EnemyDrops {
    pub mc: Vec<MagicCoreDrop>,
    pub loot: Vec<EnemyLootPool>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MagicCoreDrop {
    pub size: MagicCoreSize,
    pub qty: NonZeroU32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum MagicCoreSize {
    #[serde(rename = "XS")]
    ExtraSmall,
    #[serde(rename = "S")]
    Small,
    #[serde(rename = "M")]
    Medium,
    #[serde(rename = "L")]
    Large,
    #[serde(rename = "XL")]
    ExtraLarge,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EnemyLootPool {
    pub pool: Vec<EnemyLootEntry>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EnemyLootEntry {
    #[serde(deserialize_with = "deserialize_string")]
    pub item: String,
    pub weight: NonZeroU32,
}

/// One id-less mapping-root boss move-set document.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BossMoveSet {
    pub ai: EnemyAi,
    pub targeting: EnemyTargeting,
}

impl BossMoveSet {
    /// Boss move sets use their referenced filename stem as effective identity.
    pub fn effective_id<'a>(&self, filename_stem: &'a str) -> &'a str {
        filename_stem
    }
}

#[derive(Default)]
enum OptionalField<T> {
    #[default]
    Missing,
    Present(T),
}

impl<'de, T> Deserialize<'de> for OptionalField<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        T::deserialize(deserializer).map(Self::Present)
    }
}

struct StrictString(String);

impl<'de> Deserialize<'de> for StrictString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_string(deserializer).map(Self)
    }
}

fn deserialize_present_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

fn default_sprite_scale() -> NonZeroU32 {
    NonZeroU32::new(100).expect("100 is nonzero")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_every_invented_enemy_rule_shape_from_a_multi_document_fixture() {
        let catalog = EnemyCatalogFile::from_yaml_stream(include_str!(
            "../tests/fixtures/enemy-rule-shapes.yaml"
        ))
        .expect("source-shaped enemy stream should deserialize");

        assert_eq!(catalog.entries().len(), 8);
        assert_eq!(catalog.entries()[0].sprite_id(), "moss_hare");
        assert_eq!(catalog.entries()[0].sprite_scale_percent.get(), 100);
        assert!(!catalog.entries()[0].boss);
        assert_eq!(catalog.entries()[0].drops, EnemyDrops::default());

        let barrier = catalog
            .entries()
            .iter()
            .find(|enemy| enemy.id == "veil_wraith")
            .and_then(|enemy| enemy.barrier.as_ref())
            .expect("fixture should retain a paired barrier requirement");
        assert_eq!(barrier.requires_item, "invented_veil_key");

        let referenced = catalog
            .entries()
            .iter()
            .find(|enemy| enemy.id == "clockwork_tyrant")
            .expect("fixture should contain referenced boss behavior");
        assert!(referenced.boss);
        assert_eq!(referenced.immunities, [EnemyImmunity::InstantKill]);
        let EnemyBehavior::Referenced { ai_ref } = &referenced.behavior else {
            panic!("clockwork_tyrant should retain its boss move-set reference")
        };
        assert_eq!(ai_ref.as_str(), "boss_move_sets/clockwork_tyrant.yaml");

        let sizes = catalog
            .entries()
            .iter()
            .flat_map(|enemy| enemy.drops.mc.iter().map(|drop| drop.size))
            .collect::<Vec<_>>();
        assert_eq!(
            sizes,
            [
                MagicCoreSize::ExtraSmall,
                MagicCoreSize::Small,
                MagicCoreSize::Medium,
                MagicCoreSize::Large,
                MagicCoreSize::ExtraLarge,
            ]
        );
    }

    #[test]
    fn loads_every_boss_move_condition_and_target_override_shape() {
        let move_set: BossMoveSet = scenario_yaml::from_str(include_str!(
            "../tests/fixtures/enemy-boss-move-set-shapes.yaml"
        ))
        .expect("source-shaped boss move set should deserialize");

        assert_eq!(
            move_set.effective_id("invented_judicator"),
            "invented_judicator"
        );
        assert_eq!(move_set.ai.pattern, EnemyAiPattern::Conditional);
        assert_eq!(move_set.ai.moves.len(), 4);
        assert!(matches!(
            move_set.ai.moves[0],
            EnemyMove::Attack {
                condition: Some(EnemyMoveCondition::HpBelow(_)),
                ..
            }
        ));
        assert!(matches!(
            move_set.ai.moves[2],
            EnemyMove::Ability {
                condition: Some(EnemyMoveCondition::TurnModulo(_)),
                once: true,
                ..
            }
        ));
        assert_eq!(move_set.targeting.overrides.len(), 6);
        assert!(matches!(
            move_set.targeting.overrides[5],
            EnemyTargetOverride::AccessoryBlocked(_)
        ));
    }

    #[test]
    fn rejects_unknown_coerced_incomplete_mixed_and_out_of_range_shapes() {
        let valid = include_str!("../tests/fixtures/enemy-rule-shapes.yaml");
        for stream in [
            valid.replacen("type: beast", "type: elemental", 1),
            valid.replacen("rank: SS", "rank: Z", 1),
            valid.replacen("hp: 12", "hp: 0", 1),
            valid.replacen("atk: 3", "atk: 3.5", 1),
            valid.replacen("name: Moss Hare", "name: true", 1),
            valid.replacen("weight: 60", "weight: 0", 1),
            valid.replacen("barrier: true", "barrier: true\nunknown: value", 1),
            valid.replacen("requires_item: invented_veil_key\n", "", 1),
            valid.replacen("boss: true", "boss: false", 1),
            valid.replacen("boss: true\nimmune_to:", "immune_to:", 1),
            valid.replacen(
                "ai_ref: boss_move_sets/clockwork_tyrant.yaml",
                "ai_ref: ../outside.yaml",
                1,
            ),
            valid.replacen(
                "ai_ref: boss_move_sets/clockwork_tyrant.yaml",
                "ai_ref: boss_move_sets/clockwork_tyrant.yaml\nfamily: unknown",
                1,
            ),
        ] {
            assert!(
                EnemyCatalogFile::from_yaml_stream(&stream).is_err(),
                "unexpectedly accepted:\n{stream}"
            );
        }

        let mixed = valid.replacen(
            "ai_ref: boss_move_sets/clockwork_tyrant.yaml",
            "ai_ref: boss_move_sets/clockwork_tyrant.yaml\nai: { pattern: random, moves: [] }\ntargeting: { default: random_alive }",
            1,
        );
        assert!(EnemyCatalogFile::from_yaml_stream(&mixed).is_err());

        let move_set = include_str!("../tests/fixtures/enemy-boss-move-set-shapes.yaml");
        for document in [
            move_set.replace("hp_pct_below: 1.0", "hp_pct_below: 1"),
            move_set.replace("hp_pct_below: 1.0", "hp_pct_below: 1.1"),
            move_set.replace("every: 4", "every: 0"),
            move_set.replace("action: ability", "action: spell"),
            move_set.replace("target: highest_dex", "target: fastest"),
            move_set.replace("on_hit: hp_to_1", "on_hit: instant_ko"),
            move_set.replace("condition:\n        hp_pct_below: 1.0", "condition: null"),
        ] {
            assert!(scenario_yaml::from_str::<BossMoveSet>(&document).is_err());
        }
    }

    #[test]
    #[ignore = "requires the separately licensed pinned Python source checkout"]
    fn audits_the_complete_pinned_enemy_corpus_when_requested() {
        let root = std::env::var_os("RPG_S1_PINNED_ENEMIES_DIR")
            .map(std::path::PathBuf::from)
            .expect("RPG_S1_PINNED_ENEMIES_DIR must name the pinned data/enemies directory");

        let mut rank_files = std::fs::read_dir(&root)
            .expect("pinned enemies directory should be readable")
            .map(|entry| {
                entry
                    .expect("enemy directory entry should be readable")
                    .path()
            })
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| {
                        name.starts_with("enemies_rank_") && name.ends_with(".yaml")
                    })
            })
            .collect::<Vec<_>>();
        rank_files.sort();

        let mut enemies = Vec::new();
        for path in &rank_files {
            let stream = std::fs::read_to_string(path)
                .unwrap_or_else(|error| panic!("{} should be readable: {error}", path.display()));
            let catalog = EnemyCatalogFile::from_yaml_stream(&stream)
                .unwrap_or_else(|error| panic!("{} should load: {error}", path.display()));
            enemies.extend(catalog.0);
        }

        let move_set_root = root.join("boss_move_sets");
        let mut move_set_files = std::fs::read_dir(&move_set_root)
            .expect("pinned boss move-set directory should be readable")
            .map(|entry| {
                entry
                    .expect("move-set directory entry should be readable")
                    .path()
            })
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "yaml")
            })
            .collect::<Vec<_>>();
        move_set_files.sort();
        for path in &move_set_files {
            let document = std::fs::read_to_string(path)
                .unwrap_or_else(|error| panic!("{} should be readable: {error}", path.display()));
            let move_set: BossMoveSet = scenario_yaml::from_str(&document)
                .unwrap_or_else(|error| panic!("{} should load: {error}", path.display()));
            assert_eq!(
                move_set.effective_id(path.file_stem().unwrap().to_str().unwrap()),
                path.file_stem().unwrap().to_str().unwrap()
            );
        }

        assert_eq!(rank_files.len(), 8);
        assert_eq!(enemies.len(), 106);
        assert_eq!(move_set_files.len(), 9);
        assert_eq!(enemies.iter().filter(|enemy| enemy.boss).count(), 10);
        assert_eq!(
            enemies
                .iter()
                .filter(|enemy| enemy.barrier.is_some())
                .count(),
            2
        );
        assert_eq!(
            enemies
                .iter()
                .filter(|enemy| matches!(enemy.behavior, EnemyBehavior::Referenced { .. }))
                .count(),
            9
        );
        assert_eq!(
            enemies
                .iter()
                .flat_map(|enemy| enemy.drops.mc.iter())
                .count(),
            191
        );
        assert_eq!(
            enemies
                .iter()
                .flat_map(|enemy| enemy.drops.loot.iter())
                .flat_map(|pool| pool.pool.iter())
                .count(),
            223
        );
    }
}
