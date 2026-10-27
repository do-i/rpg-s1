//! The apothecary's illustrated, live recipe workspace.

use super::*;
use crate::{
    item_images::{ITEM_IMAGE_SIZE, spawn_item_image},
    service_domain::{RecipeAvailability, recipe_availability, recipe_input_requirements},
    service_ui::apothecary::{ApothecaryControl, VISIBLE_RECIPES, recipe_rows},
};

pub(super) fn spawn_header(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    game: &GameState,
    face: Option<DialoguePortrait>,
) {
    parent
        .spawn((
            Node {
                width: percent(100),
                height: px(76),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                column_gap: px(16),
                padding: UiRect::bottom(px(15)),
                margin: UiRect::bottom(px(16)),
                border: UiRect::bottom(px(1)),
                ..default()
            },
            BorderColor::all(status_border()),
            Name::new("Apothecary header"),
        ))
        .with_children(|header| {
            spawn_keeper_frame(header, font, 60.0, face);
            header
                .spawn(Node {
                    width: px(12),
                    height: px(42),
                    flex_shrink: 0.0,
                    ..default()
                })
                .with_children(|bars| spawn_header_bars(bars, 42.0, 0.0));
            header
                .spawn(Node {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    row_gap: px(3),
                    ..default()
                })
                .with_children(|title| {
                    spawn_status_text(title, "APOTHECARY", font, 30.0, status_ink());
                    spawn_status_text(
                        title,
                        "FORMULAE GATHERED, INGREDIENTS WEIGHED",
                        font,
                        11.0,
                        status_muted(),
                    );
                });
            header
                .spawn((
                    Node {
                        padding: UiRect::left(px(28)),
                        border: UiRect::left(px(1)),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::End,
                        row_gap: px(3),
                        ..default()
                    },
                    BorderColor::all(status_border()),
                ))
                .with_children(|wallet| {
                    spawn_status_text(wallet, "YOUR COIN", font, 11.0, status_muted());
                    spawn_status_text(
                        wallet,
                        format!("{} GP", game.repository().gp()),
                        font,
                        26.0,
                        status_gold(),
                    );
                });
        });
}

pub(super) fn spawn_page(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
) {
    let rows = recipe_rows(state, catalog, game);
    parent
        .spawn((
            Node {
                width: percent(100),
                flex_grow: 1.0,
                min_height: px(0),
                column_gap: px(16),
                ..default()
            },
            Name::new("Apothecary workspace"),
        ))
        .with_children(|columns| {
            spawn_panel(
                columns,
                Node {
                    width: px(420),
                    flex_shrink: 0.0,
                    ..default()
                },
                "FORMULAE",
                &format!("{} collected", catalog.recipes().len()),
                font,
                |panel| {
                    spawn_filters(panel, font, state.ready_recipes_only);
                    let selected = rows
                        .iter()
                        .position(|&index| index == state.selected)
                        .unwrap_or(0);
                    let window =
                        state
                            .scroll
                            .window("recipes", selected, rows.len(), VISIBLE_RECIPES);
                    spawn_scroll_list(
                        panel,
                        window,
                        Node {
                            width: percent(100),
                            flex_direction: FlexDirection::Column,
                            flex_grow: 1.0,
                            min_height: px(0),
                            row_gap: px(7),
                            ..default()
                        },
                        |cards| {
                            if rows.is_empty() {
                                spawn_status_text(
                                    cards,
                                    if state.ready_recipes_only {
                                        "No formulae are ready to brew."
                                    } else {
                                        "No recipes are known."
                                    },
                                    font,
                                    17.0,
                                    status_muted(),
                                );
                            }
                            for position in window.indices() {
                                let index = rows[position];
                                let recipe = &catalog.recipes()[index];
                                spawn_recipe_card(
                                    cards,
                                    font,
                                    recipe,
                                    catalog,
                                    recipe_availability(recipe, game.flags(), game.repository()),
                                    index,
                                    index == state.selected,
                                );
                            }
                        },
                    );
                    panel
                        .spawn((
                            Node {
                                width: percent(100),
                                flex_shrink: 0.0,
                                margin: UiRect::top(px(12)),
                                padding: UiRect::top(px(10)),
                                border: UiRect::top(px(1)),
                                justify_content: JustifyContent::SpaceBetween,
                                ..default()
                            },
                            BorderColor::all(status_border()),
                        ))
                        .with_children(|footer| {
                            spawn_status_text(
                                footer,
                                format!(
                                    "{:02} / {} formulae",
                                    if rows.is_empty() { 0 } else { selected + 1 },
                                    rows.len()
                                ),
                                font,
                                11.0,
                                status_muted(),
                            );
                            let shown = window.indices();
                            spawn_status_text(
                                footer,
                                if rows.is_empty() {
                                    "0 shown".to_owned()
                                } else {
                                    format!("{}-{} shown", shown.start + 1, shown.end)
                                },
                                font,
                                11.0,
                                status_muted(),
                            );
                        });
                },
            );
            spawn_panel(
                columns,
                Node {
                    flex_basis: px(0),
                    flex_grow: 1.0,
                    ..default()
                },
                "RECIPE",
                if state.page == Some(ServicePage::RecipeDetail) {
                    "Review before brewing"
                } else {
                    "Updates as you browse"
                },
                font,
                |panel| {
                    let Some(recipe) = rows
                        .iter()
                        .find(|&&index| index == state.selected)
                        .and_then(|&index| catalog.recipes().get(index))
                    else {
                        spawn_status_text(
                            panel,
                            "Select a formula to inspect its recipe.",
                            font,
                            17.0,
                            status_muted(),
                        );
                        return;
                    };
                    spawn_recipe_detail(panel, font, state, recipe, catalog, game);
                },
            );
        });
}

