//! Source-authored class and ability schemas.
//!
//! The pinned `0897035` corpus contains five mapping-root class files and 42 abilities. Class
//! records share one closed shape, except that only the rogue declares `passive_bonuses`.
//! Abilities use six top-level `type` values and several field-signature variants beneath those
//! values. [`AbilityKind`] retains those signatures instead of storing a bag of optional fields.
//!
//! Ability `target` is the authored battle/field target vocabulary. Buffs and debuffs in the
//! pinned data intentionally omit that top-level field and put their target in nested effect
//! mappings such as [`BuffEffect`] and [`StatModifierEffect`].
//! The field-menu teleport is represented by [`UtilityAbility::Warp`], preserving its authored
//! `warp: select` marker. Runtime field/battle filtering and effect behavior belong to later
//! milestones.

use crate::scenario_party::PartyRow;
use crate::scenario_yaml::{deserialize_string, deserialize_strings};
use serde::{Deserialize, Deserializer, de::Error as _};
use std::fmt;
use std::num::NonZeroU32;

/// One mapping-root class YAML document.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ClassDefinition {
    #[serde(rename = "class", deserialize_with = "deserialize_string")]
    pub class_id: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub name: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub description: String,
    pub base_hp: NonZeroU32,
    pub base_mp: u32,
    pub default_row: PartyRow,
    pub stat_growth: ClassStatGrowth,
    pub exp_curve: ExperienceCurve,
    pub exp_base: NonZeroU32,
    pub exp_factor: PositiveFinite,
    pub equipment_slots: ClassEquipmentSlots,
    #[serde(default, deserialize_with = "deserialize_present_option")]
    pub passive_bonuses: Option<ClassPassiveBonuses>,
    pub abilities: Vec<Ability>,
}

/// Ten-level repeating stat-growth tables consumed by the pinned level-up logic.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ClassStatGrowth {
    #[serde(rename = "str")]
    pub strength: [u32; 10],
    pub dex: [u32; 10],
    pub con: [u32; 10],
    #[serde(rename = "int")]
    pub intelligence: [u32; 10],
}

/// The fixed equipment-slot compatibility mappings in every pinned class.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ClassEquipmentSlots {
    #[serde(deserialize_with = "deserialize_strings")]
    pub weapon: Vec<String>,
    #[serde(deserialize_with = "deserialize_strings")]
    pub shield: Vec<String>,
    #[serde(deserialize_with = "deserialize_strings")]
    pub helmet: Vec<String>,
    #[serde(deserialize_with = "deserialize_strings")]
    pub body: Vec<String>,
    #[serde(deserialize_with = "deserialize_strings")]
    pub accessory: Vec<String>,
}

/// Optional class-wide bonuses. When the block is present, all three pinned fields are required.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ClassPassiveBonuses {
    pub encounter_rate_reduction: UnitInterval,
    pub flee_success_bonus: UnitInterval,
    pub chest_trap_detect: bool,
}

/// Experience formulas authored by the pinned class corpus.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExperienceCurve {
    Quadratic,
}

/// One class ability with fields shared by every top-level ability type.
#[derive(Clone, Debug, PartialEq)]
pub struct Ability {
    pub id: String,
    pub name: String,
    pub unlock_level: NonZeroU32,
    /// Missing means level-only unlocking; explicit YAML `null` is rejected.
    pub unlock_flag: Option<String>,
    pub description: String,
    pub mp_cost: u32,
    pub kind: AbilityKind,
}

/// The six top-level `type` values and their source-specific fields.
#[derive(Clone, Debug, PartialEq)]
pub enum AbilityKind {
    Physical(PhysicalAbility),
    Spell(SpellAbility),
    Heal(HealAbility),
    Buff(BuffAbility),
    Debuff(DebuffAbility),
    Utility(UtilityAbility),
}

