use super::*;

pub(super) fn sync_field_menu_overlay_lifecycle(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    theme: Res<UiTheme>,
    state: Res<FieldMenuState>,
    roots: Query<Entity, With<FieldMenuRoot>>,
) {
    if !state.open {
        for entity in &roots {
            commands.entity(entity).despawn();
        }
    } else if roots.is_empty() {
        spawn_field_menu_overlay(&mut commands, &asset_server, &root, &theme);
    }
}

#[expect(
    clippy::type_complexity,
    reason = "the overlay updates three independently styled text roles"
)]
pub(super) fn sync_field_menu_generic_text(
    state: Res<FieldMenuState>,
    catalog: Res<FieldMenuCatalog>,
    saves: Res<SaveSlotCatalog>,
    game: Option<Res<GameState>>,
    mut titles: Query<
        &mut Text,
        (
            With<FieldMenuTitle>,
            Without<FieldMenuBody>,
            Without<FieldMenuHint>,
        ),
    >,
    mut bodies: Query<
        &mut Text,
        (
            With<FieldMenuBody>,
            Without<FieldMenuTitle>,
            Without<FieldMenuHint>,
        ),
    >,
    mut hints: Query<
        &mut Text,
        (
            With<FieldMenuHint>,
            Without<FieldMenuTitle>,
            Without<FieldMenuBody>,
        ),
    >,
) {
    if !state.open {
        return;
    }
    if uses_custom_field_menu_page(&state, &catalog, game.is_some()) {
        return;
    }
    let game_changed = game.as_ref().is_some_and(|game| game.is_changed());
    if !state.is_changed() && !catalog.is_changed() && !saves.is_changed() && !game_changed {
        return;
    }
    let Some(game) = game else { return };
    if let Ok(mut title) = titles.single_mut() {
        title.0 = screen_title(&state).to_owned();
    }
    if let Ok(mut body) = bodies.single_mut() {
        body.0 = render_body(&state, &game, &catalog, &saves);
    }
    if let Ok(mut hint) = hints.single_mut() {
        hint.0 = render_hint(&state);
    }
}

pub(super) fn spawn_field_menu_overlay(
    commands: &mut Commands,
    asset_server: &AssetServer,
    root: &ScenarioRoot,
    theme: &UiTheme,
) {
    let font = asset_server.load(
        root.resolve(
            &ScenarioRelativePath::try_from("assets/fonts/Philosopher-Regular.ttf")
                .expect("field font path"),
        ),
    );
    let backdrop = asset_server.load(
        root.resolve(
            &ScenarioRelativePath::try_from(
                "assets/images/battle_bg/zone4-sanctum-bg-1280x468.webp",
            )
            .expect("field backdrop path"),
        ),
    );
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(30)),
                row_gap: px(14),
                ..default()
            },
            ImageNode::new(backdrop)
                .with_mode(NodeImageMode::Stretch)
                .with_color(Color::srgba(0.18, 0.18, 0.24, 0.36)),
            BackgroundColor(Color::srgba(0.02, 0.02, 0.08, 0.97)),
            GlobalZIndex(4_000),
            Pickable::IGNORE,
            Name::new("Field menu"),
            FieldMenuRoot,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new(""),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(34.0),
                    ..default()
                },
                TextColor(theme.name_entry_input_color),
                FieldMenuTitle,
                FieldMenuGenericContent,
            ));
            panel.spawn((
                Text::new(""),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(21.0),
                    ..default()
                },
                TextColor(Color::srgb_u8(235, 225, 190)),
                TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
                Node {
                    width: percent(100),
                    flex_grow: 1.0,
                    ..default()
                },
                FieldMenuBody,
                FieldMenuGenericContent,
            ));
            panel.spawn((
                Text::new(""),
                TextFont {
                    font: font.into(),
                    font_size: FontSize::Px(17.0),
                    ..default()
                },
                TextColor(theme.name_entry_hint_color),
                FieldMenuHint,
                FieldMenuGenericContent,
            ));
        });
}

pub(super) fn sync_custom_field_menu_content_visibility(
    state: Res<FieldMenuState>,
    catalog: Res<FieldMenuCatalog>,
    game: Option<Res<GameState>>,
    mut generic_nodes: Query<&mut Node, With<FieldMenuGenericContent>>,
) {
    let game_changed = game.as_ref().is_some_and(|game| game.is_changed());
    if !state.is_changed() && !catalog.is_changed() && !game_changed {
        return;
    }
    let show_custom_page = uses_custom_field_menu_page(&state, &catalog, game.is_some());
    for mut node in &mut generic_nodes {
        node.display = if show_custom_page {
            Display::None
        } else {
            Display::Flex
        };
    }
}

pub(super) fn uses_custom_field_menu_page(
    state: &FieldMenuState,
    catalog: &FieldMenuCatalog,
    has_game: bool,
) -> bool {
    state.open
        && has_game
        && (state.screen == FieldMenuScreen::Main
            || (catalog.status() == CatalogStatus::Ready
                && matches!(
                    state.screen,
                    FieldMenuScreen::Status
                        | FieldMenuScreen::Items
                        | FieldMenuScreen::Equipment
                        | FieldMenuScreen::Spells
                        | FieldMenuScreen::Save
                )))
}

pub(super) fn sync_main_menu_page(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    state: Res<FieldMenuState>,
    game: Option<Res<GameState>>,
    menu_roots: Query<Entity, With<FieldMenuRoot>>,
    pages: Query<Entity, With<FieldMenuMainPage>>,
) {
    let show_main = state.open && state.screen == FieldMenuScreen::Main && game.is_some();
    if !show_main {
        for entity in &pages {
            commands.entity(entity).despawn();
        }
        return;
    }

    let Ok(menu_root) = menu_roots.single() else {
        return;
    };
    if !pages.is_empty() && !state.is_changed() {
        return;
    }
    for entity in &pages {
        commands.entity(entity).despawn();
    }

    let font = asset_server.load(
        root.resolve(
            &ScenarioRelativePath::try_from("assets/fonts/Philosopher-Regular.ttf")
                .expect("field menu font path"),
        ),
    );
    commands.entity(menu_root).with_children(|parent| {
        spawn_main_menu_page(parent, &font, &state);
    });
}

pub(super) fn spawn_main_menu_page(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
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
            FieldMenuMainPage,
            Name::new("Original-style field menu page"),
        ))
        .with_children(|page| {
            spawn_main_menu_header(page, font);
            page.spawn(Node {
                width: percent(100),
                flex_grow: 1.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_children(|body| {
                spawn_main_commands_panel(body, font, state);
            });
            spawn_status_text(
                page,
                "UP/DOWN   SELECT      ENTER   CONFIRM      M / ESC   CLOSE",
                font,
                15.0,
                status_muted(),
            );

            if state.mode == FieldMenuMode::QuitConfirm {
                spawn_field_menu_quit_modal(page, font);
            }
        });
}

pub(super) fn spawn_main_menu_header(parent: &mut ChildSpawnerCommands<'_>, font: &Handle<Font>) {
    parent
        .spawn(Node {
            width: percent(100),
            height: px(64),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|header| {
            header.spawn((
                Node {
                    width: px(7),
                    height: px(46),
                    margin: UiRect::right(px(3)),
                    ..default()
                },
                BackgroundColor(status_ember()),
            ));
            header.spawn((
                Node {
                    width: px(2),
                    height: px(46),
                    margin: UiRect::right(px(15)),
                    ..default()
                },
                BackgroundColor(status_gold()),
            ));
            header
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    ..default()
                })
                .with_children(|title| {
                    spawn_status_text(title, "FIELD MENU", font, 31.0, status_ink());
                    spawn_status_text(title, "PARTY COMMAND DECK", font, 14.0, status_muted());
                });
            spawn_status_text(
                header,
                format!("{:02} COMMANDS", MAIN_COMMANDS.len()),
                font,
                16.0,
                status_gold(),
            );
        });
}

pub(super) fn spawn_main_commands_panel(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
) {
    spawn_status_panel(
        parent,
        Node {
            width: px(460),
            height: px(468),
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            padding: UiRect::all(px(16)),
            border: UiRect::all(px(2)),
            border_radius: BorderRadius::all(px(7)),
            ..default()
        },
        "COMMANDS",
        font,
        |panel| {
            for (index, command) in MAIN_COMMANDS.iter().enumerate() {
                spawn_main_command_row(panel, font, command, index, index == state.selected);
            }
        },
    );
}

