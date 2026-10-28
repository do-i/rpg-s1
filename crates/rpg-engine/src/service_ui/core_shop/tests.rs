use super::*;
use crate::{
    field_menu_domain::tests::{catalog, game},
    input_record::NormalizedAction,
};

fn app() -> App {
    let mut game = game(["story_quest_started"]);
    let _ = game.repository_mut().add_gp(23_569).unwrap();
    for (id, count) in [
        ("mc_xs", 100),
        ("mc_s", 100),
        ("mc_m", 2),
        ("mc_l", 2),
        ("mc_xl", 1),
    ] {
        let _ = game.repository_mut().add_item(id, count).unwrap();
    }
    let mut state = ServiceUiState::default();
    state.open(ServiceRequest::Shop(DialogueShopKind::MagicCore), None);
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ActionState>()
        .insert_resource(catalog())
        .insert_resource(game)
        .insert_resource(state)
        .add_message::<PlaySfx>()
        .add_systems(Update, handle_service_input);
    app.update();
    app
}

fn press(app: &mut App, action: NormalizedAction) {
    app.world_mut()
        .resource_mut::<ActionState>()
        .replace_with_normalized(&[action]);
    app.update();
}

fn repo(app: &App) -> &crate::runtime_repository::RuntimeRepository {
    app.world().resource::<GameState>().repository()
}

