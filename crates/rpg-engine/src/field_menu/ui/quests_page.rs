use super::*;

#[expect(
    clippy::too_many_arguments,
    reason = "the quests page coordinates the shared menu root, quest catalog, and live flags"
)]
pub(in crate::field_menu) fn sync_quests_page(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
    state: Res<FieldMenuState>,
    catalog: Res<FieldMenuCatalog>,
    game: Option<Res<GameState>>,
    menu_roots: Query<Entity, With<FieldMenuRoot>>,
    pages: Query<Entity, With<FieldMenuQuestsPage>>,
) {
    let show_quests = state.open
        && state.screen == FieldMenuScreen::Quests
        && catalog.status() == CatalogStatus::Ready
        && game.is_some();
    if !show_quests {
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
        spawn_quests_page(parent, &font, &state, &game, &catalog);
    });
}

pub(in crate::field_menu) fn spawn_quests_page(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
) {
    let quests = catalog.quests();
    let selected = selected_quest_index(state, quests);
    parent
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(14),
                ..default()
            },
            FieldMenuQuestsPage,
            Name::new("Quests page"),
        ))
        .with_children(|page| {
            spawn_quests_header(page, font, game, quests);
            page.spawn(Node {
                width: percent(100),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Row,
                column_gap: px(QUEST_COLUMN_GAP),
                ..default()
            })
            .with_children(|columns| {
                spawn_quest_list_column(columns, font, game, quests, selected);
                spawn_quest_detail_column(columns, font, game, quests.get(selected));
            });
            spawn_status_text(
                page,
                "UP/DOWN   SELECT QUEST      ESC   BACK      M   CLOSE",
                font,
                15.0,
                status_muted(),
            );
            if !state.message.is_empty() {
                spawn_items_message(page, font, &state.message);
            }
        });
}

pub(in crate::field_menu) fn spawn_quests_header(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    game: &GameState,
    quests: &[QuestDefinition],
) {
    let tally = QuestTally::of(quests, game);
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
                    spawn_status_text(title, "QUEST BOARD", font, 31.0, status_ink());
                    spawn_status_text(
                        title,
                        "OATHS TAKEN, ERRANDS OWED, DEBTS SETTLED",
                        font,
                        14.0,
                        status_muted(),
                    );
                });
            header
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(18),
                    ..default()
                })
                .with_children(|counters| {
                    spawn_quest_counter(counters, font, "ACTIVE", tally.active, status_gold());
                    spawn_quest_counter(counters, font, "CLEARED", tally.completed, status_teal());
                    spawn_quest_counter(
                        counters,
                        font,
                        "RECORDED",
                        quests.len(),
                        Color::srgb_u8(157, 139, 101),
                    );
                });
        });
}

pub(in crate::field_menu) fn spawn_quest_counter(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    label: &str,
    value: usize,
    accent: Color,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|counter| {
            spawn_status_text(counter, format!("{value:02}"), font, 22.0, accent);
            spawn_status_text(counter, label, font, 11.0, status_muted());
        });
}

pub(in crate::field_menu) fn spawn_quest_list_column(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    game: &GameState,
    quests: &[QuestDefinition],
    selected: usize,
) {
    spawn_status_panel(
        parent,
        Node {
            width: px(QUEST_LIST_WIDTH),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(7),
            padding: UiRect::all(px(16)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(6)),
            overflow: Overflow::clip(),
            ..default()
        },
        "REGISTER",
        font,
        |panel| {
            if quests.is_empty() {
                spawn_status_text(
                    panel,
                    "No quests are registered.",
                    font,
                    18.0,
                    status_muted(),
                );
                spawn_status_text(
                    panel,
                    "Notices appear here as the world learns your name.",
                    font,
                    14.0,
                    status_faint(),
                );
                return;
            }
            let first = quest_window_start(selected, quests.len());
            for (index, quest) in quests
                .iter()
                .enumerate()
                .skip(first)
                .take(QUEST_VISIBLE_ROWS)
            {
                spawn_quest_row(panel, font, quest, game, index == selected);
            }
            panel.spawn(Node {
                flex_grow: 1.0,
                ..default()
            });
            spawn_status_text(
                panel,
                register_footer(selected, first, quests.len()),
                font,
                11.0,
                status_faint(),
            );
        },
    );
}

