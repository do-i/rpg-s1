//! Source-authored scenario-wide balance constants.
//!
//! `data/balance.yaml` is one mapping-root document with five required groups and eleven
//! required scalar fields; a missing or null value for any of them is rejected. The
//! `mp_regen`, `world_enemy`, `party_ai` and `status_damage` groups are optional and default to
//! the built-in values; a present group still rejects null and unknown keys.
//! [`BalanceData::default`] records the complete built-in fallback set for callers that have
//! no scenario balance loaded, without making any YAML field optional. Runtime formulas and
//! tuning live elsewhere.

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
    /// Optional: scenarios that predate field MP regeneration get the built-in rates.
    #[serde(default)]
    pub mp_regen: MpRegenBalance,
    /// Optional: scenarios that predate these keys get the built-in world enemy movement.
    #[serde(default)]
    pub world_enemy: WorldEnemyBalance,
    /// Optional: scenarios that predate these keys get the built-in auto-battle heal jitter.
    #[serde(default)]
    pub party_ai: PartyAiBalance,
    /// Optional: scenarios that predate these keys get the built-in burn damage scale.
    #[serde(default)]
    pub status_damage: StatusDamageBalance,
}

impl Default for BalanceData {
    fn default() -> Self {
        Self {
            progression: ProgressionBalance {
                level_cap: PositiveInteger::new(100).expect("constant is nonzero"),
                exp_cap: PositiveInteger::new(1_000_000).expect("constant is nonzero"),
                hp_growth_con_divisor: default_hp_growth_con_divisor(),
            },
            economy: EconomyBalance {
                gp_cap: PositiveInteger::new(8_000_000).expect("constant is nonzero"),
                item_qty_cap: PositiveInteger::new(100).expect("constant is nonzero"),
                max_tags_per_item: 5,
            },
            battle: BattleBalance {
                flee_base_chance: UnitInterval::new(0.30).expect("constant is a unit interval"),
                flee_rogue_dex_bonus: UnitInterval::new(0.02).expect("constant is a unit interval"),
                party_defense_con_divisor: default_party_defense_con_divisor(),
                gp_per_exp_percent: default_gp_per_exp_percent(),
                mc_break_percent: default_mc_break_percent(),
                mc_break_rough_percent: default_mc_break_rough_percent(),
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
            mp_regen: MpRegenBalance::default(),
            world_enemy: WorldEnemyBalance::default(),
            party_ai: PartyAiBalance::default(),
            status_damage: StatusDamageBalance::default(),
        }
    }
}

/// Progression caps consumed by level and experience systems.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProgressionBalance {
    pub level_cap: PositiveInteger,
    pub exp_cap: PositiveInteger,
    /// Divides post-growth CON before the flat bonus in the per-level HP gain. Adding raw
    /// CON would inflate level-22 HP to ~1,280 against a 78-ATK final boss; dividing
    /// lands it near 420, the scale the shipped potion ladder was authored for. Defaults so a
    /// scenario predating the field still loads.
    #[serde(default = "default_hp_growth_con_divisor")]
    pub hp_growth_con_divisor: PositiveInteger,
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
    /// Divides a party member's innate CON when deriving battle defense, so equipment CON is the
    /// dominant term. Raw CON would exceed every enemy ATK in the game
    /// and pins all incoming physical damage to the `max(1)` floor. Defaults as above.
    #[serde(default = "default_party_defense_con_divisor")]
    pub party_defense_con_divisor: PositiveInteger,
    /// Percent of an enemy's EXP yield paid as gold when it has no authored `gp` (B3.2). Neither
    /// engine paid combat gold at all; magic-core exchange was the only faucet, and it swings
    /// from 8 GP per kill in zone 1 to over 5,000 in zone 10. Deriving from EXP keeps the new
    /// faucet meaningful early, where money is tight, and negligible late, where cores dominate.
    #[serde(default = "default_gp_per_exp_percent")]
    pub gp_per_exp_percent: PositiveInteger,
    /// Percent chance a defeated non-boss enemy's magic core shatters and drops nothing.
    #[serde(default = "default_mc_break_percent")]
    pub mc_break_percent: u32,
    /// The same chance when the killing blow was a critical hit, a spell or an item attack.
    #[serde(default = "default_mc_break_rough_percent")]
    pub mc_break_rough_percent: u32,
}

fn default_mc_break_percent() -> u32 {
    20
}

fn default_mc_break_rough_percent() -> u32 {
    30
}

fn default_gp_per_exp_percent() -> PositiveInteger {
    PositiveInteger::new(33).expect("constant is nonzero")
}

