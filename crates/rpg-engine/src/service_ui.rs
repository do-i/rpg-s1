//! Dialogue-routed shop, magic-core, inn, and apothecary overlays.

use bevy::{ecs::schedule::ApplyDeferred, prelude::*};

use crate::{
    action_input::{ActionState, AppAction},
    app_state::AppState,
    field_menu_domain::{
        FieldMenuCatalog, can_equip, derived_stats, item_description, item_name, preview_stats,
    },
    game_state::GameState,
    scenario_dialogue::{DialogueActions, DialogueShopKind},
    scenario_inventory::ScenarioInventory,
    scenario_item::ItemDefinition,
    scenario_map::ShopMetadata,
    scenario_root::ScenarioRoot,
    service_domain::{
        RecipeAvailability, buy, can_sell, craft, exchange_magic_core, recipe_availability,
        rest_at_inn, sell, sell_price, visible_stock,
    },
    ui_theme::UiTheme,
};

pub(crate) struct ServiceUiPlugin;

impl Plugin for ServiceUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ServiceUiState>()
            .add_systems(OnEnter(AppState::World), reset_service_ui)
            .add_systems(
                Update,
                (handle_service_input, sync_service_overlay, ApplyDeferred)
                    .chain()
                    .run_if(in_state(AppState::World)),
            )
            .add_systems(OnExit(AppState::World), cleanup_service_ui);
    }
}

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
    ShopMenu,
    Buy,
    Sell,
    MagicCore,
    Inn,
    Apothecary,
    Quantity,
    CoreConfirm,
    Result,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PendingTransaction {
    Buy,
    Sell,
    Exchange,
}

#[derive(Debug, Default, Resource)]
pub(crate) struct ServiceUiState {
    request: Option<ServiceRequest>,
    page: Option<ServicePage>,
    selected: usize,
    quantity: u32,
    pending_id: Option<String>,
    pending: Option<PendingTransaction>,
    message: String,
    suppress_confirm: bool,
}

impl ServiceUiState {
    pub(crate) const fn input_locked(&self) -> bool {
        self.request.is_some()
    }

    pub(crate) fn open(&mut self, request: ServiceRequest) {
        *self = Self {
            request: Some(request),
            page: Some(match request {
                ServiceRequest::Shop(DialogueShopKind::MagicCore) => ServicePage::MagicCore,
                ServiceRequest::Shop(_) => ServicePage::ShopMenu,
                ServiceRequest::Inn => ServicePage::Inn,
                ServiceRequest::Apothecary => ServicePage::Apothecary,
            }),
            quantity: 1,
            suppress_confirm: true,
            ..default()
        };
    }

    fn close(&mut self) {
        *self = Self::default();
    }
}

#[derive(Component)]
struct ServiceRoot;
#[derive(Component)]
struct ServiceTitle;
#[derive(Component)]
struct ServiceBody;
#[derive(Component)]
struct ServiceHint;

fn reset_service_ui(mut state: ResMut<ServiceUiState>) {
    state.close();
}