fn spawn_panel(
    parent: &mut ChildSpawnerCommands<'_>,
    node: Node,
    title: &str,
    caption: &str,
    font: &Handle<Font>,
    content: impl FnOnce(&mut ChildSpawnerCommands<'_>),
) {
    parent
        .spawn((
            Node {
                height: percent(100),
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(15)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(5)),
                overflow: Overflow::clip(),
                ..node
            },
            BackgroundColor(Color::srgba_u8(23, 24, 30, 240)),
            BorderColor::all(status_border()),
            Name::new(format!("Apothecary {title}")),
        ))
        .with_children(|panel| {
            panel
                .spawn((
                    Node {
                        width: percent(100),
                        height: px(27),
                        flex_shrink: 0.0,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Start,
                        border: UiRect::bottom(px(1)),
                        padding: UiRect::bottom(px(10)),
                        margin: UiRect::bottom(px(12)),
                        ..default()
                    },
                    BorderColor::all(status_border()),
                ))
                .with_children(|heading| {
                    spawn_status_text(heading, title, font, 12.0, status_gold());
                    spawn_status_text(heading, caption, font, 11.0, status_muted());
                });
            content(panel);
        });
}

fn spawn_filters(parent: &mut ChildSpawnerCommands<'_>, font: &Handle<Font>, ready_only: bool) {
    parent
        .spawn(Node {
            width: percent(100),
            height: px(26),
            flex_shrink: 0.0,
            column_gap: px(6),
            margin: UiRect::bottom(px(12)),
            ..default()
        })
        .with_children(|filters| {
            for (ready, text) in [(false, "All formulae"), (true, "Ready to brew")] {
                let active = ready == ready_only;
                filters
                    .spawn((
                        Node {
                            height: percent(100),
                            padding: UiRect::horizontal(px(12)),
                            align_items: AlignItems::Center,
                            border: UiRect::all(px(1)),
                            border_radius: BorderRadius::all(px(20)),
                            ..default()
                        },
                        selectable_card_colors(active),
                        ApothecaryControl::Filter(ready),
                    ))
                    .with_children(|filter| {
                        spawn_status_text(
                            filter,
                            text,
                            font,
                            12.0,
                            if active {
                                status_gold()
                            } else {
                                status_muted()
                            },
                        )
                    });
            }
        });
}