pub(super) fn spawn_main_command_row(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    command: &MainCommand,
    index: usize,
    selected: bool,
) {
    let accent = main_command_accent(index);
    let mut row = parent.spawn((
        Node {
            width: percent(100),
            height: px(58),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(12),
            padding: UiRect::axes(px(10), px(7)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        BackgroundColor(if selected {
            Color::srgba_u8(83, 55, 31, 238)
        } else {
            Color::srgba_u8(15, 15, 21, 205)
        }),
        BorderColor::all(if selected {
            status_border_active()
        } else {
            status_border()
        }),
        MainCommandRow,
        Name::new(format!("{} command", command.label)),
    ));
    if selected {
        row.insert(SelectedMainCommandRow);
    }
    row.with_children(|row| {
        row.spawn((
            Node {
                width: px(42),
                height: px(42),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(Color::srgba_u8(25, 24, 31, 235)),
            BorderColor::all(accent),
        ))
        .with_children(|badge| {
            spawn_status_text(badge, command.badge, font, 14.0, accent);
        });
        row.spawn(Node {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            ..default()
        })
        .with_children(|copy| {
            spawn_status_text(copy, command.label, font, 19.0, status_ink());
            spawn_status_text(copy, command.description, font, 12.0, status_muted());
        });
        if selected {
            spawn_status_text(row, "ENTER", font, 12.0, status_gold());
        }
    });
}

pub(super) fn spawn_field_menu_quit_modal(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
) {
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.01, 0.01, 0.03, 0.72)),
            FieldMenuQuitModal,
            Name::new("Quit game modal"),
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        width: px(410),
                        min_height: px(170),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(12),
                        padding: UiRect::all(px(18)),
                        border: UiRect::all(px(2)),
                        border_radius: BorderRadius::all(px(7)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb_u8(22, 22, 28)),
                    BorderColor::all(status_border_active()),
                ))
                .with_children(|modal| {
                    spawn_status_text(modal, "QUIT GAME?", font, 14.0, status_gold());
                    spawn_section_rule(modal);
                    spawn_status_text(modal, "Exit to desktop?", font, 20.0, status_ink());
                    spawn_status_text(
                        modal,
                        "Unsaved progress will be lost.",
                        font,
                        14.0,
                        status_muted(),
                    );
                    spawn_section_rule(modal);
                    spawn_status_text(
                        modal,
                        "ENTER / Y   CONFIRM      ESC / N   CANCEL",
                        font,
                        14.0,
                        status_gold(),
                    );
                });
        });
}

pub(super) fn main_command_accent(index: usize) -> Color {
    match index {
        0 => status_teal(),
        1 => status_violet(),
        2 => status_ember(),
        3 => status_gold(),
        4 => Color::srgb_u8(91, 143, 183),
        _ => Color::srgb_u8(190, 72, 66),
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "the status page coordinates the shared menu root, loaded scenario data, and UI assets"
)]
pub(super) fn sync_status_page(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    state: Res<FieldMenuState>,
    catalog: Res<FieldMenuCatalog>,
    game: Option<Res<GameState>>,
    menu_roots: Query<Entity, With<FieldMenuRoot>>,
    pages: Query<Entity, With<FieldMenuStatusPage>>,
) {
    let show_status = state.open
        && state.screen == FieldMenuScreen::Status
        && catalog.status() == CatalogStatus::Ready
        && game.is_some();

    if !show_status {
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
                .expect("status font path"),
        ),
    );
    let portraits = StatusPortraitAssets::load(&asset_server, &root, &game);
    commands.entity(menu_root).with_children(|parent| {
        spawn_status_page(parent, &font, &state, &game, &catalog, &portraits);
    });
}

#[expect(
    clippy::too_many_arguments,
    reason = "the items page coordinates the shared menu root and live inventory catalog"
)]
pub(super) fn sync_items_page(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
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

    let font = asset_server.load(
        root.resolve(
            &ScenarioRelativePath::try_from("assets/fonts/Philosopher-Regular.ttf")
                .expect("items font path"),
        ),
    );
    commands.entity(menu_root).with_children(|parent| {
        spawn_items_page(parent, &font, &state, &game, &catalog);
    });
}

pub(super) fn spawn_items_page(
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

            let hint = if state.mode == FieldMenuMode::Browse {
                "LEFT/RIGHT   CHANGE POUCH      UP/DOWN   BROWSE      ENTER   ACTIONS      ESC   BACK"
            } else {
                "UP/DOWN   CHOOSE      ENTER   CONFIRM      ESC   CANCEL      I   CLOSE"
            };
            spawn_status_text(page, hint, font, 15.0, status_muted());
            if !state.message.is_empty() {
                spawn_items_message(page, font, &state.message);
            }
            spawn_item_modal(page, font, state, game, catalog);
        });
}

pub(super) fn spawn_items_header(
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
            header.spawn((
                Node {
                    width: px(7),
                    height: px(46),
                    margin: UiRect::right(px(3)),
                    ..default()
                },
                BackgroundColor(status_ember()),
            ));
            header.spawn((
                Node {
                    width: px(2),
                    height: px(46),
                    margin: UiRect::right(px(15)),
                    ..default()
                },
                BackgroundColor(status_gold()),
            ));
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

pub(super) fn spawn_pouch_column(
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
                                    Color::srgb_u8(116, 108, 90)
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

pub(super) fn spawn_item_list_column(
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
                    Color::srgb_u8(116, 108, 90),
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
                Color::srgb_u8(116, 108, 90),
            );
        },
    );
}

pub(super) fn spawn_item_detail_column(
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

pub(super) fn spawn_item_chips(
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

pub(super) fn spawn_item_chip(
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

pub(super) fn spawn_items_message(
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

pub(super) fn spawn_item_modal(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
) {
    if state.mode == FieldMenuMode::Browse {
        return;
    }
    let title = state
        .pending_id
        .as_deref()
        .and_then(|id| catalog.item(id))
        .map_or("ITEM ACTION", item_name);
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
                            for (index, (label, subtitle)) in [
                                ("Use", "apply this item"),
                                ("Discard", "remove from pouch"),
                                ("Hide", "hide for this session"),
                            ]
                            .into_iter()
                            .enumerate()
                            {
                                spawn_item_modal_row(
                                    modal,
                                    font,
                                    label,
                                    subtitle,
                                    index == state.selected,
                                );
                            }
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
                        _ => {}
                    }
                },
            );
        });
}

