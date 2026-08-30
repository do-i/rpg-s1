//! Node-tree rendering for the service overlays.
//!
//! Every page is rebuilt from [`ServiceUiState`] whenever the state, the catalog, or the game
//! changes — the same contract the field-menu pages use. Nothing here mutates game state; the
//! parent module owns every transition.

use bevy::{ecs::hierarchy::ChildSpawnerCommands, prelude::*};

use super::{
    PendingTransaction, SERVICE_VISIBLE_ROWS, ServicePage, ServiceRequest, ServiceUiState,
    ToastTone, active_inn_cost, active_shop, buy_rows, is_equipment, owned_cores, pending_max,
    sell_rows, sellable_tags,
    sprites::{RecipeIcon, ServiceKeeper, ServiceSprites},
};
use crate::{
    field_menu_domain::{
        FieldMenuCatalog, can_equip, derived_stats, item_description, item_name, preview_stats,
    },
    game_state::GameState,
    menu_chrome::{
        spawn_header_bars, spawn_meter, spawn_section_rule, spawn_status_panel, spawn_status_text,
        status_border, status_border_active, status_ember, status_faint, status_gold, status_ink,
        status_muted, status_ready, status_teal, window_start,
    },
    runtime_member::RuntimeMember,
    scenario_dialogue::DialogueShopKind,
    scenario_inventory::ScenarioInventory,
    scenario_item::ItemDefinition,
    scenario_recipe::{MagicCoreSize, RecipeDefinition},
    scenario_root::ScenarioRoot,
    service_domain::{RecipeAvailability, can_sell, recipe_availability, sell_price},
    tsx_atlas_asset::TsxAtlasAsset,
    ui_theme::UiTheme,
};

#[derive(Component)]
pub(super) struct ServiceRoot;

/// Panel width for the plain list and confirmation pages (source `MODAL_W`).
const PANEL_WIDTH: f32 = 620.0;

/// Panel width once a detail column is beside the list — the source widens the same way for its
/// equipment preview (`item_shop_renderer.py`).
const WIDE_PANEL_WIDTH: f32 = 980.0;

/// Ceiling on the overlay frame, leaving a margin inside the 766px logical canvas.
///
/// This is a pixel cap rather than a percentage on purpose. The centring row above the panel
/// gives its child an indefinite cross size, so a percentage `max_height` has no basis to
/// resolve against and clamps the frame to a fraction of its content — which left the list
/// rows drawn outside their own border. The fixed gameplay canvas is 1280x766, so a pixel
/// ceiling is exact here.
const PANEL_MAX_HEIGHT: f32 = 700.0;

/// Fixed buy-page detail footprint, sized for the title, description, and all five party preview
/// rows. Consumables omit the equipment preview, but must keep this space reserved so moving the
/// stock cursor never resizes either column or the containing modal.
const BUY_DETAIL_HEIGHT: f32 = 500.0;

/// Keeps compatible and incompatible party rows the same height. Compatible rows add a stat
/// delta line; incompatible rows leave that line empty, but the cursor must not reflow the panel.
const EQUIPMENT_PREVIEW_ROW_HEIGHT: f32 = 64.0;

/// Keeper face sizes: the inn draws a larger portrait than the counter services.
const KEEPER_FACE: f32 = 64.0;
const INN_KEEPER_FACE: f32 = 96.0;

#[expect(
    clippy::too_many_arguments,
    reason = "the overlay reads the scenario root, its art, the theme, and live game state"
)]
pub(super) fn sync_service_overlay(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
    theme: Res<UiTheme>,
    state: Res<ServiceUiState>,
    catalog: Res<FieldMenuCatalog>,
    sprites: Res<ServiceSprites>,
    atlases: Res<Assets<TsxAtlasAsset>>,
    game: Option<Res<GameState>>,
    roots: Query<Entity, With<ServiceRoot>>,
) {
    if !state.input_locked() {
        for entity in &roots {
            commands.entity(entity).despawn();
        }
        return;
    }
    let Some(game) = game else { return };
    let rebuild = roots.is_empty()
        || state.is_changed()
        || catalog.is_changed()
        || game.is_changed()
        || atlases.is_changed();
    if !rebuild {
        return;
    }
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    let Some(font_path) = inventory.font.as_ref() else {
        return;
    };
    let font: Handle<Font> = asset_server.load(root.resolve(font_path));
    let face = keeper_face(&state, &sprites, &atlases);

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba_u8(2, 2, 6, 170)),
            GlobalZIndex(5_100),
            Pickable::IGNORE,
            Name::new("World service"),
            ServiceRoot,
        ))
        .with_children(|screen| {
            screen
                .spawn((
                    Node {
                        width: px(panel_width(&state)),
                        max_height: px(PANEL_MAX_HEIGHT),
                        padding: UiRect::all(px(22)),
                        border: UiRect::all(px(2)),
                        border_radius: BorderRadius::all(px(8)),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(12),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.025, 0.025, 0.09, 0.97)),
                    BorderColor::all(theme.name_entry_border_color),
                ))
                .with_children(|panel| {
                    spawn_service_header(panel, &font, &state, &game, face);
                    spawn_section_rule(panel);
                    spawn_service_body(panel, &font, &state, &catalog, &game, &sprites);
                    spawn_section_rule(panel);
                    spawn_status_text(panel, service_hint(&state), &font, 15.0, status_faint());
                    spawn_toast(panel, &font, &state);
                });
        });
}

