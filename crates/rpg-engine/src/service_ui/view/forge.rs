//! The forge workspace: equipment types, the pieces of that type, and the selected quote.
//!
//! Three columns in the field menu's item-page arrangement. The right column is the piece at
//! full icon size over a conversion: what goes into the strike, an arrow, and what comes out.
//! It stays empty until the piece list holds focus.

use super::*;
use crate::{
    forge::{
        ForgeKind, ForgeQuote, ForgeType, forgeable_of, parse_id, primary_stat, quote, tempering,
    },
    item_images::{ITEM_IMAGE_SIZE, ItemIcon},
    scenario_path::ScenarioRelativePath,
    service_ui::forge::rows,
};

const TYPE_WIDTH: f32 = 180.0;
const DETAIL_WIDTH: f32 = 600.0;
const VISIBLE_ROWS: usize = 5;
const CARD_HEIGHT: f32 = 80.0;
const SLOT_ICON_SIZE: f32 = 44.0;

/// The equipment-slot artwork drawn on each type card, in [`ForgeType::ALL`] order.
#[derive(Default)]
pub(super) struct SlotIcons(Vec<Option<Handle<Image>>>);

impl SlotIcons {
    pub(super) fn load(server: &AssetServer, root: &ScenarioRoot) -> Self {
        Self(
            ForgeType::ALL
                .into_iter()
                .map(|kind| {
                    let path = ScenarioRelativePath::try_from(format!(
                        "media/images/icons/slots/{}.png",
                        kind.label().to_lowercase()
                    ))
                    .ok()?;
                    Some(server.load(root.resolve(&path)))
                })
                .collect(),
        )
    }

    fn get(&self, index: usize) -> Option<&Handle<Image>> {
        self.0.get(index).and_then(Option::as_ref)
    }
}

/// The price of a strike is a loss, so it never wears the gold of a reward.
fn cost_color() -> Color {
    Color::srgb_u8(214, 126, 98)
}

/// A shortfall, kept redder than the ordinary cost so the two never read alike.
fn shortfall_color() -> Color {
    Color::srgb_u8(236, 84, 70)
}

pub(super) fn spawn_header(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
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
            Name::new("Forge header"),
        ))
        .with_children(|header| {
            spawn_keeper_frame(header, font, 60.0, face);
            header
                .spawn(Node {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    row_gap: px(3),
                    ..default()
                })
                .with_children(|title| {
                    spawn_status_text(title, "FORGE", font, 30.0, status_ink());
                    spawn_status_text(
                        title,
                        "TEMPER GEAR FOR GP AND MATERIAL",
                        font,
                        11.0,
                        status_muted(),
                    );
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
                        format!("{} GP", format_shop_number(game.repository().gp())),
                        font,
                        26.0,
                        status_ink(),
                    );
                });
        });
}

pub(super) fn spawn_page(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
    icons: &SlotIcons,
) {
    let listed = rows(state, catalog, game);
    let selected = state.selected.min(listed.len().saturating_sub(1));
    let in_list = state.forge_in_list && !listed.is_empty();
    parent
        .spawn((
            Node {
                width: percent(100),
                flex_grow: 1.0,
                min_height: px(0),
                column_gap: px(16),
                ..default()
            },
            Name::new("Forge workspace"),
        ))
        .with_children(|columns| {
            spawn_types(columns, font, state, catalog, game, icons, !in_list);
            spawn_pieces(
                columns, font, state, catalog, game, &listed, selected, in_list,
            );
            spawn_detail(
                columns,
                font,
                state,
                catalog,
                game,
                listed.get(selected).filter(|_| in_list),
            );
        });
}

fn spawn_panel(
    parent: &mut ChildSpawnerCommands<'_>,
    node: Node,
    title: &str,
    caption: &str,
    font: &Handle<Font>,
    focused: bool,
    content: impl FnOnce(&mut ChildSpawnerCommands<'_>),
) {
    parent
        .spawn((
            Node {
                height: percent(100),
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(15)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(5)),
                overflow: Overflow::clip(),
                ..node
            },
            BackgroundColor(Color::srgba_u8(23, 24, 30, 240)),
            BorderColor::all(if focused {
                status_border_active()
            } else {
                status_border()
            }),
            Outline {
                width: px(2),
                offset: px(0),
                color: if focused {
                    status_border_active()
                } else {
                    Color::NONE
                },
            },
            Name::new(format!("Forge {title}")),
        ))
        .with_children(|panel| {
            panel
                .spawn((
                    Node {
                        width: percent(100),
                        height: px(27),
                        flex_shrink: 0.0,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Start,
                        border: UiRect::bottom(px(1)),
                        padding: UiRect::bottom(px(10)),
                        margin: UiRect::bottom(px(12)),
                        ..default()
                    },
                    BorderColor::all(status_border()),
                ))
                .with_children(|heading| {
                    spawn_status_text(heading, title, font, 12.0, status_gold());
                    spawn_status_text(heading, caption, font, 11.0, status_muted());
                });
            content(panel);
        });
}

