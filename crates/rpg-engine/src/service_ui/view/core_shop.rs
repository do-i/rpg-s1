//! Approved two-column core counter. Review uses an overlay; its inline slot stays blank.

use super::*;
use crate::{
    service_domain::{CORE_RANK_UP_COST, next_core_id, sale_value},
    service_ui::core_shop::{self as logic, CoreControl, CoreFocus, CoreMode},
};

pub(super) const WIDTH: f32 = 1232.0;
pub(super) const HEIGHT: f32 = 712.0;
const LIST_WIDTH: f32 = 438.0;
const QUANTITY_WIDTH: f32 = 122.0;
const QUANTITY_HEIGHT: f32 = 122.0;

fn background() -> Color {
    Color::srgb_u8(29, 29, 36)
}
fn border() -> Color {
    Color::srgb_u8(84, 69, 47)
}
fn card() -> Color {
    Color::srgb_u8(34, 34, 41)
}
fn selected_color() -> Color {
    Color::srgb_u8(67, 48, 28)
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
    parent.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            bottom: px(0),
            width: px(7),
            ..default()
        },
        BackgroundColor(status_ember()),
        Pickable::IGNORE,
        Name::new("Core frame accent"),
    ));
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
                        "Choose a core and select a transaction: sell for GP or rank up.",
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
            spawn_status_text(body, "CARRIED CORES", font, 13.0, status_gold());
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
                            padding: UiRect::axes(px(10), px(8)),
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
                        spawn_item_icon(row, core_art(id), 56.0);
                        row.spawn((
                            Node {
                                width: px(4),
                                height: px(58),
                                flex_shrink: 0.0,
                                ..default()
                            },
                            BackgroundColor(if id == "mc_xl" {
                                status_faint()
                            } else {
                                status_teal()
                            }),
                        ));
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

