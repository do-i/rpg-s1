//! Mutable state for one runtime party member.
//!
//! [`PartyMember`] remains the immutable scenario definition for portraits, recruitment,
//! authored join conditions, and initial values. This type copies only session state that the
//! pinned Python `MemberState` mutates or saves, plus stable identity/class linkage used to find
//! immutable class data. It deliberately does not cache class growth, abilities, equipment
//! compatibility, item definitions, or inventory ownership.
//!
//! The Python runtime represents an empty equipment slot with `""`; runtime Rust state uses
//! `Option<String>` so absence cannot be confused with an item id. Initial catalog values are
//! validated rather than silently clamped. Later mutations clamp HP, MP, and EXP at their source
//! bounds. Level thresholds/growth and repository/equipment transactions remain later systems.

use std::{error::Error, fmt};

use crate::{
    scenario_balance::ProgressionBalance,
    scenario_party::{PartyEquipment, PartyMember, PartyRow, PartyStats},
};

/// Mutable state for one active or recruitable party member.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeMember {
    id: String,
    name: String,
    protagonist: bool,
    class_id: String,
    level: u32,
    experience: u32,
    health: u32,
    max_health: u32,
    mana: u32,
    max_mana: u32,
    stats: RuntimeMemberStats,
    row: PartyRow,
    equipment: RuntimeEquipment,
}

