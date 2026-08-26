//! Pure construction of a new runtime session from loaded scenario data.
//!
//! The pinned Python flow calls `from_new_game` only after name entry confirms. This module
//! deliberately stops at the equivalent data boundary: it accepts typed, already loaded
//! manifest, party, and balance inputs and returns one complete [`GameState`]. M3.13 owns the
//! confirmed-name handoff and atomic insertion into the Bevy world; constructing or installing
//! a default-name session while merely entering the name screen would change source behavior.

use std::{error::Error, fmt, time::Duration};

use crate::{
    game_state::{GameState, GameStateError, GameStateParts},
    gameplay_rng::GameplayRng,
    playtime::Playtime,
    runtime_flags::RuntimeFlags,
    runtime_map::{RuntimeMapId, RuntimeMapIdError, RuntimeMapState},
    runtime_member::{RuntimeMember, RuntimeMemberError},
    runtime_opened_boxes::RuntimeOpenedBoxes,
    runtime_party::{RuntimeParty, RuntimePartyError},
    runtime_repository::RuntimeRepository,
    scenario_balance::BalanceData,
    scenario_manifest::Manifest,
    scenario_party::PartyCatalog,
    scenario_spatial::CardinalDirection,
};

/// Loaded scenario inputs required to construct a new game.
///
/// References preserve the caller's catalog ownership and ensure this boundary cannot silently
/// fall back to embedded scenario values.
#[derive(Clone, Copy, Debug)]
pub struct NewGameScenario<'a> {
    pub manifest: &'a Manifest,
    pub party: &'a PartyCatalog,
    pub balance: &'a BalanceData,
}

/// Builds the source-compatible state that name confirmation will install.
///
/// Only the manifest-selected protagonist joins the initial party. The repository and opened-box
/// set begin empty, the initial facing is down, the deterministic gameplay stream uses its
/// documented default seed, and playtime starts a zero-total session at the injected wall-clock
/// sample. The confirmed protagonist name is intentionally applied by M3.13.
pub fn build_new_game_state(
    scenario: NewGameScenario<'_>,
    session_start: Duration,
) -> Result<GameState, NewGameStateError> {
    let protagonist_id = &scenario.manifest.protagonist.id;
    let protagonist = scenario
        .party
        .party
        .iter()
        .find(|member| member.data().id == *protagonist_id)
        .ok_or_else(|| NewGameStateError::ProtagonistNotFound {
            member_id: protagonist_id.clone(),
        })?;

    if !protagonist.is_protagonist() {
        return Err(NewGameStateError::SelectedMemberIsNotProtagonist {
            member_id: protagonist_id.clone(),
        });
    }
    if protagonist.data().class_id != scenario.manifest.protagonist.class {
        return Err(NewGameStateError::ProtagonistClassMismatch {
            member_id: protagonist_id.clone(),
            manifest_class_id: scenario.manifest.protagonist.class.clone(),
            party_class_id: protagonist.data().class_id.clone(),
        });
    }

    let member = RuntimeMember::try_from_catalog(protagonist, &scenario.balance.progression)
        .map_err(NewGameStateError::Member)?;
    let controlled_member_id = member.id().to_owned();
    let party = RuntimeParty::try_from_members([member]).map_err(NewGameStateError::Party)?;
    let map_id = RuntimeMapId::try_new(scenario.manifest.start.map.clone())
        .map_err(NewGameStateError::MapId)?;
    let mut playtime = Playtime::default();
    playtime.start_session(session_start);

    GameState::try_from_parts(GameStateParts {
        flags: RuntimeFlags::from_bootstrap(scenario.manifest.bootstrap_flags.iter().cloned()),
        party,
        repository: RuntimeRepository::from_balance(&scenario.balance.economy),
        map: RuntimeMapState::new(
            map_id,
            scenario.manifest.start.position,
            CardinalDirection::Down,
        ),
        opened_boxes: RuntimeOpenedBoxes::default(),
        controlled_member_id,
        rng: GameplayRng::default(),
        playtime,
    })
    .map_err(NewGameStateError::GameState)
}

/// Invalid loaded inputs at the new-game construction boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NewGameStateError {
    ProtagonistNotFound {
        member_id: String,
    },
    SelectedMemberIsNotProtagonist {
        member_id: String,
    },
    ProtagonistClassMismatch {
        member_id: String,
        manifest_class_id: String,
        party_class_id: String,
    },
    Member(RuntimeMemberError),
    Party(RuntimePartyError),
    MapId(RuntimeMapIdError),
    GameState(GameStateError),
}