pub(super) fn spawn_item_modal_row(
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

#[expect(
    clippy::too_many_arguments,
    reason = "the equipment page coordinates the shared menu root, portraits, and live inventory catalog"
)]
pub(super) fn sync_equipment_page(
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

pub(super) fn spawn_equipment_page(
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

pub(super) fn spawn_equipment_header(
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
            header.spawn((
                Node {
                    width: px(7),
                    height: px(46),
                    margin: UiRect::right(px(3)),
                    ..default()
                },
                BackgroundColor(status_ember()),
            ));
            header.spawn((
                Node {
                    width: px(2),
                    height: px(46),
                    margin: UiRect::right(px(15)),
                    ..default()
                },
                BackgroundColor(status_gold()),
            ));
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

pub(super) fn spawn_equipment_slots_column(
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
                            Color::srgb_u8(116, 108, 90)
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

pub(super) fn spawn_equipment_stat_grid(
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

pub(super) fn spawn_equipment_inventory_column(
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

pub(super) fn spawn_current_equipment_detail(
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

pub(super) fn spawn_equipment_picker(
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

pub(super) fn spawn_equipment_preview(
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

pub(super) fn equipment_preview_summary(
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

pub(super) fn stat_change_color(old: i32, new: i32) -> Color {
    if new > old {
        Color::srgb_u8(120, 220, 120)
    } else if new < old {
        Color::srgb_u8(220, 110, 110)
    } else {
        status_muted()
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "the spells page coordinates the shared menu root, portraits, and live ability catalog"
)]
pub(super) fn sync_spells_page(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    state: Res<FieldMenuState>,
    catalog: Res<FieldMenuCatalog>,
    game: Option<Res<GameState>>,
    menu_roots: Query<Entity, With<FieldMenuRoot>>,
    pages: Query<Entity, With<FieldMenuSpellsPage>>,
) {
    let show_spells = state.open
        && state.screen == FieldMenuScreen::Spells
        && catalog.status() == CatalogStatus::Ready
        && game.is_some();
    if !show_spells {
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
                .expect("spells font path"),
        ),
    );
    let portraits = StatusPortraitAssets::load(&asset_server, &root, &game);
    commands.entity(menu_root).with_children(|parent| {
        spawn_spells_page(parent, &font, &state, &game, &catalog, &portraits);
    });
}

pub(super) fn spawn_spells_page(
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
            FieldMenuSpellsPage,
            Name::new("Spells page"),
        ))
        .with_children(|page| {
            spawn_spells_header(page, font, state, game);
            page.spawn(Node {
                width: percent(100),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Row,
                column_gap: px(STATUS_COLUMN_GAP),
                ..default()
            })
            .with_children(|columns| {
                spawn_party_column(columns, font, state, game, catalog, portraits);
                spawn_spellbook_column(columns, font, state, game, catalog);
            });
            spawn_status_text(
                page,
                if matches!(
                    state.mode,
                    FieldMenuMode::SpellTarget | FieldMenuMode::TeleportPicker
                ) {
                    "UP/DOWN   CHOOSE      ENTER   CONFIRM      ESC   SPELLBOOK      M   CLOSE"
                } else {
                    "LEFT/RIGHT   CHANGE CASTER      UP/DOWN   SELECT SPELL      ENTER   CAST      ESC   BACK"
                },
                font,
                15.0,
                status_muted(),
            );
            if !state.message.is_empty() {
                spawn_items_message(page, font, &state.message);
            }
            spawn_spell_overlay(page, font, state, game, catalog);
        });
}

pub(super) fn spawn_spells_header(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
) {
    let caster = member_at(game, state.member_index);
    parent
        .spawn(Node {
            width: percent(100),
            height: px(64),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|header| {
            header.spawn((
                Node {
                    width: px(7),
                    height: px(46),
                    margin: UiRect::right(px(3)),
                    ..default()
                },
                BackgroundColor(status_ember()),
            ));
            header.spawn((
                Node {
                    width: px(2),
                    height: px(46),
                    margin: UiRect::right(px(15)),
                    ..default()
                },
                BackgroundColor(status_gold()),
            ));
            header
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    ..default()
                })
                .with_children(|title| {
                    spawn_status_text(title, "SPELLS", font, 31.0, status_ink());
                    spawn_status_text(
                        title,
                        "FIELD MAGIC AND FORBIDDEN ARTS",
                        font,
                        14.0,
                        status_muted(),
                    );
                });
            spawn_status_text(
                header,
                caster.map_or_else(
                    || "NO CASTER".to_owned(),
                    |member| {
                        format!(
                            "{}    MP  {}/{}",
                            member.name(),
                            member.mana(),
                            member.max_mana()
                        )
                    },
                ),
                font,
                16.0,
                status_gold(),
            );
        });
}

pub(super) fn spawn_spellbook_column(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
) {
    let Some(member) = member_at(game, state.member_index) else {
        return;
    };
    let abilities = learned_field_abilities(member, game, catalog);
    let selected = selected_spell_index(state, &abilities);
    spawn_status_panel(
        parent,
        Node {
            height: percent(100),
            flex_basis: px(0),
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            padding: UiRect::all(px(16)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(6)),
            overflow: Overflow::clip(),
            ..default()
        },
        "SPELLBOOK",
        font,
        |panel| {
            panel
                .spawn(Node {
                    width: percent(100),
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Row,
                    column_gap: px(16),
                    ..default()
                })
                .with_children(|body| {
                    spawn_spell_list(body, font, member, &abilities, selected);
                    spawn_spell_detail(body, font, member, abilities.get(selected).copied());
                });
        },
    );
}

pub(super) fn spawn_spell_list(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    member: &crate::runtime_member::RuntimeMember,
    abilities: &[&Ability],
    selected_index: usize,
) {
    parent
        .spawn(Node {
            flex_basis: px(0),
            flex_grow: 1.12,
            flex_direction: FlexDirection::Column,
            row_gap: px(7),
            ..default()
        })
        .with_children(|list| {
            if abilities.is_empty() {
                spawn_status_text(
                    list,
                    "No learned field abilities.",
                    font,
                    18.0,
                    status_muted(),
                );
                spawn_status_text(
                    list,
                    "New arts appear here as the caster grows.",
                    font,
                    14.0,
                    Color::srgb_u8(116, 108, 90),
                );
                return;
            }
            let visible_rows = SPELLBOOK_VISIBLE_ROWS;
            let first = selected_index
                .saturating_sub(visible_rows - 1)
                .min(abilities.len().saturating_sub(visible_rows));
            for (index, ability) in abilities.iter().enumerate().skip(first).take(visible_rows) {
                let selected = index == selected_index;
                let affordable = member.mana() >= ability.mp_cost;
                let mut row = list.spawn((
                    Node {
                        width: percent(100),
                        min_height: px(58),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
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
                    SpellbookRow,
                ));
                if selected {
                    row.insert(SelectedSpellbookRow);
                }
                row.with_children(|row| {
                    row.spawn((
                        Node {
                            width: px(6),
                            height: px(34),
                            margin: UiRect::right(px(10)),
                            ..default()
                        },
                        BackgroundColor(spell_accent(ability)),
                    ));
                    row.spawn(Node {
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::Column,
                        ..default()
                    })
                    .with_children(|copy| {
                        spawn_status_text(
                            copy,
                            &ability.name,
                            font,
                            17.0,
                            if affordable {
                                status_ink()
                            } else {
                                status_muted()
                            },
                        );
                        spawn_status_text(
                            copy,
                            spell_kind_label(ability),
                            font,
                            12.0,
                            spell_accent(ability),
                        );
                    });
                    spawn_status_text(
                        row,
                        format!("MP  {}", ability.mp_cost),
                        font,
                        14.0,
                        if affordable {
                            status_teal()
                        } else {
                            status_ember()
                        },
                    );
                });
            }
        });
}

pub(super) fn spawn_spell_detail(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    member: &crate::runtime_member::RuntimeMember,
    ability: Option<&Ability>,
) {
    parent
        .spawn((
            Node {
                flex_basis: px(0),
                flex_grow: 0.88,
                flex_direction: FlexDirection::Column,
                row_gap: px(11),
                padding: UiRect::all(px(14)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(5)),
                ..default()
            },
            BackgroundColor(Color::srgba_u8(10, 10, 14, 150)),
            BorderColor::all(status_border()),
        ))
        .with_children(|detail| {
            spawn_status_text(detail, "DETAIL", font, 13.0, status_gold());
            spawn_section_rule(detail);
            let Some(ability) = ability else {
                spawn_status_text(
                    detail,
                    "Choose a caster with learned field magic.",
                    font,
                    16.0,
                    status_muted(),
                );
                return;
            };
            let affordable = member.mana() >= ability.mp_cost;
            spawn_status_text(detail, &ability.name, font, 25.0, status_gold());
            spawn_status_text(
                detail,
                format!(
                    "{}    MP {}    {}",
                    spell_kind_label(ability),
                    ability.mp_cost,
                    if affordable { "READY" } else { "LOW MP" }
                ),
                font,
                13.0,
                if affordable {
                    status_teal()
                } else {
                    status_ember()
                },
            );
            spawn_section_rule(detail);
            spawn_status_text(detail, &ability.description, font, 16.0, status_ink());
            spawn_section_rule(detail);
            spawn_status_text(
                detail,
                spell_target_label(ability),
                font,
                13.0,
                status_muted(),
            );
            spawn_status_text(
                detail,
                "ENTER  -  CAST",
                font,
                14.0,
                if affordable {
                    status_teal()
                } else {
                    status_muted()
                },
            );
        });
}

pub(super) fn spawn_spell_overlay(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
) {
    if !matches!(
        state.mode,
        FieldMenuMode::SpellTarget | FieldMenuMode::TeleportPicker
    ) {
        return;
    }
    let title = state
        .pending_id
        .as_deref()
        .and_then(|id| {
            member_at(game, state.member_index)
                .and_then(|member| ability_by_id(game, catalog, member.id(), id))
        })
        .map_or("CAST SPELL", |ability| ability.name.as_str());
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
            BackgroundColor(Color::srgba_u8(2, 2, 6, 160)),
        ))
        .with_children(|overlay| {
            spawn_status_panel(
                overlay,
                Node {
                    width: px(440),
                    min_height: px(210),
                    max_height: px(520),
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
                    modal.spawn(SpellTargetOverlay);
                    match state.mode {
                        FieldMenuMode::SpellTarget => {
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
                        FieldMenuMode::TeleportPicker => {
                            spawn_status_text(
                                modal,
                                "CHOOSE A DESTINATION",
                                font,
                                13.0,
                                status_muted(),
                            );
                            let destinations = catalog.eligible_warp_destinations(game.map());
                            if destinations.is_empty() {
                                spawn_status_text(
                                    modal,
                                    "No eligible visited destinations.",
                                    font,
                                    16.0,
                                    status_muted(),
                                );
                            }
                            for (index, destination) in destinations.into_iter().enumerate() {
                                spawn_item_modal_row(
                                    modal,
                                    font,
                                    &destination.name,
                                    "standard map transition",
                                    index == state.selected,
                                );
                            }
                        }
                        _ => {}
                    }
                },
            );
        });
}

pub(super) fn selected_spell_index(state: &FieldMenuState, abilities: &[&Ability]) -> usize {
    if abilities.is_empty() {
        return 0;
    }
    if state.mode == FieldMenuMode::Browse {
        return state.selected.min(abilities.len() - 1);
    }
    state
        .pending_id
        .as_deref()
        .and_then(|pending| abilities.iter().position(|ability| ability.id == pending))
        .unwrap_or(0)
}

pub(super) fn spell_kind_label(ability: &Ability) -> &'static str {
    match &ability.kind {
        AbilityKind::Physical(_) => "PHYSICAL ART",
        AbilityKind::Spell(_) => "ELEMENTAL SPELL",
        AbilityKind::Heal(_) => "HEALING RITE",
        AbilityKind::Buff(_) => "WARDING SIGIL",
        AbilityKind::Debuff(_) => "HEX",
        AbilityKind::Utility(UtilityAbility::Warp { .. }) => "TRAVEL CHARM",
        AbilityKind::Utility(_) => "UTILITY ART",
    }
}

pub(super) fn spell_target_label(ability: &Ability) -> &'static str {
    match &ability.kind {
        AbilityKind::Physical(value) => ability_target_label(value.target),
        AbilityKind::Spell(value) => ability_target_label(value.target),
        AbilityKind::Heal(value) => ability_target_label(value.target),
        AbilityKind::Utility(UtilityAbility::RemoveStatus { target, .. })
        | AbilityKind::Utility(UtilityAbility::Steal { target, .. })
        | AbilityKind::Utility(UtilityAbility::Warp { target, .. }) => {
            ability_target_label(*target)
        }
        AbilityKind::Buff(_) | AbilityKind::Debuff(_) => "TARGET  SPECIAL",
    }
}

pub(super) fn ability_target_label(target: crate::scenario_class::AbilityTarget) -> &'static str {
    use crate::scenario_class::AbilityTarget;
    match target {
        AbilityTarget::SingleEnemy => "TARGET  ONE ENEMY",
        AbilityTarget::AllEnemies => "TARGET  ALL ENEMIES",
        AbilityTarget::GroupEnemies => "TARGET  ENEMY GROUP",
        AbilityTarget::SingleAlly => "TARGET  ONE ALLY",
        AbilityTarget::SelfTarget => "TARGET  SELF",
        AbilityTarget::AllAllies => "TARGET  ALL ALLIES",
    }
}

pub(super) fn spell_accent(ability: &Ability) -> Color {
    match &ability.kind {
        AbilityKind::Heal(_) => status_teal(),
        AbilityKind::Utility(UtilityAbility::Warp { .. }) => status_violet(),
        AbilityKind::Utility(_) => Color::srgb_u8(91, 143, 183),
        AbilityKind::Spell(_) => Color::srgb_u8(90, 146, 212),
        AbilityKind::Buff(_) => Color::srgb_u8(120, 190, 130),
        AbilityKind::Debuff(_) => status_ember(),
        AbilityKind::Physical(_) => status_gold(),
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "the save page coordinates the shared menu root, live session, and discovered slots"
)]
pub(super) fn sync_save_page(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    state: Res<FieldMenuState>,
    saves: Res<SaveSlotCatalog>,
    game: Option<Res<GameState>>,
    menu_roots: Query<Entity, With<FieldMenuRoot>>,
    pages: Query<Entity, With<FieldMenuSavePage>>,
) {
    let show_save = state.open && state.screen == FieldMenuScreen::Save && game.is_some();
    if !show_save {
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
    let rebuild = pages.is_empty() || state.is_changed() || saves.is_changed() || game.is_changed();
    if !rebuild {
        return;
    }
    for entity in &pages {
        commands.entity(entity).despawn();
    }

    let font = asset_server.load(
        root.resolve(
            &ScenarioRelativePath::try_from("assets/fonts/Philosopher-Regular.ttf")
                .expect("save font path"),
        ),
    );
    commands.entity(menu_root).with_children(|parent| {
        spawn_save_page(parent, &font, &state, &game, &saves);
    });
}

pub(super) fn spawn_save_page(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    saves: &SaveSlotCatalog,
) {
    parent
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            FieldMenuSavePage,
            Name::new("Save page"),
        ))
        .with_children(|page| {
            page.spawn((
                Node {
                    width: px(760),
                    height: percent(100),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(6),
                    padding: UiRect::all(px(14)),
                    border: UiRect::all(px(2)),
                    border_radius: BorderRadius::all(px(8)),
                    ..default()
                },
                BackgroundColor(Color::srgba_u8(22, 22, 28, 240)),
                BorderColor::all(status_border_active()),
            ))
            .with_children(|modal| {
                spawn_save_header(modal, font, state, game);
                spawn_section_rule(modal);
                if saves.slots().is_empty() {
                    modal
                        .spawn(Node {
                            flex_grow: 1.0,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|empty| {
                            spawn_status_text(
                                empty,
                                "Discovering native save slots...",
                                font,
                                18.0,
                                status_muted(),
                            );
                        });
                } else {
                    if let Some(autosave) = saves.slots().first() {
                        spawn_field_save_slot_row(modal, font, autosave, false, true);
                    }
                    spawn_section_rule(modal);
                    let page_start = save_page_start(state.selected);
                    for slot in saves
                        .slots()
                        .iter()
                        .skip(page_start)
                        .take(SAVE_VISIBLE_ROWS)
                    {
                        spawn_field_save_slot_row(
                            modal,
                            font,
                            slot,
                            slot.index == state.selected,
                            false,
                        );
                    }
                }
                modal.spawn(Node {
                    flex_grow: 1.0,
                    ..default()
                });
                spawn_status_text(
                    modal,
                    if state.mode == FieldMenuMode::SaveConfirm {
                        "ENTER / Y   OVERWRITE      ESC / N   CANCEL"
                    } else {
                        "UP/DOWN   SELECT SLOT      ENTER   SAVE      ESC   BACK      M   CLOSE"
                    },
                    font,
                    14.0,
                    status_muted(),
                );
                if !state.message.is_empty() {
                    spawn_save_inline_message(modal, font, &state.message);
                }
            });
            spawn_save_overwrite_modal(page, font, state, saves);
        });
}

pub(super) fn spawn_save_header(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
) {
    let page = (state.selected.saturating_sub(FIRST_PLAYER_SLOT) / SAVE_VISIBLE_ROWS) + 1;
    let page_count = (LAST_PLAYER_SLOT - FIRST_PLAYER_SLOT + 1).div_ceil(SAVE_VISIBLE_ROWS);
    parent
        .spawn(Node {
            width: percent(100),
            min_height: px(52),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|header| {
            header.spawn((
                Node {
                    width: px(7),
                    height: px(42),
                    margin: UiRect::right(px(3)),
                    ..default()
                },
                BackgroundColor(status_ember()),
            ));
            header.spawn((
                Node {
                    width: px(2),
                    height: px(42),
                    margin: UiRect::right(px(14)),
                    ..default()
                },
                BackgroundColor(status_gold()),
            ));
            header
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    ..default()
                })
                .with_children(|title| {
                    spawn_status_text(title, "SAVE GAME", font, 29.0, status_gold());
                    spawn_status_text(
                        title,
                        game.map()
                            .current()
                            .map_or("CHOOSE A RECORD".to_owned(), |map| {
                                format!("{}  /  CHOOSE A RECORD", map.as_str().to_uppercase())
                            }),
                        font,
                        13.0,
                        status_muted(),
                    );
                });
            spawn_status_text(
                header,
                format!("PAGE {page:02} / {page_count:02}"),
                font,
                13.0,
                status_muted(),
            );
        });
}

pub(super) fn spawn_field_save_slot_row(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    slot: &SaveSlot,
    selected: bool,
    pinned: bool,
) {
    let mut row = parent.spawn((
        Node {
            width: percent(100),
            min_height: px(54),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            padding: UiRect::axes(px(12), px(7)),
            border: UiRect::all(px(if selected { 2 } else { 1 })),
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        BackgroundColor(if selected {
            Color::srgba_u8(72, 49, 25, 224)
        } else if pinned {
            Color::srgba_u8(20, 20, 25, 170)
        } else {
            Color::srgba_u8(10, 10, 14, 148)
        }),
        BorderColor::all(if selected {
            status_border_active()
        } else {
            Color::srgba_u8(126, 98, 55, if pinned { 65 } else { 95 })
        }),
        FieldSaveSlotRow,
    ));
    if selected {
        row.insert(SelectedFieldSaveSlotRow);
    }
    row.with_children(|row| {
        row.spawn(Node {
            width: px(102),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|label| {
            spawn_status_text(
                label,
                slot.label().to_uppercase(),
                font,
                15.0,
                if pinned { status_muted() } else { status_ink() },
            );
            if pinned {
                spawn_status_text(label, "PINNED", font, 10.0, Color::srgb_u8(116, 108, 90));
            }
        });
        row.spawn(Node {
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|content| {
            spawn_save_slot_content(content, font, slot);
        });
        spawn_status_text(
            row,
            save_slot_state_label(slot),
            font,
            11.0,
            save_slot_state_color(slot),
        );
    });
}

pub(super) fn spawn_save_slot_content(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    slot: &SaveSlot,
) {
    match (&slot.state, &slot.metadata) {
        (SaveSlotState::Empty, _) => {
            spawn_status_text(
                parent,
                "—  EMPTY  —",
                font,
                17.0,
                Color::srgb_u8(116, 108, 90),
            );
        }
        (SaveSlotState::Valid, Some(metadata)) => {
            spawn_status_text(
                parent,
                format!("{}    ({})", metadata.location, metadata.protagonist_name),
                font,
                17.0,
                status_ink(),
            );
            spawn_status_text(
                parent,
                format!(
                    "LV {}      PLAYTIME {}",
                    metadata.protagonist_level,
                    crate::playtime::Playtime::format(metadata.playtime_seconds)
                ),
                font,
                12.0,
                status_muted(),
            );
        }
        (SaveSlotState::Corrupt(reason), _) => {
            spawn_status_text(parent, "CORRUPT SAVE", font, 16.0, status_ember());
            spawn_status_text(parent, reason, font, 11.0, status_muted());
        }
        (SaveSlotState::Incompatible(reason), _) => {
            spawn_status_text(parent, "INCOMPATIBLE SAVE", font, 16.0, status_violet());
            spawn_status_text(parent, reason, font, 11.0, status_muted());
        }
        _ => {
            spawn_status_text(parent, "INVALID METADATA", font, 16.0, status_ember());
        }
    }
}

pub(super) fn spawn_save_inline_message(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    message: &str,
) {
    parent
        .spawn((
            Node {
                width: percent(100),
                justify_content: JustifyContent::Center,
                padding: UiRect::axes(px(10), px(6)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(Color::srgba_u8(10, 10, 14, 180)),
            BorderColor::all(status_border()),
        ))
        .with_children(|message_box| {
            spawn_status_text(message_box, message, font, 13.0, status_ink());
        });
}

pub(super) fn spawn_save_overwrite_modal(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    saves: &SaveSlotCatalog,
) {
    if state.mode != FieldMenuMode::SaveConfirm {
        return;
    }
    let slot = saves.slots().get(state.selected);
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
            BackgroundColor(Color::srgba_u8(2, 2, 6, 170)),
        ))
        .with_children(|overlay| {
            spawn_status_panel(
                overlay,
                Node {
                    width: px(460),
                    min_height: px(160),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(12),
                    padding: UiRect::all(px(18)),
                    border: UiRect::all(px(2)),
                    border_radius: BorderRadius::all(px(7)),
                    ..default()
                },
                "OVERWRITE SAVE?",
                font,
                |modal| {
                    modal.spawn(SaveOverwriteModal);
                    spawn_status_text(
                        modal,
                        slot.map_or_else(|| slot_label(state.selected), SaveSlot::label),
                        font,
                        23.0,
                        status_gold(),
                    );
                    spawn_status_text(
                        modal,
                        "Existing progress in this slot will be replaced.",
                        font,
                        15.0,
                        status_ink(),
                    );
                    spawn_status_text(
                        modal,
                        "ENTER / Y   CONFIRM      ESC / N   CANCEL",
                        font,
                        13.0,
                        status_muted(),
                    );
                },
            );
        });
}

pub(super) fn save_page_start(selected: usize) -> usize {
    ((selected.saturating_sub(FIRST_PLAYER_SLOT)) / SAVE_VISIBLE_ROWS) * SAVE_VISIBLE_ROWS
        + FIRST_PLAYER_SLOT
}

pub(super) fn save_slot_state_label(slot: &SaveSlot) -> &'static str {
    match slot.state {
        SaveSlotState::Empty => "OPEN",
        SaveSlotState::Valid => "SAVED",
        SaveSlotState::Corrupt(_) => "CORRUPT",
        SaveSlotState::Incompatible(_) => "VERSION",
    }
}

pub(super) fn save_slot_state_color(slot: &SaveSlot) -> Color {
    match slot.state {
        SaveSlotState::Empty => status_muted(),
        SaveSlotState::Valid => status_teal(),
        SaveSlotState::Corrupt(_) => status_ember(),
        SaveSlotState::Incompatible(_) => status_violet(),
    }
}

pub(super) fn spawn_status_page(
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
            FieldMenuStatusPage,
            Name::new("Status page"),
        ))
        .with_children(|page| {
            spawn_status_header(page, font, state.member_index, game.party().len());

            page.spawn(Node {
                width: percent(100),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Row,
                column_gap: px(STATUS_COLUMN_GAP),
                ..default()
            })
            .with_children(|columns| {
                spawn_party_column(columns, font, state, game, catalog, portraits);
                spawn_member_column(columns, font, state, game, catalog, portraits);
                spawn_profile_column(columns, font, state, game, catalog);
            });

            spawn_status_text(
                page,
                if state.status_page == StatusPage::Roster {
                    "UP/DOWN   SELECT MEMBER      ENTER   STATS      ESC   BACK"
                } else {
                    "UP/DOWN   SELECT ACTION      ESC   PORTRAIT      M   CLOSE"
                },
                font,
                15.0,
                status_muted(),
            );
        });
}

pub(super) fn spawn_status_header(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    member_index: usize,
    party_len: usize,
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
            header.spawn((
                Node {
                    width: px(7),
                    height: px(46),
                    margin: UiRect::right(px(3)),
                    ..default()
                },
                BackgroundColor(status_ember()),
            ));
            header.spawn((
                Node {
                    width: px(2),
                    height: px(46),
                    margin: UiRect::right(px(15)),
                    ..default()
                },
                BackgroundColor(status_gold()),
            ));
            header
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    ..default()
                })
                .with_children(|title| {
                    spawn_status_text(title, "STATUS", font, 31.0, status_ink());
                    spawn_status_text(title, "PARTY ROSTER AND GROWTH", font, 14.0, status_muted());
                });
            spawn_status_text(
                header,
                format!(
                    "{:02}  /  {:02}",
                    member_index.saturating_add(1).min(party_len.max(1)),
                    party_len
                ),
                font,
                16.0,
                status_gold(),
            );
        });
}

