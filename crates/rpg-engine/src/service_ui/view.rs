//! Node-tree rendering for the service overlays.
//!
//! Every page is rebuilt from [`ServiceUiState`] whenever the state, the catalog, or the game
//! changes — the same contract the field-menu pages use. Nothing here mutates game state; the
//! parent module owns every transition.

use std::num::NonZeroU32;

use bevy::{ecs::hierarchy::ChildSpawnerCommands, prelude::*};

use super::{
    PendingTransaction, SERVICE_VISIBLE_ROWS, ServicePage, ServiceRequest, ServiceUiState,
    ShopFocus, ToastTone, active_inn_cost, active_shop, buy_rows, is_equipment,
    owned_exchangeable_cores, pending_max, sell_rows, shop_categories,
    sprites::{ServiceKeeper, ServiceSprites},
};
use crate::{
    dialogue_portrait::DialoguePortrait,
    field_menu_domain::{
        FieldMenuCatalog, can_equip, derived_stats, item_description, item_name, preview_stats,
    },
    game_state::GameState,
    item_images::{spawn_item_icon, spawn_item_image},
    menu_chrome::{
        ScrollWindow, scroll_list_node, selectable_card_colors, spawn_header_bars, spawn_meter,
        spawn_scroll_list, spawn_section_rule, spawn_status_panel, spawn_status_text,
        status_border, status_border_active, status_ember, status_faint, status_gold, status_ink,
        status_muted, status_ready, status_teal,
    },
    runtime_member::RuntimeMember,
    scenario_dialogue::DialogueShopKind,
    scenario_inventory::ScenarioInventory,
    scenario_item::ItemDefinition,
    scenario_recipe::RecipeDefinition,
    scenario_root::ScenarioRoot,
    service_domain::sell_price,
    tsx_atlas_asset::TsxAtlasAsset,
    ui_theme::UiTheme,
};

mod apothecary;

#[derive(Component)]
pub(super) struct ServiceRoot;

/// Panel width for the plain list and confirmation pages (source `MODAL_W`).
const PANEL_WIDTH: f32 = 620.0;

const APOTHECARY_WIDTH: f32 = 1228.0;
const APOTHECARY_HEIGHT: f32 = 718.0;

/// Ceiling on the overlay frame, leaving a margin inside the 766px logical canvas.
///
/// This is a pixel cap rather than a percentage on purpose. The centring row above the panel
/// gives its child an indefinite cross size, so a percentage `max_height` has no basis to
/// resolve against and clamps the frame to a fraction of its content — which left the list
/// rows drawn outside their own border. The fixed gameplay canvas is 1280x766, so a pixel
/// ceiling is exact here.
const PANEL_MAX_HEIGHT: f32 = 700.0;

/// One line per member leaves room for all five previews and the quantity controls.
const EQUIPMENT_PREVIEW_ROW_HEIGHT: f32 = 28.0;

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
                        height: if is_apothecary(&state) || is_item_shop(&state) {
                            px(APOTHECARY_HEIGHT)
                        } else {
                            Val::Auto
                        },
                        max_height: px(if is_apothecary(&state) || is_item_shop(&state) {
                            APOTHECARY_HEIGHT
                        } else {
                            PANEL_MAX_HEIGHT
                        }),
                        padding: if is_apothecary(&state) || is_item_shop(&state) {
                            UiRect {
                                left: px(22),
                                right: px(22),
                                top: px(20),
                                bottom: px(14),
                            }
                        } else {
                            UiRect::all(px(22))
                        },
                        border: UiRect::all(px(if is_apothecary(&state) || is_item_shop(&state) {
                            1
                        } else {
                            2
                        })),
                        border_radius: BorderRadius::all(px(8)),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(if is_apothecary(&state) || is_item_shop(&state) {
                            0
                        } else {
                            12
                        }),
                        ..default()
                    },
                    BackgroundColor(if is_apothecary(&state) || is_item_shop(&state) {
                        Color::srgba_u8(23, 24, 30, 250)
                    } else {
                        Color::srgba(0.025, 0.025, 0.09, 0.97)
                    }),
                    BorderColor::all(if is_apothecary(&state) || is_item_shop(&state) {
                        status_border()
                    } else {
                        theme.name_entry_border_color
                    }),
                ))
                .with_children(|panel| {
                    if is_apothecary(&state) {
                        apothecary::spawn_header(panel, &font, &game, face);
                        apothecary::spawn_page(panel, &font, &state, &catalog, &game);
                        apothecary::spawn_footer(panel, &font, &state);
                    } else if is_item_shop(&state) {
                        spawn_item_shop_header(panel, &font, &state, &game, face);
                        spawn_service_body(panel, &font, &state, &catalog, &game);
                        spawn_section_rule(panel);
                        spawn_status_text(panel, service_hint(&state), &font, 12.0, status_faint());
                        spawn_toast(panel, &font, &state);
                    } else {
                        spawn_service_header(panel, &font, &state, &game, face);
                        spawn_section_rule(panel);
                        spawn_service_body(panel, &font, &state, &catalog, &game);
                        spawn_section_rule(panel);
                        spawn_status_text(panel, service_hint(&state), &font, 15.0, status_faint());
                        spawn_toast(panel, &font, &state);
                    }
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
        Some(ServicePage::Apothecary | ServicePage::RecipeDetail) => APOTHECARY_WIDTH,
        Some(
            ServicePage::Buy | ServicePage::Sell | ServicePage::ShopConfirm | ServicePage::Equip,
        ) => APOTHECARY_WIDTH,
        _ => PANEL_WIDTH,
    }
}

