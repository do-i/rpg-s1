//! The field recipe book (roadmap B4.4).
//!
//! A reference screen, not a workshop: it reads the production recipe catalog and the live
//! inventory, and offers no way to spend anything. Crafting stays with the apothecary service,
//! so this page shares that service's availability rules rather than inventing its own.

use super::*;
use crate::item_images::{ITEM_IMAGE_SIZE, spawn_item_icon, spawn_item_image};

#[expect(
    clippy::too_many_arguments,
    reason = "the recipe book coordinates the shared menu root, recipe catalog, and live inventory"
)]
pub(in crate::field_menu) fn sync_recipes_page(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
    state: Res<FieldMenuState>,
    catalog: Res<FieldMenuCatalog>,
    game: Option<Res<GameState>>,
    menu_roots: Query<Entity, With<FieldMenuRoot>>,
    pages: Query<Entity, With<FieldMenuRecipesPage>>,
) {
    let show_recipes = state.open
        && state.screen == FieldMenuScreen::Recipes
        && catalog.status() == CatalogStatus::Ready
        && game.is_some();
    if !show_recipes {
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
        spawn_recipes_page(parent, &font, &state, &game, &catalog);
    });
}

pub(in crate::field_menu) fn spawn_recipes_page(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
) {
    let recipes = catalog.recipes();
    let selected = selected_recipe_index(state, recipes);
    parent
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(14),
                ..default()
            },
            FieldMenuRecipesPage,
            Name::new("Recipes page"),
        ))
        .with_children(|page| {
            spawn_recipes_header(page, font, game, recipes);
            page.spawn(Node {
                width: percent(100),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Row,
                column_gap: px(RECIPE_COLUMN_GAP),
                ..default()
            })
            .with_children(|columns| {
                spawn_recipe_list_column(columns, font, state, game, catalog, recipes, selected);
                spawn_recipe_detail_column(columns, font, game, catalog, recipes.get(selected));
            });
            spawn_status_text(
                page,
                "UP/DOWN   SELECT RECIPE      ESC   BACK      M   CLOSE",
                font,
                15.0,
                status_muted(),
            );
            if !state.message.is_empty() {
                spawn_items_message(page, font, &state.message);
            }
        });
}

/// How many recipes are readable, brewable now, and still sealed.
pub(in crate::field_menu) struct RecipeTally {
    pub(in crate::field_menu) ready: usize,
    pub(in crate::field_menu) sealed: usize,
}

impl RecipeTally {
    pub(in crate::field_menu) fn of(recipes: &[RecipeDefinition], game: &GameState) -> Self {
        let mut tally = Self {
            ready: 0,
            sealed: 0,
        };
        for recipe in recipes {
            match recipe_availability(recipe, game.flags(), game.repository()) {
                RecipeAvailability::Ready => tally.ready += 1,
                RecipeAvailability::Locked => tally.sealed += 1,
                _ => {}
            }
        }
        tally
    }
}

pub(in crate::field_menu) fn spawn_recipes_header(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    game: &GameState,
    recipes: &[RecipeDefinition],
) {
    let tally = RecipeTally::of(recipes, game);
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
                    spawn_status_text(title, "RECIPE BOOK", font, 31.0, status_ink());
                    spawn_status_text(
                        title,
                        "FORMULAE GATHERED, INGREDIENTS WEIGHED",
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
                    spawn_quest_counter(counters, font, "READY", tally.ready, status_ready());
                    spawn_quest_counter(counters, font, "SEALED", tally.sealed, status_faint());
                    spawn_quest_counter(
                        counters,
                        font,
                        "STUDIED",
                        recipes.len(),
                        Color::srgb_u8(157, 139, 101),
                    );
                });
        });
}

