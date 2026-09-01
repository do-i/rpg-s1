use super::*;

#[expect(
    clippy::too_many_arguments,
    reason = "the spells page coordinates the shared menu root, portraits, and live ability catalog"
)]
pub(in crate::field_menu) fn sync_spells_page(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
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

    let Some(font) = scenario_font(&asset_server, &root, &inventory) else {
        return;
    };
    let portraits = StatusPortraitAssets::load(&asset_server, &root, &game);
    commands.entity(menu_root).with_children(|parent| {
        spawn_spells_page(parent, &font, &state, &game, &catalog, &portraits);
    });
}

pub(in crate::field_menu) fn spawn_spells_page(
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

pub(in crate::field_menu) fn spawn_spells_header(
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
            spawn_header_bars(header, 46.0, 15.0);
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

pub(in crate::field_menu) fn spawn_spellbook_column(
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

pub(in crate::field_menu) fn spawn_spell_list(
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
                    status_faint(),
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

pub(in crate::field_menu) fn spawn_spell_detail(
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

pub(in crate::field_menu) fn spawn_spell_overlay(
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
                    overflow: Overflow::clip(),
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
                            let first = teleport_window_start(state.selected, destinations.len());
                            for (index, destination) in destinations
                                .iter()
                                .enumerate()
                                .skip(first)
                                .take(TELEPORT_VISIBLE_ROWS)
                            {
                                spawn_item_modal_row(
                                    modal,
                                    font,
                                    &destination.name,
                                    "standard map transition",
                                    index == state.selected,
                                );
                            }
                            if let Some(footer) =
                                teleport_footer(state.selected, first, destinations.len())
                            {
                                spawn_status_text(modal, footer, font, 11.0, status_faint());
                            }
                        }
                        _ => {}
                    }
                },
            );
        });
}

pub(in crate::field_menu) fn teleport_window_start(selected: usize, len: usize) -> usize {
    window_start(selected, len, TELEPORT_VISIBLE_ROWS)
}

/// Reports the visible span, but only once the destination list actually scrolls.
pub(in crate::field_menu) fn teleport_footer(
    selected: usize,
    first: usize,
    len: usize,
) -> Option<String> {
    if len <= TELEPORT_VISIBLE_ROWS {
        return None;
    }
    Some(format!(
        "DESTINATION {:02} OF {len:02}   ·   SHOWING {:02}-{:02}",
        selected + 1,
        first + 1,
        (first + TELEPORT_VISIBLE_ROWS).min(len)
    ))
}

pub(in crate::field_menu) fn selected_spell_index(
    state: &FieldMenuState,
    abilities: &[&Ability],
) -> usize {
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

pub(in crate::field_menu) fn spell_kind_label(ability: &Ability) -> &'static str {
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

pub(in crate::field_menu) fn spell_target_label(ability: &Ability) -> &'static str {
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

pub(in crate::field_menu) fn ability_target_label(
    target: crate::scenario_class::AbilityTarget,
) -> &'static str {
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

pub(in crate::field_menu) fn spell_accent(ability: &Ability) -> Color {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The shipped scenario has 29 maps declaring `warp_order`, so a late-game cast can offer 28
    /// destinations. Before windowing, every one of them spawned a 48px row into a 520px modal.
    const SHIPPED_DESTINATION_CEILING: usize = 28;

    #[test]
    fn the_destination_window_keeps_the_selection_visible_at_both_ends() {
        assert_eq!(teleport_window_start(0, SHIPPED_DESTINATION_CEILING), 0);
        assert_eq!(
            teleport_window_start(TELEPORT_VISIBLE_ROWS - 1, SHIPPED_DESTINATION_CEILING),
            0
        );
        assert_eq!(
            teleport_window_start(TELEPORT_VISIBLE_ROWS, SHIPPED_DESTINATION_CEILING),
            1
        );
        assert_eq!(
            teleport_window_start(SHIPPED_DESTINATION_CEILING - 1, SHIPPED_DESTINATION_CEILING),
            SHIPPED_DESTINATION_CEILING - TELEPORT_VISIBLE_ROWS
        );
    }

    #[test]
    fn a_short_destination_list_never_scrolls() {
        assert_eq!(teleport_window_start(0, 3), 0);
        assert_eq!(teleport_window_start(2, 3), 0);
        assert_eq!(teleport_window_start(0, TELEPORT_VISIBLE_ROWS), 0);
        assert_eq!(
            teleport_window_start(TELEPORT_VISIBLE_ROWS - 1, TELEPORT_VISIBLE_ROWS),
            0
        );
    }

    #[test]
    fn the_footer_appears_only_once_the_destination_list_scrolls() {
        assert_eq!(teleport_footer(2, 0, 5), None);
        assert_eq!(teleport_footer(0, 0, TELEPORT_VISIBLE_ROWS), None);
        assert_eq!(
            teleport_footer(9, 4, 28).as_deref(),
            Some("DESTINATION 10 OF 28   ·   SHOWING 05-10")
        );
    }

    /// However long the list grows, the modal only ever spawns a window of it.
    #[test]
    fn the_window_never_exceeds_the_visible_row_budget() {
        for len in 0..=SHIPPED_DESTINATION_CEILING {
            for selected in 0..len.max(1) {
                let first = teleport_window_start(selected, len);
                let shown = len.saturating_sub(first).min(TELEPORT_VISIBLE_ROWS);
                assert!(shown <= TELEPORT_VISIBLE_ROWS);
                if len > 0 {
                    assert!(
                        (first..first + shown).contains(&selected),
                        "selection {selected} fell outside the window for a list of {len}"
                    );
                }
            }
        }
    }
}
