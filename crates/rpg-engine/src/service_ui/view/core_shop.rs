//! Illustrated exchange routes with separate quantity and confirmation dialogs.

use super::*;
use crate::{
    service_domain::{CORE_RANK_UP_COST, next_core_id, sale_value},
    service_ui::core_shop::{self as logic, CoreControl, CoreFocus, CoreMode},
};

pub(super) const WIDTH: f32 = 1232.0;
pub(super) const HEIGHT: f32 = 712.0;
const LIST_WIDTH: f32 = 438.0;
const QUANTITY_WIDTH: f32 = 166.0;
const QUANTITY_HEIGHT: f32 = 128.0;

fn background() -> Color {
    Color::srgb_u8(20, 21, 26)
}
fn border() -> Color {
    Color::srgb_u8(91, 75, 53)
}
fn card() -> Color {
    Color::srgb_u8(34, 34, 41)
}
fn selected_color() -> Color {
    Color::srgb_u8(56, 46, 32)
}
fn dark() -> Color {
    Color::srgb_u8(16, 16, 21)
}

/// Reuse the exact catalog artwork selected in the approved mockups.
fn core_art(id: &str) -> &str {
    match id {
        "mc_xs" => "cave_crystal",
        "mc_s" => "ember_core",
        "mc_m" => "ancient_core",
        "mc_l" => "golem_core",
        "mc_xl" => "void_core",
        _ => id,
    }
}

fn tier(id: &str) -> String {
    id.trim_start_matches("mc_").to_uppercase()
}

pub(super) fn spawn_header(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    game: &GameState,
) {
    parent
        .spawn((
            Node {
                height: px(66),
                width: percent(100),
                flex_shrink: 0.0,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Start,
                border: UiRect::bottom(px(1)),
                ..default()
            },
            BorderColor::all(border()),
        ))
        .with_children(|header| {
            header
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(5),
                    ..default()
                })
                .with_children(|title| {
                    spawn_status_text(title, "MAGIC CORE EXCHANGE", font, 29.0, status_gold());
                    spawn_status_text(
                        title,
                        "Choose a core, then choose what you receive.",
                        font,
                        15.0,
                        status_muted(),
                    );
                });
            header
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::End,
                    row_gap: px(5),
                    ..default()
                })
                .with_children(|wallet| {
                    spawn_status_text(wallet, "YOUR COIN", font, 11.0, status_muted());
                    shop::spawn_money(wallet, font, game.repository().gp(), 24.0, status_muted());
                });
        });
}

pub(super) fn spawn_page(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
) {
    parent
        .spawn(Node {
            width: percent(100),
            flex_grow: 1.0,
            min_height: px(0),
            margin: UiRect::top(px(42)),
            flex_direction: FlexDirection::Column,
            row_gap: px(10),
            ..default()
        })
        .with_children(|body| {
            body.spawn(Node {
                width: percent(100),
                column_gap: px(20),
                ..default()
            })
            .with_children(|headings| {
                headings
                    .spawn(Node {
                        width: px(LIST_WIDTH),
                        flex_shrink: 0.0,
                        ..default()
                    })
                    .with_children(|heading| {
                        spawn_status_text(heading, "CARRIED CORES", font, 13.0, status_gold());
                    });
                spawn_status_text(headings, "CHOOSE YOUR EXCHANGE", font, 13.0, status_gold());
            });
            body.spawn(Node {
                width: percent(100),
                flex_grow: 1.0,
                min_height: px(0),
                column_gap: px(20),
                ..default()
            })
            .with_children(|columns| {
                spawn_list(columns, font, state, catalog, game);
                spawn_detail(columns, font, state, catalog, game);
            });
        });
}

