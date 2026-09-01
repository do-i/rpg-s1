use super::*;

#[expect(
    clippy::too_many_arguments,
    reason = "the items page coordinates the shared menu root and live inventory catalog"
)]
pub(in crate::field_menu) fn sync_items_page(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
    state: Res<FieldMenuState>,
    catalog: Res<FieldMenuCatalog>,
    game: Option<Res<GameState>>,
    menu_roots: Query<Entity, With<FieldMenuRoot>>,
    pages: Query<Entity, With<FieldMenuItemsPage>>,
) {
    let show_items = state.open
        && state.screen == FieldMenuScreen::Items
        && catalog.status() == CatalogStatus::Ready
        && game.is_some();
    if !show_items {
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
    commands.entity(menu_root).with_children(|parent| {
        spawn_items_page(parent, &font, &state, &game, &catalog);
    });
}

pub(in crate::field_menu) fn spawn_items_page(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
) {
    let tab = InventoryTab::ALL[state.tab_index];
    let ids = inventory_ids(game, catalog, tab);
    let list_index = selected_inventory_index(state, &ids);

    parent
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(14),
                ..default()
            },
            FieldMenuItemsPage,
            Name::new("Items page"),
        ))
        .with_children(|page| {
            spawn_items_header(page, font, tab, game.repository().gp());
            page.spawn(Node {
                width: percent(100),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Row,
                column_gap: px(ITEMS_COLUMN_GAP),
                ..default()
            })
            .with_children(|columns| {
                spawn_pouch_column(columns, font, state, game, catalog);
                spawn_item_list_column(columns, font, game, catalog, &ids, list_index);
                spawn_item_detail_column(columns, font, state, game, catalog, &ids, list_index);
            });

            let hint = match state.mode {
                FieldMenuMode::Browse => {
                    "LEFT/RIGHT   POUCH      UP/DOWN   BROWSE      ENTER   ACTIONS      H   SHOW/HIDE"
                }
                FieldMenuMode::ItemNewTag => {
                    "TYPE   a-z 0-9 _      ENTER   ADD      ESC   BACK"
                }
                _ => "UP/DOWN   CHOOSE      ENTER   CONFIRM      ESC   CANCEL      I   CLOSE",
            };
            spawn_status_text(page, hint, font, 15.0, status_muted());
            if !state.message.is_empty() {
                spawn_items_message(page, font, &state.message);
            }
            spawn_item_modal(page, font, state, game, catalog);
        });
}

pub(in crate::field_menu) fn spawn_items_header(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    tab: InventoryTab,
    gp: u32,
) {
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
                    spawn_status_text(title, "ITEMS", font, 31.0, status_ink());
                    spawn_status_text(
                        title,
                        format!("{} POUCH", tab.label().to_uppercase()),
                        font,
                        14.0,
                        status_muted(),
                    );
                });
            spawn_status_text(header, format!("GP  {gp}"), font, 18.0, status_gold());
        });
}

