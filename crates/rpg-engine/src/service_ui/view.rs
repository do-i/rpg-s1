//! Node-tree rendering for the service overlays.
//!
//! Every page is rebuilt from [`ServiceUiState`] whenever the state, the catalog, or the game
//! changes — the same contract the field-menu pages use. Nothing here mutates game state; the
//! parent module owns every transition.

use bevy::{ecs::hierarchy::ChildSpawnerCommands, prelude::*};

use super::{
    PendingTransaction, ServicePage, ServiceRequest, ServiceUiState, ShopControl, ShopFocus,
    ToastTone, active_inn_cost, active_shop, buy_rows, is_equipment, sell_rows, shop_categories,
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
        scroll_list_node, selectable_card_colors, spawn_header_bars, spawn_meter,
        spawn_scroll_list, spawn_section_rule, spawn_status_panel, spawn_status_text,
        status_border, status_border_active, status_ember, status_faint, status_gold, status_ink,
        status_muted, status_ready, status_teal,
    },
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
mod core_shop;
mod shop;

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
    if state.request.is_none() {
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
    let portraits = if is_item_shop(&state) {
        shop::Portraits::load(&asset_server, &root, &game)
    } else {
        shop::Portraits::default()
    };

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
                        height: if is_core_shop(&state) {
                            px(core_shop::HEIGHT)
                        } else if is_item_shop(&state) {
                            px(shop::HEIGHT)
                        } else if is_apothecary(&state) {
                            px(APOTHECARY_HEIGHT)
                        } else {
                            Val::Auto
                        },
                        max_height: px(if is_core_shop(&state) {
                            core_shop::HEIGHT
                        } else if is_item_shop(&state) {
                            shop::HEIGHT
                        } else if is_apothecary(&state) {
                            APOTHECARY_HEIGHT
                        } else {
                            PANEL_MAX_HEIGHT
                        }),
                        padding: if is_core_shop(&state) {
                            UiRect {
                                left: px(26),
                                right: px(26),
                                top: px(16),
                                bottom: px(10),
                            }
                        } else if is_item_shop(&state) {
                            UiRect {
                                left: px(19),
                                right: px(19),
                                top: px(14),
                                bottom: px(10),
                            }
                        } else if is_apothecary(&state) {
                            UiRect {
                                left: px(22),
                                right: px(22),
                                top: px(20),
                                bottom: px(14),
                            }
                        } else {
                            UiRect::all(px(22))
                        },
                        border: UiRect::all(px(
                            if is_apothecary(&state) || is_item_shop(&state) || is_core_shop(&state)
                            {
                                1
                            } else {
                                2
                            },
                        )),
                        border_radius: BorderRadius::all(px(8)),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(
                            if is_apothecary(&state) || is_item_shop(&state) || is_core_shop(&state)
                            {
                                0
                            } else {
                                12
                            },
                        ),
                        ..default()
                    },
                    BackgroundColor(if is_core_shop(&state) || is_item_shop(&state) {
                        Color::srgb_u8(16, 17, 22)
                    } else if is_apothecary(&state) {
                        Color::srgba_u8(23, 24, 30, 250)
                    } else {
                        Color::srgba(0.025, 0.025, 0.09, 0.97)
                    }),
                    BorderColor::all(
                        if is_apothecary(&state) || is_item_shop(&state) || is_core_shop(&state) {
                            status_border()
                        } else {
                            theme.name_entry_border_color
                        },
                    ),
                ))
                .with_children(|panel| {
                    if is_core_shop(&state) {
                        core_shop::spawn_header(panel, &font, &game);
                        core_shop::spawn_page(panel, &font, &state, &catalog, &game);
                        core_shop::spawn_footer(panel, &font, &state);
                        if state.page == Some(ServicePage::CoreConfirm) {
                            core_shop::spawn_review(panel, &font, &state, &catalog, &game);
                        } else if state.core_shop.focus == super::core_shop::CoreFocus::Quantity {
                            core_shop::spawn_quantity_dialog(panel, &font, &state, &catalog, &game);
                        }
                    } else if is_apothecary(&state) {
                        apothecary::spawn_header(panel, &font, &game, face);
                        apothecary::spawn_page(panel, &font, &state, &catalog, &game);
                        apothecary::spawn_footer(panel, &font, &state);
                    } else if is_item_shop(&state) {
                        shop::spawn_header(panel, &font, &state, &game, face);
                        spawn_service_body(panel, &font, &state, &catalog, &game, &portraits);
                        shop::spawn_footer(panel, &font, &state);
                        if let Some(error) = state.shop_error.as_deref() {
                            spawn_shop_error(panel, &font, error);
                        }
                    } else {
                        spawn_service_header(panel, &font, &state, &game, face);
                        spawn_section_rule(panel);
                        spawn_service_body(panel, &font, &state, &catalog, &game, &portraits);
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
        Some(ServicePage::MagicCore | ServicePage::CoreConfirm) => core_shop::WIDTH,
        Some(ServicePage::Apothecary | ServicePage::RecipeDetail) => APOTHECARY_WIDTH,
        Some(
            ServicePage::Buy | ServicePage::Sell | ServicePage::ShopConfirm | ServicePage::Equip,
        ) => shop::WIDTH,
        _ => PANEL_WIDTH,
    }
}

fn is_core_shop(state: &ServiceUiState) -> bool {
    state.request == Some(ServiceRequest::Shop(DialogueShopKind::MagicCore))
        && matches!(
            state.page,
            Some(ServicePage::MagicCore | ServicePage::CoreConfirm)
        )
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
    portraits: &shop::Portraits,
) {
    match state.page {
        Some(ServicePage::Buy | ServicePage::Sell) => {
            shop::spawn_page(parent, font, state, catalog, game, portraits);
        }
        Some(ServicePage::ShopConfirm) => {
            shop::spawn_page(parent, font, state, catalog, game, portraits);
            spawn_shop_confirmation(parent, font, state, catalog, game);
        }
        Some(ServicePage::MagicCore | ServicePage::CoreConfirm) => {
            core_shop::spawn_page(parent, font, state, catalog, game)
        }
        Some(ServicePage::Inn) => spawn_inn_page(parent, font, catalog, game),
        Some(ServicePage::Apothecary | ServicePage::RecipeDetail) => {
            apothecary::spawn_page(parent, font, state, catalog, game);
        }
        Some(ServicePage::Equip) => spawn_equip_page(parent, font, state, catalog, game),
        Some(ServicePage::Result) => {
            spawn_status_text(parent, "Your stay is complete.", font, 18.0, status_ready());
        }
        None => {}
    }
}

// ── Buy ──────────────────────────────────────────────────────────────────────

fn spawn_shop_error(parent: &mut ChildSpawnerCommands<'_>, font: &Handle<Font>, error: &str) {
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(-19),
                right: px(-19),
                top: px(-14),
                bottom: px(-10),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba_u8(5, 5, 8, 190)),
            ZIndex(3),
            Name::new("Shop error popup"),
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        width: px(560),
                        min_height: px(230),
                        padding: UiRect::all(px(25)),
                        border: UiRect::all(px(2)),
                        border_radius: BorderRadius::all(px(8)),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        row_gap: px(20),
                        ..default()
                    },
                    BackgroundColor(Color::srgba_u8(23, 24, 30, 255)),
                    BorderColor::all(status_ember()),
                ))
                .with_children(|dialog| {
                    spawn_status_text(dialog, "TRANSACTION UNAVAILABLE", font, 20.0, status_gold());
                    spawn_status_text(dialog, error, font, 19.0, status_ink());
                    dialog
                        .spawn((
                            Node {
                                width: px(200),
                                height: px(44),
                                border: UiRect::all(px(1)),
                                border_radius: BorderRadius::all(px(4)),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            ShopControl::DismissError,
                            BackgroundColor(Color::srgba_u8(72, 49, 25, 255)),
                            BorderColor::all(status_border_active()),
                        ))
                        .with_children(|button| {
                            spawn_status_text(
                                button,
                                "OK  ·  ENTER / ESC",
                                font,
                                16.0,
                                status_ink(),
                            );
                        });
                });
        });
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
                left: px(-19),
                right: px(-19),
                top: px(-14),
                bottom: px(-10),
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
                            format_shop_number(state.quantity),
                            item.map_or(id, |value| item_name(value))
                        ),
                        font,
                        30.0,
                        status_ink(),
                    );
                    spawn_status_text(
                        dialog,
                        format!("{} GP", format_shop_number(amount)),
                        font,
                        28.0,
                        status_gold(),
                    );
                    spawn_status_text(
                        dialog,
                        format!("Balance after: {} GP", format_shop_number(balance)),
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

fn format_shop_number(value: u32) -> String {
    let digits = value.to_string();
    let mut formatted = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            formatted.push(',');
        }
        formatted.push(digit);
    }
    formatted
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
            ShopFocus::Mode => {
                "LEFT/RIGHT  buy or sell    ENTER / DOWN  item types    ESC  leave shop"
            }
            ShopFocus::Category => {
                "LEFT/RIGHT  choose type    ENTER / DOWN  browse items    UP  buy or sell"
            }
            ShopFocus::Items => {
                if state.page == Some(ServicePage::Sell) {
                    "ESC Back    UP / DOWN Browse    ENTER / RIGHT Quantity    T Sell tags"
                } else {
                    "ESC Back    UP / DOWN Browse    ENTER / RIGHT Quantity"
                }
            }
            ShopFocus::Detail => {
                if state.page == Some(ServicePage::Sell) {
                    "ESC Back    UP / DOWN Quantity ±1    LEFT / RIGHT Quantity ±10    ENTER Review sale"
                } else {
                    "ESC Back    UP / DOWN Quantity ±1    LEFT / RIGHT Quantity ±10    ENTER Review purchase"
                }
            }
        },
        Some(ServicePage::ShopConfirm) => "ENTER  confirm transaction    ESC  cancel",
        Some(ServicePage::Inn) => "Y/ENTER  rest    N/ESC  leave",
        Some(ServicePage::CoreConfirm) => "Y/ENTER  exchange    N/ESC  cancel",
        Some(ServicePage::RecipeDetail) => "ENTER  craft    ESC  back",
        Some(ServicePage::Equip) => "UP/DOWN  choose    ENTER  equip    ESC  skip",
        Some(ServicePage::Result) => "ENTER / ESC  leave",
        _ => "UP/DOWN  choose    ENTER  confirm    ESC  back",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shop_view(focus: ShopFocus) -> App {
        shop_view_at(focus, false, 0)
    }

    fn shop_view_at(focus: ShopFocus, selling: bool, selected: usize) -> App {
        use crate::{
            field_menu_domain::tests::{catalog_with_item_shop, game},
            runtime_map::RuntimeMapId,
            scenario_spatial::{CardinalDirection, Position},
        };
        let catalog = catalog_with_item_shop();
        let mut game = game(["story_quest_started"]);
        if selling {
            for item in catalog
                .ordered_items()
                .filter(|item| sell_price(item).is_some())
                .take(15)
            {
                let _ = game.repository_mut().add_item(item.id(), 1).unwrap();
            }
        }
        game.map_mut().move_to(
            RuntimeMapId::try_new("town_01_ardel_shop_item").unwrap(),
            Position::new(6, 6),
            CardinalDirection::Up,
        );
        let mut state = ServiceUiState::default();
        state.open(ServiceRequest::Shop(DialogueShopKind::Item), None);
        state.shop_focus = focus;
        state.selected = selected;
        if selling {
            state.page = Some(ServicePage::Sell);
        }
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.world_mut()
            .commands()
            .spawn(Node::default())
            .with_children(|root| {
                shop::spawn_header(root, &Handle::default(), &state, &game, None);
                shop::spawn_page(
                    root,
                    &Handle::default(),
                    &state,
                    &catalog,
                    &game,
                    &shop::Portraits::default(),
                );
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
                texts.contains(&"BROWSE STOCK"),
                focus != ShopFocus::Mode,
                "{focus:?}"
            );
            assert_eq!(
                texts.contains(&"THE MERCHANT’S COUNTER"),
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
            assert_eq!(all == Color::srgb_u8(73, 51, 30), focus != ShopFocus::Mode);
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
    fn shop_stock_uses_seven_large_cards_and_scrolls_to_the_last_item() {
        for selected in [0, 14] {
            let mut app = shop_view_at(ShopFocus::Items, true, selected);
            let world = app.world_mut();
            let cards = world
                .query::<(&Name, &Node, &Outline)>()
                .iter(world)
                .filter(|(name, _, _)| name.as_str().starts_with("Service item:"))
                .collect::<Vec<_>>();
            assert_eq!(cards.len(), 7);
            assert_eq!(
                cards
                    .iter()
                    .filter(|(_, _, outline)| outline.color != Color::NONE)
                    .count(),
                1
            );
            assert!(
                cards
                    .iter()
                    .all(|(_, node, _)| node.min_height == px(64) && node.height == px(64))
            );
            let list = world
                .query::<(&Name, &Node)>()
                .iter(world)
                .find(|(name, _)| name.as_str() == "Scroll list")
                .unwrap()
                .1;
            assert_eq!(
                list.flex_grow, 1.0,
                "the list must fill the stock panel's free height"
            );
            if selected == 14 {
                assert!(
                    world
                        .query::<&Text>()
                        .iter(world)
                        .any(|text| text.0 == "15 / 15")
                );
            }
        }
    }

    #[test]
    fn quantity_buttons_have_visible_boxes_and_a_labelled_count() {
        let mut app = shop_view(ShopFocus::Items);
        let world = app.world_mut();
        let controls = world
            .query::<(&ShopControl, &Node)>()
            .iter(world)
            .filter(|(control, _)| matches!(control, ShopControl::Quantity(_)))
            .collect::<Vec<_>>();
        assert_eq!(controls.len(), 2);
        assert!(
            controls
                .iter()
                .all(|(_, node)| node.width == percent(100) && node.height == px(34))
        );
        let frame = world
            .query::<(&Name, &Node)>()
            .iter(world)
            .find(|(name, _)| name.as_str() == "Shop quantity control")
            .unwrap()
            .1;
        assert_eq!(frame.width, px(78));
        assert_eq!(frame.height, px(112));
        assert_eq!(frame.flex_direction, FlexDirection::Column);
        let text = world
            .query::<&Text>()
            .iter(world)
            .map(|text| text.0.as_str())
            .collect::<Vec<_>>();
        assert!(text.contains(&"QUANTITY"));
        assert!(text.contains(&"1"));
        assert!(text.contains(&"PRICE"));
        assert!(text.contains(&"TOTAL"));
    }

    #[test]
    fn shop_money_uses_commas_and_a_small_unit_under_the_gold_price() {
        for (value, expected) in [
            (0, "0"),
            (450, "450"),
            (1_000, "1,000"),
            (12_345, "12,345"),
            (1_000_000, "1,000,000"),
            (u32::MAX, "4,294,967,295"),
        ] {
            assert_eq!(format_shop_number(value), expected);
        }
        let mut app = shop_view(ShopFocus::Items);
        let world = app.world_mut();
        let mut texts = world.query::<(&Text, &TextFont, &TextColor)>();
        let (_, font, color) = texts
            .iter(world)
            .find(|(text, _, _)| text.0 == "100")
            .unwrap();
        assert_eq!(font.font_size, FontSize::Px(17.0));
        assert_eq!(color.0, status_gold());
        let (_, font, color) = texts
            .iter(world)
            .find(|(text, font, _)| text.0 == "GP" && font.font_size == FontSize::Px(11.0))
            .unwrap();
        assert_eq!(font.font_size, FontSize::Px(11.0));
        assert_eq!(color.0, status_muted());
    }

    #[test]
    fn shop_error_is_a_modal_overlay_with_an_explicit_dismiss_button() {
        let mut app = shop_view(ShopFocus::Items);
        let before = {
            let world = app.world_mut();
            world
                .query::<(&Name, &Node)>()
                .iter(world)
                .filter(|(name, _)| {
                    name.as_str().starts_with("Shop ") || name.as_str().starts_with("Service item:")
                })
                .map(|(name, node)| (name.as_str().to_owned(), node.clone()))
                .collect::<std::collections::BTreeMap<_, _>>()
        };
        app.world_mut()
            .commands()
            .spawn((Node::default(), GlobalZIndex(5_100)))
            .with_children(|root| spawn_shop_error(root, &Handle::default(), "Not enough GP."));
        app.update();
        let world = app.world_mut();
        for (name, node) in world.query::<(&Name, &Node)>().iter(world) {
            if let Some(original) = before.get(name.as_str()) {
                assert_eq!(node, original, "error feedback must not resize {name}");
            }
        }
        let (_, node, z) = world
            .query::<(&Name, &Node, &ZIndex)>()
            .iter(world)
            .find(|(name, _, _)| name.as_str() == "Shop error popup")
            .unwrap();
        assert_eq!(node.position_type, PositionType::Absolute);
        assert!(z.0 > 0);
        assert!(
            world
                .query::<&ShopControl>()
                .iter(world)
                .any(|control| matches!(control, ShopControl::DismissError))
        );
        assert!(
            world
                .query::<&Text>()
                .iter(world)
                .any(|text| text.0 == "Not enough GP.")
        );
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
            assert_eq!(
                panel_width(&state),
                if matches!(page, ServicePage::Apothecary | ServicePage::RecipeDetail) {
                    APOTHECARY_WIDTH
                } else {
                    shop::WIDTH
                },
                "{page:?}"
            );
        }
        for page in [
            ServicePage::MagicCore,
            ServicePage::Inn,
            ServicePage::CoreConfirm,
            ServicePage::Result,
        ] {
            state.page = Some(page);
            assert_eq!(
                panel_width(&state),
                if matches!(page, ServicePage::MagicCore | ServicePage::CoreConfirm) {
                    core_shop::WIDTH
                } else {
                    PANEL_WIDTH
                },
                "{page:?}"
            );
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
        assert!(text.contains(&"Balance after: 1,150 GP"));
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
