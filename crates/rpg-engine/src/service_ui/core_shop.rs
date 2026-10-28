//! Core selection, transaction mode, quantity, and explicit review.

use super::*;
use crate::service_domain::{CORE_RANK_UP_COST, CORE_TIERS, next_core_id};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum CoreMode {
    #[default]
    Sell,
    RankUp,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum CoreFocus {
    #[default]
    Items,
    Quantity,
}

#[derive(Debug, Default)]
pub(super) struct CoreShopState {
    pub(super) mode: CoreMode,
    pub(super) focus: CoreFocus,
}

#[derive(Clone, Copy, Component, Debug, Eq, PartialEq)]
pub(crate) enum CoreControl {
    Item(usize),
    Mode(CoreMode),
    Quantity(isize),
    Review,
    Confirm,
    Cancel,
}

impl CoreControl {
    pub(super) fn in_review(self) -> bool {
        matches!(self, Self::Confirm | Self::Cancel)
    }
}

pub(super) struct CoreInput {
    pub(super) vertical: Option<isize>,
    pub(super) horizontal: Option<isize>,
    pub(super) confirm: bool,
    pub(super) back: bool,
    pub(super) toggle: bool,
    pub(super) control: Option<CoreControl>,
}

pub(super) fn rows(catalog: &FieldMenuCatalog) -> Vec<&ItemDefinition> {
    CORE_TIERS
        .iter()
        .filter_map(|id| catalog.item(id))
        .filter(|item| matches!(item, ItemDefinition::MagicCore(_)))
        .collect()
}

pub(super) fn selected<'a>(
    state: &ServiceUiState,
    catalog: &'a FieldMenuCatalog,
) -> Option<&'a ItemDefinition> {
    rows(catalog).get(state.selected).copied()
}

pub(super) fn max_quantity(
    catalog: &FieldMenuCatalog,
    game: &GameState,
    id: &str,
    mode: CoreMode,
) -> u32 {
    let repo = game.repository();
    if repo.is_locked(id) {
        return 0;
    }
    match mode {
        CoreMode::Sell => {
            let Some(ItemDefinition::MagicCore(core)) = catalog.item(id) else {
                return 0;
            };
            core.exchange_rate.map_or(0, |rate| {
                repo.item_count(id)
                    .min(repo.gp_cap().saturating_sub(repo.gp()) / rate.get())
            })
        }
        CoreMode::RankUp => {
            let Some(output) = next_core_id(id).filter(|output| {
                matches!(catalog.item(output), Some(ItemDefinition::MagicCore(_)))
            }) else {
                return 0;
            };
            (repo.item_count(id) / CORE_RANK_UP_COST).min(
                repo.item_quantity_cap()
                    .saturating_sub(repo.item_count(output)),
            )
        }
    }
}

pub(super) fn unavailable(
    state: &ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
) -> Option<String> {
    let item = selected(state, catalog)?;
    let id = item.id();
    let repo = game.repository();
    if repo.is_locked(id) {
        return Some("This core stack is locked. Unlock it in Items first.".to_owned());
    }
    match state.core_shop.mode {
        CoreMode::Sell => {
            let ItemDefinition::MagicCore(core) = item else {
                return None;
            };
            if core.exchange_rate.is_none() {
                return Some("Only XS and S cores can be sold for GP.".to_owned());
            }
            if repo.item_count(id) == 0 {
                return Some("No cores of this size in your pack.".to_owned());
            }
            if max_quantity(catalog, game, id, CoreMode::Sell) == 0 {
                return Some("Your purse has no room for this sale.".to_owned());
            }
        }
        CoreMode::RankUp => {
            let Some(output) = next_core_id(id) else {
                return Some("XL is the top rank. There is no higher core.".to_owned());
            };
            if catalog.item(output).is_none() {
                return Some("The next core rank is unavailable.".to_owned());
            }
            if repo.item_count(id) < CORE_RANK_UP_COST {
                return Some(format!(
                    "Need {CORE_RANK_UP_COST} cores per batch; you own {}.",
                    repo.item_count(id)
                ));
            }
            if repo.item_count(output) >= repo.item_quantity_cap() {
                return Some(format!(
                    "The next rank's stack is full ({}/{}).",
                    repo.item_count(output),
                    repo.item_quantity_cap()
                ));
            }
        }
    }
    None
}

fn select(state: &mut ServiceUiState, catalog: &FieldMenuCatalog, index: usize) {
    state.selected = index;
    state.quantity = 1;
    state.core_shop.focus = CoreFocus::Items;
    if let Some(ItemDefinition::MagicCore(core)) = selected(state, catalog) {
        state.core_shop.mode = if core.exchange_rate.is_some() {
            CoreMode::Sell
        } else {
            CoreMode::RankUp
        };
    }
}