fn default_hp_growth_con_divisor() -> PositiveInteger {
    PositiveInteger::new(4).expect("constant is nonzero")
}

fn default_party_defense_con_divisor() -> PositiveInteger {
    PositiveInteger::new(10).expect("constant is nonzero")
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

/// Out-of-battle MP regeneration for living party members.
///
/// Each tick restores `max_percent_per_tick` percent of a member's max MP, carrying fractional
/// points between ticks so all pool sizes refill at the same percentage rate. A map with
/// ambient encounters ticks on steps while walking and on elapsed World time while standing; a
/// safe map (no encounter zone, i.e. towns and interiors) ticks faster on steps and on elapsed
/// World time whether standing or walking.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MpRegenBalance {
    pub field_steps_per_tick: PositiveInteger,
    /// Field-rest interval. This only accrues while the player is standing still on an encounter map.
    #[serde(default = "default_field_seconds_per_tick")]
    pub field_seconds_per_tick: PositiveFinite,
    pub safe_steps_per_tick: PositiveInteger,
    pub safe_seconds_per_tick: PositiveFinite,
    pub max_percent_per_tick: PositiveInteger,
}

impl Default for MpRegenBalance {
    fn default() -> Self {
        Self {
            field_steps_per_tick: PositiveInteger::new(8).expect("constant is nonzero"),
            field_seconds_per_tick: default_field_seconds_per_tick(),
            safe_steps_per_tick: PositiveInteger::new(4).expect("constant is nonzero"),
            safe_seconds_per_tick: PositiveFinite::new(2.0)
                .expect("constant is positive and finite"),
            max_percent_per_tick: PositiveInteger::new(1).expect("constant is nonzero"),
        }
    }
}

fn default_field_seconds_per_tick() -> PositiveFinite {
    PositiveFinite::new(5.0).expect("constant is positive and finite")
}

/// Largest `wander_range_tiles` accepted; keeps the pixel range and its span far from overflow.
const MAX_WANDER_RANGE_TILES: u32 = 1024;
/// Largest `heal_threshold_span_percent` accepted; the jitter is hundredths of a ratio.
const MAX_HEAL_THRESHOLD_SPAN_PERCENT: u32 = 100;

/// Field enemy movement tuning.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(try_from = "RawWorldEnemyBalance")]
pub struct WorldEnemyBalance {
    /// Chase and wander speed in tiles per second. Default 3.5.
    pub move_speed_tiles_per_second: PositiveFinite,
    /// Radius in tiles around its origin within which an idle enemy picks wander targets. The
    /// range must be between 1 and 1024. Default 4.
    pub wander_range_tiles: PositiveInteger,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawWorldEnemyBalance {
    #[serde(default = "default_enemy_move_speed_tiles_per_second")]
    move_speed_tiles_per_second: PositiveFinite,
    #[serde(default = "default_enemy_wander_range_tiles")]
    wander_range_tiles: PositiveInteger,
}

impl TryFrom<RawWorldEnemyBalance> for WorldEnemyBalance {
    type Error = String;

    fn try_from(raw: RawWorldEnemyBalance) -> Result<Self, Self::Error> {
        if raw.wander_range_tiles.get() > MAX_WANDER_RANGE_TILES {
            return Err(format!(
                "wander_range_tiles must be at most {MAX_WANDER_RANGE_TILES}"
            ));
        }
        Ok(Self {
            move_speed_tiles_per_second: raw.move_speed_tiles_per_second,
            wander_range_tiles: raw.wander_range_tiles,
        })
    }
}

impl Default for WorldEnemyBalance {
    fn default() -> Self {
        Self {
            move_speed_tiles_per_second: default_enemy_move_speed_tiles_per_second(),
            wander_range_tiles: default_enemy_wander_range_tiles(),
        }
    }
}

fn default_enemy_move_speed_tiles_per_second() -> PositiveFinite {
    PositiveFinite::new(3.5).expect("constant is positive and finite")
}

fn default_enemy_wander_range_tiles() -> PositiveInteger {
    PositiveInteger::new(4).expect("constant is nonzero")
}

/// Auto-battle party member tuning.
///
/// A healer treats allies below `heal_threshold_base + jitter` as wounded, where the jitter is
/// drawn once per heal decision as a whole number of hundredths in `0..=heal_threshold_span_percent`.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(try_from = "RawPartyAiBalance")]
pub struct PartyAiBalance {
    /// Health ratio below which an ally counts as wounded before jitter. Default 0.40.
    pub heal_threshold_base: UnitInterval,
    /// Inclusive upper bound of the jitter, in hundredths of a health ratio (0 to 100; zero still
    /// consumes the same single RNG draw). Default 20.
    pub heal_threshold_span_percent: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPartyAiBalance {
    #[serde(default = "default_heal_threshold_base")]
    heal_threshold_base: UnitInterval,
    #[serde(default = "default_heal_threshold_span_percent")]
    heal_threshold_span_percent: u32,
}

impl TryFrom<RawPartyAiBalance> for PartyAiBalance {
    type Error = String;