pub(super) fn spawn_party_column(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
    portraits: &StatusPortraitAssets,
) {
    spawn_status_panel(
        parent,
        Node {
            width: px(STATUS_PARTY_WIDTH),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(10),
            padding: UiRect::all(px(14)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        "PARTY",
        font,
        |panel| {
            for (index, member) in game.party().members().enumerate() {
                let selected = index == state.member_index;
                let mut card = panel.spawn((
                    Node {
                        width: percent(100),
                        min_height: px(84),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: px(12),
                        padding: UiRect::all(px(10)),
                        border: UiRect::all(px(if selected { 2 } else { 1 })),
                        border_radius: BorderRadius::all(px(5)),
                        ..default()
                    },
                    BackgroundColor(if selected {
                        Color::srgba_u8(79, 51, 38, 214)
                    } else {
                        Color::srgba_u8(30, 30, 38, 164)
                    }),
                    BorderColor::all(if selected {
                        status_border_active()
                    } else {
                        status_border()
                    }),
                    StatusMemberCard,
                    Name::new(format!("Party card: {}", member.name())),
                ));
                if selected {
                    card.insert(SelectedStatusMemberCard);
                }
                card.with_children(|card| {
                    spawn_portrait_frame(
                        card,
                        font,
                        member.name(),
                        portraits.profile(member.id()),
                        56.0,
                        56.0,
                        selected,
                        "Party portrait",
                    );
                    card.spawn(Node {
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::Column,
                        row_gap: px(2),
                        ..default()
                    })
                    .with_children(|summary| {
                        spawn_status_text(
                            summary,
                            format!("{}     LV {}", member.name(), member.level()),
                            font,
                            18.0,
                            status_ink(),
                        );
                        spawn_status_text(
                            summary,
                            class_name(member.class_id(), catalog).to_uppercase(),
                            font,
                            13.0,
                            status_gold(),
                        );
                        spawn_status_text(
                            summary,
                            format!(
                                "HP {:>3}/{:<3}     MP {:>3}/{:<3}",
                                member.health(),
                                member.max_health(),
                                member.mana(),
                                member.max_mana()
                            ),
                            font,
                            13.0,
                            status_muted(),
                        );
                    });
                });
            }
        },
    );
}

pub(super) fn spawn_member_column(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
    portraits: &StatusPortraitAssets,
) {
    if state.status_page == StatusPage::Roster {
        spawn_full_portrait_column(parent, font, state, game, portraits);
    } else {
        spawn_member_details_column(parent, font, state, game, catalog);
    }
}

pub(super) fn spawn_full_portrait_column(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    portraits: &StatusPortraitAssets,
) {
    let Some(member) = member_at(game, state.member_index) else {
        return;
    };
    parent
        .spawn((
            Node {
                width: px(STATUS_DETAIL_WIDTH),
                height: percent(100),
                flex_shrink: 0.0,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(px(5)),
                border: UiRect::all(px(2)),
                border_radius: BorderRadius::all(px(6)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgba_u8(8, 8, 12, 210)),
            BorderColor::all(status_border_active()),
            Name::new("Full status portrait"),
        ))
        .with_children(|portrait_panel| {
            if let Some(portrait) = portraits.large(member.id()) {
                portrait_panel.spawn((
                    ImageNode::new(portrait).with_mode(NodeImageMode::Stretch),
                    Node {
                        height: percent(100),
                        aspect_ratio: Some(418.0 / 570.0),
                        max_width: percent(100),
                        ..default()
                    },
                ));
            } else {
                spawn_status_text(
                    portrait_panel,
                    member_emblem(member.name()),
                    font,
                    42.0,
                    status_gold(),
                );
            }
        });
}

pub(super) fn spawn_member_details_column(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
) {
    let Some(member) = member_at(game, state.member_index) else {
        return;
    };
    let stats = derived_stats(member, catalog);
    let equipment = EquipmentSlot::ALL
        .into_iter()
        .map(|slot| {
            let name = member
                .equipment()
                .get(slot)
                .and_then(|id| catalog.item(id))
                .map_or("—", item_name);
            format!("{:<5}  {name}", status_slot_label(slot))
        })
        .collect::<Vec<_>>()
        .join("\n");
    spawn_status_panel(
        parent,
        Node {
            width: px(STATUS_DETAIL_WIDTH),
            height: percent(100),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            padding: UiRect::all(px(16)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        member.name(),
        font,
        |panel| {
            panel
                .spawn(Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                })
                .with_children(|level| {
                    spawn_status_text(
                        level,
                        format!("LV {}", member.level()),
                        font,
                        16.0,
                        status_ink(),
                    );
                    spawn_status_text(
                        level,
                        format!("EXP {} / {}", member.experience(), member.experience_next()),
                        font,
                        14.0,
                        status_muted(),
                    );
                });

            spawn_meter(
                panel,
                "EXP",
                member.experience(),
                member.experience_next(),
                font,
                status_violet(),
            );
            spawn_meter(
                panel,
                "HP",
                member.health(),
                member.max_health(),
                font,
                if member.health().saturating_mul(4) < member.max_health() {
                    status_ember()
                } else {
                    Color::srgb_u8(132, 196, 111)
                },
            );
            spawn_meter(
                panel,
                "MP",
                member.mana(),
                member.max_mana(),
                font,
                status_teal(),
            );

            panel
                .spawn(Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    row_gap: px(4),
                    ..default()
                })
                .with_children(|grid| {
                    for (label, value) in [
                        ("STR", stats.strength),
                        ("DEX", stats.dexterity),
                        ("CON", stats.constitution),
                        ("INT", stats.intelligence),
                    ] {
                        grid.spawn(Node {
                            width: percent(50),
                            flex_direction: FlexDirection::Row,
                            column_gap: px(14),
                            ..default()
                        })
                        .with_children(|attribute| {
                            spawn_status_text(attribute, label, font, 14.0, status_muted());
                            spawn_status_text(
                                attribute,
                                format!("{value:03}"),
                                font,
                                16.0,
                                status_ink(),
                            );
                        });
                    }
                });

            spawn_status_text(panel, equipment, font, 14.0, status_ink());

            panel.spawn(Node {
                flex_grow: 1.0,
                ..default()
            });
            spawn_section_rule(panel);
            for (index, label) in STATUS_CATEGORIES.into_iter().enumerate() {
                let selected = index == state.selected;
                panel
                    .spawn((
                        Node {
                            width: percent(100),
                            height: px(52),
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: px(12),
                            padding: UiRect::axes(px(10), px(7)),
                            border: UiRect::all(px(if selected { 2 } else { 1 })),
                            border_radius: BorderRadius::all(px(5)),
                            ..default()
                        },
                        BackgroundColor(if selected {
                            Color::srgba_u8(79, 51, 38, 214)
                        } else {
                            Color::srgba_u8(30, 30, 38, 164)
                        }),
                        BorderColor::all(if selected {
                            status_border_active()
                        } else {
                            status_border()
                        }),
                    ))
                    .with_children(|category| {
                        category
                            .spawn((
                                Node {
                                    width: px(34),
                                    height: px(34),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(px(1)),
                                    border_radius: BorderRadius::all(percent(50)),
                                    ..default()
                                },
                                BackgroundColor(if index == 0 {
                                    status_violet()
                                } else {
                                    Color::srgb_u8(100, 180, 95)
                                }),
                                BorderColor::all(status_ink()),
                            ))
                            .with_children(|icon| {
                                spawn_status_text(icon, "C", font, 15.0, Color::BLACK);
                            });
                        spawn_status_text(category, label, font, 18.0, status_ink());
                        category.spawn(Node {
                            flex_grow: 1.0,
                            ..default()
                        });
                        if index == 1 {
                            spawn_status_text(
                                category,
                                format!("{:?}", member.row()),
                                font,
                                14.0,
                                status_muted(),
                            );
                        }
                    });
            }
        },
    );
}

pub(super) fn spawn_profile_column(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
) {
    let Some(member) = member_at(game, state.member_index) else {
        return;
    };
    let description = catalog
        .class(member.class_id())
        .map_or("No class profile is available.", |class| {
            class.description.as_str()
        });
    let equipment = EquipmentSlot::ALL
        .into_iter()
        .map(|slot| {
            let name = member
                .equipment()
                .get(slot)
                .and_then(|id| catalog.item(id))
                .map_or("—", item_name);
            format!("{:<10}  {name}", slot.as_str().to_uppercase())
        })
        .collect::<Vec<_>>()
        .join("\n");
    let abilities = learned_field_abilities(member, game, catalog)
        .into_iter()
        .map(|ability| format!("•  {}", ability.name))
        .collect::<Vec<_>>();
    let statuses = member
        .status_effects()
        .map(|effect| format!("{effect:?}"))
        .collect::<Vec<_>>();

    spawn_status_panel(
        parent,
        Node {
            height: percent(100),
            flex_basis: px(0),
            flex_grow: 0.92,
            flex_direction: FlexDirection::Column,
            row_gap: px(10),
            padding: UiRect::all(px(16)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        "PROFILE & LOADOUT",
        font,
        |panel| {
            spawn_status_text(panel, "PROFILE", font, 14.0, status_gold());
            spawn_status_text(panel, description, font, 16.0, status_ink());

            spawn_section_rule(panel);
            spawn_status_text(panel, "EQUIPMENT", font, 14.0, status_gold());
            panel
                .spawn((
                    Node {
                        width: percent(100),
                        padding: UiRect::all(px(10)),
                        border: UiRect::all(px(1)),
                        border_radius: BorderRadius::all(px(4)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba_u8(10, 10, 14, 150)),
                    BorderColor::all(status_border()),
                ))
                .with_children(|box_node| {
                    spawn_status_text(box_node, &equipment, font, 14.0, status_ink());
                });

            spawn_section_rule(panel);
            spawn_status_text(panel, "FIELD ARTS", font, 14.0, status_gold());
            spawn_status_text(
                panel,
                if abilities.is_empty() {
                    "None learned".to_owned()
                } else {
                    abilities.join("\n")
                },
                font,
                15.0,
                status_teal(),
            );
            spawn_status_text(
                panel,
                format!(
                    "STATUS    {}",
                    if statuses.is_empty() {
                        "CLEAR".to_owned()
                    } else {
                        statuses.join(", ").to_uppercase()
                    }
                ),
                font,
                14.0,
                if statuses.is_empty() {
                    Color::srgb_u8(132, 196, 111)
                } else {
                    status_ember()
                },
            );
        },
    );
}

#[expect(
    clippy::too_many_arguments,
    reason = "the shared portrait frame keeps profile and status art styling identical"
)]
pub(super) fn spawn_portrait_frame(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    member_name: &str,
    portrait: Option<Handle<Image>>,
    width: f32,
    height: f32,
    selected: bool,
    label: &'static str,
) {
    parent
        .spawn((
            Node {
                width: px(width),
                height: px(height),
                flex_shrink: 0.0,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(px(if selected { 2 } else { 1 })),
                border_radius: BorderRadius::all(px(5)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgba_u8(10, 10, 14, 188)),
            BorderColor::all(if selected {
                status_border_active()
            } else {
                status_border()
            }),
            Name::new(label),
        ))
        .with_children(|frame| {
            spawn_status_text(
                frame,
                member_emblem(member_name),
                font,
                if height > 100.0 { 34.0 } else { 18.0 },
                if selected {
                    status_gold()
                } else {
                    status_ink()
                },
            );
            if let Some(portrait) = portrait {
                frame.spawn((
                    ImageNode::new(portrait).with_mode(NodeImageMode::Stretch),
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(0),
                        top: px(0),
                        width: percent(100),
                        height: percent(100),
                        ..default()
                    },
                ));
            }
        });
}

pub(super) fn load_status_image(
    asset_server: &AssetServer,
    root: &ScenarioRoot,
    relative: &str,
) -> Option<Handle<Image>> {
    let relative = ScenarioRelativePath::try_from(relative).ok()?;
    Some(asset_server.load(root.resolve(&relative)))
}

pub(super) fn profile_portrait_path(member_id: &str) -> String {
    format!("assets/images/{member_id}_profile.png")
}

pub(super) fn large_status_portrait_path(member_id: &str) -> String {
    format!("assets/images/party_portraits_large/{member_id}_status_portrait.webp")
}

pub(super) fn spawn_status_panel(
    parent: &mut ChildSpawnerCommands<'_>,
    node: Node,
    title: &str,
    font: &Handle<Font>,
    content: impl FnOnce(&mut ChildSpawnerCommands<'_>),
) {
    parent
        .spawn((
            node,
            BackgroundColor(Color::srgba_u8(22, 22, 28, 228)),
            BorderColor::all(status_border()),
        ))
        .with_children(|panel| {
            spawn_status_text(panel, title, font, 14.0, status_gold());
            panel.spawn((
                Node {
                    width: percent(100),
                    height: px(1),
                    margin: UiRect::bottom(px(2)),
                    ..default()
                },
                BackgroundColor(Color::srgba_u8(126, 98, 55, 150)),
            ));
            content(panel);
        });
}

pub(super) fn spawn_meter(
    parent: &mut ChildSpawnerCommands<'_>,
    label: &str,
    value: u32,
    maximum: u32,
    font: &Handle<Font>,
    fill: Color,
) {
    parent
        .spawn(Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(3),
            ..default()
        })
        .with_children(|meter| {
            meter
                .spawn(Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                })
                .with_children(|label_row| {
                    spawn_status_text(label_row, label, font, 13.0, status_muted());
                    spawn_status_text(
                        label_row,
                        format!("{value} / {maximum}"),
                        font,
                        13.0,
                        status_ink(),
                    );
                });
            meter
                .spawn((
                    Node {
                        width: percent(100),
                        height: px(8),
                        border_radius: BorderRadius::all(px(4)),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(Color::srgb_u8(17, 17, 22)),
                ))
                .with_children(|track| {
                    track.spawn((
                        Node {
                            width: percent(meter_percent(value, maximum)),
                            height: percent(100),
                            border_radius: BorderRadius::all(px(4)),
                            ..default()
                        },
                        BackgroundColor(fill),
                    ));
                });
        });
}

pub(super) fn spawn_section_rule(parent: &mut ChildSpawnerCommands<'_>) {
    parent.spawn((
        Node {
            width: percent(100),
            height: px(1),
            margin: UiRect::axes(px(0), px(2)),
            ..default()
        },
        BackgroundColor(Color::srgba_u8(126, 98, 55, 100)),
    ));
}

pub(super) fn spawn_status_text(
    parent: &mut ChildSpawnerCommands<'_>,
    text: impl Into<String>,
    font: &Handle<Font>,
    size: f32,
    color: Color,
) {
    parent.spawn((
        Text::new(text),
        TextFont {
            font: font.clone().into(),
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
        TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
    ));
}

pub(super) fn meter_percent(value: u32, maximum: u32) -> f32 {
    if maximum == 0 {
        0.0
    } else {
        (value as f32 / maximum as f32 * 100.0).clamp(0.0, 100.0)
    }
}

pub(super) fn selected_inventory_index(state: &FieldMenuState, ids: &[&str]) -> usize {
    if ids.is_empty() {
        return 0;
    }
    if state.mode == FieldMenuMode::Browse {
        return state.selected.min(ids.len() - 1);
    }
    state
        .pending_id
        .as_deref()
        .and_then(|pending| ids.iter().position(|id| *id == pending))
        .unwrap_or(0)
}

pub(super) fn selected_inventory_id<'a>(
    state: &'a FieldMenuState,
    ids: &'a [&'a str],
    list_index: usize,
) -> Option<&'a str> {
    if state.mode == FieldMenuMode::Browse {
        ids.get(list_index).copied()
    } else {
        state.pending_id.as_deref()
    }
}

pub(super) fn item_kind_label(item: &ItemDefinition) -> &'static str {
    match item {
        ItemDefinition::Consumable(_) => "CONSUMABLE",
        ItemDefinition::Material(_) => "MATERIAL",
        ItemDefinition::Key(_) => "KEY ITEM",
        ItemDefinition::MagicCore(_) => "MAGIC CORE",
        ItemDefinition::Weapon(_) => "WEAPON",
        ItemDefinition::Shield(_) => "SHIELD",
        ItemDefinition::Helmet(_) => "HELMET",
        ItemDefinition::Body(_) => "BODY ARMOR",
        ItemDefinition::Accessory(_) => "ACCESSORY",
    }
}

pub(super) fn item_kind_abbreviation(item: &ItemDefinition) -> &'static str {
    match item {
        ItemDefinition::Consumable(_) => "USE",
        ItemDefinition::Material(_) => "MAT",
        ItemDefinition::Key(_) => "KEY",
        ItemDefinition::MagicCore(_) => "CORE",
        ItemDefinition::Weapon(_) => "WPN",
        ItemDefinition::Shield(_) => "SHLD",
        ItemDefinition::Helmet(_) => "HELM",
        ItemDefinition::Body(_) => "BODY",
        ItemDefinition::Accessory(_) => "ACC",
    }
}