fn review(
    state: &mut ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
    sfx: &mut MenuSfx,
) {
    let Some(item) = selected(state, catalog) else {
        return;
    };
    if let Some(reason) = unavailable(state, catalog, game) {
        state.announce(reason, ToastTone::Warn);
        sfx.blocked();
        return;
    }
    if state.quantity == 0
        || state.quantity > max_quantity(catalog, game, item.id(), state.core_shop.mode)
    {
        state.announce(
            "The available quantity changed. Choose an amount again.",
            ToastTone::Warn,
        );
        state.quantity = 1;
        sfx.blocked();
        return;
    }
    state.pending_id = Some(item.id().to_owned());
    state.pending = Some(match state.core_shop.mode {
        CoreMode::Sell => PendingTransaction::Exchange,
        CoreMode::RankUp => PendingTransaction::RankUp,
    });
    state.page = Some(ServicePage::CoreConfirm);
    sfx.confirm();
}

pub(super) fn handle_input(
    state: &mut ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &mut GameState,
    sfx: &mut MenuSfx,
    input: CoreInput,
) {
    if state.page == Some(ServicePage::CoreConfirm) {
        if input.back || input.control == Some(CoreControl::Cancel) {
            state.page = Some(ServicePage::MagicCore);
            state.pending = None;
            state.pending_id = None;
            sfx.cancel();
        } else if input.confirm || input.control == Some(CoreControl::Confirm) {
            // execute_pending checks fresh capacity; the domain commits atomically.
            let selected = state.selected;
            if execute_pending(state, catalog, game) {
                sfx.play(cue::BUY_SELL);
            } else {
                state.selected = selected;
                state.core_shop.focus = CoreFocus::Quantity;
                sfx.blocked();
            }
        }
        return;
    }
    let count = rows(catalog).len();
    if count == 0 {
        if input.back {
            state.close();
        }
        return;
    }
    state.selected = state.selected.min(count - 1);
    if input.back {
        if state.core_shop.focus == CoreFocus::Quantity {
            state.core_shop.focus = CoreFocus::Items;
        } else {
            state.close();
        }
        sfx.cancel();
        return;
    }
    match input.control {
        Some(CoreControl::Item(index)) if index < count => {
            select(state, catalog, index);
            sfx.hover();
            return;
        }
        Some(CoreControl::Mode(mode)) => {
            state.core_shop.mode = mode;
            state.core_shop.focus = CoreFocus::Quantity;
            state.quantity = 1;
            sfx.hover();
            return;
        }
        Some(CoreControl::Quantity(delta)) => {
            state.core_shop.focus = CoreFocus::Quantity;
            let max = selected(state, catalog).map_or(0, |item| {
                max_quantity(catalog, game, item.id(), state.core_shop.mode)
            });
            state.quantity = adjust_quantity(state.quantity, delta, max.max(1), false);
            sfx.hover();
            return;
        }
        Some(CoreControl::Review) => {
            state.core_shop.focus = CoreFocus::Quantity;
            review(state, catalog, game, sfx);
            return;
        }
        _ => {}
    }
    match state.core_shop.focus {
        CoreFocus::Items => {
            if let Some(delta) = input.vertical {
                select(
                    state,
                    catalog,
                    wrapped_or_zero(state.selected, count, delta),
                );
                sfx.hover();
            }
            if input.confirm || input.horizontal == Some(1) {
                state.core_shop.focus = CoreFocus::Quantity;
                state.quantity = 1;
                sfx.confirm();
            }
        }
        CoreFocus::Quantity => {
            if input.toggle || input.horizontal.is_some() {
                state.core_shop.mode = match input.horizontal {
                    Some(-1) => CoreMode::Sell,
                    Some(1) => CoreMode::RankUp,
                    _ => {
                        if state.core_shop.mode == CoreMode::Sell {
                            CoreMode::RankUp
                        } else {
                            CoreMode::Sell
                        }
                    }
                };
                state.quantity = 1;
                sfx.hover();
            } else if let Some(delta) = input.vertical {
                let max = selected(state, catalog).map_or(0, |item| {
                    max_quantity(catalog, game, item.id(), state.core_shop.mode)
                });
                state.quantity = adjust_quantity(state.quantity, -delta, max.max(1), false);
                sfx.hover();
            } else if input.confirm {
                review(state, catalog, game, sfx);
            }
        }
    }
}

#[cfg(test)]
mod tests;
