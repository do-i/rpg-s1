//! Ordered runtime ownership of active party members.
//!
//! The Python `PartyState` uses list order for presentation and returns the first protagonist.
//! Rust preserves that ordering while rejecting duplicate identities and multiple protagonists,
//! which the save ADR identifies as malformed state. An empty party remains available through
//! [`Default`] for staged new-game construction; [`RuntimeParty::try_from_members`] validates a
//! complete party and therefore requires exactly one protagonist.
//!
//! Battle row remains stored only on [`RuntimeMember`]. This resource performs member lookup and
//! delegates row changes to that state rather than maintaining a parallel row collection.

use std::{error::Error, fmt};

use bevy::prelude::Resource;

use crate::{runtime_member::RuntimeMember, scenario_party::PartyRow};

/// The active party in stable insertion, authored, or save-file order.
#[derive(Clone, Debug, Default, Eq, PartialEq, Resource)]
pub struct RuntimeParty {
    members: Vec<RuntimeMember>,
}

impl RuntimeParty {
    /// Builds a complete party, rejecting duplicate ids and invalid protagonist cardinality.
    pub fn try_from_members(
        members: impl IntoIterator<Item = RuntimeMember>,
    ) -> Result<Self, RuntimePartyError> {
        let mut party = Self::default();
        for member in members {
            party.try_add(member)?;
        }
        if party.protagonist().is_none() {
            return Err(RuntimePartyError::MissingProtagonist);
        }
        Ok(party)
    }

    pub fn len(&self) -> usize {
        self.members.len()
    }

    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Iterates members in stable party order without permitting callers to reorder them.
    pub fn members(&self) -> impl ExactSizeIterator<Item = &RuntimeMember> {
        self.members.iter()
    }

    pub fn contains(&self, member_id: &str) -> bool {
        self.member(member_id).is_some()
    }

    pub fn member(&self, member_id: &str) -> Option<&RuntimeMember> {
        self.members.iter().find(|member| member.id() == member_id)
    }

    pub fn protagonist(&self) -> Option<&RuntimeMember> {
        self.members.iter().find(|member| member.is_protagonist())
    }

    /// Adds a member at the end of the party without changing existing state on failure.
    ///
    /// Duplicate ids are rejected rather than ignored: silently treating a second member as an
    /// idempotent add would discard potentially different mutable member state.
    pub fn try_add(&mut self, member: RuntimeMember) -> Result<(), RuntimePartyError> {
        if self.contains(member.id()) {
            return Err(RuntimePartyError::DuplicateMemberId {
                member_id: member.id().to_owned(),
            });
        }

        if member.is_protagonist()
            && let Some(existing) = self.protagonist()
        {
            return Err(RuntimePartyError::MultipleProtagonists {
                existing_id: existing.id().to_owned(),
                rejected_id: member.id().to_owned(),
            });
        }

        self.members.push(member);
        Ok(())
    }

    pub fn row_of(&self, member_id: &str) -> Option<PartyRow> {
        self.member(member_id).map(RuntimeMember::row)
    }

    /// Changes one member's row and returns its previous row.
    pub fn set_row(
        &mut self,
        member_id: &str,
        row: PartyRow,
    ) -> Result<PartyRow, RuntimePartyError> {
        let member = self
            .members
            .iter_mut()
            .find(|member| member.id() == member_id)
            .ok_or_else(|| RuntimePartyError::MemberNotFound {
                member_id: member_id.to_owned(),
            })?;
        Ok(member.set_row(row))
    }
}

/// Invalid runtime party construction or mutation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimePartyError {
    DuplicateMemberId {
        member_id: String,
    },
    MissingProtagonist,
    MultipleProtagonists {
        existing_id: String,
        rejected_id: String,
    },
    MemberNotFound {
        member_id: String,
    },
}

impl fmt::Display for RuntimePartyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateMemberId { member_id } => {
                write!(
                    formatter,
                    "party contains duplicate member id `{member_id}`"
                )
            }
            Self::MissingProtagonist => write!(formatter, "party requires one protagonist"),
            Self::MultipleProtagonists {
                existing_id,
                rejected_id,
            } => write!(
                formatter,
                "party protagonist `{existing_id}` conflicts with `{rejected_id}`"
            ),
            Self::MemberNotFound { member_id } => {
                write!(formatter, "party member `{member_id}` was not found")
            }
        }
    }
}