    fn try_from(raw: RawPartyAiBalance) -> Result<Self, Self::Error> {
        if raw.heal_threshold_span_percent > MAX_HEAL_THRESHOLD_SPAN_PERCENT {
            return Err(format!(
                "heal_threshold_span_percent must be at most {MAX_HEAL_THRESHOLD_SPAN_PERCENT}"
            ));
        }
        Ok(Self {
            heal_threshold_base: raw.heal_threshold_base,
            heal_threshold_span_percent: raw.heal_threshold_span_percent,
        })
    }
}

impl Default for PartyAiBalance {
    fn default() -> Self {
        Self {
            heal_threshold_base: default_heal_threshold_base(),
            heal_threshold_span_percent: default_heal_threshold_span_percent(),
        }
    }
}

fn default_heal_threshold_base() -> UnitInterval {
    UnitInterval::new(0.4).expect("constant is a unit interval")
}

fn default_heal_threshold_span_percent() -> u32 {
    20
}

/// Damage-over-time status tuning.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StatusDamageBalance {
    /// Burn deals the caster's ATTACK divided by this per turn, at least 1. Default 10.
    #[serde(default = "default_burn_attack_divisor")]
    pub burn_attack_divisor: PositiveInteger,
}

impl Default for StatusDamageBalance {
    fn default() -> Self {
        Self {
            burn_attack_divisor: default_burn_attack_divisor(),
        }
    }
}

fn default_burn_attack_divisor() -> PositiveInteger {
    PositiveInteger::new(10).expect("constant is nonzero")
}

/// A positive integer scalar, including the underscore-separated YAML integer spelling used by
/// the balance file. `serde_yaml_ng` presents that legal YAML spelling as text, so this
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
    use super::{BalanceData, PartyAiBalance, StatusDamageBalance, WorldEnemyBalance};
    use crate::scenario_yaml;
    use crate::test_support::scenario_package_dir;
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
        assert_eq!(balance.mp_regen, super::MpRegenBalance::default());
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
    fn an_authored_mp_regen_group_must_be_complete_and_positive() {
        let valid = include_str!("../../../tests/fixtures/balance-complete.yaml");
        let group = "mp_regen:\n  field_steps_per_tick: 10\n  safe_steps_per_tick: 3\n  \
                     safe_seconds_per_tick: 1.5\n  max_percent_per_tick: 2\n";
        let authored = format!("{valid}\n{group}");
        let balance: BalanceData =
            scenario_yaml::from_str(&authored).expect("complete mp_regen group should load");
        assert_eq!(balance.mp_regen.field_steps_per_tick.get(), 10);
        assert_eq!(balance.mp_regen.field_seconds_per_tick.get(), 5.0);
        assert_eq!(balance.mp_regen.safe_steps_per_tick.get(), 3);
        assert_eq!(balance.mp_regen.safe_seconds_per_tick.get(), 1.5);
        assert_eq!(balance.mp_regen.max_percent_per_tick.get(), 2);

        let with_field_rest = format!("{authored}  field_seconds_per_tick: 7.0\n");
        let custom: BalanceData = scenario_yaml::from_str(&with_field_rest).unwrap();
        assert_eq!(custom.mp_regen.field_seconds_per_tick.get(), 7.0);

        for document in [
            with_field_rest.replace("field_seconds_per_tick: 7.0", "field_seconds_per_tick: 0.0"),
            with_field_rest.replace(
                "field_seconds_per_tick: 7.0",
                "field_seconds_per_tick: -1.0",
            ),
            authored.replacen("field_steps_per_tick: 10", "field_steps_per_tick: 0", 1),
            authored.replacen(
                "safe_seconds_per_tick: 1.5",
                "safe_seconds_per_tick: 0.0",
                1,
            ),
            authored.replacen("  max_percent_per_tick: 2\n", "", 1),
        ] {
            assert!(
                scenario_yaml::from_str::<BalanceData>(&document).is_err(),
                "accepted:\n{document}"
            );
        }
    }