fn is_apothecary(state: &ServiceUiState) -> bool {
    matches!(
        state.page,
        Some(ServicePage::Apothecary | ServicePage::RecipeDetail)
    )
}

fn is_item_shop(state: &ServiceUiState) -> bool {
    matches!(
        state.request,
        Some(ServiceRequest::Shop(
            DialogueShopKind::Item | DialogueShopKind::Weapon | DialogueShopKind::Armor
        ))
    ) && matches!(
        state.page,
        Some(ServicePage::Buy | ServicePage::Sell | ServicePage::ShopConfirm | ServicePage::Equip)
    )
}

fn spawn_item_shop_header(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    game: &GameState,
    face: Option<DialoguePortrait>,
) {
    parent
        .spawn((
            Node {
                width: percent(100),
                height: px(76),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                column_gap: px(16),
                padding: UiRect::bottom(px(15)),
                margin: UiRect::bottom(px(16)),
                border: UiRect::bottom(px(1)),
                ..default()
            },
            BorderColor::all(status_border()),
            Name::new("Item shop header"),
        ))
        .with_children(|header| {
            spawn_keeper_frame(header, font, 60.0, face);
            header
                .spawn(Node {
                    width: px(12),
                    height: px(42),
                    flex_shrink: 0.0,
                    ..default()
                })
                .with_children(|bars| spawn_header_bars(bars, 42.0, 0.0));
            header
                .spawn(Node {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    row_gap: px(3),
                    ..default()
                })
                .with_children(|title| {
                    spawn_status_text(
                        title,
                        service_title(state).to_uppercase(),
                        font,
                        30.0,
                        status_ink(),
                    );
                    spawn_status_text(title, "GOODS BOUGHT AND SOLD", font, 11.0, status_muted());
                });
            header
                .spawn((
                    Node {
                        padding: UiRect::left(px(28)),
                        border: UiRect::left(px(1)),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::End,
                        row_gap: px(3),
                        ..default()
                    },
                    BorderColor::all(status_border()),
                ))
                .with_children(|wallet| {
                    spawn_status_text(wallet, "YOUR COIN", font, 11.0, status_muted());
                    spawn_status_text(
                        wallet,
                        format!("{} GP", game.repository().gp()),
                        font,
                        26.0,
                        status_gold(),
                    );
                });
        });
}

