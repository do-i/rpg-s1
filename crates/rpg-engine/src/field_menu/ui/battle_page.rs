use super::*;
use crate::battle_tactics::choice_label;

#[expect(
    clippy::too_many_arguments,
    reason = "the battle page coordinates the shared menu root, catalog readiness, and live party"
)]
pub(in crate::field_menu) fn sync_battle_page(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
    state: Res<FieldMenuState>,
    catalog: Res<FieldMenuCatalog>,
    game: Option<Res<GameState>>,
    options: Option<Res<PlayerOptions>>,
    menu_roots: Query<Entity, With<FieldMenuRoot>>,
    pages: Query<Entity, With<FieldMenuBattlePage>>,
) {
    let show = state.open
        && state.screen == FieldMenuScreen::Battle
        && catalog.status() == CatalogStatus::Ready
        && game.is_some();
    if !show {
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
    let options_changed = options.as_ref().is_some_and(|options| options.is_changed());
    if !(pages.is_empty()
        || state.is_changed()
        || catalog.is_changed()
        || game.is_changed()
        || options_changed)
    {
        return;
    }
    for entity in &pages {
        commands.entity(entity).despawn();
    }
    let Some(font) = scenario_font(&asset_server, &root, &inventory) else {
        return;
    };
    commands.entity(menu_root).with_children(|parent| {
        let default = PlayerOptions::default();
        let options = options.as_deref().unwrap_or(&default);
        spawn_battle_page(parent, &font, &state, &game, &catalog, options);
    });
}

pub(in crate::field_menu) fn spawn_battle_page(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
    options: &PlayerOptions,
) {
    let selected = state.selected.min(battle_row_count(game) - 1);
    let picking = state.mode == FieldMenuMode::BattleTacticPicker;
    parent
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(14),
                ..default()
            },
            FieldMenuBattlePage,
            Name::new("Battle page"),
        ))
        .with_children(|page| {
            spawn_battle_header(page, font, game);
            page.spawn(Node {
                width: percent(100),
                flex_grow: 1.0,
                min_height: px(0),
                flex_direction: FlexDirection::Row,
                column_gap: px(QUEST_COLUMN_GAP),
                ..default()
            })
            .with_children(|columns| {
                spawn_status_panel(
                    columns,
                    Node {
                        width: px(BATTLE_LIST_WIDTH),
                        height: percent(100),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(7),
                        padding: UiRect::all(px(16)),
                        border: UiRect::all(px(1)),
                        border_radius: BorderRadius::all(px(6)),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    "PARTY",
                    font,
                    |panel| {
                        for (index, member) in game.party().members().enumerate() {
                            spawn_battle_member_row(
                                panel,
                                font,
                                member,
                                catalog,
                                index == selected,
                            );
                        }
                        spawn_section_rule(panel);
                        let party = game.party().len();
                        spawn_battle_setting_row(
                            panel,
                            font,
                            "Enemy turns",
                            "ALL ENEMIES",
                            if options.auto_enemy_turns {
                                "AUTO"
                            } else {
                                "MANUAL"
                            },
                            options.auto_enemy_turns,
                            selected == party,
                        );
                        spawn_battle_setting_row(
                            panel,
                            font,
                            "Turn pause",
                            "BEFORE AN AUTO TURN MOVES ON",
                            options.turn_pause.label(),
                            true,
                            selected == party + 1,
                        );
                        spawn_battle_setting_row(
                            panel,
                            font,
                            "After victory",
                            "WHEN THE LAST ENEMY FALLS",
                            if options.skip_victory_screen {
                                "TO WORLD"
                            } else {
                                "SHOW REWARDS"
                            },
                            !options.skip_victory_screen,
                            selected == party + 2,
                        );
                    },
                );
                let row = battle_row(game, selected);
                let (choices, current) = battle_row_choices(game, options, row);
                let title = match row {
                    BattleRow::Member(index) => {
                        member_at(game, index).map_or_else(String::new, |m| m.name().to_owned())
                    }
                    BattleRow::EnemyTurns => "Enemy turns".to_owned(),
                    BattleRow::TurnPause => "Turn pause".to_owned(),
                    BattleRow::AfterVictory => "After victory".to_owned(),
                };
                spawn_battle_orders(
                    columns,
                    font,
                    &title,
                    &choices,
                    current,
                    picking.then_some(state.battle_choice.min(choices.len() - 1)),
                );
            });
            spawn_status_text(
                page,
                if picking {
                    "UP/DOWN   CHOOSE      ENTER   SET      ESC   CANCEL"
                } else {
                    "UP/DOWN   SELECT      ENTER   CHANGE      ESC   BACK"
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

fn spawn_battle_header(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    game: &GameState,
) {
    let auto = game
        .party()
        .members()
        .filter(|member| member.tactics().auto)
        .count();
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
                    spawn_status_text(title, "BATTLE TACTICS", font, 31.0, status_ink());
                    spawn_status_text(
                        title,
                        "WHO TAKES ORDERS, AND WHO FIGHTS ON THEIR OWN",
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
                    spawn_quest_counter(counters, font, "AUTO", auto, status_teal());
                    spawn_quest_counter(
                        counters,
                        font,
                        "MANUAL",
                        game.party().len() - auto,
                        status_gold(),
                    );
                });
        });
}

/// One member's row: name, level and class, and their current orders.
fn spawn_battle_member_row(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    member: &crate::runtime_member::RuntimeMember,
    catalog: &FieldMenuCatalog,
    selected: bool,
) {
    let tactics = member.tactics();
    let mut row = parent.spawn((
        Node {
            width: percent(100),
            min_height: px(54),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(10),
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
        BattleTacticsRow,
    ));
    if selected {
        row.insert(SelectedBattleTacticsRow);
    }
    row.with_children(|row| {
        row.spawn(Node {
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|copy| {
            spawn_status_text(copy, member.name(), font, 17.0, status_ink());
            spawn_status_text(
                copy,
                format!(
                    "LV {}   {}",
                    member.level(),
                    class_name(member.class_id(), catalog).to_uppercase()
                ),
                font,
                12.0,
                status_faint(),
            );
        });
        let color = if tactics.auto {
            status_teal()
        } else {
            status_gold()
        };
        row.spawn((
            Node {
                min_width: px(130),
                justify_content: JustifyContent::Center,
                padding: UiRect::axes(px(10), px(3)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(9)),
                ..default()
            },
            BackgroundColor(Color::srgba_u8(10, 10, 14, 170)),
            BorderColor::all(color),
        ))
        .with_children(|chip| {
            spawn_status_text(
                chip,
                choice_label(tactics.auto_command()).to_uppercase(),
                font,
                13.0,
                color,
            );
        });
    });
}

/// One battle-wide setting under the party list, styled like a member row.
fn spawn_battle_setting_row(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    name: &str,
    subtitle: &str,
    value: &str,
    active: bool,
    selected: bool,
) {
    let mut row = parent.spawn((
        Node {
            width: percent(100),
            min_height: px(46),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(10),
            padding: UiRect::axes(px(10), px(5)),
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
        BattleTacticsRow,
    ));
    if selected {
        row.insert(SelectedBattleTacticsRow);
    }
    row.with_children(|row| {
        row.spawn(Node {
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|copy| {
            spawn_status_text(copy, name, font, 16.0, status_ink());
            spawn_status_text(copy, subtitle, font, 11.0, status_faint());
        });
        let color = if active { status_teal() } else { status_gold() };
        row.spawn((
            Node {
                min_width: px(130),
                justify_content: JustifyContent::Center,
                padding: UiRect::axes(px(10), px(3)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(9)),
                ..default()
            },
            BackgroundColor(Color::srgba_u8(10, 10, 14, 170)),
            BorderColor::all(color),
        ))
        .with_children(|chip| {
            spawn_status_text(chip, value.to_uppercase(), font, 13.0, color);
        });
    });
}

/// The focused row's orders. While picking, lists every choice with the highlighted one's
/// explanation underneath; otherwise explains the setting in force.
fn spawn_battle_orders(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    title: &str,
    choices: &[(String, &'static str)],
    current: usize,
    picking: Option<usize>,
) {
    spawn_status_panel(
        parent,
        Node {
            flex_grow: 1.0,
            flex_basis: px(0),
            min_width: px(0),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            padding: UiRect::all(px(16)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(6)),
            overflow: Overflow::clip(),
            ..default()
        },
        "ORDERS",
        font,
        |panel| {
            let Some((current_label, current_description)) = choices.get(current) else {
                return;
            };
            spawn_status_text(panel, title, font, 22.0, status_gold());
            let Some(highlighted) = picking else {
                spawn_status_text(panel, current_label, font, 17.0, status_ink());
                spawn_status_text(panel, *current_description, font, 14.0, status_muted());
                spawn_status_text(panel, "Press Enter to change.", font, 13.0, status_faint());
                return;
            };
            for (index, (label, _)) in choices.iter().enumerate() {
                let focused = index == highlighted;
                panel
                    .spawn((
                        Node {
                            width: percent(100),
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::SpaceBetween,
                            padding: UiRect::axes(px(10), px(5)),
                            border: UiRect::all(px(if focused { 2 } else { 1 })),
                            border_radius: BorderRadius::all(px(4)),
                            ..default()
                        },
                        BackgroundColor(if focused {
                            Color::srgba_u8(72, 49, 25, 218)
                        } else {
                            Color::srgba_u8(10, 10, 14, 148)
                        }),
                        BorderColor::all(if focused {
                            status_border_active()
                        } else {
                            Color::srgba_u8(126, 98, 55, 90)
                        }),
                    ))
                    .with_children(|option| {
                        spawn_status_text(option, label, font, 16.0, status_ink());
                        if index == current {
                            spawn_status_text(option, "CURRENT", font, 11.0, status_teal());
                        }
                    });
            }
            spawn_status_text(panel, choices[highlighted].1, font, 14.0, status_muted());
        },
    );
}