impl RuntimeMember {
    /// Copies mutable initial state from one validated scenario member definition.
    ///
    /// Referential validation of ids remains the scenario validator's responsibility. This
    /// boundary validates numeric session invariants against the selected balance data and
    /// rejects bad authored state rather than changing it during new-game construction.
    pub fn try_from_catalog(
        member: &PartyMember,
        progression: &ProgressionBalance,
    ) -> Result<Self, RuntimeMemberError> {
        let data = member.data();
        let level_cap = progression.level_cap.get();
        let experience_cap = progression.exp_cap.get();

        if data.level == 0 || data.level > level_cap {
            return Err(RuntimeMemberError::LevelOutsideBalance {
                member_id: data.id.clone(),
                level: data.level,
                level_cap,
            });
        }
        if data.exp > experience_cap {
            return Err(RuntimeMemberError::ExperienceAboveBalance {
                member_id: data.id.clone(),
                experience: data.exp,
                experience_cap,
            });
        }
        if data.hp_max == 0 {
            return Err(RuntimeMemberError::ZeroMaxHealth {
                member_id: data.id.clone(),
            });
        }
        if data.hp > data.hp_max {
            return Err(RuntimeMemberError::HealthAboveMaximum {
                member_id: data.id.clone(),
                health: data.hp,
                max_health: data.hp_max,
            });
        }
        if data.mp > data.mp_max {
            return Err(RuntimeMemberError::ManaAboveMaximum {
                member_id: data.id.clone(),
                mana: data.mp,
                max_mana: data.mp_max,
            });
        }

        Ok(Self {
            id: data.id.clone(),
            name: data.name.clone(),
            protagonist: member.is_protagonist(),
            class_id: data.class_id.clone(),
            level: data.level,
            experience: data.exp,
            health: data.hp,
            max_health: data.hp_max,
            mana: data.mp,
            max_mana: data.mp_max,
            stats: RuntimeMemberStats::from(&data.stats),
            row: data.row,
            equipment: RuntimeEquipment::from(&data.equipped),
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn is_protagonist(&self) -> bool {
        self.protagonist
    }

    pub fn class_id(&self) -> &str {
        &self.class_id
    }

    pub fn level(&self) -> u32 {
        self.level
    }

    pub fn experience(&self) -> u32 {
        self.experience
    }

    pub fn health(&self) -> u32 {
        self.health
    }

    pub fn max_health(&self) -> u32 {
        self.max_health
    }

    pub fn mana(&self) -> u32 {
        self.mana
    }

    pub fn max_mana(&self) -> u32 {
        self.max_mana
    }

    pub fn stats(&self) -> &RuntimeMemberStats {
        &self.stats
    }

    pub fn row(&self) -> PartyRow {
        self.row
    }

    pub fn equipment(&self) -> &RuntimeEquipment {
        &self.equipment
    }

    pub fn is_knocked_out(&self) -> bool {
        self.health == 0
    }

    /// Applies damage, floors health at zero, and returns actual health lost.
    pub fn apply_damage(&mut self, amount: u32) -> u32 {
        let actual = amount.min(self.health);
        self.health -= actual;
        actual
    }

    /// Restores a living member's health and returns the actual amount restored.
    ///
    /// Ordinary source healing does not revive a knocked-out member. Dedicated revive behavior
    /// remains with the later item/spell effect system.
    pub fn restore_health(&mut self, amount: u32) -> u32 {
        if self.is_knocked_out() {
            return 0;
        }
        let before = self.health;
        self.health = self.health.saturating_add(amount).min(self.max_health);
        self.health - before
    }

    /// Spends up to the available mana and returns the actual amount spent.
    pub fn spend_mana(&mut self, amount: u32) -> u32 {
        let actual = amount.min(self.mana);
        self.mana -= actual;
        actual
    }

    /// Restores mana up to its maximum and returns the actual amount restored.
    pub fn restore_mana(&mut self, amount: u32) -> u32 {
        let before = self.mana;
        self.mana = self.mana.saturating_add(amount).min(self.max_mana);
        self.mana - before
    }

    /// Adds EXP up to the selected scenario cap, without applying level thresholds.
    ///
    /// The source awards no EXP once a member is already at the level cap. M10 owns threshold
    /// calculation and level growth; this method only preserves the capped accumulator rule.
    pub fn add_experience(&mut self, amount: u32, progression: &ProgressionBalance) -> u32 {
        if self.level >= progression.level_cap.get() {
            return 0;
        }
        let before = self.experience;
        self.experience = self
            .experience
            .saturating_add(amount)
            .min(progression.exp_cap.get());
        self.experience.saturating_sub(before)
    }

    /// Replaces one slot after a caller has validated inventory ownership and compatibility.
    ///
    /// Those transactional checks belong to M6; this value-level operation rejects the Python
    /// empty-string sentinel and returns the displaced item id for that later transaction.
    pub fn equip(
        &mut self,
        slot: EquipmentSlot,
        item_id: impl Into<String>,
    ) -> Result<Option<String>, EquipmentMutationError> {
        let item_id = item_id.into();
        if item_id.is_empty() {
            return Err(EquipmentMutationError { slot });
        }
        Ok(self.equipment.replace(slot, Some(item_id)))
    }

    /// Clears one slot and returns its previous item id, if any.
    pub fn unequip(&mut self, slot: EquipmentSlot) -> Option<String> {
        self.equipment.replace(slot, None)
    }
}

/// Mutable base stats saved by the Python runtime. Equipment-derived totals remain computed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeMemberStats {
    strength: u32,
    dexterity: u32,
    constitution: u32,
    intelligence: u32,
}

impl RuntimeMemberStats {
    pub fn strength(&self) -> u32 {
        self.strength
    }

    pub fn dexterity(&self) -> u32 {
        self.dexterity
    }

    pub fn constitution(&self) -> u32 {
        self.constitution
    }

    pub fn intelligence(&self) -> u32 {
        self.intelligence
    }
}

impl From<&PartyStats> for RuntimeMemberStats {
    fn from(stats: &PartyStats) -> Self {
        Self {
            strength: stats.strength,
            dexterity: stats.dex,
            constitution: stats.con,
            intelligence: stats.intelligence,
        }
    }
}

/// The five fixed mutable equipment slots, using `None` for unequipped.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RuntimeEquipment {
    weapon: Option<String>,
    shield: Option<String>,
    helmet: Option<String>,
    body: Option<String>,
    accessory: Option<String>,
}

impl RuntimeEquipment {
    pub fn get(&self, slot: EquipmentSlot) -> Option<&str> {
        self.slot(slot).as_deref()
    }