fn spawn_types(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
    icons: &SlotIcons,
    focused: bool,
) {
    spawn_panel(
        parent,
        Node {
            width: px(TYPE_WIDTH),
            flex_shrink: 0.0,
            ..default()
        },
        "EQUIPMENT",
        "",
        font,
        focused,
        |panel| {
            // The five types always fit, so the cards share the column's full height.
            panel
                .spawn(Node {
                    width: percent(100),
                    flex_grow: 1.0,
                    min_height: px(0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(8),
                    ..default()
                })
                .with_children(|list| {
                    for (index, kind) in ForgeType::ALL.into_iter().enumerate() {
                        let current = index == state.forge_type;
                        let count = forgeable_of(catalog, game.repository(), kind).len();
                        list.spawn((
                            Node {
                                width: percent(100),
                                flex_grow: 1.0,
                                flex_basis: px(0),
                                min_height: px(0),
                                align_items: AlignItems::Center,
                                column_gap: px(8),
                                padding: UiRect::horizontal(px(12)),
                                border: UiRect::all(px(if current { 2 } else { 1 })),
                                border_radius: BorderRadius::all(px(4)),
                                ..default()
                            },
                            selectable_card_colors(current),
                            Name::new(format!("Forge type: {}", kind.label())),
                        ))
                        .with_children(|row| {
                            row.spawn(Node {
                                flex_grow: 1.0,
                                min_width: px(0),
                                flex_direction: FlexDirection::Column,
                                row_gap: px(5),
                                ..default()
                            })
                            .with_children(|label| {
                                spawn_status_text(
                                    label,
                                    kind.label(),
                                    font,
                                    18.0,
                                    if count == 0 {
                                        status_faint()
                                    } else {
                                        status_ink()
                                    },
                                );
                                spawn_status_text(
                                    label,
                                    format!("x{count}"),
                                    font,
                                    17.0,
                                    if count == 0 {
                                        status_faint()
                                    } else {
                                        status_muted()
                                    },
                                );
                            });
                            let mut icon = row.spawn(Node {
                                width: px(SLOT_ICON_SIZE),
                                height: px(SLOT_ICON_SIZE),
                                flex_shrink: 0.0,
                                ..default()
                            });
                            if let Some(image) = icons.get(index) {
                                icon.insert(ImageNode {
                                    color: Color::WHITE.with_alpha(if count == 0 {
                                        0.35
                                    } else {
                                        0.9
                                    }),
                                    ..ImageNode::new(image.clone())
                                });
                            }
                        });
                    }
                });
        },
    );
}

#[expect(
    clippy::too_many_arguments,
    reason = "the piece list reads the scroll memory alongside the filtered inventory"
)]
fn spawn_pieces(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
    listed: &[String],
    selected: usize,
    focused: bool,
) {
    let kind = ForgeType::ALL[state.forge_type.min(ForgeType::ALL.len() - 1)];
    spawn_panel(
        parent,
        Node {
            flex_grow: 1.0,
            flex_basis: px(0),
            ..default()
        },
        "PIECES",
        &format!("{} you carry", kind.label()),
        font,
        focused,
        |panel| {
            if listed.is_empty() {
                spawn_status_text(
                    panel,
                    format!("No {} to temper.", kind.label().to_lowercase()),
                    font,
                    16.0,
                    status_muted(),
                );
                spawn_status_text(
                    panel,
                    "Unequip a piece to bring it here.",
                    font,
                    13.0,
                    status_faint(),
                );
                return;
            }
            let window = state
                .scroll
                .window("forge", selected, listed.len(), VISIBLE_ROWS);
            spawn_scroll_list(
                panel,
                window,
                Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    min_height: px(0),
                    row_gap: px(8),
                    ..default()
                },
                |cards| {
                    for index in window.indices() {
                        let id = &listed[index];
                        let Some(item) = catalog.item(id) else {
                            continue;
                        };
                        let current = focused && index == selected;
                        cards
                            .spawn((
                                Node {
                                    width: percent(100),
                                    height: px(CARD_HEIGHT),
                                    flex_shrink: 0.0,
                                    align_items: AlignItems::Center,
                                    column_gap: px(12),
                                    padding: UiRect::horizontal(px(10)),
                                    border: UiRect::all(px(if current { 2 } else { 1 })),
                                    border_radius: BorderRadius::all(px(4)),
                                    ..default()
                                },
                                selectable_card_colors(current),
                            ))
                            .with_children(|row| {
                                spawn_gear_icon(row, font, id, 56.0, 13.0);
                                row.spawn(Node {
                                    flex_grow: 1.0,
                                    min_width: px(0),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: px(6),
                                    ..default()
                                })
                                .with_children(|text| {
                                    spawn_status_text(
                                        text,
                                        item_name(item),
                                        font,
                                        17.0,
                                        status_ink(),
                                    );
                                    text.spawn(Node {
                                        align_items: AlignItems::Baseline,
                                        column_gap: px(14),
                                        ..default()
                                    })
                                    .with_children(
                                        |numbers| {
                                            spawn_status_text(
                                                numbers,
                                                format!("x{}", game.repository().item_count(id)),
                                                font,
                                                17.0,
                                                status_muted(),
                                            );
                                            if let Some((level, top)) = tempering(catalog, id) {
                                                let (label, color) = if level >= top {
                                                    ("MAX".to_owned(), status_ready())
                                                } else {
                                                    (format!("+{level} / +{top}"), status_faint())
                                                };
                                                spawn_status_text(
                                                    numbers, label, font, 16.0, color,
                                                );
                                            }
                                        },
                                    );
                                });
                            });
                    }
                },
            );
            spawn_status_text(
                panel,
                format!("PIECE {:02} / {:02}", selected + 1, listed.len()),
                font,
                12.0,
                status_faint(),
            );
        },
    );
}