fn spawn_recipe_card(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    recipe: &RecipeDefinition,
    catalog: &FieldMenuCatalog,
    availability: RecipeAvailability,
    index: usize,
    selected: bool,
) {
    let sealed = availability == RecipeAvailability::Locked;
    parent
        .spawn((
            Node {
                width: percent(100),
                flex_grow: 1.0,
                flex_basis: px(0),
                min_height: px(72),
                max_height: px(100),
                align_items: AlignItems::Center,
                padding: UiRect::axes(
                    px(if selected { 9 } else { 10 }),
                    px(if selected { 7 } else { 8 }),
                ),
                border: UiRect::all(px(if selected { 2 } else { 1 })),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            selectable_card_colors(selected),
            ApothecaryControl::Recipe(index),
            Name::new("Apothecary recipe card"),
        ))
        .with_children(|row| {
            if sealed {
                row.spawn(Node {
                    width: px(56),
                    height: px(56),
                    flex_shrink: 0.0,
                    margin: UiRect::right(px(8)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|placeholder| {
                    spawn_status_text(placeholder, "?", font, 22.0, status_faint())
                });
            } else {
                spawn_item_icon(row, &recipe.output.item, 56.0);
            }
            row.spawn(Node {
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(4),
                ..default()
            })
            .with_children(|copy| {
                spawn_status_text(
                    copy,
                    if sealed {
                        "Sealed formula"
                    } else {
                        &recipe.scroll_name
                    },
                    font,
                    16.0,
                    if sealed { status_muted() } else { status_ink() },
                );
                spawn_status_text(
                    copy,
                    if sealed {
                        "Continue the story".to_owned()
                    } else {
                        format!("{} x{}", output_name(recipe, catalog), recipe.output.qty)
                    },
                    font,
                    12.0,
                    status_muted(),
                );
                spawn_status_text(
                    copy,
                    availability_label(availability),
                    font,
                    11.0,
                    availability_color(availability),
                );
            });
            if !sealed {
                row.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::End,
                    flex_shrink: 0.0,
                    margin: UiRect::left(px(8)),
                    row_gap: px(3),
                    ..default()
                })
                .with_children(|price| {
                    spawn_status_text(price, recipe.gp_cost.to_string(), font, 13.0, status_gold());
                    spawn_status_text(price, "GP", font, 10.0, status_faint());
                });
            }
        });
}

fn output_name<'a>(recipe: &'a RecipeDefinition, catalog: &'a FieldMenuCatalog) -> &'a str {
    catalog
        .item(&recipe.output.item)
        .map_or(recipe.output.item.as_str(), item_name)
}

fn availability_label(availability: RecipeAvailability) -> &'static str {
    match availability {
        RecipeAvailability::Ready => "Ready to brew",
        RecipeAvailability::MissingInputs => "Missing ingredients",
        RecipeAvailability::Unaffordable => "Not enough GP",
        RecipeAvailability::UniqueOwned => "Already carried",
        RecipeAvailability::OutputCap => "Inventory full",
        RecipeAvailability::Locked => "Sealed formula",
    }
}

fn availability_color(availability: RecipeAvailability) -> Color {
    match availability {
        RecipeAvailability::Ready => status_ready(),
        RecipeAvailability::MissingInputs
        | RecipeAvailability::Unaffordable
        | RecipeAvailability::OutputCap => Color::srgb_u8(227, 141, 104),
        RecipeAvailability::UniqueOwned => status_teal(),
        RecipeAvailability::Locked => status_muted(),
    }
}