pub(super) fn item_accent(item: &ItemDefinition) -> Color {
    match item {
        ItemDefinition::Consumable(_) => status_teal(),
        ItemDefinition::Material(_) => Color::srgb_u8(157, 139, 101),
        ItemDefinition::Key(_) => status_gold(),
        ItemDefinition::MagicCore(_) => status_violet(),
        ItemDefinition::Weapon(_) => status_ember(),
        ItemDefinition::Shield(_) => Color::srgb_u8(91, 143, 183),
        ItemDefinition::Helmet(_) => Color::srgb_u8(126, 151, 174),
        ItemDefinition::Body(_) => Color::srgb_u8(112, 126, 148),
        ItemDefinition::Accessory(_) => Color::srgb_u8(196, 116, 168),
    }
}

pub(super) fn member_emblem(name: &str) -> String {
    let words = name.split_whitespace().collect::<Vec<_>>();
    let letters = if words.len() > 1 {
        words
            .iter()
            .filter_map(|word| word.chars().next())
            .take(2)
            .collect::<String>()
    } else {
        name.chars()
            .filter(|character| !character.is_whitespace())
            .take(2)
            .collect()
    };
    let emblem = letters.to_uppercase();
    if emblem.is_empty() {
        "?".to_owned()
    } else {
        emblem
    }
}