/// An item icon with its tempering level stamped on the corner: `+2`, or `+2!` once cracked.
fn spawn_gear_icon(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    id: &str,
    size: f32,
    badge_size: f32,
) {
    parent
        .spawn(Node {
            width: px(size),
            height: px(size),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|slot| {
            slot.spawn((
                ItemIcon::new(id),
                Node {
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                Name::new(format!("Item image: {id}")),
            ));
            let Some(parsed) = parse_id(id) else { return };
            let (label, color) = if parsed.cracked {
                (format!("+{}!", parsed.level), shortfall_color())
            } else {
                (format!("+{}", parsed.level), status_ready())
            };
            slot.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: px(-4),
                    bottom: px(-4),
                    padding: UiRect::axes(px(4), px(1)),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(3)),
                    ..default()
                },
                BackgroundColor(Color::srgba_u8(10, 10, 14, 235)),
                BorderColor::all(color),
            ))
            .with_children(|badge| {
                spawn_status_text(badge, label, font, badge_size, color);
            });
        });
}

fn spawn_detail(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
    id: Option<&String>,
) {
    spawn_panel(
        parent,
        Node {
            width: px(DETAIL_WIDTH),
            flex_shrink: 0.0,
            row_gap: px(12),
            ..default()
        },
        "DETAIL",
        if state.page == Some(ServicePage::ForgeConfirm) {
            "READY TO STRIKE"
        } else {
            ""
        },
        font,
        state.page == Some(ServicePage::ForgeConfirm),
        |panel| {
            let Some((id, item)) = id.and_then(|id| Some((id, catalog.item(id)?))) else {
                spawn_status_text(
                    panel,
                    "Choose a piece to see what the smith asks.",
                    font,
                    15.0,
                    status_muted(),
                );
                return;
            };
            panel
                .spawn(Node {
                    width: percent(100),
                    flex_shrink: 0.0,
                    align_items: AlignItems::Center,
                    column_gap: px(14),
                    ..default()
                })
                .with_children(|heading| {
                    spawn_gear_icon(heading, font, id, ITEM_IMAGE_SIZE, 17.0);
                    heading
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            flex_basis: px(0),
                            flex_grow: 1.0,
                            min_width: px(0),
                            row_gap: px(5),
                            ..default()
                        })
                        .with_children(|title| {
                            spawn_status_text(title, item_name(item), font, 22.0, status_ink());
                            spawn_status_text(
                                title,
                                item_description(item),
                                font,
                                13.0,
                                status_muted(),
                            );
                        });
                });
            let cap = tempering(catalog, id);
            if let Some((level, top)) = cap {
                spawn_status_text(
                    panel,
                    format!("Tempered +{level} of +{top}"),
                    font,
                    13.0,
                    status_muted(),
                );
            }
            let Some(quote) = quote(catalog, id) else {
                if let Some((_, top)) = cap {
                    spawn_status_text(
                        panel,
                        format!("Fully tempered. +{top} is the most this piece can take."),
                        font,
                        15.0,
                        status_ready(),
                    );
                }
                return;
            };
            panel
                .spawn((
                    Node {
                        width: percent(100),
                        flex_grow: 1.0,
                        min_height: px(0),
                        column_gap: px(10),
                        ..default()
                    },
                    Name::new("Forge conversion"),
                ))
                .with_children(|row| {
                    spawn_offering(row, font, catalog, game, &quote);
                    spawn_conversion_arrow(row);
                    spawn_result(row, font, catalog, item, &quote);
                });
            let note = match (quote.kind, quote.odds) {
                (ForgeKind::Enhance, Some(odds)) => format!(
                    "Great {}%    Tempered {}%    Fizzle {}%    Crack {}%",
                    odds.great, odds.success, odds.fizzle, odds.backfire
                ),
                _ => "A mend always holds.".to_owned(),
            };
            spawn_status_text(panel, note, font, 14.0, status_muted());
        },
    );
}