/// A physical ability. Optional fields remain optional rather than receiving unobserved defaults.
#[derive(Clone, Debug, PartialEq)]
pub struct PhysicalAbility {
    pub attack_range: AttackRange,
    pub damage_coeff: PositiveFinite,
    pub target: AbilityTarget,
    pub effect: Option<StatModifierEffect>,
    pub hits: Option<NonZeroU32>,
    pub guaranteed_crit: Option<bool>,
    pub instant_kill: Option<InstantKill>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InstantKill {
    pub chance: UnitInterval,
    pub blocked_by: Vec<InstantKillBlocker>,
}

/// An offensive elemental spell. Missing `side_effects` has the observed empty-list default.
#[derive(Clone, Debug, PartialEq)]
pub struct SpellAbility {
    pub element: AbilityElement,
    pub spell_coeff: PositiveFinite,
    pub target: AbilityTarget,
    pub side_effects: Vec<AbilitySideEffect>,
}

/// A healing ability, classified by the mutually exclusive source field signatures.
#[derive(Clone, Debug, PartialEq)]
pub struct HealAbility {
    pub element: Option<AbilityElement>,
    pub target: AbilityTarget,
    pub healing: HealingMethod,
}

#[derive(Clone, Debug, PartialEq)]
pub enum HealingMethod {
    Restore {
        coefficient: PositiveFinite,
        max_hp_percent: Option<UnitInterval>,
    },
    Revive {
        hp_percent: UnitInterval,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct BuffAbility {
    pub element: Option<AbilityElement>,
    pub effect: BuffEffect,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DebuffAbility {
    pub effect: StatModifierEffect,
}

/// The three field signatures exercised by current utility abilities.
#[derive(Clone, Debug, PartialEq)]
pub enum UtilityAbility {
    RemoveStatus {
        target: AbilityTarget,
        effect: RemoveStatusEffect,
    },
    Steal {
        target: AbilityTarget,
        chance: UnitInterval,
    },
    Warp {
        target: AbilityTarget,
        mode: WarpMode,
    },
}

/// Top-level targets authored on current class abilities.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AbilityTarget {
    SingleEnemy,
    AllEnemies,
    GroupEnemies,
    SingleAlly,
    #[serde(rename = "self")]
    SelfTarget,
    AllAllies,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AttackRange {
    Melee,
    Ranged,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AbilityElement {
    Fire,
    Water,
    Wind,
    Earth,
    Holy,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InstantKillBlocker {
    Boss,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WarpMode {
    Select,
}

/// The four closed effect mappings accepted by current buff abilities.
#[derive(Clone, Debug, PartialEq)]
pub enum BuffEffect {
    StatModifier(StatModifierEffect),
    Aggro(AggroEffect),
    RedirectDamage(RedirectDamageEffect),
    DamageReduction(DamageReductionEffect),
}

/// Private wire enum used to classify nested effect mappings by closed field signature.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(untagged)]
enum AbilityEffectDocument {
    StatModifier(StatModifierEffect),
    RemoveStatus(RemoveStatusEffect),
    Aggro(AggroEffect),
    RedirectDamage(RedirectDamageEffect),
    DamageReduction(DamageReductionEffect),
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StatModifierEffect {
    pub target: EffectTarget,
    pub stat: EffectStat,
    pub modifier: PositiveFinite,
    pub duration_turns: NonZeroU32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RemoveStatusEffect {
    pub remove_status: AllValue,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AggroEffect {
    pub target: EffectTarget,
    pub aggro: AllValue,
    pub duration_turns: NonZeroU32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RedirectDamageEffect {
    pub target: EffectTarget,
    pub redirect_damage_to: SelfValue,
    pub duration_turns: NonZeroU32,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DamageReductionEffect {
    pub target: EffectTarget,
    pub damage_reduction: UnitInterval,
    pub duration_turns: NonZeroU32,
    pub restrict_actions: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EffectTarget {
    Party,
    AllEnemies,
    SingleEnemy,
    #[serde(rename = "self")]
    SelfTarget,
    SingleAlly,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EffectStat {
    #[serde(rename = "atk")]
    Attack,
    #[serde(rename = "def")]
    Defense,
    #[serde(rename = "mres")]
    MagicResistance,
    HitChance,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AllValue {
    All,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SelfValue {
    #[serde(rename = "self")]
    SelfTarget,
}

/// The five side-effect discriminants authored by current class abilities.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum AbilitySideEffect {
    Burn {
        chance: UnitInterval,
        #[serde(deserialize_with = "deserialize_string")]
        damage_per_turn: String,
        duration_turns: NonZeroU32,
        timing: SideEffectTiming,
    },
    Freeze {
        chance: UnitInterval,
        duration_turns: NonZeroU32,
    },
    Stun {
        chance: UnitInterval,
        duration_turns: NonZeroU32,
    },
    Silence {
        chance: UnitInterval,
        duration_turns: NonZeroU32,
    },
    Knockback {
        chance: UnitInterval,
        atk_modifier: PositiveFinite,
        duration_turns: NonZeroU32,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SideEffectTiming {
    EndOfTurn,
}

/// A finite YAML floating-point scalar greater than zero.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct PositiveFinite(f64);

impl PositiveFinite {
    /// Creates a positive finite value, or returns `None` for zero, negative, or non-finite
    /// input.
    pub fn new(value: f64) -> Option<Self> {
        (value.is_finite() && value > 0.0).then_some(Self(value))
    }

    pub fn get(self) -> f64 {
        self.0
    }
}

impl<'de> Deserialize<'de> for PositiveFinite {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = StrictFiniteFloat::deserialize(deserializer)?.0;
        if value > 0.0 {
            Ok(Self(value))
        } else {
            Err(D::Error::custom(
                "expected a floating-point number greater than zero",
            ))
        }
    }
}

/// A finite YAML floating-point scalar in the inclusive range `0.0..=1.0`.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct UnitInterval(f64);

impl UnitInterval {
    pub fn get(self) -> f64 {
        self.0
    }
}

impl<'de> Deserialize<'de> for UnitInterval {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = StrictFiniteFloat::deserialize(deserializer)?.0;
        if (0.0..=1.0).contains(&value) {
            Ok(Self(value))
        } else {
            Err(D::Error::custom(
                "expected a floating-point number between zero and one",
            ))
        }
    }
}

struct StrictFiniteFloat(f64);

impl<'de> Deserialize<'de> for StrictFiniteFloat {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(StrictFiniteFloatVisitor)
    }
}

struct StrictFiniteFloatVisitor;

impl serde::de::Visitor<'_> for StrictFiniteFloatVisitor {
    type Value = StrictFiniteFloat;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a finite YAML floating-point scalar")
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if value.is_finite() {
            Ok(StrictFiniteFloat(value))
        } else {
            Err(E::custom("expected a finite floating-point number"))
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AbilityDocument {
    #[serde(deserialize_with = "deserialize_string")]
    id: String,
    #[serde(deserialize_with = "deserialize_string")]
    name: String,
    unlock_level: NonZeroU32,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    unlock_flag: Option<String>,
    #[serde(rename = "type")]
    ability_type: AbilityType,
    #[serde(default)]
    attack_range: OptionalField<AttackRange>,
    #[serde(default)]
    damage_coeff: OptionalField<PositiveFinite>,
    #[serde(default)]
    effect: OptionalField<AbilityEffectDocument>,
    #[serde(default)]
    element: OptionalField<AbilityElement>,
    #[serde(deserialize_with = "deserialize_string")]
    description: String,
    mp_cost: u32,
    #[serde(default)]
    guaranteed_crit: OptionalField<bool>,
    #[serde(default)]
    heal_coeff: OptionalField<PositiveFinite>,
    #[serde(default)]
    heal_pct: OptionalField<UnitInterval>,
    #[serde(default)]
    hits: OptionalField<NonZeroU32>,
    #[serde(default)]
    instant_kill_blocked_by: OptionalField<Vec<InstantKillBlocker>>,
    #[serde(default)]
    instant_kill_chance: OptionalField<UnitInterval>,
    #[serde(default)]
    revive_hp_pct: OptionalField<UnitInterval>,
    #[serde(default)]
    side_effects: OptionalField<Vec<AbilitySideEffect>>,
    #[serde(default)]
    spell_coeff: OptionalField<PositiveFinite>,
    #[serde(default)]
    steal_chance: OptionalField<UnitInterval>,
    #[serde(default)]
    target: OptionalField<AbilityTarget>,
    #[serde(default)]
    warp: OptionalField<WarpMode>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum AbilityType {
    Physical,
    Spell,
    Heal,
    Buff,
    Debuff,
    Utility,
}

impl<'de> Deserialize<'de> for Ability {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        AbilityDocument::deserialize(deserializer)?
            .into_ability()
            .map_err(D::Error::custom)
    }
}

impl AbilityDocument {
    fn into_ability(self) -> Result<Ability, String> {
        let Self {
            id,
            name,
            unlock_level,
            unlock_flag,
            ability_type,
            attack_range,
            damage_coeff,
            effect,
            element,
            description,
            mp_cost,
            guaranteed_crit,
            heal_coeff,
            heal_pct,
            hits,
            instant_kill_blocked_by,
            instant_kill_chance,
            revive_hp_pct,
            side_effects,
            spell_coeff,
            steal_chance,
            target,
            warp,
        } = self;

        let fields = AbilityFields {
            attack_range,
            damage_coeff,
            effect,
            element,
            guaranteed_crit,
            heal_coeff,
            heal_pct,
            hits,
            instant_kill_blocked_by,
            instant_kill_chance,
            revive_hp_pct,
            side_effects,
            spell_coeff,
            steal_chance,
            target,
            warp,
        };
        let kind = match ability_type {
            AbilityType::Physical => fields.into_physical()?,
            AbilityType::Spell => fields.into_spell()?,
            AbilityType::Heal => fields.into_heal()?,
            AbilityType::Buff => fields.into_buff()?,
            AbilityType::Debuff => fields.into_debuff()?,
            AbilityType::Utility => fields.into_utility()?,
        };

        Ok(Ability {
            id,
            name,
            unlock_level,
            unlock_flag,
            description,
            mp_cost,
            kind,
        })
    }
}

struct AbilityFields {
    attack_range: OptionalField<AttackRange>,
    damage_coeff: OptionalField<PositiveFinite>,
    effect: OptionalField<AbilityEffectDocument>,
    element: OptionalField<AbilityElement>,
    guaranteed_crit: OptionalField<bool>,
    heal_coeff: OptionalField<PositiveFinite>,
    heal_pct: OptionalField<UnitInterval>,
    hits: OptionalField<NonZeroU32>,
    instant_kill_blocked_by: OptionalField<Vec<InstantKillBlocker>>,
    instant_kill_chance: OptionalField<UnitInterval>,
    revive_hp_pct: OptionalField<UnitInterval>,
    side_effects: OptionalField<Vec<AbilitySideEffect>>,
    spell_coeff: OptionalField<PositiveFinite>,
    steal_chance: OptionalField<UnitInterval>,
    target: OptionalField<AbilityTarget>,
    warp: OptionalField<WarpMode>,
}

impl AbilityFields {
    fn into_physical(self) -> Result<AbilityKind, String> {
        reject_present(
            "physical",
            [
                ("element", self.element.is_present()),
                ("heal_coeff", self.heal_coeff.is_present()),
                ("heal_pct", self.heal_pct.is_present()),
                ("revive_hp_pct", self.revive_hp_pct.is_present()),
                ("side_effects", self.side_effects.is_present()),
                ("spell_coeff", self.spell_coeff.is_present()),
                ("steal_chance", self.steal_chance.is_present()),
                ("warp", self.warp.is_present()),
            ],
        )?;
        let instant_kill = match (
            self.instant_kill_chance.into_option(),
            self.instant_kill_blocked_by.into_option(),
        ) {
            (None, None) => None,
            (Some(chance), Some(blocked_by)) if !blocked_by.is_empty() => {
                Some(InstantKill { chance, blocked_by })
            }
            (Some(_), Some(_)) => {
                return Err("physical ability `instant_kill_blocked_by` must not be empty".into());
            }
            _ => {
                return Err(
                    "physical ability must define `instant_kill_chance` and `instant_kill_blocked_by` together"
                        .into(),
                );
            }
        };
        let effect = match self.effect.into_option() {
            None => None,
            Some(AbilityEffectDocument::StatModifier(effect)) => Some(effect),
            Some(_) => {
                return Err("physical ability accepts only a stat-modifier `effect`".into());
            }
        };
        Ok(AbilityKind::Physical(PhysicalAbility {
            attack_range: self.attack_range.required("physical", "attack_range")?,
            damage_coeff: self.damage_coeff.required("physical", "damage_coeff")?,
            target: self.target.required("physical", "target")?,
            effect,
            hits: self.hits.into_option(),
            guaranteed_crit: self.guaranteed_crit.into_option(),
            instant_kill,
        }))
    }

    fn into_spell(self) -> Result<AbilityKind, String> {
        reject_present(
            "spell",
            [
                ("attack_range", self.attack_range.is_present()),
                ("damage_coeff", self.damage_coeff.is_present()),
                ("effect", self.effect.is_present()),
                ("guaranteed_crit", self.guaranteed_crit.is_present()),
                ("heal_coeff", self.heal_coeff.is_present()),
                ("heal_pct", self.heal_pct.is_present()),
                ("hits", self.hits.is_present()),
                (
                    "instant_kill_blocked_by",
                    self.instant_kill_blocked_by.is_present(),
                ),
                ("instant_kill_chance", self.instant_kill_chance.is_present()),
                ("revive_hp_pct", self.revive_hp_pct.is_present()),
                ("steal_chance", self.steal_chance.is_present()),
                ("warp", self.warp.is_present()),
            ],
        )?;
        Ok(AbilityKind::Spell(SpellAbility {
            element: self.element.required("spell", "element")?,
            spell_coeff: self.spell_coeff.required("spell", "spell_coeff")?,
            target: self.target.required("spell", "target")?,
            side_effects: self.side_effects.into_option().unwrap_or_default(),
        }))
    }

    fn into_heal(self) -> Result<AbilityKind, String> {
        reject_present(
            "heal",
            [
                ("attack_range", self.attack_range.is_present()),
                ("damage_coeff", self.damage_coeff.is_present()),
                ("effect", self.effect.is_present()),
                ("guaranteed_crit", self.guaranteed_crit.is_present()),
                ("hits", self.hits.is_present()),
                (
                    "instant_kill_blocked_by",
                    self.instant_kill_blocked_by.is_present(),
                ),
                ("instant_kill_chance", self.instant_kill_chance.is_present()),
                ("side_effects", self.side_effects.is_present()),
                ("spell_coeff", self.spell_coeff.is_present()),
                ("steal_chance", self.steal_chance.is_present()),
                ("warp", self.warp.is_present()),
            ],
        )?;
        let healing = match (
            self.heal_coeff.into_option(),
            self.heal_pct.into_option(),
            self.revive_hp_pct.into_option(),
        ) {
            (Some(coefficient), max_hp_percent, None) => HealingMethod::Restore {
                coefficient,
                max_hp_percent,
            },
            (None, None, Some(hp_percent)) => HealingMethod::Revive { hp_percent },
            _ => {
                return Err(
                    "heal ability must define `heal_coeff` (optionally `heal_pct`) or only `revive_hp_pct`"
                        .into(),
                );
            }
        };
        Ok(AbilityKind::Heal(HealAbility {
            element: self.element.into_option(),
            target: self.target.required("heal", "target")?,
            healing,
        }))
    }

    fn into_buff(self) -> Result<AbilityKind, String> {
        reject_present(
            "buff",
            [
                ("attack_range", self.attack_range.is_present()),
                ("damage_coeff", self.damage_coeff.is_present()),
                ("guaranteed_crit", self.guaranteed_crit.is_present()),
                ("heal_coeff", self.heal_coeff.is_present()),
                ("heal_pct", self.heal_pct.is_present()),
                ("hits", self.hits.is_present()),
                (
                    "instant_kill_blocked_by",
                    self.instant_kill_blocked_by.is_present(),
                ),
                ("instant_kill_chance", self.instant_kill_chance.is_present()),
                ("revive_hp_pct", self.revive_hp_pct.is_present()),
                ("side_effects", self.side_effects.is_present()),
                ("spell_coeff", self.spell_coeff.is_present()),
                ("steal_chance", self.steal_chance.is_present()),
                ("target", self.target.is_present()),
                ("warp", self.warp.is_present()),
            ],
        )?;
        let effect = match self.effect.required("buff", "effect")? {
            AbilityEffectDocument::StatModifier(effect) => BuffEffect::StatModifier(effect),
            AbilityEffectDocument::Aggro(effect) => BuffEffect::Aggro(effect),
            AbilityEffectDocument::RedirectDamage(effect) => BuffEffect::RedirectDamage(effect),
            AbilityEffectDocument::DamageReduction(effect) => BuffEffect::DamageReduction(effect),
            AbilityEffectDocument::RemoveStatus(_) => {
                return Err("buff ability does not accept a remove-status `effect`".into());
            }
        };
        Ok(AbilityKind::Buff(BuffAbility {
            element: self.element.into_option(),
            effect,
        }))
    }

    fn into_debuff(self) -> Result<AbilityKind, String> {
        reject_present(
            "debuff",
            [
                ("attack_range", self.attack_range.is_present()),
                ("damage_coeff", self.damage_coeff.is_present()),
                ("element", self.element.is_present()),
                ("guaranteed_crit", self.guaranteed_crit.is_present()),
                ("heal_coeff", self.heal_coeff.is_present()),
                ("heal_pct", self.heal_pct.is_present()),
                ("hits", self.hits.is_present()),
                (
                    "instant_kill_blocked_by",
                    self.instant_kill_blocked_by.is_present(),
                ),
                ("instant_kill_chance", self.instant_kill_chance.is_present()),
                ("revive_hp_pct", self.revive_hp_pct.is_present()),
                ("side_effects", self.side_effects.is_present()),
                ("spell_coeff", self.spell_coeff.is_present()),
                ("steal_chance", self.steal_chance.is_present()),
                ("target", self.target.is_present()),
                ("warp", self.warp.is_present()),
            ],
        )?;
        let AbilityEffectDocument::StatModifier(effect) =
            self.effect.required("debuff", "effect")?
        else {
            return Err("debuff ability accepts only a stat-modifier `effect`".into());
        };
        Ok(AbilityKind::Debuff(DebuffAbility { effect }))
    }

    fn into_utility(self) -> Result<AbilityKind, String> {
        reject_present(
            "utility",
            [
                ("attack_range", self.attack_range.is_present()),
                ("damage_coeff", self.damage_coeff.is_present()),
                ("element", self.element.is_present()),
                ("guaranteed_crit", self.guaranteed_crit.is_present()),
                ("heal_coeff", self.heal_coeff.is_present()),
                ("heal_pct", self.heal_pct.is_present()),
                ("hits", self.hits.is_present()),
                (
                    "instant_kill_blocked_by",
                    self.instant_kill_blocked_by.is_present(),
                ),
                ("instant_kill_chance", self.instant_kill_chance.is_present()),
                ("revive_hp_pct", self.revive_hp_pct.is_present()),
                ("side_effects", self.side_effects.is_present()),
                ("spell_coeff", self.spell_coeff.is_present()),
            ],
        )?;
        let target = self.target.required("utility", "target")?;
        let utility = match (
            self.effect.into_option(),
            self.steal_chance.into_option(),
            self.warp.into_option(),
        ) {
            (Some(AbilityEffectDocument::RemoveStatus(effect)), None, None) => {
                UtilityAbility::RemoveStatus { target, effect }
            }
            (None, Some(chance), None) => UtilityAbility::Steal { target, chance },
            (None, None, Some(mode)) => UtilityAbility::Warp { target, mode },
            _ => {
                return Err(
                    "utility ability must define exactly one of a remove-status `effect`, `steal_chance`, or `warp`"
                        .into(),
                );
            }
        };
        Ok(AbilityKind::Utility(utility))
    }
}

fn reject_present<const N: usize>(
    ability_type: &str,
    fields: [(&str, bool); N],
) -> Result<(), String> {
    if let Some((field, _)) = fields.into_iter().find(|(_, present)| *present) {
        Err(format!(
            "{ability_type} ability does not accept field `{field}`"
        ))
    } else {
        Ok(())
    }
}

#[derive(Default)]
enum OptionalField<T> {
    #[default]
    Missing,
    Present(T),
}

impl<T> OptionalField<T> {
    fn is_present(&self) -> bool {
        matches!(self, Self::Present(_))
    }

    fn into_option(self) -> Option<T> {
        match self {
            Self::Missing => None,
            Self::Present(value) => Some(value),
        }
    }

    fn required(self, ability_type: &str, field: &str) -> Result<T, String> {
        self.into_option()
            .ok_or_else(|| format!("{ability_type} ability must define required field `{field}`"))
    }
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

fn deserialize_present_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

fn deserialize_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_string(deserializer).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario_yaml;

    #[test]
    fn loads_complete_class_and_every_authored_ability_field_shape() {
        let class: ClassDefinition =
            scenario_yaml::from_str(include_str!("../tests/fixtures/class-ability-shapes.yaml"))
                .expect("source-shaped class fixture should deserialize");

        assert_eq!(class.class_id, "warden");
        assert_eq!(class.name, "Warden");
        assert_eq!(class.base_hp.get(), 24);
        assert_eq!(class.base_mp, 9);
        assert_eq!(class.default_row, PartyRow::Front);
        assert_eq!(class.stat_growth.strength, [2, 3, 2, 3, 2, 3, 2, 3, 2, 3]);
        assert_eq!(class.exp_curve, ExperienceCurve::Quadratic);
        assert_eq!(class.exp_base.get(), 105);
        assert_eq!(class.exp_factor.get(), 2.0);
        assert_eq!(class.equipment_slots.weapon, ["blade", "polearm"]);
        assert!(class.equipment_slots.shield.is_empty());
        let bonuses = class.passive_bonuses.as_ref().expect("bonuses should load");
        assert_eq!(bonuses.encounter_rate_reduction.get(), 0.10);
        assert_eq!(bonuses.flee_success_bonus.get(), 0.30);
        assert!(bonuses.chest_trap_detect);
        assert_eq!(class.abilities.len(), 18);

        let steady = ability(&class, "steady_cut");
        assert_eq!(steady.name, "Steady Cut");
        assert_eq!(steady.unlock_level.get(), 1);
        assert!(steady.unlock_flag.is_none());
        assert_eq!(steady.description, "A measured close-range strike.");
        assert_eq!(steady.mp_cost, 0);
        let AbilityKind::Physical(steady) = &steady.kind else {
            panic!("steady_cut should be physical")
        };
        assert!(steady.effect.is_none());
        assert!(steady.hits.is_none());
        assert!(steady.guaranteed_crit.is_none());
        assert!(steady.instant_kill.is_none());

        let AbilityKind::Physical(physical) = &ability(&class, "double_cut").kind else {
            panic!("double_cut should be physical")
        };
        assert_eq!(physical.attack_range, AttackRange::Melee);
        assert_eq!(physical.damage_coeff.get(), 0.75);
        assert_eq!(physical.target, AbilityTarget::SingleEnemy);
        assert_eq!(physical.hits.expect("hits").get(), 2);

        let AbilityKind::Physical(critical) = &ability(&class, "sure_cut").kind else {
            panic!("sure_cut should be physical")
        };
        assert_eq!(critical.guaranteed_crit, Some(true));

        let AbilityKind::Physical(finisher) = &ability(&class, "final_cut").kind else {
            panic!("final_cut should be physical")
        };
        let instant = finisher
            .instant_kill
            .as_ref()
            .expect("instant kill should load");
        assert_eq!(instant.chance.get(), 0.15);
        assert_eq!(instant.blocked_by, [InstantKillBlocker::Boss]);

        let AbilityKind::Physical(breaker) = &ability(&class, "armor_break").kind else {
            panic!("armor_break should be physical")
        };
        assert!(breaker.effect.is_some());

        let AbilityKind::Spell(spell) = &ability(&class, "prism_storm").kind else {
            panic!("prism_storm should be a spell")
        };
        assert_eq!(spell.element, AbilityElement::Wind);
        assert_eq!(spell.spell_coeff.get(), 1.65);
        assert_eq!(spell.target, AbilityTarget::AllEnemies);
        assert_eq!(spell.side_effects.len(), 5);
        let AbilitySideEffect::Burn {
            chance,
            damage_per_turn,
            duration_turns,
            timing,
        } = &spell.side_effects[0]
        else {
            panic!("first side effect should be burn")
        };
        assert_eq!(chance.get(), 0.25);
        assert_eq!(damage_per_turn, "max(1, floor(caster_atk * 0.08))");
        assert_eq!(duration_turns.get(), 3);
        assert_eq!(*timing, SideEffectTiming::EndOfTurn);
        assert!(matches!(
            spell.side_effects[1],
            AbilitySideEffect::Freeze { .. }
        ));
        assert!(matches!(
            spell.side_effects[2],
            AbilitySideEffect::Stun { .. }
        ));
        assert!(matches!(
            spell.side_effects[3],
            AbilitySideEffect::Silence { .. }
        ));
        assert!(matches!(
            spell.side_effects[4],
            AbilitySideEffect::Knockback { .. }
        ));

        let AbilityKind::Spell(plain_spell) = &ability(&class, "stone_spark").kind else {
            panic!("stone_spark should be a spell")
        };
        assert!(plain_spell.side_effects.is_empty());

        let AbilityKind::Heal(restore) = &ability(&class, "mend").kind else {
            panic!("mend should heal")
        };
        assert_eq!(restore.element, Some(AbilityElement::Holy));
        assert!(matches!(
            restore.healing,
            HealingMethod::Restore {
                max_hp_percent: None,
                ..
            }
        ));

        let AbilityKind::Heal(percent) = &ability(&class, "renew_self").kind else {
            panic!("renew_self should heal")
        };
        let HealingMethod::Restore {
            coefficient,
            max_hp_percent: Some(max_hp_percent),
        } = percent.healing
        else {
            panic!("renew_self should retain coefficient and percent")
        };
        assert_eq!(coefficient.get(), 1.0);
        assert_eq!(max_hp_percent.get(), 0.20);

        let AbilityKind::Heal(revive) = &ability(&class, "return").kind else {
            panic!("return should heal")
        };
        let HealingMethod::Revive { hp_percent } = revive.healing else {
            panic!("return should retain revive percent")
        };
        assert_eq!(hp_percent.get(), 0.45);

        assert!(matches!(
            ability(&class, "cleanse").kind,
            AbilityKind::Utility(UtilityAbility::RemoveStatus { .. })
        ));
        assert!(matches!(
            ability(&class, "pilfer").kind,
            AbilityKind::Utility(UtilityAbility::Steal { .. })
        ));
        let AbilityKind::Utility(UtilityAbility::Steal { target, chance }) =
            &ability(&class, "pilfer").kind
        else {
            panic!("pilfer should retain its steal fields")
        };
        assert_eq!(*target, AbilityTarget::SingleEnemy);
        assert_eq!(chance.get(), 0.35);
        let teleport = ability(&class, "waystep");
        assert_eq!(teleport.unlock_flag.as_deref(), Some("waystep_unlocked"));
        assert!(matches!(
            teleport.kind,
            AbilityKind::Utility(UtilityAbility::Warp {
                target: AbilityTarget::SelfTarget,
                mode: WarpMode::Select,
            })
        ));

        for (id, expected) in [
            ("guard_song", "stat"),
            ("draw_fire", "aggro"),
            ("interpose", "redirect"),
            ("bastion", "reduction"),
        ] {
            let AbilityKind::Buff(value) = &ability(&class, id).kind else {
                panic!("{id} should carry a buff effect")
            };
            assert!(
                matches!(
                    (expected, &value.effect),
                    ("stat", BuffEffect::StatModifier(_))
                        | ("aggro", BuffEffect::Aggro(_))
                        | ("redirect", BuffEffect::RedirectDamage(_))
                        | ("reduction", BuffEffect::DamageReduction(_))
                ),
                "unexpected effect for {id}"
            );
        }

        let AbilityKind::Buff(guard_song) = &ability(&class, "guard_song").kind else {
            panic!("guard_song should be a buff")
        };
        let BuffEffect::StatModifier(modifier) = &guard_song.effect else {
            panic!("guard_song should retain a stat modifier")
        };
        assert_eq!(guard_song.element, Some(AbilityElement::Holy));
        assert_eq!(modifier.target, EffectTarget::Party);
        assert_eq!(modifier.stat, EffectStat::Defense);
        assert_eq!(modifier.modifier.get(), 1.20);
        assert_eq!(modifier.duration_turns.get(), 3);

        let AbilityKind::Debuff(sap) = &ability(&class, "sap").kind else {
            panic!("sap should be a debuff")
        };
        assert_eq!(sap.effect.target, EffectTarget::AllEnemies);
        assert_eq!(sap.effect.stat, EffectStat::Attack);
        assert_eq!(sap.effect.modifier.get(), 0.80);
        assert_eq!(sap.effect.duration_turns.get(), 2);
    }

    #[test]
    fn rejects_malformed_discriminants_types_unknown_fields_and_invalid_ranges() {
        let valid = minimal_class(
            r#"
  - id: ember_arc
    name: Ember Arc
    unlock_level: 1
    type: spell
    element: fire
    description: A small arc of flame.
    mp_cost: 3
    spell_coeff: 1.0
    target: single_enemy
"#,
        );

        for document in [
            valid.replace("type: spell", "type: ritual"),
            valid.replace("type: spell", "type: true"),
            valid.replace("id: ember_arc", "id: 42"),
            valid.replace("mp_cost: 3", "mp_cost: 3.5"),
            valid.replace("spell_coeff: 1.0", "spell_coeff: 1"),
            valid.replace("spell_coeff: 1.0", "spell_coeff: -1.0"),
            valid.replace("target: single_enemy", "target: every_enemy"),
            valid.replace(
                "target: single_enemy",
                "target: single_enemy\n    mystery: true",
            ),
            valid.replace("class: sentinel", "class: sentinel\nmystery: true"),
            valid.replace("exp_factor: 2.0", "exp_factor: 2"),
            valid.replace("exp_factor: 2.0", "exp_factor: .inf"),
            valid.replace("abilities:", "passive_bonuses: null\nabilities:"),
            valid.replace("unlock_level: 1", "unlock_level: 0"),
        ] {
            assert!(
                scenario_yaml::from_str::<ClassDefinition>(&document).is_err(),
                "document should be rejected:\n{document}"
            );
        }
    }

    #[test]
    fn rejects_wrong_variant_fields_and_malformed_nested_effects() {
        for ability in [
            r#"
  - id: bad_spell
    name: Bad Spell
    unlock_level: 1
    type: spell
    element: fire
    description: Invalid cross-variant field.
    mp_cost: 2
    spell_coeff: 1.0
    damage_coeff: 1.0
    target: single_enemy
"#,
            r#"
  - id: bad_heal
    name: Bad Heal
    unlock_level: 1
    type: heal
    description: Missing healing method.
    mp_cost: 2
    target: single_ally
"#,
            r#"
  - id: bad_utility
    name: Bad Utility
    unlock_level: 1
    type: utility
    description: Two utility signatures.
    mp_cost: 2
    target: self
    steal_chance: 0.25
    warp: select
"#,
            r#"
  - id: bad_buff
    name: Bad Buff
    unlock_level: 1
    type: buff
    description: Unknown nested field.
    mp_cost: 2
    effect: { target: self, stat: def, modifier: 1.2, duration_turns: 2, typo: true }
"#,
            r#"
  - id: bad_side_effect
    name: Bad Side Effect
    unlock_level: 1
    type: spell
    element: earth
    description: Unknown side-effect type.
    mp_cost: 2
    spell_coeff: 1.0
    target: single_enemy
    side_effects: [{ type: petrify, chance: 0.5, duration_turns: 1 }]
"#,
            r#"
  - id: bad_chance
    name: Bad Chance
    unlock_level: 1
    type: utility
    description: Chance is outside its domain.
    mp_cost: 2
    target: single_enemy
    steal_chance: 1.1
"#,
        ] {
            let document = minimal_class(ability);
            assert!(scenario_yaml::from_str::<ClassDefinition>(&document).is_err());
        }
    }

    #[test]
    #[ignore = "requires the separately licensed pinned Python source checkout"]
    fn audits_the_pinned_class_corpus_when_requested() {
        let root = std::env::var_os("RPG_S1_PINNED_CLASSES_DIR")
            .map(std::path::PathBuf::from)
            .expect("RPG_S1_PINNED_CLASSES_DIR must name the pinned data/classes directory");
        let mut files = std::fs::read_dir(root)
            .expect("pinned classes directory should be readable")
            .map(|entry| {
                entry
                    .expect("class directory entry should be readable")
                    .path()
            })
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "yaml")
            })
            .collect::<Vec<_>>();
        files.sort();

        let mut ability_count = 0;
        for path in &files {
            let document = std::fs::read_to_string(path)
                .unwrap_or_else(|error| panic!("{} should be readable: {error}", path.display()));
            let class: ClassDefinition = scenario_yaml::from_str(&document)
                .unwrap_or_else(|error| panic!("{} should load: {error}", path.display()));
            ability_count += class.abilities.len();
        }

        assert_eq!(files.len(), 5);
        assert_eq!(ability_count, 42);
    }

    fn ability<'a>(class: &'a ClassDefinition, id: &str) -> &'a Ability {
        class
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .unwrap_or_else(|| panic!("missing ability {id}"))
    }

    fn minimal_class(abilities: &str) -> String {
        format!(
            "class: sentinel\nname: Sentinel\ndescription: A test class.\nbase_hp: 10\nbase_mp: 5\ndefault_row: front\nstat_growth:\n  str: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1]\n  dex: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1]\n  con: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1]\n  int: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1]\nexp_curve: quadratic\nexp_base: 100\nexp_factor: 2.0\nequipment_slots:\n  weapon: []\n  shield: []\n  helmet: []\n  body: []\n  accessory: []\nabilities:\n{abilities}"
        )
    }
}