fn spawn_list(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
) {
    parent
        .spawn((
            Node {
                width: px(LIST_WIDTH),
                height: percent(100),
                flex_shrink: 0.0,
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(12)),
                row_gap: px(6),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(5)),
                ..default()
            },
            BackgroundColor(background()),
            BorderColor::all(border()),
            Name::new("Core inventory"),
        ))
        .with_children(|panel| {
            for (index, item) in logic::rows(catalog).into_iter().enumerate() {
                let chosen = index == state.selected;
                let id = item.id();
                panel
                    .spawn((
                        Node {
                            width: percent(100),
                            flex_basis: px(0),
                            flex_grow: 1.0,
                            min_height: px(0),
                            padding: UiRect::axes(px(6), px(6)),
                            align_items: AlignItems::Center,
                            column_gap: px(10),
                            border: UiRect::all(px(if chosen { 2 } else { 1 })),
                            border_radius: BorderRadius::all(px(4)),
                            ..default()
                        },
                        BackgroundColor(if chosen { selected_color() } else { card() }),
                        BorderColor::all(if chosen {
                            status_border_active()
                        } else {
                            border()
                        }),
                        CoreControl::Item(index),
                        Name::new(format!("Core row: {id}")),
                    ))
                    .with_children(|row| {
                        spawn_item_icon(row, core_art(id), 76.0);
                        row.spawn(Node {
                            flex_grow: 1.0,
                            min_width: px(0),
                            flex_direction: FlexDirection::Column,
                            row_gap: px(9),
                            ..default()
                        })
                        .with_children(|label| {
                            spawn_status_text(label, item_name(item), font, 21.0, status_ink());
                            let owned = game.repository().item_count(id);
                            let note = if id == "mc_xl" {
                                format!("OWNED {owned} · TOP RANK")
                            } else if sale_value(item).is_none() {
                                format!(
                                    "OWNED {owned} · 50 -> 1 {}",
                                    next_core_id(id).map_or_else(String::new, tier)
                                )
                            } else {
                                format!("OWNED {owned}")
                            };
                            spawn_status_text(label, note, font, 12.0, status_muted());
                        });
                        if let Some(value) = sale_value(item) {
                            shop::spawn_money(row, font, value, 19.0, status_muted());
                        } else if next_core_id(id).is_some() {
                            spawn_status_text(row, "50 : 1", font, 13.0, status_muted());
                        }
                    });
            }
        });
}

fn spawn_detail(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
) {
    let focused = state.core_shop.focus == CoreFocus::Exchange;
    parent
        .spawn((
            Node {
                flex_basis: px(0),
                flex_grow: 1.0,
                height: percent(100),
                min_width: px(0),
                padding: UiRect::all(px(24)),
                border: UiRect::all(px(if focused { 2 } else { 1 })),
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                border_radius: BorderRadius::all(px(5)),
                ..default()
            },
            BackgroundColor(background()),
            BorderColor::all(if focused {
                status_border_active()
            } else {
                border()
            }),
            Name::new("Core exchange selector"),
        ))
        .with_children(|detail| {
            let Some(item) = logic::selected(state, catalog) else {
                return;
            };
            spawn_status_text(detail, item_name(item), font, 22.0, status_gold());
            spawn_status_text(
                detail,
                "Choose a destination. Exchange rates are shown per unit or batch.",
                font,
                15.0,
                status_muted(),
            );
            detail
                .spawn(Node {
                    width: percent(100),
                    height: px(369),
                    flex_shrink: 0.0,
                    margin: UiRect::top(px(16)),
                    ..default()
                })
                .with_children(|graph| {
                    graph
                        .spawn(Node {
                            position_type: PositionType::Absolute,
                            left: px(0),
                            top: px(52),
                            width: px(174),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            row_gap: px(12),
                            ..default()
                        })
                        .with_children(|source| {
                            spawn_status_text(source, "YOU GIVE", font, 12.0, status_muted());
                            spawn_item_image(source, core_art(item.id()));
                            spawn_status_text(source, item_name(item), font, 20.0, status_ink());
                            spawn_status_text(
                                source,
                                format!("{} in your pack", game.repository().item_count(item.id())),
                                font,
                                15.0,
                                status_muted(),
                            );
                            if let Some(reason) = logic::unavailable(state, catalog, game) {
                                spawn_status_text(source, reason, font, 14.0, status_muted());
                            }
                        });
                    graph.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(166),
                            top: px(147),
                            width: px(38),
                            height: px(2),
                            ..default()
                        },
                        BackgroundColor(status_muted()),
                        Pickable::IGNORE,
                    ));
                    graph.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(204),
                            top: px(82),
                            width: px(2),
                            height: px(205),
                            ..default()
                        },
                        BackgroundColor(status_muted()),
                        Pickable::IGNORE,
                    ));
                    for (mode, top) in [(CoreMode::Sell, 0.0), (CoreMode::RankUp, 205.0)] {
                        spawn_route(graph, font, state, catalog, item, mode, top);
                    }
                });
        });
}