pub(in crate::field_menu) fn spawn_pouch_column(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
) {
    spawn_status_panel(
        parent,
        Node {
            width: px(ITEMS_POUCH_WIDTH),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(7),
            padding: UiRect::all(px(14)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        "POUCH",
        font,
        |panel| {
            for (index, tab) in InventoryTab::ALL.into_iter().enumerate() {
                let selected = index == state.tab_index;
                let count = inventory_ids(game, catalog, tab).len();
                panel
                    .spawn((
                        Node {
                            width: percent(100),
                            height: px(48),
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            padding: UiRect::horizontal(px(10)),
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
                            Color::srgba_u8(126, 98, 55, 96)
                        }),
                        ItemPouchRow,
                    ))
                    .with_children(|row| {
                        row.spawn((
                            Node {
                                width: px(5),
                                height: px(28),
                                margin: UiRect::right(px(10)),
                                ..default()
                            },
                            BackgroundColor(if selected {
                                status_ember()
                            } else {
                                status_border()
                            }),
                        ));
                        row.spawn(Node {
                            flex_grow: 1.0,
                            ..default()
                        })
                        .with_children(|label| {
                            spawn_status_text(
                                label,
                                tab.label(),
                                font,
                                17.0,
                                if count == 0 {
                                    status_faint()
                                } else {
                                    status_ink()
                                },
                            );
                        });
                        spawn_status_text(row, count.to_string(), font, 14.0, status_muted());
                    });
            }
        },
    );
}

pub(in crate::field_menu) fn spawn_item_list_column(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    game: &GameState,
    catalog: &FieldMenuCatalog,
    ids: &[&str],
    list_index: usize,
) {
    spawn_status_panel(
        parent,
        Node {
            height: percent(100),
            flex_basis: px(0),
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(5),
            padding: UiRect::all(px(14)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(6)),
            overflow: Overflow::clip(),
            ..default()
        },
        "ITEMS",
        font,
        |panel| {
            if ids.is_empty() {
                spawn_status_text(panel, "Pouch is empty.", font, 18.0, status_muted());
                spawn_status_text(
                    panel,
                    "Use LEFT/RIGHT to inspect another pouch.",
                    font,
                    14.0,
                    status_faint(),
                );
                return;
            }
            let page = inventory_page_range(ids.len(), list_index);
            for (index, id) in ids.iter().enumerate().skip(page.start).take(page.len()) {
                let item = catalog.item(id).expect("filtered inventory item exists");
                let selected = index == list_index;
                let mut row = panel.spawn((
                    Node {
                        width: percent(100),
                        min_height: px(40),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
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
                    ItemListRow,
                ));
                if selected {
                    row.insert(SelectedItemListRow);
                }
                row.with_children(|row| {
                    row.spawn((
                        Node {
                            width: px(5),
                            height: px(24),
                            margin: UiRect::right(px(9)),
                            ..default()
                        },
                        BackgroundColor(item_accent(item)),
                    ));
                    row.spawn(Node {
                        flex_grow: 1.0,
                        ..default()
                    })
                    .with_children(|name| {
                        spawn_status_text(name, item_name(item), font, 16.0, status_ink());
                    });
                    if game.repository().is_new_item(id) {
                        spawn_status_text(row, "NEW", font, 11.0, status_ember());
                    }
                    spawn_status_text(
                        row,
                        format!("  x{}", game.repository().item_count(id)),
                        font,
                        14.0,
                        status_muted(),
                    );
                });
            }
            let page_number = list_index / INVENTORY_PAGE_ROWS + 1;
            let page_count = ids.len().div_ceil(INVENTORY_PAGE_ROWS);
            spawn_status_text(
                panel,
                format!("PAGE {page_number:02} / {page_count:02}"),
                font,
                12.0,
                status_faint(),
            );
        },
    );
}

pub(in crate::field_menu) fn spawn_item_detail_column(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
    ids: &[&str],
    list_index: usize,
) {
    spawn_status_panel(
        parent,
        Node {
            width: px(ITEMS_DETAIL_WIDTH),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(11),
            padding: UiRect::all(px(16)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        "DETAIL",
        font,
        |panel| {
            let Some(id) = selected_inventory_id(state, ids, list_index) else {
                spawn_status_text(
                    panel,
                    "Select a pouch containing items to view details.",
                    font,
                    16.0,
                    status_muted(),
                );
                return;
            };
            let Some(item) = catalog.item(id) else {
                return;
            };
            panel
                .spawn(Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(12),
                    ..default()
                })
                .with_children(|heading| {
                    heading
                        .spawn((
                            Node {
                                width: px(56),
                                height: px(56),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(px(2)),
                                border_radius: BorderRadius::all(px(6)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba_u8(10, 10, 14, 188)),
                            BorderColor::all(item_accent(item)),
                        ))
                        .with_children(|icon| {
                            spawn_status_text(
                                icon,
                                item_kind_abbreviation(item),
                                font,
                                15.0,
                                item_accent(item),
                            );
                        });
                    heading
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            flex_grow: 1.0,
                            ..default()
                        })
                        .with_children(|title| {
                            spawn_status_text(title, item_name(item), font, 23.0, status_gold());
                            spawn_status_text(
                                title,
                                format!(
                                    "{}    QUANTITY  x{}",
                                    item_kind_label(item),
                                    game.repository().item_count(id)
                                ),
                                font,
                                13.0,
                                status_muted(),
                            );
                        });
                });
            spawn_section_rule(panel);
            spawn_status_text(panel, "DESCRIPTION", font, 13.0, status_gold());
            spawn_status_text(panel, item_description(item), font, 16.0, status_ink());
            spawn_section_rule(panel);
            spawn_item_chips(panel, font, game, catalog, id, item);
            spawn_status_text(panel, "ENTER  -  ACTIONS", font, 14.0, status_muted());
        },
    );
}

pub(in crate::field_menu) fn spawn_item_chips(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    game: &GameState,
    catalog: &FieldMenuCatalog,
    id: &str,
    item: &ItemDefinition,
) {
    parent
        .spawn(Node {
            width: percent(100),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(7),
            row_gap: px(7),
            ..default()
        })
        .with_children(|chips| {
            spawn_item_chip(chips, font, item_kind_label(item), item_accent(item));
            if catalog.field_use(id).is_some() {
                spawn_item_chip(chips, font, "FIELD USE", status_teal());
            }
            if game.repository().is_new_item(id) {
                spawn_item_chip(chips, font, "NEW", status_ember());
            }
            if game.repository().is_locked(id) {
                spawn_item_chip(chips, font, "LOCKED", status_violet());
            }
        });
}

pub(in crate::field_menu) fn spawn_item_chip(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    label: &str,
    color: Color,
) {
    parent
        .spawn((
            Node {
                padding: UiRect::axes(px(9), px(4)),
                border_radius: BorderRadius::all(px(11)),
                ..default()
            },
            BackgroundColor(color),
        ))
        .with_children(|chip| {
            spawn_status_text(chip, label, font, 11.0, Color::srgb_u8(20, 17, 12));
        });
}

pub(in crate::field_menu) fn spawn_items_message(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    message: &str,
) {
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(360),
                right: px(360),
                bottom: px(40),
                justify_content: JustifyContent::Center,
                padding: UiRect::axes(px(14), px(8)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(5)),
                ..default()
            },
            BackgroundColor(Color::srgba_u8(22, 22, 28, 244)),
            BorderColor::all(status_border_active()),
        ))
        .with_children(|banner| {
            spawn_status_text(banner, message, font, 14.0, status_ink());
        });
}