/// One side of the conversion: half the row, full height.
fn frame_node() -> Node {
    Node {
        flex_grow: 1.0,
        flex_basis: px(0),
        min_width: px(0),
        flex_direction: FlexDirection::Column,
        row_gap: px(10),
        padding: UiRect::all(px(14)),
        border: UiRect::all(px(1)),
        border_radius: BorderRadius::all(px(5)),
        ..default()
    }
}

/// What goes into the strike: the smith's fee and the material.
fn spawn_offering(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    catalog: &FieldMenuCatalog,
    game: &GameState,
    quote: &ForgeQuote,
) {
    let short = game.repository().gp() < quote.gp;
    spawn_status_panel(parent, frame_node(), "YOU GIVE", font, |panel| {
        spawn_status_text(
            panel,
            format!("{} GP", format_shop_number(quote.gp)),
            font,
            26.0,
            if short {
                shortfall_color()
            } else {
                cost_color()
            },
        );
        if short {
            spawn_status_text(panel, "NOT ENOUGH GP", font, 12.0, shortfall_color());
        }
        match quote.material {
            Some((material, count)) => {
                let have = game.repository().item_count(material);
                let name = catalog.item(material).map_or(material, item_name);
                panel
                    .spawn(Node {
                        align_items: AlignItems::Center,
                        column_gap: px(10),
                        ..default()
                    })
                    .with_children(|row| {
                        spawn_gear_icon(row, font, material, 44.0, 11.0);
                        row.spawn(Node {
                            min_width: px(0),
                            flex_direction: FlexDirection::Column,
                            row_gap: px(4),
                            ..default()
                        })
                        .with_children(|text| {
                            spawn_status_text(
                                text,
                                format!("{name}  x{count}"),
                                font,
                                17.0,
                                status_ink(),
                            );
                            spawn_status_text(
                                text,
                                format!("have {have}"),
                                font,
                                14.0,
                                if have >= count {
                                    status_muted()
                                } else {
                                    shortfall_color()
                                },
                            );
                        });
                    });
            }
            None => spawn_status_text(panel, "No material needed.", font, 15.0, status_muted()),
        }
    });
}

/// A right-pointing arrow between the two frames: what goes in becomes what comes out.
fn spawn_conversion_arrow(parent: &mut ChildSpawnerCommands<'_>) {
    let color = status_gold();
    parent
        .spawn((
            Node {
                width: px(40),
                flex_shrink: 0.0,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            Name::new("Forge conversion arrow"),
        ))
        .with_children(|slot| {
            slot.spawn((
                Node {
                    width: px(32),
                    height: px(4),
                    ..default()
                },
                BackgroundColor(color),
            ))
            .with_children(|shaft| {
                for (offset, rotation) in [(-5.0, 45.0), (5.0, -45.0)] {
                    shaft.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            right: px(-2),
                            top: px(offset),
                            width: px(16),
                            height: px(4),
                            ..default()
                        },
                        BackgroundColor(color),
                        UiTransform::from_rotation(Rot2::degrees(rotation)),
                    ));
                }
            });
        });
}