fn spawn_recipe_detail(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    recipe: &RecipeDefinition,
    catalog: &FieldMenuCatalog,
    game: &GameState,
) {
    let availability = recipe_availability(recipe, game.flags(), game.repository());
    let sealed = availability == RecipeAvailability::Locked;
    parent
        .spawn((
            Node {
                width: percent(100),
                height: px(140),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                column_gap: px(18),
                border: UiRect::bottom(px(1)),
                padding: UiRect::bottom(px(12)),
                ..default()
            },
            BorderColor::all(status_border()),
            Name::new("Apothecary recipe result"),
        ))
        .with_children(|hero| {
            if !sealed {
                spawn_item_image(hero, &recipe.output.item);
            }
            hero.spawn(Node {
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                ..default()
            })
            .with_children(|copy| {
                copy.spawn(Node {
                    width: percent(100),
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: px(12),
                    ..default()
                })
                .with_children(|title| {
                    title
                        .spawn(Node {
                            flex_grow: 1.0,
                            flex_basis: px(0),
                            min_width: px(0),
                            ..default()
                        })
                        .with_children(|name| {
                            spawn_status_text(
                                name,
                                if sealed {
                                    "Sealed formula"
                                } else {
                                    &recipe.scroll_name
                                },
                                font,
                                27.0,
                                if sealed {
                                    status_muted()
                                } else {
                                    status_gold()
                                },
                            );
                        });
                    spawn_pill(title, font, availability);
                });
                if sealed {
                    spawn_status_text(
                        copy,
                        "This formula becomes readable as the story unfolds.",
                        font,
                        14.0,
                        status_muted(),
                    );
                } else {
                    spawn_status_text(
                        copy,
                        format!(
                            "YIELDS    {} x{}",
                            output_name(recipe, catalog),
                            recipe.output.qty
                        ),
                        font,
                        16.0,
                        status_ink(),
                    );
                    if let Some(item) = catalog.item(&recipe.output.item) {
                        spawn_status_text(copy, item_description(item), font, 14.0, status_muted());
                    }
                }
            });
        });
    if !sealed {
        let requirements = recipe_input_requirements(recipe);
        let available = requirements
            .iter()
            .filter(|(id, need)| game.repository().item_count(id) >= *need)
            .count();
        parent
            .spawn(Node {
                width: percent(100),
                justify_content: JustifyContent::SpaceBetween,
                flex_shrink: 0.0,
                margin: UiRect {
                    top: px(17),
                    bottom: px(12),
                    ..default()
                },
                ..default()
            })
            .with_children(|label| {
                spawn_status_text(label, "INGREDIENTS", font, 11.0, status_muted());
                spawn_status_text(
                    label,
                    format!("{available} / {} available", requirements.len()),
                    font,
                    11.0,
                    status_muted(),
                );
            });
        parent
            .spawn(Node {
                width: percent(100),
                display: Display::Grid,
                grid_template_columns: RepeatedGridTrack::flex(4, 1.0),
                column_gap: px(9),
                row_gap: px(9),
                flex_shrink: 0.0,
                ..default()
            })
            .with_children(|grid| {
                for (id, need) in requirements {
                    spawn_ingredient(
                        grid,
                        font,
                        catalog,
                        &id,
                        need,
                        game.repository().item_count(&id),
                    );
                }
            });
    }
    parent.spawn(Node {
        flex_grow: 1.0,
        min_height: px(8),
        ..default()
    });
    spawn_action(parent, font, state, recipe, game, availability);
}

fn spawn_pill(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    availability: RecipeAvailability,
) {
    parent
        .spawn((
            Node {
                padding: UiRect::axes(px(10), px(4)),
                flex_shrink: 0.0,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(20)),
                ..default()
            },
            BorderColor::all(availability_color(availability)),
        ))
        .with_children(|pill| {
            spawn_status_text(
                pill,
                match availability {
                    RecipeAvailability::Ready => "READY",
                    RecipeAvailability::MissingInputs => "SHORT",
                    RecipeAvailability::Unaffordable => "LOW GP",
                    RecipeAvailability::UniqueOwned => "CARRIED",
                    RecipeAvailability::OutputCap => "FULL",
                    RecipeAvailability::Locked => "SEALED",
                },
                font,
                11.0,
                availability_color(availability),
            );
        });
}