pub(in crate::field_menu) fn spawn_item_modal(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
) {
    if state.mode == FieldMenuMode::Browse {
        return;
    }
    let title = match state.mode {
        FieldMenuMode::ItemManage => "MANAGE POUCH",
        _ => state
            .pending_id
            .as_deref()
            .and_then(|id| catalog.item(id))
            .map_or("ITEM ACTION", item_name),
    };
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(0),
                bottom: px(0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba_u8(2, 2, 6, 150)),
        ))
        .with_children(|overlay| {
            spawn_status_panel(
                overlay,
                Node {
                    width: px(390),
                    min_height: px(190),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(9),
                    padding: UiRect::all(px(16)),
                    border: UiRect::all(px(2)),
                    border_radius: BorderRadius::all(px(7)),
                    ..default()
                },
                &title.to_uppercase(),
                font,
                |modal| {
                    modal.spawn(ItemActionModal);
                    match state.mode {
                        FieldMenuMode::ItemActions => {
                            for (index, (label, subtitle)) in ITEM_ACTIONS.into_iter().enumerate() {
                                spawn_item_modal_row(
                                    modal,
                                    font,
                                    label,
                                    subtitle,
                                    index == state.selected,
                                );
                            }
                        }
                        FieldMenuMode::ItemTags => spawn_tag_editor(modal, font, state, game),
                        FieldMenuMode::ItemNewTag => spawn_new_tag_prompt(modal, font, state),
                        FieldMenuMode::ItemManage => {
                            spawn_manage_rows(modal, font, state, game, catalog);
                        }
                        FieldMenuMode::DiscardQuantity => {
                            spawn_status_text(
                                modal,
                                "DISCARD QUANTITY",
                                font,
                                13.0,
                                status_muted(),
                            );
                            spawn_status_text(
                                modal,
                                format!("{:02}", state.quantity),
                                font,
                                34.0,
                                status_gold(),
                            );
                            spawn_status_text(
                                modal,
                                "LEFT one    RIGHT whole stack    UP/DOWN adjust",
                                font,
                                14.0,
                                status_muted(),
                            );
                        }
                        FieldMenuMode::ItemTarget => {
                            spawn_status_text(modal, "CHOOSE A TARGET", font, 13.0, status_muted());
                            for (index, member) in game.party().members().enumerate() {
                                spawn_item_modal_row(
                                    modal,
                                    font,
                                    member.name(),
                                    &format!(
                                        "HP {}/{}    MP {}/{}",
                                        member.health(),
                                        member.max_health(),
                                        member.mana(),
                                        member.max_mana()
                                    ),
                                    index == state.selected,
                                );
                            }
                        }
                        FieldMenuMode::ItemAoeConfirm => {
                            spawn_status_text(
                                modal,
                                "USE ON THE WHOLE PARTY?",
                                font,
                                13.0,
                                status_muted(),
                            );
                            for member in game
                                .party()
                                .members()
                                .filter(|member| !member.is_knocked_out())
                            {
                                spawn_item_modal_row(
                                    modal,
                                    font,
                                    member.name(),
                                    &format!(
                                        "HP {}/{}    MP {}/{}",
                                        member.health(),
                                        member.max_health(),
                                        member.mana(),
                                        member.max_mana()
                                    ),
                                    false,
                                );
                            }
                            spawn_status_text(
                                modal,
                                "Y/ENTER use    N/ESC cancel",
                                font,
                                14.0,
                                status_muted(),
                            );
                        }
                        _ => {}
                    }
                },
            );
        });
}