pub(super) fn class_name<'a>(class_id: &'a str, catalog: &'a FieldMenuCatalog) -> &'a str {
    catalog
        .class(class_id)
        .map_or(class_id, |class| class.name.as_str())
}

pub(super) fn status_slot_label(slot: EquipmentSlot) -> &'static str {
    match slot {
        EquipmentSlot::Weapon => "WPN",
        EquipmentSlot::Shield => "SHLD",
        EquipmentSlot::Helmet => "HELM",
        EquipmentSlot::Body => "BODY",
        EquipmentSlot::Accessory => "ACC",
    }
}

pub(super) fn status_ink() -> Color {
    Color::srgb_u8(242, 236, 211)
}

pub(super) fn status_muted() -> Color {
    Color::srgb_u8(184, 174, 142)
}

pub(super) fn status_gold() -> Color {
    Color::srgb_u8(231, 184, 86)
}

pub(super) fn status_ember() -> Color {
    Color::srgb_u8(203, 82, 47)
}

pub(super) fn status_teal() -> Color {
    Color::srgb_u8(67, 166, 160)
}

pub(super) fn status_violet() -> Color {
    Color::srgb_u8(126, 101, 204)
}

pub(super) fn status_border() -> Color {
    Color::srgb_u8(126, 98, 55)
}

