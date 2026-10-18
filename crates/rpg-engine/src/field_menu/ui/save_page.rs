use super::*;

#[expect(
    clippy::too_many_arguments,
    reason = "the save page coordinates the shared menu root, live session, and discovered slots"
)]
pub(in crate::field_menu) fn sync_save_page(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
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

    let Some(font) = scenario_font(&asset_server, &root, &inventory) else {
        return;
    };
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
                    // Sized to its rows rather than the full content box, so no empty band sits
                    // under the last slot; the cap keeps it on screen if the rows ever outgrow it.
                    max_height: percent(100),
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
                    // The autosave is not a save target, so the list and its scroll indicators
                    // start at the first player slot: slot 1 has nothing above it.
                    let window = save_window(state);
                    spawn_scroll_list(modal, window, scroll_list_node(6.0), |modal| {
                        for index in window.indices() {
                            let Some(slot) = saves.slots().get(FIRST_PLAYER_SLOT + index) else {
                                continue;
                            };
                            spawn_field_save_slot_row(
                                modal,
                                font,
                                slot,
                                slot.index == state.selected,
                            );
                        }
                    });
                }
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
                    spawn_save_message(modal, font, &state.message, status_border());
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
                format!("SLOT {:02} / {LAST_PLAYER_SLOT:02}", state.selected),
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
        selectable_card_colors(selected),
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
            spawn_status_text(label, slot.label().to_uppercase(), font, 15.0, status_ink());
        });
        row.spawn(Node {
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|content| {
            spawn_save_slot_summary(content, font, slot);
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

const PLAYER_SLOT_COUNT: usize = LAST_PLAYER_SLOT - FIRST_PLAYER_SLOT + 1;

/// The visible player slots, indexed from the first player slot.
pub(in crate::field_menu) fn save_window(state: &FieldMenuState) -> ScrollWindow {
    state.scroll.window(
        "save_slots",
        state.selected.saturating_sub(FIRST_PLAYER_SLOT),
        PLAYER_SLOT_COUNT,
        SAVE_VISIBLE_ROWS,
    )
}
