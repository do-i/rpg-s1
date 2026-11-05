use super::*;

pub(in crate::field_menu) struct StatusPageArt<'a> {
    pub portraits: &'a StatusPortraitAssets,
    pub thumbnails: &'a PartyWalkThumbnails,
    pub inventory: Option<&'a ScenarioInventory>,
}

#[expect(
    clippy::too_many_arguments,
    reason = "status coordinates live menu, portraits, and walk-sheet assets"
)]
pub(in crate::field_menu) fn sync_status_page(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
    state: Res<FieldMenuState>,
    catalog: Res<FieldMenuCatalog>,
    game: Option<Res<GameState>>,
    walk_sheets: Res<PartyWalkSheets>,
    atlases: Res<Assets<TsxAtlasAsset>>,
    menu_roots: Query<Entity, With<FieldMenuRoot>>,
    pages: Query<Entity, With<FieldMenuStatusPage>>,
) {
    let show = state.open
        && state.screen == FieldMenuScreen::Status
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
    if !pages.is_empty()
        && !state.is_changed()
        && !catalog.is_changed()
        && !game.is_changed()
        && !atlases.is_changed()
        && !inventory.is_changed()
    {
        return;
    }
    for entity in &pages {
        commands.entity(entity).despawn();
    }
    let Some(font) = scenario_font(&asset_server, &root, &inventory) else {
        return;
    };
    let portraits = StatusPortraitAssets::load(&asset_server, &root, &game);
    let thumbnails = PartyWalkThumbnails::resolve(&walk_sheets, &atlases, &game, &inventory);
    commands.entity(menu_root).with_children(|parent| {
        spawn_status_page(
            parent,
            &font,
            &state,
            &game,
            &catalog,
            &StatusPageArt {
                portraits: &portraits,
                thumbnails: &thumbnails,
                inventory: Some(&inventory),
            },
        );
    });
}

pub(in crate::field_menu) fn spawn_status_page(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
    art: &StatusPageArt<'_>,
) {
    parent
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                ..default()
            },
            FieldMenuStatusPage,
            Name::new("Status page"),
        ))
        .with_children(|page| {
            spawn_status_header(page, font, state.member_index, game.party().len());
            page.spawn(Node {
                width: percent(100),
                min_height: px(0),
                flex_grow: 1.0,
                column_gap: px(14),
                ..default()
            })
            .with_children(|columns| {
                columns
                    .spawn(Node {
                        flex_basis: px(0),
                        min_width: px(0),
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::Column,
                        row_gap: px(14),
                        ..default()
                    })
                    .with_children(|left| {
                        spawn_status_chooser(left, font, state, game, art.portraits);
                        spawn_status_dossier(left, font, state, game, catalog, art);
                    });
                spawn_status_portrait(columns, font, state, game, catalog, art.portraits);
            });
            spawn_status_text(
                page,
                if state.message.is_empty() {
                    status_hint(state)
                } else {
                    &state.message
                },
                font,
                13.0,
                if state.message.is_empty() {
                    status_muted()
                } else {
                    status_teal()
                },
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
            height: px(48),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            column_gap: px(12),
            ..default()
        })
        .with_children(|header| {
            spawn_items_menu_icon(header, "media/images/icons/menu/status.png", 42.0, 0.0);
            header
                .spawn(Node {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    ..default()
                })
                .with_children(|title| {
                    spawn_status_text(title, "STATUS", font, 28.0, status_ink());
                    spawn_status_text(title, "PARTY ROSTER AND GROWTH", font, 12.0, status_muted());
                });
            spawn_status_text(
                header,
                format!(
                    "{:02} / {:02}",
                    member_index.saturating_add(1).min(party_len),
                    party_len
                ),
                font,
                15.0,
                status_gold(),
            );
        });
}