fn handle_service_input(
    keys: Res<ButtonInput<KeyCode>>,
    actions: Res<ActionState>,
    catalog: Res<FieldMenuCatalog>,
    game: Option<ResMut<GameState>>,
    mut state: ResMut<ServiceUiState>,
) {
    if !state.input_locked() {
        return;
    }
    if state.suppress_confirm {
        state.suppress_confirm = false;
        return;
    }
    let Some(mut game) = game else { return };
    if actions.just_pressed(AppAction::Back) {
        match state.page {
            Some(ServicePage::Buy | ServicePage::Sell) => {
                state.page = Some(ServicePage::ShopMenu);
                state.selected = 0;
                state.message.clear();
            }
            Some(ServicePage::Quantity) => {
                state.page = Some(match state.pending {
                    Some(PendingTransaction::Buy) => ServicePage::Buy,
                    Some(PendingTransaction::Sell) => ServicePage::Sell,
                    Some(PendingTransaction::Exchange) => ServicePage::MagicCore,
                    None => ServicePage::ShopMenu,
                });
                state.selected = 0;
                state.quantity = 1;
                state.pending_id = None;
                state.pending = None;
                state.message.clear();
            }
            Some(ServicePage::CoreConfirm) => {
                state.page = Some(ServicePage::Quantity);
                state.message.clear();
            }
            _ => state.close(),
        }
        return;
    }
    let delta = actions.menu_navigation();
    match state.page {
        Some(ServicePage::ShopMenu) => {
            if let Some(delta) = delta {
                state.selected = wrapped(state.selected, 2, delta);
            }
            if actions.just_pressed(AppAction::Confirm) {
                state.page = Some(if state.selected == 0 {
                    ServicePage::Buy
                } else {
                    ServicePage::Sell
                });
                state.selected = 0;
                state.message.clear();
            }
        }
        Some(ServicePage::Buy) => {
            let rows = active_shop(&state, &catalog, &game)
                .map(|shop| visible_stock(shop, game.flags()))
                .unwrap_or_default();
            if let Some(delta) = delta {
                state.selected = wrapped_or_zero(state.selected, rows.len(), delta);
            }
            if actions.just_pressed(AppAction::Confirm) {
                let Some(row) = rows.get(state.selected) else {
                    state.message = "No stock is currently unlocked.".to_owned();
                    return;
                };
                let room = game
                    .repository()
                    .item_quantity_cap()
                    .saturating_sub(game.repository().item_count(row.id()));
                let affordable = game.repository().gp() / row.buy_price().get();
                if room.min(affordable) == 0 {
                    state.message = if affordable == 0 {
                        "Not enough GP.".to_owned()
                    } else {
                        "Item quantity cap reached.".to_owned()
                    };
                } else {
                    state.pending_id = Some(row.id().to_owned());
                    state.pending = Some(PendingTransaction::Buy);
                    state.page = Some(ServicePage::Quantity);
                    state.quantity = 1;
                }
            }
        }
        Some(ServicePage::Sell) => {
            let rows = owned_items(&catalog, &game);
            if let Some(delta) = delta {
                state.selected = wrapped_or_zero(state.selected, rows.len(), delta);
            }
            if actions.just_pressed(AppAction::Confirm) {
                let Some(item) = rows.get(state.selected) else {
                    state.message = "There is nothing to sell.".to_owned();
                    return;
                };
                if !can_sell(game.repository(), item) {
                    state.message = "That item is locked or has no sale value.".to_owned();
                } else {
                    state.pending_id = Some(item.id().to_owned());
                    state.pending = Some(PendingTransaction::Sell);
                    state.page = Some(ServicePage::Quantity);
                    state.quantity = 1;
                }
            }
        }
        Some(ServicePage::MagicCore) => {
            let rows = owned_cores(&catalog, &game);
            if let Some(delta) = delta {
                state.selected = wrapped_or_zero(state.selected, rows.len(), delta);
            }
            if actions.just_pressed(AppAction::Confirm) {
                let Some(item) = rows.get(state.selected) else {
                    state.message = "No magic cores to exchange.".to_owned();
                    return;
                };
                state.pending_id = Some(item.id().to_owned());
                state.pending = Some(PendingTransaction::Exchange);
                state.page = Some(ServicePage::Quantity);
                state.quantity = 1;
            }
        }
        Some(ServicePage::Inn) => {
            if keys.just_pressed(KeyCode::KeyN) {
                state.close();
            } else if keys.just_pressed(KeyCode::KeyY) || actions.just_pressed(AppAction::Confirm) {
                let Some(cost) = active_inn_cost(&catalog, &game) else {
                    state.message = "This map has no inn service.".to_owned();
                    return;
                };
                let (repository, party) = game.repository_and_party_mut();
                match rest_at_inn(repository, party, cost) {
                    Ok(()) => {
                        state.page = Some(ServicePage::Result);
                        state.message = "The party is fully rested.".to_owned();
                    }
                    Err(error) => state.message = error.to_string(),
                }
            }
        }
        Some(ServicePage::Apothecary) => {
            let recipes = catalog.recipes();
            if let Some(delta) = delta {
                state.selected = wrapped_or_zero(state.selected, recipes.len(), delta);
            }
            if actions.just_pressed(AppAction::Confirm) {
                let Some(recipe) = recipes.get(state.selected) else {
                    return;
                };
                let flags = game.flags().clone();
                match craft(game.repository_mut(), &flags, recipe) {
                    Ok(()) => state.message = format!("Crafted {}.", recipe.scroll_name),
                    Err(error) => state.message = error.to_string(),
                }
            }
        }
        Some(ServicePage::Quantity) => {
            let max = pending_max(&state, &catalog, &game).max(1);
            if let Some(delta) = delta {
                state.quantity = wrapped_quantity(state.quantity, max, delta);
            }
            if keys.just_pressed(KeyCode::ArrowLeft) {
                state.quantity = 1;
            } else if keys.just_pressed(KeyCode::ArrowRight) {
                state.quantity = max;
            }
            if actions.just_pressed(AppAction::Confirm) {
                let high_value_core = state.pending == Some(PendingTransaction::Exchange)
                    && state
                        .pending_id
                        .as_deref()
                        .and_then(|id| catalog.item(id))
                        .is_some_and(|item| {
                            matches!(item, ItemDefinition::MagicCore(core) if core.exchange_rate.get() >= 1_000)
                        });
                if high_value_core {
                    state.page = Some(ServicePage::CoreConfirm);
                    state.message.clear();
                } else {
                    execute_pending(&mut state, &catalog, &mut game);
                }
            }
        }
        Some(ServicePage::CoreConfirm) => {
            if keys.just_pressed(KeyCode::KeyN) {
                state.page = Some(ServicePage::Quantity);
            } else if keys.just_pressed(KeyCode::KeyY) || actions.just_pressed(AppAction::Confirm) {
                execute_pending(&mut state, &catalog, &mut game);
            }
        }
        Some(ServicePage::Result) if actions.just_pressed(AppAction::Confirm) => state.close(),
        Some(ServicePage::Result) => {}
        None => {}
    }
}

