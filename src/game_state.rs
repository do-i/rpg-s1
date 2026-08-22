//! Validated root ownership for one active game session.
//!
//! This resource owns every currently defined mutable session value: flags, party, repository,
//! world location, opened boxes, controlled-member selection, deterministic gameplay RNG, and
//! playtime. Quest status remains a pure view of the owned flags and immutable quest definitions,
//! so it is deliberately not stored here.
//!
//! M3.08 defines ownership and construction invariants only. M3.09 builds this root from typed
//! scenario data, and M3.13 owns the confirmed-name handoff that can atomically install it into
//! the Bevy world and retire transitional standalone resources. M7 owns serialization.

use std::{error::Error, fmt};

use bevy::prelude::Resource;

use crate::{
    gameplay_rng::GameplayRng,
    playtime::Playtime,
    runtime_flags::RuntimeFlags,
    runtime_map::RuntimeMapState,
    runtime_opened_boxes::RuntimeOpenedBoxes,
    runtime_party::RuntimeParty,
    runtime_quest::{QuestStatus, quest_status},
    runtime_repository::RuntimeRepository,
    scenario_quest::QuestDefinition,
};

/// Complete child state supplied to the validated root boundary.
#[derive(Debug)]
pub struct GameStateParts {
    pub flags: RuntimeFlags,
    pub party: RuntimeParty,
    pub repository: RuntimeRepository,
    pub map: RuntimeMapState,
    pub opened_boxes: RuntimeOpenedBoxes,
    pub controlled_member_id: String,
    pub rng: GameplayRng,
    pub playtime: Playtime,
}

/// The sole owner of mutable state for one initialized game session.
#[derive(Clone, Debug, Eq, PartialEq, Resource)]
pub struct GameState {
    flags: RuntimeFlags,
    party: RuntimeParty,
    repository: RuntimeRepository,
    map: RuntimeMapState,
    opened_boxes: RuntimeOpenedBoxes,
    controlled_member_id: String,
    rng: GameplayRng,
    playtime: Playtime,
}

impl GameState {
    /// Composes a complete session without performing catalog or new-game loading.
    pub fn try_from_parts(parts: GameStateParts) -> Result<Self, GameStateError> {
        if parts.party.protagonist().is_none() {
            return Err(GameStateError::MissingProtagonist);
        }
        if parts.map.current().is_none() {
            return Err(GameStateError::MissingCurrentMap);
        }
        validate_controlled_member(&parts.party, &parts.controlled_member_id)?;

        Ok(Self {
            flags: parts.flags,
            party: parts.party,
            repository: parts.repository,
            map: parts.map,
            opened_boxes: parts.opened_boxes,
            controlled_member_id: parts.controlled_member_id,
            rng: parts.rng,
            playtime: parts.playtime,
        })
    }

    pub fn flags(&self) -> &RuntimeFlags {
        &self.flags
    }

    pub fn flags_mut(&mut self) -> &mut RuntimeFlags {
        &mut self.flags
    }

    pub fn party(&self) -> &RuntimeParty {
        &self.party
    }

    pub fn party_mut(&mut self) -> &mut RuntimeParty {
        &mut self.party
    }

    pub fn repository(&self) -> &RuntimeRepository {
        &self.repository
    }

    pub fn repository_mut(&mut self) -> &mut RuntimeRepository {
        &mut self.repository
    }

    pub(crate) fn repository_and_party_mut(
        &mut self,
    ) -> (&mut RuntimeRepository, &mut RuntimeParty) {
        (&mut self.repository, &mut self.party)
    }

    pub fn map(&self) -> &RuntimeMapState {
        &self.map
    }

    pub fn map_mut(&mut self) -> &mut RuntimeMapState {
        &mut self.map
    }

    pub fn opened_boxes(&self) -> &RuntimeOpenedBoxes {
        &self.opened_boxes
    }

    pub fn opened_boxes_mut(&mut self) -> &mut RuntimeOpenedBoxes {
        &mut self.opened_boxes
    }

    pub fn controlled_member_id(&self) -> &str {
        &self.controlled_member_id
    }

    /// Selects an owned party member without changing the selection on failure.
    pub fn set_controlled_member(
        &mut self,
        member_id: impl Into<String>,
    ) -> Result<(), GameStateError> {
        let member_id = member_id.into();
        validate_controlled_member(&self.party, &member_id)?;
        self.controlled_member_id = member_id;
        Ok(())
    }

