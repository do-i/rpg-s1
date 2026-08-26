use super::*;

pub(in crate::field_menu) fn sync_field_menu_overlay_lifecycle(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
    theme: Res<UiTheme>,
    state: Res<FieldMenuState>,
    roots: Query<Entity, With<FieldMenuRoot>>,
) {
    if !state.open {
        for entity in &roots {
            commands.entity(entity).despawn();
        }
    } else if roots.is_empty() {
        spawn_field_menu_overlay(&mut commands, &asset_server, &root, &inventory, &theme);
    }
}

#[expect(
    clippy::type_complexity,
    reason = "the overlay updates three independently styled text roles"
)]
pub(in crate::field_menu) fn sync_field_menu_generic_text(
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

pub(in crate::field_menu) fn spawn_field_menu_overlay(
    commands: &mut Commands,
    asset_server: &AssetServer,
    root: &ScenarioRoot,
    inventory: &ScenarioInventory,
    theme: &UiTheme,
) {
    let Some(font) = scenario_font(asset_server, root, inventory) else {
        return;
    };
    let Some(backdrop_path) = inventory.menu_backdrop.as_ref() else {
        return;
    };
    let backdrop = asset_server.load(root.resolve(backdrop_path));
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

pub(in crate::field_menu) fn sync_custom_field_menu_content_visibility(
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

pub(in crate::field_menu) fn uses_custom_field_menu_page(
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
                        | FieldMenuScreen::Quests
                        | FieldMenuScreen::Save
                )))
}

pub(in crate::field_menu) fn cleanup_field_menu(
    mut commands: Commands,
    roots: Query<Entity, With<FieldMenuRoot>>,
    mut state: ResMut<FieldMenuState>,
) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    state.close();
}