pub(super) fn cleanup_service_ui(
    mut commands: Commands,
    roots: Query<Entity, With<ServiceRoot>>,
    mut state: ResMut<ServiceUiState>,
) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    state.close();
}

/// The pages that put a detail column beside the list need the wider frame.
fn panel_width(state: &ServiceUiState) -> f32 {
    match state.page {
        Some(ServicePage::Buy | ServicePage::Apothecary | ServicePage::RecipeDetail) => {
            WIDE_PANEL_WIDTH
        }
        _ => PANEL_WIDTH,
    }
}

fn keeper_face(
    state: &ServiceUiState,
    sprites: &ServiceSprites,
    atlases: &Assets<TsxAtlasAsset>,
) -> Option<ImageNode> {
    let keeper = match state.request? {
        ServiceRequest::Shop(DialogueShopKind::Item | DialogueShopKind::MagicCore) => {
            ServiceKeeper::ItemShop
        }
        ServiceRequest::Shop(DialogueShopKind::Weapon) => ServiceKeeper::WeaponShop,
        ServiceRequest::Shop(DialogueShopKind::Armor) => ServiceKeeper::ArmorShop,
        ServiceRequest::Inn => ServiceKeeper::Inn,
        ServiceRequest::Apothecary => ServiceKeeper::Apothecary,
    };
    sprites.keeper_face(keeper, atlases)
}

fn spawn_service_header(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    game: &GameState,
    face: Option<ImageNode>,
) {
    let inn = state.request == Some(ServiceRequest::Inn);
    let size = if inn { INN_KEEPER_FACE } else { KEEPER_FACE };
    parent
        .spawn(Node {
            width: percent(100),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(14),
            ..default()
        })
        .with_children(|header| {
            spawn_keeper_frame(header, font, size, face);
            spawn_header_bars(header, size * 0.6, 12.0);
            header
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    ..default()
                })
                .with_children(|title| {
                    spawn_status_text(title, service_title(state), font, 29.0, status_ink());
                    spawn_status_text(title, service_subtitle(state), font, 13.0, status_muted());
                });
            spawn_status_text(
                header,
                format!("GP  {}", game.repository().gp()),
                font,
                20.0,
                status_gold(),
            );
        });
}

/// Draws the keeper portrait, or a lettered frame while the atlas is still loading.
fn spawn_keeper_frame(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    size: f32,
    face: Option<ImageNode>,
) {
    let mut frame = parent.spawn((
        Node {
            width: px(size),
            height: px(size),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(px(2)),
            border_radius: BorderRadius::all(px(6)),
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(Color::srgba_u8(10, 10, 14, 200)),
        BorderColor::all(status_border()),
    ));
    match face {
        Some(image) => {
            frame.with_children(|slot| {
                slot.spawn((
                    image,
                    Node {
                        width: percent(100),
                        height: percent(100),
                        ..default()
                    },
                ));
            });
        }
        None => {
            frame.with_children(|slot| {
                spawn_status_text(slot, "?", font, size * 0.4, status_faint());
            });
        }
    }
}

fn spawn_service_body(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
    sprites: &ServiceSprites,
) {
    match state.page {
        Some(ServicePage::ShopMenu) => spawn_shop_menu(parent, font, state),
        Some(ServicePage::Buy) => spawn_buy_page(parent, font, state, catalog, game),
        Some(ServicePage::Sell) => spawn_sell_page(parent, font, state, catalog, game),
        Some(ServicePage::MagicCore) => spawn_core_page(parent, font, state, catalog, game),
        Some(ServicePage::Inn) => spawn_inn_page(parent, font, catalog, game),
        Some(ServicePage::Apothecary | ServicePage::RecipeDetail) => {
            spawn_apothecary_page(parent, font, state, catalog, game, sprites);
        }
        Some(ServicePage::Quantity | ServicePage::CoreConfirm) => {
            spawn_quantity_page(parent, font, state, catalog, game);
        }
        Some(ServicePage::Equip) => spawn_equip_page(parent, font, state, catalog, game),
        Some(ServicePage::Result) => {
            spawn_status_text(parent, "Your stay is complete.", font, 18.0, status_ready());
        }
        None => {}
    }
}

// ── Shop menu ────────────────────────────────────────────────────────────────

fn spawn_shop_menu(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
) {
    parent
        .spawn(Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(6),
            ..default()
        })
        .with_children(|list| {
            for (index, (label, subtitle)) in [
                ("Buy", "browse the counter's stock"),
                ("Sell", "turn carried goods into GP"),
            ]
            .into_iter()
            .enumerate()
            {
                spawn_choice_row(list, font, label, subtitle, index == state.selected);
            }
        });
}

