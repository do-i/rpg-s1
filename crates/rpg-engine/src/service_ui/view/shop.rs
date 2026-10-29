//! The shop's stock, merchant counter, and optional party equipment rail.

use std::collections::BTreeMap;

use super::*;
use crate::{field_menu_domain::item_stat_bonuses, scenario_path::ScenarioRelativePath};

pub(super) const WIDTH: f32 = 1260.0;
pub(super) const HEIGHT: f32 = 746.0;
const STOCK_WIDTH: f32 = 360.0;
const COUNTER_WIDTH: f32 = 474.0;
const PARTY_WIDTH: f32 = 362.0;
const VISIBLE_ROWS: usize = 7;
const CARD_HEIGHT: f32 = 64.0;

#[derive(Default)]
pub(super) struct Portraits(BTreeMap<String, Handle<Image>>);

impl Portraits {
    pub(super) fn load(server: &AssetServer, root: &ScenarioRoot, game: &GameState) -> Self {
        Self(
            game.party()
                .members()
                .filter_map(|member| {
                    let path = ScenarioRelativePath::try_from(format!(
                        "media/images/{}_profile.png",
                        member.id()
                    ))
                    .ok()?;
                    Some((member.id().to_owned(), server.load(root.resolve(&path))))
                })
                .collect(),
        )
    }
}

fn buying(state: &ServiceUiState) -> bool {
    state
        .pending
        .map_or(state.page == Some(ServicePage::Buy), |p| {
            p == PendingTransaction::Buy
        })
}

pub(super) fn spawn_header(
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
                height: px(68),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                column_gap: px(16),
                padding: UiRect::bottom(px(14)),
                margin: UiRect::bottom(px(10)),
                border: UiRect::bottom(px(1)),
                ..default()
            },
            BorderColor::all(shop_border()),
            Name::new("Item shop header"),
        ))
        .with_children(|header| {
            if let Some(face) = face {
                header.spawn((
                    face.image_node(),
                    Node {
                        width: px(52),
                        height: px(52),
                        flex_shrink: 0.0,
                        ..default()
                    },
                ));
            } else {
                spawn_keeper_frame(header, font, 52.0, None);
            }
            header.spawn((
                Node {
                    width: px(4),
                    height: px(53),
                    flex_shrink: 0.0,
                    ..default()
                },
                BackgroundColor(status_ember()),
            ));
            header
                .spawn(Node {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    row_gap: px(2),
                    ..default()
                })
                .with_children(|title| {
                    spawn_status_text(
                        title,
                        service_title(state).to_uppercase(),
                        font,
                        27.0,
                        status_ink(),
                    );
                    spawn_status_text(
                        title,
                        "SUPPLIES FOR THE ROAD AHEAD",
                        font,
                        11.0,
                        status_muted(),
                    );
                });
            header
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::End,
                    row_gap: px(2),
                    ..default()
                })
                .with_children(|wallet| {
                    spawn_status_text(wallet, "YOUR COIN", font, 10.0, status_muted());
                    spawn_money(wallet, font, game.repository().gp(), 25.0, status_gold());
                });
            header
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(0),
                        top: px(0),
                        width: percent(100),
                        height: px(53),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        column_gap: px(16),
                        ..default()
                    },
                    Name::new("Shop centered modes"),
                ))
                .with_children(|modes| {
                    for (is_buy, label) in [(true, "BUY"), (false, "SELL")] {
                        spawn_pill(
                            modes,
                            font,
                            label,
                            buying(state) == is_buy,
                            state.shop_focus == ShopFocus::Mode && buying(state) == is_buy,
                            Some(ShopControl::Mode(is_buy)),
                            true,
                        );
                    }
                });
        });
}

