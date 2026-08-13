//! Source-authored party catalog schema.
//!
//! The pinned `0897035` `data/party.yaml` contains one protagonist record and four recruit
//! records. Every member has the same required identity, join, initial-stat, equipment, and
//! ability fields. The protagonist omits `recruit`; every non-protagonist requires it. That
//! relationship is represented by [`PartyMember`] instead of retaining a boolean plus an
//! independently optional block that could describe an impossible source record.
//!
//! The `join.condition` field is one required flag identifier, not the shared
//! [`crate::scenario_condition::FlagConditions`] `requires`/`excludes` mapping. Portraits are
//! scenario-relative files, while class, map, item, ability, dialogue, NPC, and flag values are
//! catalog identifiers whose referential validation belongs to M2.25.

use bevy::{asset::Asset, reflect::TypePath};

use crate::scenario_path::ScenarioRelativePath;
use crate::scenario_spatial::Position;
use crate::scenario_yaml::{deserialize_string, deserialize_strings};
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

/// The mapping-root party catalog from `data/party.yaml`.
#[derive(Asset, Clone, Debug, Deserialize, Eq, PartialEq, TypePath)]
#[serde(deny_unknown_fields)]
pub struct PartyCatalog {
    /// Party definitions in authored order.
    pub party: Vec<PartyMember>,
}

/// One source-authored party definition, classified by its recruitment shape.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PartyMember {
    /// The player-named initial member. Source data has `protagonist: true` and no `recruit`.
    Protagonist(PartyMemberData),
    /// A story recruit. Source data has `protagonist: false` and a required `recruit` block.
    Recruit {
        member: PartyMemberData,
        recruit: PartyRecruit,
    },
}

impl PartyMember {
    /// Returns the fields shared by protagonist and recruit records.
    pub fn data(&self) -> &PartyMemberData {
        match self {
            Self::Protagonist(member) | Self::Recruit { member, .. } => member,
        }
    }

    /// Returns the recruit linkage for a recruit, or `None` for the protagonist.
    pub fn recruit(&self) -> Option<&PartyRecruit> {
        match self {
            Self::Protagonist(_) => None,
            Self::Recruit { recruit, .. } => Some(recruit),
        }
    }

    /// Returns whether this is the source protagonist variant.
    pub fn is_protagonist(&self) -> bool {
        matches!(self, Self::Protagonist(_))
    }
}

/// Fields required on both source party-member variants.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PartyMemberData {
    pub id: String,
    pub name: String,
    pub class_id: String,
    pub portrait: ScenarioRelativePath,
    pub join: PartyJoin,
    pub row: PartyRow,
    /// Cosmetic source age. Non-negative source integers are retained without a gameplay cap.
    pub age: u32,
    pub gender: PartyMemberGender,
    pub level: u32,
    pub exp: u32,
    pub hp: u32,
    pub hp_max: u32,
    pub mp: u32,
    pub mp_max: u32,
    pub stats: PartyStats,
    pub equipped: PartyEquipment,
    /// Ability identifiers in authored order.
    pub abilities_unlocked: Vec<String>,
    /// Initial status-effect identifiers in authored order.
    pub status_effects: Vec<String>,
}

/// The source flag and tile location at which a party member becomes available.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PartyJoin {
    /// One required story-flag identifier.
    #[serde(deserialize_with = "deserialize_string")]
    pub condition: String,
    /// A map identifier, not a scenario-relative file path.
    #[serde(deserialize_with = "deserialize_string")]
    pub map: String,
    pub position: Position,
}

/// Source references that connect a recruit to their NPC dialogue and joined flag.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PartyRecruit {
    #[serde(deserialize_with = "deserialize_string")]
    pub npc: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub dialogue: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub joined_flag: String,
}

/// A party member's initial battle row.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PartyRow {
    Front,
    Back,
}

/// Cosmetic gender values exercised by the pinned party corpus.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PartyMemberGender {
    Male,
    Female,
}

/// The four required initial base stats.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PartyStats {
    #[serde(rename = "str")]
    pub strength: u32,
    pub dex: u32,
    pub con: u32,
    #[serde(rename = "int")]
    pub intelligence: u32,
}

