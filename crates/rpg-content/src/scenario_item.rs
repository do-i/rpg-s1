//! Source-authored item catalog schemas.
//!
//! The pinned `0897035` scenario has twelve list-root item metadata files plus the separate
//! list-root `field_use.yaml` dispatch catalog. Metadata records are classified by their nine
//! authored `type` discriminants. Consumable, key-item, accessory, and field-use effects are
//! further classified by closed field signatures so unrelated values cannot be combined in a
//! bag of optional fields.
//!
//! Every metadata record has an explicit `id`; item identity is never derived from its filename.
//! The filenames group records for authors, but do not change their schema or identity. Runtime
//! inventory, equipment, shop, and effect behavior belongs to later milestones.

use crate::scenario_class::UnitInterval;
use crate::scenario_yaml::{deserialize_string, deserialize_strings};
use bevy::{asset::Asset, reflect::TypePath};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use std::num::NonZeroU32;

/// One list-root metadata catalog beneath `data/items/`, excluding `field_use.yaml`.
#[derive(Asset, Clone, Debug, Deserialize, PartialEq, TypePath)]
#[serde(transparent)]
pub struct ItemCatalogFile(pub Vec<ItemDefinition>);

impl ItemCatalogFile {
    pub fn entries(&self) -> &[ItemDefinition] {
        &self.0
    }
}

/// The nine metadata item types exercised by the pinned scenario.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ItemDefinition {
    Consumable(ConsumableItem),
    Material(MaterialItem),
    Key(KeyItem),
    MagicCore(MagicCoreItem),
    Weapon(WeaponItem),
    Shield(ShieldItem),
    Helmet(HelmetItem),
    Body(BodyItem),
    Accessory(AccessoryItem),
}

