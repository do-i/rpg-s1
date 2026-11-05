use super::*;
use crate::{field_menu_domain::item_stat_bonuses, item_images::ItemIcon};

const PORTRAIT_WIDTH: f32 = 418.0;
const PORTRAIT_HEIGHT: f32 = 570.0;
const EQUIPMENT_WIDTH: f32 = 320.0;
const EQUIPMENT_ICON: f32 = 56.0;

pub(super) fn spawn_status_dossier(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
    art: &StatusPageArt<'_>,
) {
    let Some(member) = member_at(game, state.member_index) else {
        return;
    };
    parent
        .spawn((
            Node {
                flex_grow: 1.0,
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(12),
                padding: UiRect::all(px(16)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(6)),
                ..default()
            },
            BackgroundColor(Color::srgb_u8(22, 22, 28)),
            BorderColor::all(status_border()),
            Name::new("Status combined frame"),
        ))
        .with_children(|panel| {
            panel
                .spawn(Node {
                    height: px(32),
                    flex_shrink: 0.0,
                    align_items: AlignItems::Center,
                    column_gap: px(16),
                    ..default()
                })
                .with_children(|identity| {
                    spawn_status_text(identity, member.name(), font, 30.0, status_ink());
                    spawn_status_text(
                        identity,
                        class_name(member.class_id(), catalog).to_uppercase(),
                        font,
                        13.0,
                        status_gold(),
                    );
                    identity.spawn(Node {
                        flex_grow: 1.0,
                        ..default()
                    });
                    spawn_status_text(
                        identity,
                        format!("LV {:02}", member.level()),
                        font,
                        17.0,
                        status_ink(),
                    );
                });
            spawn_panel_rule(panel);
            panel
                .spawn(Node {
                    flex_grow: 1.0,
                    min_height: px(0),
                    column_gap: px(24),
                    ..default()
                })
                .with_children(|columns| {
                    columns
                        .spawn(Node {
                            flex_grow: 1.0,
                            flex_basis: px(0),
                            min_width: px(0),
                            flex_direction: FlexDirection::Column,
                            row_gap: px(12),
                            ..default()
                        })
                        .with_children(|vitals| {
                            vitals
                                .spawn(Node {
                                    justify_content: JustifyContent::SpaceBetween,
                                    ..default()
                                })
                                .with_children(|heading| {
                                    spawn_status_text(
                                        heading,
                                        "VITALS & ATTRIBUTES",
                                        font,
                                        13.0,
                                        status_gold(),
                                    );
                                    let statuses = member
                                        .status_effects()
                                        .map(|effect| format!("{effect:?}").to_uppercase())
                                        .collect::<Vec<_>>();
                                    spawn_status_text(
                                        heading,
                                        if statuses.is_empty() {
                                            "CLEAR".to_owned()
                                        } else {
                                            statuses.join(", ")
                                        },
                                        font,
                                        12.0,
                                        if statuses.is_empty() {
                                            status_ready()
                                        } else {
                                            status_ember()
                                        },
                                    );
                                });
                            for (label, value, maximum, color) in [
                                (
                                    "HP",
                                    member.health(),
                                    member.max_health(),
                                    if member.health().saturating_mul(4) < member.max_health() {
                                        status_ember()
                                    } else {
                                        status_ready()
                                    },
                                ),
                                ("MP", member.mana(), member.max_mana(), status_teal()),
                                (
                                    "EXP",
                                    member.experience(),
                                    member.experience_next(),
                                    status_violet(),
                                ),
                            ] {
                                spawn_status_meter(vitals, font, label, value, maximum, color);
                            }
                            spawn_status_attributes(vitals, font, member, catalog);
                            vitals.spawn(Node {
                                flex_grow: 1.0,
                                min_height: px(0),
                                ..default()
                            });
                            spawn_status_position(vitals, font, state, member);
                            spawn_status_sprites(vitals, font, state, member, game, art);
                        });
                    spawn_status_equipment(columns, font, member, catalog);
                });
        });
}

