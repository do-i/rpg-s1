use super::*;
use crate::item_images::spawn_item_image;

#[expect(
    clippy::too_many_arguments,
    reason = "the equipment page coordinates the shared menu root, portraits, and live inventory catalog"
)]
pub(in crate::field_menu) fn sync_equipment_page(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
    state: Res<FieldMenuState>,
    catalog: Res<FieldMenuCatalog>,
    game: Option<Res<GameState>>,
    menu_roots: Query<Entity, With<FieldMenuRoot>>,
    pages: Query<Entity, With<FieldMenuEquipmentPage>>,
) {
    let show_equipment = state.open
        && state.screen == FieldMenuScreen::Equipment
        && catalog.status() == CatalogStatus::Ready
        && game.is_some();
    if !show_equipment {
        for entity in &pages {
            commands.entity(entity).despawn();
        }
        return;
    }

    let Ok(menu_root) = menu_roots.single() else {
        return;
    };
    let Some(game) = game else {
        return;
    };
    let rebuild =
        pages.is_empty() || state.is_changed() || catalog.is_changed() || game.is_changed();
    if !rebuild {
        return;
    }
    for entity in &pages {
        commands.entity(entity).despawn();
    }

    let Some(font) = scenario_font(&asset_server, &root, &inventory) else {
        return;
    };
    let portraits = StatusPortraitAssets::load(&asset_server, &root, &game);
    commands.entity(menu_root).with_children(|parent| {
        spawn_equipment_page(parent, &font, &state, &game, &catalog, &portraits);
    });
}

pub(in crate::field_menu) fn spawn_equipment_page(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
    portraits: &StatusPortraitAssets,
) {
    parent
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(14),
                ..default()
            },
            FieldMenuEquipmentPage,
            Name::new("Equipment page"),
        ))
        .with_children(|page| {
            spawn_equipment_header(page, font, state, game);
            page.spawn(Node {
                width: percent(100),
                flex_grow: 1.0,
                min_height: px(0),
                column_gap: px(STATUS_COLUMN_GAP),
                ..default()
            })
            .with_children(|columns| {
                spawn_party_column(
                    columns,
                    font,
                    state,
                    game,
                    catalog,
                    portraits,
                    state.mode == FieldMenuMode::Browse,
                );
                columns
                    .spawn(Node {
                        flex_basis: px(0),
                        flex_grow: 1.0,
                        min_width: px(0),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(16),
                        ..default()
                    })
                    .with_children(|workspace| {
                        spawn_equipped_shelf(workspace, font, state, game, catalog);
                        workspace
                            .spawn(Node {
                                width: percent(100),
                                flex_grow: 1.0,
                                min_height: px(0),
                                column_gap: px(16),
                                ..default()
                            })
                            .with_children(|lower| {
                                spawn_equipment_inventory(lower, font, state, game, catalog);
                                spawn_equipment_details(lower, font, state, game, catalog);
                            });
                    });
            });
            let hint = equipment_hint(state.mode);
            spawn_status_text(
                page,
                if state.message.is_empty() {
                    hint
                } else {
                    &state.message
                },
                font,
                15.0,
                if state.message.is_empty() {
                    status_muted()
                } else {
                    status_gold()
                },
            );
        });
}

pub(in crate::field_menu) fn equipment_hint(mode: FieldMenuMode) -> &'static str {
    match mode {
        FieldMenuMode::EquipmentSlots => {
            "LEFT/RIGHT   SELECT SLOT      DOWN / ENTER   INVENTORY      ESC   PARTY"
        }
        FieldMenuMode::EquipmentPicker => {
            "UP/DOWN   PREVIEW ITEM      RIGHT / ENTER   EQUIP BUTTON      LEFT / ESC   EQUIPPED"
        }
        FieldMenuMode::EquipmentConfirm => {
            "ENTER   EQUIP & RETURN TO INVENTORY      LEFT / ESC   CANCEL"
        }
        _ => "UP/DOWN   SELECT MEMBER      RIGHT / ENTER   EQUIPPED      ESC   BACK",
    }
}

fn spawn_equipment_header(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
) {
    let member_name =
        member_at(game, state.member_index).map_or("NO MEMBER", |member| member.name());
    parent
        .spawn(Node {
            width: percent(100),
            height: px(64),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|header| {
            spawn_header_bars(header, 46.0, 15.0);
            header
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    ..default()
                })
                .with_children(|title| {
                    spawn_status_text(title, "EQUIPMENT", font, 31.0, status_ink());
                    spawn_status_text(title, "GEAR, COMPARE, COMMIT", font, 14.0, status_muted());
                });
            spawn_status_text(
                header,
                member_name.to_uppercase(),
                font,
                16.0,
                status_gold(),
            );
        });
}

