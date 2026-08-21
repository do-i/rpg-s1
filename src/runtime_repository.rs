//! Mutable GP and shared item-count state for one game session.
//!
//! The pinned Python repository stores every item kind in one dictionary. Consumables,
//! equipment, materials, key items, and magic cores therefore remain one shared collection here;
//! catalog definitions and later tags decide how menus and services classify them.
//!
//! Additions retain the source's cap-and-clip behavior, but return [`AdditionOutcome`] so lost
//! overflow is observable without relying on a log message. Unlike Python's `remove_item`, an
//! oversized removal is rejected without deleting the stack. This stricter boundary prevents a
//! caller error from silently consuming more items than requested. M7 persists locks, tags,
//! loot identity, and latest-loot-batch metadata; the explicit Hide command remains session-only.

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use bevy::prelude::Resource;

use crate::scenario_balance::{BalanceData, EconomyBalance};

/// Shared party funds and item stacks.
#[derive(Clone, Debug, Eq, PartialEq, Resource)]
pub struct RuntimeRepository {
    gp: u32,
    item_counts: BTreeMap<String, u32>,
    gp_cap: u32,
    item_quantity_cap: u32,
    max_tags_per_item: u32,
    hidden_item_ids: BTreeSet<String>,
    locked_item_ids: BTreeSet<String>,
    item_tags: BTreeMap<String, BTreeSet<String>>,
    loot_item_ids: BTreeSet<String>,
    loot_batch_sequence: u64,
    item_loot_batches: BTreeMap<String, u64>,
}

impl RuntimeRepository {
    /// Creates an empty repository using the selected scenario's economy caps.
    pub fn from_balance(balance: &EconomyBalance) -> Self {
        Self {
            gp: 0,
            item_counts: BTreeMap::new(),
            gp_cap: balance.gp_cap.get(),
            item_quantity_cap: balance.item_qty_cap.get(),
            max_tags_per_item: balance.max_tags_per_item,
            hidden_item_ids: BTreeSet::new(),
            locked_item_ids: BTreeSet::new(),
            item_tags: BTreeMap::new(),
            loot_item_ids: BTreeSet::new(),
            loot_batch_sequence: 0,
            item_loot_batches: BTreeMap::new(),
        }
    }

