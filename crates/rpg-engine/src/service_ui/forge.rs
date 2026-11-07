//! The smith's flow: pick an equipment type, pick a piece, review the quote, then strike.

use super::*;
use crate::forge::{ForgeKind, ForgeOutcome, ForgeType, check, forge, forgeable_of};

#[derive(Clone, Copy)]
pub(super) struct ForgeInput {
    pub(super) vertical: Option<isize>,
    pub(super) horizontal: Option<isize>,
    pub(super) confirm: bool,
    pub(super) back: bool,
}

/// The pieces listed for the type column's current selection.
pub(super) fn rows(
    state: &ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &GameState,
) -> Vec<String> {
    let kind = ForgeType::ALL[state.forge_type.min(ForgeType::ALL.len() - 1)];
    forgeable_of(catalog, game.repository(), kind)
}

pub(super) fn handle_input(
    state: &mut ServiceUiState,
    catalog: &FieldMenuCatalog,
    game: &mut GameState,
    menu_sfx: &mut MenuSfx,
    input: ForgeInput,
) {
    let listed = rows(state, catalog, game);
    state.forge_type = state.forge_type.min(ForgeType::ALL.len() - 1);
    state.selected = state.selected.min(listed.len().saturating_sub(1));
    if listed.is_empty() {
        state.forge_in_list = false;
    }
    if state.page == Some(ServicePage::ForgeConfirm) && !state.forge_in_list {
        state.page = Some(ServicePage::Forge);
    }
    if input.back {
        menu_sfx.cancel();
        if state.page == Some(ServicePage::ForgeConfirm) {
            state.page = Some(ServicePage::Forge);
        } else if state.forge_in_list {
            state.forge_in_list = false;
        } else {
            state.close();
        }
        return;
    }
    if let Some(step) = input.horizontal {
        if step > 0 && !state.forge_in_list && !listed.is_empty() {
            state.forge_in_list = true;
            state.selected = 0;
            menu_sfx.hover();
        } else if step < 0 && state.forge_in_list {
            state.forge_in_list = false;
            state.page = Some(ServicePage::Forge);
            menu_sfx.hover();
        }
        return;
    }
    if let Some(delta) = input.vertical {
        // Moving a cursor always leaves review, so a selection change can never strike.
        state.page = Some(ServicePage::Forge);
        if state.forge_in_list {
            state.selected = wrapped_or_zero(state.selected, listed.len(), delta);
        } else {
            state.forge_type = wrapped(state.forge_type, ForgeType::ALL.len(), delta);
            state.selected = 0;
        }
        return;
    }
    if !input.confirm {
        return;
    }
    if !state.forge_in_list {
        if listed.is_empty() {
            menu_sfx.blocked();
            state.announce("You carry nothing of that kind to temper.", ToastTone::Warn);
        } else {
            menu_sfx.confirm();
            state.forge_in_list = true;
            state.selected = 0;
        }
        return;
    }
    let Some(id) = listed.get(state.selected) else {
        return;
    };
    if state.page != Some(ServicePage::ForgeConfirm) {
        menu_sfx.confirm();
        state.page = Some(ServicePage::ForgeConfirm);
        return;
    }
    // The gameplay RNG is saved and shared with battle, so only a strike that will actually roll
    // may advance it: a mend or a refused attempt leaves the stream untouched.
    let draw = match check(game.repository(), catalog, id) {
        Ok(quote) if quote.kind == ForgeKind::Enhance => game.rng_mut().next_u64(),
        _ => 0,
    };
    state.page = Some(ServicePage::Forge);
    match forge(game.repository_mut(), catalog, id, draw) {
        Ok(outcome) => {
            let name = |id: &str| catalog.item(id).map_or(id, item_name).to_owned();
            let (text, tone) = match &outcome {
                ForgeOutcome::Success(to) => (format!("Tempered: {}.", name(to)), ToastTone::Good),
                ForgeOutcome::Great(to) => {
                    (format!("A flawless strike! {}.", name(to)), ToastTone::Good)
                }
                ForgeOutcome::Mended(to) => (format!("Mended: {}.", name(to)), ToastTone::Good),
                ForgeOutcome::Fizzle => (
                    "The metal would not take. Half the GP is returned.".to_owned(),
                    ToastTone::Warn,
                ),
                ForgeOutcome::Backfire(to) => {
                    (format!("It cracked! {}.", name(to)), ToastTone::Warn)
                }
            };
            if tone == ToastTone::Good {
                menu_sfx.confirm();
            } else {
                menu_sfx.blocked();
            }
            state.announce(text, tone);
        }
        Err(error) => {
            menu_sfx.blocked();
            state.announce(error.to_string(), ToastTone::Warn);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        field_menu_domain::tests::{catalog, game},
        input_record::NormalizedAction,
    };

    fn app() -> App {
        let mut game = game([]);
        let repo = game.repository_mut();
        let _ = repo.add_gp(2_000).unwrap();
        for (id, qty) in [("iron_sword", 1), ("short_sword", 1), ("iron_plate", 5)] {
            let _ = repo.add_item(id, qty).unwrap();
        }
        let mut state = ServiceUiState::default();
        state.open(ServiceRequest::Forge, None);
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ActionState>()
            .init_resource::<EngineSettings>()
            .insert_resource(catalog())
            .insert_resource(game)
            .insert_resource(state)
            .add_message::<PlaySfx>()
            .add_systems(PreUpdate, super::super::reset_service_input_lock)
            .add_systems(Update, handle_service_input);
        app.update();
        app
    }

    fn press(app: &mut App, action: NormalizedAction) {
        app.world_mut()
            .resource_mut::<ActionState>()
            .replace_with_normalized(&[action]);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.update();
    }

    #[test]
    fn reviewing_and_cancelling_never_spend_and_the_second_confirm_strikes() {
        let mut app = app();
        let gp = app.world().resource::<GameState>().repository().gp();
        // The cursor starts in the type column on Weapon; Enter steps into the list.
        press(&mut app, NormalizedAction::Confirm);
        assert!(app.world().resource::<ServiceUiState>().forge_in_list);
        press(&mut app, NormalizedAction::Confirm);
        assert_eq!(
            app.world().resource::<ServiceUiState>().page,
            Some(ServicePage::ForgeConfirm)
        );
        // Weapons list in id order: iron_sword, short_sword.
        press(&mut app, NormalizedAction::MenuDown);
        let state = app.world().resource::<ServiceUiState>();
        assert_eq!((state.page, state.selected), (Some(ServicePage::Forge), 1));
        press(&mut app, NormalizedAction::Confirm);
        press(&mut app, NormalizedAction::Back);
        assert_eq!(
            app.world().resource::<ServiceUiState>().page,
            Some(ServicePage::Forge)
        );
        assert_eq!(app.world().resource::<GameState>().repository().gp(), gp);

        press(&mut app, NormalizedAction::Confirm);
        press(&mut app, NormalizedAction::Confirm);
        let game = app.world().resource::<GameState>();
        // A fizzle returns half; every other outcome costs the full 37 GP.
        assert!(game.repository().gp() < gp);
        let tempered: u32 = game
            .repository()
            .item_counts()
            .filter(|(id, _)| crate::forge::base_id(id) == "short_sword")
            .map(|(_, n)| n)
            .sum();
        assert_eq!(tempered, 1);
        assert!(app.world().resource::<ServiceUiState>().toast.is_some());
    }

    #[test]
    fn a_refused_strike_leaves_the_saved_rng_untouched() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<GameState>()
            .repository_mut()
            .remove_item("iron_plate", 5)
            .unwrap();
        let before = app.world().resource::<GameState>().rng().clone();
        press(&mut app, NormalizedAction::Confirm);
        press(&mut app, NormalizedAction::Confirm);
        press(&mut app, NormalizedAction::Confirm);
        let game = app.world().resource::<GameState>();
        assert_eq!(game.repository().item_count("iron_sword"), 1);
        assert_eq!(*game.rng(), before);
    }

    #[test]
    fn back_steps_out_one_level_at_a_time_and_empty_types_refuse() {
        let mut app = app();
        press(&mut app, NormalizedAction::Confirm);
        press(&mut app, NormalizedAction::Back);
        let state = app.world().resource::<ServiceUiState>();
        assert!(state.is_open() && !state.forge_in_list);
        // Shield holds nothing in the fixture, so Enter stays in the type column.
        press(&mut app, NormalizedAction::MenuDown);
        assert_eq!(app.world().resource::<ServiceUiState>().forge_type, 1);
        press(&mut app, NormalizedAction::Confirm);
        let state = app.world().resource::<ServiceUiState>();
        assert!(!state.forge_in_list && state.toast.is_some());
        press(&mut app, NormalizedAction::Back);
        assert!(!app.world().resource::<ServiceUiState>().is_open());
    }
}