// ── Buy ──────────────────────────────────────────────────────────────────────

fn spawn_buy_page(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
) {
    let rows = buy_rows(state, catalog, game);
    spawn_columns(parent, |columns| {
        spawn_status_panel(columns, list_panel_node(), "STOCK", font, |panel| {
            if rows.is_empty() {
                spawn_status_text(
                    panel,
                    "No stock is currently unlocked.",
                    font,
                    17.0,
                    status_muted(),
                );
                return;
            }
            let first = window_start(state.selected, rows.len(), SERVICE_VISIBLE_ROWS);
            for (index, row) in rows
                .iter()
                .enumerate()
                .skip(first)
                .take(SERVICE_VISIBLE_ROWS)
            {
                let price = row.buy_price().get();
                let affordable = game.repository().gp() >= price;
                let item = catalog.item(row.id());
                spawn_list_row(
                    panel,
                    font,
                    ListRow {
                        selected: index == state.selected,
                        accent: item.map_or_else(status_teal, item_accent),
                        label: item.map_or(row.id(), |item| item_name(item)),
                        note: format!("owned {}", game.repository().item_count(row.id())),
                        value: format!("{price} GP"),
                        value_color: if affordable {
                            status_gold()
                        } else {
                            status_ember()
                        },
                        dim: !affordable,
                    },
                );
            }
            spawn_window_footer(panel, font, state.selected, rows.len());
        });
        spawn_status_panel(columns, buy_detail_panel_node(), "DETAIL", font, |panel| {
            let Some(item) = rows
                .get(state.selected)
                .and_then(|row| catalog.item(row.id()))
            else {
                spawn_status_text(panel, "Nothing to inspect.", font, 16.0, status_muted());
                return;
            };
            spawn_status_text(panel, item_name(item), font, 21.0, status_gold());
            spawn_status_text(panel, item_description(item), font, 15.0, status_ink());
            spawn_equipment_preview(panel, font, state, catalog, game, item);
        });
    });
}

/// The bordered party panel Python draws beside an equippable purchase.
///
/// Each member shows what currently fills the slot and the signed change every stat would take,
/// with the focused row highlighted — Left/Right moves that focus.
fn spawn_equipment_preview(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
    item: &ItemDefinition,
) {
    // A consumable has no slot to compare against, so the source shows no party panel at all.
    if !is_equipment(item) {
        return;
    }
    spawn_section_rule(parent);
    spawn_status_text(parent, "IF EQUIPPED", font, 13.0, status_gold());
    for (index, member) in game.party().members().enumerate() {
        let focused = index == state.equip_selection;
        match can_equip(member, item, catalog) {
            Ok(slot) => {
                let before = derived_stats(member, catalog);
                let after = preview_stats(member, catalog, slot, Some(item.id()));
                let current = member
                    .equipment()
                    .get(slot)
                    .and_then(|id| catalog.item(id))
                    .map_or("empty", |worn| item_name(worn));
                let worn = format!("Now: {current}");
                spawn_preview_row(parent, font, member, focused, &worn, |deltas| {
                    for (label, delta) in [
                        ("STR", after.strength - before.strength),
                        ("DEX", after.dexterity - before.dexterity),
                        ("CON", after.constitution - before.constitution),
                        ("INT", after.intelligence - before.intelligence),
                    ] {
                        spawn_status_text(
                            deltas,
                            format!("{label} {delta:+}"),
                            font,
                            13.0,
                            delta_color(delta),
                        );
                    }
                });
            }
            Err(_) => {
                spawn_preview_row(parent, font, member, focused, "cannot equip", |_| {});
            }
        }
    }
}