    /// Restores complete persistent repository metadata from a native or converted save.
    pub(crate) fn try_from_saved(
        balance: &EconomyBalance,
        gp: u32,
        items: impl IntoIterator<Item = RuntimeRepositoryItemParts>,
    ) -> Result<Self, RepositoryError> {
        let mut repository = Self::from_balance(balance);
        if gp > repository.gp_cap {
            return Err(RepositoryError::GpAboveCap {
                gp,
                cap: repository.gp_cap,
            });
        }
        repository.gp = gp;
        for item in items {
            require_item_id(&item.id)?;
            require_positive(item.quantity)?;
            if item.quantity > repository.item_quantity_cap {
                return Err(RepositoryError::ItemQuantityAboveCap {
                    item_id: item.id,
                    quantity: item.quantity,
                    cap: repository.item_quantity_cap,
                });
            }
            if repository.item_counts.contains_key(&item.id) {
                return Err(RepositoryError::DuplicateItemId { item_id: item.id });
            }
            if item.tags.iter().any(String::is_empty) {
                return Err(RepositoryError::EmptyTag { item_id: item.id });
            }
            if item.tags.len() > balance.max_tags_per_item as usize {
                return Err(RepositoryError::TooManyTags {
                    item_id: item.id,
                    count: item.tags.len(),
                    cap: balance.max_tags_per_item,
                });
            }
            if item.loot_batch > 0 {
                repository.loot_batch_sequence =
                    repository.loot_batch_sequence.max(item.loot_batch);
                repository
                    .item_loot_batches
                    .insert(item.id.clone(), item.loot_batch);
            }
            if item.locked {
                repository.locked_item_ids.insert(item.id.clone());
            }
            if item.is_loot {
                repository.loot_item_ids.insert(item.id.clone());
            }
            if !item.tags.is_empty() {
                let mut tags = BTreeSet::new();
                for tag in item.tags {
                    if !tags.insert(tag.clone()) {
                        return Err(RepositoryError::DuplicateTag {
                            item_id: item.id,
                            tag,
                        });
                    }
                }
                repository.item_tags.insert(item.id.clone(), tags);
            }
            repository.item_counts.insert(item.id, item.quantity);
        }
        Ok(repository)
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

    /// Begins a source-compatible loot acquisition batch and returns its stable identifier.
    pub fn start_loot_batch(&mut self) -> u64 {
        self.loot_batch_sequence = self.loot_batch_sequence.saturating_add(1);
        self.loot_batch_sequence
    }

    pub fn latest_loot_batch(&self) -> Option<u64> {
        (self.loot_batch_sequence > 0).then_some(self.loot_batch_sequence)
    }

    pub fn item_loot_batch(&self, item_id: &str) -> Option<u64> {
        self.item_loot_batches.get(item_id).copied()
    }

    pub fn is_new_item(&self, item_id: &str) -> bool {
        self.latest_loot_batch()
            .is_some_and(|batch| self.item_loot_batch(item_id) == Some(batch))
    }

    /// Adds an item and stamps it with an existing acquisition batch when any quantity lands.
    pub fn add_item_in_batch(
        &mut self,
        item_id: impl Into<String>,
        amount: u32,
        batch: u64,
    ) -> Result<AdditionOutcome, RepositoryError> {
        if batch == 0 || batch > self.loot_batch_sequence {
            return Err(RepositoryError::InvalidLootBatch { batch });
        }
        let item_id = item_id.into();
        let outcome = self.add_item(item_id.clone(), amount)?;
        if outcome.added > 0 {
            self.item_loot_batches.insert(item_id, batch);
        }
        Ok(outcome)
    }

    pub(crate) fn add_loot_item_in_batch(
        &mut self,
        item_id: impl Into<String>,
        amount: u32,
        batch: u64,
        tags: impl IntoIterator<Item = String>,
    ) -> Result<AdditionOutcome, RepositoryError> {
        let item_id = item_id.into();
        let tags = tags.into_iter().collect::<BTreeSet<_>>();
        if tags.iter().any(String::is_empty) {
            return Err(RepositoryError::EmptyTag { item_id });
        }
        let existing = self.item_tags.get(&item_id);
        let tag_count = existing.map_or(0, BTreeSet::len)
            + tags
                .iter()
                .filter(|tag| existing.is_none_or(|current| !current.contains(*tag)))
                .count();
        if tag_count > self.max_tags_per_item as usize {
            return Err(RepositoryError::TooManyTags {
                item_id,
                count: tag_count,
                cap: self.max_tags_per_item,
            });
        }
        let outcome = self.add_item_in_batch(item_id.clone(), amount, batch)?;
        if outcome.added > 0 {
            self.loot_item_ids.insert(item_id.clone());
            self.item_tags.entry(item_id).or_default().extend(tags);
        }
        Ok(outcome)
    }

    pub fn is_hidden(&self, item_id: &str) -> bool {
        self.hidden_item_ids.contains(item_id)
    }

    /// Changes only the current session's presentation filter.
    pub fn set_hidden(&mut self, item_id: impl Into<String>, hidden: bool) {
        let item_id = item_id.into();
        if hidden {
            self.hidden_item_ids.insert(item_id);
        } else {
            self.hidden_item_ids.remove(&item_id);
        }
    }

    pub fn is_locked(&self, item_id: &str) -> bool {
        self.locked_item_ids.contains(item_id)
    }

    pub fn item_tags(&self, item_id: &str) -> impl Iterator<Item = &str> {
        self.item_tags
            .get(item_id)
            .into_iter()
            .flatten()
            .map(String::as_str)
    }

    pub fn is_loot(&self, item_id: &str) -> bool {
        self.loot_item_ids.contains(item_id)
    }

    pub fn set_locked(&mut self, item_id: impl Into<String>, locked: bool) {
        let item_id = item_id.into();
        if locked {
            self.locked_item_ids.insert(item_id);
        } else {
            self.locked_item_ids.remove(&item_id);
        }
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
            self.item_loot_batches.remove(item_id);
            self.item_tags.remove(item_id);
            self.locked_item_ids.remove(item_id);
            self.loot_item_ids.remove(item_id);
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RuntimeRepositoryItemParts {
    pub id: String,
    pub quantity: u32,
    pub tags: Vec<String>,
    pub locked: bool,
    pub is_loot: bool,
    pub loot_batch: u64,
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
    InvalidLootBatch {
        batch: u64,
    },
    GpAboveCap {
        gp: u32,
        cap: u32,
    },
    ItemQuantityAboveCap {
        item_id: String,
        quantity: u32,
        cap: u32,
    },
    DuplicateItemId {
        item_id: String,
    },
    EmptyTag {
        item_id: String,
    },
    DuplicateTag {
        item_id: String,
        tag: String,
    },
    TooManyTags {
        item_id: String,
        count: usize,
        cap: u32,
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
            Self::InvalidLootBatch { batch } => {
                write!(
                    formatter,
                    "loot batch {batch} is not active in this repository"
                )
            }
            Self::GpAboveCap { gp, cap } => {
                write!(formatter, "repository GP {gp} exceeds cap {cap}")
            }
            Self::ItemQuantityAboveCap {
                item_id,
                quantity,
                cap,
            } => write!(
                formatter,
                "repository item `{item_id}` quantity {quantity} exceeds cap {cap}"
            ),
            Self::DuplicateItemId { item_id } => {
                write!(
                    formatter,
                    "repository contains duplicate item id `{item_id}`"
                )
            }
            Self::EmptyTag { item_id } => {
                write!(formatter, "repository item `{item_id}` has an empty tag")
            }
            Self::DuplicateTag { item_id, tag } => write!(
                formatter,
                "repository item `{item_id}` contains duplicate tag `{tag}`"
            ),
            Self::TooManyTags {
                item_id,
                count,
                cap,
            } => write!(
                formatter,
                "repository item `{item_id}` has {count} tags, exceeding cap {cap}"
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

    #[test]
    fn loot_addition_stamps_batch_and_tags_only_after_all_metadata_validates() {
        let mut rules = balance(10, 5);
        rules.max_tags_per_item = 1;
        let mut repository = RuntimeRepository::from_balance(&rules);
        let batch = repository.start_loot_batch();
        let before = repository.clone();

        assert!(matches!(
            repository.add_loot_item_in_batch("mc_xs", 2, batch, [String::new()]),
            Err(RepositoryError::EmptyTag { .. })
        ));
        assert_eq!(repository, before);

        let added = repository
            .add_loot_item_in_batch("mc_xs", 2, batch, ["magic_core".to_owned()])
            .unwrap();
        assert_eq!(added.added(), 2);
        assert!(repository.is_loot("mc_xs"));
        assert!(repository.is_new_item("mc_xs"));
        assert_eq!(
            repository.item_tags("mc_xs").collect::<Vec<_>>(),
            ["magic_core"]
        );

        let before = repository.clone();
        assert!(matches!(
            repository.add_loot_item_in_batch("mc_xs", 1, batch, ["second".to_owned()]),
            Err(RepositoryError::TooManyTags { .. })
        ));
        assert_eq!(repository, before);
    }
}