fn execute_pending(state: &mut ServiceUiState, catalog: &FieldMenuCatalog, game: &mut GameState) {
    let Some(id) = state.pending_id.clone() else {
        return;
    };
    let result = match state.pending {
        Some(PendingTransaction::Buy) => active_shop(state, catalog, game)
            .and_then(|shop| shop.items.iter().find(|row| row.id() == id))
            .ok_or_else(|| "shop item is unavailable".to_owned())
            .and_then(|row| {
                buy(game.repository_mut(), row, state.quantity).map_err(|e| e.to_string())
            }),
        Some(PendingTransaction::Sell) => catalog
            .item(&id)
            .ok_or_else(|| "item metadata is unavailable".to_owned())
            .and_then(|item| {
                sell(game.repository_mut(), item, state.quantity).map_err(|e| e.to_string())
            }),
        Some(PendingTransaction::Exchange) => catalog
            .item(&id)
            .ok_or_else(|| "core metadata is unavailable".to_owned())
            .and_then(|item| {
                exchange_magic_core(game.repository_mut(), item, state.quantity)
                    .map_err(|e| e.to_string())
            }),
        None => return,
    };
    match result {
        Ok(total) => {
            state.message = format!("Transaction complete: {total} GP.");
            state.page = Some(match state.pending {
                Some(PendingTransaction::Buy) => ServicePage::Buy,
                Some(PendingTransaction::Sell) => ServicePage::Sell,
                Some(PendingTransaction::Exchange) => ServicePage::MagicCore,
                None => unreachable!(),
            });
            state.selected = 0;
            state.quantity = 1;
            state.pending_id = None;
            state.pending = None;
        }
        Err(error) => state.message = error,
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

fn owned_items<'a>(catalog: &'a FieldMenuCatalog, game: &GameState) -> Vec<&'a ItemDefinition> {
    catalog
        .ordered_items()
        .filter(|item| game.repository().item_count(item.id()) > 0)
        .collect()
}

fn owned_cores<'a>(catalog: &'a FieldMenuCatalog, game: &GameState) -> Vec<&'a ItemDefinition> {
    owned_items(catalog, game)
        .into_iter()
        .filter(|item| matches!(item, ItemDefinition::MagicCore(_)))
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