/// `from  >  to`, the old value small and dim and the new one large and bright.
fn spawn_diff(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    from: &str,
    to: &str,
    to_size: f32,
) {
    parent
        .spawn(Node {
            align_items: AlignItems::Baseline,
            column_gap: px(10),
            flex_wrap: FlexWrap::Wrap,
            ..default()
        })
        .with_children(|row| {
            spawn_status_text(row, from, font, 15.0, status_muted());
            spawn_status_text(row, ">", font, 15.0, status_faint());
            spawn_status_text(row, to, font, to_size, status_ready());
        });
}

/// What comes out: the tempered (or mended) piece and the stat it raises.
fn spawn_result(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    catalog: &FieldMenuCatalog,
    from: &ItemDefinition,
    quote: &ForgeQuote,
) {
    let to = catalog.item(&quote.target_id);
    spawn_status_panel(parent, frame_node(), "YOU GET", font, |panel| {
        let Some(to) = to else { return };
        panel
            .spawn(Node {
                align_items: AlignItems::Center,
                column_gap: px(10),
                ..default()
            })
            .with_children(|row| {
                spawn_gear_icon(row, font, &quote.target_id, 44.0, 11.0);
                row.spawn(Node {
                    min_width: px(0),
                    flex_grow: 1.0,
                    ..default()
                })
                .with_children(|name| {
                    spawn_status_text(name, item_name(to), font, 18.0, status_ready());
                });
            });
        if let (Some((label, now)), Some((_, next))) = (primary_stat(from), primary_stat(to)) {
            spawn_diff(
                panel,
                font,
                &format!("{label} {now}"),
                &format!("{label} {next}"),
                28.0,
            );
        }
        if quote.kind == ForgeKind::Mend {
            spawn_status_text(
                panel,
                "Seals the crack and restores the stat.",
                font,
                14.0,
                status_muted(),
            );
        }
    });
}

pub(super) fn spawn_footer(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
) {
    let hint = if state.page == Some(ServicePage::ForgeConfirm) {
        "ENTER  strike     ESC  back     UP/DOWN  choose another"
    } else if state.forge_in_list {
        "UP/DOWN  browse     ENTER  review quote     LEFT / ESC  equipment types"
    } else {
        "UP/DOWN  equipment type     RIGHT / ENTER  pieces     ESC  leave forge"
    };
    parent
        .spawn(Node {
            width: percent(100),
            margin: UiRect::top(px(14)),
            height: px(20),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|footer| spawn_status_text(footer, hint, font, 12.0, status_muted()));
    spawn_toast(parent, font, state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_menu_domain::tests::{catalog, game};

    fn texts(in_list: bool) -> Vec<String> {
        let catalog = catalog();
        let mut game = game([]);
        let repo = game.repository_mut();
        let _ = repo.add_gp(2_000).unwrap();
        let _ = repo.add_item("iron_sword", 1).unwrap();
        let _ = repo.add_item("iron_plate", 1).unwrap();
        let mut state = ServiceUiState::default();
        state.open(ServiceRequest::Forge, None);
        state.forge_in_list = in_list;
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
                    &SlotIcons::default(),
                );
            });
        app.update();
        let world = app.world_mut();
        world
            .query::<&Text>()
            .iter(world)
            .map(|text| text.0.clone())
            .collect()
    }

    #[test]
    fn the_detail_waits_for_the_piece_list_then_shows_give_and_get() {
        let browsing = texts(false);
        assert!(browsing.iter().any(|t| t == "Weapon"));
        assert!(!browsing.iter().any(|t| t == "YOU GIVE" || t == "YOU GET"));

        let picking = texts(true);
        for label in ["YOU GIVE", "YOU GET", "87 GP", "Iron Sword +1", "+0 / +5"] {
            assert!(picking.iter().any(|t| t == label), "{label}: {picking:?}");
        }
        assert!(!picking.iter().any(|t| t.starts_with("you hold")));
    }
}
