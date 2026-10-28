//! Core promotion is atomic: fifty input cores buy one of the next tier.

use super::{ServiceError, repository_error, require_quantity};
use crate::{runtime_repository::RuntimeRepository, scenario_item::ItemDefinition};

pub(crate) const CORE_RANK_UP_COST: u32 = 50;
pub(crate) const CORE_TIERS: [&str; 5] = ["mc_xs", "mc_s", "mc_m", "mc_l", "mc_xl"];

pub(crate) fn next_core_id(id: &str) -> Option<&'static str> {
    CORE_TIERS
        .windows(2)
        .find_map(|tiers| (tiers[0] == id).then_some(tiers[1]))
}

/// Returns the number of output cores; no mutation occurs on any failed check.
pub(crate) fn rank_up_magic_core(
    repository: &mut RuntimeRepository,
    core: &ItemDefinition,
    batches: u32,
) -> Result<u32, ServiceError> {
    require_quantity(batches)?;
    if !matches!(core, ItemDefinition::MagicCore(_)) {
        return Err(ServiceError::WrongCore);
    }
    let id = core.id();
    let output = next_core_id(id).ok_or(ServiceError::NotExchangeable)?;
    if repository.is_locked(id) {
        return Err(ServiceError::Locked);
    }
    let cost = CORE_RANK_UP_COST
        .checked_mul(batches)
        .ok_or(ServiceError::Overflow)?;
    if repository.item_count(id) < cost {
        return Err(ServiceError::NotOwned);
    }
    if batches
        > repository
            .item_quantity_cap()
            .saturating_sub(repository.item_count(output))
    {
        return Err(ServiceError::ItemCap);
    }
    let mut updated = repository.clone();
    updated.remove_item(id, cost).map_err(repository_error)?;
    let outcome = updated
        .add_item(output, batches)
        .map_err(repository_error)?;
    if outcome.added() != batches {
        return Err(ServiceError::ItemCap);
    }
    *repository = updated;
    Ok(batches)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_menu_domain::tests::{catalog, game};

    #[test]
    fn all_four_promotions_consume_exact_batches_and_leave_gp_unchanged() {
        let catalog = catalog();
        for pair in CORE_TIERS.windows(2) {
            let mut game = game([]);
            let repo = game.repository_mut();
            let _ = repo.add_item(pair[0], 100).unwrap();
            let _ = repo.add_item(pair[1], 3).unwrap();
            let before_gp = repo.gp();
            assert_eq!(
                rank_up_magic_core(repo, catalog.item(pair[0]).unwrap(), 2),
                Ok(2)
            );
            assert_eq!(repo.item_count(pair[0]), 0);
            assert_eq!(repo.item_count(pair[1]), 5);
            assert_eq!(repo.gp(), before_gp);
        }
        assert_eq!(next_core_id("mc_xl"), None);
    }

    #[test]
    fn invalid_locked_insufficient_and_full_output_transactions_are_atomic() {
        let catalog = catalog();
        for (id, count, output, locked, batches, expected) in [
            ("mc_xs", 100, 0, false, 0, ServiceError::ZeroQuantity),
            ("mc_xs", 49, 0, false, 1, ServiceError::NotOwned),
            ("mc_xs", 100, 0, true, 1, ServiceError::Locked),
            ("mc_xs", 100, 100, false, 1, ServiceError::ItemCap),
            ("mc_xs", 100, 99, false, 2, ServiceError::ItemCap),
            ("mc_xl", 100, 0, false, 1, ServiceError::NotExchangeable),
            ("potion", 100, 0, false, 1, ServiceError::WrongCore),
            ("mc_xs", 100, 0, false, u32::MAX, ServiceError::Overflow),
        ] {
            let mut game = game([]);
            let repo = game.repository_mut();
            let _ = repo.add_item(id, count).unwrap();
            if output > 0 {
                let _ = repo.add_item("mc_s", output).unwrap();
            }
            repo.set_locked(id, locked);
            let before = repo.clone();
            assert_eq!(
                rank_up_magic_core(repo, catalog.item(id).unwrap(), batches),
                Err(expected)
            );
            assert_eq!(*repo, before);
        }
    }

    #[test]
    fn one_remaining_output_slot_accepts_one_batch_and_gp_sales_respect_locks() {
        let catalog = catalog();
        let mut game = game([]);
        let repo = game.repository_mut();
        let _ = repo.add_item("mc_xs", 100).unwrap();
        let _ = repo.add_item("mc_s", 99).unwrap();
        assert_eq!(
            rank_up_magic_core(repo, catalog.item("mc_xs").unwrap(), 1),
            Ok(1)
        );
        assert_eq!(repo.item_count("mc_s"), 100);
        assert_eq!(repo.item_count("mc_xs"), 50);
        repo.set_locked("mc_xs", true);
        let before = repo.clone();
        assert_eq!(
            super::super::exchange_magic_core(repo, catalog.item("mc_xs").unwrap(), 1),
            Err(ServiceError::Locked)
        );
        assert_eq!(*repo, before);
    }
}
