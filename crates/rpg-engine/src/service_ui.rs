//! Dialogue-routed shop, magic-core, inn, and apothecary overlays.
//!
//! This module owns the service state machine and its input; [`view`] draws it. The split
//! matches `field_menu`: the pages are node trees rebuilt from state, not a formatted string.

use bevy::{ecs::schedule::ApplyDeferred, prelude::*, ui::UiGlobalTransform};

use crate::{
    action_input::{ActionState, AppAction},
    app_state::AppState,
    dialogue_portrait::DialoguePortrait,
    engine_settings::EngineSettings,
    field_menu_domain::{
        FieldMenuCatalog, InventoryTab, can_equip, equip_item, item_matches_tab, item_name,
    },
    game_state::GameState,
    gameplay_canvas::pointer::CanvasPointer,
    scenario_dialogue::{DialogueActions, DialogueShopKind},
    scenario_item::ItemDefinition,
    scenario_map::ShopMetadata,
    service_domain::{
        RecipeAvailability, buy, can_sell, craft, exchange_magic_core, recipe_availability,
        rest_at_inn, sell, visible_stock,
    },
    sfx_cue::{MenuSfx, PlaySfx, cue},
};

mod apothecary;
#[cfg(test)]
mod shop_tests;
mod sprites;
mod view;

use apothecary::{ApothecaryControl, ApothecaryInput, handle_apothecary_input};
use sprites::{ServiceSprites, load_service_sprites};

pub(crate) struct ServiceUiPlugin;

impl Plugin for ServiceUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PlaySfx>()
            .init_resource::<ServiceUiState>()
            .init_resource::<ServiceSprites>()
            .add_systems(Startup, load_service_sprites)
            .add_systems(OnEnter(AppState::World), reset_service_ui)
            .add_systems(
                Update,
                (
                    handle_service_input,
                    view::sync_service_overlay,
                    ApplyDeferred,
                )
                    .chain()
                    .run_if(in_state(AppState::World)),
            )
            .add_systems(OnExit(AppState::World), view::cleanup_service_ui);
    }
}

/// Rows a service list shows before it scrolls, matching the source `VISIBLE_ROWS`.
const SERVICE_VISIBLE_ROWS: usize = 7;

/// Bulk quantity step for the redesigned shop's detail controls.
const SHOP_QUANTITY_STEP: u32 = 10;

/// Coarse quantity step for the magic-core exchange (`magic_core_shop_scene.py:25`).
const CORE_QUANTITY_STEP: u32 = 10;

/// Exchange rate at or above which a core stack asks for confirmation.
const HIGH_VALUE_CORE_RATE: u32 = 1_000;

/// Seconds a result toast stays up before it fades on its own.
const TOAST_SECONDS: f32 = 3.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ServiceRequest {
    Shop(DialogueShopKind),
    Inn,
    Apothecary,
}

