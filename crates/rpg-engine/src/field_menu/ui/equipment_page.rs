use super::*;

#[expect(
    clippy::too_many_arguments,
    reason = "the equipment page coordinates the shared menu root, portraits, and live inventory catalog"
)]
pub(in crate::field_menu) fn sync_equipment_page(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
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

    let font = asset_server.load(
        root.resolve(
            &ScenarioRelativePath::try_from("assets/fonts/Philosopher-Regular.ttf")
                .expect("equipment font path"),
        ),
    );
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
                flex_direction: FlexDirection::Row,
                column_gap: px(STATUS_COLUMN_GAP),
                ..default()
            })
            .with_children(|columns| {
                spawn_party_column(columns, font, state, game, catalog, portraits);
                spawn_equipment_slots_column(columns, font, state, game, catalog);
                spawn_equipment_inventory_column(columns, font, state, game, catalog);
            });
            spawn_status_text(
                page,
                if state.mode == FieldMenuMode::EquipmentPicker {
                    "UP/DOWN   PREVIEW ITEM      ENTER   EQUIP      ESC   SLOTS      M   CLOSE"
                } else {
                    "LEFT/RIGHT   CHANGE MEMBER      UP/DOWN   SELECT SLOT      ENTER   INVENTORY      ESC   BACK"
                },
                font,
                15.0,
                status_muted(),
            );
            if !state.message.is_empty() {
                spawn_items_message(page, font, &state.message);
            }
        });
}