pub(in crate::field_menu) fn spawn_recipe_list_column(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
    recipes: &[RecipeDefinition],
    selected: usize,
) {
    spawn_status_panel(
        parent,
        Node {
            width: px(RECIPE_LIST_WIDTH),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(7),
            padding: UiRect::all(px(16)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(6)),
            overflow: Overflow::clip(),
            ..default()
        },
        "FORMULAE",
        font,
        |panel| {
            if recipes.is_empty() {
                spawn_status_text(panel, "No recipes are known.", font, 18.0, status_muted());
                spawn_status_text(
                    panel,
                    "Scrolls copied into this book appear here.",
                    font,
                    14.0,
                    status_faint(),
                );
                return;
            }
            let window =
                state
                    .scroll
                    .window("recipes", selected, recipes.len(), RECIPE_VISIBLE_ROWS);
            // The cards share the column's height, so a full window fills it edge to edge.
            let cards_node = Node {
                width: percent(100),
                flex_grow: 1.0,
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(7),
                ..default()
            };
            spawn_scroll_list(panel, window, cards_node, |cards| {
                for index in window.indices() {
                    let recipe = &recipes[index];
                    spawn_recipe_book_row(cards, font, recipe, game, catalog, index == selected);
                }
            });
            spawn_panel_rule(panel);
            spawn_status_text(
                panel,
                register_footer(
                    "RECIPE",
                    selected,
                    window.first,
                    recipes.len(),
                    RECIPE_VISIBLE_ROWS,
                ),
                font,
                11.0,
                status_faint(),
            );
        },
    );
}