fn keeper_face(
    state: &ServiceUiState,
    sprites: &ServiceSprites,
    atlases: &Assets<TsxAtlasAsset>,
) -> Option<DialoguePortrait> {
    if let Some(portrait) = state.keeper_portrait.as_ref() {
        return Some(portrait.clone());
    }
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
    face: Option<DialoguePortrait>,
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
    face: Option<DialoguePortrait>,
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
                    image.image_node(),
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
) {
    match state.page {
        Some(ServicePage::Buy) => spawn_buy_page(parent, font, state, catalog, game),
        Some(ServicePage::Sell) => spawn_sell_page(parent, font, state, catalog, game),
        Some(ServicePage::ShopConfirm) => {
            spawn_item_shop(parent, font, state, catalog, game);
            spawn_shop_confirmation(parent, font, state, catalog, game);
        }
        Some(ServicePage::MagicCore) => spawn_core_page(parent, font, state, catalog, game),
        Some(ServicePage::Inn) => spawn_inn_page(parent, font, catalog, game),
        Some(ServicePage::Apothecary | ServicePage::RecipeDetail) => {
            apothecary::spawn_page(parent, font, state, catalog, game);
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

// ── Buy ──────────────────────────────────────────────────────────────────────

fn spawn_buy_page(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
) {
    spawn_item_shop(parent, font, state, catalog, game);
}

fn spawn_item_shop(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
) {
    let buying = state
        .pending
        .map_or(state.page == Some(ServicePage::Buy), |pending| {
            pending == PendingTransaction::Buy
        });
    let buy = buy_rows(state, catalog, game);
    let sell = sell_rows(state, catalog, game);
    let row_count = if buying { buy.len() } else { sell.len() };
    parent
        .spawn(Node {
            width: percent(100),
            flex_grow: 1.0,
            min_height: px(0),
            flex_direction: FlexDirection::Column,
            row_gap: px(10),
            ..default()
        })
        .with_children(|body| {
            body.spawn(Node {
                width: percent(100),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(8),
                ..default()
            })
            .with_children(|bar| {
                for (index, label) in ["BUY", "SELL"].into_iter().enumerate() {
                    let selected = buying == (index == 0);
                    bar.spawn((
                        Node {
                            padding: UiRect::axes(px(18), px(7)),
                            border: UiRect::all(px(1)),
                            border_radius: BorderRadius::all(px(15)),
                            ..default()
                        },
                        shop_focus_outline(selected && state.shop_focus == ShopFocus::Mode),
                        Name::new(format!("Shop mode: {label}")),
                        BackgroundColor(if selected {
                            Color::srgba_u8(72, 49, 25, 230)
                        } else {
                            Color::srgba_u8(15, 15, 20, 220)
                        }),
                        BorderColor::all(if selected {
                            status_border_active()
                        } else {
                            status_border()
                        }),
                    ))
                    .with_children(|pill| {
                        spawn_status_text(
                            pill,
                            label,
                            font,
                            14.0,
                            if selected {
                                status_gold()
                            } else {
                                status_muted()
                            },
                        )
                    });
                }
                spawn_status_text(bar, "LEFT / RIGHT", font, 12.0, status_faint());
            });
            body.spawn(Node {
                width: percent(100),
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                column_gap: px(7),
                row_gap: px(5),
                ..default()
            })
            .with_children(|pills| {
                for tab in shop_categories(state, buying, catalog, game) {
                    let selected = state.shop_stock_visible() && tab == state.shop_category;
                    pills
                        .spawn((
                            Node {
                                padding: UiRect::axes(px(11), px(5)),
                                border: UiRect::all(px(1)),
                                border_radius: BorderRadius::all(px(13)),
                                ..default()
                            },
                            shop_focus_outline(selected && state.shop_focus == ShopFocus::Category),
                            Name::new(format!("Shop category: {}", tab.label())),
                            BackgroundColor(if selected {
                                Color::srgba_u8(72, 49, 25, 220)
                            } else {
                                Color::srgba_u8(15, 15, 20, 210)
                            }),
                            BorderColor::all(if selected {
                                status_border_active()
                            } else {
                                status_border()
                            }),
                        ))
                        .with_children(|pill| {
                            spawn_status_text(
                                pill,
                                tab.label(),
                                font,
                                12.0,
                                if selected {
                                    status_gold()
                                } else {
                                    status_muted()
                                },
                            )
                        });
                }
            });
            if !state.shop_stock_visible() {
                return;
            }
            if !buying && let Some(tag) = state.sell_tag.as_deref() {
                spawn_status_text(
                    body,
                    format!("TAG FILTER: {tag}  ·  T to change"),
                    font,
                    12.0,
                    status_teal(),
                );
            }
            spawn_columns(body, |columns| {
                spawn_status_panel(
                    columns,
                    Node {
                        width: px(420),
                        flex_shrink: 0.0,
                        height: percent(100),
                        min_height: px(0),
                        ..stacked_panel_node()
                    },
                    if buying { "STOCK" } else { "CARRIED" },
                    font,
                    |panel| {
                        let window =
                            state
                                .scroll
                                .window("shop-items", state.selected, row_count, 5);
                        spawn_scroll_list(panel, window, scroll_list_node(6.0), |list| {
                            if row_count == 0 {
                                spawn_status_text(
                                    list,
                                    if buying {
                                        "No stock in this category."
                                    } else {
                                        "Nothing to sell in this category."
                                    },
                                    font,
                                    17.0,
                                    status_muted(),
                                );
                            }
                            for index in window.indices() {
                                if buying {
                                    let row = buy[index];
                                    let item = catalog.item(row.id());
                                    spawn_list_row(
                                        list,
                                        font,
                                        ListRow {
                                            item_id: Some(row.id()),
                                            selected: state.shop_item_selected()
                                                && index == state.selected,
                                            accent: item.map_or_else(status_teal, item_accent),
                                            label: item.map_or(row.id(), |i| item_name(i)),
                                            note: format!(
                                                "owned {}",
                                                game.repository().item_count(row.id())
                                            ),
                                            value: format!("{} GP", row.buy_price().get()),
                                            value_color: status_gold(),
                                            dim: false,
                                        },
                                    );
                                } else {
                                    let item = sell[index];
                                    spawn_list_row(
                                        list,
                                        font,
                                        ListRow {
                                            item_id: Some(item.id()),
                                            selected: state.shop_item_selected()
                                                && index == state.selected,
                                            accent: item_accent(item),
                                            label: item_name(item),
                                            note: format!(
                                                "x{} owned",
                                                game.repository().item_count(item.id())
                                            ),
                                            value: sell_price(item).map_or_else(
                                                || "—".to_owned(),
                                                |v| format!("{v} GP"),
                                            ),
                                            value_color: status_gold(),
                                            dim: false,
                                        },
                                    );
                                }
                            }
                        });
                        if state.shop_item_selected() {
                            spawn_window_footer(panel, font, state.selected, window);
                        } else {
                            spawn_status_text(
                                panel,
                                format!("{row_count} items"),
                                font,
                                12.0,
                                status_faint(),
                            );
                        }
                    },
                );
                if !state.shop_item_selected() || row_count == 0 {
                    return;
                }
                spawn_status_panel(
                    columns,
                    Node {
                        flex_basis: px(0),
                        flex_grow: 1.0,
                        height: percent(100),
                        min_height: px(0),
                        ..stacked_panel_node()
                    },
                    "ITEM DETAILS",
                    font,
                    |panel| {
                        let item = if buying {
                            buy.get(state.selected).and_then(|r| catalog.item(r.id()))
                        } else {
                            sell.get(state.selected).copied()
                        };
                        let Some(item) = item else {
                            spawn_status_text(
                                panel,
                                "Select an item to see its details.",
                                font,
                                18.0,
                                status_muted(),
                            );
                            return;
                        };
                        panel
                            .spawn(Node {
                                width: percent(100),
                                height: px(128),
                                min_height: px(128),
                                flex_shrink: 0.0,
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                column_gap: px(22),
                                ..default()
                            })
                            .with_children(|heading| {
                                spawn_item_image(heading, item.id());
                                heading
                                    .spawn(Node {
                                        flex_grow: 1.0,
                                        flex_basis: px(0),
                                        min_width: px(0),
                                        flex_direction: FlexDirection::Column,
                                        row_gap: px(10),
                                        ..default()
                                    })
                                    .with_children(|copy| {
                                        spawn_status_text(
                                            copy,
                                            item_name(item),
                                            font,
                                            27.0,
                                            status_gold(),
                                        );
                                        spawn_status_text(
                                            copy,
                                            item_description(item),
                                            font,
                                            17.0,
                                            status_ink(),
                                        );
                                    });
                            });
                        spawn_section_rule(panel);
                        spawn_equipment_preview(panel, font, state, catalog, game, item);
                        panel.spawn((Node {
                            width: percent(100),
                            flex_grow: 1.0,
                            ..default()
                        },));
                        panel
                            .spawn((
                                Node {
                                    width: percent(100),
                                    min_height: px(64),
                                    flex_shrink: 0.0,
                                    flex_direction: FlexDirection::Row,
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::SpaceBetween,
                                    padding: UiRect::horizontal(px(18)),
                                    border: UiRect::all(px(1)),
                                    border_radius: BorderRadius::all(px(5)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba_u8(12, 12, 16, 230)),
                                BorderColor::all(if state.shop_focus == ShopFocus::Detail {
                                    status_border_active()
                                } else {
                                    status_border()
                                }),
                            ))
                            .with_children(|qty| {
                                qty.spawn(Node {
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::Center,
                                    row_gap: px(4),
                                    ..default()
                                })
                                .with_children(|controls| {
                                    spawn_status_text(controls, "+", font, 17.0, status_gold());
                                    spawn_status_text(controls, "-", font, 17.0, status_gold());
                                });
                                spawn_status_text(
                                    qty,
                                    format!("{}", state.quantity),
                                    font,
                                    31.0,
                                    status_ink(),
                                );
                                let unit = if buying {
                                    buy.get(state.selected).map_or(0, |r| r.buy_price().get())
                                } else {
                                    sell.get(state.selected)
                                        .and_then(|r| sell_price(r))
                                        .unwrap_or(0)
                                };
                                spawn_status_text(
                                    qty,
                                    format!("{} GP total", unit.saturating_mul(state.quantity)),
                                    font,
                                    21.0,
                                    status_gold(),
                                );
                                spawn_status_text(
                                    qty,
                                    if buying {
                                        "ENTER  ·  BUY"
                                    } else {
                                        "ENTER  ·  SELL"
                                    },
                                    font,
                                    17.0,
                                    status_ink(),
                                );
                            });
                        let unit = if buying {
                            buy.get(state.selected).map_or(0, |r| r.buy_price().get())
                        } else {
                            sell.get(state.selected)
                                .and_then(|r| sell_price(r))
                                .unwrap_or(0)
                        };
                        let delta = unit.saturating_mul(state.quantity);
                        let balance = if buying {
                            game.repository().gp().saturating_sub(delta)
                        } else {
                            game.repository()
                                .gp()
                                .saturating_add(delta)
                                .min(game.repository().gp_cap())
                        };
                        spawn_status_text(
                            panel,
                            format!("{} GP per item   ·   After transaction: {balance} GP", unit),
                            font,
                            14.0,
                            status_muted(),
                        );
                    },
                );
            });
        });
}

fn spawn_sell_page(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
) {
    spawn_item_shop(parent, font, state, catalog, game);
}

fn spawn_shop_confirmation(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
) {
    let id = state.pending_id.as_deref().unwrap_or_default();
    let item = catalog.item(id);
    let buying = state.pending == Some(PendingTransaction::Buy);
    let unit = if buying {
        active_shop(state, catalog, game)
            .and_then(|shop| shop.items.iter().find(|row| row.id() == id))
            .map_or(0, |row| row.buy_price().get())
    } else {
        item.and_then(sell_price).unwrap_or(0)
    };
    let amount = unit.saturating_mul(state.quantity);
    let balance = if buying {
        game.repository().gp().saturating_sub(amount)
    } else {
        game.repository()
            .gp()
            .saturating_add(amount)
            .min(game.repository().gp_cap())
    };
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(-22),
                right: px(-22),
                top: px(-20),
                bottom: px(-14),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba_u8(5, 5, 8, 190)),
            ZIndex(2),
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        width: px(560),
                        min_height: px(300),
                        padding: UiRect::all(px(25)),
                        border: UiRect::all(px(2)),
                        border_radius: BorderRadius::all(px(8)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        flex_direction: FlexDirection::Column,
                        row_gap: px(18),
                        ..default()
                    },
                    BackgroundColor(Color::srgba_u8(23, 24, 30, 255)),
                    BorderColor::all(status_border_active()),
                ))
                .with_children(|dialog| {
                    spawn_status_text(dialog, "CONFIRM TRANSACTION", font, 16.0, status_gold());
                    spawn_status_text(
                        dialog,
                        format!(
                            "{} × {}",
                            state.quantity,
                            item.map_or(id, |value| item_name(value))
                        ),
                        font,
                        30.0,
                        status_ink(),
                    );
                    spawn_status_text(dialog, format!("{} GP", amount), font, 28.0, status_gold());
                    spawn_status_text(
                        dialog,
                        format!("Balance after: {balance} GP"),
                        font,
                        18.0,
                        status_ready(),
                    );
                    dialog
                        .spawn(Node {
                            width: percent(100),
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::Center,
                            column_gap: px(12),
                            ..default()
                        })
                        .with_children(|buttons| {
                            for (label, selected) in [
                                (
                                    if buying {
                                        "BUY  ·  ENTER"
                                    } else {
                                        "SELL  ·  ENTER"
                                    },
                                    true,
                                ),
                                ("CANCEL  ·  ESC", false),
                            ] {
                                buttons
                                    .spawn((
                                        Node {
                                            padding: UiRect::axes(px(20), px(11)),
                                            border: UiRect::all(px(if selected { 2 } else { 1 })),
                                            border_radius: BorderRadius::all(px(5)),
                                            ..default()
                                        },
                                        BackgroundColor(if selected {
                                            Color::srgba_u8(86, 61, 31, 255)
                                        } else {
                                            Color::srgba_u8(14, 14, 18, 255)
                                        }),
                                        BorderColor::all(if selected {
                                            status_border_active()
                                        } else {
                                            status_border()
                                        }),
                                    ))
                                    .with_children(|button| {
                                        spawn_status_text(
                                            button,
                                            label,
                                            font,
                                            17.0,
                                            if selected {
                                                status_gold()
                                            } else {
                                                status_muted()
                                            },
                                        )
                                    });
                            }
                        });
                });
        });
}

