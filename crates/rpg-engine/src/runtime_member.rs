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
//! bounds. Level thresholds/growth and serialization remain later systems.

use std::{collections::BTreeSet, error::Error, fmt};

use crate::{
    scenario_balance::ProgressionBalance,
    scenario_class::ClassDefinition,
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
    experience_next: u32,
    health: u32,
    max_health: u32,
    mana: u32,
    max_mana: u32,
    stats: RuntimeMemberStats,
    row: PartyRow,
    equipment: RuntimeEquipment,
    status_effects: BTreeSet<crate::scenario_item::ItemStatus>,
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
            experience_next: 0,
            health: data.hp,
            max_health: data.hp_max,
            mana: data.mp,
            max_mana: data.mp_max,
            stats: RuntimeMemberStats::from(&data.stats),
            row: data.row,
            equipment: RuntimeEquipment::from(&data.equipped),
            status_effects: BTreeSet::new(),
        })
    }

    /// Restores one native-save member after applying the same numeric invariants as new game.
    pub(crate) fn try_from_saved(
        parts: RuntimeMemberParts,
        progression: &ProgressionBalance,
    ) -> Result<Self, RuntimeMemberError> {
        validate_member_numbers(
            &parts.id,
            parts.level,
            parts.experience,
            parts.health,
            parts.max_health,
            parts.mana,
            parts.max_mana,
            progression,
        )?;
        if parts.id.is_empty() {
            return Err(RuntimeMemberError::EmptyId);
        }
        if parts.name.is_empty() {
            return Err(RuntimeMemberError::EmptyName {
                member_id: parts.id,
            });
        }
        if parts.class_id.is_empty() {
            return Err(RuntimeMemberError::EmptyClassId {
                member_id: parts.id,
            });
        }
        for (slot, item) in EquipmentSlot::ALL.into_iter().zip(parts.equipment.iter()) {
            if item.as_deref() == Some("") {
                return Err(RuntimeMemberError::EmptyEquipmentId {
                    member_id: parts.id,
                    slot,
                });
            }
        }
        let mut status_effects = BTreeSet::new();
        for status in parts.status_effects {
            if !status_effects.insert(status) {
                return Err(RuntimeMemberError::DuplicateStatusEffect {
                    member_id: parts.id,
                    status,
                });
            }
        }
        Ok(Self {
            id: parts.id,
            name: parts.name,
            protagonist: parts.protagonist,
            class_id: parts.class_id,
            level: parts.level,
            experience: parts.experience,
            experience_next: parts.experience_next,
            health: parts.health,
            max_health: parts.max_health,
            mana: parts.mana,
            max_mana: parts.max_mana,
            stats: RuntimeMemberStats {
                strength: parts.stats[0],
                dexterity: parts.stats[1],
                constitution: parts.stats[2],
                intelligence: parts.stats[3],
            },
            row: parts.row,
            equipment: RuntimeEquipment {
                weapon: parts.equipment[0].clone(),
                shield: parts.equipment[1].clone(),
                helmet: parts.equipment[2].clone(),
                body: parts.equipment[3].clone(),
                accessory: parts.equipment[4].clone(),
            },
            status_effects,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Changes only this session member's display name; catalog identity and class linkage stay immutable.
    pub(crate) fn rename(&mut self, name: impl Into<String>) {
        self.name = name.into();
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

    pub fn experience_next(&self) -> u32 {
        self.experience_next
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

    /// Moves this member to a battle row and returns the previous row.
    ///
    /// Runtime party state owns member lookup and exposes the public row-change operation. The
    /// row itself stays here so there is only one mutable source of truth for member position.
    pub(crate) fn set_row(&mut self, row: PartyRow) -> PartyRow {
        std::mem::replace(&mut self.row, row)
    }

    pub fn equipment(&self) -> &RuntimeEquipment {
        &self.equipment
    }

    pub fn is_knocked_out(&self) -> bool {
        self.health == 0
    }

    pub fn status_effects(
        &self,
    ) -> impl ExactSizeIterator<Item = crate::scenario_item::ItemStatus> + '_ {
        self.status_effects.iter().copied()
    }

    pub fn has_status(&self, status: crate::scenario_item::ItemStatus) -> bool {
        self.status_effects.contains(&status)
    }

    pub fn add_status(&mut self, status: crate::scenario_item::ItemStatus) -> bool {
        self.status_effects.insert(status)
    }

    pub fn cure_status(&mut self, status: crate::scenario_item::ItemStatus) -> bool {
        self.status_effects.remove(&status)
    }

    /// Applies the pinned inn's full-party recovery semantics, including knocked-out members.
    pub(crate) fn recover_at_inn(&mut self) {
        self.health = self.max_health;
        self.mana = self.max_mana;
        self.status_effects.clear();
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

    /// Revives a knocked-out member to the source-authored fraction of maximum health.
    pub(crate) fn revive(&mut self, fraction: f64) -> u32 {
        if !self.is_knocked_out() {
            return 0;
        }
        self.health = ((f64::from(self.max_health) * fraction) as u32)
            .max(1)
            .min(self.max_health);
        self.health
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

    pub(crate) fn sync_battle_pools(&mut self, health: u32, mana: u32) {
        self.health = health.min(self.max_health);
        self.mana = mana.min(self.max_mana);
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

    pub(crate) fn apply_experience_progression(
        &mut self,
        amount: u32,
        class: &ClassDefinition,
        progression: &ProgressionBalance,
    ) -> ExperienceProgression {
        let added = self.add_experience(amount, progression);
        let mut level_ups = Vec::new();
        while self.level < progression.level_cap.get()
            && self.experience >= experience_required(class, self.level.saturating_add(1))
        {
            let old_level = self.level;
            self.level += 1;
            let growth_index =
                (self.level.saturating_sub(1) as usize) % class.stat_growth.strength.len();
            let strength = class.stat_growth.strength[growth_index];
            let dexterity = class.stat_growth.dex[growth_index];
            let constitution = class.stat_growth.con[growth_index];
            let intelligence = class.stat_growth.intelligence[growth_index];
            self.stats.strength = self.stats.strength.saturating_add(strength);
            self.stats.dexterity = self.stats.dexterity.saturating_add(dexterity);
            self.stats.constitution = self.stats.constitution.saturating_add(constitution);
            self.stats.intelligence = self.stats.intelligence.saturating_add(intelligence);
            let health = self.stats.constitution.saturating_add(6);
            let mana = self.stats.intelligence.saturating_add(6);
            self.max_health = self.max_health.saturating_add(health);
            self.max_mana = self.max_mana.saturating_add(mana);
            self.health = self.max_health;
            self.mana = self.max_mana;
            level_ups.push(RuntimeLevelUp {
                old_level,
                new_level: self.level,
                health,
                mana,
                strength,
                dexterity,
                constitution,
                intelligence,
                max_health: self.max_health,
                max_mana: self.max_mana,
                total_strength: self.stats.strength,
                total_dexterity: self.stats.dexterity,
                total_constitution: self.stats.constitution,
                total_intelligence: self.stats.intelligence,
            });
        }
        self.experience_next = if self.level >= progression.level_cap.get() {
            0
        } else {
            experience_required(class, self.level.saturating_add(1))
        };
        ExperienceProgression { added, level_ups }
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

pub(crate) fn experience_required(class: &ClassDefinition, level: u32) -> u32 {
    (f64::from(class.exp_base.get()) * f64::from(level).powf(class.exp_factor.get())) as u32
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ExperienceProgression {
    pub(crate) added: u32,
    pub(crate) level_ups: Vec<RuntimeLevelUp>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RuntimeLevelUp {
    pub(crate) old_level: u32,
    pub(crate) new_level: u32,
    pub(crate) health: u32,
    pub(crate) mana: u32,
    pub(crate) strength: u32,
    pub(crate) dexterity: u32,
    pub(crate) constitution: u32,
    pub(crate) intelligence: u32,
    pub(crate) max_health: u32,
    pub(crate) max_mana: u32,
    pub(crate) total_strength: u32,
    pub(crate) total_dexterity: u32,
    pub(crate) total_constitution: u32,
    pub(crate) total_intelligence: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RuntimeMemberParts {
    pub id: String,
    pub name: String,
    pub protagonist: bool,
    pub class_id: String,
    pub level: u32,
    pub experience: u32,
    pub experience_next: u32,
    pub health: u32,
    pub max_health: u32,
    pub mana: u32,
    pub max_mana: u32,
    pub stats: [u32; 4],
    pub row: PartyRow,
    pub equipment: [Option<String>; 5],
    pub status_effects: Vec<crate::scenario_item::ItemStatus>,
}

#[expect(
    clippy::too_many_arguments,
    reason = "the validation boundary checks one complete persisted member numeric record"
)]
fn validate_member_numbers(
    member_id: &str,
    level: u32,
    experience: u32,
    health: u32,
    max_health: u32,
    mana: u32,
    max_mana: u32,
    progression: &ProgressionBalance,
) -> Result<(), RuntimeMemberError> {
    let level_cap = progression.level_cap.get();
    let experience_cap = progression.exp_cap.get();
    if level == 0 || level > level_cap {
        return Err(RuntimeMemberError::LevelOutsideBalance {
            member_id: member_id.to_owned(),
            level,
            level_cap,
        });
    }
    if experience > experience_cap {
        return Err(RuntimeMemberError::ExperienceAboveBalance {
            member_id: member_id.to_owned(),
            experience,
            experience_cap,
        });
    }
    if max_health == 0 {
        return Err(RuntimeMemberError::ZeroMaxHealth {
            member_id: member_id.to_owned(),
        });
    }
    if health > max_health {
        return Err(RuntimeMemberError::HealthAboveMaximum {
            member_id: member_id.to_owned(),
            health,
            max_health,
        });
    }
    if mana > max_mana {
        return Err(RuntimeMemberError::ManaAboveMaximum {
            member_id: member_id.to_owned(),
            mana,
            max_mana,
        });
    }
    Ok(())
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
    pub const ALL: [Self; 5] = [
        Self::Weapon,
        Self::Shield,
        Self::Helmet,
        Self::Body,
        Self::Accessory,
    ];

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
    EmptyId,
    EmptyName {
        member_id: String,
    },
    EmptyClassId {
        member_id: String,
    },
    EmptyEquipmentId {
        member_id: String,
        slot: EquipmentSlot,
    },
    DuplicateStatusEffect {
        member_id: String,
        status: crate::scenario_item::ItemStatus,
    },
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
            Self::EmptyId => formatter.write_str("member id must not be empty"),
            Self::EmptyName { member_id } => {
                write!(formatter, "member `{member_id}` name must not be empty")
            }
            Self::EmptyClassId { member_id } => {
                write!(formatter, "member `{member_id}` class id must not be empty")
            }
            Self::EmptyEquipmentId { member_id, slot } => write!(
                formatter,
                "member `{member_id}` equipment slot `{}` has an empty item id",
                slot.as_str()
            ),
            Self::DuplicateStatusEffect { member_id, status } => write!(
                formatter,
                "member `{member_id}` contains duplicate status effect `{status:?}`"
            ),
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
        scenario_yaml::from_str(include_str!(
            "../../../tests/fixtures/party-catalog-shapes.yaml"
        ))
        .expect("invented party catalog should deserialize")
    }

    fn progression(level_cap: u32, experience_cap: u32) -> ProgressionBalance {
        ProgressionBalance {
            level_cap: PositiveInteger::new(level_cap).expect("test level cap is positive"),
            exp_cap: PositiveInteger::new(experience_cap).expect("test EXP cap is positive"),
        }
    }

    fn hero_class() -> ClassDefinition {
        scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/data/classes/hero.yaml"
        ))
        .unwrap()
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
    fn class_threshold_and_one_level_growth_match_source_formulas() {
        let class = hero_class();
        assert_eq!(experience_required(&class, 2), 400);
        assert_eq!(experience_required(&class, 3), 900);
        assert_eq!(experience_required(&class, 10), 10_000);

        let source: PartyCatalog = scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/data/party.yaml"
        ))
        .unwrap();
        let mut member =
            RuntimeMember::try_from_catalog(&source.party[0], &progression(100, 1_000_000))
                .unwrap();
        member.apply_damage(10);
        member.spend_mana(10);
        let result = member.apply_experience_progression(400, &class, &progression(100, 1_000_000));

        assert_eq!(result.added, 400);
        assert_eq!(result.level_ups.len(), 1);
        let level = &result.level_ups[0];
        assert_eq!((level.old_level, level.new_level), (1, 2));
        assert_eq!(
            (
                level.strength,
                level.dexterity,
                level.constitution,
                level.intelligence,
            ),
            (2, 2, 3, 1)
        );
        assert_eq!((level.health, level.mana), (37, 12));
        assert_eq!((member.max_health(), member.max_mana()), (59, 24));
        assert_eq!((member.health(), member.mana()), (59, 24));
        assert_eq!(member.experience_next(), 900);
    }

    #[test]
    fn every_production_class_threshold_curve_matches_the_source_values() {
        let fixtures = [
            (
                include_str!("../../../assets/scenarios/rusted_kingdoms/data/classes/cleric.yaml"),
                [380, 855, 9_500],
            ),
            (
                include_str!("../../../assets/scenarios/rusted_kingdoms/data/classes/hero.yaml"),
                [400, 900, 10_000],
            ),
            (
                include_str!("../../../assets/scenarios/rusted_kingdoms/data/classes/rogue.yaml"),
                [360, 810, 9_000],
            ),
            (
                include_str!(
                    "../../../assets/scenarios/rusted_kingdoms/data/classes/sorcerer.yaml"
                ),
                [380, 855, 9_500],
            ),
            (
                include_str!("../../../assets/scenarios/rusted_kingdoms/data/classes/warrior.yaml"),
                [440, 990, 11_000],
            ),
        ];
        for (document, expected) in fixtures {
            let class: ClassDefinition = scenario_yaml::from_str(document).unwrap();
            assert_eq!(
                [
                    experience_required(&class, 2),
                    experience_required(&class, 3),
                    experience_required(&class, 10),
                ],
                expected,
                "{} threshold curve",
                class.class_id
            );
        }
    }

    #[test]
    fn one_experience_award_applies_every_crossed_level_once() {
        let class = hero_class();
        let source: PartyCatalog = scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/data/party.yaml"
        ))
        .unwrap();
        let mut member =
            RuntimeMember::try_from_catalog(&source.party[0], &progression(100, 1_000_000))
                .unwrap();

        let result =
            member.apply_experience_progression(1_600, &class, &progression(100, 1_000_000));

        assert_eq!(member.level(), 4);
        assert_eq!(
            result
                .level_ups
                .iter()
                .map(|growth| (growth.old_level, growth.new_level))
                .collect::<Vec<_>>(),
            [(1, 2), (2, 3), (3, 4)]
        );
        assert_eq!(member.experience_next(), 2_500);
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