pub(in crate::field_menu) fn spawn_recipe_book_row(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    recipe: &RecipeDefinition,
    game: &GameState,
    catalog: &FieldMenuCatalog,
    selected: bool,
) {
    let availability = recipe_availability(recipe, game.flags(), game.repository());
    let sealed = availability == RecipeAvailability::Locked;
    let mut row = parent.spawn((
        Node {
            width: percent(100),
            // Equal shares of the list's height, between the old fixed size and a cap that
            // stops a short list from ballooning its few cards.
            flex_grow: 1.0,
            flex_basis: px(0),
            min_height: px(REGISTER_CARD_MIN_HEIGHT),
            max_height: px(REGISTER_CARD_MAX_HEIGHT),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            padding: UiRect::axes(px(10), px(6)),
            border: UiRect::all(px(if selected { 2 } else { 1 })),
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        selectable_card_colors(selected),
        RecipeBookRow,
    ));
    if selected {
        row.insert(SelectedRecipeBookRow);
    }
    row.with_children(|row| {
        if !sealed {
            spawn_item_icon(row, &recipe.output.item, 32.0);
        }
        row.spawn((
            Node {
                width: px(6),
                height: px(34),
                margin: UiRect::right(px(10)),
                ..default()
            },
            BackgroundColor(recipe_availability_color(availability)),
        ));
        row.spawn(Node {
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|copy| {
            spawn_status_text(
                copy,
                recipe_title(recipe, sealed),
                font,
                17.0,
                if sealed { status_muted() } else { status_ink() },
            );
            spawn_status_text(
                copy,
                recipe_subtitle(recipe, catalog, sealed),
                font,
                12.0,
                status_faint(),
            );
        });
        spawn_recipe_state_pill(row, font, availability);
    });
}

pub(in crate::field_menu) fn spawn_recipe_state_pill(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    availability: RecipeAvailability,
) {
    parent
        .spawn((
            Node {
                min_width: px(104),
                justify_content: JustifyContent::Center,
                padding: UiRect::axes(px(8), px(3)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(9)),
                ..default()
            },
            BackgroundColor(Color::srgba_u8(10, 10, 14, 170)),
            BorderColor::all(recipe_availability_color(availability)),
        ))
        .with_children(|pill| {
            spawn_status_text(
                pill,
                recipe_state_label(availability),
                font,
                11.0,
                recipe_availability_color(availability),
            );
        });
}

pub(in crate::field_menu) fn spawn_recipe_detail_column(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    game: &GameState,
    catalog: &FieldMenuCatalog,
    recipe: Option<&RecipeDefinition>,
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
        "FORMULA",
        font,
        |panel| {
            let Some(recipe) = recipe else {
                spawn_status_text(
                    panel,
                    "Select a recipe to read its formula.",
                    font,
                    16.0,
                    status_muted(),
                );
                return;
            };
            let availability = recipe_availability(recipe, game.flags(), game.repository());
            let sealed = availability == RecipeAvailability::Locked;
            panel
                .spawn((
                    Node {
                        width: percent(100),
                        height: px(ITEM_IMAGE_SIZE),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Start,
                        column_gap: px(16),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    Name::new("Recipe formula header"),
                ))
                .with_children(|heading| {
                    if !sealed {
                        spawn_item_image(heading, &recipe.output.item);
                    }
                    heading
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Start,
                            flex_basis: px(0),
                            flex_grow: 1.0,
                            min_width: px(0),
                            height: percent(100),
                            row_gap: px(12),
                            ..default()
                        })
                        .with_children(|meta| {
                            meta.spawn(Node {
                                width: percent(100),
                                align_items: AlignItems::Center,
                                column_gap: px(12),
                                flex_shrink: 0.0,
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
                                            recipe_title(recipe, sealed),
                                            font,
                                            24.0,
                                            if sealed {
                                                status_faint()
                                            } else {
                                                status_gold()
                                            },
                                        );
                                    });
                                spawn_recipe_state_pill(title, font, availability);
                            });
                            if !sealed {
                                meta.spawn(Node {
                                    width: percent(100),
                                    display: Display::Grid,
                                    grid_template_columns: RepeatedGridTrack::flex(2, 1.0),
                                    column_gap: px(16),
                                    row_gap: px(8),
                                    flex_shrink: 0.0,
                                    ..default()
                                })
                                .with_children(|values| {
                                    spawn_quest_meta_row(
                                        values,
                                        font,
                                        "YIELDS",
                                        &format!(
                                            "{} x{}",
                                            loot_item_name(catalog, &recipe.output.item),
                                            recipe.output.qty
                                        ),
                                    );
                                    spawn_quest_meta_row(
                                        values,
                                        font,
                                        "COST",
                                        &format!("{} GP", recipe.gp_cost),
                                    );
                                    if recipe.unique_output {
                                        spawn_quest_meta_row(
                                            values,
                                            font,
                                            "LIMIT",
                                            "one at a time",
                                        );
                                    }
                                });
                            }
                        });
                });
            spawn_section_rule(panel);
            if sealed {
                // A sealed formula is listed so the count is honest, but its ingredients stay
                // unreadable until the story unseals it.
                spawn_status_text(
                    panel,
                    "The ink has not settled. This formula is still sealed.",
                    font,
                    16.0,
                    status_muted(),
                );
                spawn_recipe_footnote(panel, font);
                return;
            }
            spawn_status_text(panel, "INGREDIENTS", font, 12.0, status_muted());
            let requirements = recipe_input_requirements(recipe);
            if requirements.is_empty() {
                spawn_status_text(panel, "Needs nothing but coin.", font, 15.0, status_faint());
            }
            if !requirements.is_empty() {
                panel
                    .spawn((
                        Node {
                            width: percent(100),
                            flex_direction: FlexDirection::Row,
                            flex_wrap: FlexWrap::Wrap,
                            align_items: AlignItems::Start,
                            align_content: AlignContent::Start,
                            column_gap: px(10),
                            row_gap: px(10),
                            flex_shrink: 0.0,
                            ..default()
                        },
                        Name::new("Recipe ingredients"),
                    ))
                    .with_children(|grid| {
                        for (id, required) in requirements {
                            let owned = game.repository().item_count(&id);
                            spawn_recipe_ingredient_card(
                                grid,
                                font,
                                &id,
                                &loot_item_name(catalog, &id),
                                required,
                                owned,
                            );
                        }
                    });
            }
            panel.spawn(Node {
                flex_grow: 1.0,
                ..default()
            });
            spawn_status_text(
                panel,
                recipe_state_hint(availability),
                font,
                14.0,
                recipe_availability_color(availability),
            );
            spawn_section_rule(panel);
            spawn_recipe_footnote(panel, font);
        },
    );
}

