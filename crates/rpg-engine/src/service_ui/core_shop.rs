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
    Exchange,
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
    Back,
}

impl CoreControl {
    pub(super) fn active(self, state: &ServiceUiState) -> bool {
        if state.page == Some(ServicePage::CoreConfirm) {
            matches!(self, Self::Confirm | Self::Back)
        } else if state.core_shop.focus == CoreFocus::Quantity {
            matches!(self, Self::Quantity(_) | Self::Review | Self::Back)
        } else {
            matches!(self, Self::Item(_) | Self::Mode(_))
        }
    }
}

pub(super) struct CoreInput {
    pub(super) vertical: Option<isize>,
    pub(super) horizontal: Option<isize>,
    pub(super) confirm: bool,
    pub(super) back: bool,
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
    state.toast = None;
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

fn open_quantity(
    state: &mut ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
    sfx: &mut MenuSfx,
) {
    if let Some(reason) = unavailable(state, catalog, game) {
        state.announce(reason, ToastTone::Warn);
        sfx.blocked();
    } else {
        state.toast = None;
        state.core_shop.focus = CoreFocus::Quantity;
        sfx.confirm();
    }
}

pub(super) fn handle_input(
    state: &mut ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &mut GameState,
    sfx: &mut MenuSfx,
    input: CoreInput,
) {
    // Modal stages accept only their own pointer controls, including in headless callers.
    let control = input.control.filter(|control| control.active(state));
    if state.page == Some(ServicePage::CoreConfirm) {
        if input.back || control == Some(CoreControl::Back) {
            state.page = Some(ServicePage::MagicCore);
            state.pending = None;
            state.pending_id = None;
            state.core_shop.focus = CoreFocus::Quantity;
            sfx.cancel();
        } else if input.confirm || control == Some(CoreControl::Confirm) {
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
    if input.back || control == Some(CoreControl::Back) {
        match state.core_shop.focus {
            CoreFocus::Quantity => state.core_shop.focus = CoreFocus::Exchange,
            CoreFocus::Exchange => state.core_shop.focus = CoreFocus::Items,
            CoreFocus::Items => state.close(),
        }
        sfx.cancel();
        return;
    }
    match control {
        Some(CoreControl::Item(index)) if index < count => {
            select(state, catalog, index);
            sfx.hover();
            return;
        }
        Some(CoreControl::Mode(mode)) => {
            if state.core_shop.mode != mode {
                state.quantity = 1;
            }
            state.core_shop.mode = mode;
            state.core_shop.focus = CoreFocus::Exchange;
            open_quantity(state, catalog, game, sfx);
            return;
        }
        Some(CoreControl::Review) => {
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
            } else if input.confirm || input.horizontal == Some(1) {
                state.core_shop.focus = CoreFocus::Exchange;
                sfx.confirm();
            }
        }
        CoreFocus::Exchange => {
            if input.horizontal == Some(-1) {
                state.core_shop.focus = CoreFocus::Items;
                sfx.cancel();
            } else if input.vertical.is_some() {
                state.core_shop.mode = match state.core_shop.mode {
                    CoreMode::Sell => CoreMode::RankUp,
                    CoreMode::RankUp => CoreMode::Sell,
                };
                state.quantity = 1;
                state.toast = None;
                sfx.hover();
            } else if input.confirm {
                open_quantity(state, catalog, game, sfx);
            }
        }
        CoreFocus::Quantity => {
            let delta = match control {
                Some(CoreControl::Quantity(delta)) => Some(delta),
                _ => input
                    .vertical
                    .map(|delta| -delta)
                    .or_else(|| input.horizontal.map(|delta| delta * 10)),
            };
            if let Some(delta) = delta {
                let max = selected(state, catalog).map_or(0, |item| {
                    max_quantity(catalog, game, item.id(), state.core_shop.mode)
                });
                state.quantity = adjust_quantity(state.quantity, delta, max.max(1), false);
                sfx.hover();
            } else if input.confirm {
                review(state, catalog, game, sfx);
            }
        }
    }
}

#[cfg(test)]
mod tests;