#[expect(
    clippy::too_many_arguments,
    clippy::type_complexity,
    reason = "the service overlay synchronizes disjoint Bevy text roles from shared service state"
)]
fn sync_service_overlay(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
    theme: Res<UiTheme>,
    state: Res<ServiceUiState>,
    catalog: Res<FieldMenuCatalog>,
    game: Option<Res<GameState>>,
    roots: Query<Entity, With<ServiceRoot>>,
    mut titles: Query<
        &mut Text,
        (
            With<ServiceTitle>,
            Without<ServiceBody>,
            Without<ServiceHint>,
        ),
    >,
    mut bodies: Query<
        &mut Text,
        (
            With<ServiceBody>,
            Without<ServiceTitle>,
            Without<ServiceHint>,
        ),
    >,
    mut hints: Query<
        &mut Text,
        (
            With<ServiceHint>,
            Without<ServiceTitle>,
            Without<ServiceBody>,
        ),
    >,
) {
    if !state.input_locked() {
        for entity in &roots {
            commands.entity(entity).despawn();
        }
        return;
    }
    if roots.is_empty() {
        spawn_service_overlay(&mut commands, &asset_server, &root, &inventory, &theme);
        return;
    }
    let Some(game) = game else { return };
    if let Ok(mut title) = titles.single_mut() {
        title.0 = service_title(&state).to_owned();
    }
    if let Ok(mut body) = bodies.single_mut() {
        body.0 = render_service(&state, &catalog, &game);
    }
    if let Ok(mut hint) = hints.single_mut() {
        hint.0 = service_hint(&state).to_owned();
    }
}

fn service_title(state: &ServiceUiState) -> &'static str {
    match state.request {
        Some(ServiceRequest::Shop(DialogueShopKind::Item)) => "Item Shop",
        Some(ServiceRequest::Shop(DialogueShopKind::Weapon)) => "Weapon Shop",
        Some(ServiceRequest::Shop(DialogueShopKind::Armor)) => "Armor Shop",
        Some(ServiceRequest::Shop(DialogueShopKind::MagicCore)) => "Magic Core Exchange",
        Some(ServiceRequest::Inn) => "Inn",
        Some(ServiceRequest::Apothecary) => "Apothecary",
        None => "Service",
    }
}

fn render_service(state: &ServiceUiState, catalog: &FieldMenuCatalog, game: &GameState) -> String {
    let gp = game.repository().gp();
    let content = match state.page {
        Some(ServicePage::ShopMenu) => ["Buy", "Sell"]
            .into_iter()
            .enumerate()
            .map(|(index, label)| format!("{} {label}", cursor(index, state.selected)))
            .collect::<Vec<_>>()
            .join("\n"),
        Some(ServicePage::Buy) => render_buy_rows(state, catalog, game),
        Some(ServicePage::Sell) => render_sell_rows(state, catalog, game),
        Some(ServicePage::MagicCore) => render_core_rows(state, catalog, game),
        Some(ServicePage::Inn) => {
            let cost = active_inn_cost(catalog, game).unwrap_or(0);
            format!(
                "Rest for {cost} GP?\nAll HP, MP, and status effects will be restored.\n\nY/ENTER Yes     N/ESC No"
            )
        }
        Some(ServicePage::Apothecary) => render_recipes(state, catalog, game),
        Some(ServicePage::Quantity) => render_quantity(state, catalog, game),
        Some(ServicePage::CoreConfirm) => format!(
            "{}\n\nExchange this high-value core stack?\nY/ENTER Yes     N/ESC No",
            render_quantity(state, catalog, game)
        ),
        Some(ServicePage::Result) => "Your stay is complete.".to_owned(),
        None => String::new(),
    };
    let message = if state.message.is_empty() {
        String::new()
    } else {
        format!("\n\n{}", state.message)
    };
    format!("GP {gp}\n\n{content}{message}")
}

