//! Recipe filtering and the review/commit boundary for brewing.

use super::*;

pub(super) const VISIBLE_RECIPES: usize = 5;

#[derive(Clone, Copy, Component, Debug, Eq, PartialEq)]
pub(crate) enum ApothecaryControl {
    Recipe(usize),
    Filter(bool),
    Brew,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        field_menu_domain::tests::{catalog, game},
        input_record::NormalizedAction,
    };

    fn ready_game() -> GameState {
        let mut game = game([
            "story_quest_started",
            "story_act2_started",
            "story_act3_started",
            "story_act4_started",
        ]);
        let _ = game.repository_mut().add_gp(10_000).unwrap();
        for (id, qty) in [
            ("venom_sac", 2),
            ("herb_red", 1),
            ("mc_m", 1),
            ("mana_droplet", 1),
        ] {
            let _ = game.repository_mut().add_item(id, qty).unwrap();
        }
        game
    }

    fn app(game: GameState) -> App {
        let mut state = ServiceUiState::default();
        state.open(ServiceRequest::Apothecary, None);
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ActionState>()
            .init_resource::<EngineSettings>()
            .insert_resource(catalog())
            .insert_resource(game)
            .insert_resource(state)
            .add_message::<PlaySfx>()
            .add_systems(PreUpdate, super::reset_service_input_lock)
            .add_systems(Update, handle_service_input);
        app.update();
        app
    }

    fn press(app: &mut App, actions: &[NormalizedAction], key: Option<KeyCode>) {
        app.world_mut()
            .resource_mut::<ActionState>()
            .replace_with_normalized(actions);
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.reset_all();
        if let Some(key) = key {
            keys.press(key);
        }
        app.update();
    }

    #[test]
    fn browsing_review_and_cancelling_never_spend_ingredients_or_gp() {
        let game = ready_game();
        let before = (
            game.repository().gp(),
            game.repository().item_count("venom_sac"),
        );
        let mut app = app(game);
        press(&mut app, &[NormalizedAction::Confirm], None);
        assert_eq!(
            app.world().resource::<ServiceUiState>().page,
            Some(ServicePage::RecipeDetail)
        );
        // Navigating while reviewing cancels that review and shows the newly selected recipe.
        press(
            &mut app,
            &[NormalizedAction::MenuDown, NormalizedAction::Confirm],
            None,
        );
        let state = app.world().resource::<ServiceUiState>();
        assert_eq!(state.selected, 1);
        assert_eq!(state.page, Some(ServicePage::Apothecary));
        press(&mut app, &[NormalizedAction::Confirm], None);
        press(&mut app, &[NormalizedAction::Back], None);
        assert_eq!(
            app.world().resource::<ServiceUiState>().page,
            Some(ServicePage::Apothecary)
        );
        let game = app.world().resource::<GameState>();
        assert_eq!(
            (
                game.repository().gp(),
                game.repository().item_count("venom_sac")
            ),
            before
        );
        assert_eq!(game.repository().item_count("remedy"), 0);
        assert_eq!(game.repository().item_count("ether"), 0);
    }

    #[test]
    fn ready_filter_preserves_catalog_identity_and_refreshes_after_brewing() {
        let mut app = app(ready_game());
        press(&mut app, &[], Some(KeyCode::Tab));
        let state = app.world().resource::<ServiceUiState>();
        assert!(state.ready_recipes_only);
        assert_eq!(
            recipe_rows(state, app.world().resource(), app.world().resource()),
            [0, 1]
        );
        let before_gp = app.world().resource::<GameState>().repository().gp();
        press(&mut app, &[NormalizedAction::Confirm], None);
        assert_eq!(
            app.world().resource::<GameState>().repository().gp(),
            before_gp
        );
        press(&mut app, &[NormalizedAction::Confirm], None);
        let game = app.world().resource::<GameState>();
        assert_eq!(game.repository().gp(), before_gp - 80);
        assert_eq!(game.repository().item_count("remedy"), 1);
        assert_eq!(game.repository().item_count("venom_sac"), 0);
        let state = app.world().resource::<ServiceUiState>();
        assert_eq!(state.selected, 1);
        assert_eq!(recipe_rows(state, app.world().resource(), game), [1]);
        // The selected visible row is catalog recipe 1, not filtered position 0.
        press(&mut app, &[NormalizedAction::Confirm], None);
        press(&mut app, &[NormalizedAction::Confirm], None);
        assert_eq!(
            app.world()
                .resource::<GameState>()
                .repository()
                .item_count("ether"),
            2
        );
        let state = app.world().resource::<ServiceUiState>();
        assert!(recipe_rows(state, app.world().resource(), app.world().resource()).is_empty());
        press(
            &mut app,
            &[NormalizedAction::Confirm, NormalizedAction::MenuDown],
            None,
        );
        assert_eq!(
            app.world().resource::<ServiceUiState>().page,
            Some(ServicePage::Apothecary)
        );
        press(&mut app, &[], Some(KeyCode::Tab));
        assert!(!app.world().resource::<ServiceUiState>().ready_recipes_only);
    }

    #[test]
    fn unavailable_recipes_cannot_enter_review_or_commit() {
        for mut game in [game([]), game(["story_quest_started"]), ready_game()] {
            if game.repository().item_count("venom_sac") > 0 {
                let gp = game.repository().gp();
                game.repository_mut().spend_gp(gp).unwrap();
            }
            let mut app = app(game);
            let before = app.world().resource::<GameState>().repository().gp();
            press(&mut app, &[NormalizedAction::Confirm], None);
            press(&mut app, &[NormalizedAction::Confirm], None);
            assert_eq!(
                app.world().resource::<ServiceUiState>().page,
                Some(ServicePage::Apothecary)
            );
            assert_eq!(
                app.world().resource::<GameState>().repository().gp(),
                before
            );
            assert_eq!(
                app.world()
                    .resource::<GameState>()
                    .repository()
                    .item_count("remedy"),
                0
            );
        }
    }

    #[test]
    fn changed_inventory_is_revalidated_at_confirmation() {
        let mut app = app(ready_game());
        let before = app.world().resource::<GameState>().repository().gp();
        press(&mut app, &[NormalizedAction::Confirm], None);
        app.world_mut()
            .resource_mut::<GameState>()
            .repository_mut()
            .remove_item("venom_sac", 2)
            .unwrap();
        press(&mut app, &[NormalizedAction::Confirm], None);
        assert_eq!(
            app.world().resource::<GameState>().repository().gp(),
            before
        );
        assert_eq!(
            app.world()
                .resource::<GameState>()
                .repository()
                .item_count("remedy"),
            0
        );
        assert_eq!(
            app.world().resource::<ServiceUiState>().page,
            Some(ServicePage::Apothecary)
        );
    }

    #[test]
    fn pointer_controls_select_filter_and_review_before_committing() {
        fn click(app: &mut App, control: ApothecaryControl) {
            let entity = app
                .world_mut()
                .spawn((
                    control,
                    ComputedNode {
                        size: Vec2::splat(100.0),
                        ..default()
                    },
                    UiGlobalTransform::default(),
                ))
                .id();
            app.insert_resource(CanvasPointer::at_viewport_position(Vec2::ZERO, true));
            app.update();
            app.world_mut().despawn(entity);
        }
        let mut app = app(ready_game());
        let gp = app.world().resource::<GameState>().repository().gp();
        click(&mut app, ApothecaryControl::Recipe(1));
        assert_eq!(app.world().resource::<ServiceUiState>().selected, 1);
        click(&mut app, ApothecaryControl::Filter(true));
        assert!(app.world().resource::<ServiceUiState>().ready_recipes_only);
        click(&mut app, ApothecaryControl::Brew);
        assert_eq!(
            app.world().resource::<ServiceUiState>().page,
            Some(ServicePage::RecipeDetail)
        );
        assert_eq!(app.world().resource::<GameState>().repository().gp(), gp);
        assert!(
            !app.world().resource::<CanvasPointer>().just_clicked(),
            "service clicks must be consumed"
        );
        click(&mut app, ApothecaryControl::Brew);
        assert_eq!(
            app.world().resource::<GameState>().repository().gp(),
            gp - 100
        );
        assert_eq!(
            app.world()
                .resource::<GameState>()
                .repository()
                .item_count("ether"),
            2
        );
    }
}