    fn slot(&self, slot: EquipmentSlot) -> &Option<String> {
        match slot {
            EquipmentSlot::Weapon => &self.weapon,
            EquipmentSlot::Shield => &self.shield,
            EquipmentSlot::Helmet => &self.helmet,
            EquipmentSlot::Body => &self.body,
            EquipmentSlot::Accessory => &self.accessory,
        }
    }

    fn replace(&mut self, slot: EquipmentSlot, item: Option<String>) -> Option<String> {
        let target = match slot {
            EquipmentSlot::Weapon => &mut self.weapon,
            EquipmentSlot::Shield => &mut self.shield,
            EquipmentSlot::Helmet => &mut self.helmet,
            EquipmentSlot::Body => &mut self.body,
            EquipmentSlot::Accessory => &mut self.accessory,
        };
        std::mem::replace(target, item)
    }
}

impl From<&PartyEquipment> for RuntimeEquipment {
    fn from(equipment: &PartyEquipment) -> Self {
        fn item(id: &str) -> Option<String> {
            (!id.is_empty()).then(|| id.to_owned())
        }

        Self {
            weapon: item(&equipment.weapon),
            shield: item(&equipment.shield),
            helmet: item(&equipment.helmet),
            body: item(&equipment.body),
            accessory: item(&equipment.accessory),
        }
    }
}

/// Stable equipment-slot vocabulary shared by member state and later equipment services.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EquipmentSlot {
    Weapon,
    Shield,
    Helmet,
    Body,
    Accessory,
}

impl EquipmentSlot {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Weapon => "weapon",
            Self::Shield => "shield",
            Self::Helmet => "helmet",
            Self::Body => "body",
            Self::Accessory => "accessory",
        }
    }
}

/// Invalid authored numeric state at the catalog-to-runtime boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeMemberError {
    LevelOutsideBalance {
        member_id: String,
        level: u32,
        level_cap: u32,
    },
    ExperienceAboveBalance {
        member_id: String,
        experience: u32,
        experience_cap: u32,
    },
    ZeroMaxHealth {
        member_id: String,
    },
    HealthAboveMaximum {
        member_id: String,
        health: u32,
        max_health: u32,
    },
    ManaAboveMaximum {
        member_id: String,
        mana: u32,
        max_mana: u32,
    },
}

impl fmt::Display for RuntimeMemberError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LevelOutsideBalance {
                member_id,
                level,
                level_cap,
            } => write!(
                formatter,
                "member `{member_id}` level {level} is outside 1..={level_cap}"
            ),
            Self::ExperienceAboveBalance {
                member_id,
                experience,
                experience_cap,
            } => write!(
                formatter,
                "member `{member_id}` EXP {experience} exceeds cap {experience_cap}"
            ),
            Self::ZeroMaxHealth { member_id } => {
                write!(
                    formatter,
                    "member `{member_id}` maximum health must be positive"
                )
            }
            Self::HealthAboveMaximum {
                member_id,
                health,
                max_health,
            } => write!(
                formatter,
                "member `{member_id}` health {health} exceeds maximum {max_health}"
            ),
            Self::ManaAboveMaximum {
                member_id,
                mana,
                max_mana,
            } => write!(
                formatter,
                "member `{member_id}` mana {mana} exceeds maximum {max_mana}"
            ),
        }
    }
}

impl Error for RuntimeMemberError {}

/// Empty item ids are not valid equipped values in runtime state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EquipmentMutationError {
    pub slot: EquipmentSlot,
}

impl fmt::Display for EquipmentMutationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "equipment slot `{}` requires a non-empty item id",
            self.slot.as_str()
        )
    }
}

