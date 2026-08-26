//! Source-authored scenario-wide balance constants.
//!
//! The pinned `0897035` scenario has one mapping-root `data/balance.yaml` document with five
//! required groups and eleven required scalar fields. Python's `BalanceData.load` rejects a
//! missing or null value for every one of them. Its older module constants are only fallbacks
//! when no `BalanceData` is injected, so [`BalanceData::default`] records that complete fallback
//! set without making any source YAML field optional. Runtime formulas and tuning are deferred.

use bevy::{asset::Asset, reflect::TypePath};

use crate::scenario_class::{PositiveFinite, UnitInterval};
use serde::{Deserialize, Deserializer};
use std::fmt;
use std::num::NonZeroU32;

/// Complete scenario-wide balance data selected by `manifest.refs.balance`.
#[derive(Asset, Clone, Debug, Deserialize, PartialEq, TypePath)]
#[serde(deny_unknown_fields)]
pub struct BalanceData {
    pub progression: ProgressionBalance,
    pub economy: EconomyBalance,
    pub battle: BattleBalance,
    pub spawner: SpawnerBalance,
    pub movement: MovementBalance,
}

impl Default for BalanceData {
    fn default() -> Self {
        Self {
            progression: ProgressionBalance {
                level_cap: PositiveInteger::new(100).expect("constant is nonzero"),
                exp_cap: PositiveInteger::new(1_000_000).expect("constant is nonzero"),
            },
            economy: EconomyBalance {
                gp_cap: PositiveInteger::new(8_000_000).expect("constant is nonzero"),
                item_qty_cap: PositiveInteger::new(100).expect("constant is nonzero"),
                max_tags_per_item: 5,
            },
            battle: BattleBalance {
                flee_base_chance: UnitInterval::new(0.30).expect("constant is a unit interval"),
                flee_rogue_dex_bonus: UnitInterval::new(0.02).expect("constant is a unit interval"),
            },
            spawner: SpawnerBalance {
                rogue_chase_reduction: 2,
                stealth_cloak_reduction: 3,
                lure_charm_interval_mult: PositiveFinite::new(0.5)
                    .expect("constant is positive and finite"),
            },
            movement: MovementBalance {
                player_speed: PositiveInteger::new(5).expect("constant is nonzero"),
            },
        }
    }
}

/// Progression caps consumed by level and experience systems.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProgressionBalance {
    pub level_cap: PositiveInteger,
    pub exp_cap: PositiveInteger,
}

/// Economy inventory limits.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EconomyBalance {
    pub gp_cap: PositiveInteger,
    pub item_qty_cap: PositiveInteger,
    /// Zero is meaningful: it disables additional item tags while retaining the item system.
    pub max_tags_per_item: u32,
}

/// Battle escape probabilities.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BattleBalance {
    pub flee_base_chance: UnitInterval,
    pub flee_rogue_dex_bonus: UnitInterval,
}

/// Field enemy-spawner modifiers.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SpawnerBalance {
    /// Tiles removed from a chase range; zero preserves the baseline range.
    pub rogue_chase_reduction: u32,
    /// Tiles removed by the accessory; zero makes the accessory neutral.
    pub stealth_cloak_reduction: u32,
    /// Positive multiplier applied to spawn intervals.
    pub lure_charm_interval_mult: PositiveFinite,
}

/// Player field-movement configuration.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MovementBalance {
    pub player_speed: PositiveInteger,
}

/// A positive integer scalar, including the underscore-separated YAML integer spelling used by
/// the source balance file. `serde_yaml_ng` presents that legal YAML spelling as text, so this
/// visitor accepts only an underscore-containing all-digit spelling in `visit_str`. (Its Serde
/// interface cannot distinguish that parser representation from an equivalently quoted scalar.)
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PositiveInteger(NonZeroU32);

impl PositiveInteger {
    pub fn new(value: u32) -> Option<Self> {
        NonZeroU32::new(value).map(Self)
    }

    pub fn get(self) -> u32 {
        self.0.get()
    }
}

impl<'de> Deserialize<'de> for PositiveInteger {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(PositiveIntegerVisitor)
    }
}

struct PositiveIntegerVisitor;

impl serde::de::Visitor<'_> for PositiveIntegerVisitor {
    type Value = PositiveInteger;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a positive YAML integer")
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        u32::try_from(value)
            .ok()
            .and_then(PositiveInteger::new)
            .ok_or_else(|| E::custom("expected a positive u32"))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        u64::try_from(value)
            .ok()
            .and_then(|value| u32::try_from(value).ok())
            .and_then(PositiveInteger::new)
            .ok_or_else(|| E::custom("expected a positive u32"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let source_integer = value.contains('_')
            && !value.starts_with('_')
            && !value.ends_with('_')
            && value
                .split('_')
                .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()));
        if !source_integer {
            return Err(E::custom("expected a positive YAML integer"));
        }
        value
            .replace('_', "")
            .parse::<u32>()
            .ok()
            .and_then(PositiveInteger::new)
            .ok_or_else(|| E::custom("expected a positive u32"))
    }
}

#[cfg(test)]
mod tests {
    use super::BalanceData;
    use crate::scenario_yaml;
    use std::fs;