pub(super) fn status_border_active() -> Color {
    Color::srgb_u8(235, 190, 89)
}

pub(super) fn screen_title(state: &FieldMenuState) -> &'static str {
    match state.screen {
        FieldMenuScreen::Main => "Field Menu",
        FieldMenuScreen::Status => "Status",
        FieldMenuScreen::Items => "Items",
        FieldMenuScreen::Equipment => "Equipment",
        FieldMenuScreen::Spells => "Spells",
        FieldMenuScreen::Save => "Save Game",
    }
}

pub(super) fn render_body(
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
    saves: &SaveSlotCatalog,
) -> String {
    if catalog.status() == CatalogStatus::Loading {
        return "Loading class and item catalogs...".to_owned();
    }
    if catalog.status() == CatalogStatus::Failed {
        return format!(
            "Catalog load failed:\n{}",
            catalog.failure().unwrap_or("unknown failure")
        );
    }
    let mut text = match state.screen {
        FieldMenuScreen::Main => render_main(state, game),
        FieldMenuScreen::Status => render_status(state, game, catalog),
        FieldMenuScreen::Items => render_items(state, game, catalog),
        FieldMenuScreen::Equipment => render_equipment(state, game, catalog),
        FieldMenuScreen::Spells => render_spells(state, game, catalog),
        FieldMenuScreen::Save => render_save(state, saves),
    };
    if !state.message.is_empty() {
        text.push_str("\n\n");
        text.push_str(&state.message);
    }
    text
}