fn spawn_status_chooser(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    portraits: &StatusPortraitAssets,
) {
    parent
        .spawn((
            Node {
                height: px(116),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                padding: UiRect::all(px(12)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(6)),
                ..default()
            },
            BackgroundColor(Color::srgb_u8(22, 22, 28)),
            BorderColor::all(status_border()),
            Name::new("Status character chooser"),
        ))
        .with_children(|chooser| {
            chooser
                .spawn(Node {
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                })
                .with_children(|header| {
                    spawn_status_text(header, "CHARACTERS", font, 12.0, status_gold());
                    spawn_status_text(header, "<  Q / E  >", font, 12.0, status_muted());
                });
            chooser
                .spawn(Node {
                    flex_grow: 1.0,
                    column_gap: px(8),
                    ..default()
                })
                .with_children(|cards| {
                    for (index, member) in game.party().members().enumerate() {
                        let selected = index == state.member_index;
                        let mut card = cards.spawn((
                            Node {
                                flex_basis: px(0),
                                flex_grow: 1.0,
                                min_width: px(0),
                                align_items: AlignItems::Center,
                                column_gap: px(10),
                                padding: UiRect::all(px(6)),
                                border: UiRect::all(px(1)),
                                border_radius: BorderRadius::all(px(4)),
                                ..default()
                            },
                            BackgroundColor(if selected {
                                Color::srgb_u8(58, 46, 37)
                            } else {
                                Color::srgb_u8(32, 32, 39)
                            }),
                            BorderColor::all(if selected {
                                status_border_active()
                            } else {
                                status_border()
                            }),
                            StatusMemberCard,
                            Name::new(format!("Status character: {}", member.name())),
                        ));
                        if selected {
                            card.insert(SelectedStatusMemberCard);
                        }
                        card.with_children(|card| {
                            if let Some(portrait) = portraits.profile(member.id()) {
                                card.spawn((
                                    ImageNode::new(portrait),
                                    Node {
                                        width: px(52),
                                        height: px(52),
                                        flex_shrink: 0.0,
                                        ..default()
                                    },
                                ));
                            }
                            card.spawn(Node {
                                flex_direction: FlexDirection::Column,
                                min_width: px(0),
                                row_gap: px(3),
                                ..default()
                            })
                            .with_children(|summary| {
                                spawn_status_text(summary, member.name(), font, 18.0, status_ink());
                                spawn_status_text(
                                    summary,
                                    format!("LV {}", member.level()),
                                    font,
                                    12.0,
                                    status_muted(),
                                );
                            });
                        });
                    }
                });
        });
}

pub(in crate::field_menu) fn spawn_party_column(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
    portraits: &StatusPortraitAssets,
    focused: bool,
) {
    spawn_focus_panel(
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
        focused,
        |panel| {
            for (index, member) in game.party().members().enumerate() {
                let selected = index == state.member_index;
                let mut card = panel.spawn((
                    // Cards share the column's height, so a full party of five still fits
                    // while a smaller one gets larger cards and portraits.
                    Node {
                        width: percent(100),
                        flex_grow: 1.0,
                        flex_basis: px(0),
                        min_height: px(STATUS_CARD_MIN_HEIGHT),
                        max_height: px(STATUS_CARD_MAX_HEIGHT),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Stretch,
                        column_gap: px(12),
                        padding: UiRect::all(px(8)),
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
                        selected,
                        "Party portrait",
                    );
                    card.spawn(Node {
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
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
                        spawn_member_experience(summary, font, member);
                    });
                });
            }
        },
    );
}

/// `EXP 420/1000` beside a thin bar filled to the same fraction, or `MAX LEVEL` at the cap.
fn spawn_member_experience(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    member: &crate::runtime_member::RuntimeMember,
) {
    let next = member.experience_next();
    let (label, fraction) = if next == 0 {
        ("MAX LEVEL".to_owned(), 1.0)
    } else {
        (
            format!("EXP {}/{next}", member.experience()),
            (member.experience() as f32 / next as f32).clamp(0.0, 1.0),
        )
    };
    parent
        .spawn(Node {
            width: percent(100),
            align_items: AlignItems::Center,
            column_gap: px(8),
            ..default()
        })
        .with_children(|line| {
            spawn_status_text(line, label, font, 13.0, status_teal());
            line.spawn((
                Node {
                    flex_grow: 1.0,
                    height: px(4),
                    border_radius: BorderRadius::all(px(2)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(Color::srgba_u8(255, 255, 255, 28)),
            ))
            .with_children(|track| {
                track.spawn((
                    Node {
                        width: percent(fraction * 100.0),
                        height: percent(100),
                        ..default()
                    },
                    BackgroundColor(status_teal()),
                ));
            });
        });
}

pub(in crate::field_menu) fn spawn_portrait_frame(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    member_name: &str,
    portrait: Option<Handle<Image>>,
    selected: bool,
    label: &'static str,
) {
    // A square as tall as the card it sits in, so the portrait grows with the card.
    parent
        .spawn((
            Node {
                height: percent(100),
                aspect_ratio: Some(1.0),
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
                22.0,
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
    format!("media/images/{member_id}_profile.png")
}

pub(in crate::field_menu) fn large_status_portrait_path(member_id: &str) -> String {
    format!("media/images/party_portraits_large/{member_id}_status_portrait.webp")
}
