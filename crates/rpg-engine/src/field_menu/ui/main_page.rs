use super::*;

pub(in crate::field_menu) fn sync_main_menu_page(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
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

    let Some(font) = scenario_font(&asset_server, &root, &inventory) else {
        return;
    };
    commands.entity(menu_root).with_children(|parent| {
        spawn_main_menu_page(parent, &font, &state);
    });
}

pub(in crate::field_menu) fn spawn_main_menu_page(
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
                "ARROWS   SELECT      ENTER   CONFIRM      M / ESC   CLOSE",
                font,
                15.0,
                status_muted(),
            );

            if state.mode == FieldMenuMode::QuitConfirm {
                spawn_field_menu_quit_modal(page, font);
            }
        });
}

pub(in crate::field_menu) fn spawn_main_menu_header(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
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

pub(in crate::field_menu) fn spawn_main_commands_panel(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
) {
    spawn_status_panel(
        parent,
        Node {
            width: px(MAIN_DECK_WIDTH),
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
            panel
                .spawn(Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Row,
                    column_gap: px(MAIN_DECK_COLUMN_GAP),
                    align_items: AlignItems::Start,
                    ..default()
                })
                .with_children(|columns| {
                    for column in 0..main_command_columns() {
                        spawn_main_command_column(columns, font, state, column);
                    }
                });
        },
    );
}

pub(in crate::field_menu) fn spawn_main_command_column(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    column: usize,
) {
    parent
        .spawn(Node {
            flex_basis: px(0),
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            ..default()
        })
        .with_children(|list| {
            for (index, command) in MAIN_COMMANDS
                .iter()
                .enumerate()
                .skip(column * MAIN_COMMAND_ROWS)
                .take(MAIN_COMMAND_ROWS)
            {
                spawn_main_command_row(list, font, command, index, index == state.selected);
            }
        });
}

pub(in crate::field_menu) fn spawn_main_command_row(
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

pub(in crate::field_menu) fn spawn_field_menu_quit_modal(
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

pub(in crate::field_menu) fn main_command_accent(index: usize) -> Color {
    match index {
        0 => status_teal(),
        1 => status_violet(),
        2 => status_ember(),
        3 => status_gold(),
        4 => Color::srgb_u8(91, 143, 183),
        5 => Color::srgb_u8(124, 158, 116),
        _ => Color::srgb_u8(190, 72, 66),
    }
}