impl fmt::Display for NewGameStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProtagonistNotFound { member_id } => write!(
                formatter,
                "manifest protagonist `{member_id}` is absent from the party catalog"
            ),
            Self::SelectedMemberIsNotProtagonist { member_id } => write!(
                formatter,
                "manifest protagonist `{member_id}` is not a protagonist party record"
            ),
            Self::ProtagonistClassMismatch {
                member_id,
                manifest_class_id,
                party_class_id,
            } => write!(
                formatter,
                "manifest protagonist `{member_id}` class `{manifest_class_id}` disagrees with party class `{party_class_id}`"
            ),
            Self::Member(error) => {
                write!(formatter, "protagonist runtime state is invalid: {error}")
            }
            Self::Party(error) => write!(formatter, "initial party is invalid: {error}"),
            Self::MapId(error) => write!(formatter, "new-game map is invalid: {error}"),
            Self::GameState(error) => write!(formatter, "new-game state is invalid: {error}"),
        }
    }
}

impl Error for NewGameStateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Member(error) => Some(error),
            Self::Party(error) => Some(error),
            Self::MapId(error) => Some(error),
            Self::GameState(error) => Some(error),
            Self::ProtagonistNotFound { .. }
            | Self::SelectedMemberIsNotProtagonist { .. }
            | Self::ProtagonistClassMismatch { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path, process::Command};

    use super::*;
    use crate::{
        gameplay_rng::{DEFAULT_GAMEPLAY_SEED, GameplayRng},
        runtime_member::EquipmentSlot,
        scenario_party::PartyMember,
        scenario_spatial::Position,
        scenario_yaml,
    };

    fn manifest() -> Manifest {
        scenario_yaml::from_str(include_str!(
            "../../../tests/fixtures/rusted-kingdoms-manifest-complete.yaml"
        ))
        .expect("invented complete manifest should deserialize")
    }

    fn party() -> PartyCatalog {
        let mut party: PartyCatalog = scenario_yaml::from_str(include_str!(
            "../../../tests/fixtures/party-catalog-shapes.yaml"
        ))
        .expect("invented party catalog should deserialize");
        let PartyMember::Protagonist(protagonist) = &mut party.party[0] else {
            panic!("invented first member should be the protagonist");
        };
        protagonist.id = "aric".to_owned();
        protagonist.name = "Aric".to_owned();
        protagonist.class_id = "hero".to_owned();
        party
    }

    fn balance() -> BalanceData {
        scenario_yaml::from_str(include_str!(
            "../../../tests/fixtures/balance-complete.yaml"
        ))
        .expect("invented balance should deserialize")
    }

    #[test]
    fn builds_complete_new_game_state_from_typed_loaded_inputs() {
        let manifest = manifest();
        let party = party();
        let balance = balance();
        let mut state = build_new_game_state(
            NewGameScenario {
                manifest: &manifest,
                party: &party,
                balance: &balance,
            },
            Duration::from_secs(42),
        )
        .expect("valid invented inputs should build a game state");

        assert_eq!(state.controlled_member_id(), "aric");
        assert_eq!(
            state.party().protagonist().map(RuntimeMember::id),
            Some("aric")
        );
        assert_eq!(
            state.party().protagonist().map(RuntimeMember::name),
            Some("Aric")
        );
        assert!(state.party().member("mira").is_none());
        assert_eq!(
            state.map().current().map(RuntimeMapId::as_str),
            Some("town_01_ardel")
        );
        assert_eq!(state.map().position(), Position::new(14, 5));
        assert_eq!(state.map().facing(), CardinalDirection::Down);
        assert_eq!(state.map().visited().count(), 0);
        assert_eq!(
            state.flags().iter().collect::<Vec<_>>(),
            ["aric_teleport_unlocked", "story_quest_started"]
        );
        assert_eq!(state.repository().gp(), 0);
        assert_eq!(state.repository().item_counts().count(), 0);
        assert_eq!(state.repository().gp_cap(), 600_000);
        assert_eq!(state.repository().item_quantity_cap(), 88);
        assert_eq!(state.opened_boxes().iter().count(), 0);
        assert_eq!(state.playtime().total_seconds(), 0);
        state.playtime_mut().commit_session(Duration::from_secs(43));
        assert_eq!(state.playtime().total_seconds(), 1);

        let actual = state.rng_mut().next_u64();
        let expected = GameplayRng::from_seed(DEFAULT_GAMEPLAY_SEED).next_u64();
        assert_eq!(actual, expected);
    }

    #[test]
    fn new_game_invariants_use_selected_caps_exact_bootstrap_flags_and_only_the_protagonist() {
        let manifest = manifest();
        let party = party();
        let balance = balance();
        let state = build_new_game_state(
            NewGameScenario {
                manifest: &manifest,
                party: &party,
                balance: &balance,
            },
            Duration::ZERO,
        )
        .expect("valid invented inputs should build a game state");

        assert_eq!(state.repository().gp(), 0);
        assert_eq!(state.repository().item_counts().count(), 0);
        assert_eq!(state.repository().gp_cap(), balance.economy.gp_cap.get());
        assert_eq!(
            state.repository().item_quantity_cap(),
            balance.economy.item_qty_cap.get()
        );

        assert_eq!(
            state.flags().iter().collect::<Vec<_>>(),
            ["aric_teleport_unlocked", "story_quest_started"]
        );
        assert!(!state.flags().is_set("story_act2_started"));

        assert_eq!(state.party().len(), 1);
        assert_eq!(
            state
                .party()
                .members()
                .map(RuntimeMember::id)
                .collect::<Vec<_>>(),
            [manifest.protagonist.id.as_str()]
        );
        assert_eq!(state.controlled_member_id(), manifest.protagonist.id);
        let protagonist = state.party().protagonist().unwrap();
        assert_eq!(
            protagonist.equipment().get(EquipmentSlot::Weapon),
            Some("iron_blade")
        );
        assert_eq!(
            protagonist.equipment().get(EquipmentSlot::Shield),
            Some("round_shield")
        );
        assert_eq!(
            protagonist.equipment().get(EquipmentSlot::Helmet),
            Some("leather_cap")
        );
        assert_eq!(
            protagonist.equipment().get(EquipmentSlot::Body),
            Some("leather_coat")
        );
        assert_eq!(protagonist.equipment().get(EquipmentSlot::Accessory), None);
    }

    #[test]
    fn rejects_missing_non_protagonist_and_class_mismatched_manifest_selections() {
        let mut manifest = manifest();
        let party = party();
        let balance = balance();

        manifest.protagonist.id = "missing".to_owned();
        assert_eq!(
            build_new_game_state(
                NewGameScenario {
                    manifest: &manifest,
                    party: &party,
                    balance: &balance,
                },
                Duration::ZERO,
            )
            .unwrap_err(),
            NewGameStateError::ProtagonistNotFound {
                member_id: "missing".to_owned()
            }
        );

        manifest.protagonist.id = "mira".to_owned();
        manifest.protagonist.class = "mystic".to_owned();
        assert_eq!(
            build_new_game_state(
                NewGameScenario {
                    manifest: &manifest,
                    party: &party,
                    balance: &balance,
                },
                Duration::ZERO,
            )
            .unwrap_err(),
            NewGameStateError::SelectedMemberIsNotProtagonist {
                member_id: "mira".to_owned()
            }
        );

        manifest.protagonist.id = "aric".to_owned();
        manifest.protagonist.class = "other_class".to_owned();
        assert_eq!(
            build_new_game_state(
                NewGameScenario {
                    manifest: &manifest,
                    party: &party,
                    balance: &balance,
                },
                Duration::ZERO,
            )
            .unwrap_err(),
            NewGameStateError::ProtagonistClassMismatch {
                member_id: "aric".to_owned(),
                manifest_class_id: "other_class".to_owned(),
                party_class_id: "hero".to_owned(),
            }
        );
    }

    #[test]
    fn failed_construction_returns_no_partial_state_and_retains_no_builder_state() {
        let manifest = manifest();
        let mut party = party();
        let balance = balance();
        let PartyMember::Protagonist(protagonist) = &mut party.party[0] else {
            unreachable!();
        };
        let valid_health = protagonist.hp;
        protagonist.hp = protagonist.hp_max + 1;

        let failed = build_new_game_state(
            NewGameScenario {
                manifest: &manifest,
                party: &party,
                balance: &balance,
            },
            Duration::from_secs(9),
        );
        assert!(matches!(
            failed,
            Err(NewGameStateError::Member(
                RuntimeMemberError::HealthAboveMaximum { .. }
            ))
        ));

        let PartyMember::Protagonist(protagonist) = &mut party.party[0] else {
            unreachable!();
        };
        protagonist.hp = valid_health;
        let pristine = build_new_game_state(
            NewGameScenario {
                manifest: &manifest,
                party: &party,
                balance: &balance,
            },
            Duration::from_secs(9),
        )
        .expect("a corrected input should build independently after a failed attempt");
        assert_eq!(pristine.repository().gp(), 0);
        assert_eq!(pristine.opened_boxes().iter().count(), 0);
        assert_eq!(pristine.playtime().total_seconds(), 0);
    }

    #[test]
    #[ignore = "requires RPG_S1_PINNED_SOURCE_DIR at the clean pinned Python source checkout"]
    fn audits_exact_new_game_values_from_the_pinned_python_source() {
        const PIN: &str = "08970359d6cb03586948625d29b0d3351dbbf785";
        let source = std::env::var_os("RPG_S1_PINNED_SOURCE_DIR")
            .map(std::path::PathBuf::from)
            .expect("set RPG_S1_PINNED_SOURCE_DIR");
        assert_clean_pin(&source, PIN);
        let scenario_root = source.join("rusted_kingdoms");
        let manifest: Manifest = read_yaml(&scenario_root.join("manifest.yaml"));
        let party: PartyCatalog = read_yaml(&scenario_root.join(manifest.refs.party.as_str()));
        let balance: BalanceData = read_yaml(&scenario_root.join(manifest.refs.balance.as_str()));

        let state = build_new_game_state(
            NewGameScenario {
                manifest: &manifest,
                party: &party,
                balance: &balance,
            },
            Duration::from_secs(123),
        )
        .expect("pinned source should build a valid new game");

        assert_eq!(state.controlled_member_id(), "aric");
        assert_eq!(
            state.party().protagonist().map(RuntimeMember::id),
            Some("aric")
        );
        assert_eq!(
            state.party().protagonist().map(RuntimeMember::name),
            Some("Aric")
        );
        assert_eq!(state.party().len(), 1);
        assert_eq!(
            state
                .party()
                .members()
                .map(RuntimeMember::id)
                .collect::<Vec<_>>(),
            [manifest.protagonist.id.as_str()]
        );
        assert_eq!(state.repository().gp(), 0);
        assert_eq!(state.repository().item_counts().count(), 0);
        assert_eq!(state.repository().gp_cap(), balance.economy.gp_cap.get());
        assert_eq!(
            state.repository().item_quantity_cap(),
            balance.economy.item_qty_cap.get()
        );
        let protagonist = state.party().protagonist().unwrap();
        assert_eq!(
            protagonist.equipment().get(EquipmentSlot::Weapon),
            Some("iron_sword")
        );
        assert_eq!(
            protagonist.equipment().get(EquipmentSlot::Shield),
            Some("buckler")
        );
        assert_eq!(
            protagonist.equipment().get(EquipmentSlot::Helmet),
            Some("leather_hat")
        );
        assert_eq!(
            protagonist.equipment().get(EquipmentSlot::Body),
            Some("leather_vest")
        );
        assert_eq!(protagonist.equipment().get(EquipmentSlot::Accessory), None);
        assert_eq!(
            state.map().current().map(RuntimeMapId::as_str),
            Some("town_01_ardel")
        );
        assert_eq!(state.map().position(), Position::new(14, 5));
        assert_eq!(state.map().facing(), CardinalDirection::Down);
        assert_eq!(
            state.flags().iter().collect::<Vec<_>>(),
            ["aric_teleport_unlocked", "story_quest_started"]
        );
    }

    fn read_yaml<T: serde::de::DeserializeOwned>(path: &Path) -> T {
        let document = fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("{} should be readable: {error}", path.display()));
        scenario_yaml::from_str(&document)
            .unwrap_or_else(|error| panic!("{} should deserialize: {error}", path.display()))
    }

    fn assert_clean_pin(source: &Path, pin: &str) {
        let head = Command::new("git")
            .arg("-C")
            .arg(source)
            .args(["rev-parse", "HEAD"])
            .output()
            .expect("source HEAD query should run");
        assert!(head.status.success(), "source HEAD query failed");
        assert_eq!(String::from_utf8(head.stdout).unwrap().trim(), pin);

        let status = Command::new("git")
            .arg("-C")
            .arg(source)
            .args(["status", "--short"])
            .output()
            .expect("source worktree query should run");
        assert!(status.status.success(), "source worktree query failed");
        assert!(
            status.stdout.is_empty(),
            "the pinned Python source worktree must be clean"
        );
    }
}