/// Tag editor rows: the curatorial set, this item's own tags, then the free-text prompt.
///
/// Ports the `M_TAGS` modal from `item_scene`. A held tag reads `ON` so one glance shows what the
/// item already carries, and custom tags are marked so the player can tell them from the fixed set.
fn spawn_tag_editor(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
) {
    let Some(id) = state.pending_id.as_deref() else {
        return;
    };
    spawn_status_text(parent, "EDIT TAGS", font, 13.0, status_muted());
    let rows = tag_editor_rows(game, id);
    let first = window_start(state.selected, rows.len(), TAG_EDITOR_VISIBLE_ROWS);
    for (index, row) in rows
        .into_iter()
        .enumerate()
        .skip(first)
        .take(TAG_EDITOR_VISIBLE_ROWS)
    {
        let selected = index == state.selected;
        match row {
            TagEditorRow::New => {
                spawn_item_modal_row(parent, font, "New tag…", "type a custom tag", selected);
            }
            TagEditorRow::Tag(tag) => {
                let held = game
                    .repository()
                    .item_tags(id)
                    .any(|current| current == tag);
                let custom = !EDITABLE_SYSTEM_TAGS.contains(&tag.as_str());
                let subtitle = match (held, custom) {
                    (true, true) => "ON    custom",
                    (true, false) => "ON",
                    (false, true) => "custom",
                    (false, false) => "OFF",
                };
                spawn_item_modal_row(parent, font, &tag, subtitle, selected);
            }
        }
    }
}

/// Free-text prompt for a new custom tag, with a live character budget.
fn spawn_new_tag_prompt(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
) {
    spawn_status_text(parent, "NEW TAG", font, 13.0, status_muted());
    parent
        .spawn((
            Node {
                width: percent(100),
                min_height: px(46),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(11), px(6)),
                border: UiRect::all(px(2)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(Color::srgba_u8(10, 10, 14, 200)),
            BorderColor::all(status_border_active()),
        ))
        .with_children(|field| {
            spawn_status_text(
                field,
                format!("{}_", state.text_input),
                font,
                22.0,
                status_gold(),
            );
        });
    spawn_status_text(
        parent,
        format!(
            "{}/{CUSTOM_TAG_MAX_LENGTH}    a-z 0-9 _",
            state.text_input.chars().count()
        ),
        font,
        12.0,
        status_muted(),
    );
    spawn_status_text(
        parent,
        "ENTER   ADD      ESC   BACK",
        font,
        13.0,
        status_muted(),
    );
}

/// The show/hide manager: every owned item, hidden ones included, so any of them can come back.
fn spawn_manage_rows(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
) {
    let ids = manage_ids(game, catalog);
    spawn_status_text(parent, "SHOW / HIDE", font, 13.0, status_muted());
    let first = window_start(state.selected, ids.len(), ITEM_MANAGE_VISIBLE_ROWS);
    for (index, id) in ids
        .iter()
        .enumerate()
        .skip(first)
        .take(ITEM_MANAGE_VISIBLE_ROWS)
    {
        let name = catalog.item(id).map_or(*id, item_name);
        let hidden = game.repository().is_hidden(id);
        spawn_item_modal_row(
            parent,
            font,
            name,
            if hidden { "HIDDEN" } else { "shown" },
            index == state.selected,
        );
    }
    spawn_status_text(
        parent,
        "ENTER   TOGGLE      H / ESC   BACK",
        font,
        13.0,
        status_muted(),
    );
}

pub(in crate::field_menu) fn spawn_item_modal_row(
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
                min_height: px(48),
                flex_direction: FlexDirection::Column,
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