    pub fn rng(&self) -> &GameplayRng {
        &self.rng
    }

    pub fn rng_mut(&mut self) -> &mut GameplayRng {
        &mut self.rng
    }

    pub fn playtime(&self) -> &Playtime {
        &self.playtime
    }

    pub fn playtime_mut(&mut self) -> &mut Playtime {
        &mut self.playtime
    }

    /// Derives quest lifecycle from this session's flags without storing duplicate state.
    pub fn quest_status(&self, quest: &QuestDefinition) -> QuestStatus {
        quest_status(quest, &self.flags)
    }
}

fn validate_controlled_member(party: &RuntimeParty, member_id: &str) -> Result<(), GameStateError> {
    if member_id.is_empty() {
        return Err(GameStateError::EmptyControlledMemberId);
    }
    if !party.contains(member_id) {
        return Err(GameStateError::ControlledMemberNotInParty {
            member_id: member_id.to_owned(),
        });
    }
    Ok(())
}

/// Invalid composition or controlled-member selection at the root boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GameStateError {
    MissingProtagonist,
    MissingCurrentMap,
    EmptyControlledMemberId,
    ControlledMemberNotInParty { member_id: String },
}

impl fmt::Display for GameStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingProtagonist => formatter.write_str("game state requires one protagonist"),
            Self::MissingCurrentMap => formatter.write_str("game state requires a current map"),
            Self::EmptyControlledMemberId => {
                formatter.write_str("controlled member id must not be empty")
            }
            Self::ControlledMemberNotInParty { member_id } => write!(
                formatter,
                "controlled member `{member_id}` is not in the active party"
            ),
        }
    }
}

