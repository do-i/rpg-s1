//! The field recipe book (roadmap B4.4).
//!
//! A reference screen, not a workshop: it reads the production recipe catalog and the live
//! inventory, and offers no way to spend anything. Crafting stays with the apothecary service,
//! so this page shares that service's availability rules rather than inventing its own.

use super::*;

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
                spawn_recipe_list_column(columns, font, game, catalog, recipes, selected);
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
            let first = recipe_window_start(selected, recipes.len());
            for (index, recipe) in recipes
                .iter()
                .enumerate()
                .skip(first)
                .take(RECIPE_VISIBLE_ROWS)
            {
                spawn_recipe_book_row(panel, font, recipe, game, catalog, index == selected);
            }
            panel.spawn(Node {
                flex_grow: 1.0,
                ..default()
            });
            spawn_status_text(
                panel,
                register_footer(selected, first, recipes.len()),
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
        RecipeBookRow,
    ));
    if selected {
        row.insert(SelectedRecipeBookRow);
    }
    row.with_children(|row| {
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
            spawn_status_text(
                panel,
                recipe_title(recipe, sealed),
                font,
                24.0,
                if sealed {
                    status_faint()
                } else {
                    status_gold()
                },
            );
            spawn_recipe_state_pill(panel, font, availability);
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
            spawn_quest_meta_row(
                panel,
                font,
                "YIELDS",
                &format!(
                    "{} x{}",
                    loot_item_name(catalog, &recipe.output.item),
                    recipe.output.qty
                ),
            );
            spawn_quest_meta_row(panel, font, "COST", &format!("{} GP", recipe.gp_cost));
            if recipe.unique_output {
                spawn_quest_meta_row(panel, font, "LIMIT", "one at a time");
            }
            spawn_section_rule(panel);
            spawn_status_text(panel, "INGREDIENTS", font, 12.0, status_muted());
            let requirements = recipe_input_requirements(recipe);
            if requirements.is_empty() {
                spawn_status_text(panel, "Needs nothing but coin.", font, 15.0, status_faint());
            }
            for (id, required) in requirements {
                let owned = game.repository().item_count(&id);
                spawn_status_text(
                    panel,
                    format!(
                        "{}   x{required}   (carried {owned})",
                        loot_item_name(catalog, &id)
                    ),
                    font,
                    15.0,
                    if owned >= required {
                        status_ink()
                    } else {
                        status_ember()
                    },
                );
            }
            panel.spawn(Node {
                flex_grow: 1.0,
                ..default()
            });
            spawn_recipe_footnote(panel, font);
        },
    );
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

pub(in crate::field_menu) fn recipe_window_start(selected: usize, len: usize) -> usize {
    window_start(selected, len, RECIPE_VISIBLE_ROWS)
}