fn spawn_preview_row(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    member: &RuntimeMember,
    focused: bool,
    subtitle: &str,
    deltas: impl FnOnce(&mut ChildSpawnerCommands<'_>),
) {
    parent
        .spawn((
            equipment_preview_row_node(focused),
            BackgroundColor(if focused {
                Color::srgba_u8(72, 49, 25, 218)
            } else {
                Color::srgba_u8(10, 10, 14, 150)
            }),
            BorderColor::all(if focused {
                status_border_active()
            } else {
                status_border()
            }),
        ))
        .with_children(|row| {
            spawn_status_text(row, member.name(), font, 16.0, status_ink());
            spawn_status_text(row, subtitle, font, 12.0, status_muted());
            row.spawn(Node {
                width: percent(100),
                flex_direction: FlexDirection::Row,
                column_gap: px(10),
                ..default()
            })
            .with_children(deltas);
        });
}

fn equipment_preview_row_node(focused: bool) -> Node {
    Node {
        width: percent(100),
        height: px(EQUIPMENT_PREVIEW_ROW_HEIGHT),
        min_height: px(EQUIPMENT_PREVIEW_ROW_HEIGHT),
        max_height: px(EQUIPMENT_PREVIEW_ROW_HEIGHT),
        flex_direction: FlexDirection::Column,
        padding: UiRect::axes(px(10), px(5)),
        border: UiRect::all(px(if focused { 2 } else { 1 })),
        border_radius: BorderRadius::all(px(4)),
        ..default()
    }
}

fn delta_color(delta: i32) -> Color {
    match delta.signum() {
        1 => status_ready(),
        -1 => status_ember(),
        _ => status_faint(),
    }
}

// ── Sell ─────────────────────────────────────────────────────────────────────

fn spawn_sell_page(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
) {
    let filter = state.sell_tag.as_deref().unwrap_or("ALL");
    let tag_count = sellable_tags(catalog, game).len();
    spawn_status_text(
        parent,
        format!(
            "FILTER  {}   ·   {tag_count} TAGS AVAILABLE",
            filter.to_uppercase()
        ),
        font,
        13.0,
        status_teal(),
    );
    let rows = sell_rows(state, catalog, game);
    spawn_status_panel(parent, stacked_panel_node(), "CARRIED", font, |panel| {
        if rows.is_empty() {
            spawn_status_text(
                panel,
                "There is nothing to sell.",
                font,
                17.0,
                status_muted(),
            );
            return;
        }
        let first = window_start(state.selected, rows.len(), SERVICE_VISIBLE_ROWS);
        for (index, item) in rows
            .iter()
            .enumerate()
            .skip(first)
            .take(SERVICE_VISIBLE_ROWS)
        {
            let sellable = can_sell(game.repository(), item);
            spawn_list_row(
                panel,
                font,
                ListRow {
                    selected: index == state.selected,
                    accent: item_accent(item),
                    label: item_name(item),
                    note: format!("x{}", game.repository().item_count(item.id())),
                    value: sell_price(item)
                        .map_or_else(|| "—".to_owned(), |price| format!("{price} GP")),
                    value_color: if sellable {
                        status_gold()
                    } else {
                        status_faint()
                    },
                    dim: !sellable,
                },
            );
        }
        spawn_window_footer(panel, font, state.selected, rows.len());
    });
}

// ── Magic core ───────────────────────────────────────────────────────────────

fn spawn_core_page(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
) {
    let rows = owned_cores(catalog, game);
    spawn_status_panel(parent, stacked_panel_node(), "CORES", font, |panel| {
        if rows.is_empty() {
            spawn_status_text(
                panel,
                "No magic cores to exchange.",
                font,
                17.0,
                status_muted(),
            );
            return;
        }
        let first = window_start(state.selected, rows.len(), SERVICE_VISIBLE_ROWS);
        for (index, item) in rows
            .iter()
            .enumerate()
            .skip(first)
            .take(SERVICE_VISIBLE_ROWS)
        {
            let ItemDefinition::MagicCore(core) = item else {
                continue;
            };
            let rate = core.exchange_rate.get();
            spawn_list_row(
                panel,
                font,
                ListRow {
                    selected: index == state.selected,
                    accent: item_accent(item),
                    label: item_name(item),
                    // The owned count is what the player is choosing against, so it stays on
                    // every row; the confirmation warning is appended, never substituted.
                    note: {
                        let owned = game.repository().item_count(item.id());
                        if rate >= super::HIGH_VALUE_CORE_RATE {
                            format!("x{owned}   ·   confirms before exchange")
                        } else {
                            format!("x{owned}")
                        }
                    },
                    value: format!("{rate} GP"),
                    value_color: status_gold(),
                    dim: false,
                },
            );
        }
        spawn_window_footer(panel, font, state.selected, rows.len());
    });
}

// ── Inn ──────────────────────────────────────────────────────────────────────

fn spawn_inn_page(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    catalog: &FieldMenuCatalog,
    game: &GameState,
) {
    let cost = active_inn_cost(catalog, game).unwrap_or(0);
    let affordable = game.repository().gp() >= cost;
    spawn_status_text(
        parent,
        format!("{cost} GP / night"),
        font,
        26.0,
        if affordable {
            status_gold()
        } else {
            status_ember()
        },
    );
    spawn_status_panel(parent, stacked_panel_node(), "PARTY", font, |panel| {
        for member in game.party().members() {
            panel
                .spawn(Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(12),
                    ..default()
                })
                .with_children(|row| {
                    row.spawn(Node {
                        width: px(120),
                        ..default()
                    })
                    .with_children(|name| {
                        spawn_status_text(name, member.name(), font, 16.0, status_ink());
                    });
                    row.spawn(Node {
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::Column,
                        row_gap: px(4),
                        ..default()
                    })
                    .with_children(|meters| {
                        spawn_meter(
                            meters,
                            "HP",
                            member.health(),
                            member.max_health(),
                            font,
                            status_ember(),
                        );
                        spawn_meter(
                            meters,
                            "MP",
                            member.mana(),
                            member.max_mana(),
                            font,
                            status_teal(),
                        );
                    });
                });
        }
    });
    spawn_status_text(
        parent,
        "All HP, MP, and status effects will be restored.",
        font,
        14.0,
        status_muted(),
    );
}