pub(super) fn render_main(state: &FieldMenuState, game: &GameState) -> String {
    let commands = MAIN_COMMANDS
        .iter()
        .enumerate()
        .map(|(index, command)| {
            let cursor = if index == state.selected { ">" } else { " " };
            format!("{cursor} {}", command.label)
        })
        .collect::<Vec<_>>()
        .join("\n");
    let party = game
        .party()
        .members()
        .map(|member| {
            format!(
                "{:<12} Lv {:>2}  HP {:>3}/{:<3}  MP {:>3}/{:<3}  {:?}",
                member.name(),
                member.level(),
                member.health(),
                member.max_health(),
                member.mana(),
                member.max_mana(),
                member.row()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "{commands}\n\nParty                                      GP {:>8}\n{party}",
        game.repository().gp()
    )
}

pub(super) fn render_status(
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
) -> String {
    let Some(member) = member_at(game, state.member_index) else {
        return "No party members.".to_owned();
    };
    let stats = derived_stats(member, catalog);
    let statuses = member
        .status_effects()
        .map(|status| format!("{status:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    let equipment = EquipmentSlot::ALL
        .into_iter()
        .map(|slot| {
            let name = member
                .equipment()
                .get(slot)
                .and_then(|id| catalog.item(id))
                .map_or("(Empty)", item_name);
            format!("  {:<10} {name}", slot.as_str())
        })
        .collect::<Vec<_>>()
        .join("\n");
    let abilities = learned_field_abilities(member, game, catalog)
        .into_iter()
        .map(|ability| ability.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Member {}/{}  {} — {}  Lv {}  {:?}\nHP {}/{}    MP {}/{}    EXP {}\n\nBase     STR {:>3}  DEX {:>3}  CON {:>3}  INT {:>3}\nDerived  STR {:>3}  DEX {:>3}  CON {:>3}  INT {:>3}\n\nEquipment\n{}\n\nField abilities: {}\nStatus effects: {}",
        state.member_index + 1,
        game.party().len(),
        member.name(),
        member.class_id(),
        member.level(),
        member.row(),
        member.health(),
        member.max_health(),
        member.mana(),
        member.max_mana(),
        member.experience(),
        member.stats().strength(),
        member.stats().dexterity(),
        member.stats().constitution(),
        member.stats().intelligence(),
        stats.strength,
        stats.dexterity,
        stats.constitution,
        stats.intelligence,
        equipment,
        if abilities.is_empty() {
            "None"
        } else {
            &abilities
        },
        if statuses.is_empty() {
            "None"
        } else {
            &statuses
        }
    )
}

pub(super) fn render_items(
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
) -> String {
    let tabs = InventoryTab::ALL
        .iter()
        .enumerate()
        .map(|(index, tab)| {
            if index == state.tab_index {
                format!("[{}]", tab.label())
            } else {
                tab.label().to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("  ");
    let ids = inventory_ids(game, catalog, InventoryTab::ALL[state.tab_index]);
    let page = inventory_page_range(ids.len(), state.selected);
    let rows = ids
        .iter()
        .enumerate()
        .skip(page.start)
        .take(page.len())
        .map(|(index, id)| {
            let cursor = if state.mode == FieldMenuMode::Browse && index == state.selected {
                ">"
            } else {
                " "
            };
            let item = catalog.item(id).expect("filtered catalog item");
            format!(
                "{cursor} {:<30} x{:>3}",
                item_name(item),
                game.repository().item_count(id)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let detail = state
        .pending_id
        .as_deref()
        .or_else(|| ids.get(state.selected).copied())
        .and_then(|id| catalog.item(id))
        .map(item_description)
        .unwrap_or("No items in this tab.");
    let overlay = match state.mode {
        FieldMenuMode::ItemActions => ["Use", "Discard", "Hide"]
            .iter()
            .enumerate()
            .map(|(index, label)| {
                format!(
                    "{} {label}",
                    if index == state.selected { ">" } else { " " }
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
        FieldMenuMode::DiscardQuantity => format!("Discard how many?  {}", state.quantity),
        FieldMenuMode::ItemTarget => render_targets(state, game),
        _ => String::new(),
    };
    format!(
        "{tabs}\n\n{}\n\n{}\n{}",
        if rows.is_empty() { "(empty)" } else { &rows },
        detail,
        overlay
    )
}

pub(super) fn render_equipment(
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
) -> String {
    let Some(member) = member_at(game, state.member_index) else {
        return "No party members.".to_owned();
    };
    let slot_index = state_slot_index(state);
    let slots = EquipmentSlot::ALL
        .into_iter()
        .enumerate()
        .map(|(index, slot)| {
            let cursor = if state.mode == FieldMenuMode::Browse && index == state.selected {
                ">"
            } else {
                " "
            };
            let name = member
                .equipment()
                .get(slot)
                .and_then(|id| catalog.item(id))
                .map_or("(Empty)", item_name);
            format!("{cursor} {:<10} {name}", slot.as_str())
        })
        .collect::<Vec<_>>()
        .join("\n");
    if state.mode != FieldMenuMode::EquipmentPicker {
        return format!(
            "Member {}/{}  {} — {}\n\n{slots}",
            state.member_index + 1,
            game.party().len(),
            member.name(),
            member.class_id()
        );
    }
    let slot = EquipmentSlot::ALL[slot_index];
    let candidates = equipment_candidates(game, catalog, slot);
    let rows = std::iter::once(None)
        .chain(candidates.iter().map(|id| Some(id.as_str())))
        .enumerate()
        .map(|(index, id)| {
            let cursor = if index == state.selected { ">" } else { " " };
            match id {
                None => format!("{cursor} (Unequip)"),
                Some(id) => {
                    let item = catalog.item(id).expect("candidate exists");
                    let blocked = can_equip(member, item, catalog)
                        .err()
                        .map(|error| format!("  [{}]", error))
                        .unwrap_or_default();
                    format!("{cursor} {}{blocked}", item_name(item))
                }
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let selected_id = state
        .selected
        .checked_sub(1)
        .and_then(|index| candidates.get(index).map(String::as_str));
    let before = derived_stats(member, catalog);
    let after = preview_stats(member, catalog, slot, selected_id);
    format!(
        "Member {}/{}  {} — {}\n\n{slots}\n\nChoose {}\n{}\n\nPreview  STR {}->{}, DEX {}->{}, CON {}->{}, INT {}->{}",
        state.member_index + 1,
        game.party().len(),
        member.name(),
        member.class_id(),
        slot.as_str(),
        rows,
        before.strength,
        after.strength,
        before.dexterity,
        after.dexterity,
        before.constitution,
        after.constitution,
        before.intelligence,
        after.intelligence
    )
}

pub(super) fn render_spells(
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
) -> String {
    let Some(member) = member_at(game, state.member_index) else {
        return "No party members.".to_owned();
    };
    let abilities = learned_field_abilities(member, game, catalog);
    let rows = abilities
        .iter()
        .enumerate()
        .map(|(index, ability)| {
            format!(
                "{} {:<24} MP {:>3}  {}",
                if state.mode == FieldMenuMode::Browse && index == state.selected {
                    ">"
                } else {
                    " "
                },
                ability.name,
                ability.mp_cost,
                ability.description
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let overlay = match state.mode {
        FieldMenuMode::SpellTarget => render_targets(state, game),
        FieldMenuMode::TeleportPicker => catalog
            .eligible_warp_destinations(game.map())
            .iter()
            .enumerate()
            .map(|(index, destination)| {
                format!(
                    "{} {}",
                    if index == state.selected { ">" } else { " " },
                    destination.name
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    };
    format!(
        "Member {}/{}  {} — MP {}/{}\n\n{}\n\n{}",
        state.member_index + 1,
        game.party().len(),
        member.name(),
        member.mana(),
        member.max_mana(),
        if rows.is_empty() {
            "(No learned field abilities)"
        } else {
            &rows
        },
        overlay
    )
}

pub(super) fn render_targets(state: &FieldMenuState, game: &GameState) -> String {
    game.party()
        .members()
        .enumerate()
        .map(|(index, member)| {
            format!(
                "{} {:<12} HP {}/{}  MP {}/{}",
                if index == state.selected { ">" } else { " " },
                member.name(),
                member.health(),
                member.max_health(),
                member.mana(),
                member.max_mana()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn render_save(state: &FieldMenuState, saves: &SaveSlotCatalog) -> String {
    let page_start =
        ((state.selected.saturating_sub(FIRST_PLAYER_SLOT)) / 7) * 7 + FIRST_PLAYER_SLOT;
    let rows = saves
        .slots()
        .iter()
        .skip(page_start)
        .take(7)
        .map(|slot| {
            let cursor = if slot.index == state.selected {
                ">"
            } else {
                " "
            };
            match (&slot.state, &slot.metadata) {
                (SaveSlotState::Empty, _) => {
                    format!("{cursor} {:<8} --- Empty ---", slot.label())
                }
                (SaveSlotState::Valid, Some(metadata)) => format!(
                    "{cursor} {:<8} {} Lv{}  {}  {}",
                    slot.label(),
                    metadata.protagonist_name,
                    metadata.protagonist_level,
                    crate::playtime::Playtime::format(metadata.playtime_seconds),
                    metadata.location,
                ),
                (SaveSlotState::Corrupt(_), _) => {
                    format!("{cursor} {:<8} [CORRUPT]", slot.label())
                }
                (SaveSlotState::Incompatible(_), _) => {
                    format!("{cursor} {:<8} [INCOMPATIBLE]", slot.label())
                }
                _ => format!("{cursor} {:<8} [INVALID METADATA]", slot.label()),
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    if rows.is_empty() {
        "Discovering native save slots...".to_owned()
    } else {
        rows
    }
}

pub(super) fn render_hint(state: &FieldMenuState) -> String {
    match (state.screen, state.mode) {
        (FieldMenuScreen::Main, FieldMenuMode::QuitConfirm) => {
            "Y/ENTER exit to desktop  N/ESC cancel"
        }
        (FieldMenuScreen::Main, _) => "UP/DOWN choose  ENTER open  M/ESC close",
        (FieldMenuScreen::Status, _) if state.status_page == StatusPage::Roster => {
            "UP/DOWN member  ENTER stats  ESC back  M close"
        }
        (FieldMenuScreen::Status, _) => "UP/DOWN action  ESC portrait  M close",
        (FieldMenuScreen::Items, FieldMenuMode::Browse) => {
            "LEFT/RIGHT tab  UP/DOWN item  ENTER actions  ESC back"
        }
        (FieldMenuScreen::Items, FieldMenuMode::DiscardQuantity) => {
            "UP/DOWN quantity  LEFT one  RIGHT whole stack  ENTER discard  ESC cancel"
        }
        (FieldMenuScreen::Equipment, FieldMenuMode::Browse) => {
            "LEFT/RIGHT member  UP/DOWN slot  ENTER choose  ESC back"
        }
        (FieldMenuScreen::Spells, FieldMenuMode::Browse) => {
            "LEFT/RIGHT member  UP/DOWN spell  ENTER cast  ESC back"
        }
        (FieldMenuScreen::Save, FieldMenuMode::Browse) => {
            "UP/DOWN slot  ENTER save  ESC back  M close"
        }
        (FieldMenuScreen::Save, FieldMenuMode::SaveConfirm) => "Y/ENTER overwrite  N/ESC cancel",
        (_, FieldMenuMode::ItemTarget | FieldMenuMode::SpellTarget) => {
            "UP/DOWN target  ENTER confirm  ESC cancel"
        }
        (_, FieldMenuMode::TeleportPicker) => "UP/DOWN destination  ENTER teleport  ESC cancel",
        _ => "UP/DOWN choose  ENTER confirm  ESC cancel  M close",
    }
    .to_owned()
}

pub(super) fn cleanup_field_menu(
    mut commands: Commands,
    roots: Query<Entity, With<FieldMenuRoot>>,
    mut state: ResMut<FieldMenuState>,
) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    state.close();
}
