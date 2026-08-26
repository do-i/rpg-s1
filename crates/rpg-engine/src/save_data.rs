//! Versioned native save envelope and validated runtime-state conversion.

use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

use crate::{
    game_state::{GameState, GameStateParts},
    gameplay_rng::GameplayRng,
    playtime::Playtime,
    runtime_flags::RuntimeFlags,
    runtime_map::{RuntimeMapId, RuntimeMapState},
    runtime_member::{EquipmentSlot, RuntimeMember, RuntimeMemberParts},
    runtime_opened_boxes::{OpenedBoxKey, RuntimeOpenedBoxes},
    runtime_party::RuntimeParty,
    runtime_repository::{RuntimeRepository, RuntimeRepositoryItemParts},
    scenario_balance::BalanceData,
    scenario_item::ItemStatus,
    scenario_party::PartyRow,
    scenario_spatial::{CardinalDirection, Position},
};

pub(crate) const NATIVE_SAVE_FORMAT_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct NativeSaveEnvelope {
    pub format_version: u32,
    pub scenario_id: String,
    pub scenario_version: String,
    pub saved_at_unix_seconds: u64,
    pub metadata: SaveMetadata,
    pub payload: SavePayload,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub import_provenance: Option<ImportProvenance>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct SaveMetadata {
    pub protagonist_name: String,
    pub protagonist_level: u32,
    pub location: String,
    pub playtime_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct ImportProvenance {
    pub source_kind: String,
    pub adapter_commit: String,
    pub source_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_timestamp: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_location: Option<String>,
    pub checksum_verified: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct SavePayload {
    pub flags: Vec<String>,
    pub party: Vec<SaveMember>,
    pub repository: SaveRepository,
    pub map: SaveMap,
    pub opened_boxes: Vec<SaveOpenedBox>,
    pub controlled_member_id: String,
    pub rng_state: u64,
    pub playtime_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct SaveMember {
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
    pub stats: SaveStats,
    pub row: PartyRow,
    pub equipment: SaveEquipment,
    pub status_effects: Vec<ItemStatus>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct SaveStats {
    pub strength: u32,
    pub dexterity: u32,
    pub constitution: u32,
    pub intelligence: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct SaveEquipment {
    pub weapon: Option<String>,
    pub shield: Option<String>,
    pub helmet: Option<String>,
    pub body: Option<String>,
    pub accessory: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct SaveRepository {
    pub gp: u32,
    pub items: Vec<SaveRepositoryItem>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct SaveRepositoryItem {
    pub id: String,
    pub quantity: u32,
    pub tags: Vec<String>,
    pub locked: bool,
    pub is_loot: bool,
    pub loot_batch: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct SaveMap {
    pub current: String,
    pub position: Position,
    pub facing: CardinalDirection,
    pub visited: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct SaveOpenedBox {
    pub map_id: String,
    pub box_id: String,
}

impl NativeSaveEnvelope {
    pub(crate) fn from_game_state(
        game: &GameState,
        scenario_id: impl Into<String>,
        scenario_version: impl Into<String>,
        saved_at_unix_seconds: u64,
        location: impl Into<String>,
    ) -> Result<Self, NativeSaveError> {
        let protagonist = game
            .party()
            .protagonist()
            .ok_or_else(|| NativeSaveError::InvalidState("party has no protagonist".to_owned()))?;
        let playtime_seconds = game.playtime().to_seconds();
        Ok(Self {
            format_version: NATIVE_SAVE_FORMAT_VERSION,
            scenario_id: scenario_id.into(),
            scenario_version: scenario_version.into(),
            saved_at_unix_seconds,
            metadata: SaveMetadata {
                protagonist_name: protagonist.name().to_owned(),
                protagonist_level: protagonist.level(),
                location: location.into(),
                playtime_seconds,
            },
            payload: SavePayload::from_game_state(game)?,
            import_provenance: None,
        })
    }

    pub(crate) fn encode(&self) -> Result<Vec<u8>, NativeSaveError> {
        serde_yaml_ng::to_string(self)
            .map(String::into_bytes)
            .map_err(|error| NativeSaveError::Encode(error.to_string()))
    }

    pub(crate) fn decode(
        bytes: &[u8],
        expected_scenario_id: &str,
        expected_scenario_version: &str,
        balance: &BalanceData,
    ) -> Result<(Self, GameState), NativeSaveError> {
        let document = std::str::from_utf8(bytes)
            .map_err(|error| NativeSaveError::Decode(format!("save is not UTF-8: {error}")))?;
        #[derive(Deserialize)]
        struct VersionProbe {
            format_version: Option<u32>,
        }
        let probe: VersionProbe = serde_yaml_ng::from_str(document)
            .map_err(|error| NativeSaveError::Decode(error.to_string()))?;
        if probe.format_version != Some(NATIVE_SAVE_FORMAT_VERSION) {
            return Err(NativeSaveError::UnsupportedVersion(probe.format_version));
        }
        let deserializer = serde_yaml_ng::Deserializer::from_str(document);
        let envelope: Self = serde_path_to_error::deserialize(deserializer)
            .map_err(|error| NativeSaveError::Decode(error.to_string()))?;
        if envelope.scenario_id != expected_scenario_id
            || envelope.scenario_version != expected_scenario_version
        {
            return Err(NativeSaveError::ScenarioMismatch {
                found_id: envelope.scenario_id,
                found_version: envelope.scenario_version,
                expected_id: expected_scenario_id.to_owned(),
                expected_version: expected_scenario_version.to_owned(),
            });
        }
        let game = envelope.payload.clone().into_game_state(balance)?;
        envelope.validate_metadata(&game)?;
        Ok((envelope, game))
    }

    fn validate_metadata(&self, game: &GameState) -> Result<(), NativeSaveError> {
        let protagonist = game
            .party()
            .protagonist()
            .expect("state boundary validated");
        if self.metadata.protagonist_name != protagonist.name()
            || self.metadata.protagonist_level != protagonist.level()
            || self.metadata.playtime_seconds != game.playtime().to_seconds()
        {
            return Err(NativeSaveError::InvalidState(
                "slot metadata does not match its payload".to_owned(),
            ));
        }
        if self.metadata.location.is_empty() {
            return Err(NativeSaveError::InvalidState(
                "slot location metadata must not be empty".to_owned(),
            ));
        }
        Ok(())
    }
}

impl SavePayload {
    fn from_game_state(game: &GameState) -> Result<Self, NativeSaveError> {
        let current = game
            .map()
            .current()
            .ok_or_else(|| NativeSaveError::InvalidState("map has no current id".to_owned()))?;
        Ok(Self {
            flags: game.flags().iter().map(str::to_owned).collect(),
            party: game.party().members().map(SaveMember::from).collect(),
            repository: SaveRepository {
                gp: game.repository().gp(),
                items: game
                    .repository()
                    .item_counts()
                    .map(|(id, quantity)| SaveRepositoryItem {
                        id: id.to_owned(),
                        quantity,
                        tags: game.repository().item_tags(id).map(str::to_owned).collect(),
                        locked: game.repository().is_locked(id),
                        is_loot: game.repository().is_loot(id),
                        loot_batch: game.repository().item_loot_batch(id).unwrap_or(0),
                    })
                    .collect(),
            },
            map: SaveMap {
                current: current.as_str().to_owned(),
                position: game.map().position(),
                facing: game.map().facing(),
                visited: game
                    .map()
                    .visited()
                    .map(|id| id.as_str().to_owned())
                    .collect(),
            },
            opened_boxes: game
                .opened_boxes()
                .iter()
                .map(|key| SaveOpenedBox {
                    map_id: key.map_id().as_str().to_owned(),
                    box_id: key.box_id().to_owned(),
                })
                .collect(),
            controlled_member_id: game.controlled_member_id().to_owned(),
            rng_state: game.rng().state(),
            playtime_seconds: game.playtime().to_seconds(),
        })
    }

    pub(crate) fn into_game_state(
        self,
        balance: &BalanceData,
    ) -> Result<GameState, NativeSaveError> {
        let mut flags = std::collections::BTreeSet::new();
        for flag in self.flags {
            if flag.is_empty() {
                return Err(NativeSaveError::InvalidState(
                    "flag id must not be empty".to_owned(),
                ));
            }
            if !flags.insert(flag.clone()) {
                return Err(NativeSaveError::InvalidState(format!(
                    "flag `{flag}` appears more than once"
                )));
            }
        }
        let party = RuntimeParty::try_from_members(
            self.party
                .into_iter()
                .map(|member| {
                    RuntimeMember::try_from_saved(member.into_parts(), &balance.progression)
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(invalid)?,
        )
        .map_err(invalid)?;
        let repository = RuntimeRepository::try_from_saved(
            &balance.economy,
            self.repository.gp,
            self.repository
                .items
                .into_iter()
                .map(SaveRepositoryItem::into_parts),
        )
        .map_err(invalid)?;
        let current = RuntimeMapId::try_new(self.map.current).map_err(invalid)?;
        let visited = self
            .map
            .visited
            .into_iter()
            .map(RuntimeMapId::try_new)
            .collect::<Result<Vec<_>, _>>()
            .map_err(invalid)?;
        let map =
            RuntimeMapState::try_from_saved(current, self.map.position, self.map.facing, visited)
                .map_err(invalid)?;
        let mut opened_boxes = RuntimeOpenedBoxes::default();
        for saved in self.opened_boxes {
            let key = OpenedBoxKey::try_new(
                RuntimeMapId::try_new(saved.map_id).map_err(invalid)?,
                saved.box_id,
            )
            .map_err(invalid)?;
            if !opened_boxes.record(key) {
                return Err(NativeSaveError::InvalidState(
                    "opened item-box identity appears more than once".to_owned(),
                ));
            }
        }
        GameState::try_from_parts(GameStateParts {
            flags: RuntimeFlags::from_bootstrap(flags),
            party,
            repository,
            map,
            opened_boxes,
            controlled_member_id: self.controlled_member_id,
            rng: GameplayRng::from_state(self.rng_state),
            playtime: Playtime::from_seconds(self.playtime_seconds),
        })
        .map_err(invalid)
    }
}

impl From<&RuntimeMember> for SaveMember {
    fn from(member: &RuntimeMember) -> Self {
        Self {
            id: member.id().to_owned(),
            name: member.name().to_owned(),
            protagonist: member.is_protagonist(),
            class_id: member.class_id().to_owned(),
            level: member.level(),
            experience: member.experience(),
            experience_next: member.experience_next(),
            health: member.health(),
            max_health: member.max_health(),
            mana: member.mana(),
            max_mana: member.max_mana(),
            stats: SaveStats {
                strength: member.stats().strength(),
                dexterity: member.stats().dexterity(),
                constitution: member.stats().constitution(),
                intelligence: member.stats().intelligence(),
            },
            row: member.row(),
            equipment: SaveEquipment {
                weapon: member
                    .equipment()
                    .get(EquipmentSlot::Weapon)
                    .map(str::to_owned),
                shield: member
                    .equipment()
                    .get(EquipmentSlot::Shield)
                    .map(str::to_owned),
                helmet: member
                    .equipment()
                    .get(EquipmentSlot::Helmet)
                    .map(str::to_owned),
                body: member
                    .equipment()
                    .get(EquipmentSlot::Body)
                    .map(str::to_owned),
                accessory: member
                    .equipment()
                    .get(EquipmentSlot::Accessory)
                    .map(str::to_owned),
            },
            status_effects: member.status_effects().collect(),
        }
    }
}

impl SaveMember {
    fn into_parts(self) -> RuntimeMemberParts {
        RuntimeMemberParts {
            id: self.id,
            name: self.name,
            protagonist: self.protagonist,
            class_id: self.class_id,
            level: self.level,
            experience: self.experience,
            experience_next: self.experience_next,
            health: self.health,
            max_health: self.max_health,
            mana: self.mana,
            max_mana: self.max_mana,
            stats: [
                self.stats.strength,
                self.stats.dexterity,
                self.stats.constitution,
                self.stats.intelligence,
            ],
            row: self.row,
            equipment: [
                self.equipment.weapon,
                self.equipment.shield,
                self.equipment.helmet,
                self.equipment.body,
                self.equipment.accessory,
            ],
            status_effects: self.status_effects,
        }
    }
}

impl SaveRepositoryItem {
    fn into_parts(self) -> RuntimeRepositoryItemParts {
        RuntimeRepositoryItemParts {
            id: self.id,
            quantity: self.quantity,
            tags: self.tags,
            locked: self.locked,
            is_loot: self.is_loot,
            loot_batch: self.loot_batch,
        }
    }
}

fn invalid(error: impl fmt::Display) -> NativeSaveError {
    NativeSaveError::InvalidState(error.to_string())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum NativeSaveError {
    Encode(String),
    Decode(String),
    UnsupportedVersion(Option<u32>),
    ScenarioMismatch {
        found_id: String,
        found_version: String,
        expected_id: String,
        expected_version: String,
    },
    InvalidState(String),
}

impl fmt::Display for NativeSaveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Encode(reason) => write!(formatter, "could not encode native save: {reason}"),
            Self::Decode(reason) => write!(formatter, "could not decode native save: {reason}"),
            Self::UnsupportedVersion(Some(version)) => write!(
                formatter,
                "native save format version {version} is unsupported; migration is required"
            ),
            Self::UnsupportedVersion(None) => formatter.write_str(
                "unversioned save is unsupported; use the explicit Python save converter",
            ),
            Self::ScenarioMismatch {
                found_id,
                found_version,
                expected_id,
                expected_version,
            } => write!(
                formatter,
                "save targets scenario {found_id} {found_version}, but this game provides {expected_id} {expected_version}"
            ),
            Self::InvalidState(reason) => {
                write!(formatter, "native save state is invalid: {reason}")
            }
        }
    }
}

impl Error for NativeSaveError {}

#[cfg(test)]
pub(crate) mod tests {
    use std::time::Duration;

    use super::*;
    use crate::{
        new_game::{NewGameScenario, build_new_game_state},
        runtime_opened_boxes::OpenedBoxKey,
        scenario_manifest::Manifest,
        scenario_party::PartyCatalog,
        scenario_quest::{QuestDefinition, QuestKind},
        scenario_yaml,
    };

    pub(crate) fn fixture_balance() -> BalanceData {
        scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/data/balance.yaml"
        ))
        .unwrap()
    }

    pub(crate) fn fixture_game() -> GameState {
        let manifest: Manifest = scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/manifest.yaml"
        ))
        .unwrap();
        let party: PartyCatalog = scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/data/party.yaml"
        ))
        .unwrap();
        let balance = fixture_balance();
        let mut game = build_new_game_state(
            NewGameScenario {
                manifest: &manifest,
                party: &party,
                balance: &balance,
            },
            Duration::ZERO,
        )
        .unwrap();
        game.party_mut().rename_protagonist("Golden Aric").unwrap();
        game.party_mut()
            .member_mut("aric")
            .unwrap()
            .add_status(ItemStatus::Poison);
        game.flags_mut().set("golden_quest_complete");
        let _ = game.repository_mut().add_gp(345).unwrap();
        let batch = game.repository_mut().start_loot_batch();
        let _ = game
            .repository_mut()
            .add_item_in_batch("potion", 3, batch)
            .unwrap();
        game.repository_mut().set_locked("potion", true);
        game.repository_mut().set_hidden("potion", true);
        game.map_mut().move_to(
            RuntimeMapId::try_new("zone_01_starting_forest").unwrap(),
            Position::new(29, 1),
            CardinalDirection::Up,
        );
        game.opened_boxes_mut().record(
            OpenedBoxKey::try_new(RuntimeMapId::try_new("town_01_ardel").unwrap(), "box_01")
                .unwrap(),
        );
        let _ = game.rng_mut().next_u64();
        *game.playtime_mut() = Playtime::from_seconds(3_661);
        game
    }

    pub(crate) fn fixture_envelope() -> NativeSaveEnvelope {
        NativeSaveEnvelope::from_game_state(
            &fixture_game(),
            "my_rpg_story",
            "1.0.0",
            1_700_000_000,
            "Starting Forest",
        )
        .unwrap()
    }

    #[test]
    fn complete_persisted_state_round_trips_and_session_visibility_resets() {
        let envelope = fixture_envelope();
        let encoded = envelope.encode().unwrap();
        let (decoded, restored) =
            NativeSaveEnvelope::decode(&encoded, "my_rpg_story", "1.0.0", &fixture_balance())
                .unwrap();
        assert_eq!(decoded, envelope);
        assert_eq!(
            SavePayload::from_game_state(&restored).unwrap(),
            envelope.payload
        );
        let quest = QuestDefinition {
            id: "golden_quest".to_owned(),
            name: "Golden Quest".to_owned(),
            kind: QuestKind::Sub,
            location: "Ardel".to_owned(),
            description: "Save round trip".to_owned(),
            started_flag: "story_quest_started".to_owned(),
            completed_flag: "golden_quest_complete".to_owned(),
        };
        assert_eq!(
            restored.quest_status(&quest),
            crate::runtime_quest::QuestStatus::Completed
        );
        assert_eq!(restored.rng().state(), envelope.payload.rng_state);
        assert_eq!(restored.playtime().to_seconds(), 3_661);
        assert!(restored.repository().is_locked("potion"));
        assert!(!restored.repository().is_hidden("potion"));
        assert_eq!(restored.repository().item_loot_batch("potion"), Some(1));
        assert!(
            restored
                .opened_boxes()
                .iter()
                .any(|key| key.box_id() == "box_01")
        );
    }

    #[test]
    fn harmless_unknown_fields_are_forward_tolerant() {
        let encoded = String::from_utf8(fixture_envelope().encode().unwrap()).unwrap();
        let extended = encoded
            .replacen(
                "format_version: 1\n",
                "format_version: 1\nfuture_note: harmless\n",
                1,
            )
            .replacen("  flags:\n", "  future_payload_value: 99\n  flags:\n", 1);
        NativeSaveEnvelope::decode(
            extended.as_bytes(),
            "my_rpg_story",
            "1.0.0",
            &fixture_balance(),
        )
        .unwrap();
    }

    #[test]
    fn old_unversioned_and_wrong_scenario_saves_route_without_partial_state() {
        let envelope = fixture_envelope();
        let old = String::from_utf8(envelope.encode().unwrap())
            .unwrap()
            .replacen("format_version: 1", "format_version: 0", 1);
        assert_eq!(
            NativeSaveEnvelope::decode(old.as_bytes(), "my_rpg_story", "1.0.0", &fixture_balance(),),
            Err(NativeSaveError::UnsupportedVersion(Some(0)))
        );
        let unversioned = old.replacen("format_version: 0\n", "", 1);
        assert_eq!(
            NativeSaveEnvelope::decode(
                unversioned.as_bytes(),
                "my_rpg_story",
                "1.0.0",
                &fixture_balance(),
            ),
            Err(NativeSaveError::UnsupportedVersion(None))
        );
        assert!(matches!(
            NativeSaveEnvelope::decode(
                &envelope.encode().unwrap(),
                "another_story",
                "1.0.0",
                &fixture_balance(),
            ),
            Err(NativeSaveError::ScenarioMismatch { .. })
        ));
    }

    #[test]
    fn malformed_duplicate_collections_are_rejected_instead_of_normalized() {
        let encoded = String::from_utf8(fixture_envelope().encode().unwrap()).unwrap();
        for malformed in [
            encoded.replacen(
                "  - golden_quest_complete\n",
                "  - golden_quest_complete\n  - golden_quest_complete\n",
                1,
            ),
            encoded.replacen(
                "      tags: []\n",
                "      tags:\n      - repeated\n      - repeated\n",
                1,
            ),
            encoded.replacen(
                "    status_effects:\n    - poison\n",
                "    status_effects:\n    - poison\n    - poison\n",
                1,
            ),
            encoded.replacen(
                "      tags: []\n",
                "      tags:\n      - one\n      - two\n      - three\n      - four\n      - five\n      - six\n",
                1,
            ),
        ] {
            assert!(matches!(
                NativeSaveEnvelope::decode(
                    malformed.as_bytes(),
                    "my_rpg_story",
                    "1.0.0",
                    &fixture_balance(),
                ),
                Err(NativeSaveError::InvalidState(_))
            ));
        }
    }

    #[test]
    fn native_schema_golden_is_stable() {
        let actual = String::from_utf8(fixture_envelope().encode().unwrap()).unwrap();
        assert_eq!(
            actual,
            include_str!("../../../tests/fixtures/native-save-v1.yaml")
        );
    }
}