// ── Apothecary ───────────────────────────────────────────────────────────────

fn spawn_apothecary_page(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
    sprites: &ServiceSprites,
) {
    let recipes = catalog.recipes();
    spawn_columns(parent, |columns| {
        spawn_status_panel(columns, list_panel_node(), "RECIPES", font, |panel| {
            if recipes.is_empty() {
                spawn_status_text(panel, "No recipes are known.", font, 17.0, status_muted());
                return;
            }
            let first = window_start(state.selected, recipes.len(), SERVICE_VISIBLE_ROWS);
            for (index, recipe) in recipes
                .iter()
                .enumerate()
                .skip(first)
                .take(SERVICE_VISIBLE_ROWS)
            {
                let availability = recipe_availability(recipe, game.flags(), game.repository());
                spawn_recipe_row(
                    panel,
                    font,
                    sprites,
                    recipe,
                    catalog,
                    availability,
                    index == state.selected,
                );
            }
            spawn_window_footer(panel, font, state.selected, recipes.len());
        });
        spawn_status_panel(columns, detail_panel_node(), "RECIPE", font, |panel| {
            let Some(recipe) = recipes.get(state.selected) else {
                spawn_status_text(panel, "Nothing selected.", font, 16.0, status_muted());
                return;
            };
            if state.page == Some(ServicePage::RecipeDetail) {
                spawn_recipe_detail(panel, font, recipe, catalog, game);
            } else {
                spawn_status_text(panel, &recipe.scroll_name, font, 21.0, status_gold());
                spawn_status_text(
                    panel,
                    "ENTER opens the full recipe before anything is spent.",
                    font,
                    15.0,
                    status_muted(),
                );
            }
        });
    });
}