fn spawn_mode(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    mode: CoreMode,
    selected: bool,
) {
    parent
        .spawn((
            Node {
                flex_basis: px(0),
                flex_grow: 1.0,
                height: percent(100),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(px(if selected { 2 } else { 1 })),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            CoreControl::Mode(mode),
            BackgroundColor(if selected { selected_color() } else { dark() }),
            BorderColor::all(if selected {
                status_border_active()
            } else {
                border()
            }),
        ))
        .with_children(|tab| {
            spawn_status_text(
                tab,
                if mode == CoreMode::Sell {
                    "SELL FOR GP"
                } else {
                    "RANK UP"
                },
                font,
                13.0,
                if selected {
                    status_gold()
                } else {
                    status_muted()
                },
            );
        });
}

fn spawn_detail(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
) {
    let focused = state.core_shop.focus == CoreFocus::Quantity;
    let mode = state.core_shop.mode;
    parent
        .spawn((
            Node {
                flex_basis: px(0),
                flex_grow: 1.0,
                height: percent(100),
                min_width: px(0),
                min_height: px(0),
                padding: UiRect::all(px(26)),
                border: UiRect::all(px(if focused { 2 } else { 1 })),
                flex_direction: FlexDirection::Column,
                border_radius: BorderRadius::all(px(5)),
                ..default()
            },
            BackgroundColor(background()),
            BorderColor::all(if focused {
                status_border_active()
            } else {
                border()
            }),
            Name::new("Core transaction detail"),
        ))
        .with_children(|detail| {
            let Some(item) = logic::selected(state, catalog) else {
                return;
            };
            detail
                .spawn(Node {
                    height: px(78),
                    flex_shrink: 0.0,
                    column_gap: px(12),
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|identity| {
                    spawn_item_icon(identity, core_art(item.id()), 78.0);
                    identity
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            flex_grow: 1.0,
                            min_width: px(0),
                            flex_basis: px(0),
                            row_gap: px(9),
                            ..default()
                        })
                        .with_children(|title| {
                            spawn_status_text(title, item_name(item), font, 26.0, status_gold());
                            spawn_status_text(
                                title,
                                item_description(item),
                                font,
                                15.0,
                                status_ink(),
                            );
                        });
                });
            detail
                .spawn(Node {
                    height: px(36),
                    flex_shrink: 0.0,
                    column_gap: px(12),
                    ..default()
                })
                .with_children(|modes| {
                    spawn_mode(modes, font, CoreMode::Sell, mode == CoreMode::Sell);
                    spawn_mode(modes, font, CoreMode::RankUp, mode == CoreMode::RankUp);
                });
            detail
                .spawn(Node {
                    height: px(36),
                    flex_shrink: 0.0,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                })
                .with_children(|rate| {
                    spawn_status_text(
                        rate,
                        if mode == CoreMode::Sell {
                            "UNIT VALUE"
                        } else {
                            "RANK-UP RATE"
                        },
                        font,
                        12.0,
                        status_muted(),
                    );
                    if mode == CoreMode::Sell {
                        if let Some(value) = sale_value(item) {
                            shop::spawn_money(rate, font, value, 20.0, status_muted());
                        } else {
                            spawn_status_text(rate, "NOT SOLD FOR GP", font, 13.0, status_muted());
                        }
                    } else {
                        let label = next_core_id(item.id()).map_or_else(
                            || "TOP RANK".to_owned(),
                            |next| format!("50 {} -> 1 {}", tier(item.id()), tier(next)),
                        );
                        spawn_status_text(rate, label, font, 17.0, status_gold());
                    }
                });
            detail
                .spawn(Node {
                    height: px(26),
                    flex_shrink: 0.0,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|label| {
                    spawn_status_text(
                        label,
                        if mode == CoreMode::Sell {
                            "QUANTITY"
                        } else {
                            "BATCHES"
                        },
                        font,
                        12.0,
                        status_muted(),
                    );
                });
            detail
                .spawn(Node {
                    height: px(QUANTITY_HEIGHT),
                    flex_shrink: 0.0,
                    column_gap: px(30),
                    ..default()
                })
                .with_children(|controls| {
                    spawn_quantity(controls, font, state, focused);
                    spawn_summary(controls, font, state, game, item);
                });
            if let Some(reason) = logic::unavailable(state, catalog, game) {
                detail
                    .spawn(Node {
                        margin: UiRect::top(px(12)),
                        ..default()
                    })
                    .with_children(|note| {
                        spawn_status_text(note, reason, font, 14.0, status_muted());
                    });
            }
            // The approved review-action area occupies space but has no visible frame or text.
            detail.spawn((
                Node {
                    flex_grow: 1.0,
                    min_height: px(10),
                    ..default()
                },
                Name::new("Reserved core review space"),
            ));
        });
}

fn spawn_quantity(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    focused: bool,
) {
    parent
        .spawn((
            Node {
                width: px(QUANTITY_WIDTH),
                height: px(QUANTITY_HEIGHT),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                border: UiRect::all(px(if focused { 2 } else { 1 })),
                border_radius: BorderRadius::all(px(5)),
                ..default()
            },
            BackgroundColor(dark()),
            BorderColor::all(if focused {
                status_border_active()
            } else {
                border()
            }),
            Name::new("Vertical core quantity"),
        ))
        .with_children(|picker| {
            for (label, control, size) in [
                ("+".to_owned(), CoreControl::Quantity(1), 24.0),
                (state.quantity.to_string(), CoreControl::Review, 27.0),
                ("-".to_owned(), CoreControl::Quantity(-1), 24.0),
            ] {
                picker
                    .spawn((
                        Node {
                            width: percent(100),
                            flex_basis: px(0),
                            flex_grow: 1.0,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        control,
                    ))
                    .with_children(|button| {
                        spawn_status_text(
                            button,
                            label,
                            font,
                            size,
                            if control == CoreControl::Review {
                                status_ink()
                            } else {
                                status_gold()
                            },
                        );
                    });
            }
        });
}

fn spawn_balance(parent: &mut ChildSpawnerCommands<'_>, font: &Handle<Font>, value: u32) {
    parent
        .spawn(Node {
            align_items: AlignItems::Baseline,
            justify_content: JustifyContent::End,
            column_gap: px(6),
            ..default()
        })
        .with_children(|money| {
            spawn_status_text(money, format_shop_number(value), font, 19.0, status_teal());
            spawn_status_text(money, "GP", font, 12.0, status_muted());
        });
}

fn spawn_summary(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    game: &GameState,
    item: &ItemDefinition,
) {
    parent
        .spawn(Node {
            flex_basis: px(0),
            flex_grow: 1.0,
            min_width: px(0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        })
        .with_children(|summary| {
            if state.core_shop.mode == CoreMode::RankUp && next_core_id(item.id()).is_none() {
                spawn_status_text(summary, "TOP RANK", font, 12.0, status_muted());
                spawn_status_text(
                    summary,
                    "XL cores have no higher rank.",
                    font,
                    17.0,
                    status_ink(),
                );
                return;
            }

            if state.core_shop.mode == CoreMode::Sell {
                let amount = sale_value(item).unwrap_or(0).saturating_mul(state.quantity);
                spawn_status_text(summary, "PAYOUT", font, 12.0, status_muted());
                shop::spawn_money(summary, font, amount, 32.0, status_muted());
                spawn_status_text(summary, "After exchange", font, 14.0, status_muted());
                spawn_balance(
                    summary,
                    font,
                    game.repository()
                        .gp()
                        .saturating_add(amount)
                        .min(game.repository().gp_cap()),
                );
            } else {
                spawn_status_text(summary, "CONSUME", font, 12.0, status_muted());
                summary
                    .spawn(Node {
                        justify_content: JustifyContent::End,
                        ..default()
                    })
                    .with_children(|value| {
                        spawn_status_text(
                            value,
                            format!(
                                "{} × {}",
                                state.quantity.saturating_mul(CORE_RANK_UP_COST),
                                item_name(item)
                            ),
                            font,
                            17.0,
                            status_ink(),
                        );
                    });
                spawn_status_text(summary, "YOU RECEIVE", font, 12.0, status_muted());
                if let Some(output) = next_core_id(item.id()) {
                    summary
                        .spawn(Node {
                            align_items: AlignItems::Center,
                            column_gap: px(5),
                            ..default()
                        })
                        .with_children(|reward| {
                            spawn_item_icon(reward, core_art(output), 36.0);
                            spawn_status_text(
                                reward,
                                format!("{} × Magic Core ({})", state.quantity, tier(output)),
                                font,
                                18.0,
                                status_gold(),
                            );
                        });
                } else {
                    spawn_status_text(summary, "No higher rank", font, 17.0, status_muted());
                }
            }
        });
}

pub(super) fn spawn_footer(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
) {
    parent.spawn((Node { height: px(42), margin: UiRect::top(px(14)), width: percent(100), flex_shrink: 0.0, flex_direction: FlexDirection::Column, row_gap: px(4), border: UiRect::top(px(1)), ..default() }, BorderColor::all(border()))).with_children(|footer| {
        if let Some(toast) = &state.toast { spawn_status_text(footer, &toast.text, font, 13.0, if toast.tone == ToastTone::Good { status_ready() } else { status_ember() }); }
        let hint = if state.page == Some(ServicePage::CoreConfirm) { "ENTER / CLICK CONFIRM  ·  ESC CANCEL" }
        else if state.core_shop.focus == CoreFocus::Items { "UP / DOWN SELECT CORE  ·  RIGHT / ENTER TRANSACTION  ·  ESC LEAVE" }
        else { "UP / DOWN QUANTITY  ·  LEFT / RIGHT MODE  ·  ENTER / CLICK COUNT REVIEW  ·  ESC CORES" };
        spawn_status_text(footer, hint, font, 13.0, status_muted());
    });
}

pub(super) fn spawn_review(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
) {
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(-26),
                right: px(-26),
                top: px(-16),
                bottom: px(-10),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba_u8(5, 5, 8, 210)),
            ZIndex(3),
            Name::new("Core transaction review"),
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        width: px(620),
                        min_height: px(300),
                        padding: UiRect::all(px(25)),
                        border: UiRect::all(px(2)),
                        border_radius: BorderRadius::all(px(8)),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(20),
                        ..default()
                    },
                    BackgroundColor(background()),
                    BorderColor::all(status_border_active()),
                ))
                .with_children(|dialog| {
                    spawn_status_text(dialog, "REVIEW EXCHANGE", font, 24.0, status_gold());
                    if let Some(item) = state.pending_id.as_deref().and_then(|id| catalog.item(id))
                    {
                        let text = if state.pending == Some(PendingTransaction::RankUp) {
                            format!(
                                "{} × {} -> {} × Magic Core ({})",
                                state.quantity.saturating_mul(CORE_RANK_UP_COST),
                                item_name(item),
                                state.quantity,
                                next_core_id(item.id()).map_or_else(String::new, tier)
                            )
                        } else {
                            format!(
                                "{} × {} -> {} GP",
                                state.quantity,
                                item_name(item),
                                sale_value(item).unwrap_or(0).saturating_mul(state.quantity)
                            )
                        };
                        spawn_status_text(dialog, text, font, 20.0, status_ink());
                        if state.pending == Some(PendingTransaction::RankUp) {
                            if let Some(output) = next_core_id(item.id()) {
                                spawn_status_text(
                                    dialog,
                                    format!(
                                        "Output stack after exchange: {} / {}",
                                        game.repository()
                                            .item_count(output)
                                            .saturating_add(state.quantity),
                                        game.repository().item_quantity_cap()
                                    ),
                                    font,
                                    16.0,
                                    status_muted(),
                                );
                            }
                        } else {
                            spawn_status_text(
                                dialog,
                                "GP and core counts change only when you confirm.",
                                font,
                                16.0,
                                status_muted(),
                            );
                        }
                    }
                    dialog
                        .spawn(Node {
                            width: percent(100),
                            column_gap: px(14),
                            ..default()
                        })
                        .with_children(|buttons| {
                            for (label, control) in [
                                ("CONFIRM · ENTER", CoreControl::Confirm),
                                ("CANCEL · ESC", CoreControl::Cancel),
                            ] {
                                buttons
                                    .spawn((
                                        Node {
                                            flex_grow: 1.0,
                                            flex_basis: px(0),
                                            height: px(44),
                                            align_items: AlignItems::Center,
                                            justify_content: JustifyContent::Center,
                                            border: UiRect::all(px(1)),
                                            border_radius: BorderRadius::all(px(4)),
                                            ..default()
                                        },
                                        control,
                                        BackgroundColor(if control == CoreControl::Confirm {
                                            selected_color()
                                        } else {
                                            dark()
                                        }),
                                        BorderColor::all(border()),
                                    ))
                                    .with_children(|button| {
                                        spawn_status_text(button, label, font, 16.0, status_gold());
                                    });
                            }
                        });
                });
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_menu_domain::tests::{catalog, game};

    fn view(mode: CoreMode, quantity: u32, review: bool) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        let mut game = game([]);
        let _ = game.repository_mut().add_item("mc_s", 100).unwrap();
        let mut state = ServiceUiState::default();
        state.open(ServiceRequest::Shop(DialogueShopKind::MagicCore), None);
        state.selected = 1;
        state.core_shop.mode = mode;
        state.core_shop.focus = CoreFocus::Quantity;
        state.quantity = quantity;
        state.pending_id = Some("mc_s".to_owned());
        state.pending = Some(if mode == CoreMode::RankUp {
            PendingTransaction::RankUp
        } else {
            PendingTransaction::Exchange
        });
        let catalog = catalog();
        app.world_mut()
            .commands()
            .spawn(Node::default())
            .with_children(|root| {
                spawn_header(root, &Handle::default(), &game);
                spawn_page(root, &Handle::default(), &state, &catalog, &game);
                if review {
                    spawn_review(root, &Handle::default(), &state, &catalog, &game);
                }
            });
        app.update();
        app
    }

    #[test]
    fn five_full_height_rows_and_vertical_quantity_preserve_the_approved_hidden_action_area() {
        for quantity in [1, 9, 10, 99, 100] {
            let mut app = view(CoreMode::Sell, quantity, false);
            let world = app.world_mut();
            let rows = world
                .query::<(&CoreControl, &Node)>()
                .iter(world)
                .filter(|(control, _)| matches!(control, CoreControl::Item(_)))
                .collect::<Vec<_>>();
            assert_eq!(rows.len(), 5);
            assert!(
                rows.iter()
                    .all(|(_, node)| node.flex_grow == 1.0 && node.flex_basis == px(0))
            );
            let (picker, children) = world
                .query::<(&Name, &Node, &Children)>()
                .iter(world)
                .find(|(name, _, _)| name.as_str() == "Vertical core quantity")
                .map(|(_, node, children)| (node, children))
                .unwrap();
            assert_eq!(picker.width, px(122));
            assert_eq!(picker.height, px(122));
            assert_eq!(picker.flex_direction, FlexDirection::Column);
            let controls = children
                .iter()
                .map(|child| *world.get::<CoreControl>(child).unwrap())
                .collect::<Vec<_>>();
            assert_eq!(
                controls,
                [
                    CoreControl::Quantity(1),
                    CoreControl::Review,
                    CoreControl::Quantity(-1)
                ]
            );
            let texts = world
                .query::<&Text>()
                .iter(world)
                .map(|text| text.0.as_str())
                .collect::<Vec<_>>();
            assert!(!texts.contains(&"REVIEW EXCHANGE"));
            assert!(!texts.iter().any(|text| text.contains("LAYOUT")
                || text.contains("SELECTED CORE")
                || text.contains("OF 05")));
        }
    }

    #[test]
    fn rank_up_review_names_both_stacks_and_sits_above_the_service_panel() {
        let mut app = view(CoreMode::RankUp, 2, true);
        let world = app.world_mut();
        let texts = world
            .query::<&Text>()
            .iter(world)
            .map(|text| text.0.as_str())
            .collect::<Vec<_>>();
        assert!(texts.contains(&"50 S -> 1 M"));
        assert!(texts.contains(&"100 × Magic Core (S)"));
        assert!(texts.contains(&"2 × Magic Core (M)"));
        assert!(texts.contains(&"100 × Magic Core (S) -> 2 × Magic Core (M)"));
        let layer = world
            .query::<(&Name, &ZIndex)>()
            .iter(world)
            .find(|(name, _)| name.as_str() == "Core transaction review")
            .unwrap()
            .1;
        assert!(layer.0 > 0);
        let controls = world
            .query::<&CoreControl>()
            .iter(world)
            .filter(|control| control.in_review())
            .count();
        assert_eq!(controls, 2);
    }
}