fn click(app: &mut App, control: CoreControl) {
    app.world_mut()
        .resource_mut::<ActionState>()
        .replace_with_normalized(&[]);
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

fn enter_rank_up(app: &mut App) {
    press(app, NormalizedAction::MenuDown); // S
    press(app, NormalizedAction::Confirm); // quantity
    press(app, NormalizedAction::MenuRight); // rank up
}

#[test]
fn normalized_sale_navigation_reviews_and_cancels_before_committing_exact_counts() {
    let mut app = app();
    let before = repo(&app).clone();
    press(&mut app, NormalizedAction::MenuDown);
    press(&mut app, NormalizedAction::MenuRight);
    assert_eq!(
        app.world().resource::<ServiceUiState>().core_shop.focus,
        CoreFocus::Quantity
    );
    press(&mut app, NormalizedAction::MenuUp);
    press(&mut app, NormalizedAction::Confirm);
    assert_eq!(
        app.world().resource::<ServiceUiState>().page,
        Some(ServicePage::CoreConfirm)
    );
    assert_eq!(*repo(&app), before);
    press(&mut app, NormalizedAction::Back);
    assert_eq!(*repo(&app), before);
    assert_eq!(app.world().resource::<ServiceUiState>().quantity, 2);
    press(&mut app, NormalizedAction::Confirm);
    press(&mut app, NormalizedAction::Confirm);
    assert_eq!(repo(&app).item_count("mc_s"), 98);
    assert_eq!(repo(&app).gp(), before.gp() + 20);
    let state = app.world().resource::<ServiceUiState>();
    assert_eq!(state.page, Some(ServicePage::MagicCore));
    assert_eq!(state.selected, 1);
    assert_eq!(state.core_shop.focus, CoreFocus::Items);
}

#[test]
fn rank_up_two_batches_is_atomic_and_keeps_all_five_rows_after_spending_the_stack() {
    let mut app = app();
    enter_rank_up(&mut app);
    // Clear the receiving stack to make the exact 100 S -> 2 M example.
    app.world_mut()
        .resource_mut::<GameState>()
        .repository_mut()
        .remove_item("mc_m", 2)
        .unwrap();
    let before = repo(&app).clone();
    press(&mut app, NormalizedAction::MenuUp);
    press(&mut app, NormalizedAction::MenuUp); // clamp at 2
    assert_eq!(app.world().resource::<ServiceUiState>().quantity, 2);
    press(&mut app, NormalizedAction::Confirm);
    assert_eq!(*repo(&app), before);
    press(&mut app, NormalizedAction::Back);
    assert_eq!(*repo(&app), before);
    press(&mut app, NormalizedAction::Confirm);
    press(&mut app, NormalizedAction::Confirm);
    assert_eq!(repo(&app).item_count("mc_s"), 0);
    assert_eq!(repo(&app).item_count("mc_m"), 2);
    assert_eq!(repo(&app).gp(), before.gp());
    assert_eq!(
        app.world().resource::<ServiceUiState>().page,
        Some(ServicePage::MagicCore)
    );
    assert_eq!(rows(app.world().resource()).len(), 5);
}

#[test]
fn confirmation_rechecks_source_lock_quantity_and_destination_capacity() {
    for change in [0, 1, 2] {
        let mut app = app();
        enter_rank_up(&mut app);
        press(&mut app, NormalizedAction::Confirm);
        assert_eq!(
            app.world().resource::<ServiceUiState>().page,
            Some(ServicePage::CoreConfirm)
        );
        let repo = app
            .world_mut()
            .resource_mut::<GameState>()
            .into_inner()
            .repository_mut();
        match change {
            0 => repo.remove_item("mc_s", 51).unwrap(),
            1 => {
                let _ = repo.add_item("mc_m", 98).unwrap();
            }
            _ => repo.set_locked("mc_s", true),
        }
        let before = repo.clone();
        press(&mut app, NormalizedAction::Confirm);
        assert_eq!(*self::repo(&app), before);
        let state = app.world().resource::<ServiceUiState>();
        assert_eq!(state.page, Some(ServicePage::MagicCore));
        assert_eq!(state.selected, 1);
        assert_eq!(state.core_shop.focus, CoreFocus::Quantity);
        assert!(state.toast.is_some());
    }
}

#[test]
fn xl_missing_inputs_locked_cores_and_full_stacks_cannot_enter_review() {
    for change in [0, 1, 2, 3] {
        let mut app = app();
        click(&mut app, CoreControl::Item(if change == 0 { 4 } else { 1 }));
        click(&mut app, CoreControl::Mode(CoreMode::RankUp));
        let repo = app
            .world_mut()
            .resource_mut::<GameState>()
            .into_inner()
            .repository_mut();
        match change {
            1 => repo.remove_item("mc_s", 51).unwrap(),
            2 => repo.set_locked("mc_s", true),
            3 => {
                let _ = repo.add_item("mc_m", 98).unwrap();
            }
            _ => {}
        }
        let before = repo.clone();
        press(&mut app, NormalizedAction::Confirm);
        assert_eq!(*self::repo(&app), before);
        assert_eq!(
            app.world().resource::<ServiceUiState>().page,
            Some(ServicePage::MagicCore)
        );
    }
}

#[test]
fn pointer_modes_quantity_and_review_observe_the_same_transaction_boundary() {
    let mut app = app();
    click(&mut app, CoreControl::Item(1));
    click(&mut app, CoreControl::Mode(CoreMode::RankUp));
    click(&mut app, CoreControl::Quantity(1));
    let before = repo(&app).clone();
    click(&mut app, CoreControl::Review);
    assert_eq!(*repo(&app), before);
    // Clicks on the obscured quantity buttons cannot modify a staged review.
    click(&mut app, CoreControl::Quantity(-1));
    assert_eq!(app.world().resource::<ServiceUiState>().quantity, 2);
    click(&mut app, CoreControl::Cancel);
    assert_eq!(*repo(&app), before);
    click(&mut app, CoreControl::Review);
    click(&mut app, CoreControl::Confirm);
    assert_eq!(repo(&app).item_count("mc_s"), 0);
    assert_eq!(repo(&app).item_count("mc_m"), 4);
    assert_eq!(repo(&app).gp(), before.gp());
}

#[test]
fn output_room_and_purse_room_limit_the_picker_and_are_rechecked_at_review() {
    let mut app = app();
    let repo = app
        .world_mut()
        .resource_mut::<GameState>()
        .into_inner()
        .repository_mut();
    let _ = repo.add_item("mc_m", 97).unwrap();
    let _ = repo.add_gp(repo.gp_cap() - repo.gp() - 5).unwrap();
    enter_rank_up(&mut app);
    press(&mut app, NormalizedAction::MenuUp);
    assert_eq!(app.world().resource::<ServiceUiState>().quantity, 1);
    press(&mut app, NormalizedAction::MenuLeft); // S sale cannot fit even one 10 GP unit.
    press(&mut app, NormalizedAction::Confirm);
    assert_eq!(
        app.world().resource::<ServiceUiState>().page,
        Some(ServicePage::MagicCore)
    );
    press(&mut app, NormalizedAction::Back);
    press(&mut app, NormalizedAction::MenuUp); // XS
    press(&mut app, NormalizedAction::Confirm);
    for _ in 0..10 {
        press(&mut app, NormalizedAction::MenuUp);
    }
    assert_eq!(app.world().resource::<ServiceUiState>().quantity, 5);
    press(&mut app, NormalizedAction::Confirm);
    let repo = app
        .world_mut()
        .resource_mut::<GameState>()
        .into_inner()
        .repository_mut();
    let _ = repo.add_gp(5).unwrap();
    let before = repo.clone();
    press(&mut app, NormalizedAction::Confirm);
    assert_eq!(*self::repo(&app), before);
}