fn spawn_recipe_row(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    sprites: &ServiceSprites,
    recipe: &RecipeDefinition,
    catalog: &FieldMenuCatalog,
    availability: RecipeAvailability,
    selected: bool,
) {
    let icon = RecipeIcon::for_availability(availability);
    let ready = availability == RecipeAvailability::Ready;
    let output = catalog
        .item(&recipe.output.item)
        .map_or(recipe.output.item.as_str(), |item| item_name(item));
    parent
        .spawn((
            Node {
                width: percent(100),
                min_height: px(40),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(9),
                padding: UiRect::horizontal(px(9)),
                border: UiRect::all(px(if selected { 2 } else { 1 })),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(if selected {
                Color::srgba_u8(72, 49, 25, 218)
            } else {
                Color::srgba_u8(10, 10, 14, 148)
            }),
            BorderColor::all(if selected {
                status_border_active()
            } else {
                Color::srgba_u8(126, 98, 55, 80)
            }),
        ))
        .with_children(|row| {
            match sprites.recipe_icon(icon) {
                Some(image) => {
                    row.spawn((
                        ImageNode::new(image),
                        Node {
                            width: px(22),
                            height: px(22),
                            ..default()
                        },
                    ));
                }
                None => {
                    row.spawn((
                        Node {
                            width: px(5),
                            height: px(22),
                            ..default()
                        },
                        BackgroundColor(availability_color(availability)),
                    ));
                }
            }
            row.spawn(Node {
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                ..default()
            })
            .with_children(|label| {
                spawn_status_text(
                    label,
                    &recipe.scroll_name,
                    font,
                    16.0,
                    if ready { status_ink() } else { status_muted() },
                );
                spawn_status_text(
                    label,
                    format!("-> {output} x{}", recipe.output.qty),
                    font,
                    12.0,
                    status_faint(),
                );
            });
            spawn_status_text(
                row,
                format!("{} GP", recipe.gp_cost),
                font,
                14.0,
                availability_color(availability),
            );
        });
}

/// The committed-recipe view: output, every ingredient against what the player holds, and cost.
fn spawn_recipe_detail(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    recipe: &RecipeDefinition,
    catalog: &FieldMenuCatalog,
    game: &GameState,
) {
    let output = catalog
        .item(&recipe.output.item)
        .map_or(recipe.output.item.as_str(), |item| item_name(item));
    spawn_status_text(parent, &recipe.scroll_name, font, 21.0, status_gold());
    spawn_status_text(
        parent,
        format!("Output:  {output} x{}", recipe.output.qty),
        font,
        16.0,
        status_ready(),
    );
    spawn_section_rule(parent);
    spawn_status_text(parent, "Inputs:", font, 14.0, status_ink());
    for (id, required) in recipe_inputs(recipe) {
        let owned = game.repository().item_count(&id);
        let name = catalog
            .item(&id)
            .map_or_else(|| id.clone(), |item| item_name(item).to_owned());
        spawn_status_text(
            parent,
            format!("  {name}  x{required}  (owned: {owned})"),
            font,
            15.0,
            if owned >= required {
                status_ink()
            } else {
                status_ember()
            },
        );
    }
    spawn_section_rule(parent);
    let affordable = game.repository().gp() >= recipe.gp_cost;
    spawn_status_text(
        parent,
        format!("Cost: {} GP", recipe.gp_cost),
        font,
        16.0,
        if affordable {
            status_gold()
        } else {
            status_ember()
        },
    );
    spawn_status_text(
        parent,
        format!("Balance: {} GP", game.repository().gp()),
        font,
        13.0,
        status_muted(),
    );
}

/// Flattens a recipe's item and magic-core inputs into `(item id, required quantity)` pairs.
fn recipe_inputs(recipe: &RecipeDefinition) -> Vec<(String, u32)> {
    let items = recipe
        .inputs
        .items
        .iter()
        .map(|input| (input.id.clone(), input.qty.get()));
    let cores = recipe.inputs.mc.iter().map(|input| {
        let id = match input.size {
            MagicCoreSize::XS => "mc_xs",
            MagicCoreSize::S => "mc_s",
            MagicCoreSize::M => "mc_m",
            MagicCoreSize::L => "mc_l",
            MagicCoreSize::XL => "mc_xl",
        };
        (id.to_owned(), input.qty.get())
    });
    items.chain(cores).collect()
}

fn availability_color(availability: RecipeAvailability) -> Color {
    match availability {
        RecipeAvailability::Ready => status_ready(),
        RecipeAvailability::MissingInputs | RecipeAvailability::Unaffordable => status_ember(),
        RecipeAvailability::Locked
        | RecipeAvailability::UniqueOwned
        | RecipeAvailability::OutputCap => status_faint(),
    }
}

// ── Quantity, confirmation, equip ────────────────────────────────────────────

fn spawn_quantity_page(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
) {
    let id = state.pending_id.as_deref().unwrap_or_default();
    let name = catalog
        .item(id)
        .map_or(id, |item| item_name(item))
        .to_owned();
    let unit = unit_price(state, catalog, game);
    spawn_status_text(parent, &name, font, 23.0, status_gold());
    spawn_status_text(
        parent,
        format!(
            "{:02}  /  {}",
            state.quantity,
            pending_max(state, catalog, game)
        ),
        font,
        34.0,
        status_ink(),
    );
    spawn_status_text(
        parent,
        format!("Total: {} GP", unit.saturating_mul(state.quantity)),
        font,
        18.0,
        status_gold(),
    );
    if state.page == Some(ServicePage::CoreConfirm) {
        spawn_section_rule(parent);
        spawn_status_text(
            parent,
            "Exchange this high-value core stack?",
            font,
            17.0,
            status_ember(),
        );
    }
}

fn unit_price(state: &ServiceUiState, catalog: &FieldMenuCatalog, game: &GameState) -> u32 {
    let Some(id) = state.pending_id.as_deref() else {
        return 0;
    };
    match state.pending {
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
    .unwrap_or(0)
}

fn spawn_equip_page(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
) {
    let item = state
        .pending_equip_id
        .as_deref()
        .and_then(|id| catalog.item(id));
    let name = item.map_or("that", |item| item_name(item));
    spawn_status_text(parent, format!("Equip {name}?"), font, 21.0, status_gold());
    parent
        .spawn(Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(6),
            ..default()
        })
        .with_children(|list| {
            for (index, member) in game.party().members().enumerate() {
                let fit = item.is_some_and(|item| can_equip(member, item, catalog).is_ok());
                spawn_choice_row(
                    list,
                    font,
                    member.name(),
                    if fit { "can equip" } else { "cannot equip" },
                    index == state.equip_selection,
                );
            }
        });
}

// ── Shared row builders ──────────────────────────────────────────────────────

struct ListRow<'a> {
    selected: bool,
    accent: Color,
    label: &'a str,
    note: String,
    value: String,
    value_color: Color,
    dim: bool,
}