fn spawn_pill(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    label: &str,
    selected: bool,
    focused: bool,
    control: Option<ShopControl>,
    mode: bool,
) {
    let mut pill = parent.spawn((
        Node {
            min_width: px(if mode { 69 } else { 48 }),
            height: px(28),
            padding: UiRect::horizontal(px(13)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            flex_shrink: 0.0,
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(14)),
            ..default()
        },
        BackgroundColor(if selected {
            Color::srgb_u8(73, 51, 30)
        } else {
            Color::srgb_u8(23, 23, 28)
        }),
        BorderColor::all(if selected {
            status_border_active()
        } else {
            shop_border()
        }),
        shop_focus_outline(focused),
        Name::new(format!(
            "Shop {}: {label}",
            if mode { "mode" } else { "category" }
        )),
    ));
    if let Some(control) = control {
        pill.insert(control);
    }
    pill.with_children(|pill| {
        spawn_status_text(
            pill,
            label,
            font,
            13.0,
            if selected {
                status_gold()
            } else {
                status_muted()
            },
        )
    });
}

pub(super) fn spawn_page(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
    portraits: &Portraits,
) {
    let buying = buying(state);
    let buy = buy_rows(state, catalog, game);
    let sell = sell_rows(state, catalog, game);
    let row_count = if buying { buy.len() } else { sell.len() };
    parent
        .spawn(Node {
            width: percent(100),
            flex_grow: 1.0,
            min_height: px(0),
            flex_direction: FlexDirection::Column,
            row_gap: px(16),
            ..default()
        })
        .with_children(|body| {
            body.spawn(Node {
                width: percent(100),
                min_height: px(28),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                column_gap: px(6),
                ..default()
            })
            .with_children(|bar| {
                for tab in shop_categories(state, buying, catalog, game) {
                    spawn_pill(
                        bar,
                        font,
                        tab.label(),
                        state.shop_stock_visible() && tab == state.shop_category,
                        state.shop_focus == ShopFocus::Category && tab == state.shop_category,
                        Some(ShopControl::Category(tab)),
                        false,
                    );
                }
                bar.spawn(Node {
                    flex_grow: 1.0,
                    ..default()
                });
                spawn_status_text(
                    bar,
                    match state.shop_focus {
                        ShopFocus::Mode => "LEFT / RIGHT MODE   ENTER / DOWN ITEM TYPES",
                        ShopFocus::Category => "LEFT / RIGHT TYPE   ENTER / DOWN BROWSE",
                        ShopFocus::Items => "UP / DOWN BROWSE   ENTER QUANTITY",
                        ShopFocus::Detail => "UP / DOWN QUANTITY   ENTER REVIEW",
                    },
                    font,
                    10.0,
                    status_faint(),
                );
            });
            if !state.shop_stock_visible() {
                return;
            }
            body.spawn(Node {
                width: percent(100),
                flex_grow: 1.0,
                min_height: px(0),
                column_gap: px(12),
                ..default()
            })
            .with_children(|columns| {
                columns
                    .spawn((
                        panel_node(STOCK_WIDTH, 10.0),
                        panel_color(),
                        BorderColor::all(shop_border()),
                        Name::new("Shop stock panel"),
                    ))
                    .with_children(|panel| {
                        panel
                            .spawn(Node {
                                width: percent(100),
                                height: px(24),
                                flex_shrink: 0.0,
                                justify_content: JustifyContent::SpaceBetween,
                                align_items: AlignItems::Center,
                                ..default()
                            })
                            .with_children(|heading| {
                                spawn_status_text(
                                    heading,
                                    if buying {
                                        "BROWSE STOCK"
                                    } else {
                                        "CARRIED GOODS"
                                    },
                                    font,
                                    13.0,
                                    status_gold(),
                                );
                                spawn_status_text(
                                    heading,
                                    format!(
                                        "{:02} / {row_count:02}",
                                        if row_count == 0 {
                                            0
                                        } else {
                                            state.selected + 1
                                        }
                                    ),
                                    font,
                                    11.0,
                                    status_muted(),
                                );
                            });
                        if !buying && let Some(tag) = state.sell_tag.as_deref() {
                            spawn_status_text(
                                panel,
                                format!("TAG: {tag}"),
                                font,
                                12.0,
                                status_teal(),
                            );
                        }
                        let window = state.scroll.window(
                            "shop-items",
                            state.selected,
                            row_count,
                            VISIBLE_ROWS,
                        );
                        spawn_scroll_list(
                            panel,
                            window,
                            Node {
                                flex_grow: 1.0,
                                flex_shrink: 1.0,
                                min_height: px(0),
                                width: px(348),
                                margin: UiRect::top(px(39)),
                                ..scroll_list_node(4.0)
                            },
                            |list| {
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
                                    let (id, label, price) = if buying {
                                        let row = buy[index];
                                        (
                                            row.id(),
                                            catalog.item(row.id()).map_or(row.id(), item_name),
                                            row.buy_price().get(),
                                        )
                                    } else {
                                        let item = sell[index];
                                        (item.id(), item_name(item), sell_price(item).unwrap_or(0))
                                    };
                                    spawn_stock_card(
                                        list,
                                        font,
                                        id,
                                        label,
                                        game.repository().item_count(id),
                                        price,
                                        state.shop_item_selected() && index == state.selected,
                                        index,
                                    );
                                }
                            },
                        );
                    });
                let item = if !state.shop_item_selected() {
                    None
                } else if buying {
                    buy.get(state.selected).and_then(|r| catalog.item(r.id()))
                } else {
                    sell.get(state.selected).copied()
                };
                let Some(item) = item else {
                    return;
                };
                let unit = if buying {
                    buy.get(state.selected).map_or(0, |r| r.buy_price().get())
                } else {
                    sell_price(item).unwrap_or(0)
                };
                spawn_counter(columns, font, state, game, item, unit, buying);
                if is_equipment(item) {
                    spawn_party(columns, font, catalog, game, portraits, item);
                }
            });
        });
}