impl Error for GameStateError {}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::{
        runtime_map::RuntimeMapId,
        runtime_member::RuntimeMember,
        runtime_opened_boxes::OpenedBoxKey,
        scenario_balance::BalanceData,
        scenario_party::{PartyCatalog, PartyMember},
        scenario_quest::QuestKind,
        scenario_spatial::{CardinalDirection, Position},
        scenario_yaml,
    };

    fn catalog() -> PartyCatalog {
        scenario_yaml::from_str(include_str!("../tests/fixtures/party-catalog-shapes.yaml"))
            .expect("invented party catalog should deserialize")
    }

    fn balance() -> BalanceData {
        scenario_yaml::from_str(include_str!("../tests/fixtures/balance-complete.yaml"))
            .expect("invented balance should deserialize")
    }

    fn runtime(member: &PartyMember, balance: &BalanceData) -> RuntimeMember {
        RuntimeMember::try_from_catalog(member, &balance.progression)
            .expect("invented member should construct runtime state")
    }

    fn map_id(id: &str) -> RuntimeMapId {
        RuntimeMapId::try_new(id).expect("test map id should be nonempty")
    }

    fn valid_parts() -> GameStateParts {
        let catalog = catalog();
        let balance = balance();
        let party = RuntimeParty::try_from_members(
            catalog.party.iter().map(|member| runtime(member, &balance)),
        )
        .expect("invented party should be complete");
        let mut repository = RuntimeRepository::from_balance(&balance.economy);
        assert_eq!(repository.add_gp(250).expect("GP should add").added(), 250);
        assert_eq!(
            repository
                .add_item("invented_potion", 3)
                .expect("item should add")
                .added(),
            3
        );
        let mut opened_boxes = RuntimeOpenedBoxes::default();
        assert!(
            opened_boxes.record(
                OpenedBoxKey::try_new(map_id("invented_town"), "box_01")
                    .expect("box key should be valid")
            )
        );
        let mut playtime = Playtime::from_seconds(90);
        playtime.start_session(Duration::from_secs(10));
        playtime.commit_session(Duration::from_secs(20));

        GameStateParts {
            flags: RuntimeFlags::from_bootstrap(["quest_started", "story_ready"]),
            party,
            repository,
            map: RuntimeMapState::new(
                map_id("invented_town"),
                Position::new(7, 9),
                CardinalDirection::Right,
            ),
            opened_boxes,
            controlled_member_id: "ember".to_owned(),
            rng: GameplayRng::from_seed(42),
            playtime,
        }
    }

    fn quest() -> QuestDefinition {
        QuestDefinition {
            id: "invented_quest".to_owned(),
            name: "An Invented Quest".to_owned(),
            kind: QuestKind::Sub,
            location: "Invented Town".to_owned(),
            description: "Test root-owned flags.".to_owned(),
            started_flag: "quest_started".to_owned(),
            completed_flag: "quest_done".to_owned(),
        }
    }

    #[test]
    fn root_owns_every_session_component_and_derives_quest_status() {
        let mut state = GameState::try_from_parts(valid_parts()).expect("parts should be complete");

        assert!(state.flags().is_set("story_ready"));
        assert_eq!(state.party().len(), 2);
        assert_eq!(state.repository().gp(), 250);
        assert_eq!(state.repository().item_count("invented_potion"), 3);
        assert_eq!(
            state.map().current().map(RuntimeMapId::as_str),
            Some("invented_town")
        );
        assert_eq!(state.map().position(), Position::new(7, 9));
        assert_eq!(state.map().facing(), CardinalDirection::Right);
        let box_key = OpenedBoxKey::try_new(map_id("invented_town"), "box_01").unwrap();
        assert!(state.opened_boxes().contains(&box_key));
        assert_eq!(state.controlled_member_id(), "ember");
        assert_eq!(state.playtime().total_seconds(), 100);
        assert_eq!(state.quest_status(&quest()), QuestStatus::Active);

        let mut expected_rng = GameplayRng::from_seed(42);
        assert_eq!(state.rng_mut().next_u64(), expected_rng.next_u64());
        assert!(state.flags_mut().set("quest_done"));
        assert_eq!(state.quest_status(&quest()), QuestStatus::Completed);
    }

    #[test]
    fn composition_rejects_incomplete_party_map_and_controlled_member() {
        let mut no_protagonist = valid_parts();
        no_protagonist.party = RuntimeParty::default();
        assert!(matches!(
            GameState::try_from_parts(no_protagonist),
            Err(GameStateError::MissingProtagonist)
        ));

        let mut no_map = valid_parts();
        no_map.map = RuntimeMapState::default();
        assert!(matches!(
            GameState::try_from_parts(no_map),
            Err(GameStateError::MissingCurrentMap)
        ));

        let mut empty_controlled = valid_parts();
        empty_controlled.controlled_member_id.clear();
        assert!(matches!(
            GameState::try_from_parts(empty_controlled),
            Err(GameStateError::EmptyControlledMemberId)
        ));

        let mut unknown_controlled = valid_parts();
        unknown_controlled.controlled_member_id = "outsider".to_owned();
        assert!(matches!(
            GameState::try_from_parts(unknown_controlled),
            Err(GameStateError::ControlledMemberNotInParty { member_id })
                if member_id == "outsider"
        ));
    }

    #[test]
    fn controlled_member_change_is_atomic_and_accepts_only_party_members() {
        let mut state = GameState::try_from_parts(valid_parts()).expect("parts should be complete");

        assert_eq!(
            state.set_controlled_member("outsider"),
            Err(GameStateError::ControlledMemberNotInParty {
                member_id: "outsider".to_owned()
            })
        );
        assert_eq!(state.controlled_member_id(), "ember");
        assert_eq!(
            state.set_controlled_member(""),
            Err(GameStateError::EmptyControlledMemberId)
        );
        assert_eq!(state.controlled_member_id(), "ember");
        assert_eq!(state.set_controlled_member("mira"), Ok(()));
        assert_eq!(state.controlled_member_id(), "mira");
    }

    #[test]
    fn root_mutation_does_not_change_catalog_or_balance_sources() {
        let catalog = catalog();
        let catalog_before = catalog.clone();
        let balance = balance();
        let balance_before = balance.clone();
        let party = RuntimeParty::try_from_members(
            catalog.party.iter().map(|member| runtime(member, &balance)),
        )
        .expect("invented party should be complete");
        let mut parts = valid_parts();
        parts.party = party;
        parts.repository = RuntimeRepository::from_balance(&balance.economy);
        let mut state = GameState::try_from_parts(parts).expect("parts should be complete");

        assert!(state.flags_mut().set("runtime_only"));
        assert_eq!(state.repository_mut().add_gp(10).unwrap().added(), 10);
        state
            .map_mut()
            .set_position(Position::new(i32::MAX, i32::MIN));
        assert!(
            state
                .opened_boxes_mut()
                .record(OpenedBoxKey::try_new(map_id("invented_town"), "box_02").unwrap())
        );
        state.playtime_mut().commit_session(Duration::from_secs(30));

        assert_eq!(catalog, catalog_before);
        assert_eq!(balance, balance_before);
    }
}