/// A full-size ingredient image with its name and required/carried counts underneath.
fn spawn_recipe_ingredient_card(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    id: &str,
    name: &str,
    required: u32,
    owned: u32,
) {
    let accent = if owned >= required {
        status_ink()
    } else {
        status_ember()
    };
    parent
        .spawn((
            Node {
                width: px(148),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(5),
                padding: UiRect::all(px(8)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(Color::srgba_u8(10, 10, 14, 148)),
            BorderColor::all(Color::srgba_u8(126, 98, 55, 90)),
            Name::new(format!("Recipe ingredient: {id}")),
        ))
        .with_children(|row| {
            spawn_item_image(row, id);
            spawn_status_text(row, name, font, 14.0, accent);
            row.spawn(Node {
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::Center,
                column_gap: px(8),
                row_gap: px(3),
                ..default()
            })
            .with_children(|counts| {
                spawn_status_text(
                    counts,
                    format!("NEED {required}"),
                    font,
                    11.0,
                    status_muted(),
                );
                spawn_status_text(counts, format!("HAVE {owned}"), font, 11.0, accent);
            });
        });
}

/// The line that keeps the book honest about being a reference rather than a workshop.
fn spawn_recipe_footnote(parent: &mut ChildSpawnerCommands<'_>, font: &Handle<Font>) {
    spawn_status_text(
        parent,
        "Brewing is done at an apothecary.",
        font,
        13.0,
        status_faint(),
    );
}

/// The display name for a catalog item id, falling back to the raw id.
fn loot_item_name(catalog: &FieldMenuCatalog, id: &str) -> String {
    catalog
        .item(id)
        .map_or_else(|| id.to_owned(), |item| item_name(item).to_owned())
}

fn recipe_title(recipe: &RecipeDefinition, sealed: bool) -> String {
    if sealed {
        "Sealed Formula".to_owned()
    } else {
        recipe.scroll_name.clone()
    }
}

fn recipe_subtitle(recipe: &RecipeDefinition, catalog: &FieldMenuCatalog, sealed: bool) -> String {
    if sealed {
        "UNREAD".to_owned()
    } else {
        format!(
            "{}   ·   {} GP",
            loot_item_name(catalog, &recipe.output.item).to_uppercase(),
            recipe.gp_cost
        )
    }
}

pub(in crate::field_menu) const fn recipe_state_label(
    availability: RecipeAvailability,
) -> &'static str {
    match availability {
        RecipeAvailability::Ready => "READY",
        RecipeAvailability::MissingInputs => "SHORT",
        RecipeAvailability::Unaffordable => "COSTLY",
        RecipeAvailability::UniqueOwned => "CARRIED",
        RecipeAvailability::OutputCap => "FULL",
        RecipeAvailability::Locked => "SEALED",
    }
}

fn recipe_state_hint(availability: RecipeAvailability) -> &'static str {
    match availability {
        RecipeAvailability::Ready => "Ingredients and coin are on hand.",
        RecipeAvailability::MissingInputs => "Some ingredients are missing.",
        RecipeAvailability::Unaffordable => "Not enough coin to brew.",
        RecipeAvailability::UniqueOwned => "One is already carried.",
        RecipeAvailability::OutputCap => "No room to carry another.",
        RecipeAvailability::Locked => "Not yet unsealed.",
    }
}

pub(in crate::field_menu) fn recipe_availability_color(availability: RecipeAvailability) -> Color {
    match availability {
        RecipeAvailability::Ready => status_ready(),
        RecipeAvailability::MissingInputs | RecipeAvailability::Unaffordable => status_ember(),
        RecipeAvailability::UniqueOwned => status_teal(),
        RecipeAvailability::Locked | RecipeAvailability::OutputCap => status_faint(),
    }
}

pub(in crate::field_menu) fn selected_recipe_index(
    state: &FieldMenuState,
    recipes: &[RecipeDefinition],
) -> usize {
    if recipes.is_empty() {
        0
    } else {
        state.selected.min(recipes.len() - 1)
    }
}