fn panel_node(width: f32, horizontal_padding: f32) -> Node {
    Node {
        width: px(width),
        height: percent(100),
        flex_shrink: 0.0,
        min_height: px(0),
        flex_direction: FlexDirection::Column,
        padding: UiRect {
            left: px(horizontal_padding),
            right: px(horizontal_padding),
            top: px(12),
            bottom: px(6),
        },
        border: UiRect::all(px(1)),
        border_radius: BorderRadius::all(px(5)),
        ..default()
    }
}

fn shop_border() -> Color {
    Color::srgb_u8(91, 75, 53)
}

fn panel_color() -> BackgroundColor {
    BackgroundColor(Color::srgb_u8(20, 21, 26))
}

#[expect(
    clippy::too_many_arguments,
    reason = "each stock card carries an item, its price, and its navigation index"
)]
fn spawn_stock_card(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    id: &str,
    label: &str,
    owned: u32,
    price: u32,
    selected: bool,
    index: usize,
) {
    parent
        .spawn((
            Node {
                width: percent(100),
                height: px(CARD_HEIGHT),
                min_height: px(CARD_HEIGHT),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(8), px(4)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(if selected {
                Color::srgb_u8(69, 50, 29)
            } else {
                Color::srgb_u8(25, 26, 32)
            }),
            BorderColor::all(if selected {
                status_border_active()
            } else {
                shop_border()
            }),
            shop_focus_outline(selected),
            ShopControl::Item(index),
            Name::new(format!("Service item: {label}")),
        ))
        .with_children(|card| {
            spawn_item_icon(card, id, 54.0);
            card.spawn(Node {
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(4),
                margin: UiRect::right(px(12)),
                ..default()
            })
            .with_children(|copy| {
                spawn_status_text(copy, label, font, 18.0, status_ink());
                spawn_status_text(
                    copy,
                    format!("owned {}", format_shop_number(owned)),
                    font,
                    13.0,
                    status_muted(),
                );
            });
            card.spawn(Node {
                min_width: px(43),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::End,
                row_gap: px(3),
                ..default()
            })
            .with_children(|cost| {
                spawn_status_text(cost, format_shop_number(price), font, 17.0, status_gold());
                spawn_status_text(cost, "GP", font, 11.0, status_muted());
            });
        });
}

fn spawn_counter(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    game: &GameState,
    item: &ItemDefinition,
    unit: u32,
    buying: bool,
) {
    parent
        .spawn((
            panel_node(COUNTER_WIDTH, 16.0),
            panel_color(),
            BorderColor::all(shop_border()),
            Name::new("Shop counter panel"),
        ))
        .with_children(|counter| {
            counter
                .spawn((
                    Node {
                        width: percent(100),
                        height: px(24),
                        flex_shrink: 0.0,
                        margin: UiRect::bottom(px(12)),
                        border: UiRect::bottom(px(1)),
                        ..default()
                    },
                    BorderColor::all(shop_border()),
                ))
                .with_children(|heading| {
                    spawn_status_text(heading, "THE MERCHANT’S COUNTER", font, 13.0, status_gold());
                });
            counter
                .spawn((
                    Node {
                        width: percent(100),
                        height: px(174),
                        flex_shrink: 0.0,
                        padding: UiRect::all(px(12)),
                        column_gap: px(24),
                        border: UiRect::all(px(1)),
                        border_radius: BorderRadius::all(px(4)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb_u8(28, 27, 25)),
                    BorderColor::all(shop_border()),
                    Name::new("Shop item showcase"),
                ))
                .with_children(|showcase| {
                    spawn_item_image(showcase, item.id());
                    showcase
                        .spawn(Node {
                            flex_grow: 1.0,
                            flex_basis: px(0),
                            min_width: px(0),
                            flex_direction: FlexDirection::Column,
                            row_gap: px(6),
                            ..default()
                        })
                        .with_children(|copy| {
                            spawn_status_text(copy, item_name(item), font, 23.0, status_gold());
                            spawn_status_text(
                                copy,
                                item_description(item),
                                font,
                                15.0,
                                status_ink(),
                            );
                            copy.spawn(Node {
                                flex_grow: 1.0,
                                min_height: px(4),
                                ..default()
                            });
                            copy.spawn(Node {
                                width: percent(100),
                                align_items: AlignItems::Baseline,
                                justify_content: JustifyContent::SpaceBetween,
                                column_gap: px(8),
                                ..default()
                            })
                            .with_children(|price| {
                                spawn_status_text(price, "PRICE", font, 10.0, status_muted());
                                spawn_money(price, font, unit, 19.0, status_gold());
                            });
                        });
                });
            if is_equipment(item) {
                spawn_stat_pills(counter, font, item);
            } else {
                counter.spawn(Node {
                    flex_grow: 1.0,
                    min_height: px(24),
                    ..default()
                });
            }
            spawn_quantity(counter, font, state, game, unit, buying);
        });
}

fn stat_values(stats: crate::field_menu_domain::DerivedStats) -> [(&'static str, i32); 4] {
    [
        ("STR", stats.strength),
        ("DEX", stats.dexterity),
        ("CON", stats.constitution),
        ("INT", stats.intelligence),
    ]
}

fn stat_color(delta: i32) -> Color {
    match delta.signum() {
        1 => Color::srgb_u8(98, 201, 122),
        -1 => Color::srgb_u8(237, 119, 112),
        _ => Color::srgb_u8(150, 153, 163),
    }
}

fn spawn_stat_pills(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    item: &ItemDefinition,
) {
    parent
        .spawn((
            Node {
                width: percent(100),
                height: px(94),
                flex_shrink: 0.0,
                padding: UiRect {
                    top: px(26),
                    bottom: px(14),
                    ..default()
                },
                column_gap: px(19),
                border: UiRect::bottom(px(1)),
                ..default()
            },
            BorderColor::all(shop_border()),
            Name::new("Shop item stats"),
        ))
        .with_children(|row| {
            for (label, value) in stat_values(item_stat_bonuses(item)) {
                let color = stat_color(value);
                row.spawn((
                    Node {
                        width: px(95),
                        height: px(52),
                        flex_shrink: 0.0,
                        padding: UiRect::horizontal(px(13)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceBetween,
                        border: UiRect::all(px(1.5)),
                        border_radius: BorderRadius::all(px(26)),
                        ..default()
                    },
                    BorderColor::all(color),
                    BackgroundColor(color.with_alpha(0.07)),
                    Name::new(format!("Shop stat: {label}")),
                ))
                .with_children(|pill| {
                    pill.spawn((
                        Text::new(label),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(15.0),
                            ..default()
                        },
                        TextColor(status_ink()),
                        TextLayout::new(Justify::Left, LineBreak::NoWrap),
                        Node {
                            flex_shrink: 0.0,
                            ..default()
                        },
                    ));
                    pill.spawn(Node {
                        width: px(32),
                        flex_shrink: 0.0,
                        justify_content: JustifyContent::End,
                        ..default()
                    })
                    .with_children(|value_slot| {
                        spawn_status_text(
                            value_slot,
                            if value > 0 {
                                format!("+{value}")
                            } else {
                                value.to_string()
                            },
                            font,
                            17.0,
                            color,
                        );
                    });
                });
            }
        });
}

fn spawn_quantity(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    game: &GameState,
    unit: u32,
    buying: bool,
) {
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
                width: percent(100),
                flex_grow: if is_item_shop(state) { 1.0 } else { 0.0 },
                min_height: px(210),
                max_height: px(212),
                flex_shrink: 0.0,
                margin: UiRect::top(px(24)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(Color::srgb_u8(27, 28, 34)),
            BorderColor::all(shop_border()),
            Name::new("Shop quantity panel"),
        ))
        .with_children(|panel| {
            panel
                .spawn(Node {
                    position_type: PositionType::Absolute,
                    left: px(41),
                    top: px(12),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(8),
                    ..default()
                })
                .with_children(|qty| {
                    spawn_status_text(qty, "QUANTITY", font, 12.0, status_muted());
                    qty.spawn((
                        Node {
                            width: px(78),
                            height: px(112),
                            flex_shrink: 0.0,
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::SpaceBetween,
                            border: UiRect::all(px(1)),
                            border_radius: BorderRadius::all(px(4)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb_u8(48, 39, 25)),
                        BorderColor::all(status_border_active()),
                        shop_focus_outline(state.shop_focus == ShopFocus::Detail),
                        Name::new("Shop quantity control"),
                    ))
                    .with_children(|control| {
                        for (label, step) in [("+", 1), ("", 0), ("-", -1)] {
                            if step == 0 {
                                spawn_status_text(
                                    control,
                                    format_shop_number(state.quantity),
                                    font,
                                    20.0,
                                    status_ink(),
                                );
                            } else {
                                control
                                    .spawn((
                                        Node {
                                            width: percent(100),
                                            height: px(34),
                                            align_items: AlignItems::Center,
                                            justify_content: JustifyContent::Center,
                                            ..default()
                                        },
                                        ShopControl::Quantity(step),
                                        Name::new(format!("Shop quantity: {label}")),
                                    ))
                                    .with_children(|step| {
                                        spawn_status_text(step, label, font, 23.0, status_gold())
                                    });
                            }
                        }
                    });
                });
            panel
                .spawn(Node {
                    position_type: PositionType::Absolute,
                    right: px(14),
                    bottom: px(22),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::End,
                    row_gap: px(3),
                    ..default()
                })
                .with_children(|total| {
                    spawn_status_text(total, "TOTAL", font, 19.0, status_muted());
                    spawn_money(total, font, amount, 36.0, status_ink());
                    spawn_status_text(
                        total,
                        format!("After transaction: {} GP", format_shop_number(balance)),
                        font,
                        14.0,
                        status_muted(),
                    );
                });
            panel
                .spawn(Node {
                    position_type: PositionType::Absolute,
                    left: px(13),
                    bottom: px(10),
                    ..default()
                })
                .with_children(|hint| {
                    spawn_status_text(
                        hint,
                        if buying {
                            "ENTER Review purchase"
                        } else {
                            "ENTER Review sale"
                        },
                        font,
                        14.0,
                        status_gold(),
                    );
                });
        });
}

pub(super) fn spawn_money(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    value: u32,
    size: f32,
    unit_color: Color,
) {
    parent
        .spawn(Node {
            align_items: AlignItems::Baseline,
            justify_content: JustifyContent::End,
            column_gap: px(6),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|money| {
            spawn_status_text(money, format_shop_number(value), font, size, status_gold());
            spawn_status_text(
                money,
                "GP",
                font,
                if size >= 30.0 { 14.0 } else { 12.0 },
                unit_color,
            );
        });
}

fn spawn_party(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    catalog: &FieldMenuCatalog,
    game: &GameState,
    portraits: &Portraits,
    item: &ItemDefinition,
) {
    parent
        .spawn((
            panel_node(PARTY_WIDTH, 10.0),
            panel_color(),
            BorderColor::all(shop_border()),
            Name::new("Shop party panel"),
        ))
        .with_children(|panel| {
            spawn_status_text(panel, "PARTY EQUIPMENT FIT", font, 13.0, status_gold());
            panel
                .spawn(Node {
                    width: percent(100),
                    flex_grow: 1.0,
                    min_height: px(0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4),
                    justify_content: JustifyContent::End,
                    margin: UiRect::bottom(px(12)),
                    ..default()
                })
                .with_children(|members| {
                    for member in game.party().members() {
                        members
                            .spawn((
                                Node {
                                    width: percent(100),
                                    height: px(92),
                                    flex_shrink: 0.0,
                                    padding: UiRect::all(px(4)),
                                    column_gap: px(8),
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(px(1)),
                                    border_radius: BorderRadius::all(px(4)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb_u8(28, 29, 34)),
                                BorderColor::all(shop_border()),
                                Name::new(format!("Shop member: {}", member.id())),
                            ))
                            .with_children(|card| {
                                let mut portrait = card.spawn((
                                    Node {
                                        width: px(82),
                                        height: px(82),
                                        flex_shrink: 0.0,
                                        ..default()
                                    },
                                    Name::new(format!("Shop portrait: {}", member.id())),
                                ));
                                if let Some(image) = portraits.0.get(member.id()) {
                                    portrait.insert(
                                        ImageNode::new(image.clone())
                                            .with_mode(NodeImageMode::Stretch),
                                    );
                                }
                                if let Ok(slot) = can_equip(member, item, catalog) {
                                    let before = stat_values(derived_stats(member, catalog));
                                    let after = stat_values(preview_stats(
                                        member,
                                        catalog,
                                        slot,
                                        Some(item.id()),
                                    ));
                                    card.spawn(Node {
                                        flex_grow: 1.0,
                                        min_width: px(0),
                                        flex_basis: px(0),
                                        display: Display::Grid,
                                        grid_template_columns: RepeatedGridTrack::flex(2, 1.0),
                                        row_gap: px(16),
                                        column_gap: px(8),
                                        ..default()
                                    })
                                    .with_children(|stats| {
                                        for ((label, old), (_, new)) in
                                            before.into_iter().zip(after)
                                        {
                                            spawn_status_text(
                                                stats,
                                                format!("{label} {old} -> {new}"),
                                                font,
                                                13.0,
                                                if new == old {
                                                    status_ink()
                                                } else {
                                                    stat_color(new - old)
                                                },
                                            );
                                        }
                                    });
                                } else {
                                    spawn_status_text(
                                        card,
                                        "Cannot equip",
                                        font,
                                        15.0,
                                        status_muted(),
                                    );
                                }
                            });
                    }
                });
        });
}

pub(super) fn spawn_footer(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
) {
    parent
        .spawn((
            Node {
                width: percent(100),
                height: px(30),
                flex_shrink: 0.0,
                margin: UiRect::top(px(8)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: px(16),
                border: UiRect::top(px(1)),
                ..default()
            },
            BorderColor::all(shop_border()),
            Name::new("Shop footer"),
        ))
        .with_children(|footer| {
            spawn_status_text(footer, service_hint(state), font, 12.0, status_muted());
            if let Some(toast) = &state.toast {
                spawn_status_text(
                    footer,
                    &toast.text,
                    font,
                    12.0,
                    match toast.tone {
                        ToastTone::Good => status_ready(),
                        ToastTone::Warn => status_ember(),
                    },
                );
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        field_menu_domain::tests::{catalog, game},
        scenario_balance::BalanceData,
        scenario_party::PartyCatalog,
        scenario_yaml,
    };

    #[test]
    fn consumables_hide_party_fit_and_keep_the_same_counter_width_as_equipment() {
        for (id, equipment) in [("potion", false), ("dragonscale_shield", true)] {
            let catalog = catalog();
            let mut game = game([]);
            let _ = game.repository_mut().add_item(id, 1).unwrap();
            let mut state = ServiceUiState::default();
            state.open(ServiceRequest::Shop(DialogueShopKind::Item), None);
            state.page = Some(ServicePage::Sell);
            state.shop_focus = ShopFocus::Items;
            let mut app = App::new();
            app.add_plugins(MinimalPlugins);
            app.world_mut()
                .commands()
                .spawn(Node::default())
                .with_children(|root| {
                    spawn_page(
                        root,
                        &Handle::default(),
                        &state,
                        &catalog,
                        &game,
                        &Portraits::default(),
                    );
                });
            app.update();
            let world = app.world_mut();
            let mut nodes = world.query::<(&Name, &Node)>();
            let counter = nodes
                .iter(world)
                .find(|(name, _)| name.as_str() == "Shop counter panel")
                .unwrap()
                .1;
            assert_eq!(counter.width, px(474));
            assert_eq!(counter.flex_shrink, 0.0);
            assert_eq!(
                nodes
                    .iter(world)
                    .any(|(name, _)| name.as_str() == "Shop party panel"),
                equipment
            );
        }
    }

    #[test]
    fn equipment_bonuses_and_each_members_replacement_preview_are_distinct() {
        let catalog = catalog();
        let mut game = game([]);
        let party: PartyCatalog = scenario_yaml::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/scenarios/rusted_kingdoms/data/party.yaml"
        )))
        .unwrap();
        let balance: BalanceData = scenario_yaml::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/scenarios/rusted_kingdoms/data/balance.yaml"
        )))
        .unwrap();
        for source in &party.party {
            if !game.party().contains(&source.data().id) {
                let member = crate::runtime_member::RuntimeMember::try_from_catalog(
                    source,
                    catalog.class(&source.data().class_id).unwrap(),
                    &balance.progression,
                )
                .unwrap();
                game.party_mut().try_add(member).unwrap();
            }
        }
        let item = catalog.item("dragonscale_shield").unwrap();
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.world_mut()
            .commands()
            .spawn(Node::default())
            .with_children(|root| {
                spawn_stat_pills(root, &Handle::default(), item);
                spawn_party(
                    root,
                    &Handle::default(),
                    &catalog,
                    &game,
                    &Portraits::default(),
                    item,
                );
            });
        app.update();
        let world = app.world_mut();
        let members = world
            .query::<(&Name, &BackgroundColor, Option<&Outline>)>()
            .iter(world)
            .filter(|(name, _, _)| name.as_str().starts_with("Shop member:"))
            .collect::<Vec<_>>();
        assert_eq!(members.len(), 5);
        assert!(members.iter().all(|(_, background, outline)| {
            background.0 == Color::srgb_u8(28, 29, 34) && outline.is_none()
        }));
        let text = world
            .query::<&Text>()
            .iter(world)
            .map(|t| t.0.as_str())
            .collect::<Vec<_>>();
        assert!(text.contains(&"+11"));
        assert!(text.contains(&"-2"));
        assert_eq!(text.iter().filter(|&&value| value == "0").count(), 2);
        assert_eq!(
            text.iter()
                .filter(|&&value| value == "Cannot equip")
                .count(),
            3
        );
        for member in game.party().members() {
            if let Ok(slot) = can_equip(member, item, &catalog) {
                let before = derived_stats(member, &catalog);
                let after = preview_stats(member, &catalog, slot, Some(item.id()));
                assert!(text.contains(
                    &format!("CON {} -> {}", before.constitution, after.constitution).as_str()
                ));
            }
        }
    }
}