    #[test]
    fn audits_the_complete_shipped_balance_file() {
        let path = scenario_package_dir().join("data/balance.yaml");
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
        assert_eq!(balance.mp_regen.field_steps_per_tick.get(), 8);
        assert_eq!(balance.mp_regen.field_seconds_per_tick.get(), 5.0);
        assert_eq!(balance.mp_regen.safe_steps_per_tick.get(), 4);
        assert_eq!(balance.mp_regen.safe_seconds_per_tick.get(), 2.0);
        assert_eq!(balance.mp_regen.max_percent_per_tick.get(), 1);
        assert_eq!(balance.world_enemy, WorldEnemyBalance::default());
        assert_eq!(balance.party_ai, PartyAiBalance::default());
        assert_eq!(balance.status_damage, StatusDamageBalance::default());
    }

    #[test]
    fn tuning_groups_default_to_the_former_code_literals() {
        let balance = BalanceData::default();
        assert_eq!(balance.world_enemy.move_speed_tiles_per_second.get(), 3.5);
        assert_eq!(balance.world_enemy.wander_range_tiles.get(), 4);
        assert_eq!(balance.party_ai.heal_threshold_base.get(), 0.4);
        assert_eq!(balance.party_ai.heal_threshold_span_percent, 20);
        assert_eq!(balance.status_damage.burn_attack_divisor.get(), 10);
        let fixture: BalanceData = scenario_yaml::from_str(include_str!(
            "../../../tests/fixtures/balance-complete.yaml"
        ))
        .unwrap();
        assert_eq!(fixture.world_enemy, balance.world_enemy);
        assert_eq!(fixture.party_ai, balance.party_ai);
        assert_eq!(fixture.status_damage, balance.status_damage);
    }

    #[test]
    fn authored_tuning_groups_load_and_default_per_key() {
        let valid = include_str!("../../../tests/fixtures/balance-complete.yaml");
        let authored = format!(
            "{valid}\nworld_enemy:\n  move_speed_tiles_per_second: 2.0\n  wander_range_tiles: 2\n\
             party_ai:\n  heal_threshold_base: 0.5\n  heal_threshold_span_percent: 0\n\
             status_damage:\n  burn_attack_divisor: 5\n"
        );
        let balance: BalanceData = scenario_yaml::from_str(&authored).unwrap();
        assert_eq!(balance.world_enemy.move_speed_tiles_per_second.get(), 2.0);
        assert_eq!(balance.world_enemy.wander_range_tiles.get(), 2);
        assert_eq!(balance.party_ai.heal_threshold_base.get(), 0.5);
        assert_eq!(balance.party_ai.heal_threshold_span_percent, 0);
        assert_eq!(balance.status_damage.burn_attack_divisor.get(), 5);

        let partial = format!("{valid}\nworld_enemy:\n  wander_range_tiles: 6\n");
        let balance: BalanceData = scenario_yaml::from_str(&partial).unwrap();
        assert_eq!(balance.world_enemy.move_speed_tiles_per_second.get(), 3.5);
        assert_eq!(balance.world_enemy.wander_range_tiles.get(), 6);
        let empty = format!("{valid}\nstatus_damage: {{}}\n");
        let balance: BalanceData = scenario_yaml::from_str(&empty).unwrap();
        assert_eq!(balance.status_damage, StatusDamageBalance::default());
    }

    #[test]
    fn invalid_tuning_values_are_rejected() {
        let valid = include_str!("../../../tests/fixtures/balance-complete.yaml");
        for group in [
            "world_enemy:\n  move_speed_tiles_per_second: 0.0\n",
            "world_enemy:\n  move_speed_tiles_per_second: -1.0\n",
            "world_enemy:\n  wander_range_tiles: 0\n",
            "world_enemy:\n  wander_range_tiles: 1025\n",
            "world_enemy:\n  wander_range_tiles: 4294967295\n",
            "world_enemy:\n  mystery: 1\n",
            "world_enemy: null\n",
            "party_ai:\n  heal_threshold_base: 1.5\n",
            "party_ai:\n  heal_threshold_base: -0.1\n",
            "party_ai:\n  heal_threshold_span_percent: 101\n",
            "party_ai:\n  heal_threshold_span_percent: -1\n",
            "party_ai: null\n",
            "status_damage:\n  burn_attack_divisor: 0\n",
            "status_damage:\n  burn_attack_divisor: -3\n",
            "status_damage:\n  burn_attack_divisor: null\n",
            "status_damage:\n  mystery: 1\n",
            "status_damage: null\n",
        ] {
            let document = format!("{valid}\n{group}");
            assert!(
                scenario_yaml::from_str::<BalanceData>(&document).is_err(),
                "accepted:\n{document}"
            );
        }
    }
}