fn spawn_status_meter(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    label: &str,
    value: u32,
    maximum: u32,
    color: Color,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(4),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|meter| {
            meter
                .spawn(Node {
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                })
                .with_children(|line| {
                    spawn_status_text(line, label, font, 14.0, status_muted());
                    line.spawn((
                        Text::new(if label == "EXP" && maximum == 0 {
                            "MAX LEVEL".to_owned()
                        } else {
                            format!("{value} / {maximum}")
                        }),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(16.0),
                            ..default()
                        },
                        TextColor(status_ink()),
                        TextLayout::justify(Justify::Right),
                        Name::new(format!("Status {label} value")),
                    ));
                });
            meter
                .spawn((
                    Node {
                        height: px(5),
                        width: percent(100),
                        border_radius: BorderRadius::all(px(2)),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(Color::srgb_u8(52, 52, 61)),
                ))
                .with_children(|track| {
                    let fraction = if maximum == 0 {
                        1.0
                    } else {
                        (value as f32 / maximum as f32).clamp(0.0, 1.0)
                    };
                    track.spawn((
                        Node {
                            width: percent(fraction * 100.0),
                            height: percent(100),
                            ..default()
                        },
                        BackgroundColor(color),
                    ));
                });
        });
}

fn spawn_status_attributes(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    member: &crate::runtime_member::RuntimeMember,
    catalog: &FieldMenuCatalog,
) {
    let stats = derived_stats(member, catalog);
    parent
        .spawn((
            Node {
                height: px(55),
                flex_shrink: 0.0,
                column_gap: px(8),
                ..default()
            },
            Name::new("Status attribute row"),
        ))
        .with_children(|row| {
            for (label, value) in [
                ("STR", stats.strength),
                ("DEX", stats.dexterity),
                ("CON", stats.constitution),
                ("INT", stats.intelligence),
            ] {
                row.spawn((
                    Node {
                        flex_grow: 1.0,
                        flex_basis: px(0),
                        min_width: px(0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        row_gap: px(3),
                        border: UiRect::all(px(1)),
                        border_radius: BorderRadius::all(px(4)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb_u8(32, 32, 39)),
                    BorderColor::all(Color::srgb_u8(57, 52, 58)),
                    Name::new(format!("Status attribute {label}")),
                ))
                .with_children(|card| {
                    spawn_status_text(card, label, font, 13.0, status_muted());
                    spawn_status_text(card, format!("{value:02}"), font, 23.0, status_ink());
                });
            }
        });
}

fn spawn_status_position(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    member: &crate::runtime_member::RuntimeMember,
) {
    let focused = state.status_page == StatusPage::Position;
    parent
        .spawn((
            Node {
                height: px(48),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                column_gap: px(8),
                padding: UiRect::all(px(7)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(Color::srgb_u8(36, 35, 40)),
            BorderColor::all(if focused {
                status_border_active()
            } else {
                status_border()
            }),
            Name::new("Status position"),
        ))
        .with_children(|position| {
            spawn_status_text(position, "POSITION", font, 13.0, status_muted());
            position.spawn(Node {
                flex_grow: 1.0,
                ..default()
            });
            for (index, row) in STATUS_ROWS.into_iter().enumerate() {
                let selected = if focused {
                    state.selected == index
                } else {
                    member.row() == row
                };
                position
                    .spawn((
                        choice_node(94.0, 32.0),
                        BackgroundColor(choice_background(selected)),
                        BorderColor::all(if selected {
                            status_border_active()
                        } else {
                            status_border()
                        }),
                        Name::new(format!("Status row {}", row_label(row))),
                    ))
                    .with_children(|button| {
                        spawn_status_text(
                            button,
                            row_label(row),
                            font,
                            16.0,
                            if selected {
                                status_gold()
                            } else {
                                status_muted()
                            },
                        );
                    });
            }
        });
}

fn choice_node(width: f32, height: f32) -> Node {
    Node {
        width: px(width),
        height: px(height),
        flex_shrink: 0.0,
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        border: UiRect::all(px(1)),
        border_radius: BorderRadius::all(px(4)),
        ..default()
    }
}

fn choice_background(selected: bool) -> Color {
    if selected {
        Color::srgb_u8(58, 46, 37)
    } else {
        Color::srgb_u8(32, 32, 39)
    }
}

fn spawn_status_sprites(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    member: &crate::runtime_member::RuntimeMember,
    game: &GameState,
    art: &StatusPageArt<'_>,
) {
    let focused = state.status_page == StatusPage::Sprite;
    let variants = art
        .inventory
        .filter(|_| member.is_protagonist())
        .map(|inventory| &inventory.protagonist_sprite_variants);
    let count = variants.map_or(1, |variants| variants.len().max(1));
    let current = if focused {
        state.sprite_choice
    } else {
        art.inventory
            .map_or(0, |inventory| inventory.sprite_variant_index(game))
    };
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|section| {
            section
                .spawn(Node {
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                })
                .with_children(|heading| {
                    spawn_status_text(heading, "FIELD SPRITE", font, 13.0, status_gold());
                    spawn_status_text(
                        heading,
                        "USE ON MAP >",
                        font,
                        12.0,
                        if focused {
                            status_gold()
                        } else {
                            status_muted()
                        },
                    );
                });
            section
                .spawn(Node {
                    height: px(100),
                    column_gap: px(10),
                    ..default()
                })
                .with_children(|cards| {
                    for index in 0..count {
                        let variant = variants.and_then(|variants| variants.get(index));
                        let selected = variant.is_none() || index == current;
                        let frame = if variant.is_some() {
                            art.thumbnails.variant_frame(index)
                        } else {
                            art.thumbnails.frame(member.id())
                        };
                        cards
                            .spawn((
                                Node {
                                    flex_basis: px(0),
                                    flex_grow: 1.0,
                                    min_width: px(0),
                                    ..choice_node(0.0, 100.0)
                                },
                                BackgroundColor(choice_background(selected)),
                                BorderColor::all(if selected {
                                    status_border_active()
                                } else {
                                    status_border()
                                }),
                                Name::new(format!("Status sprite choice {index}")),
                            ))
                            .with_children(|card| {
                                let node = Node {
                                    width: px(80),
                                    height: px(80),
                                    flex_shrink: 0.0,
                                    ..default()
                                };
                                if let Some(frame) = frame {
                                    card.spawn((frame, node));
                                } else {
                                    card.spawn(node);
                                }
                                spawn_status_text(
                                    card,
                                    variant.map_or("Default", |variant| variant.name.as_str()),
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
                });
        });
}

fn spawn_status_equipment(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    member: &crate::runtime_member::RuntimeMember,
    catalog: &FieldMenuCatalog,
) {
    parent
        .spawn(Node {
            width: px(EQUIPMENT_WIDTH),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(12),
            ..default()
        })
        .with_children(|equipment| {
            spawn_status_text(equipment, "EQUIPMENT", font, 13.0, status_gold());
            equipment
                .spawn(Node {
                    flex_grow: 1.0,
                    min_height: px(0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(8),
                    ..default()
                })
                .with_children(|cards| {
                    for slot in EquipmentSlot::ALL {
                        let id = member.equipment().get(slot);
                        let item = id.and_then(|id| catalog.item(id));
                        cards
                            .spawn((
                                Node {
                                    flex_grow: 1.0,
                                    flex_basis: px(0),
                                    min_height: px(70),
                                    align_items: AlignItems::Center,
                                    column_gap: px(8),
                                    padding: UiRect::axes(px(8), px(6)),
                                    border: UiRect::all(px(1)),
                                    border_radius: BorderRadius::all(px(4)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb_u8(32, 32, 39)),
                                BorderColor::all(Color::srgb_u8(55, 50, 58)),
                                Name::new(format!("Status equipment {}", slot.as_str())),
                            ))
                            .with_children(|card| {
                                let mut icon = card.spawn((
                                    Node {
                                        width: px(EQUIPMENT_ICON),
                                        height: px(EQUIPMENT_ICON),
                                        flex_shrink: 0.0,
                                        align_items: AlignItems::Center,
                                        justify_content: JustifyContent::Center,
                                        ..default()
                                    },
                                    Name::new("Status equipment icon"),
                                ));
                                if let Some(id) = id {
                                    icon.insert(ItemIcon::new(id));
                                }
                                icon.with_children(|fallback| {
                                    spawn_status_text(fallback, "-", font, 24.0, status_muted());
                                });
                                card.spawn(Node {
                                    flex_grow: 1.0,
                                    min_width: px(0),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: px(8),
                                    ..default()
                                })
                                .with_children(|name| {
                                    spawn_status_text(
                                        name,
                                        match slot {
                                            EquipmentSlot::Weapon => "WEAPON",
                                            EquipmentSlot::Shield => "SHIELD",
                                            EquipmentSlot::Helmet => "HEAD",
                                            EquipmentSlot::Body => "BODY",
                                            EquipmentSlot::Accessory => "ACCESSORY",
                                        },
                                        font,
                                        11.0,
                                        status_muted(),
                                    );
                                    spawn_status_text(
                                        name,
                                        item.map_or("Empty slot", item_name),
                                        font,
                                        17.0,
                                        if item.is_some() {
                                            status_ink()
                                        } else {
                                            status_muted()
                                        },
                                    );
                                });
                                card.spawn(Node {
                                    width: px(54),
                                    flex_shrink: 0.0,
                                    align_items: AlignItems::End,
                                    flex_direction: FlexDirection::Column,
                                    row_gap: px(3),
                                    ..default()
                                })
                                .with_children(|bonuses| {
                                    if let Some(item) = item {
                                        let stats = item_stat_bonuses(item);
                                        for (label, value) in [
                                            ("STR", stats.strength),
                                            ("DEX", stats.dexterity),
                                            ("CON", stats.constitution),
                                            ("INT", stats.intelligence),
                                        ] {
                                            if value != 0 {
                                                spawn_status_text(
                                                    bonuses,
                                                    format!("{label} {value:+}"),
                                                    font,
                                                    13.0,
                                                    if value < 0 {
                                                        status_ember()
                                                    } else {
                                                        status_teal()
                                                    },
                                                );
                                            }
                                        }
                                    }
                                });
                            });
                    }
                });
        });
}

pub(super) fn spawn_status_portrait(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
    portraits: &StatusPortraitAssets,
) {
    let Some(member) = member_at(game, state.member_index) else {
        return;
    };
    parent
        .spawn((
            Node {
                width: px(440),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(4),
                padding: UiRect::all(px(10)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(6)),
                ..default()
            },
            BackgroundColor(Color::srgb_u8(22, 22, 28)),
            BorderColor::all(status_border()),
            Name::new("Full status portrait"),
        ))
        .with_children(|panel| {
            let node = Node {
                width: px(PORTRAIT_WIDTH),
                height: px(PORTRAIT_HEIGHT),
                flex_shrink: 0.0,
                ..default()
            };
            if let Some(image) = portraits.large(member.id()) {
                panel.spawn((ImageNode::new(image), node));
            } else {
                panel.spawn(node);
            }
            panel
                .spawn(Node {
                    width: px(PORTRAIT_WIDTH),
                    flex_grow: 1.0,
                    min_height: px(0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                })
                .with_children(|description| {
                    description.spawn((
                        Text::new(catalog.class(member.class_id()).map_or_else(
                            || "No class profile is available.".to_owned(),
                            |class| class.description.replacen(". ", ".\n", 1),
                        )),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor(status_muted()),
                        TextLayout::justify(Justify::Center),
                        Node {
                            width: percent(100),
                            ..default()
                        },
                        Name::new("Status class description"),
                    ));
                });
        });
}