pub(in crate::field_menu) fn spawn_quest_row(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    quest: &QuestDefinition,
    game: &GameState,
    selected: bool,
) {
    let status = quest_status(quest, game.flags());
    let mut row = parent.spawn((
        Node {
            width: percent(100),
            min_height: px(54),
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
        QuestBoardRow,
    ));
    if selected {
        row.insert(SelectedQuestBoardRow);
    }
    row.with_children(|row| {
        row.spawn((
            Node {
                width: px(6),
                height: px(34),
                margin: UiRect::right(px(10)),
                ..default()
            },
            BackgroundColor(quest_kind_accent(quest.kind)),
        ));
        row.spawn(Node {
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|copy| {
            spawn_status_text(
                copy,
                &quest.name,
                font,
                17.0,
                if status == QuestStatus::Inactive {
                    status_muted()
                } else {
                    status_ink()
                },
            );
            spawn_status_text(
                copy,
                format!(
                    "{}   ·   {}",
                    quest_kind_label(quest.kind),
                    quest.location.to_uppercase()
                ),
                font,
                12.0,
                status_faint(),
            );
        });
        spawn_quest_status_pill(row, font, status);
    });
}

pub(in crate::field_menu) fn spawn_quest_status_pill(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    status: QuestStatus,
) {
    parent
        .spawn((
            Node {
                min_width: px(94),
                justify_content: JustifyContent::Center,
                padding: UiRect::axes(px(8), px(3)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(9)),
                ..default()
            },
            BackgroundColor(Color::srgba_u8(10, 10, 14, 170)),
            BorderColor::all(quest_status_color(status)),
        ))
        .with_children(|pill| {
            spawn_status_text(
                pill,
                quest_status_label(status),
                font,
                11.0,
                quest_status_color(status),
            );
        });
}

pub(in crate::field_menu) fn spawn_quest_detail_column(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    game: &GameState,
    quest: Option<&QuestDefinition>,
) {
    spawn_status_panel(
        parent,
        Node {
            flex_basis: px(0),
            flex_grow: 1.0,
            height: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(10),
            padding: UiRect::all(px(18)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(6)),
            overflow: Overflow::clip(),
            ..default()
        },
        "NOTICE",
        font,
        |panel| {
            let Some(quest) = quest else {
                spawn_status_text(
                    panel,
                    "Select a quest to read its notice.",
                    font,
                    16.0,
                    status_muted(),
                );
                return;
            };
            let status = quest_status(quest, game.flags());
            spawn_status_text(panel, &quest.name, font, 24.0, status_gold());
            panel
                .spawn(Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(12),
                    ..default()
                })
                .with_children(|meta| {
                    spawn_quest_status_pill(meta, font, status);
                    spawn_status_text(
                        meta,
                        quest_kind_label(quest.kind),
                        font,
                        12.0,
                        quest_kind_accent(quest.kind),
                    );
                });
            spawn_section_rule(panel);
            spawn_quest_meta_row(panel, font, "WHERE", &quest.location);
            spawn_status_text(panel, &quest.description, font, 16.0, status_ink());
            panel.spawn(Node {
                flex_grow: 1.0,
                ..default()
            });
            spawn_section_rule(panel);
            spawn_quest_track(panel, font, status);
        },
    );
}

pub(in crate::field_menu) fn spawn_quest_meta_row(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    label: &str,
    value: &str,
) {
    parent
        .spawn(Node {
            width: percent(100),
            flex_direction: FlexDirection::Row,
            column_gap: px(10),
            ..default()
        })
        .with_children(|row| {
            row.spawn(Node {
                width: px(58),
                ..default()
            })
            .with_children(|slot| {
                spawn_status_text(slot, label, font, 12.0, status_muted());
            });
            row.spawn(Node {
                flex_grow: 1.0,
                ..default()
            })
            .with_children(|slot| {
                spawn_status_text(slot, value, font, 14.0, status_ink());
            });
        });
}

/// Draws the three flag-derived board states with the current one lit.
///
/// The pinned game keeps no objective counters, so this track is the only progress the quest data
/// can honestly show.
pub(in crate::field_menu) fn spawn_quest_track(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    status: QuestStatus,
) {
    let stages = [
        QuestStatus::Inactive,
        QuestStatus::Active,
        QuestStatus::Completed,
    ];
    let reached = stages
        .iter()
        .position(|stage| *stage == status)
        .unwrap_or(0);
    parent
        .spawn(Node {
            width: percent(100),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(6),
            ..default()
        })
        .with_children(|track| {
            for (index, stage) in stages.into_iter().enumerate() {
                let lit = index <= reached;
                track
                    .spawn(Node {
                        flex_basis: px(0),
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::Column,
                        row_gap: px(4),
                        ..default()
                    })
                    .with_children(|step| {
                        step.spawn((
                            Node {
                                width: percent(100),
                                height: px(4),
                                border_radius: BorderRadius::all(px(2)),
                                ..default()
                            },
                            BackgroundColor(if lit {
                                quest_status_color(status)
                            } else {
                                Color::srgb_u8(38, 36, 32)
                            }),
                        ));
                        spawn_status_text(
                            step,
                            quest_status_label(stage),
                            font,
                            10.0,
                            if index == reached {
                                quest_status_color(status)
                            } else {
                                status_faint()
                            },
                        );
                    });
            }
        });
}

/// Counts board states once so the header does not re-derive them per counter.
struct QuestTally {
    active: usize,
    completed: usize,
}

impl QuestTally {
    fn of(quests: &[QuestDefinition], game: &GameState) -> Self {
        let mut tally = Self {
            active: 0,
            completed: 0,
        };
        for quest in quests {
            match quest_status(quest, game.flags()) {
                QuestStatus::Active => tally.active += 1,
                QuestStatus::Completed => tally.completed += 1,
                QuestStatus::Inactive => {}
            }
        }
        tally
    }
}

pub(in crate::field_menu) fn selected_quest_index(
    state: &FieldMenuState,
    quests: &[QuestDefinition],
) -> usize {
    if quests.is_empty() {
        0
    } else {
        state.selected.min(quests.len() - 1)
    }
}

pub(in crate::field_menu) fn quest_window_start(selected: usize, len: usize) -> usize {
    window_start(selected, len, QUEST_VISIBLE_ROWS)
}

/// Reports the cursor position, and the visible span too once the register scrolls.
pub(in crate::field_menu) fn register_footer(selected: usize, first: usize, len: usize) -> String {
    let position = format!("QUEST {:02} OF {len:02}", selected + 1);
    if len <= QUEST_VISIBLE_ROWS {
        position
    } else {
        format!(
            "{position}   ·   SHOWING {:02}-{:02}",
            first + 1,
            (first + QUEST_VISIBLE_ROWS).min(len)
        )
    }
}

pub(in crate::field_menu) fn quest_kind_label(kind: QuestKind) -> &'static str {
    match kind {
        QuestKind::Main => "MAIN STORY",
        QuestKind::Sub => "SIDE QUEST",
    }
}