impl Error for EquipmentMutationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        scenario_balance::{PositiveInteger, ProgressionBalance},
        scenario_party::{PartyCatalog, PartyMemberData},
        scenario_yaml,
    };

    type InvalidCatalogCase = (fn(&mut PartyMemberData), RuntimeMemberError);

    fn catalog() -> PartyCatalog {
        scenario_yaml::from_str(include_str!("../tests/fixtures/party-catalog-shapes.yaml"))
            .expect("invented party catalog should deserialize")
    }

    fn progression(level_cap: u32, experience_cap: u32) -> ProgressionBalance {
        ProgressionBalance {
            level_cap: PositiveInteger::new(level_cap).expect("test level cap is positive"),
            exp_cap: PositiveInteger::new(experience_cap).expect("test EXP cap is positive"),
        }
    }

    fn with_data(member: &mut PartyMember, mutate: impl FnOnce(&mut PartyMemberData)) {
        match member {
            PartyMember::Protagonist(data) | PartyMember::Recruit { member: data, .. } => {
                mutate(data);
            }
        }
    }

    #[test]
    fn construction_copies_only_runtime_state_and_keeps_stable_linkage() {
        let source = catalog().party.remove(0);
        let source_before = source.clone();
        let mut runtime = RuntimeMember::try_from_catalog(&source, &progression(100, 1_000_000))
            .expect("valid invented member should construct runtime state");

        assert_eq!(runtime.id(), "ember");
        assert_eq!(runtime.name(), "Ember");
        assert!(runtime.is_protagonist());
        assert_eq!(runtime.class_id(), "vanguard");
        assert_eq!(runtime.level(), 1);
        assert_eq!(runtime.experience(), 0);
        assert_eq!((runtime.health(), runtime.max_health()), (22, 22));
        assert_eq!((runtime.mana(), runtime.max_mana()), (12, 12));
        assert_eq!(runtime.stats().strength(), 28);
        assert_eq!(runtime.stats().dexterity(), 17);
        assert_eq!(runtime.stats().constitution(), 28);
        assert_eq!(runtime.stats().intelligence(), 5);
        assert_eq!(runtime.row(), PartyRow::Front);
        assert_eq!(
            runtime.equipment().get(EquipmentSlot::Weapon),
            Some("iron_blade")
        );
        assert_eq!(runtime.equipment().get(EquipmentSlot::Accessory), None);

        assert_eq!(runtime.apply_damage(5), 5);
        runtime
            .equip(EquipmentSlot::Weapon, "invented_blade")
            .expect("non-empty equipment should be accepted at the value boundary");
        assert_eq!(
            source, source_before,
            "runtime mutation changed catalog data"
        );
    }

    #[test]
    fn constructor_rejects_inconsistent_catalog_pools_and_progression() {
        let cases: [InvalidCatalogCase; 5] = [
            (
                |data| data.level = 0,
                RuntimeMemberError::LevelOutsideBalance {
                    member_id: "ember".to_owned(),
                    level: 0,
                    level_cap: 10,
                },
            ),
            (
                |data| data.exp = 101,
                RuntimeMemberError::ExperienceAboveBalance {
                    member_id: "ember".to_owned(),
                    experience: 101,
                    experience_cap: 100,
                },
            ),
            (
                |data| data.hp_max = 0,
                RuntimeMemberError::ZeroMaxHealth {
                    member_id: "ember".to_owned(),
                },
            ),
            (
                |data| data.hp = data.hp_max + 1,
                RuntimeMemberError::HealthAboveMaximum {
                    member_id: "ember".to_owned(),
                    health: 23,
                    max_health: 22,
                },
            ),
            (
                |data| data.mp = data.mp_max + 1,
                RuntimeMemberError::ManaAboveMaximum {
                    member_id: "ember".to_owned(),
                    mana: 13,
                    max_mana: 12,
                },
            ),
        ];

        for (mutate, expected) in cases {
            let mut member = catalog().party.remove(0);
            with_data(&mut member, mutate);
            assert_eq!(
                RuntimeMember::try_from_catalog(&member, &progression(10, 100)),
                Err(expected)
            );
        }

        let mut above_level_cap = catalog().party.remove(0);
        with_data(&mut above_level_cap, |data| data.level = 11);
        assert!(matches!(
            RuntimeMember::try_from_catalog(&above_level_cap, &progression(10, 100)),
            Err(RuntimeMemberError::LevelOutsideBalance { level: 11, .. })
        ));
    }

    #[test]
    fn health_and_mana_mutations_clamp_and_plain_healing_does_not_revive() {
        let source = catalog().party.remove(0);
        let mut runtime = RuntimeMember::try_from_catalog(&source, &progression(100, 1_000_000))
            .expect("valid invented member should construct runtime state");

        assert_eq!(runtime.apply_damage(7), 7);
        assert_eq!(runtime.health(), 15);
        assert_eq!(runtime.restore_health(u32::MAX), 7);
        assert_eq!(runtime.health(), 22);
        assert_eq!(runtime.apply_damage(u32::MAX), 22);
        assert!(runtime.is_knocked_out());
        assert_eq!(runtime.restore_health(10), 0);
        assert_eq!(runtime.health(), 0);

        assert_eq!(runtime.spend_mana(5), 5);
        assert_eq!(runtime.mana(), 7);
        assert_eq!(runtime.restore_mana(u32::MAX), 5);
        assert_eq!(runtime.mana(), 12);
        assert_eq!(runtime.spend_mana(u32::MAX), 12);
        assert_eq!(runtime.spend_mana(1), 0);
    }

    #[test]
    fn experience_accumulates_to_balance_cap_but_not_at_level_cap() {
        let mut source = catalog().party.remove(0);
        with_data(&mut source, |data| data.exp = 90);
        let balance = progression(10, 100);
        let mut runtime = RuntimeMember::try_from_catalog(&source, &balance)
            .expect("valid invented member should construct runtime state");

        assert_eq!(runtime.add_experience(7, &balance), 7);
        assert_eq!(runtime.add_experience(u32::MAX, &balance), 3);
        assert_eq!(runtime.experience(), 100);
        assert_eq!(runtime.add_experience(1, &balance), 0);

        let mut capped_source = catalog().party.remove(0);
        with_data(&mut capped_source, |data| data.level = 10);
        let mut capped = RuntimeMember::try_from_catalog(&capped_source, &balance)
            .expect("member at level cap is valid");
        assert_eq!(capped.add_experience(50, &balance), 0);
        assert_eq!(capped.experience(), 0);
    }

    #[test]
    fn equipment_mutation_uses_options_and_rejects_empty_item_ids() {
        let source = catalog().party.remove(1);
        let mut runtime = RuntimeMember::try_from_catalog(&source, &progression(100, 1_000_000))
            .expect("valid invented recruit should construct runtime state");

        assert!(!runtime.is_protagonist());
        assert_eq!(runtime.class_id(), "mystic");
        assert_eq!(runtime.equipment().get(EquipmentSlot::Shield), None);
        assert_eq!(
            runtime.equip(EquipmentSlot::Shield, "willow_buckler"),
            Ok(None)
        );
        assert_eq!(
            runtime.equipment().get(EquipmentSlot::Shield),
            Some("willow_buckler")
        );
        assert_eq!(
            runtime.equip(EquipmentSlot::Shield, "silver_buckler"),
            Ok(Some("willow_buckler".to_owned()))
        );
        assert_eq!(
            runtime.unequip(EquipmentSlot::Shield),
            Some("silver_buckler".to_owned())
        );
        assert_eq!(runtime.equipment().get(EquipmentSlot::Shield), None);

        assert_eq!(
            runtime.equip(EquipmentSlot::Shield, ""),
            Err(EquipmentMutationError {
                slot: EquipmentSlot::Shield
            })
        );
        assert_eq!(runtime.equipment().get(EquipmentSlot::Shield), None);
    }
}