impl Error for RuntimePartyError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        runtime_member::RuntimeMember,
        scenario_balance::{PositiveInteger, ProgressionBalance},
        scenario_party::{PartyCatalog, PartyMember},
        scenario_yaml,
    };

    fn catalog() -> PartyCatalog {
        scenario_yaml::from_str(include_str!("../tests/fixtures/party-catalog-shapes.yaml"))
            .expect("invented party catalog should deserialize")
    }

    fn progression() -> ProgressionBalance {
        ProgressionBalance {
            level_cap: PositiveInteger::new(100).expect("test level cap is positive"),
            exp_cap: PositiveInteger::new(1_000_000).expect("test EXP cap is positive"),
        }
    }

    fn runtime(member: &PartyMember) -> RuntimeMember {
        RuntimeMember::try_from_catalog(member, &progression())
            .expect("invented member should construct runtime state")
    }

    fn with_id(member: &mut PartyMember, id: &str) {
        match member {
            PartyMember::Protagonist(data) | PartyMember::Recruit { member: data, .. } => {
                data.id = id.to_owned();
            }
        }
    }

    #[test]
    fn complete_construction_preserves_order_and_finds_the_protagonist() {
        let source = catalog();
        let party = RuntimeParty::try_from_members(source.party.iter().map(runtime))
            .expect("invented party should be complete");

        assert_eq!(party.len(), 2);
        assert!(!party.is_empty());
        assert_eq!(
            party.members().map(RuntimeMember::id).collect::<Vec<_>>(),
            ["ember", "mira"]
        );
        assert_eq!(party.protagonist().map(RuntimeMember::id), Some("ember"));
    }

    #[test]
    fn staged_add_supports_membership_and_rejects_duplicate_ids_atomically() {
        let source = catalog();
        let mut party = RuntimeParty::default();
        assert!(party.is_empty());

        party
            .try_add(runtime(&source.party[1]))
            .expect("first recruit should be accepted");
        party
            .try_add(runtime(&source.party[0]))
            .expect("first protagonist should be accepted");
        assert!(party.contains("mira"));
        assert!(party.contains("ember"));
        assert!(!party.contains("unknown"));
        assert_eq!(party.member("mira").map(RuntimeMember::name), Some("Mira"));

        let before = party.clone();
        assert_eq!(
            party.try_add(runtime(&source.party[0])),
            Err(RuntimePartyError::DuplicateMemberId {
                member_id: "ember".to_owned()
            })
        );
        assert_eq!(party, before, "a rejected duplicate changed the party");
    }

    #[test]
    fn complete_party_requires_exactly_one_protagonist() {
        let source = catalog();
        assert_eq!(
            RuntimeParty::try_from_members([runtime(&source.party[1])]),
            Err(RuntimePartyError::MissingProtagonist)
        );

        let mut second_protagonist = source.party[0].clone();
        with_id(&mut second_protagonist, "second_hero");
        let original = runtime(&source.party[0]);
        let second = runtime(&second_protagonist);
        assert_eq!(
            RuntimeParty::try_from_members([original, second]),
            Err(RuntimePartyError::MultipleProtagonists {
                existing_id: "ember".to_owned(),
                rejected_id: "second_hero".to_owned()
            })
        );
    }

    #[test]
    fn row_changes_live_on_members_and_leave_catalog_data_immutable() {
        let source = catalog();
        let source_before = source.clone();
        let mut party = RuntimeParty::try_from_members(source.party.iter().map(runtime))
            .expect("invented party should be complete");

        assert_eq!(party.row_of("ember"), Some(PartyRow::Front));
        assert_eq!(party.set_row("ember", PartyRow::Back), Ok(PartyRow::Front));
        assert_eq!(party.row_of("ember"), Some(PartyRow::Back));
        assert_eq!(
            party.member("ember").map(RuntimeMember::row),
            Some(PartyRow::Back)
        );
        assert_eq!(
            party.set_row("unknown", PartyRow::Front),
            Err(RuntimePartyError::MemberNotFound {
                member_id: "unknown".to_owned()
            })
        );
        assert_eq!(
            source, source_before,
            "runtime row change mutated catalog data"
        );
    }
}