/// The five fixed initial equipment slots.
///
/// An empty source string means unequipped and remains an empty string; it is intentionally not
/// coerced to `None`, so source values round-trip into later catalog validation without loss.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PartyEquipment {
    #[serde(deserialize_with = "deserialize_string")]
    pub weapon: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub shield: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub helmet: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub body: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub accessory: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PartyMemberDocument {
    #[serde(deserialize_with = "deserialize_string")]
    id: String,
    #[serde(deserialize_with = "deserialize_string")]
    name: String,
    #[serde(rename = "class", deserialize_with = "deserialize_string")]
    class_id: String,
    protagonist: bool,
    portrait: ScenarioRelativePath,
    join: PartyJoin,
    #[serde(default)]
    recruit: OptionalField<PartyRecruit>,
    row: PartyRow,
    age: u32,
    gender: PartyMemberGender,
    level: u32,
    exp: u32,
    hp: u32,
    hp_max: u32,
    mp: u32,
    mp_max: u32,
    stats: PartyStats,
    equipped: PartyEquipment,
    #[serde(deserialize_with = "deserialize_strings")]
    abilities_unlocked: Vec<String>,
    #[serde(deserialize_with = "deserialize_strings")]
    status_effects: Vec<String>,
}

impl<'de> Deserialize<'de> for PartyMember {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let document = PartyMemberDocument::deserialize(deserializer)?;
        let PartyMemberDocument {
            id,
            name,
            class_id,
            protagonist,
            portrait,
            join,
            recruit,
            row,
            age,
            gender,
            level,
            exp,
            hp,
            hp_max,
            mp,
            mp_max,
            stats,
            equipped,
            abilities_unlocked,
            status_effects,
        } = document;
        let member = PartyMemberData {
            id,
            name,
            class_id,
            portrait,
            join,
            row,
            age,
            gender,
            level,
            exp,
            hp,
            hp_max,
            mp,
            mp_max,
            stats,
            equipped,
            abilities_unlocked,
            status_effects,
        };

        match (protagonist, recruit) {
            (true, OptionalField::Missing) => Ok(Self::Protagonist(member)),
            (false, OptionalField::Present(recruit)) => Ok(Self::Recruit { member, recruit }),
            (true, OptionalField::Present(_)) => Err(D::Error::custom(
                "protagonist party member must not define `recruit`",
            )),
            (false, OptionalField::Missing) => Err(D::Error::custom(
                "non-protagonist party member must define `recruit`",
            )),
        }
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

#[cfg(test)]
mod tests {
    use super::{PartyCatalog, PartyMember, PartyMemberGender, PartyRow};
    use crate::scenario_yaml;

    #[test]
    fn loads_every_party_record_shape_and_field_without_loss() {
        let catalog: PartyCatalog =
            scenario_yaml::from_str(include_str!("../tests/fixtures/party-catalog-shapes.yaml"))
                .expect("source-shaped party fixture should deserialize");

        assert_eq!(catalog.party.len(), 2);

        let protagonist = &catalog.party[0];
        assert!(protagonist.is_protagonist());
        assert_eq!(protagonist.data().id, "ember");
        assert_eq!(protagonist.data().name, "Ember");
        assert_eq!(protagonist.data().class_id, "vanguard");
        assert_eq!(
            protagonist.data().portrait.as_str(),
            "assets/images/ember-profile.webp"
        );
        assert_eq!(protagonist.data().join.condition, "story_started");
        assert_eq!(protagonist.data().join.map, "harbor_town");
        assert_eq!(
            (
                protagonist.data().join.position.x,
                protagonist.data().join.position.y
            ),
            (12, 8)
        );
        assert_eq!(protagonist.data().row, PartyRow::Front);
        assert_eq!(protagonist.data().age, 19);
        assert_eq!(protagonist.data().gender, PartyMemberGender::Male);
        assert_eq!(protagonist.data().level, 1);
        assert_eq!(protagonist.data().exp, 0);
        assert_eq!((protagonist.data().hp, protagonist.data().hp_max), (22, 22));
        assert_eq!((protagonist.data().mp, protagonist.data().mp_max), (12, 12));
        assert_eq!(protagonist.data().stats.strength, 28);
        assert_eq!(protagonist.data().stats.dex, 17);
        assert_eq!(protagonist.data().stats.con, 28);
        assert_eq!(protagonist.data().stats.intelligence, 5);
        assert_eq!(protagonist.data().equipped.weapon, "iron_blade");
        assert_eq!(protagonist.data().equipped.shield, "round_shield");
        assert_eq!(protagonist.data().equipped.helmet, "leather_cap");
        assert_eq!(protagonist.data().equipped.body, "leather_coat");
        assert_eq!(protagonist.data().equipped.accessory, "");
        assert!(protagonist.data().abilities_unlocked.is_empty());
        assert!(protagonist.data().status_effects.is_empty());
        assert!(protagonist.recruit().is_none());

        let recruit = &catalog.party[1];
        assert!(matches!(recruit, PartyMember::Recruit { .. }));
        assert!(!recruit.is_protagonist());
        assert_eq!(recruit.data().row, PartyRow::Back);
        assert_eq!(recruit.data().abilities_unlocked, ["mend", "ward"]);
        assert_eq!(recruit.data().status_effects, ["blessed"]);
        assert_eq!(recruit.data().equipped.accessory, "silver_charm");
        let linkage = recruit.recruit().expect("recruit block should be retained");
        assert_eq!(linkage.npc, "mira");
        assert_eq!(linkage.dialogue, "mira_join");
        assert_eq!(linkage.joined_flag, "npc_mira_joined");
    }

