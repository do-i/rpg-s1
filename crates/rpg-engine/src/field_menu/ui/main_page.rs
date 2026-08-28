use super::*;

#[expect(
    clippy::too_many_arguments,
    reason = "Bevy menu synchronization keeps each resource and query boundary explicit"
)]
pub(in crate::field_menu) fn sync_main_menu_page(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
    state: Res<FieldMenuState>,
    game: Option<Res<GameState>>,
    walk_sheets: Res<PartyWalkSheets>,
    atlases: Res<Assets<TsxAtlasAsset>>,
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
    let Some(game) = game else {
        return;
    };
    // The switch overlay draws sprite frames, so it also rebuilds as those sheets finish loading.
    if !pages.is_empty() && !state.is_changed() && !atlases.is_changed() {
        return;
    }
    for entity in &pages {
        commands.entity(entity).despawn();
    }

    let Some(font) = scenario_font(&asset_server, &root, &inventory) else {
        return;
    };
    let thumbnails = PartyWalkThumbnails::resolve(&walk_sheets, &atlases, &game);
    commands.entity(menu_root).with_children(|parent| {
        spawn_main_menu_page(parent, &font, &state, &game, &thumbnails);
    });
}

/// The idle down-facing frame of each party member's walk sheet, once its TSX has loaded.
#[derive(Default)]
pub(in crate::field_menu) struct PartyWalkThumbnails {
    frames: std::collections::BTreeMap<String, (Handle<Image>, Handle<TextureAtlasLayout>)>,
}

impl PartyWalkThumbnails {
    pub(in crate::field_menu) fn resolve(
        sheets: &PartyWalkSheets,
        atlases: &Assets<TsxAtlasAsset>,
        game: &GameState,
    ) -> Self {
        let mut thumbnails = Self::default();
        for member in game.party().members() {
            let Some(atlas) = sheets
                .sheet(member.id())
                .and_then(|handle| atlases.get(handle))
            else {
                continue;
            };
            thumbnails.frames.insert(
                member.id().to_owned(),
                (atlas.image().clone(), atlas.layout().clone()),
            );
        }
        thumbnails
    }

    fn frame(&self, member_id: &str) -> Option<ImageNode> {
        let (image, layout) = self.frames.get(member_id)?;
        Some(ImageNode::from_atlas_image(
            image.clone(),
            TextureAtlas {
                layout: layout.clone(),
                index: WALK_SHEET_IDLE_DOWN_TILE,
            },
        ))
    }
}

pub(in crate::field_menu) fn spawn_main_menu_page(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    thumbnails: &PartyWalkThumbnails,
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
            if state.mode == FieldMenuMode::CharacterSwitch {
                spawn_field_menu_character_modal(page, font, state, game, thumbnails);
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

/// Roster overlay for choosing which member the World sprite follows.
///
/// Ports `switch_character_scene.render`: one row per member with their idle walk frame, and an
/// `ACTIVE` badge on whoever is currently controlled.
pub(in crate::field_menu) fn spawn_field_menu_character_modal(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    thumbnails: &PartyWalkThumbnails,
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
            FieldMenuCharacterModal,
            Name::new("Switch character modal"),
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        width: px(460),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(8),
                        padding: UiRect::all(px(18)),
                        border: UiRect::all(px(2)),
                        border_radius: BorderRadius::all(px(7)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb_u8(22, 22, 28)),
                    BorderColor::all(status_border_active()),
                ))
                .with_children(|modal| {
                    spawn_status_text(modal, "SWITCH CHARACTER", font, 14.0, status_gold());
                    spawn_section_rule(modal);
                    for (index, member) in game.party().members().enumerate() {
                        let selected = index == state.selected;
                        let active = member.id() == game.controlled_member_id();
                        spawn_character_row(
                            modal,
                            font,
                            member.name(),
                            thumbnails.frame(member.id()),
                            selected,
                            active,
                        );
                    }
                    spawn_section_rule(modal);
                    spawn_status_text(
                        modal,
                        "UP/DOWN   SELECT      ENTER   CONFIRM      ESC   CANCEL",
                        font,
                        14.0,
                        status_gold(),
                    );
                });
        });
}

fn spawn_character_row(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    name: &str,
    frame: Option<ImageNode>,
    selected: bool,
    active: bool,
) {
    parent
        .spawn((
            Node {
                width: percent(100),
                height: px(54),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(12),
                padding: UiRect::axes(px(8), px(6)),
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
        .with_children(|row| {
            let sprite_frame = Node {
                width: px(40),
                height: px(40),
                flex_shrink: 0.0,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                overflow: Overflow::clip(),
                ..default()
            };
            if let Some(frame) = frame {
                row.spawn((frame, sprite_frame));
            } else {
                // The sheet has not finished loading, or the member ships none.
                row.spawn(sprite_frame).with_children(|placeholder| {
                    spawn_status_text(placeholder, member_emblem(name), font, 15.0, status_muted());
                });
            }
            spawn_status_text(
                row,
                name,
                font,
                19.0,
                if selected {
                    status_ink()
                } else {
                    status_muted()
                },
            );
            row.spawn(Node {
                flex_grow: 1.0,
                ..default()
            });
            if active {
                spawn_status_text(
                    row,
                    "ACTIVE",
                    font,
                    13.0,
                    if selected {
                        status_gold()
                    } else {
                        status_faint()
                    },
                );
            }
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