fn spawn_ingredient(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    catalog: &FieldMenuCatalog,
    id: &str,
    need: u32,
    owned: u32,
) {
    parent
        .spawn((
            Node {
                min_width: px(ITEM_IMAGE_SIZE + 10.0),
                height: px(214),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(4), px(10)),
                row_gap: px(6),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(Color::srgb_u8(16, 17, 22)),
            BorderColor::all(status_border()),
            Name::new(format!("Apothecary ingredient: {id}")),
        ))
        .with_children(|card| {
            spawn_item_image(card, id);
            spawn_status_text(
                card,
                catalog.item(id).map_or(id, item_name),
                font,
                14.0,
                status_ink(),
            );
            card.spawn(Node {
                column_gap: px(9),
                ..default()
            })
            .with_children(|counts| {
                spawn_status_text(counts, format!("Need {need}"), font, 12.0, status_muted());
                spawn_status_text(
                    counts,
                    format!("Have {owned}"),
                    font,
                    12.0,
                    if owned >= need {
                        status_ready()
                    } else {
                        availability_color(RecipeAvailability::MissingInputs)
                    },
                );
            });
            spawn_status_text(
                card,
                if owned < need {
                    format!("Missing {}", need - owned)
                } else {
                    String::new()
                },
                font,
                11.0,
                availability_color(RecipeAvailability::MissingInputs),
            );
        });
}

fn spawn_action(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
    recipe: &RecipeDefinition,
    game: &GameState,
    availability: RecipeAvailability,
) {
    let ready = availability == RecipeAvailability::Ready;
    let sealed = availability == RecipeAvailability::Locked;
    parent
        .spawn((
            Node {
                width: percent(100),
                min_height: px(62),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                column_gap: px(15),
                border: UiRect::top(px(1)),
                padding: UiRect::top(px(14)),
                ..default()
            },
            BorderColor::all(status_border()),
            Name::new("Apothecary brew action"),
        ))
        .with_children(|action| {
            action
                .spawn(Node {
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    min_width: px(0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4),
                    ..default()
                })
                .with_children(|copy| {
                    spawn_status_text(
                        copy,
                        if sealed {
                            "Formula sealed".to_owned()
                        } else {
                            format!("{} GP per brew", recipe.gp_cost)
                        },
                        font,
                        21.0,
                        status_gold(),
                    );
                    let (message, accent) = if let Some(toast) = state.toast.as_ref() {
                        (
                            toast.text.clone(),
                            match toast.tone {
                                ToastTone::Good => status_ready(),
                                ToastTone::Warn => {
                                    availability_color(RecipeAvailability::MissingInputs)
                                }
                            },
                        )
                    } else {
                        (
                            match availability {
                                RecipeAvailability::Ready
                                    if state.page == Some(ServicePage::RecipeDetail) =>
                                {
                                    "Confirm to spend these ingredients and GP.".to_owned()
                                }
                                RecipeAvailability::Ready => format!(
                                    "Ingredients ready - {} GP after brewing",
                                    game.repository().gp().saturating_sub(recipe.gp_cost)
                                ),
                                RecipeAvailability::MissingInputs => {
                                    "Gather the missing ingredients to brew.".to_owned()
                                }
                                RecipeAvailability::Unaffordable => {
                                    "Gather more GP to brew this formula.".to_owned()
                                }
                                RecipeAvailability::UniqueOwned => {
                                    "Only one can be carried at a time.".to_owned()
                                }
                                RecipeAvailability::OutputCap => {
                                    "Make room for the full batch before brewing.".to_owned()
                                }
                                RecipeAvailability::Locked => {
                                    "Continue the story to reveal this formula.".to_owned()
                                }
                            },
                            availability_color(availability),
                        )
                    };
                    spawn_status_text(copy, message, font, 13.0, accent);
                });
            let mut button = action.spawn((
                Node {
                    width: px(190),
                    height: px(46),
                    flex_shrink: 0.0,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(4)),
                    ..default()
                },
                selectable_card_colors(ready),
                Name::new("Apothecary brew button"),
            ));
            if ready {
                button.insert(ApothecaryControl::Brew);
            }
            button.with_children(|button| {
                spawn_status_text(
                    button,
                    if ready {
                        if state.page == Some(ServicePage::RecipeDetail) {
                            "ENTER  Confirm brew"
                        } else {
                            "ENTER  Review brew"
                        }
                    } else {
                        match availability {
                            RecipeAvailability::MissingInputs => "Ingredients needed",
                            RecipeAvailability::Unaffordable => "Not enough GP",
                            RecipeAvailability::UniqueOwned => "Already carried",
                            RecipeAvailability::OutputCap => "Inventory full",
                            _ => "Formula sealed",
                        }
                    },
                    font,
                    15.0,
                    if ready { status_ink() } else { status_muted() },
                )
            });
        });
}