impl ItemDefinition {
    pub fn id(&self) -> &str {
        match self {
            Self::Consumable(item) => &item.id,
            Self::Material(item) => &item.id,
            Self::Key(item) => &item.id,
            Self::MagicCore(item) => &item.id,
            Self::Weapon(item) => &item.id,
            Self::Shield(item) => &item.id,
            Self::Helmet(item) => &item.id,
            Self::Body(item) => &item.id,
            Self::Accessory(item) => &item.id,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConsumableItem {
    #[serde(deserialize_with = "deserialize_string")]
    pub id: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub name: String,
    pub use_context: Vec<ItemUseContext>,
    pub effect: ConsumableEffect,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub buy_price: Option<u32>,
    pub sell_price: u32,
    #[serde(deserialize_with = "deserialize_string")]
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MaterialItem {
    #[serde(deserialize_with = "deserialize_string")]
    pub id: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub name: String,
    pub sell_price: u32,
    #[serde(deserialize_with = "deserialize_string")]
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct KeyItem {
    #[serde(deserialize_with = "deserialize_string")]
    pub id: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub name: String,
    pub usable: bool,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub use_context: Option<Vec<ItemUseContext>>,
    pub effect: KeyItemEffect,
    pub sellable: bool,
    pub droppable: bool,
    #[serde(deserialize_with = "deserialize_string")]
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MagicCoreItem {
    #[serde(deserialize_with = "deserialize_string")]
    pub id: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub name: String,
    pub tags: Vec<ItemTag>,
    pub exchange_rate: NonZeroU32,
    #[serde(deserialize_with = "deserialize_string")]
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WeaponItem {
    #[serde(deserialize_with = "deserialize_string")]
    pub id: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub name: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub slot_category: String,
    pub stats: WeaponStats,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub buy_price: Option<u32>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub sell_price: Option<u32>,
    #[serde(deserialize_with = "deserialize_string")]
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ShieldItem {
    #[serde(deserialize_with = "deserialize_string")]
    pub id: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub name: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub slot_category: String,
    pub stats: ShieldStats,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub buy_price: Option<u32>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub sell_price: Option<u32>,
    #[serde(deserialize_with = "deserialize_string")]
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HelmetItem {
    #[serde(deserialize_with = "deserialize_string")]
    pub id: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub name: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub slot_category: String,
    pub stats: HelmetStats,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub buy_price: Option<u32>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub sell_price: Option<u32>,
    #[serde(deserialize_with = "deserialize_string")]
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BodyItem {
    #[serde(deserialize_with = "deserialize_string")]
    pub id: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub name: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub slot_category: String,
    pub stats: BodyStats,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub buy_price: Option<u32>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub sell_price: Option<u32>,
    #[serde(deserialize_with = "deserialize_string")]
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AccessoryItem {
    #[serde(deserialize_with = "deserialize_string")]
    pub id: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub name: String,
    #[serde(deserialize_with = "deserialize_strings")]
    pub equippable: Vec<String>,
    pub stats: AccessoryStats,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub buy_price: Option<u32>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub sell_price: Option<u32>,
    #[serde(deserialize_with = "deserialize_string")]
    pub description: String,
}

/// Closed nested effect shapes from the five consumable metadata files.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ConsumableEffect {
    Throw(ThrowEffect),
    BypassBarrier(BypassBarrierEffect),
    Rest(RestEffect),
    Action(ItemActionEffect),
    FullRecovery(FullRecoveryEffect),
    RestoreHp(RestoreHpEffect),
    RestoreMp(RestoreMpEffect),
    Revive(ReviveEffect),
    Cure(CureEffect),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ThrowEffect {
    pub damage: NonZeroU32,
    pub element: ItemElement,
    pub target: ThrowTarget,
    #[serde(default, deserialize_with = "deserialize_present_option")]
    pub bonus_vs: Option<Vec<EnemyTrait>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BypassBarrierEffect {
    pub bypass_barrier: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RestEffect {
    pub restore: [RecoveryPool; 2],
    pub cure: Vec<ItemStatus>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ItemActionEffect {
    pub action: ItemAction,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FullRecoveryEffect {
    pub restore_hp: FullValue,
    pub restore_mp: FullValue,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RestoreHpEffect {
    pub restore_hp: NonZeroU32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RestoreMpEffect {
    pub restore_mp: NonZeroU32,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReviveEffect {
    pub revive_hp_pct: UnitInterval,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CureEffect {
    pub cure: Vec<ItemStatus>,
}

/// The two key-item effect shapes in the pinned file.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum KeyItemEffect {
    Revive(ReviveEffect),
    Unlock(UnlockEffect),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UnlockEffect {
    #[serde(deserialize_with = "deserialize_string")]
    pub unlock_flag: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WeaponStats {
    #[serde(
        rename = "str",
        default,
        deserialize_with = "deserialize_present_option"
    )]
    pub strength: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_present_option")]
    pub dex: Option<i32>,
    #[serde(
        rename = "int",
        default,
        deserialize_with = "deserialize_present_option"
    )]
    pub intelligence: Option<i32>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ShieldStats {
    pub con: i32,
    #[serde(default, deserialize_with = "deserialize_present_option")]
    pub dex: Option<i32>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(untagged)]
pub enum HelmetStats {
    Constitution(HelmetConstitutionStats),
    Intelligence(HelmetIntelligenceStats),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HelmetConstitutionStats {
    pub con: i32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HelmetIntelligenceStats {
    #[serde(rename = "int")]
    pub intelligence: i32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BodyStats {
    pub con: i32,
    #[serde(default, deserialize_with = "deserialize_present_option")]
    pub dex: Option<i32>,
    #[serde(
        rename = "int",
        default,
        deserialize_with = "deserialize_present_option"
    )]
    pub intelligence: Option<i32>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum AccessoryStats {
    Encounter(EncounterModifierStats),
    AbilityBlock(AbilityBlockStats),
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EncounterModifierStats {
    pub encounter_modifier: SignedUnitInterval,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AbilityBlockStats {
    #[serde(deserialize_with = "deserialize_string")]
    pub blocks_ability: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ItemUseContext {
    Battle,
    WorldMap,
    Town,
    Dungeon,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ItemTag {
    MagicCore,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ItemElement {
    Fire,
    Water,
    Wind,
    Holy,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ThrowTarget {
    SingleEnemy,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EnemyTrait {
    Undead,
    Demon,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryPool {
    Hp,
    Mp,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemStatus {
    Poison,
    Silence,
    Sleep,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ItemAction {
    Warp,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FullValue {
    Full,
}

/// A finite YAML floating-point scalar in the inclusive range `-1.0..=1.0`.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct SignedUnitInterval(f64);

impl SignedUnitInterval {
    pub fn get(self) -> f64 {
        self.0
    }
}

impl<'de> Deserialize<'de> for SignedUnitInterval {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(SignedUnitIntervalVisitor)
    }
}

struct SignedUnitIntervalVisitor;

impl serde::de::Visitor<'_> for SignedUnitIntervalVisitor {
    type Value = SignedUnitInterval;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a finite YAML floating-point number between -1 and 1")
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if value.is_finite() && (-1.0..=1.0).contains(&value) {
            Ok(SignedUnitInterval(value))
        } else {
            Err(E::custom(
                "expected a finite floating-point number between -1 and 1",
            ))
        }
    }
}

/// The separate list-root `field_use.yaml` catalog.
#[derive(Asset, Clone, Debug, Deserialize, PartialEq, TypePath)]
#[serde(transparent)]
pub struct FieldUseCatalogFile(pub Vec<FieldUseDefinition>);

impl FieldUseCatalogFile {
    pub fn entries(&self) -> &[FieldUseDefinition] {
        &self.0
    }
}

/// Field-menu/battle dispatch variants. Missing `consumable` means `true`, matching Python.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(tag = "effect", rename_all = "snake_case", deny_unknown_fields)]
pub enum FieldUseDefinition {
    RestoreHp {
        #[serde(deserialize_with = "deserialize_string")]
        id: String,
        amount: NonZeroU32,
        target: SingleAliveTarget,
        #[serde(default = "default_consumable")]
        consumable: bool,
    },
    RestoreMp {
        #[serde(deserialize_with = "deserialize_string")]
        id: String,
        amount: NonZeroU32,
        target: SingleAliveTarget,
        #[serde(default = "default_consumable")]
        consumable: bool,
    },
    RestoreFull {
        #[serde(deserialize_with = "deserialize_string")]
        id: String,
        target: FullRecoveryTarget,
        /// Omission means no statuses are cured, matching the Python field-effect loader.
        #[serde(default)]
        cures: Vec<ItemStatus>,
        #[serde(default = "default_consumable")]
        consumable: bool,
    },
    Cure {
        #[serde(deserialize_with = "deserialize_string")]
        id: String,
        cures: Vec<ItemStatus>,
        target: SingleAliveTarget,
        #[serde(default = "default_consumable")]
        consumable: bool,
    },
    Revive {
        #[serde(deserialize_with = "deserialize_string")]
        id: String,
        revive_hp_pct: UnitInterval,
        target: SingleKoTarget,
        #[serde(default = "default_consumable")]
        consumable: bool,
    },
}

impl FieldUseDefinition {
    pub fn id(&self) -> &str {
        match self {
            Self::RestoreHp { id, .. }
            | Self::RestoreMp { id, .. }
            | Self::RestoreFull { id, .. }
            | Self::Cure { id, .. }
            | Self::Revive { id, .. } => id,
        }
    }

    pub fn consumable(&self) -> bool {
        match self {
            Self::RestoreHp { consumable, .. }
            | Self::RestoreMp { consumable, .. }
            | Self::RestoreFull { consumable, .. }
            | Self::Cure { consumable, .. }
            | Self::Revive { consumable, .. } => *consumable,
        }
    }

    pub fn target(&self) -> FieldItemTarget {
        match self {
            Self::RestoreHp { .. } | Self::RestoreMp { .. } | Self::Cure { .. } => {
                FieldItemTarget::SingleAlive
            }
            Self::RestoreFull { target, .. } => match target {
                FullRecoveryTarget::SingleAlive => FieldItemTarget::SingleAlive,
                FullRecoveryTarget::AllAlive => FieldItemTarget::AllAlive,
            },
            Self::Revive { .. } => FieldItemTarget::SingleKo,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FieldItemTarget {
    SingleAlive,
    SingleKo,
    AllAlive,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SingleAliveTarget {
    SingleAlive,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FullRecoveryTarget {
    SingleAlive,
    AllAlive,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SingleKoTarget {
    SingleKo,
}

fn default_consumable() -> bool {
    true
}

fn deserialize_required_nullable<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

fn deserialize_present_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario_yaml;

    const METADATA_FIXTURES: [(&str, &str, usize); 12] = [
        (
            "accessories.yaml",
            include_str!("../../../tests/fixtures/items/accessories.yaml"),
            2,
        ),
        (
            "body.yaml",
            include_str!("../../../tests/fixtures/items/body.yaml"),
            2,
        ),
        (
            "consumables_battle_throw.yaml",
            include_str!("../../../tests/fixtures/items/consumables_battle_throw.yaml"),
            2,
        ),
        (
            "consumables_field.yaml",
            include_str!("../../../tests/fixtures/items/consumables_field.yaml"),
            3,
        ),
        (
            "consumables_recovery.yaml",
            include_str!("../../../tests/fixtures/items/consumables_recovery.yaml"),
            4,
        ),
        (
            "consumables_status_cure.yaml",
            include_str!("../../../tests/fixtures/items/consumables_status_cure.yaml"),
            1,
        ),
        (
            "helmets.yaml",
            include_str!("../../../tests/fixtures/items/helmets.yaml"),
            2,
        ),
        (
            "key_items.yaml",
            include_str!("../../../tests/fixtures/items/key_items.yaml"),
            2,
        ),
        (
            "magic_cores.yaml",
            include_str!("../../../tests/fixtures/items/magic_cores.yaml"),
            1,
        ),
        (
            "materials.yaml",
            include_str!("../../../tests/fixtures/items/materials.yaml"),
            1,
        ),
        (
            "shields.yaml",
            include_str!("../../../tests/fixtures/items/shields.yaml"),
            2,
        ),
        (
            "weapons.yaml",
            include_str!("../../../tests/fixtures/items/weapons.yaml"),
            3,
        ),
    ];

    #[test]
    fn loads_one_complete_fixture_for_every_metadata_category() {
        let mut total = 0;
        for (name, document, expected) in METADATA_FIXTURES {
            let file: ItemCatalogFile = scenario_yaml::from_str(document)
                .unwrap_or_else(|error| panic!("{name} should load: {error}"));
            assert_eq!(file.entries().len(), expected, "wrong count for {name}");
            assert!(file.entries().iter().all(|item| !item.id().is_empty()));
            total += file.entries().len();
        }
        assert_eq!(total, 25);
    }

    #[test]
    fn zone_one_migration_drop_repairs_are_bounded_materials() {
        let catalog: ItemCatalogFile = scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/data/items/migration_zone1_drops.yaml"
        ))
        .unwrap();
        let materials = catalog
            .entries()
            .iter()
            .map(|item| match item {
                ItemDefinition::Material(material) => (material.id.as_str(), material.sell_price),
                _ => panic!("migration drops must not invent equipment or use semantics"),
            })
            .collect::<Vec<_>>();
        assert_eq!(
            materials,
            [
                ("goblin_ear", 10),
                ("goblin_fang", 15),
                ("rusty_blade", 25),
                ("goblin_shield", 30),
            ]
        );
    }

    #[test]
    fn retains_every_consumable_effect_signature() {
        let mut effects = Vec::new();
        for (_, document, _) in METADATA_FIXTURES
            .into_iter()
            .filter(|(name, _, _)| name.starts_with("consumables_"))
        {
            let file: ItemCatalogFile = scenario_yaml::from_str(document).unwrap();
            effects.extend(file.0.into_iter().map(|item| {
                let ItemDefinition::Consumable(item) = item else {
                    panic!("consumable fixture contained a non-consumable")
                };
                item.effect
            }));
        }

        assert_eq!(effects.len(), 10);
        assert!(effects.iter().any(|effect| matches!(
            effect,
            ConsumableEffect::Throw(ThrowEffect { bonus_vs: None, .. })
        )));
        let throw = effects.iter().find_map(|effect| match effect {
            ConsumableEffect::Throw(effect) if effect.bonus_vs.is_some() => Some(effect),
            _ => None,
        });
        assert_eq!(
            throw.and_then(|effect| effect.bonus_vs.as_deref()),
            Some([EnemyTrait::Undead, EnemyTrait::Demon].as_slice())
        );
        assert!(effects.iter().any(|effect| matches!(
            effect,
            ConsumableEffect::BypassBarrier(BypassBarrierEffect {
                bypass_barrier: true
            })
        )));
        assert!(effects.iter().any(|effect| matches!(
            effect,
            ConsumableEffect::Rest(RestEffect {
                restore: [RecoveryPool::Hp, RecoveryPool::Mp],
                ..
            })
        )));
        assert!(effects.iter().any(|effect| matches!(
            effect,
            ConsumableEffect::Action(ItemActionEffect {
                action: ItemAction::Warp
            })
        )));
        assert!(
            effects
                .iter()
                .any(|effect| matches!(effect, ConsumableEffect::RestoreHp(_)))
        );
        assert!(
            effects
                .iter()
                .any(|effect| matches!(effect, ConsumableEffect::RestoreMp(_)))
        );
        assert!(
            effects
                .iter()
                .any(|effect| matches!(effect, ConsumableEffect::FullRecovery(_)))
        );
        assert!(
            effects
                .iter()
                .any(|effect| matches!(effect, ConsumableEffect::Revive(_)))
        );
        assert!(
            effects
                .iter()
                .any(|effect| matches!(effect, ConsumableEffect::Cure(_)))
        );
    }

    #[test]
    fn retains_key_accessory_and_equipment_variants() {
        let keys: ItemCatalogFile = scenario_yaml::from_str(include_str!(
            "../../../tests/fixtures/items/key_items.yaml"
        ))
        .unwrap();
        let ItemDefinition::Key(revive) = &keys.0[0] else {
            panic!("first key should be the revive variant")
        };
        assert!(revive.usable);
        assert_eq!(revive.use_context, Some(vec![ItemUseContext::WorldMap]));
        assert!(matches!(revive.effect, KeyItemEffect::Revive(_)));
        let ItemDefinition::Key(unlock) = &keys.0[1] else {
            panic!("second key should be the unlock variant")
        };
        assert_eq!(unlock.use_context, None);
        assert!(matches!(unlock.effect, KeyItemEffect::Unlock(_)));

        let accessories: ItemCatalogFile = scenario_yaml::from_str(include_str!(
            "../../../tests/fixtures/items/accessories.yaml"
        ))
        .unwrap();
        let ItemDefinition::Accessory(encounter) = &accessories.0[0] else {
            panic!("first accessory should load")
        };
        let AccessoryStats::Encounter(stats) = &encounter.stats else {
            panic!("first accessory should retain the encounter modifier")
        };
        assert_eq!(stats.encounter_modifier.get(), -0.25);
        assert!(matches!(
            accessories.0[1],
            ItemDefinition::Accessory(AccessoryItem {
                stats: AccessoryStats::AbilityBlock(_),
                buy_price: None,
                sell_price: Some(0),
                ..
            })
        ));

        let helmets: ItemCatalogFile = scenario_yaml::from_str(include_str!(
            "../../../tests/fixtures/items/helmets.yaml"
        ))
        .unwrap();
        assert!(matches!(
            helmets.0[0],
            ItemDefinition::Helmet(HelmetItem {
                stats: HelmetStats::Constitution(_),
                ..
            })
        ));
        assert!(matches!(
            helmets.0[1],
            ItemDefinition::Helmet(HelmetItem {
                stats: HelmetStats::Intelligence(_),
                ..
            })
        ));
    }

    #[test]
    fn loads_all_field_use_variants_and_explicit_defaults() {
        let file: FieldUseCatalogFile = scenario_yaml::from_str(include_str!(
            "../../../tests/fixtures/items/field_use.yaml"
        ))
        .unwrap();
        assert_eq!(file.entries().len(), 7);
        assert_eq!(
            file.entries()
                .iter()
                .filter(|entry| !entry.consumable())
                .count(),
            1
        );
        assert!(
            file.entries()[..6]
                .iter()
                .all(FieldUseDefinition::consumable)
        );
        assert!(!file.entries()[6].consumable());
        assert_eq!(file.entries()[0].target(), FieldItemTarget::SingleAlive);
        assert_eq!(file.entries()[3].target(), FieldItemTarget::AllAlive);
        assert_eq!(file.entries()[6].target(), FieldItemTarget::SingleKo);
        assert!(matches!(
            file.entries()[2],
            FieldUseDefinition::RestoreFull { ref cures, .. } if cures.is_empty()
        ));
        assert!(matches!(
            file.entries()[3],
            FieldUseDefinition::RestoreFull { ref cures, .. }
                if cures == &[ItemStatus::Poison, ItemStatus::Silence]
        ));
        assert!(
            file.entries()
                .iter()
                .any(|entry| matches!(entry, FieldUseDefinition::RestoreHp { .. }))
        );
        assert!(
            file.entries()
                .iter()
                .any(|entry| matches!(entry, FieldUseDefinition::RestoreMp { .. }))
        );
        assert!(
            file.entries()
                .iter()
                .any(|entry| matches!(entry, FieldUseDefinition::Cure { .. }))
        );
        assert!(
            file.entries()
                .iter()
                .any(|entry| matches!(entry, FieldUseDefinition::Revive { .. }))
        );
    }

    #[test]
    fn rejects_scalar_coercion_unknown_fields_bad_ranges_and_wrong_shapes() {
        let valid = include_str!("../../../tests/fixtures/items/consumables_recovery.yaml");
        for document in [
            valid.replace("id: leaf_tonic", "id: 42"),
            valid.replace("type: consumable", "type: ritual"),
            valid.replace("restore_hp: 80", "restore_hp: 80.0"),
            valid.replace("sell_price: 30", "sell_price: -30"),
            valid.replace("buy_price: 60", "buy_price: true"),
            valid.replace("revive_hp_pct: 0.75", "revive_hp_pct: 1.25"),
            valid.replace("restore_mp: 40", "restore_mp: 40\n    cure: [sleep]"),
            valid.replace("description:", "mystery: true\n  description:"),
            valid.replace("  buy_price: 60\n", ""),
        ] {
            assert!(
                scenario_yaml::from_str::<ItemCatalogFile>(&document).is_err(),
                "document should be rejected:\n{document}"
            );
        }

        let throws =
            include_str!("../../../tests/fixtures/items/consumables_battle_throw.yaml");
        assert!(
            scenario_yaml::from_str::<ItemCatalogFile>(
                &throws.replace("bonus_vs: [undead, demon]", "bonus_vs: null")
            )
            .is_err()
        );
        assert!(scenario_yaml::from_str::<ItemCatalogFile>("id: not_a_list\n").is_err());
    }

    #[test]
    fn rejects_invalid_field_use_discriminants_and_variant_fields() {
        let valid = include_str!("../../../tests/fixtures/items/field_use.yaml");
        for document in [
            valid.replace("effect: restore_hp", "effect: transmute"),
            valid.replace("id: leaf_tonic", "id: 12"),
            valid.replace("amount: 80", "amount: 0"),
            valid.replace("amount: 80", "amount: 80.0"),
            valid.replace("target: single_alive", "target: everyone"),
            valid.replacen("target: single_alive", "target: single_ko", 1),
            valid.replacen("target: single_ko", "target: all_alive", 1),
            valid.replace("revive_hp_pct: 0.75", "revive_hp_pct: 2.0"),
            valid.replace("  amount: 80\n", ""),
            valid.replace("amount: 80", "amount: 80\n  cures: [poison]"),
        ] {
            assert!(
                scenario_yaml::from_str::<FieldUseCatalogFile>(&document).is_err(),
                "document should be rejected:\n{document}"
            );
        }
    }

    #[test]
    #[ignore = "requires the separately licensed pinned Python source checkout"]
    fn audits_the_pinned_item_corpus_when_requested() {
        let root = std::env::var_os("RPG_S1_PINNED_ITEMS_DIR")
            .map(std::path::PathBuf::from)
            .expect("RPG_S1_PINNED_ITEMS_DIR must name the pinned data/items directory");
        let mut files = std::fs::read_dir(root)
            .expect("pinned items directory should be readable")
            .map(|entry| {
                entry
                    .expect("item directory entry should be readable")
                    .path()
            })
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "yaml")
            })
            .collect::<Vec<_>>();
        files.sort();

        let expected_file_counts = [
            ("accessories.yaml", 3),
            ("body.yaml", 4),
            ("consumables_battle_throw.yaml", 4),
            ("consumables_field.yaml", 4),
            ("consumables_recovery.yaml", 6),
            ("consumables_status_cure.yaml", 4),
            ("field_use.yaml", 13),
            ("helmets.yaml", 4),
            ("key_items.yaml", 2),
            ("magic_cores.yaml", 5),
            ("materials.yaml", 120),
            ("shields.yaml", 4),
            ("weapons.yaml", 12),
        ];
        let mut observed_file_counts = Vec::new();
        let mut metadata_count = 0;
        let mut field_use_count = 0;
        let mut item_types = [0_usize; 9];
        let mut consumable_effects = [0_usize; 9];
        let mut field_effects = [0_usize; 5];

        for path in &files {
            let name = path.file_name().unwrap().to_str().unwrap();
            let document = std::fs::read_to_string(path)
                .unwrap_or_else(|error| panic!("{} should be readable: {error}", path.display()));
            if name == "field_use.yaml" {
                let file: FieldUseCatalogFile = scenario_yaml::from_str(&document)
                    .unwrap_or_else(|error| panic!("{} should load: {error}", path.display()));
                field_use_count += file.0.len();
                observed_file_counts.push((name, file.0.len()));
                for entry in file.0 {
                    field_effects[match entry {
                        FieldUseDefinition::RestoreHp { .. } => 0,
                        FieldUseDefinition::RestoreMp { .. } => 1,
                        FieldUseDefinition::RestoreFull { .. } => 2,
                        FieldUseDefinition::Cure { .. } => 3,
                        FieldUseDefinition::Revive { .. } => 4,
                    }] += 1;
                }
            } else {
                let file: ItemCatalogFile = scenario_yaml::from_str(&document)
                    .unwrap_or_else(|error| panic!("{} should load: {error}", path.display()));
                metadata_count += file.0.len();
                observed_file_counts.push((name, file.0.len()));
                for item in file.0 {
                    let kind = match item {
                        ItemDefinition::Consumable(item) => {
                            consumable_effects[match item.effect {
                                ConsumableEffect::Throw(_) => 0,
                                ConsumableEffect::BypassBarrier(_) => 1,
                                ConsumableEffect::Rest(_) => 2,
                                ConsumableEffect::Action(_) => 3,
                                ConsumableEffect::FullRecovery(_) => 4,
                                ConsumableEffect::RestoreHp(_) => 5,
                                ConsumableEffect::RestoreMp(_) => 6,
                                ConsumableEffect::Revive(_) => 7,
                                ConsumableEffect::Cure(_) => 8,
                            }] += 1;
                            0
                        }
                        ItemDefinition::Material(_) => 1,
                        ItemDefinition::Key(_) => 2,
                        ItemDefinition::MagicCore(_) => 3,
                        ItemDefinition::Weapon(_) => 4,
                        ItemDefinition::Shield(_) => 5,
                        ItemDefinition::Helmet(_) => 6,
                        ItemDefinition::Body(_) => 7,
                        ItemDefinition::Accessory(_) => 8,
                    };
                    item_types[kind] += 1;
                }
            }
        }

        assert_eq!(files.len(), 13);
        assert_eq!(observed_file_counts, expected_file_counts);
        assert_eq!(metadata_count, 172);
        assert_eq!(field_use_count, 13);
        assert_eq!(item_types, [18, 120, 2, 5, 12, 4, 4, 4, 3]);
        assert_eq!(consumable_effects, [4, 1, 2, 1, 1, 2, 2, 1, 4]);
        assert_eq!(field_effects, [2, 2, 3, 4, 2]);
    }
}