fn spawn_route(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    catalog: &FieldMenuCatalog,
    item: &ItemDefinition,
    mode: CoreMode,
    top: f32,
) {
    let chosen = state.core_shop.mode == mode;
    let accent = if chosen {
        status_gold()
    } else {
        status_faint()
    };
    spawn_arrow(
        parent,
        Node {
            position_type: PositionType::Absolute,
            left: px(204),
            top: px(top + 81.0),
            width: px(96),
            height: px(if chosen { 3 } else { 2 }),
            ..default()
        },
        accent,
        if chosen { 3.0 } else { 2.0 },
    );
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(314),
                right: px(0),
                top: px(top),
                height: px(164),
                padding: UiRect {
                    left: px(12),
                    right: px(12),
                    top: px(12),
                    bottom: px(6),
                },
                border: UiRect::all(px(if chosen { 2 } else { 1 })),
                border_radius: BorderRadius::all(px(5)),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            CoreControl::Mode(mode),
            BackgroundColor(if chosen { selected_color() } else { card() }),
            BorderColor::all(if chosen {
                status_border_active()
            } else {
                border()
            }),
            Name::new(format!("Core route: {mode:?}")),
        ))
        .with_children(|route| {
            spawn_status_text(
                route,
                if mode == CoreMode::Sell {
                    "SELL FOR GP"
                } else {
                    "RANK UP"
                },
                font,
                12.0,
                if chosen {
                    status_gold()
                } else {
                    status_muted()
                },
            );
            route
                .spawn(Node {
                    width: percent(100),
                    align_items: AlignItems::Center,
                    column_gap: px(12),
                    ..default()
                })
                .with_children(|target| {
                    let output = if mode == CoreMode::Sell {
                        Some("rare_coin")
                    } else {
                        next_core_id(item.id()).map(core_art)
                    };
                    if let Some(output) = output {
                        spawn_item_image(target, output);
                    }
                    target
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: px(12),
                            flex_grow: 1.0,
                            min_width: px(0),
                            ..default()
                        })
                        .with_children(|labels| {
                            if mode == CoreMode::Sell {
                                if let Some(value) = sale_value(item) {
                                    shop::spawn_money(labels, font, value, 22.0, status_muted());
                                    spawn_status_text(
                                        labels,
                                        format!("Give 1 × {}", tier(item.id())),
                                        font,
                                        16.0,
                                        status_muted(),
                                    );
                                    spawn_status_text(
                                        labels,
                                        "Per core",
                                        font,
                                        15.0,
                                        status_muted(),
                                    );
                                } else {
                                    spawn_status_text(
                                        labels,
                                        "Unavailable",
                                        font,
                                        20.0,
                                        status_muted(),
                                    );
                                    spawn_status_text(
                                        labels,
                                        "Only XS and S sell for GP.",
                                        font,
                                        15.0,
                                        status_muted(),
                                    );
                                }
                            } else if let Some(output) = next_core_id(item.id()) {
                                spawn_status_text(
                                    labels,
                                    catalog.item(output).map_or("Unavailable", item_name),
                                    font,
                                    21.0,
                                    if chosen { status_gold() } else { status_ink() },
                                );
                                spawn_status_text(
                                    labels,
                                    format!("Give {CORE_RANK_UP_COST} × {}", tier(item.id())),
                                    font,
                                    16.0,
                                    status_muted(),
                                );
                                spawn_status_text(
                                    labels,
                                    format!("Receive 1 × {}", tier(output)),
                                    font,
                                    15.0,
                                    status_muted(),
                                );
                            } else {
                                spawn_status_text(labels, "TOP RANK", font, 22.0, status_muted());
                                spawn_status_text(
                                    labels,
                                    "XL has no higher core.",
                                    font,
                                    16.0,
                                    status_muted(),
                                );
                            }
                        });
                });
        });
}