impl ServiceRequest {
    pub(crate) fn from_dialogue(actions: &DialogueActions) -> Option<Self> {
        actions
            .open_shop
            .map(Self::Shop)
            .or(actions.open_inn.map(|_| Self::Inn))
            .or(actions.open_apothecary.map(|_| Self::Apothecary))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ServicePage {
    Buy,
    Sell,
    MagicCore,
    Inn,
    Apothecary,
    /// The apothecary's second stage: explicit confirmation of the visible recipe.
    RecipeDetail,
    Quantity,
    CoreConfirm,
    ShopConfirm,
    /// The post-purchase party picker offered after buying wearable gear.
    Equip,
    Result,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PendingTransaction {
    Buy,
    Sell,
    Exchange,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum ShopFocus {
    #[default]
    Mode,
    Category,
    Items,
    Detail,
}

#[derive(Clone, Copy, Component, Debug)]
pub(crate) enum ShopControl {
    Quantity(isize),
    DismissError,
}

impl PendingTransaction {
    const fn origin(self) -> ServicePage {
        match self {
            Self::Buy => ServicePage::Buy,
            Self::Sell => ServicePage::Sell,
            Self::Exchange => ServicePage::MagicCore,
        }
    }

    const fn quantity_step(self) -> u32 {
        match self {
            Self::Buy | Self::Sell => SHOP_QUANTITY_STEP,
            Self::Exchange => CORE_QUANTITY_STEP,
        }
    }

    /// The core exchange wraps its quantity cursor; the item shop clamps (`quantity_picker.py`).
    const fn wraps_quantity(self) -> bool {
        matches!(self, Self::Exchange)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ToastTone {
    Good,
    Warn,
}

#[derive(Clone, Debug)]
struct Toast {
    text: String,
    tone: ToastTone,
    remaining: f32,
}

#[derive(Debug, Default, Resource)]
pub(crate) struct ServiceUiState {
    request: Option<ServiceRequest>,
    /// The face of the NPC whose dialogue opened this service, so the header shows the clerk
    /// the player actually spoke to. `None` falls back to the manifest's per-service face.
    keeper_portrait: Option<DialoguePortrait>,
    page: Option<ServicePage>,
    selected: usize,
    quantity: u32,
    pending_id: Option<String>,
    pending: Option<PendingTransaction>,
    /// Which party member the buy-page stat preview and the equip prompt are focused on.
    equip_selection: usize,
    /// The item a completed purchase is offering to equip.
    pending_equip_id: Option<String>,
    /// Sell-side tag filter; `None` shows every sellable stack.
    sell_tag: Option<String>,
    shop_focus: ShopFocus,
    shop_category: InventoryTab,
    /// The apothecary keeps actual catalog indices while filtering its visible rows.
    ready_recipes_only: bool,
    toast: Option<Toast>,
    shop_error: Option<String>,
    suppress_confirm: bool,
    /// Set for the rest of the frame a Back press closes the service, so the same press
    /// cannot fall through to the field menu.
    closed_this_frame: bool,
    /// Where each of this screen's lists was last scrolled.
    scroll: crate::menu_chrome::ScrollMemory,
}

impl ServiceUiState {
    pub(crate) const fn input_locked(&self) -> bool {
        self.request.is_some() || self.closed_this_frame
    }

    /// Whether a service overlay is up, ignoring the one-frame close latch.
    #[cfg(test)]
    const fn is_open(&self) -> bool {
        self.request.is_some()
    }

    /// Opens the service, with the speaking NPC's face in the header when one is given.
    pub(crate) fn open(
        &mut self,
        request: ServiceRequest,
        keeper_portrait: Option<DialoguePortrait>,
    ) {
        *self = Self {
            request: Some(request),
            keeper_portrait,
            page: Some(match request {
                ServiceRequest::Shop(DialogueShopKind::MagicCore) => ServicePage::MagicCore,
                ServiceRequest::Shop(_) => ServicePage::Buy,
                ServiceRequest::Inn => ServicePage::Inn,
                ServiceRequest::Apothecary => ServicePage::Apothecary,
            }),
            quantity: 1,
            suppress_confirm: true,
            ..default()
        };
    }

    fn shop_stock_visible(&self) -> bool {
        self.shop_focus != ShopFocus::Mode
    }

    fn shop_item_selected(&self) -> bool {
        matches!(self.shop_focus, ShopFocus::Items | ShopFocus::Detail)
    }

    fn close(&mut self) {
        *self = Self {
            closed_this_frame: true,
            ..Self::default()
        };
    }

    fn announce(&mut self, text: impl Into<String>, tone: ToastTone) {
        let text = text.into();
        if tone == ToastTone::Warn
            && matches!(
                self.request,
                Some(ServiceRequest::Shop(
                    DialogueShopKind::Item | DialogueShopKind::Weapon | DialogueShopKind::Armor
                ))
            )
        {
            self.shop_error = Some(text);
            self.toast = None;
            return;
        }
        self.toast = Some(Toast {
            text,
            tone,
            remaining: TOAST_SECONDS,
        });
    }

    /// Returns to a list page with the cursor and pending transaction cleared.
    fn return_to(&mut self, page: ServicePage) {
        self.page = Some(page);
        self.selected = 0;
        self.quantity = 1;
        self.pending_id = None;
        self.pending = None;
    }
}

fn reset_service_ui(mut state: ResMut<ServiceUiState>) {
    state.close();
}

#[expect(
    clippy::too_many_lines,
    clippy::too_many_arguments,
    reason = "the service state machine reads as one table of page transitions over live state"
)]
pub(crate) fn handle_service_input(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    actions: Res<ActionState>,
    catalog: Res<FieldMenuCatalog>,
    settings: Res<EngineSettings>,
    game: Option<ResMut<GameState>>,
    mut state: ResMut<ServiceUiState>,
    mut menu_sfx: MenuSfx,
    mut pointer: Option<ResMut<CanvasPointer>>,
    controls: Query<(&ApothecaryControl, &ComputedNode, &UiGlobalTransform)>,
    shop_controls: Query<(&ShopControl, &ComputedNode, &UiGlobalTransform)>,
) {
    // The latch belongs to the frame that set it; releasing it here keeps the field menu
    // from reacting to a Back press this service already consumed.
    state.closed_this_frame = false;
    if !state.input_locked() {
        return;
    }
    expire_toast(&mut state, time.delta_secs());
    if state.suppress_confirm {
        state.suppress_confirm = false;
        return;
    }
    let Some(mut game) = game else { return };
    if any_input(&keys, &actions) {
        state.toast = None;
    }
    let delta = actions.menu_navigation();
    if delta.is_some() && !matches!(state.page, Some(ServicePage::Buy | ServicePage::Sell)) {
        menu_sfx.hover();
    }
    let confirm = actions.just_pressed(AppAction::Confirm);
    let back = actions.just_pressed(AppAction::Back);

    let apothecary_control = if matches!(
        state.page,
        Some(ServicePage::Apothecary | ServicePage::RecipeDetail)
    ) {
        pointer
            .as_deref()
            .filter(|p| p.just_clicked())
            .and_then(|p| {
                controls
                    .iter()
                    .find(|(_, node, transform)| p.is_over(node, transform))
                    .map(|(control, _, _)| *control)
            })
    } else {
        None
    };
    let shop_control = if state.shop_error.is_some()
        || (matches!(state.page, Some(ServicePage::Buy | ServicePage::Sell))
            && state.shop_item_selected())
    {
        pointer
            .as_deref()
            .filter(|p| p.just_clicked())
            .and_then(|p| {
                shop_controls
                    .iter()
                    .find(|(control, node, transform)| {
                        matches!(control, ShopControl::DismissError) == state.shop_error.is_some()
                            && p.is_over(node, transform)
                    })
                    .map(|(control, _, _)| *control)
            })
    } else {
        None
    };
    if apothecary_control.is_some() || shop_control.is_some() {
        state.toast = None;
        if let Some(pointer) = pointer.as_deref_mut() {
            pointer.consume();
        }
    }

    if state.shop_error.is_some() {
        if confirm || back || matches!(shop_control, Some(ShopControl::DismissError)) {
            state.shop_error = None;
            menu_sfx.cancel();
        }
        return;
    }

    match state.page {
        Some(ServicePage::Buy | ServicePage::Sell) => {
            let is_buy = state.page == Some(ServicePage::Buy);
            let horizontal = actions.menu_navigation_horizontal();
            let categories = shop_categories(&state, is_buy, &catalog, &game);
            if !categories.contains(&state.shop_category) {
                state.shop_category = InventoryTab::All;
                state.selected = 0;
                state.quantity = 1;
            }
            if back {
                menu_sfx.cancel();
                match state.shop_focus {
                    ShopFocus::Detail => state.shop_focus = ShopFocus::Items,
                    ShopFocus::Items => state.shop_focus = ShopFocus::Category,
                    ShopFocus::Category => state.shop_focus = ShopFocus::Mode,
                    ShopFocus::Mode => state.close(),
                }
                state.quantity = 1;
                return;
            }
            if let Some(ShopControl::Quantity(step)) = shop_control {
                state.shop_focus = ShopFocus::Detail;
                state.quantity = adjust_quantity(
                    state.quantity,
                    step,
                    pending_max_for_selected(&state, is_buy, &catalog, &game),
                    false,
                );
                menu_sfx.hover();
                return;
            }
            if !is_buy && keys.just_pressed(KeyCode::KeyT) {
                cycle_sell_tag(&mut state, &catalog, &game);
                state.quantity = 1;
                menu_sfx.hover();
                return;
            }
            state.quantity = if state.shop_focus == ShopFocus::Detail {
                state
                    .quantity
                    .clamp(1, pending_max_for_selected(&state, is_buy, &catalog, &game))
            } else {
                1
            };
            match state.shop_focus {
                ShopFocus::Mode => {
                    if let Some(step) = horizontal {
                        state.page = Some(if step < 0 {
                            ServicePage::Buy
                        } else {
                            ServicePage::Sell
                        });
                        state.selected = 0;
                        state.shop_category = InventoryTab::All;
                        state.sell_tag = None;
                        menu_sfx.hover();
                    }
                    if delta == Some(1) {
                        state.shop_category = InventoryTab::All;
                        state.selected = 0;
                        state.shop_focus = ShopFocus::Category;
                        menu_sfx.hover();
                    }
                }
                ShopFocus::Category => {
                    if delta == Some(-1) {
                        state.shop_focus = ShopFocus::Mode;
                        menu_sfx.hover();
                        return;
                    }
                    if let Some(step) = horizontal {
                        let index = categories
                            .iter()
                            .position(|tab| *tab == state.shop_category)
                            .unwrap_or(0);
                        state.shop_category = categories[wrapped(index, categories.len(), step)];
                        state.selected = 0;
                        menu_sfx.hover();
                    }
                    if delta == Some(1) {
                        state.selected = 0;
                        if selected_shop_item_id(&state, is_buy, &catalog, &game).is_some() {
                            state.shop_focus = ShopFocus::Items;
                            menu_sfx.hover();
                        } else {
                            menu_sfx.blocked();
                        }
                    }
                }
                ShopFocus::Items => {
                    let row_count = if is_buy {
                        buy_rows(&state, &catalog, &game).len()
                    } else {
                        sell_rows(&state, &catalog, &game).len()
                    };
                    if delta == Some(-1) && state.selected == 0 {
                        state.shop_focus = ShopFocus::Category;
                        menu_sfx.hover();
                        return;
                    } else if let Some(delta) = delta {
                        state.selected = wrapped_or_zero(state.selected, row_count, delta);
                        menu_sfx.hover();
                    }
                    if horizontal == Some(1) {
                        state.shop_focus = ShopFocus::Detail;
                        menu_sfx.hover();
                    }
                    if confirm {
                        let Some(id) = selected_shop_item_id(&state, is_buy, &catalog, &game)
                        else {
                            menu_sfx.blocked();
                            state.announce(
                                if is_buy {
                                    "No stock in this category."
                                } else {
                                    "Nothing to sell in this category."
                                },
                                ToastTone::Warn,
                            );
                            return;
                        };
                        state.pending_id = Some(id.clone());
                        state.pending = Some(if is_buy {
                            PendingTransaction::Buy
                        } else {
                            PendingTransaction::Sell
                        });
                        state.quantity = 1;
                        let max = pending_max(&state, &catalog, &game);
                        if max == 0 {
                            state.pending_id = None;
                            state.pending = None;
                            menu_sfx.blocked();
                            state.announce(
                                if is_buy {
                                    "Not enough GP or item capacity."
                                } else {
                                    "That item cannot be sold."
                                },
                                ToastTone::Warn,
                            );
                        } else {
                            state.page = Some(ServicePage::ShopConfirm);
                            menu_sfx.confirm();
                        }
                    }
                }
                ShopFocus::Detail => {
                    if delta == Some(-1) {
                        state.quantity = adjust_quantity(
                            state.quantity,
                            1,
                            pending_max_for_selected(&state, is_buy, &catalog, &game),
                            false,
                        );
                        menu_sfx.hover();
                    } else if delta == Some(1) {
                        state.quantity = adjust_quantity(
                            state.quantity,
                            -1,
                            pending_max_for_selected(&state, is_buy, &catalog, &game),
                            false,
                        );
                        menu_sfx.hover();
                    }
                    if let Some(step) = horizontal {
                        state.quantity = adjust_quantity(
                            state.quantity,
                            step * SHOP_QUANTITY_STEP as isize,
                            pending_max_for_selected(&state, is_buy, &catalog, &game),
                            false,
                        );
                        menu_sfx.hover();
                    }
                    if confirm
                        && let Some(id) = selected_shop_item_id(&state, is_buy, &catalog, &game)
                    {
                        state.pending_id = Some(id);
                        state.pending = Some(if is_buy {
                            PendingTransaction::Buy
                        } else {
                            PendingTransaction::Sell
                        });
                        let max = pending_max(&state, &catalog, &game);
                        if max == 0 {
                            state.pending_id = None;
                            state.pending = None;
                            menu_sfx.blocked();
                            state.announce(
                                if is_buy {
                                    "Not enough GP or item capacity."
                                } else {
                                    "That item cannot be sold."
                                },
                                ToastTone::Warn,
                            );
                        } else {
                            state.quantity = state.quantity.min(max);
                            state.page = Some(ServicePage::ShopConfirm);
                            menu_sfx.confirm();
                        }
                    }
                }
            }
        }
        Some(ServicePage::ShopConfirm) => {
            if back {
                menu_sfx.cancel();
                state.page = Some(
                    state
                        .pending
                        .map_or(ServicePage::Buy, PendingTransaction::origin),
                );
                state.pending_id = None;
                state.pending = None;
                state.quantity = 1;
                return;
            }
            if confirm {
                menu_sfx.confirm();
                if execute_pending(&mut state, &catalog, &mut game) {
                    menu_sfx.play(cue::BUY_SELL);
                } else {
                    menu_sfx.blocked();
                }
            }
        }
        Some(ServicePage::MagicCore) => {
            if back {
                menu_sfx.cancel();
                state.close();
                return;
            }
            let rows = owned_exchangeable_cores(&catalog, &game);
            if let Some(delta) = delta {
                state.selected = wrapped_or_zero(state.selected, rows.len(), delta);
            }
            if confirm {
                let Some(item) = rows.get(state.selected) else {
                    menu_sfx.blocked();
                    state.announce("No magic cores to exchange.", ToastTone::Warn);
                    return;
                };
                menu_sfx.confirm();
                state.pending_id = Some(item.id().to_owned());
                state.pending = Some(PendingTransaction::Exchange);
                state.page = Some(ServicePage::Quantity);
                state.quantity = 1;
            }
        }
        Some(ServicePage::Inn) => {
            if back || keys.just_pressed(KeyCode::KeyN) {
                menu_sfx.cancel();
                state.close();
                return;
            }
            if keys.just_pressed(KeyCode::KeyY) || confirm {
                let Some(cost) = active_inn_cost(&catalog, &game) else {
                    menu_sfx.blocked();
                    state.announce("This map has no inn service.", ToastTone::Warn);
                    return;
                };
                let (repository, party) = game.repository_and_party_mut();
                match rest_at_inn(repository, party, cost) {
                    Ok(()) => {
                        menu_sfx.confirm();
                        state.page = Some(ServicePage::Result);
                        state.announce("The party rested and recovered!", ToastTone::Good);
                    }
                    Err(error) => {
                        menu_sfx.blocked();
                        state.announce(error.to_string(), ToastTone::Warn);
                    }
                }
            }
        }
        Some(ServicePage::Apothecary | ServicePage::RecipeDetail) => {
            let filter = match apothecary_control {
                Some(ApothecaryControl::Filter(ready)) => Some(ready),
                _ if keys.just_pressed(KeyCode::Tab) => Some(!state.ready_recipes_only),
                _ => actions.menu_navigation_horizontal().map(|step| step > 0),
            };
            handle_apothecary_input(
                &mut state,
                &catalog,
                &mut game,
                &mut menu_sfx,
                ApothecaryInput {
                    delta,
                    filter,
                    recipe: match apothecary_control {
                        Some(ApothecaryControl::Recipe(index)) => Some(index),
                        _ => None,
                    },
                    confirm: confirm || apothecary_control == Some(ApothecaryControl::Brew),
                    back,
                },
            );
        }
        Some(ServicePage::Quantity) => {
            if back {
                menu_sfx.cancel();
                let origin = state
                    .pending
                    .map_or(ServicePage::Buy, |pending| pending.origin());
                state.return_to(origin);
                return;
            }
            let max = pending_max(&state, &catalog, &game).max(1);
            let step = state.pending.map_or(1, PendingTransaction::quantity_step);
            let wraps = state
                .pending
                .is_some_and(PendingTransaction::wraps_quantity);
            if let Some(delta) = delta {
                state.quantity =
                    adjust_quantity(state.quantity, coarse_step(delta, step), max, wraps);
            }
            if let Some(delta) = horizontal_step(&keys) {
                state.quantity = adjust_quantity(state.quantity, delta, max, wraps);
                menu_sfx.hover();
            }
            if confirm {
                menu_sfx.confirm();
                if settings.mc_exchange_confirm_large && is_high_value_core(&state, &catalog) {
                    state.page = Some(ServicePage::CoreConfirm);
                } else {
                    if execute_pending(&mut state, &catalog, &mut game) {
                        menu_sfx.play(cue::BUY_SELL);
                    } else {
                        menu_sfx.blocked();
                    }
                }
            }
        }
        Some(ServicePage::CoreConfirm) => {
            if back || keys.just_pressed(KeyCode::KeyN) {
                menu_sfx.cancel();
                state.page = Some(ServicePage::Quantity);
                return;
            }
            if keys.just_pressed(KeyCode::KeyY) || confirm {
                menu_sfx.confirm();
                if execute_pending(&mut state, &catalog, &mut game) {
                    menu_sfx.play(cue::BUY_SELL);
                } else {
                    menu_sfx.blocked();
                }
            }
        }
        Some(ServicePage::Equip) => {
            let member_count = game.party().members().count();
            if back {
                menu_sfx.cancel();
                state.pending_equip_id = None;
                state.return_to(ServicePage::Buy);
                return;
            }
            if let Some(delta) = delta {
                state.equip_selection = wrapped_or_zero(state.equip_selection, member_count, delta);
            }
            if confirm {
                equip_pending_purchase(&mut state, &catalog, &mut game, &mut menu_sfx);
            }
        }
        Some(ServicePage::Result) if confirm || back => {
            menu_sfx.cancel();
            state.close();
        }
        Some(ServicePage::Result) | None => {}
    }
}

fn expire_toast(state: &mut ServiceUiState, delta_seconds: f32) {
    let Some(toast) = state.toast.as_mut() else {
        return;
    };
    toast.remaining -= delta_seconds;
    if toast.remaining <= 0.0 {
        state.toast = None;
    }
}

/// Reports whether the player pressed anything the overlay reacts to this frame.
///
/// Any such press dismisses a standing toast, so a result banner never outlives the moment the
/// player moved on from it.
fn any_input(keys: &ButtonInput<KeyCode>, actions: &ActionState) -> bool {
    actions.just_pressed(AppAction::Confirm)
        || actions.just_pressed(AppAction::Back)
        || actions.menu_navigation().is_some()
        || actions.menu_navigation_horizontal().is_some()
        || horizontal_step(keys).is_some()
        || keys.any_just_pressed([KeyCode::Tab, KeyCode::KeyT, KeyCode::KeyY, KeyCode::KeyN])
}

/// Left/Right as a single signed step, with Left winning a same-frame tie.
///
/// The legacy quantity picker reads physical keys; the redesigned shop uses the shared
/// horizontal actions so rebinding, gamepads, and recorded input work there too.
fn horizontal_step(keys: &ButtonInput<KeyCode>) -> Option<isize> {
    if keys.just_pressed(KeyCode::ArrowLeft) {
        Some(-1)
    } else if keys.just_pressed(KeyCode::ArrowRight) {
        Some(1)
    } else {
        None
    }
}

/// Turns a menu-navigation delta into a coarse quantity step, with Up raising the count.
///
/// `menu_navigation` reports Up as `-1` because it indexes list rows downward, so the sign is
/// flipped here. The two source scenes disagree on this: the item shop binds Up to
/// `increase_large` (`item_shop_scene.py:277-282`) while the magic-core shop binds Up to
/// `decrease_large` (`magic_core_shop_scene.py:130-135`). That contradiction is a source bug —
/// the same key raises the quantity at one counter and lowers it at the next — so the port
/// settles it the way `menu_navigation` already settles simultaneous Up/Down: one rule
/// everywhere, and it is the item shop's, which is both the more-used screen and the reading
/// that matches "up means more".
const fn coarse_step(delta: isize, step: u32) -> isize {
    -delta * step as isize
}

/// Applies one quantity step, clamping to `1..=max` or wrapping past both ends.
fn adjust_quantity(current: u32, delta: isize, max: u32, wraps: bool) -> u32 {
    let target = current as isize + delta;
    if wraps {
        if target < 1 {
            max
        } else if target > max as isize {
            1
        } else {
            target as u32
        }
    } else {
        target.clamp(1, max as isize) as u32
    }
}

fn is_equipment(item: &ItemDefinition) -> bool {
    matches!(
        item,
        ItemDefinition::Weapon(_)
            | ItemDefinition::Shield(_)
            | ItemDefinition::Helmet(_)
            | ItemDefinition::Body(_)
            | ItemDefinition::Accessory(_)
    )
}

fn is_high_value_core(state: &ServiceUiState, catalog: &FieldMenuCatalog) -> bool {
    state.pending == Some(PendingTransaction::Exchange)
        && state
            .pending_id
            .as_deref()
            .and_then(|id| catalog.item(id))
            .is_some_and(|item| {
                matches!(item, ItemDefinition::MagicCore(core)
                    if core.exchange_rate.is_some_and(|rate|
                        rate.get() >= HIGH_VALUE_CORE_RATE))
            })
}

/// Cycles the sell filter through `All → tag → … → All` (`item_shop_scene.py:121-133`).
fn cycle_sell_tag(state: &mut ServiceUiState, catalog: &FieldMenuCatalog, game: &GameState) {
    let tags = sellable_tags(catalog, game);
    if tags.is_empty() {
        state.sell_tag = None;
        state.selected = 0;
        return;
    }
    let position = state
        .sell_tag
        .as_ref()
        .and_then(|current| tags.iter().position(|tag| tag == current))
        .map_or(0, |index| index + 1);
    state.sell_tag = tags.get(position % (tags.len() + 1)).cloned();
    state.selected = 0;
}

/// Every distinct tag across the stacks the player could sell, sorted.
fn sellable_tags(catalog: &FieldMenuCatalog, game: &GameState) -> Vec<String> {
    let mut tags = catalog
        .ordered_items()
        .filter(|item| can_sell(game.repository(), item))
        .flat_map(|item| game.repository().item_tags(item.id()))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    tags.sort_unstable();
    tags.dedup();
    tags
}

/// Commits the staged shop transaction, reporting whether it went through so the caller — which
/// is the system that owns the audio writer — can sound the right cue.
fn execute_pending(
    state: &mut ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &mut GameState,
) -> bool {
    let Some(id) = state.pending_id.clone() else {
        return false;
    };
    let quantity = state.quantity;
    let result = match state.pending {
        Some(PendingTransaction::Buy) => active_shop(state, catalog, game)
            .and_then(|shop| {
                visible_stock(shop, game.flags())
                    .into_iter()
                    .find(|row| row.id() == id)
            })
            .ok_or_else(|| "shop item is unavailable".to_owned())
            .and_then(|row| buy(game.repository_mut(), row, quantity).map_err(|e| e.to_string())),
        Some(PendingTransaction::Sell) => catalog
            .item(&id)
            .ok_or_else(|| "item metadata is unavailable".to_owned())
            .and_then(|item| {
                sell(game.repository_mut(), item, quantity).map_err(|e| e.to_string())
            }),
        Some(PendingTransaction::Exchange) => catalog
            .item(&id)
            .ok_or_else(|| "core metadata is unavailable".to_owned())
            .and_then(|item| {
                exchange_magic_core(game.repository_mut(), item, quantity)
                    .map_err(|e| e.to_string())
            }),
        None => return false,
    };
    let Some(pending) = state.pending else {
        return false;
    };
    let name = catalog
        .item(&id)
        .map_or_else(|| id.clone(), |item| item_name(item).to_owned());
    match result {
        Ok(total) => {
            let announcement = match pending {
                PendingTransaction::Buy => format!("Bought {quantity} x {name}"),
                PendingTransaction::Sell => format!("Sold {quantity} x {name} for {total} GP"),
                PendingTransaction::Exchange => {
                    format!("Exchanged {quantity} x {name}    +{total} GP")
                }
            };
            state.return_to(pending.origin());
            state.announce(announcement, ToastTone::Good);
            match pending {
                PendingTransaction::Buy => offer_equip_after_purchase(state, catalog, game, &id),
                // The source closes the exchange rather than showing an empty list once the
                // last core is spent (`magic_core_shop_scene.py:96-101`).
                PendingTransaction::Exchange
                    if owned_exchangeable_cores(catalog, game).is_empty() =>
                {
                    state.close();
                }
                _ => {}
            }
            true
        }
        Err(error) => {
            state.return_to(pending.origin());
            state.announce(error, ToastTone::Warn);
            false
        }
    }
}

/// Pushes the party picker when a purchase is wearable by somebody (`item_shop_scene.py:355-359`).
fn offer_equip_after_purchase(
    state: &mut ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
    item_id: &str,
) {
    let Some(item) = catalog.item(item_id) else {
        return;
    };
    if !is_equipment(item) {
        return;
    }
    let Some(index) = game
        .party()
        .members()
        .position(|member| can_equip(member, item, catalog).is_ok())
    else {
        return;
    };
    state.pending_equip_id = Some(item_id.to_owned());
    state.equip_selection = index;
    state.page = Some(ServicePage::Equip);
}

fn equip_pending_purchase(
    state: &mut ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &mut GameState,
    menu_sfx: &mut MenuSfx,
) {
    let Some(item_id) = state.pending_equip_id.clone() else {
        state.return_to(ServicePage::Buy);
        return;
    };
    let Some(member_id) = game
        .party()
        .members()
        .nth(state.equip_selection)
        .map(|member| member.id().to_owned())
    else {
        menu_sfx.blocked();
        return;
    };
    let member_name = game
        .party()
        .member(&member_id)
        .map_or_else(|| member_id.clone(), |member| member.name().to_owned());
    let item_name = catalog
        .item(&item_id)
        .map_or_else(|| item_id.clone(), |item| item_name(item).to_owned());
    match equip_item(game, catalog, &member_id, &item_id) {
        Ok(_displaced) => {
            menu_sfx.play(cue::EQUIP);
            state.pending_equip_id = None;
            state.return_to(ServicePage::Buy);
            state.announce(
                format!("Equipped {item_name} on {member_name}"),
                ToastTone::Good,
            );
        }
        Err(error) => {
            menu_sfx.blocked();
            state.announce(error.to_string(), ToastTone::Warn);
        }
    }
}

fn active_map<'a>(
    catalog: &'a FieldMenuCatalog,
    game: &GameState,
) -> Option<&'a crate::scenario_map::MapMetadata> {
    catalog.map(game.map().current()?.as_str())
}

fn active_shop<'a>(
    state: &ServiceUiState,
    catalog: &'a FieldMenuCatalog,
    game: &GameState,
) -> Option<&'a ShopMetadata> {
    let map = active_map(catalog, game)?;
    match state.request? {
        ServiceRequest::Shop(DialogueShopKind::Item) => map.shop.as_ref(),
        ServiceRequest::Shop(DialogueShopKind::Weapon) => map.weapon_shop.as_ref(),
        ServiceRequest::Shop(DialogueShopKind::Armor) => map.armor_shop.as_ref(),
        _ => None,
    }
}