    #[test]
    fn loads_complete_source_shaped_balance_data() {
        let balance: BalanceData = scenario_yaml::from_str(include_str!(
            "../../../tests/fixtures/balance-complete.yaml"
        ))
        .expect("complete balance fixture should deserialize");

        assert_eq!(balance.progression.level_cap.get(), 77);
        assert_eq!(balance.progression.exp_cap.get(), 900_000);
        assert_eq!(balance.economy.gp_cap.get(), 600_000);
        assert_eq!(balance.economy.item_qty_cap.get(), 88);
        assert_eq!(balance.economy.max_tags_per_item, 0);
        assert_eq!(balance.battle.flee_base_chance.get(), 0.25);
        assert_eq!(balance.battle.flee_rogue_dex_bonus.get(), 0.01);
        assert_eq!(balance.spawner.rogue_chase_reduction, 0);
        assert_eq!(balance.spawner.stealth_cloak_reduction, 4);
        assert_eq!(balance.spawner.lure_charm_interval_mult.get(), 1.25);
        assert_eq!(balance.movement.player_speed.get(), 6);
    }

    #[test]
    fn exposes_the_python_no_injected_balance_fallbacks_explicitly() {
        let balance = BalanceData::default();
        assert_eq!(balance.progression.level_cap.get(), 100);
        assert_eq!(balance.progression.exp_cap.get(), 1_000_000);
        assert_eq!(balance.economy.gp_cap.get(), 8_000_000);
        assert_eq!(balance.economy.item_qty_cap.get(), 100);
        assert_eq!(balance.economy.max_tags_per_item, 5);
        assert_eq!(balance.battle.flee_base_chance.get(), 0.30);
        assert_eq!(balance.battle.flee_rogue_dex_bonus.get(), 0.02);
        assert_eq!(balance.spawner.rogue_chase_reduction, 2);
        assert_eq!(balance.spawner.stealth_cloak_reduction, 3);
        assert_eq!(balance.spawner.lure_charm_interval_mult.get(), 0.5);
        assert_eq!(balance.movement.player_speed.get(), 5);
    }

    #[test]
    fn rejects_missing_null_coerced_invalid_and_unknown_shapes() {
        let valid = include_str!("../../../tests/fixtures/balance-complete.yaml");
        for document in [
            valid.replacen("level_cap: 77", "level_cap: 0", 1),
            valid.replacen("exp_cap: 900_000", "exp_cap: -1", 1),
            valid.replacen("gp_cap: 600_000", "gp_cap: 1.5", 1),
            valid.replacen("item_qty_cap: 88", "item_qty_cap: true", 1),
            valid.replacen("flee_base_chance: 0.25", "flee_base_chance: 1", 1),
            valid.replacen("flee_rogue_dex_bonus: 0.01", "flee_rogue_dex_bonus: 1.1", 1),
            valid.replacen("rogue_chase_reduction: 0", "rogue_chase_reduction: -1", 1),
            valid.replacen(
                "lure_charm_interval_mult: 1.25",
                "lure_charm_interval_mult: 0.0",
                1,
            ),
            valid.replacen("player_speed: 6", "player_speed: 0", 1),
            valid.replacen("player_speed: 6", "player_speed: 6\n  unknown: nope", 1),
            valid.replacen("movement:\n", "mystery: 1\nmovement:\n", 1),
        ] {
            assert!(
                scenario_yaml::from_str::<BalanceData>(&document).is_err(),
                "accepted:\n{document}"
            );
        }

        for document in [
            valid.replacen("max_tags_per_item: 0\n", "", 1),
            valid.replacen("battle:\n", "battle: null\n", 1),
            valid.replacen(
                "stealth_cloak_reduction: 4",
                "stealth_cloak_reduction: null",
                1,
            ),
            valid.replacen("progression:\n", "progression: []\n", 1),
        ] {
            assert!(
                scenario_yaml::from_str::<BalanceData>(&document).is_err(),
                "accepted:\n{document}"
            );
        }
    }

    #[test]
    #[ignore = "requires the separately licensed pinned Python source checkout"]
    fn audits_the_complete_pinned_balance_file_when_requested() {
        let path = std::env::var_os("RPG_S1_PINNED_BALANCE_FILE")
            .map(std::path::PathBuf::from)
            .expect("RPG_S1_PINNED_BALANCE_FILE must name the pinned data/balance.yaml file");
        let document = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{} should be readable: {error}", path.display()));
        let balance: BalanceData = scenario_yaml::from_str(&document)
            .unwrap_or_else(|error| panic!("{} should load: {error}", path.display()));

        assert_eq!(balance.progression.level_cap.get(), 100);
        assert_eq!(balance.progression.exp_cap.get(), 1_000_000);
        assert_eq!(balance.economy.gp_cap.get(), 8_000_000);
        assert_eq!(balance.economy.item_qty_cap.get(), 100);
        assert_eq!(balance.economy.max_tags_per_item, 5);
        assert_eq!(balance.battle.flee_base_chance.get(), 0.30);
        assert_eq!(balance.battle.flee_rogue_dex_bonus.get(), 0.02);
        assert_eq!(balance.spawner.rogue_chase_reduction, 2);
        assert_eq!(balance.spawner.stealth_cloak_reduction, 3);
        assert_eq!(balance.spawner.lure_charm_interval_mult.get(), 0.5);
        assert_eq!(balance.movement.player_speed.get(), 5);
    }
}