fn spawn_arrow(parent: &mut ChildSpawnerCommands<'_>, node: Node, color: Color, thickness: f32) {
    parent
        .spawn((node, BackgroundColor(color), Pickable::IGNORE))
        .with_children(|arrow| {
            for (offset, rotation) in [(-4.0, 45.0), (4.0, -45.0)] {
                arrow.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        right: px(-1),
                        top: px(offset),
                        width: px(12),
                        height: px(thickness),
                        ..default()
                    },
                    BackgroundColor(color),
                    UiTransform::from_rotation(Rot2::degrees(rotation)),
                    Pickable::IGNORE,
                ));
            }
        });
}

fn spawn_button(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    label: &str,
    control: CoreControl,
    primary: bool,
) {
    parent
        .spawn((
            Node {
                flex_grow: if primary { 1.5 } else { 1.0 },
                flex_basis: px(0),
                height: px(45),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            control,
            BackgroundColor(if primary { selected_color() } else { dark() }),
            BorderColor::all(if primary {
                status_border_active()
            } else {
                border()
            }),
        ))
        .with_children(|button| {
            spawn_status_text(
                button,
                label,
                font,
                15.0,
                if primary {
                    status_gold()
                } else {
                    status_muted()
                },
            );
        });
}

/// Each modal covers the whole service frame and sits above its inactive controls.
fn spawn_dialog(
    parent: &mut ChildSpawnerCommands<'_>,
    name: &'static str,
    content: impl FnOnce(&mut ChildSpawnerCommands<'_>),
) {
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(-26),
                right: px(-26),
                top: px(-16),
                bottom: px(-10),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba_u8(5, 5, 8, 210)),
            ZIndex(3),
            Name::new(name),
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        width: px(620),
                        min_height: px(464),
                        padding: UiRect::all(px(26)),
                        border: UiRect::all(px(1)),
                        border_radius: BorderRadius::all(px(8)),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(20),
                        ..default()
                    },
                    BackgroundColor(Color::srgb_u8(23, 24, 30)),
                    BorderColor::all(status_border()),
                ))
                .with_children(content);
        });
}

