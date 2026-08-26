use super::*;

#[expect(
    clippy::too_many_arguments,
    reason = "the status page coordinates the shared menu root, loaded scenario data, and UI assets"
)]
pub(in crate::field_menu) fn sync_status_page(
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

pub(in crate::field_menu) fn spawn_status_page(
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

pub(in crate::field_menu) fn spawn_status_header(
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
            spawn_header_bars(header, 46.0, 15.0);
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

pub(in crate::field_menu) fn spawn_party_column(
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

pub(in crate::field_menu) fn spawn_member_column(
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

pub(in crate::field_menu) fn spawn_full_portrait_column(
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

pub(in crate::field_menu) fn spawn_member_details_column(
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

pub(in crate::field_menu) fn spawn_profile_column(
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
pub(in crate::field_menu) fn spawn_portrait_frame(
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

pub(in crate::field_menu) fn load_status_image(
    asset_server: &AssetServer,
    root: &ScenarioRoot,
    relative: &str,
) -> Option<Handle<Image>> {
    let relative = ScenarioRelativePath::try_from(relative).ok()?;
    Some(asset_server.load(root.resolve(&relative)))
}

pub(in crate::field_menu) fn profile_portrait_path(member_id: &str) -> String {
    format!("assets/images/{member_id}_profile.png")
}

pub(in crate::field_menu) fn large_status_portrait_path(member_id: &str) -> String {
    format!("assets/images/party_portraits_large/{member_id}_status_portrait.webp")
}