pub(super) fn spawn_footer(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &ServiceUiState,
) {
    parent.spawn(Node { width: percent(100), margin: UiRect::top(px(14)), height: px(20), flex_shrink: 0.0, align_items: AlignItems::Center, ..default() })
        .with_children(|footer| spawn_status_text(footer, if state.page == Some(ServicePage::RecipeDetail) {
            "ENTER  confirm brew     ESC  cancel review     UP/DOWN  browse"
        } else { "UP/DOWN  browse recipes     TAB / LEFT/RIGHT  filter     ENTER  review brew     ESC  leave shop" }, font, 12.0, status_muted()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_menu_domain::tests::{catalog, game};

    fn rendered(game: GameState, selected: usize) -> World {
        let mut app = App::new();
        app.insert_resource(game)
            .insert_resource(catalog())
            .insert_resource(ServiceUiState {
                selected,
                page: Some(ServicePage::Apothecary),
                ..default()
            })
            .add_systems(
                Update,
                |mut commands: Commands,
                 game: Res<GameState>,
                 catalog: Res<FieldMenuCatalog>,
                 state: Res<ServiceUiState>| {
                    commands.spawn_empty().with_children(|parent| {
                        spawn_page(parent, &Handle::default(), &state, &catalog, &game)
                    });
                },
            );
        app.update();
        std::mem::replace(app.world_mut(), World::new())
    }

    #[test]
    fn sealed_formulae_hide_output_requirements_and_prices_without_lock_art() {
        let mut world = rendered(game([]), 10);
        let text = world
            .query::<&Text>()
            .iter(&world)
            .map(|text| text.0.as_str())
            .collect::<Vec<_>>();
        assert!(text.contains(&"Sealed formula"));
        for hidden in [
            "Grand Elixir",
            "Elixir",
            "Rare Herb",
            "Soul Fragment",
            "800 GP per brew",
            "INGREDIENTS",
        ] {
            assert!(!text.contains(&hidden), "leaked sealed content: {hidden}");
        }
        assert!(
            !world
                .query::<&Name>()
                .iter(&world)
                .any(|name| name.as_str().starts_with("Item image:"))
        );
        assert!(
            !world
                .query::<&ApothecaryControl>()
                .iter(&world)
                .any(|control| *control == ApothecaryControl::Brew)
        );
        assert_eq!(
            world.query::<&ImageNode>().iter(&world).count(),
            0,
            "no recipe status icon is rendered"
        );
    }

    #[test]
    fn browsing_renders_the_selected_formula_and_four_native_size_ingredients() {
        let flags = [
            "story_quest_started",
            "story_act2_started",
            "story_act3_started",
            "story_act4_started",
        ];
        let mut world = rendered(game(flags), 10);
        let text = world
            .query::<&Text>()
            .iter(&world)
            .map(|text| text.0.clone())
            .collect::<Vec<_>>();
        for expected in [
            "Grand Elixir",
            "YIELDS    Elixir x1",
            "INGREDIENTS",
            "800 GP per brew",
            "Ingredients needed",
        ] {
            assert!(
                text.iter().any(|label| label == expected),
                "missing {expected}"
            );
        }
        let artwork = world
            .query::<(&Name, &Node)>()
            .iter(&world)
            .filter(|(name, node)| {
                name.as_str().starts_with("Item image:")
                    && node.width == px(128)
                    && node.height == px(128)
            })
            .map(|(name, _)| name.as_str().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            artwork.len(),
            5,
            "one output and every ingredient use 100% artwork"
        );
        for id in ["elixir", "rare_herb", "soul_fragment", "mc_m", "mc_l"] {
            assert!(artwork.contains(&format!("Item image: {id}")));
        }
        let mut next = rendered(game(flags), 2);
        assert!(
            next.query::<&Text>()
                .iter(&next)
                .any(|text| text.0 == "YIELDS    Hi-Potion x2")
        );
        assert!(
            !next
                .query::<&Name>()
                .iter(&next)
                .any(|name| name.as_str() == "Apothecary ingredient: soul_fragment")
        );
    }
}