fn spawn_equipped_shelf(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
) {
    let Some(member) = member_at(game, state.member_index) else {
        return;
    };
    let focused = state.mode == FieldMenuMode::EquipmentSlots;
    let selected_slot = (state.mode != FieldMenuMode::Browse).then(|| state_slot_index(state));
    spawn_focus_panel(
        parent,
        Node {
            width: percent(100),
            height: px(EQUIPMENT_SHELF_HEIGHT),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            padding: UiRect::all(px(14)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        "EQUIPPED",
        font,
        focused,
        |panel| {
            panel
                .spawn((
                    Node {
                        width: percent(100),
                        flex_grow: 1.0,
                        min_height: px(0),
                        column_gap: px(10),
                        ..default()
                    },
                    Name::new("Equipped cards"),
                ))
                .with_children(|cards| {
                    for (index, slot) in EquipmentSlot::ALL.into_iter().enumerate() {
                        let selected = Some(index) == selected_slot;
                        let equipped = member.equipment().get(slot).and_then(|id| catalog.item(id));
                        let colors = if selected && !focused {
                            (
                                BackgroundColor(Color::srgba_u8(10, 10, 14, 148)),
                                BorderColor::all(status_gold().with_alpha(0.65)),
                            )
                        } else {
                            selectable_card_colors(selected)
                        };
                        let mut card = cards.spawn((
                            Node {
                                flex_basis: px(0),
                                flex_grow: 1.0,
                                min_width: px(142),
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                row_gap: px(4),
                                padding: UiRect::all(px(6)),
                                border: UiRect::all(px(1)),
                                border_radius: BorderRadius::all(px(4)),
                                ..default()
                            },
                            colors,
                            EquipmentSlotRow,
                        ));
                        if selected {
                            card.insert(SelectedEquipmentSlotRow);
                        }
                        card.with_children(|card| {
                            spawn_status_text(
                                card,
                                slot.as_str().to_uppercase(),
                                font,
                                12.0,
                                status_muted(),
                            );
                            if let Some(item) = equipped {
                                spawn_item_image(card, item.id());
                            } else {
                                card.spawn(Node {
                                    width: px(128),
                                    height: px(128),
                                    flex_shrink: 0.0,
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    ..default()
                                })
                                .with_children(|empty| {
                                    spawn_status_text(empty, "EMPTY", font, 15.0, status_faint());
                                });
                            }
                            card.spawn((
                                Text::new(equipped.map_or("Empty slot", item_name)),
                                TextFont {
                                    font: font.clone().into(),
                                    font_size: FontSize::Px(16.0),
                                    ..default()
                                },
                                TextColor(if equipped.is_some() {
                                    status_ink()
                                } else {
                                    status_faint()
                                }),
                                TextLayout::new(Justify::Center, LineBreak::WordOrCharacter),
                                Node {
                                    width: percent(100),
                                    min_height: px(34),
                                    ..default()
                                },
                            ));
                        });
                    }
                });
        },
    );
}

fn spawn_equipment_inventory(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
) {
    let Some(member) = member_at(game, state.member_index) else {
        return;
    };
    let slot = EquipmentSlot::ALL[state_slot_index(state)];
    let browsing = state.mode == FieldMenuMode::Browse;
    let title = if browsing {
        "INVENTORY".to_owned()
    } else {
        format!("{} INVENTORY", slot.as_str().to_uppercase())
    };
    spawn_focus_panel(
        parent,
        Node {
            width: percent(46),
            height: percent(100),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            padding: UiRect::all(px(14)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        &title,
        font,
        state.mode == FieldMenuMode::EquipmentPicker,
        |panel| {
            if browsing {
                panel
                    .spawn(Node {
                        width: percent(100),
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        row_gap: px(12),
                        ..default()
                    })
                    .with_children(|empty| {
                        spawn_status_text(
                            empty,
                            "Choose an equipped card",
                            font,
                            22.0,
                            status_muted(),
                        );
                        spawn_status_text(
                            empty,
                            "Its available items will appear here.",
                            font,
                            15.0,
                            status_muted(),
                        );
                    });
                return;
            }
            spawn_equipment_picker(panel, font, state, game, catalog, member, slot);
        },
    );
}

fn spawn_equipment_picker(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
    member: &crate::runtime_member::RuntimeMember,
    slot: EquipmentSlot,
) {
    let candidates = equipment_candidates(game, catalog, slot);
    let picking = matches!(
        state.mode,
        FieldMenuMode::EquipmentPicker | FieldMenuMode::EquipmentConfirm
    );
    let window = state.scroll.window(
        "equipment_picker",
        if picking { state.selected } else { 0 },
        candidates.len() + 1,
        EQUIPMENT_PICKER_VISIBLE_ROWS,
    );
    spawn_scroll_list(parent, window, scroll_list_node(8.0), |list| {
        for index in window.indices() {
            let candidate_id = candidates.get(index).map(String::as_str);
            let item = candidate_id.and_then(|id| catalog.item(id));
            let refusal = candidate_id.and_then(|id| equip_refusal(member, id, catalog));
            let selected = picking && index == state.selected;
            let focused = state.mode == FieldMenuMode::EquipmentPicker;
            let colors = if selected && !focused {
                (
                    BackgroundColor(Color::srgba_u8(30, 29, 31, 218)),
                    BorderColor::all(status_gold().with_alpha(0.65)),
                )
            } else {
                selectable_card_colors(selected)
            };
            let mut row = list.spawn((
                Node {
                    width: percent(100),
                    height: px(52),
                    flex_shrink: 0.0,
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    padding: UiRect::axes(px(11), px(5)),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(4)),
                    ..default()
                },
                colors,
                EquipmentPickerRow,
            ));
            if selected {
                row.insert(SelectedEquipmentPickerRow);
            }
            row.with_children(|row| {
                spawn_status_text(
                    row,
                    item.map_or("(Unequip)", item_name),
                    font,
                    16.0,
                    if refusal.is_none() {
                        status_ink()
                    } else {
                        status_muted()
                    },
                );
                let (summary, color) = if refusal.is_some() {
                    ("CAN'T EQUIP - CLASS RESTRICTED".to_owned(), status_ember())
                } else if let Some(id) = candidate_id {
                    (
                        equipment_item_summary(member, catalog, slot, id),
                        status_muted(),
                    )
                } else if member.equipment().get(slot).is_none() {
                    ("slot already empty".to_owned(), status_faint())
                } else {
                    (
                        "return current item to inventory".to_owned(),
                        status_muted(),
                    )
                };
                spawn_status_text(row, summary, font, 12.0, color);
            });
        }
    });
    if candidates.is_empty() {
        spawn_status_text(
            parent,
            "No replacement items in inventory.",
            font,
            13.0,
            status_muted(),
        );
    }
}

fn spawn_equipment_details(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
) {
    let Some(member) = member_at(game, state.member_index) else {
        return;
    };
    let slot = EquipmentSlot::ALL[state_slot_index(state)];
    let comparing = matches!(
        state.mode,
        FieldMenuMode::EquipmentPicker | FieldMenuMode::EquipmentConfirm
    );
    spawn_focus_panel(
        parent,
        Node {
            height: percent(100),
            flex_basis: px(0),
            flex_grow: 1.0,
            min_width: px(0),
            flex_direction: FlexDirection::Column,
            row_gap: px(6),
            padding: UiRect::all(px(14)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        if comparing { "COMPARE" } else { "DETAILS" },
        font,
        state.mode == FieldMenuMode::EquipmentConfirm,
        |panel| {
            let context = if comparing {
                let candidates = equipment_candidates(game, catalog, slot);
                let current = member.equipment().get(slot).and_then(|id| catalog.item(id));
                let replacement =
                    equipment_candidate(state, &candidates).and_then(|id| catalog.item(id));
                format!(
                    "{} -> {}",
                    current.map_or("Empty", item_name),
                    replacement.map_or("Unequip", item_name)
                )
            } else if state.mode == FieldMenuMode::Browse {
                member.name().to_uppercase()
            } else {
                format!("{} / CURRENTLY EQUIPPED", slot.as_str().to_uppercase())
            };
            panel
                .spawn(Node {
                    position_type: PositionType::Absolute,
                    top: px(14),
                    right: px(14),
                    max_width: percent(76),
                    ..default()
                })
                .with_children(|context_row| {
                    spawn_status_text(context_row, context, font, 12.0, status_muted());
                });
            if state.mode == FieldMenuMode::Browse {
                panel.spawn(Node {
                    flex_grow: 1.0,
                    ..default()
                });
                spawn_status_text(panel, "Current loadout", font, 26.0, status_gold());
                spawn_status_text(
                    panel,
                    "Choose a slot to inspect its equipped item.",
                    font,
                    15.0,
                    status_muted(),
                );
                panel.spawn(Node {
                    flex_grow: 1.0,
                    ..default()
                });
                spawn_status_text(panel, "CHARACTER TOTALS", font, 12.0, status_muted());
                spawn_equipment_stat_grid(panel, font, derived_stats(member, catalog));
            } else if comparing {
                let candidates = equipment_candidates(game, catalog, slot);
                spawn_equipment_preview(panel, font, state, catalog, member, slot, &candidates);
            } else {
                let equipped = member.equipment().get(slot).and_then(|id| catalog.item(id));
                spawn_equipment_item_heading(
                    panel,
                    font,
                    equipped,
                    "EMPTY SLOT",
                    "No item is currently equipped in this slot.",
                );
                if let Some(item) = equipped {
                    spawn_status_text(
                        panel,
                        equipment_item_summary(member, catalog, slot, item.id()),
                        font,
                        14.0,
                        status_gold(),
                    );
                }
                panel.spawn(Node {
                    flex_grow: 1.0,
                    min_height: px(0),
                    ..default()
                });
                spawn_status_text(
                    panel,
                    "CURRENT CHARACTER TOTALS",
                    font,
                    12.0,
                    status_muted(),
                );
                spawn_equipment_stat_grid(panel, font, derived_stats(member, catalog));
                spawn_status_text(
                    panel,
                    "DOWN / ENTER   SELECT REPLACEMENT",
                    font,
                    12.0,
                    status_muted(),
                );
            }
        },
    );
}

fn spawn_equipment_item_heading(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    item: Option<&ItemDefinition>,
    empty_title: &str,
    empty_description: &str,
) {
    parent
        .spawn(Node {
            width: percent(100),
            height: px(128),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            column_gap: px(12),
            ..default()
        })
        .with_children(|heading| {
            if let Some(item) = item {
                spawn_item_image(heading, item.id());
            }
            heading
                .spawn(Node {
                    flex_basis: px(0),
                    flex_grow: 1.0,
                    min_width: px(0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(8),
                    ..default()
                })
                .with_children(|copy| {
                    spawn_status_text(
                        copy,
                        item.map_or(empty_title, item_name),
                        font,
                        24.0,
                        status_gold(),
                    );
                    spawn_status_text(
                        copy,
                        item.map_or(empty_description, item_description),
                        font,
                        13.0,
                        status_muted(),
                    );
                });
        });
}

fn spawn_equipment_preview(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    catalog: &FieldMenuCatalog,
    member: &crate::runtime_member::RuntimeMember,
    slot: EquipmentSlot,
    candidates: &[String],
) {
    let candidate_id = equipment_candidate(state, candidates);
    let item = candidate_id.and_then(|id| catalog.item(id));
    let refusal = candidate_id.and_then(|id| equip_refusal(member, id, catalog));
    spawn_equipment_item_heading(
        parent,
        font,
        item,
        "(Unequip)",
        "Return the equipped item to inventory.",
    );
    parent.spawn(Node {
        flex_grow: 1.0,
        min_height: px(0),
        ..default()
    });
    let before = derived_stats(member, catalog);
    let empty_slot = candidate_id.is_none() && member.equipment().get(slot).is_none();
    let available = refusal.is_none() && !empty_slot;
    if let Some(reason) = &refusal {
        spawn_status_text(parent, reason, font, 12.0, status_ember());
        spawn_equipment_stat_grid(parent, font, before);
    } else {
        spawn_status_text(
            parent,
            "CHARACTER TOTALS AFTER EQUIPPING",
            font,
            12.0,
            status_muted(),
        );
        spawn_stat_change_grid(
            parent,
            font,
            before,
            preview_stats(member, catalog, slot, candidate_id),
        );
    }
    let focused = state.mode == FieldMenuMode::EquipmentConfirm;
    parent
        .spawn((
            Node {
                width: percent(100),
                height: px(34),
                flex_shrink: 0.0,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            selectable_card_colors(focused && available),
            EquipmentCommitButton,
        ))
        .with_children(|button| {
            let label = if empty_slot {
                "SLOT ALREADY EMPTY"
            } else if !available {
                "CANNOT EQUIP"
            } else if !focused {
                "RIGHT / ENTER   REVIEW & EQUIP"
            } else if candidate_id.is_some() {
                "ENTER   EQUIP"
            } else {
                "ENTER   UNEQUIP"
            };
            spawn_status_text(
                button,
                label,
                font,
                13.0,
                if available {
                    status_gold()
                } else {
                    status_faint()
                },
            );
        });
}

fn spawn_equipment_stat_grid(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    stats: crate::field_menu_domain::DerivedStats,
) {
    parent
        .spawn(Node {
            width: percent(100),
            column_gap: px(8),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|grid| {
            for (label, value) in [
                ("STR", stats.strength),
                ("DEX", stats.dexterity),
                ("CON", stats.constitution),
                ("INT", stats.intelligence),
            ] {
                grid.spawn((
                    Node {
                        flex_basis: px(0),
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::axes(px(3), px(6)),
                        border: UiRect::all(px(1)),
                        border_radius: BorderRadius::all(px(4)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba_u8(10, 10, 14, 150)),
                    BorderColor::all(status_border()),
                ))
                .with_children(|stat| {
                    spawn_status_text(stat, label, font, 11.0, status_muted());
                    spawn_status_text(stat, value.to_string(), font, 18.0, status_gold());
                });
            }
        });
}

fn equipment_item_summary(
    member: &crate::runtime_member::RuntimeMember,
    catalog: &FieldMenuCatalog,
    slot: EquipmentSlot,
    id: &str,
) -> String {
    let base = preview_stats(member, catalog, slot, None);
    let with_item = preview_stats(member, catalog, slot, Some(id));
    let summary = [
        ("STR", with_item.strength - base.strength),
        ("DEX", with_item.dexterity - base.dexterity),
        ("CON", with_item.constitution - base.constitution),
        ("INT", with_item.intelligence - base.intelligence),
    ]
    .into_iter()
    .filter(|(_, delta)| *delta != 0)
    .map(|(label, delta)| format!("{delta:+} {label}"))
    .collect::<Vec<_>>()
    .join("  /  ");
    if summary.is_empty() {
        "No attribute bonus".to_owned()
    } else {
        summary
    }
}

/// One tile per attribute reading `old -> new`, bordered and tinted by the change's direction.
pub(in crate::field_menu) fn spawn_stat_change_grid(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    before: crate::field_menu_domain::DerivedStats,
    after: crate::field_menu_domain::DerivedStats,
) {
    parent
        .spawn(Node {
            width: percent(100),
            flex_direction: FlexDirection::Row,
            column_gap: px(8),
            ..default()
        })
        .with_children(|preview| {
            for (label, old, new) in [
                ("STR", before.strength, after.strength),
                ("DEX", before.dexterity, after.dexterity),
                ("CON", before.constitution, after.constitution),
                ("INT", before.intelligence, after.intelligence),
            ] {
                let color = stat_change_color(old, new);
                preview
                    .spawn((
                        Node {
                            flex_basis: px(0),
                            flex_grow: 1.0,
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            padding: UiRect::axes(px(3), px(6)),
                            border: UiRect::all(px(1)),
                            border_radius: BorderRadius::all(px(4)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba_u8(10, 10, 14, 150)),
                        BorderColor::all(color),
                    ))
                    .with_children(|stat| {
                        spawn_status_text(stat, label, font, 11.0, status_muted());
                        spawn_status_text(stat, format!("{old} -> {new}"), font, 14.0, color);
                    });
            }
        });
}

pub(in crate::field_menu) fn equipment_preview_summary(
    member: &crate::runtime_member::RuntimeMember,
    catalog: &FieldMenuCatalog,
    slot: EquipmentSlot,
    candidate_id: Option<&str>,
) -> String {
    let before = derived_stats(member, catalog);
    let after = preview_stats(member, catalog, slot, candidate_id);
    [
        ("STR", before.strength, after.strength),
        ("DEX", before.dexterity, after.dexterity),
        ("CON", before.constitution, after.constitution),
        ("INT", before.intelligence, after.intelligence),
    ]
    .into_iter()
    .filter(|(_, old, new)| old != new)
    .map(|(label, old, new)| format!("{label} {old} -> {new}"))
    .collect::<Vec<_>>()
    .join("    ")
}

pub(in crate::field_menu) fn stat_change_color(old: i32, new: i32) -> Color {
    if new > old {
        Color::srgb_u8(120, 220, 120)
    } else if new < old {
        Color::srgb_u8(220, 110, 110)
    } else {
        status_muted()
    }
}