fn render_buy_rows(state: &ServiceUiState, catalog: &FieldMenuCatalog, game: &GameState) -> String {
    let rows = active_shop(state, catalog, game)
        .map(|shop| visible_stock(shop, game.flags()))
        .unwrap_or_default();
    let mut text = rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let item = catalog.item(row.id());
            let name = item.map(item_name).unwrap_or(row.id());
            let owned = game.repository().item_count(row.id());
            let afford = if game.repository().gp() >= row.buy_price().get() {
                ""
            } else {
                " [unaffordable]"
            };
            format!(
                "{} {:<22} {:>5} GP  owned {:>2}{afford}",
                cursor(index, state.selected),
                name,
                row.buy_price(),
                owned
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    if let Some(row) = rows.get(state.selected)
        && let Some(item) = catalog.item(row.id())
    {
        text.push_str(&format!("\n\n{}", item_description(item)));
        text.push_str(&equipment_preview(item, catalog, game));
    }
    if text.is_empty() {
        "No stock is currently unlocked.".to_owned()
    } else {
        text
    }
}

fn equipment_preview(
    item: &ItemDefinition,
    catalog: &FieldMenuCatalog,
    game: &GameState,
) -> String {
    let mut lines = Vec::new();
    for member in game.party().members() {
        match can_equip(member, item, catalog) {
            Ok(slot) => {
                let before = derived_stats(member, catalog);
                let after = preview_stats(member, catalog, slot, Some(item.id()));
                lines.push(format!(
                    "{}: equip  STR {:+} DEX {:+} CON {:+} INT {:+}",
                    member.name(),
                    after.strength - before.strength,
                    after.dexterity - before.dexterity,
                    after.constitution - before.constitution,
                    after.intelligence - before.intelligence
                ));
            }
            Err(_)
                if matches!(
                    item,
                    ItemDefinition::Weapon(_)
                        | ItemDefinition::Shield(_)
                        | ItemDefinition::Helmet(_)
                        | ItemDefinition::Body(_)
                        | ItemDefinition::Accessory(_)
                ) =>
            {
                lines.push(format!("{}: incompatible", member.name()))
            }
            Err(_) => {}
        }
    }
    if lines.is_empty() {
        String::new()
    } else {
        format!("\n{}", lines.join("\n"))
    }
}

fn render_sell_rows(
    state: &ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
) -> String {
    let rows = owned_items(catalog, game);
    if rows.is_empty() {
        return "There is nothing to sell.".to_owned();
    }
    rows.iter()
        .enumerate()
        .map(|(index, item)| {
            let price = sell_price(item).unwrap_or(0);
            let disabled = if can_sell(game.repository(), item) {
                ""
            } else {
                " [locked]"
            };
            format!(
                "{} {:<22} x{:>2}  {:>5} GP{disabled}",
                cursor(index, state.selected),
                item_name(item),
                game.repository().item_count(item.id()),
                price
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_core_rows(
    state: &ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
) -> String {
    let rows = owned_cores(catalog, game);
    if rows.is_empty() {
        return "No magic cores to exchange.".to_owned();
    }
    rows.iter()
        .enumerate()
        .map(|(index, item)| {
            let ItemDefinition::MagicCore(core) = item else {
                unreachable!()
            };
            let confirm = if core.exchange_rate.get() >= 1_000 {
                " [confirm]"
            } else {
                ""
            };
            format!(
                "{} {:<22} x{:>2}  {:>5} GP{confirm}",
                cursor(index, state.selected),
                item_name(item),
                game.repository().item_count(item.id()),
                core.exchange_rate
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_recipes(state: &ServiceUiState, catalog: &FieldMenuCatalog, game: &GameState) -> String {
    let mut text = catalog
        .recipes()
        .iter()
        .enumerate()
        .map(|(index, recipe)| {
            let status = match recipe_availability(recipe, game.flags(), game.repository()) {
                RecipeAvailability::Locked => "locked",
                RecipeAvailability::MissingInputs => "missing inputs",
                RecipeAvailability::Unaffordable => "not enough GP",
                RecipeAvailability::UniqueOwned => "already owned",
                RecipeAvailability::OutputCap => "item cap reached",
                RecipeAvailability::Ready => "ready",
            };
            let output = catalog
                .item(&recipe.output.item)
                .map(item_name)
                .unwrap_or(&recipe.output.item);
            format!(
                "{} {:<24} -> {:<20} {:>4} GP  [{status}]",
                cursor(index, state.selected),
                recipe.scroll_name,
                output,
                recipe.gp_cost
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    if let Some(recipe) = catalog.recipes().get(state.selected) {
        let output = catalog
            .item(&recipe.output.item)
            .map(item_name)
            .unwrap_or(&recipe.output.item);
        let mut inputs = recipe
            .inputs
            .items
            .iter()
            .map(|input| {
                let name = catalog.item(&input.id).map(item_name).unwrap_or(&input.id);
                format!(
                    "{name} {}/{}",
                    game.repository().item_count(&input.id),
                    input.qty
                )
            })
            .collect::<Vec<_>>();
        inputs.extend(recipe.inputs.mc.iter().map(|input| {
            let id = match input.size {
                crate::scenario_recipe::MagicCoreSize::XS => "mc_xs",
                crate::scenario_recipe::MagicCoreSize::S => "mc_s",
                crate::scenario_recipe::MagicCoreSize::M => "mc_m",
                crate::scenario_recipe::MagicCoreSize::L => "mc_l",
                crate::scenario_recipe::MagicCoreSize::XL => "mc_xl",
            };
            let name = catalog.item(id).map(item_name).unwrap_or(id);
            format!("{name} {}/{}", game.repository().item_count(id), input.qty)
        }));
        text.push_str(&format!(
            "\n\nOutput: {output} x{}\nIngredients: {}",
            recipe.output.qty,
            if inputs.is_empty() {
                "None".to_owned()
            } else {
                inputs.join(", ")
            }
        ));
    }
    text
}

fn render_quantity(state: &ServiceUiState, catalog: &FieldMenuCatalog, game: &GameState) -> String {
    let id = state.pending_id.as_deref().unwrap_or_default();
    let name = catalog.item(id).map(item_name).unwrap_or(id);
    let unit = match state.pending {
        Some(PendingTransaction::Buy) => active_shop(state, catalog, game)
            .and_then(|shop| shop.items.iter().find(|row| row.id() == id))
            .map(|row| row.buy_price().get()),
        Some(PendingTransaction::Sell) => catalog.item(id).and_then(sell_price),
        Some(PendingTransaction::Exchange) => match catalog.item(id) {
            Some(ItemDefinition::MagicCore(core)) => Some(core.exchange_rate.get()),
            _ => None,
        },
        None => None,
    }
    .unwrap_or(0);
    format!(
        "{name}\nQuantity: {} / {}\nTotal: {} GP",
        state.quantity,
        pending_max(state, catalog, game),
        unit.saturating_mul(state.quantity)
    )
}

fn service_hint(state: &ServiceUiState) -> &'static str {
    match state.page {
        Some(ServicePage::Inn) => "Y/ENTER · rest     N/ESC · leave",
        Some(ServicePage::Quantity) => {
            "UP/DOWN · quantity     LEFT/RIGHT · min/max     ENTER · confirm     ESC · back"
        }
        Some(ServicePage::CoreConfirm) => "Y/ENTER · exchange     N/ESC · cancel",
        Some(ServicePage::Result) => "ENTER / ESC · leave",
        _ => "UP/DOWN · choose     ENTER · confirm     ESC · back",
    }
}

fn cursor(index: usize, selected: usize) -> &'static str {
    if index == selected { ">" } else { " " }
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
fn wrapped_quantity(current: u32, max: u32, delta: isize) -> u32 {
    ((current.saturating_sub(1) as isize + delta).rem_euclid(max as isize) + 1) as u32
}

fn spawn_service_overlay(
    commands: &mut Commands,
    assets: &AssetServer,
    root: &ScenarioRoot,
    inventory: &ScenarioInventory,
    theme: &UiTheme,
) {
    let Some(font_path) = inventory.font.as_ref() else {
        return;
    };
    let font = assets.load(root.resolve(font_path));
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(70),
                top: px(55),
                width: px(820),
                height: px(430),
                padding: UiRect::all(px(24)),
                border: UiRect::all(px(2)),
                flex_direction: FlexDirection::Column,
                row_gap: px(12),
                ..default()
            },
            BackgroundColor(Color::srgba(0.025, 0.025, 0.09, 0.97)),
            BorderColor::all(theme.name_entry_border_color),
            GlobalZIndex(5_100),
            Pickable::IGNORE,
            Name::new("World service"),
            ServiceRoot,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new(""),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(32.0),
                    ..default()
                },
                TextColor(theme.name_entry_input_color),
                ServiceTitle,
            ));
            panel.spawn((
                Text::new(""),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::srgb_u8(235, 225, 190)),
                TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
                Node {
                    width: percent(100),
                    flex_grow: 1.0,
                    ..default()
                },
                ServiceBody,
            ));
            panel.spawn((
                Text::new(""),
                TextFont {
                    font: font.into(),
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(theme.name_entry_hint_color),
                ServiceHint,
            ));
        });
}

fn cleanup_service_ui(
    mut commands: Commands,
    roots: Query<Entity, With<ServiceRoot>>,
    mut state: ResMut<ServiceUiState>,
) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    state.close();
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
