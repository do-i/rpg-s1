//! Mutable GP and shared item-count state for one game session.
//!
//! The pinned Python repository stores every item kind in one dictionary. Consumables,
//! equipment, materials, key items, and magic cores therefore remain one shared collection here;
//! catalog definitions and later tags decide how menus and services classify them.
//!
//! Additions retain the source's cap-and-clip behavior, but return [`AdditionOutcome`] so lost
//! overflow is observable without relying on a log message. Unlike Python's `remove_item`, an
//! oversized removal is rejected without deleting the stack. This stricter boundary prevents a
//! caller error from silently consuming more items than requested. Item metadata, loot batches,
//! locks, transactions, serialization, and UI remain with their later milestones.

use std::{collections::BTreeMap, error::Error, fmt};

use bevy::prelude::Resource;

use crate::scenario_balance::{BalanceData, EconomyBalance};

/// Shared party funds and item stacks.
#[derive(Clone, Debug, Eq, PartialEq, Resource)]
pub struct RuntimeRepository {
    gp: u32,
    item_counts: BTreeMap<String, u32>,
    gp_cap: u32,
    item_quantity_cap: u32,
}

impl RuntimeRepository {
    /// Creates an empty repository using the selected scenario's economy caps.
    pub fn from_balance(balance: &EconomyBalance) -> Self {
        Self {
            gp: 0,
            item_counts: BTreeMap::new(),
            gp_cap: balance.gp_cap.get(),
            item_quantity_cap: balance.item_qty_cap.get(),
        }
    }

    pub fn gp(&self) -> u32 {
        self.gp
    }

    pub fn gp_cap(&self) -> u32 {
        self.gp_cap
    }

    pub fn item_quantity_cap(&self) -> u32 {
        self.item_quantity_cap
    }

    /// Adds GP up to the configured cap and reports any clipped remainder.
    pub fn add_gp(&mut self, amount: u32) -> Result<AdditionOutcome, RepositoryError> {
        require_positive(amount)?;
        let outcome = capped_addition(self.gp, amount, self.gp_cap);
        self.gp += outcome.added;
        Ok(outcome)
    }

    /// Spends GP exactly, leaving the balance unchanged when funds are insufficient.
    pub fn spend_gp(&mut self, amount: u32) -> Result<(), RepositoryError> {
        require_positive(amount)?;
        if amount > self.gp {
            return Err(RepositoryError::InsufficientGp {
                available: self.gp,
                requested: amount,
            });
        }
        self.gp -= amount;
        Ok(())
    }

    pub fn contains_item(&self, item_id: &str) -> bool {
        self.item_counts.contains_key(item_id)
    }

    pub fn item_count(&self, item_id: &str) -> u32 {
        self.item_counts.get(item_id).copied().unwrap_or(0)
    }

    /// Iterates non-empty stacks in stable item-id order.
    pub fn item_counts(&self) -> impl ExactSizeIterator<Item = (&str, u32)> {
        self.item_counts
            .iter()
            .map(|(item_id, quantity)| (item_id.as_str(), *quantity))
    }

    /// Adds to one shared stack up to the configured per-item cap.
    pub fn add_item(
        &mut self,
        item_id: impl Into<String>,
        amount: u32,
    ) -> Result<AdditionOutcome, RepositoryError> {
        let item_id = item_id.into();
        require_item_id(&item_id)?;
        require_positive(amount)?;

        let current = self.item_count(&item_id);
        let outcome = capped_addition(current, amount, self.item_quantity_cap);
        if outcome.added > 0 {
            self.item_counts.insert(item_id, current + outcome.added);
        }
        Ok(outcome)
    }