fn spawn_list_row(parent: &mut ChildSpawnerCommands<'_>, font: &Handle<Font>, row: ListRow<'_>) {
    parent
        .spawn((
            Node {
                width: percent(100),
                min_height: px(40),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(px(9)),
                border: UiRect::all(px(if row.selected { 2 } else { 1 })),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(if row.selected {
                Color::srgba_u8(72, 49, 25, 218)
            } else {
                Color::srgba_u8(10, 10, 14, 148)
            }),
            BorderColor::all(if row.selected {
                status_border_active()
            } else {
                Color::srgba_u8(126, 98, 55, 80)
            }),
        ))
        .with_children(|node| {
            node.spawn((
                Node {
                    width: px(5),
                    height: px(24),
                    margin: UiRect::right(px(9)),
                    ..default()
                },
                BackgroundColor(row.accent),
            ));
            node.spawn(Node {
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                ..default()
            })
            .with_children(|label| {
                spawn_status_text(
                    label,
                    row.label,
                    font,
                    16.0,
                    if row.dim {
                        status_faint()
                    } else {
                        status_ink()
                    },
                );
                spawn_status_text(label, row.note, font, 12.0, status_muted());
            });
            spawn_status_text(node, row.value, font, 15.0, row.value_color);
        });
}

fn spawn_choice_row(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    label: &str,
    subtitle: &str,
    selected: bool,
) {
    parent
        .spawn((
            Node {
                width: percent(100),
                min_height: px(46),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                padding: UiRect::axes(px(11), px(5)),
                border: UiRect::all(px(if selected { 2 } else { 1 })),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(if selected {
                Color::srgba_u8(72, 49, 25, 230)
            } else {
                Color::srgba_u8(10, 10, 14, 170)
            }),
            BorderColor::all(if selected {
                status_border_active()
            } else {
                status_border()
            }),
        ))
        .with_children(|row| {
            spawn_status_text(row, label, font, 17.0, status_ink());
            spawn_status_text(row, subtitle, font, 12.0, status_muted());
        });
}

/// Reports the cursor position, and the visible span too once the list scrolls.
fn spawn_window_footer(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    selected: usize,
    len: usize,
) {
    let position = format!("{:02} OF {len:02}", selected.min(len.saturating_sub(1)) + 1);
    let text = if len <= SERVICE_VISIBLE_ROWS {
        position
    } else {
        let first = window_start(selected, len, SERVICE_VISIBLE_ROWS);
        format!(
            "{position}   ·   SHOWING {:02}-{:02}",
            first + 1,
            (first + SERVICE_VISIBLE_ROWS).min(len)
        )
    };
    spawn_status_text(parent, text, font, 12.0, status_faint());
}

fn spawn_columns(
    parent: &mut ChildSpawnerCommands<'_>,
    columns: impl FnOnce(&mut ChildSpawnerCommands<'_>),
) {
    parent
        .spawn(Node {
            width: percent(100),
            flex_direction: FlexDirection::Row,
            column_gap: px(14),
            ..default()
        })
        .with_children(columns);
}

/// The list frame for a page whose list sits *beside* a detail column.
///
/// `flex_basis: 0` with `flex_grow` divides the row's width. It must not be used in the column
/// pages: there the same properties would divide *height* instead, and since the overlay panel
/// is sized by its content there is no free height to grow into — the frame would collapse to
/// zero and its rows would spill outside the border. Those pages take [`stacked_panel_node`].
fn list_panel_node() -> Node {
    Node {
        flex_basis: px(0),
        flex_grow: 1.0,
        ..stacked_panel_node()
    }
}

/// The list frame for a page that stacks its list under the header, sized by its own rows.
fn stacked_panel_node() -> Node {
    Node {
        width: percent(100),
        flex_direction: FlexDirection::Column,
        row_gap: px(5),
        padding: UiRect::all(px(14)),
        border: UiRect::all(px(1)),
        border_radius: BorderRadius::all(px(6)),
        ..default()
    }
}

fn detail_panel_node() -> Node {
    Node {
        width: px(380),
        flex_direction: FlexDirection::Column,
        row_gap: px(8),
        padding: UiRect::all(px(14)),
        border: UiRect::all(px(1)),
        border_radius: BorderRadius::all(px(6)),
        ..default()
    }
}

fn buy_detail_panel_node() -> Node {
    Node {
        height: px(BUY_DETAIL_HEIGHT),
        min_height: px(BUY_DETAIL_HEIGHT),
        max_height: px(BUY_DETAIL_HEIGHT),
        ..detail_panel_node()
    }
}

fn spawn_toast(parent: &mut ChildSpawnerCommands<'_>, font: &Handle<Font>, state: &ServiceUiState) {
    let Some(toast) = state.toast.as_ref() else {
        return;
    };
    let accent = match toast.tone {
        ToastTone::Good => status_ready(),
        ToastTone::Warn => status_ember(),
    };
    parent
        .spawn((
            Node {
                width: percent(100),
                justify_content: JustifyContent::Center,
                padding: UiRect::axes(px(14), px(8)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(5)),
                ..default()
            },
            BackgroundColor(Color::srgba_u8(22, 22, 28, 244)),
            BorderColor::all(accent),
        ))
        .with_children(|banner| {
            spawn_status_text(banner, toast.text.clone(), font, 15.0, accent);
        });
}

// ── Labels ───────────────────────────────────────────────────────────────────

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

fn service_subtitle(state: &ServiceUiState) -> &'static str {
    match state.page {
        Some(ServicePage::ShopMenu) => "COUNTER",
        Some(ServicePage::Buy) => "BUYING",
        Some(ServicePage::Sell) => "SELLING",
        Some(ServicePage::MagicCore) => "EXCHANGE",
        Some(ServicePage::Inn) => "REST",
        Some(ServicePage::Apothecary) => "RECIPES",
        Some(ServicePage::RecipeDetail) => "RECIPE DETAIL",
        Some(ServicePage::Quantity) => "QUANTITY",
        Some(ServicePage::CoreConfirm) => "CONFIRM",
        Some(ServicePage::Equip) => "EQUIP",
        Some(ServicePage::Result) => "RESTED",
        None => "",
    }
}

fn service_hint(state: &ServiceUiState) -> &'static str {
    match state.page {
        Some(ServicePage::Buy) => {
            "UP/DOWN  browse    LEFT/RIGHT  preview member    TAB  sell    ENTER  buy    ESC  back"
        }
        Some(ServicePage::Sell) => {
            "UP/DOWN  browse    T  filter by tag    TAB  buy    ENTER  sell    ESC  back"
        }
        Some(ServicePage::Inn) => "Y/ENTER  rest    N/ESC  leave",
        Some(ServicePage::Quantity) => {
            "LEFT/RIGHT  one    UP/DOWN  coarse step    ENTER  confirm    ESC  back"
        }
        Some(ServicePage::CoreConfirm) => "Y/ENTER  exchange    N/ESC  cancel",
        Some(ServicePage::RecipeDetail) => "ENTER  craft    ESC  back",
        Some(ServicePage::Equip) => "UP/DOWN  choose    ENTER  equip    ESC  skip",
        Some(ServicePage::Result) => "ENTER / ESC  leave",
        _ => "UP/DOWN  choose    ENTER  confirm    ESC  back",
    }
}

/// The category stripe colour a row carries, mirroring the field menu's item accents.
fn item_accent(item: &ItemDefinition) -> Color {
    match item {
        ItemDefinition::Consumable(_) => status_teal(),
        ItemDefinition::Material(_) => Color::srgb_u8(157, 139, 101),
        ItemDefinition::Key(_) => status_gold(),
        ItemDefinition::MagicCore(_) => Color::srgb_u8(126, 101, 204),
        ItemDefinition::Weapon(_) => status_ember(),
        ItemDefinition::Shield(_) => Color::srgb_u8(91, 143, 183),
        ItemDefinition::Helmet(_) => Color::srgb_u8(126, 151, 174),
        ItemDefinition::Body(_) => Color::srgb_u8(112, 126, 148),
        ItemDefinition::Accessory(_) => Color::srgb_u8(196, 116, 168),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_detail_pages_claim_the_wide_frame_and_the_rest_stay_narrow() {
        let mut state = ServiceUiState::default();
        for page in [
            ServicePage::Buy,
            ServicePage::Apothecary,
            ServicePage::RecipeDetail,
        ] {
            state.page = Some(page);
            assert_eq!(panel_width(&state), WIDE_PANEL_WIDTH, "{page:?}");
        }
        for page in [
            ServicePage::ShopMenu,
            ServicePage::Sell,
            ServicePage::MagicCore,
            ServicePage::Inn,
            ServicePage::Quantity,
            ServicePage::CoreConfirm,
            ServicePage::Equip,
            ServicePage::Result,
        ] {
            state.page = Some(page);
            assert_eq!(panel_width(&state), PANEL_WIDTH, "{page:?}");
        }
    }

    #[test]
    fn the_buy_detail_reserves_the_full_equipment_preview_height() {
        let node = buy_detail_panel_node();

        assert_eq!(node.height, px(BUY_DETAIL_HEIGHT));
        assert_eq!(node.min_height, px(BUY_DETAIL_HEIGHT));
        assert_eq!(node.max_height, px(BUY_DETAIL_HEIGHT));
        for focused in [false, true] {
            let row = equipment_preview_row_node(focused);
            assert_eq!(row.height, px(EQUIPMENT_PREVIEW_ROW_HEIGHT));
            assert_eq!(row.min_height, px(EQUIPMENT_PREVIEW_ROW_HEIGHT));
            assert_eq!(row.max_height, px(EQUIPMENT_PREVIEW_ROW_HEIGHT));
        }
    }

    #[test]
    fn every_page_names_itself_in_the_header_and_the_hint() {
        let mut state = ServiceUiState::default();
        for page in [
            ServicePage::ShopMenu,
            ServicePage::Buy,
            ServicePage::Sell,
            ServicePage::MagicCore,
            ServicePage::Inn,
            ServicePage::Apothecary,
            ServicePage::RecipeDetail,
            ServicePage::Quantity,
            ServicePage::CoreConfirm,
            ServicePage::Equip,
            ServicePage::Result,
        ] {
            state.page = Some(page);
            assert!(!service_subtitle(&state).is_empty(), "{page:?}");
            assert!(!service_hint(&state).is_empty(), "{page:?}");
        }
    }
}