fn active_inn_cost(catalog: &FieldMenuCatalog, game: &GameState) -> Option<u32> {
    Some(active_map(catalog, game)?.inn.as_ref()?.cost.get())
}

/// Only offer types represented by unlocked stock or carried sellable items.
fn shop_categories(
    state: &ServiceUiState,
    buying: bool,
    catalog: &FieldMenuCatalog,
    game: &GameState,
) -> Vec<InventoryTab> {
    let items = if buying {
        active_shop(state, catalog, game)
            .map(|shop| {
                visible_stock(shop, game.flags())
                    .into_iter()
                    .filter_map(|row| catalog.item(row.id()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    } else {
        owned_items(catalog, game)
            .into_iter()
            .filter(|item| can_sell(game.repository(), item))
            .collect()
    };
    InventoryTab::ALL
        .into_iter()
        .filter(|&tab| {
            tab == InventoryTab::All
                || items.iter().any(|item| {
                    item_matches_tab(item, game.repository().is_new_item(item.id()), tab)
                })
        })
        .collect()
}

fn buy_rows<'a>(
    state: &ServiceUiState,
    catalog: &'a FieldMenuCatalog,
    game: &GameState,
) -> Vec<&'a crate::scenario_map::ShopItem> {
    active_shop(state, catalog, game)
        .map(|shop| {
            visible_stock(shop, game.flags())
                .into_iter()
                .filter(|row| {
                    catalog.item(row.id()).is_some_and(|item| {
                        item_matches_tab(
                            item,
                            game.repository().is_new_item(row.id()),
                            state.shop_category,
                        )
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn owned_items<'a>(catalog: &'a FieldMenuCatalog, game: &GameState) -> Vec<&'a ItemDefinition> {
    catalog
        .ordered_items()
        .filter(|item| game.repository().item_count(item.id()) > 0)
        .collect()
}

/// Sellable stacks, narrowed by the active tag filter.
fn sell_rows<'a>(
    state: &ServiceUiState,
    catalog: &'a FieldMenuCatalog,
    game: &GameState,
) -> Vec<&'a ItemDefinition> {
    owned_items(catalog, game)
        .into_iter()
        .filter(|item| {
            can_sell(game.repository(), item)
                && item_matches_tab(
                    item,
                    game.repository().is_new_item(item.id()),
                    state.shop_category,
                )
                && state.sell_tag.as_ref().is_none_or(|tag| {
                    game.repository()
                        .item_tags(item.id())
                        .any(|owned| owned == tag)
                })
        })
        .collect()
}

fn pending_max_for_selected(
    state: &ServiceUiState,
    buying: bool,
    catalog: &FieldMenuCatalog,
    game: &GameState,
) -> u32 {
    let id = if buying {
        buy_rows(state, catalog, game)
            .get(state.selected)
            .map(|row| row.id())
    } else {
        sell_rows(state, catalog, game)
            .get(state.selected)
            .map(|item| item.id())
    };
    let Some(id) = id else { return 1 };
    if buying {
        let room = game
            .repository()
            .item_quantity_cap()
            .saturating_sub(game.repository().item_count(id));
        room.min(
            game.repository().gp()
                / active_shop(state, catalog, game)
                    .and_then(|shop| shop.items.iter().find(|row| row.id() == id))
                    .map_or(1, |row| row.buy_price().get()),
        )
        .max(1)
    } else {
        game.repository().item_count(id).max(1)
    }
}

fn selected_shop_item_id(
    state: &ServiceUiState,
    buying: bool,
    catalog: &FieldMenuCatalog,
    game: &GameState,
) -> Option<String> {
    if buying {
        buy_rows(state, catalog, game)
            .get(state.selected)
            .map(|row| row.id().to_owned())
    } else {
        sell_rows(state, catalog, game)
            .get(state.selected)
            .map(|item| item.id().to_owned())
    }
}

fn owned_exchangeable_cores<'a>(
    catalog: &'a FieldMenuCatalog,
    game: &GameState,
) -> Vec<&'a ItemDefinition> {
    owned_items(catalog, game)
        .into_iter()
        .filter(
            |item| matches!(item, ItemDefinition::MagicCore(core) if core.exchange_rate.is_some()),
        )
        .collect()
}

fn pending_max(state: &ServiceUiState, catalog: &FieldMenuCatalog, game: &GameState) -> u32 {
    let Some(id) = state.pending_id.as_deref() else {
        return 0;
    };
    match state.pending {
        Some(PendingTransaction::Buy) => active_shop(state, catalog, game)
            .and_then(|shop| shop.items.iter().find(|row| row.id() == id))
            .map(|row| {
                game.repository()
                    .item_quantity_cap()
                    .saturating_sub(game.repository().item_count(id))
                    .min(game.repository().gp() / row.buy_price().get())
            })
            .unwrap_or(0),
        Some(PendingTransaction::Sell | PendingTransaction::Exchange) => {
            game.repository().item_count(id)
        }
        None => 0,
    }
}

fn wrapped(current: usize, count: usize, delta: isize) -> usize {
    (current as isize + delta).rem_euclid(count as isize) as usize
}

fn wrapped_or_zero(current: usize, count: usize, delta: isize) -> usize {
    if count == 0 {
        0
    } else {
        wrapped(current.min(count - 1), count, delta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_menu_domain::tests::{catalog, game};

    /// The first catalog item some party member is actually allowed to wear.
    fn wearable_for_party(catalog: &FieldMenuCatalog, game: &GameState) -> String {
        catalog
            .ordered_items()
            .find(|item| {
                is_equipment(item)
                    && game
                        .party()
                        .members()
                        .any(|member| can_equip(member, item, catalog).is_ok())
            })
            .expect("the production catalog equips somebody in the starting party")
            .id()
            .to_owned()
    }

    #[test]
    fn routes_each_dialogue_service_action_without_conflating_shop_kinds() {
        for (yaml, expected) in [
            (
                "open_shop: item\n",
                ServiceRequest::Shop(DialogueShopKind::Item),
            ),
            (
                "open_shop: weapon\n",
                ServiceRequest::Shop(DialogueShopKind::Weapon),
            ),
            (
                "open_shop: armor\n",
                ServiceRequest::Shop(DialogueShopKind::Armor),
            ),
            (
                "open_shop: magic_core\n",
                ServiceRequest::Shop(DialogueShopKind::MagicCore),
            ),
            ("open_inn: true\n", ServiceRequest::Inn),
            ("open_apothecary: true\n", ServiceRequest::Apothecary),
        ] {
            let actions: DialogueActions = crate::scenario_yaml::from_str(yaml).unwrap();
            assert_eq!(ServiceRequest::from_dialogue(&actions), Some(expected));
        }
    }

    #[test]
    fn shop_quantity_clamps_at_both_ends_while_the_core_exchange_wraps() {
        // Item shop: coarse steps saturate rather than roll over (`quantity_picker.py:_adjust`).
        assert_eq!(adjust_quantity(1, -1, 9, false), 1);
        assert_eq!(adjust_quantity(1, 5, 9, false), 6);
        assert_eq!(adjust_quantity(6, 5, 9, false), 9);
        assert_eq!(adjust_quantity(9, 1, 9, false), 9);

        // Magic core: the same picker constructed with `loop=True`.
        assert_eq!(adjust_quantity(1, -1, 9, true), 9);
        assert_eq!(adjust_quantity(9, 1, 9, true), 1);
        assert_eq!(adjust_quantity(5, 10, 9, true), 1);
    }

    #[test]
    fn coarse_steps_match_the_two_source_pickers() {
        assert_eq!(PendingTransaction::Buy.quantity_step(), SHOP_QUANTITY_STEP);
        assert_eq!(PendingTransaction::Sell.quantity_step(), SHOP_QUANTITY_STEP);
        assert_eq!(
            PendingTransaction::Exchange.quantity_step(),
            CORE_QUANTITY_STEP
        );
        assert!(!PendingTransaction::Buy.wraps_quantity());
        assert!(PendingTransaction::Exchange.wraps_quantity());
    }

    #[test]
    fn up_raises_the_quantity_at_every_counter() {
        // `menu_navigation` reports Up as -1; the picker must still count upward, and it must
        // do so identically for the shop and the core exchange.
        for step in [SHOP_QUANTITY_STEP, CORE_QUANTITY_STEP] {
            assert_eq!(coarse_step(-1, step), step as isize, "Up adds {step}");
            assert_eq!(
                coarse_step(1, step),
                -(step as isize),
                "Down removes {step}"
            );
        }
        assert_eq!(
            adjust_quantity(1, coarse_step(-1, SHOP_QUANTITY_STEP), 80, false),
            11
        );
    }

    #[test]
    fn a_toast_expires_on_its_own_without_being_dismissed() {
        let mut state = ServiceUiState::default();
        state.announce("Bought 1 x Potion", ToastTone::Good);
        expire_toast(&mut state, TOAST_SECONDS / 2.0);
        assert!(state.toast.is_some());
        expire_toast(&mut state, TOAST_SECONDS);
        assert!(state.toast.is_none());
    }

    #[test]
    fn the_sell_filter_cycles_all_then_each_tag_and_narrows_the_rows() {
        let catalog = catalog();
        let mut game = game([]);
        let _ = game.repository_mut().add_item("potion", 2).unwrap();
        let _ = game.repository_mut().add_item("ether", 2).unwrap();
        game.repository_mut()
            .add_tags("potion", ["healing"])
            .unwrap();
        game.repository_mut().add_tags("ether", ["mana"]).unwrap();

        let mut state = ServiceUiState {
            page: Some(ServicePage::Sell),
            ..default()
        };
        assert_eq!(sell_rows(&state, &catalog, &game).len(), 2);

        // Sorted, so `healing` comes before `mana`, and the cycle returns to All.
        cycle_sell_tag(&mut state, &catalog, &game);
        assert_eq!(state.sell_tag.as_deref(), Some("healing"));
        let rows = sell_rows(&state, &catalog, &game);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id(), "potion");

        cycle_sell_tag(&mut state, &catalog, &game);
        assert_eq!(state.sell_tag.as_deref(), Some("mana"));
        assert_eq!(sell_rows(&state, &catalog, &game)[0].id(), "ether");

        cycle_sell_tag(&mut state, &catalog, &game);
        assert_eq!(state.sell_tag, None);
        assert_eq!(sell_rows(&state, &catalog, &game).len(), 2);
    }

    #[test]
    fn buying_wearable_gear_opens_the_party_picker_and_ordinary_goods_do_not() {
        let catalog = catalog();
        let game = game([]);
        let wearable = wearable_for_party(&catalog, &game);

        let mut state = ServiceUiState {
            page: Some(ServicePage::Buy),
            ..default()
        };
        offer_equip_after_purchase(&mut state, &catalog, &game, &wearable);
        assert_eq!(state.page, Some(ServicePage::Equip));
        assert_eq!(state.pending_equip_id.as_deref(), Some(wearable.as_str()));
        assert!(
            game.party()
                .members()
                .nth(state.equip_selection)
                .is_some_and(|member| can_equip(
                    member,
                    catalog.item(&wearable).unwrap(),
                    &catalog
                )
                .is_ok())
        );

        let mut state = ServiceUiState {
            page: Some(ServicePage::Buy),
            ..default()
        };
        offer_equip_after_purchase(&mut state, &catalog, &game, "potion");
        assert_eq!(state.page, Some(ServicePage::Buy));
        assert_eq!(state.pending_equip_id, None);
    }

    #[test]
    fn the_core_exchange_closes_itself_only_once_the_last_core_is_spent() {
        let catalog = catalog();

        let mut game = game([]);
        let _ = game.repository_mut().add_item("mc_xs", 2).unwrap();
        let mut state = ServiceUiState::default();
        state.open(ServiceRequest::Shop(DialogueShopKind::MagicCore), None);
        state.pending = Some(PendingTransaction::Exchange);
        state.pending_id = Some("mc_xs".to_owned());
        state.quantity = 1;
        let _committed = execute_pending(&mut state, &catalog, &mut game);
        assert!(state.is_open(), "one core is left, so the list stays up");
        assert_eq!(state.page, Some(ServicePage::MagicCore));

        execute_pending_last_core(&catalog);
    }

    #[test]
    fn core_exchange_lists_only_scenario_authored_gp_tiers() {
        let catalog = catalog();
        let mut game = game([]);
        for id in ["mc_xs", "mc_s", "mc_m", "mc_l", "mc_xl"] {
            let _outcome = game.repository_mut().add_item(id, 1).unwrap();
        }

        assert_eq!(
            owned_exchangeable_cores(&catalog, &game)
                .into_iter()
                .map(ItemDefinition::id)
                .collect::<Vec<_>>(),
            ["mc_s", "mc_xs"]
        );
    }

    fn execute_pending_last_core(catalog: &FieldMenuCatalog) {
        let mut game = game([]);
        let _ = game.repository_mut().add_item("mc_xs", 1).unwrap();
        let mut state = ServiceUiState::default();
        state.open(ServiceRequest::Shop(DialogueShopKind::MagicCore), None);
        state.pending = Some(PendingTransaction::Exchange);
        state.pending_id = Some("mc_xs".to_owned());
        state.quantity = 1;
        let _committed = execute_pending(&mut state, catalog, &mut game);
        assert!(
            !state.is_open(),
            "the source closes rather than showing an empty exchange"
        );
    }
}