    /// Removes exactly `amount`, deleting the stack when its count reaches zero.
    ///
    /// Missing and insufficient stacks are errors and never mutate repository state.
    pub fn remove_item(&mut self, item_id: &str, amount: u32) -> Result<(), RepositoryError> {
        require_item_id(item_id)?;
        require_positive(amount)?;

        let available = self.item_count(item_id);
        if available == 0 {
            return Err(RepositoryError::ItemNotFound {
                item_id: item_id.to_owned(),
            });
        }
        if amount > available {
            return Err(RepositoryError::InsufficientItems {
                item_id: item_id.to_owned(),
                available,
                requested: amount,
            });
        }

        if amount == available {
            self.item_counts.remove(item_id);
        } else {
            let quantity = self
                .item_counts
                .get_mut(item_id)
                .expect("positive count was read from this map");
            *quantity -= amount;
        }
        Ok(())
    }
}

impl Default for RuntimeRepository {
    fn default() -> Self {
        Self::from_balance(&BalanceData::default().economy)
    }
}

fn require_positive(amount: u32) -> Result<(), RepositoryError> {
    if amount == 0 {
        Err(RepositoryError::ZeroAmount)
    } else {
        Ok(())
    }
}

fn require_item_id(item_id: &str) -> Result<(), RepositoryError> {
    if item_id.is_empty() {
        Err(RepositoryError::EmptyItemId)
    } else {
        Ok(())
    }
}

fn capped_addition(current: u32, requested: u32, cap: u32) -> AdditionOutcome {
    let added = requested.min(cap - current);
    AdditionOutcome {
        added,
        remainder: requested - added,
    }
}

/// Observable result of a source-compatible capped addition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[must_use]
pub struct AdditionOutcome {
    added: u32,
    remainder: u32,
}

impl AdditionOutcome {
    pub fn added(self) -> u32 {
        self.added
    }

    pub fn remainder(self) -> u32 {
        self.remainder
    }

    pub fn was_capped(self) -> bool {
        self.remainder > 0
    }
}

/// Invalid repository input or an exact operation that cannot be completed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RepositoryError {
    ZeroAmount,
    EmptyItemId,
    InsufficientGp {
        available: u32,
        requested: u32,
    },
    ItemNotFound {
        item_id: String,
    },
    InsufficientItems {
        item_id: String,
        available: u32,
        requested: u32,
    },
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroAmount => write!(formatter, "repository amount must be positive"),
            Self::EmptyItemId => write!(formatter, "repository item id must not be empty"),
            Self::InsufficientGp {
                available,
                requested,
            } => write!(
                formatter,
                "cannot spend {requested} GP with only {available} available"
            ),
            Self::ItemNotFound { item_id } => {
                write!(formatter, "repository item `{item_id}` was not found")
            }
            Self::InsufficientItems {
                item_id,
                available,
                requested,
            } => write!(
                formatter,
                "cannot remove {requested} of `{item_id}` with only {available} available"
            ),
        }
    }
}

