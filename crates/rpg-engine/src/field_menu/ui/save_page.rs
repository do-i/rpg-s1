use super::*;

#[expect(
    clippy::too_many_arguments,
    reason = "the save page coordinates the shared menu root, live session, and discovered slots"
)]
pub(in crate::field_menu) fn sync_save_page(
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

pub(in crate::field_menu) fn spawn_save_page(
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

pub(in crate::field_menu) fn spawn_save_header(
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
            spawn_header_bars(header, 42.0, 14.0);
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
                                format!(
                                    "{}  /  CHOOSE A RECORD",
                                    location_display_name(map.as_str()).to_uppercase()
                                )
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

pub(in crate::field_menu) fn spawn_field_save_slot_row(
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
                spawn_status_text(label, "PINNED", font, 10.0, status_faint());
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

pub(in crate::field_menu) fn spawn_save_slot_content(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    slot: &SaveSlot,
) {
    match (&slot.state, &slot.metadata) {
        (SaveSlotState::Empty, _) => {
            spawn_status_text(parent, "—  EMPTY  —", font, 17.0, status_faint());
        }
        (SaveSlotState::Valid, Some(metadata)) => {
            spawn_status_text(
                parent,
                format!(
                    "{}    ({})",
                    location_display_name(&metadata.location),
                    metadata.protagonist_name
                ),
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

pub(in crate::field_menu) fn spawn_save_inline_message(
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

pub(in crate::field_menu) fn spawn_save_overwrite_modal(
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

pub(in crate::field_menu) fn save_page_start(selected: usize) -> usize {
    ((selected.saturating_sub(FIRST_PLAYER_SLOT)) / SAVE_VISIBLE_ROWS) * SAVE_VISIBLE_ROWS
        + FIRST_PLAYER_SLOT
}