pub(super) fn spawn_quantity_dialog(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
) {
    let Some(item) = logic::selected(state, catalog) else {
        return;
    };
    spawn_dialog(parent, "Core quantity dialog", |dialog| {
        spawn_status_text(dialog, "CHOOSE QUANTITY", font, 25.0, status_gold());
        let output = next_core_id(item.id());
        let subtitle = if state.core_shop.mode == CoreMode::Sell {
            format!("Sell · {} to GP", item_name(item))
        } else {
            format!(
                "Rank up · {} to Magic Core ({})",
                item_name(item),
                output.map_or_else(String::new, tier)
            )
        };
        spawn_status_text(dialog, subtitle, font, 17.0, status_ink());
        dialog.spawn((
            Node {
                height: px(1),
                width: percent(100),
                ..default()
            },
            BackgroundColor(border()),
        ));
        dialog
            .spawn(Node {
                width: percent(100),
                height: px(208),
                column_gap: px(24),
                ..default()
            })
            .with_children(|body| {
                body.spawn(Node {
                    width: px(QUANTITY_WIDTH),
                    flex_shrink: 0.0,
                    flex_direction: FlexDirection::Column,
                    row_gap: px(12),
                    ..default()
                })
                .with_children(|amount| {
                    spawn_status_text(
                        amount,
                        if state.core_shop.mode == CoreMode::Sell {
                            "QUANTITY"
                        } else {
                            "BATCHES"
                        },
                        font,
                        12.0,
                        status_muted(),
                    );
                    amount
                        .spawn((
                            Node {
                                width: px(QUANTITY_WIDTH),
                                height: px(QUANTITY_HEIGHT),
                                flex_shrink: 0.0,
                                flex_direction: FlexDirection::Column,
                                border: UiRect::all(px(1)),
                                border_radius: BorderRadius::all(px(5)),
                                ..default()
                            },
                            BackgroundColor(dark()),
                            BorderColor::all(status_border()),
                            Name::new("Vertical core quantity"),
                        ))
                        .with_children(|picker| {
                            for (label, control, size) in [
                                ("+".to_owned(), Some(CoreControl::Quantity(1)), 24.0),
                                (state.quantity.to_string(), None, 36.0),
                                ("-".to_owned(), Some(CoreControl::Quantity(-1)), 24.0),
                            ] {
                                let mut cell = picker.spawn(Node {
                                    width: percent(100),
                                    flex_grow: 1.0,
                                    flex_basis: px(0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                });
                                if let Some(control) = control {
                                    cell.insert(control);
                                }
                                cell.with_children(|label_node| {
                                    spawn_status_text(
                                        label_node,
                                        label,
                                        font,
                                        size,
                                        if control.is_some() {
                                            status_gold()
                                        } else {
                                            status_ink()
                                        },
                                    );
                                });
                            }
                        });
                });
                body.spawn(Node {
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    min_width: px(0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(10),
                    ..default()
                })
                .with_children(|summary| {
                    spawn_status_text(summary, "YOU GIVE", font, 12.0, status_muted());
                    let consumed =
                        state
                            .quantity
                            .saturating_mul(if state.core_shop.mode == CoreMode::Sell {
                                1
                            } else {
                                CORE_RANK_UP_COST
                            });
                    spawn_status_text(
                        summary,
                        format!("{consumed} × {}", item_name(item)),
                        font,
                        21.0,
                        status_ink(),
                    );
                    spawn_status_text(summary, "YOU RECEIVE", font, 12.0, status_muted());
                    summary
                        .spawn(Node {
                            align_items: AlignItems::Center,
                            column_gap: px(12),
                            ..default()
                        })
                        .with_children(|reward| {
                            if state.core_shop.mode == CoreMode::Sell {
                                spawn_item_image(reward, "rare_coin");
                                shop::spawn_money(
                                    reward,
                                    font,
                                    sale_value(item).unwrap_or(0).saturating_mul(state.quantity),
                                    24.0,
                                    status_muted(),
                                );
                            } else if let Some(output) = output {
                                spawn_item_image(reward, core_art(output));
                                spawn_status_text(
                                    reward,
                                    format!("{} × Magic Core ({})", state.quantity, tier(output)),
                                    font,
                                    20.0,
                                    status_gold(),
                                );
                            }
                        });
                });
            });
        let max = logic::max_quantity(catalog, game, item.id(), state.core_shop.mode);
        spawn_status_text(
            dialog,
            format!(
                "Available: {max} {}",
                if state.core_shop.mode == CoreMode::Sell {
                    "cores"
                } else {
                    "batches"
                }
            ),
            font,
            16.0,
            status_muted(),
        );
        if let Some(reason) = logic::unavailable(state, catalog, game) {
            spawn_status_text(dialog, reason, font, 14.0, status_muted());
        }
        dialog
            .spawn(Node {
                width: percent(100),
                column_gap: px(14),
                ..default()
            })
            .with_children(|buttons| {
                spawn_button(buttons, font, "BACK · ESC", CoreControl::Back, false);
                spawn_button(
                    buttons,
                    font,
                    "REVIEW EXCHANGE · ENTER",
                    CoreControl::Review,
                    true,
                );
            });
    });
}

pub(super) fn spawn_footer(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
) {
    parent
        .spawn(Node {
            height: px(42),
            margin: UiRect::top(px(14)),
            width: percent(100),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|footer| {
            if let Some(toast) = &state.toast {
                spawn_status_text(
                    footer,
                    &toast.text,
                    font,
                    14.0,
                    if toast.tone == ToastTone::Good {
                        status_ready()
                    } else {
                        status_muted()
                    },
                );
            }
        });
}

pub(super) fn spawn_review(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
) {
    let Some(item) = state.pending_id.as_deref().and_then(|id| catalog.item(id)) else {
        return;
    };
    let rank_up = state.pending == Some(PendingTransaction::RankUp);
    let output = next_core_id(item.id());
    spawn_dialog(parent, "Core transaction review", |dialog| {
        spawn_status_text(dialog, "REVIEW EXCHANGE", font, 25.0, status_gold());
        spawn_status_text(
            dialog,
            if rank_up {
                format!(
                    "Rank up · {} to Magic Core ({})",
                    item_name(item),
                    output.map_or_else(String::new, tier)
                )
            } else {
                format!("Sell · {} to GP", item_name(item))
            },
            font,
            17.0,
            status_ink(),
        );
        dialog.spawn((
            Node {
                height: px(1),
                width: percent(100),
                ..default()
            },
            BackgroundColor(border()),
        ));
        dialog
            .spawn(Node {
                width: percent(100),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|exchange| {
                exchange
                    .spawn(Node {
                        width: px(190),
                        align_items: AlignItems::Center,
                        flex_direction: FlexDirection::Column,
                        row_gap: px(10),
                        ..default()
                    })
                    .with_children(|source| {
                        spawn_item_image(source, core_art(item.id()));
                        spawn_status_text(
                            source,
                            format!(
                                "{} × {}",
                                state.quantity.saturating_mul(if rank_up {
                                    CORE_RANK_UP_COST
                                } else {
                                    1
                                }),
                                tier(item.id())
                            ),
                            font,
                            20.0,
                            status_ink(),
                        );
                    });
                spawn_arrow(
                    exchange,
                    Node {
                        width: px(70),
                        height: px(3),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    status_gold(),
                    3.0,
                );
                exchange
                    .spawn(Node {
                        width: px(190),
                        align_items: AlignItems::Center,
                        flex_direction: FlexDirection::Column,
                        row_gap: px(10),
                        ..default()
                    })
                    .with_children(|target| {
                        if rank_up {
                            if let Some(output) = output {
                                spawn_item_image(target, core_art(output));
                                spawn_status_text(
                                    target,
                                    format!("{} × {}", state.quantity, tier(output)),
                                    font,
                                    20.0,
                                    status_ink(),
                                );
                            }
                        } else {
                            spawn_item_image(target, "rare_coin");
                            shop::spawn_money(
                                target,
                                font,
                                sale_value(item).unwrap_or(0).saturating_mul(state.quantity),
                                20.0,
                                status_muted(),
                            );
                        }
                    });
            });
        let after =
            if rank_up {
                format!(
                    "After exchange: {} × {} · {} × {} · {} GP",
                    game.repository()
                        .item_count(item.id())
                        .saturating_sub(state.quantity.saturating_mul(CORE_RANK_UP_COST)),
                    tier(item.id()),
                    output
                        .map_or(0, |id| game.repository().item_count(id))
                        .saturating_add(state.quantity),
                    output.map_or_else(String::new, tier),
                    format_shop_number(game.repository().gp())
                )
            } else {
                format!(
                    "After exchange: {} × {} · {} GP",
                    game.repository()
                        .item_count(item.id())
                        .saturating_sub(state.quantity),
                    tier(item.id()),
                    format_shop_number(game.repository().gp().saturating_add(
                        sale_value(item).unwrap_or(0).saturating_mul(state.quantity)
                    ))
                )
            };
        spawn_status_text(dialog, after, font, 17.0, status_muted());
        dialog
            .spawn(Node {
                width: percent(100),
                column_gap: px(14),
                ..default()
            })
            .with_children(|buttons| {
                spawn_button(buttons, font, "BACK · ESC", CoreControl::Back, false);
                spawn_button(
                    buttons,
                    font,
                    "CONFIRM EXCHANGE · ENTER",
                    CoreControl::Confirm,
                    true,
                );
            });
    });
}

#[cfg(test)]
mod tests;