#[derive(Default)]
pub(super) struct ApothecaryInput {
    pub(super) delta: Option<isize>,
    pub(super) filter: Option<bool>,
    pub(super) recipe: Option<usize>,
    pub(super) confirm: bool,
    pub(super) back: bool,
}

pub(super) fn recipe_rows(
    state: &ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
) -> Vec<usize> {
    catalog
        .recipes()
        .iter()
        .enumerate()
        .filter(|(_, recipe)| {
            !state.ready_recipes_only
                || recipe_availability(recipe, game.flags(), game.repository())
                    == RecipeAvailability::Ready
        })
        .map(|(index, _)| index)
        .collect()
}

pub(super) fn handle_apothecary_input(
    state: &mut ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &mut GameState,
    menu_sfx: &mut MenuSfx,
    input: ApothecaryInput,
) {
    if input.back {
        menu_sfx.cancel();
        if state.page == Some(ServicePage::RecipeDetail) {
            state.page = Some(ServicePage::Apothecary);
        } else {
            state.close();
        }
        return;
    }
    let changing_filter = input
        .filter
        .is_some_and(|ready| ready != state.ready_recipes_only);
    if let Some(ready) = input.filter {
        state.ready_recipes_only = ready;
    }
    let rows = recipe_rows(state, catalog, game);
    let old_selected = state.selected;
    if !rows.contains(&state.selected) {
        state.selected = rows.first().copied().unwrap_or_default();
    }
    if let Some(index) = input.recipe.filter(|index| rows.contains(index)) {
        state.selected = index;
    }
    if let Some(delta) = input.delta {
        let position = rows
            .iter()
            .position(|&index| index == state.selected)
            .unwrap_or(0);
        state.selected = rows
            .get(wrapped_or_zero(position, rows.len(), delta))
            .copied()
            .unwrap_or_default();
    }
    if changing_filter
        || input.recipe.is_some()
        || input.delta.is_some()
        || old_selected != state.selected
    {
        // Navigation always leaves review. A selection change can never commit a brew.
        state.page = Some(ServicePage::Apothecary);
        if changing_filter || input.recipe.is_some() {
            menu_sfx.hover();
        }
        return;
    }
    if !input.confirm {
        return;
    }
    let Some(recipe) = rows
        .iter()
        .find(|&&index| index == state.selected)
        .and_then(|&index| catalog.recipes().get(index))
    else {
        menu_sfx.blocked();
        state.announce("No formulae are ready to brew.", ToastTone::Warn);
        return;
    };
    let availability = recipe_availability(recipe, game.flags(), game.repository());
    if availability != RecipeAvailability::Ready {
        menu_sfx.blocked();
        state.page = Some(ServicePage::Apothecary);
        state.announce(availability.to_string(), ToastTone::Warn);
        return;
    }
    if state.page != Some(ServicePage::RecipeDetail) {
        menu_sfx.confirm();
        state.page = Some(ServicePage::RecipeDetail);
        return;
    }
    let flags = game.flags().clone();
    match craft(game.repository_mut(), &flags, recipe) {
        Ok(()) => {
            menu_sfx.confirm();
            state.page = Some(ServicePage::Apothecary);
            state.announce(
                format!("Brewed {} x{}.", recipe.scroll_name, recipe.output.qty),
                ToastTone::Good,
            );
            let rows = recipe_rows(state, catalog, game);
            if !rows.contains(&state.selected) {
                state.selected = rows.first().copied().unwrap_or_default();
            }
        }
        Err(error) => {
            menu_sfx.blocked();
            state.page = Some(ServicePage::Apothecary);
            state.announce(error.to_string(), ToastTone::Warn);
        }
    }
}