impl Error for RepositoryError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario_balance::PositiveInteger;

    fn balance(gp_cap: u32, item_quantity_cap: u32) -> EconomyBalance {
        EconomyBalance {
            gp_cap: PositiveInteger::new(gp_cap).expect("test GP cap is positive"),
            item_qty_cap: PositiveInteger::new(item_quantity_cap)
                .expect("test item cap is positive"),
            max_tags_per_item: 0,
        }
    }

    #[test]
    fn defaults_are_empty_and_caps_come_from_balance() {
        let default_repository = RuntimeRepository::default();
        assert_eq!(default_repository.gp(), 0);
        assert_eq!(default_repository.gp_cap(), 8_000_000);
        assert_eq!(default_repository.item_quantity_cap(), 100);
        assert_eq!(default_repository.item_counts().count(), 0);

        let custom = RuntimeRepository::from_balance(&balance(50, 7));
        assert_eq!(custom.gp_cap(), 50);
        assert_eq!(custom.item_quantity_cap(), 7);
    }

    #[test]
    fn gp_addition_reports_clipping_and_spending_is_exact_and_atomic() {
        let mut repository = RuntimeRepository::from_balance(&balance(10, 5));

        let first = repository.add_gp(8).expect("positive GP should add");
        assert_eq!((first.added(), first.remainder()), (8, 0));
        assert!(!first.was_capped());

        let clipped = repository
            .add_gp(u32::MAX)
            .expect("overflowing GP should clip, not fail");
        assert_eq!((clipped.added(), clipped.remainder()), (2, u32::MAX - 2));
        assert!(clipped.was_capped());
        assert_eq!(repository.gp(), 10);

        assert_eq!(repository.spend_gp(10), Ok(()));
        assert_eq!(repository.gp(), 0);
        assert_eq!(
            repository.spend_gp(1),
            Err(RepositoryError::InsufficientGp {
                available: 0,
                requested: 1
            })
        );
        assert_eq!(repository.gp(), 0);
        assert_eq!(repository.add_gp(0), Err(RepositoryError::ZeroAmount));
        assert_eq!(repository.spend_gp(0), Err(RepositoryError::ZeroAmount));
    }

    #[test]
    fn every_item_kind_shares_one_deterministically_ordered_collection() {
        let mut repository = RuntimeRepository::from_balance(&balance(10, 5));
        assert_eq!(
            repository
                .add_item("potion", 2)
                .expect("item should add")
                .added(),
            2
        );
        assert_eq!(
            repository
                .add_item("quest_key", 1)
                .expect("key item should share the repository")
                .added(),
            1
        );
        assert_eq!(
            repository
                .add_item("mc_xs", 3)
                .expect("magic core should share the repository")
                .added(),
            3
        );

        assert!(repository.contains_item("quest_key"));
        assert_eq!(repository.item_count("mc_xs"), 3);
        assert_eq!(repository.item_count("missing"), 0);
        assert_eq!(
            repository.item_counts().collect::<Vec<_>>(),
            [("mc_xs", 3), ("potion", 2), ("quest_key", 1)]
        );
    }

    #[test]
    fn item_addition_clips_observably_and_rejects_invalid_inputs() {
        let mut repository = RuntimeRepository::from_balance(&balance(10, 5));
        let clipped = repository
            .add_item("potion", u32::MAX)
            .expect("oversized item addition should clip");
        assert_eq!((clipped.added(), clipped.remainder()), (5, u32::MAX - 5));
        assert_eq!(repository.item_count("potion"), 5);

        let before = repository.clone();
        assert_eq!(
            repository.add_item("", 1),
            Err(RepositoryError::EmptyItemId)
        );
        assert_eq!(
            repository.add_item("zero", 0),
            Err(RepositoryError::ZeroAmount)
        );
        assert_eq!(repository, before);
    }

    #[test]
    fn removal_decrements_and_deletes_empty_stacks_without_partial_failures() {
        let mut repository = RuntimeRepository::from_balance(&balance(10, 10));
        assert_eq!(
            repository
                .add_item("potion", 5)
                .expect("item should add")
                .added(),
            5
        );

        repository
            .remove_item("potion", 2)
            .expect("owned quantity should remove");
        assert_eq!(repository.item_count("potion"), 3);

        let before = repository.clone();
        assert_eq!(
            repository.remove_item("potion", 4),
            Err(RepositoryError::InsufficientItems {
                item_id: "potion".to_owned(),
                available: 3,
                requested: 4
            })
        );
        assert_eq!(repository, before);
        assert_eq!(
            repository.remove_item("missing", 1),
            Err(RepositoryError::ItemNotFound {
                item_id: "missing".to_owned()
            })
        );
        assert_eq!(
            repository.remove_item("potion", 0),
            Err(RepositoryError::ZeroAmount)
        );
        assert_eq!(repository, before);

        repository
            .remove_item("potion", 3)
            .expect("exact removal should succeed");
        assert!(!repository.contains_item("potion"));
        assert_eq!(repository.item_count("potion"), 0);
        assert_eq!(repository.item_counts().count(), 0);
    }
}