    #[test]
    fn requires_exact_protagonist_and_recruit_shapes() {
        for document in [
            fixture_with("protagonist: false\n", ""),
            fixture_with(
                "protagonist: true\n",
                "    recruit: { npc: mira, dialogue: mira_join, joined_flag: npc_mira_joined }\n",
            ),
            fixture_with("protagonist: false\n", "    recruit: null\n"),
        ] {
            assert!(scenario_yaml::from_str::<PartyCatalog>(&document).is_err());
        }
    }

    #[test]
    fn rejects_unknown_missing_and_wrong_typed_party_fields() {
        for document in [
            "party: {}\n".to_owned(),
            fixture_with("protagonist: true\n    mystery: value\n", ""),
            fixture_with("protagonist: true\n", "").replace("id: ember", "id: 42"),
            fixture_with("protagonist: true\n", "").replace("name: Ember", "name: true"),
            fixture_with("protagonist: true\n", "").replace("class: vanguard", "class: false"),
            fixture_with("protagonist: true\n", "").replace("    row: front\n", ""),
            fixture_with("protagonist: true\n", "").replace("    abilities_unlocked: []\n", ""),
            fixture_with("protagonist: true\n", "").replace("    level: 1\n", "    level: -1\n"),
            fixture_with("protagonist: true\n", "").replace(
                "stats: { str: 28, dex: 17, con: 28, int: 5 }",
                "stats: { str: 28, dex: 17, con: 28, int: 5, luck: 4 }",
            ),
            fixture_with("protagonist: true\n", "")
                .replace("accessory: '' }", "accessory: '', boots: sandals }"),
            fixture_with("protagonist: true\n", "").replace("weapon: iron_blade", "weapon: 42"),
            fixture_with("protagonist: true\n", "").replace("accessory: ''", "accessory: true"),
            fixture_with("protagonist: true\n", "")
                .replace("condition: story_started", "condition: [story_started]"),
            fixture_with("protagonist: true\n", "")
                .replace("condition: story_started", "condition: 42"),
            fixture_with("protagonist: true\n", "").replace("map: harbor_town", "map: false"),
            fixture_with("protagonist: true\n", "").replace(
                "    portrait: assets/portrait.webp",
                "    portrait: ../../portrait.webp",
            ),
            fixture_with("protagonist: true\n", "")
                .replace("portrait: assets/portrait.webp", "portrait: 42"),
            fixture_with("protagonist: true\n", "")
                .replace("portrait: assets/portrait.webp", "portrait: false"),
            fixture_with("protagonist: true\n", "").replace("    row: front", "    row: middle"),
            fixture_with("protagonist: true\n", "")
                .replace("    gender: female", "    gender: unknown"),
            fixture_with("protagonist: true\n", "")
                .replace("abilities_unlocked: []", "abilities_unlocked: [42]"),
            fixture_with("protagonist: true\n", "").replace(
                "    status_effects: []",
                "    status_effects: [{ effect: poison }]",
            ),
            fixture_with("protagonist: true\n", "")
                .replace("status_effects: []", "status_effects: [false]"),
            recruit_fixture().replace("npc: mira", "npc: 42"),
            recruit_fixture().replace("dialogue: mira_join", "dialogue: false"),
            recruit_fixture().replace("joined_flag: npc_mira_joined", "joined_flag: 7"),
        ] {
            assert!(scenario_yaml::from_str::<PartyCatalog>(&document).is_err());
        }
    }

    fn fixture_with(protagonist_line: &str, recruit_line: &str) -> String {
        format!(
            "party:\n  - id: ember\n    name: Ember\n    class: vanguard\n    {protagonist_line}    portrait: assets/portrait.webp\n    join: {{ condition: story_started, map: harbor_town, position: [1, 2] }}\n{recruit_line}    row: front\n    age: 19\n    gender: female\n    level: 1\n    exp: 0\n    hp: 22\n    hp_max: 22\n    mp: 12\n    mp_max: 12\n    stats: {{ str: 28, dex: 17, con: 28, int: 5 }}\n    equipped: {{ weapon: iron_blade, shield: '', helmet: '', body: leather_coat, accessory: '' }}\n    abilities_unlocked: []\n    status_effects: []\n"
        )
    }

    fn recruit_fixture() -> String {
        fixture_with(
            "protagonist: false\n",
            "    recruit: { npc: mira, dialogue: mira_join, joined_flag: npc_mira_joined }\n",
        )
    }
}