pub(in crate::field_menu) fn quest_kind_accent(kind: QuestKind) -> Color {
    match kind {
        QuestKind::Main => status_ember(),
        QuestKind::Sub => Color::srgb_u8(91, 143, 183),
    }
}

pub(in crate::field_menu) fn quest_status_label(status: QuestStatus) -> &'static str {
    match status {
        QuestStatus::Inactive => "NOT STARTED",
        QuestStatus::Active => "IN PROGRESS",
        QuestStatus::Completed => "COMPLETE",
    }
}

pub(in crate::field_menu) fn quest_status_color(status: QuestStatus) -> Color {
    match status {
        QuestStatus::Inactive => status_muted(),
        QuestStatus::Active => status_gold(),
        QuestStatus::Completed => status_teal(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_keeps_the_selection_visible_at_both_ends() {
        assert_eq!(quest_window_start(0, 16), 0);
        assert_eq!(quest_window_start(QUEST_VISIBLE_ROWS - 1, 16), 0);
        assert_eq!(quest_window_start(QUEST_VISIBLE_ROWS, 16), 1);
        assert_eq!(quest_window_start(15, 16), 16 - QUEST_VISIBLE_ROWS);
    }

    #[test]
    fn the_footer_adds_the_visible_span_only_once_the_register_scrolls() {
        assert_eq!(register_footer(2, 0, 5), "QUEST 03 OF 05");
        assert_eq!(
            register_footer(9, 3, 16),
            "QUEST 10 OF 16   ·   SHOWING 04-10"
        );
    }

    #[test]
    fn a_short_register_never_scrolls() {
        assert_eq!(quest_window_start(0, 3), 0);
        assert_eq!(quest_window_start(2, 3), 0);
        assert_eq!(quest_window_start(0, QUEST_VISIBLE_ROWS), 0);
    }
}