pub(in crate::field_menu) fn spawn_equipment_header(
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
            flex_direction: FlexDirection::Row,
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

pub(in crate::field_menu) fn spawn_equipment_slots_column(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
) {
    let Some(member) = member_at(game, state.member_index) else {
        return;
    };
    let selected_slot = state_slot_index(state);
    spawn_status_panel(
        parent,
        Node {
            width: px(EQUIPMENT_SLOT_WIDTH),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            padding: UiRect::all(px(14)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        "SLOTS",
        font,
        |panel| {
            for (index, slot) in EquipmentSlot::ALL.into_iter().enumerate() {
                let selected = index == selected_slot;
                let equipped = member.equipment().get(slot).and_then(|id| catalog.item(id));
                let mut row = panel.spawn((
                    Node {
                        width: percent(100),
                        min_height: px(58),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        padding: UiRect::axes(px(10), px(6)),
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
                        Color::srgba_u8(126, 98, 55, 90)
                    }),
                    EquipmentSlotRow,
                ));
                if selected {
                    row.insert(SelectedEquipmentSlotRow);
                }
                row.with_children(|row| {
                    spawn_status_text(
                        row,
                        slot.as_str().to_uppercase(),
                        font,
                        12.0,
                        status_muted(),
                    );
                    spawn_status_text(
                        row,
                        equipped.map_or("—", item_name),
                        font,
                        16.0,
                        if equipped.is_some() {
                            status_ink()
                        } else {
                            status_faint()
                        },
                    );
                });
            }
            spawn_section_rule(panel);
            spawn_status_text(panel, "TOTALS", font, 13.0, status_gold());
            spawn_equipment_stat_grid(panel, font, derived_stats(member, catalog));
        },
    );
}

pub(in crate::field_menu) fn spawn_equipment_stat_grid(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    stats: crate::field_menu_domain::DerivedStats,
) {
    parent
        .spawn(Node {
            width: percent(100),
            flex_direction: FlexDirection::Row,
            column_gap: px(8),
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

pub(in crate::field_menu) fn spawn_equipment_inventory_column(
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
    spawn_status_panel(
        parent,
        Node {
            height: percent(100),
            flex_basis: px(0),
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            padding: UiRect::all(px(16)),
            border: UiRect::all(px(if state.mode == FieldMenuMode::EquipmentPicker {
                2
            } else {
                1
            })),
            border_radius: BorderRadius::all(px(6)),
            overflow: Overflow::clip(),
            ..default()
        },
        "INVENTORY",
        font,
        |panel| {
            spawn_status_text(
                panel,
                format!("{}  /  {}", member.name(), slot.as_str().to_uppercase()),
                font,
                13.0,
                status_muted(),
            );
            if state.mode == FieldMenuMode::EquipmentPicker {
                spawn_equipment_picker(panel, font, state, game, catalog, member, slot);
            } else {
                spawn_current_equipment_detail(panel, font, member, slot, catalog);
            }
        },
    );
}

pub(in crate::field_menu) fn spawn_current_equipment_detail(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    member: &crate::runtime_member::RuntimeMember,
    slot: EquipmentSlot,
    catalog: &FieldMenuCatalog,
) {
    let equipped = member.equipment().get(slot).and_then(|id| catalog.item(id));
    spawn_section_rule(parent);
    spawn_status_text(
        parent,
        equipped.map_or("EMPTY SLOT", item_name),
        font,
        24.0,
        if equipped.is_some() {
            status_gold()
        } else {
            status_muted()
        },
    );
    spawn_status_text(
        parent,
        equipped.map_or(
            "No item is currently equipped in this slot.",
            item_description,
        ),
        font,
        16.0,
        status_ink(),
    );
    spawn_section_rule(parent);
    spawn_status_text(
        parent,
        "Press ENTER to compare compatible inventory items.",
        font,
        14.0,
        status_muted(),
    );
}

pub(in crate::field_menu) fn spawn_equipment_picker(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
    member: &crate::runtime_member::RuntimeMember,
    slot: EquipmentSlot,
) {
    let candidates = equipment_candidates(game, catalog, slot);
    let total = candidates.len() + 1;
    let visible_rows = EQUIPMENT_PICKER_VISIBLE_ROWS;
    let first = state
        .selected
        .saturating_sub(visible_rows - 1)
        .min(total.saturating_sub(visible_rows));
    for index in first..(first + visible_rows).min(total) {
        let candidate_id = index
            .checked_sub(1)
            .and_then(|candidate| candidates.get(candidate).map(String::as_str));
        let item = candidate_id.and_then(|id| catalog.item(id));
        let selected = index == state.selected;
        parent
            .spawn((
                Node {
                    width: percent(100),
                    min_height: px(52),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    padding: UiRect::axes(px(11), px(5)),
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
                    Color::srgba_u8(126, 98, 55, 90)
                }),
                EquipmentPickerRow,
            ))
            .with_children(|row| {
                spawn_status_text(
                    row,
                    item.map_or("(Unequip)", item_name),
                    font,
                    16.0,
                    if item.is_some() {
                        status_ink()
                    } else {
                        status_muted()
                    },
                );
                spawn_status_text(
                    row,
                    candidate_id.map_or("return current item to pouch".to_owned(), |id| {
                        equipment_preview_summary(member, catalog, slot, Some(id))
                    }),
                    font,
                    12.0,
                    status_muted(),
                );
            });
    }
    spawn_section_rule(parent);
    spawn_equipment_preview(parent, font, state, catalog, member, slot, &candidates);
}

pub(in crate::field_menu) fn spawn_equipment_preview(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    catalog: &FieldMenuCatalog,
    member: &crate::runtime_member::RuntimeMember,
    slot: EquipmentSlot,
    candidates: &[String],
) {
    let candidate_id = state
        .selected
        .checked_sub(1)
        .and_then(|index| candidates.get(index).map(String::as_str));
    let before = derived_stats(member, catalog);
    let after = preview_stats(member, catalog, slot, candidate_id);
    spawn_status_text(parent, "PREVIEW", font, 13.0, status_gold());
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
    if let Some(item) = candidate_id.and_then(|id| catalog.item(id)) {
        spawn_status_text(parent, item_description(item), font, 13.0, status_muted());
    }
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