/// The bordered party panel Python draws beside an equippable purchase.
///
/// Each member shows what currently fills the slot and the signed change every stat would take.
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
            // Keep the member, worn gear, and deltas on one line for a full party.
            row.spawn(Node {
                width: percent(45),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                column_gap: px(8),
                ..default()
            })
            .with_children(|line| {
                spawn_status_text(line, member.name(), font, 16.0, status_ink());
                spawn_status_text(line, subtitle, font, 12.0, status_muted());
            });
            row.spawn(Node {
                flex_grow: 1.0,
                justify_content: JustifyContent::End,
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
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: px(12),
        flex_shrink: 0.0,
        padding: UiRect::axes(px(10), px(3)),
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

// ── Magic core ───────────────────────────────────────────────────────────────

fn spawn_core_page(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
) {
    let rows = owned_exchangeable_cores(catalog, game);
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
        let window =
            state
                .scroll
                .window("exchange", state.selected, rows.len(), SERVICE_VISIBLE_ROWS);
        spawn_scroll_list(panel, window, scroll_list_node(5.0), |panel| {
            for index in window.indices() {
                let item = &rows[index];
                let ItemDefinition::MagicCore(core) = item else {
                    continue;
                };
                let Some(rate) = core.exchange_rate.map(NonZeroU32::get) else {
                    continue;
                };
                spawn_list_row(
                    panel,
                    font,
                    ListRow {
                        item_id: None,
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
        });
        spawn_window_footer(panel, font, state.selected, window);
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
            Some(ItemDefinition::MagicCore(core)) => core.exchange_rate.map(NonZeroU32::get),
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
    item_id: Option<&'a str>,
    selected: bool,
    accent: Color,
    label: &'a str,
    note: String,
    value: String,
    value_color: Color,
    dim: bool,
}

/// An outline marks focus without changing a control's measured size.
fn shop_focus_outline(focused: bool) -> Outline {
    Outline {
        width: px(1),
        offset: px(0),
        color: if focused {
            status_border_active()
        } else {
            Color::NONE
        },
    }
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
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            selectable_card_colors(row.selected),
            shop_focus_outline(row.selected),
            Name::new(format!("Service item: {}", row.label)),
        ))
        .with_children(|node| {
            if let Some(id) = row.item_id {
                spawn_item_icon(node, id, 32.0);
            }
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
    window: ScrollWindow,
) {
    let len = window.len;
    let position = format!(
        "{:02} OF {len:02}",
        if len == 0 {
            0
        } else {
            selected.min(len - 1) + 1
        }
    );
    let text = if window.scrolls() {
        let shown = window.indices();
        format!(
            "{position}   ·   SHOWING {:02}-{:02}",
            shown.start + 1,
            shown.end
        )
    } else {
        position
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
            flex_grow: 1.0,
            min_height: px(0),
            flex_direction: FlexDirection::Row,
            column_gap: px(14),
            ..default()
        })
        .with_children(columns);
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
        Some(ServicePage::Buy) => "BUYING",
        Some(ServicePage::Sell) => "SELLING",
        Some(ServicePage::MagicCore) => "EXCHANGE",
        Some(ServicePage::Inn) => "REST",
        Some(ServicePage::Apothecary) => "RECIPES",
        Some(ServicePage::RecipeDetail) => "RECIPE DETAIL",
        Some(ServicePage::Quantity) => "QUANTITY",
        Some(ServicePage::CoreConfirm) => "CONFIRM",
        Some(ServicePage::ShopConfirm) => "REVIEW PURCHASE",
        Some(ServicePage::Equip) => "EQUIP",
        Some(ServicePage::Result) => "RESTED",
        None => "",
    }
}

fn service_hint(state: &ServiceUiState) -> &'static str {
    match state.page {
        Some(ServicePage::Buy | ServicePage::Sell) => match state.shop_focus {
            ShopFocus::Mode => "LEFT/RIGHT  buy or sell    DOWN  item types    ESC  leave shop",
            ShopFocus::Category => "LEFT/RIGHT  choose type    UP/DOWN  buy or browse items",
            ShopFocus::Items => {
                "UP/DOWN  browse    RIGHT  details    ENTER  review 1    T  sell tags    ESC  item types"
            }
            ShopFocus::Detail => {
                "UP/DOWN  quantity ±1    LEFT/RIGHT  quantity ±10    ENTER  review    ESC  item list"
            }
        },
        Some(ServicePage::ShopConfirm) => "ENTER  confirm transaction    ESC  cancel",
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

    fn shop_view(focus: ShopFocus) -> App {
        use crate::{
            field_menu_domain::tests::{catalog_with_item_shop, game},
            runtime_map::RuntimeMapId,
            scenario_spatial::{CardinalDirection, Position},
        };
        let catalog = catalog_with_item_shop();
        let mut game = game(["story_quest_started"]);
        game.map_mut().move_to(
            RuntimeMapId::try_new("town_01_ardel_shop_item").unwrap(),
            Position::new(6, 6),
            CardinalDirection::Up,
        );
        let mut state = ServiceUiState::default();
        state.open(ServiceRequest::Shop(DialogueShopKind::Item), None);
        state.shop_focus = focus;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.world_mut()
            .commands()
            .spawn(Node::default())
            .with_children(|root| {
                spawn_item_shop(root, &Handle::default(), &state, &catalog, &game);
            });
        app.update();
        app
    }

    #[test]
    fn shop_landing_categories_and_stock_render_only_the_requested_sections() {
        for focus in [
            ShopFocus::Mode,
            ShopFocus::Category,
            ShopFocus::Items,
            ShopFocus::Detail,
        ] {
            let mut app = shop_view(focus);
            let world = app.world_mut();
            let texts = world
                .query::<&Text>()
                .iter(world)
                .map(|text| text.0.as_str())
                .collect::<Vec<_>>();
            assert!(texts.contains(&"BUY"));
            assert!(texts.contains(&"SELL"));
            assert!(texts.contains(&"All"));
            assert!(texts.contains(&"Recovery"));
            assert!(texts.contains(&"Status"));
            assert!(!texts.contains(&"Core"));
            assert!(!texts.contains(&"Key"));
            assert_eq!(
                texts.contains(&"STOCK"),
                focus != ShopFocus::Mode,
                "{focus:?}"
            );
            assert_eq!(
                texts.contains(&"ITEM DETAILS"),
                matches!(focus, ShopFocus::Items | ShopFocus::Detail),
                "{focus:?}"
            );
            let selected_rows = world
                .query::<(&Name, &Outline)>()
                .iter(world)
                .filter(|(name, outline)| {
                    name.as_str().starts_with("Service item:") && outline.color != Color::NONE
                })
                .map(|(name, _)| name.as_str())
                .collect::<Vec<_>>();
            if matches!(focus, ShopFocus::Items | ShopFocus::Detail) {
                assert_eq!(selected_rows, ["Service item: Potion"]);
            } else {
                assert!(selected_rows.is_empty());
            }
            let all = world
                .query::<(&Name, &BackgroundColor)>()
                .iter(world)
                .find(|(name, _)| name.as_str() == "Shop category: All")
                .unwrap()
                .1
                .0;
            assert_eq!(
                all == Color::srgba_u8(72, 49, 25, 220),
                focus != ShopFocus::Mode
            );
        }
    }

    #[test]
    fn shop_focus_changes_never_change_pill_or_stock_row_geometry() {
        let mut previous = std::collections::BTreeMap::<String, Node>::new();
        for focus in [
            ShopFocus::Mode,
            ShopFocus::Category,
            ShopFocus::Items,
            ShopFocus::Detail,
            ShopFocus::Category,
        ] {
            let mut app = shop_view(focus);
            let world = app.world_mut();
            for (name, node) in world.query::<(&Name, &Node)>().iter(world) {
                let name = name.as_str();
                if name.starts_with("Shop mode:")
                    || name.starts_with("Shop category:")
                    || name.starts_with("Service item:")
                {
                    if let Some(before) = previous.get(name) {
                        assert_eq!(node, before, "{name} changes geometry at {focus:?}");
                    }
                    previous.insert(name.to_owned(), node.clone());
                }
            }
        }
    }

    #[test]
    fn the_detail_pages_claim_the_wide_frame_and_the_rest_stay_narrow() {
        let mut state = ServiceUiState::default();
        for page in [
            ServicePage::Buy,
            ServicePage::Sell,
            ServicePage::ShopConfirm,
            ServicePage::Equip,
            ServicePage::Apothecary,
            ServicePage::RecipeDetail,
        ] {
            state.page = Some(page);
            assert_eq!(panel_width(&state), APOTHECARY_WIDTH, "{page:?}");
        }
        for page in [
            ServicePage::MagicCore,
            ServicePage::Inn,
            ServicePage::Quantity,
            ServicePage::CoreConfirm,
            ServicePage::Result,
        ] {
            state.page = Some(page);
            assert_eq!(panel_width(&state), PANEL_WIDTH, "{page:?}");
        }
    }

    #[test]
    fn every_page_names_itself_in_the_header_and_the_hint() {
        let mut state = ServiceUiState::default();
        for page in [
            ServicePage::Buy,
            ServicePage::Sell,
            ServicePage::MagicCore,
            ServicePage::Inn,
            ServicePage::Apothecary,
            ServicePage::RecipeDetail,
            ServicePage::Quantity,
            ServicePage::CoreConfirm,
            ServicePage::ShopConfirm,
            ServicePage::Equip,
            ServicePage::Result,
        ] {
            state.page = Some(page);
            assert!(!service_subtitle(&state).is_empty(), "{page:?}");
            assert!(!service_hint(&state).is_empty(), "{page:?}");
        }
    }

    #[test]
    fn transaction_review_stays_above_the_shop_and_displays_the_staged_sale() {
        use crate::field_menu_domain::tests::{catalog, game};
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        let catalog = catalog();
        let mut game = game([]);
        let _ = game.repository_mut().add_gp(1_000).unwrap();
        let state = ServiceUiState {
            page: Some(ServicePage::ShopConfirm),
            pending: Some(PendingTransaction::Sell),
            pending_id: Some("potion".to_owned()),
            quantity: 3,
            ..default()
        };
        let root = app
            .world_mut()
            .commands()
            .spawn((Node::default(), GlobalZIndex(5_100)))
            .with_children(|root| {
                spawn_shop_confirmation(root, &Handle::default(), &state, &catalog, &game);
            })
            .id();
        app.update();
        let world = app.world_mut();
        let overlay = world.get::<Children>(root).unwrap()[0];
        // GlobalZIndex(2) moves the entire dialog behind the service root at 5100.
        assert!(world.get::<GlobalZIndex>(overlay).is_none());
        assert!(world.get::<ZIndex>(overlay).is_some_and(|z| z.0 > 0));
        let text = world
            .query::<&Text>()
            .iter(world)
            .map(|text| text.0.as_str())
            .collect::<Vec<_>>();
        assert!(text.contains(&"3 × Potion"));
        assert!(text.contains(&"150 GP"));
        assert!(text.contains(&"Balance after: 1150 GP"));
        assert!(text.contains(&"SELL  ·  ENTER"));
    }

    #[test]
    fn the_header_shows_the_clerk_the_player_spoke_to() {
        let clerk = DialoguePortrait {
            image: Handle::default(),
            rect: Rect::new(0.0, 10.0, 64.0, 42.0),
        };
        let mut state = ServiceUiState::default();
        state.open(
            ServiceRequest::Shop(DialogueShopKind::Weapon),
            Some(clerk.clone()),
        );
        let face = keeper_face(
            &state,
            &ServiceSprites::default(),
            &Assets::<TsxAtlasAsset>::default(),
        );
        assert_eq!(face, Some(clerk));
    }
}
