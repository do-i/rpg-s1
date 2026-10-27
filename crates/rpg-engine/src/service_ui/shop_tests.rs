//! Regression coverage for the redesigned shop's navigation and review boundary.

use super::*;
use crate::{
    field_menu_domain::tests::{catalog_with_item_shop, game},
    input_record::NormalizedAction,
    runtime_map::RuntimeMapId,
    scenario_spatial::{CardinalDirection, Position},
};

fn app() -> App {
    let mut game = game(["story_quest_started"]);
    let _ = game.repository_mut().add_gp(10_000).unwrap();
    let _ = game.repository_mut().add_item("potion", 15).unwrap();
    game.map_mut().move_to(
        RuntimeMapId::try_new("town_01_ardel_shop_item").unwrap(),
        Position::new(6, 5),
        CardinalDirection::Up,
    );
    let mut state = ServiceUiState::default();
    state.open(ServiceRequest::Shop(DialogueShopKind::Item), None);
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ActionState>()
        .init_resource::<EngineSettings>()
        .insert_resource(catalog_with_item_shop())
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
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    app.update();
}

fn enter_items(app: &mut App) {
    press(app, NormalizedAction::MenuDown);
    press(app, NormalizedAction::MenuDown);
    assert_eq!(
        app.world().resource::<ServiceUiState>().shop_focus,
        ShopFocus::Items
    );
}

fn repository(app: &App) -> &crate::runtime_repository::RuntimeRepository {
    app.world().resource::<GameState>().repository()
}

#[test]
fn normalized_input_reaches_modes_categories_details_and_bulk_review() {
    let mut app = app();
    press(&mut app, NormalizedAction::MenuRight);
    assert_eq!(
        app.world().resource::<ServiceUiState>().page,
        Some(ServicePage::Sell)
    );
    press(&mut app, NormalizedAction::MenuLeft);
    press(&mut app, NormalizedAction::MenuDown);
    press(&mut app, NormalizedAction::MenuLeft);
    assert_eq!(
        app.world().resource::<ServiceUiState>().shop_category,
        InventoryTab::Status
    );
    press(&mut app, NormalizedAction::MenuRight);
    press(&mut app, NormalizedAction::MenuDown);
    press(&mut app, NormalizedAction::MenuRight);
    assert_eq!(
        app.world().resource::<ServiceUiState>().shop_focus,
        ShopFocus::Detail
    );
    press(&mut app, NormalizedAction::MenuRight);
    assert_eq!(app.world().resource::<ServiceUiState>().quantity, 11);
    let before = repository(&app).clone();
    press(&mut app, NormalizedAction::Confirm);
    let state = app.world().resource::<ServiceUiState>();
    assert_eq!(state.page, Some(ServicePage::ShopConfirm));
    assert_eq!(state.quantity, 11);
    assert_eq!(state.pending_id.as_deref(), Some("potion"));
    assert_eq!(repository(&app), &before);
    press(&mut app, NormalizedAction::Back);
    assert_eq!(repository(&app), &before);
    assert_eq!(app.world().resource::<ServiceUiState>().pending, None);
    press(&mut app, NormalizedAction::Confirm);
    press(&mut app, NormalizedAction::Confirm);
    assert_eq!(repository(&app).gp(), before.gp() - 100);
    assert_eq!(
        repository(&app).item_count("potion"),
        before.item_count("potion") + 1
    );
}

#[test]
fn leaving_detail_resets_the_visible_quantity_before_quick_review() {
    let mut app = app();
    enter_items(&mut app);
    press(&mut app, NormalizedAction::MenuRight);
    press(&mut app, NormalizedAction::MenuRight);
    assert_eq!(app.world().resource::<ServiceUiState>().quantity, 11);
    press(&mut app, NormalizedAction::Back);
    assert_eq!(app.world().resource::<ServiceUiState>().quantity, 1);
    press(&mut app, NormalizedAction::MenuDown);
    press(&mut app, NormalizedAction::Confirm);
    let state = app.world().resource::<ServiceUiState>();
    assert_eq!(state.pending_id.as_deref(), Some("ether"));
    assert_eq!(state.quantity, 1);
}

#[test]
fn bulk_buy_and_sell_commit_the_reviewed_counts_and_balances() {
    for selling in [false, true] {
        let mut app = app();
        if selling {
            press(&mut app, NormalizedAction::MenuRight);
        }
        enter_items(&mut app);
        press(&mut app, NormalizedAction::MenuRight);
        press(&mut app, NormalizedAction::MenuRight);
        let before = repository(&app).clone();
        press(&mut app, NormalizedAction::Confirm);
        assert_eq!(repository(&app), &before);
        press(&mut app, NormalizedAction::Confirm);
        assert_eq!(
            repository(&app).item_count("potion"),
            if selling { 4 } else { 26 }
        );
        assert_eq!(repository(&app).gp(), if selling { 10_550 } else { 8_900 });
        assert_eq!(app.world().resource::<ServiceUiState>().pending, None);
    }
}

#[test]
fn detail_quantity_tracks_live_affordability_and_inventory_caps() {
    let mut app = app();
    enter_items(&mut app);
    press(&mut app, NormalizedAction::MenuRight);
    press(&mut app, NormalizedAction::MenuRight);
    app.world_mut()
        .resource_mut::<GameState>()
        .repository_mut()
        .spend_gp(9_800)
        .unwrap();
    press(&mut app, NormalizedAction::MenuUp);
    assert_eq!(app.world().resource::<ServiceUiState>().quantity, 2);
    let cap = repository(&app).item_quantity_cap();
    let _ = app
        .world_mut()
        .resource_mut::<GameState>()
        .repository_mut()
        .add_item("potion", cap)
        .unwrap();
    let before = repository(&app).clone();
    press(&mut app, NormalizedAction::Confirm);
    let state = app.world().resource::<ServiceUiState>();
    assert_eq!(state.page, Some(ServicePage::Buy));
    assert_eq!(state.quantity, 1);
    assert_eq!(state.pending, None);
    assert_eq!(repository(&app), &before);
}

#[test]
fn changed_funds_locks_and_stock_are_revalidated_without_partial_transactions() {
    for change in 0..3 {
        let mut app = app();
        if change == 1 {
            press(&mut app, NormalizedAction::MenuRight);
        }
        enter_items(&mut app);
        press(&mut app, NormalizedAction::Confirm);
        assert_eq!(
            app.world().resource::<ServiceUiState>().page,
            Some(ServicePage::ShopConfirm)
        );
        let before = {
            let mut game = app.world_mut().resource_mut::<GameState>();
            match change {
                0 => {
                    game.repository_mut().spend_gp(10_000).unwrap();
                }
                1 => {
                    game.repository_mut().set_locked("potion", true);
                }
                _ => {
                    game.flags_mut().unset("story_quest_started");
                }
            }
            game.repository().clone()
        };
        press(&mut app, NormalizedAction::Confirm);
        assert_eq!(repository(&app), &before);
        let state = app.world().resource::<ServiceUiState>();
        assert_eq!(
            state.page,
            Some(if change == 1 {
                ServicePage::Sell
            } else {
                ServicePage::Buy
            })
        );
        assert_eq!(state.pending, None);
        assert_eq!(state.pending_id, None);
        assert!(state.shop_error.is_some());
    }
}

#[test]
fn empty_categories_and_unaffordable_items_cannot_enter_review() {
    let mut app = app();
    app.world_mut()
        .resource_mut::<GameState>()
        .repository_mut()
        .remove_item("potion", 15)
        .unwrap();
    press(&mut app, NormalizedAction::MenuRight);
    press(&mut app, NormalizedAction::MenuDown);
    press(&mut app, NormalizedAction::MenuDown);
    press(&mut app, NormalizedAction::Confirm);
    let state = app.world().resource::<ServiceUiState>();
    assert_eq!(state.page, Some(ServicePage::Sell));
    assert_eq!(state.shop_focus, ShopFocus::Category);
    assert!(!state.shop_item_selected());
    assert_eq!(state.pending, None);
    press(&mut app, NormalizedAction::MenuUp);
    press(&mut app, NormalizedAction::MenuLeft);
    enter_items(&mut app);
    app.world_mut()
        .resource_mut::<GameState>()
        .repository_mut()
        .spend_gp(10_000)
        .unwrap();
    press(&mut app, NormalizedAction::Confirm);
    assert_eq!(
        app.world().resource::<ServiceUiState>().page,
        Some(ServicePage::Buy)
    );
    assert_eq!(repository(&app).item_count("potion"), 0);
}

#[test]
fn horizontal_actions_dismiss_toasts_and_sell_tags_remain_accessible() {
    let mut app = app();
    app.world_mut()
        .resource_mut::<ServiceUiState>()
        .announce("Previous result", ToastTone::Good);
    press(&mut app, NormalizedAction::MenuRight);
    assert!(app.world().resource::<ServiceUiState>().toast.is_none());
    {
        let mut game = app.world_mut().resource_mut::<GameState>();
        game.repository_mut()
            .add_tags("potion", ["healing"])
            .unwrap();
        let _ = game.repository_mut().add_item("ether", 1).unwrap();
    }
    app.world_mut()
        .resource_mut::<ActionState>()
        .replace_with_normalized(&[]);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyT);
    app.update();
    let state = app.world().resource::<ServiceUiState>();
    assert_eq!(state.sell_tag.as_deref(), Some("healing"));
    assert_eq!(
        sell_rows(state, app.world().resource(), app.world().resource())
            .into_iter()
            .map(ItemDefinition::id)
            .collect::<Vec<_>>(),
        ["potion"]
    );
    app.world_mut()
        .resource_mut::<GameState>()
        .repository_mut()
        .remove_item("potion", 15)
        .unwrap();
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.reset_all();
    keys.press(KeyCode::KeyT);
    app.update();
    assert_eq!(app.world().resource::<ServiceUiState>().sell_tag, None);
}

#[test]
fn stock_selection_and_details_follow_down_and_up_without_losing_the_type() {
    let mut app = app();
    let before = repository(&app).clone();
    let state = app.world().resource::<ServiceUiState>();
    assert!(!state.shop_stock_visible());
    assert!(!state.shop_item_selected());
    press(&mut app, NormalizedAction::MenuDown);
    let state = app.world().resource::<ServiceUiState>();
    assert_eq!(state.shop_category, InventoryTab::All);
    assert!(state.shop_stock_visible());
    assert!(!state.shop_item_selected());
    // Categories never purchase the unhighlighted first item.
    press(&mut app, NormalizedAction::Confirm);
    assert_eq!(
        app.world().resource::<ServiceUiState>().page,
        Some(ServicePage::Buy)
    );
    press(&mut app, NormalizedAction::MenuRight);
    assert_eq!(
        app.world().resource::<ServiceUiState>().shop_category,
        InventoryTab::Recovery
    );
    press(&mut app, NormalizedAction::MenuDown);
    let state = app.world().resource::<ServiceUiState>();
    assert!(state.shop_item_selected());
    assert_eq!(state.selected, 0);
    press(&mut app, NormalizedAction::MenuDown);
    assert_eq!(app.world().resource::<ServiceUiState>().selected, 1);
    press(&mut app, NormalizedAction::MenuUp);
    assert!(
        app.world()
            .resource::<ServiceUiState>()
            .shop_item_selected()
    );
    press(&mut app, NormalizedAction::MenuUp);
    let state = app.world().resource::<ServiceUiState>();
    assert_eq!(state.shop_focus, ShopFocus::Category);
    assert_eq!(state.shop_category, InventoryTab::Recovery);
    assert!(state.shop_stock_visible());
    assert!(!state.shop_item_selected());
    press(&mut app, NormalizedAction::MenuUp);
    assert!(
        !app.world()
            .resource::<ServiceUiState>()
            .shop_stock_visible()
    );
    press(&mut app, NormalizedAction::MenuDown);
    assert_eq!(
        app.world().resource::<ServiceUiState>().shop_category,
        InventoryTab::All
    );
    assert_eq!(repository(&app), &before);
}

#[test]
fn type_pills_only_offer_available_stock_or_sellable_inventory() {
    let mut app = app();
    let types = |app: &App, buying| {
        shop_categories(
            app.world().resource(),
            buying,
            app.world().resource(),
            app.world().resource(),
        )
    };
    assert_eq!(
        types(&app, true),
        [
            InventoryTab::All,
            InventoryTab::Recovery,
            InventoryTab::Status
        ]
    );
    {
        let mut game = app.world_mut().resource_mut::<GameState>();
        for id in ["mc_xs", "phoenix_wing"] {
            let _ = game.repository_mut().add_item(id, 1).unwrap();
        }
    }
    assert_eq!(
        types(&app, false),
        [InventoryTab::All, InventoryTab::Recovery]
    );
    press(&mut app, NormalizedAction::MenuDown);
    press(&mut app, NormalizedAction::MenuLeft);
    assert_eq!(
        app.world().resource::<ServiceUiState>().shop_category,
        InventoryTab::Status
    );
    press(&mut app, NormalizedAction::MenuUp);
    press(&mut app, NormalizedAction::MenuRight);
    press(&mut app, NormalizedAction::MenuDown);
    assert_eq!(
        app.world().resource::<ServiceUiState>().shop_category,
        InventoryTab::All
    );
    press(&mut app, NormalizedAction::MenuLeft);
    assert_eq!(
        app.world().resource::<ServiceUiState>().shop_category,
        InventoryTab::Recovery
    );
    // Removing the only sellable type also clears its stale category selection.
    app.world_mut()
        .resource_mut::<GameState>()
        .repository_mut()
        .set_locked("potion", true);
    press(&mut app, NormalizedAction::MenuDown);
    let state = app.world().resource::<ServiceUiState>();
    assert_eq!(state.shop_category, InventoryTab::All);
    assert_eq!(state.shop_focus, ShopFocus::Category);
    assert_eq!(types(&app, false), [InventoryTab::All]);
}

#[test]
fn pointer_quantity_buttons_change_counts_without_bypassing_review() {
    fn click(app: &mut App, step: isize) {
        app.world_mut()
            .resource_mut::<ActionState>()
            .replace_with_normalized(&[]);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        let button = app
            .world_mut()
            .spawn((
                ShopControl::Quantity(step),
                ComputedNode {
                    size: Vec2::splat(100.0),
                    ..default()
                },
                UiGlobalTransform::default(),
            ))
            .id();
        app.insert_resource(CanvasPointer::at_viewport_position(Vec2::ZERO, true));
        app.update();
        app.world_mut().despawn(button);
    }
    let mut app = app();
    enter_items(&mut app);
    let before = repository(&app).clone();
    click(&mut app, 1);
    assert_eq!(app.world().resource::<ServiceUiState>().quantity, 2);
    assert_eq!(
        app.world().resource::<ServiceUiState>().shop_focus,
        ShopFocus::Detail
    );
    click(&mut app, -1);
    click(&mut app, -1);
    assert_eq!(app.world().resource::<ServiceUiState>().quantity, 1);
    assert_eq!(repository(&app), &before);
    // A changed wallet clamps mouse input just like the keyboard controls.
    app.world_mut()
        .resource_mut::<GameState>()
        .repository_mut()
        .spend_gp(9_900)
        .unwrap();
    click(&mut app, 1);
    assert_eq!(app.world().resource::<ServiceUiState>().quantity, 1);
    let before = repository(&app).clone();
    press(&mut app, NormalizedAction::Confirm);
    assert_eq!(
        app.world().resource::<ServiceUiState>().page,
        Some(ServicePage::ShopConfirm)
    );
    click(&mut app, -1);
    assert_eq!(app.world().resource::<ServiceUiState>().quantity, 1);
    assert_eq!(repository(&app), &before);
    press(&mut app, NormalizedAction::Confirm);
    assert_eq!(repository(&app).gp(), 0);
    assert_eq!(
        repository(&app).item_count("potion"),
        before.item_count("potion") + 1
    );
}

#[test]
fn insufficient_funds_opens_a_modal_that_consumes_dismissal_input() {
    let mut app = app();
    enter_items(&mut app);
    app.world_mut()
        .resource_mut::<GameState>()
        .repository_mut()
        .spend_gp(10_000)
        .unwrap();
    let before = repository(&app).clone();
    press(&mut app, NormalizedAction::Confirm);
    let state = app.world().resource::<ServiceUiState>();
    assert!(state.shop_error.is_some());
    assert!(state.toast.is_none());
    assert_eq!(state.page, Some(ServicePage::Buy));
    assert_eq!(state.shop_focus, ShopFocus::Items);
    // Navigation and elapsed time leave the modal open and the background selection still.
    press(&mut app, NormalizedAction::MenuDown);
    let state = app.world().resource::<ServiceUiState>();
    assert!(state.shop_error.is_some());
    assert_eq!(state.selected, 0);
    press(&mut app, NormalizedAction::Confirm);
    let state = app.world().resource::<ServiceUiState>();
    assert!(state.shop_error.is_none());
    assert_eq!(state.page, Some(ServicePage::Buy));
    assert_eq!(state.pending, None);
    assert_eq!(repository(&app), &before);
    press(&mut app, NormalizedAction::Confirm);
    assert!(
        app.world()
            .resource::<ServiceUiState>()
            .shop_error
            .is_some()
    );
    press(&mut app, NormalizedAction::Back);
    assert!(
        app.world()
            .resource::<ServiceUiState>()
            .shop_error
            .is_none()
    );
    assert_eq!(
        app.world().resource::<ServiceUiState>().shop_focus,
        ShopFocus::Items
    );
    assert_eq!(repository(&app), &before);
}

#[test]
fn shop_error_can_be_dismissed_by_its_mouse_button() {
    let mut app = app();
    enter_items(&mut app);
    app.world_mut()
        .resource_mut::<ServiceUiState>()
        .announce("Not enough GP.", ToastTone::Warn);
    let before = repository(&app).clone();
    app.world_mut()
        .resource_mut::<ActionState>()
        .replace_with_normalized(&[]);
    app.world_mut().spawn((
        ShopControl::DismissError,
        ComputedNode {
            size: Vec2::splat(100.0),
            ..default()
        },
        UiGlobalTransform::default(),
    ));
    app.insert_resource(CanvasPointer::at_viewport_position(Vec2::ZERO, true));
    app.update();
    assert!(
        app.world()
            .resource::<ServiceUiState>()
            .shop_error
            .is_none()
    );
    assert_eq!(
        app.world().resource::<ServiceUiState>().page,
        Some(ServicePage::Buy)
    );
    assert_eq!(app.world().resource::<ServiceUiState>().pending, None);
    assert_eq!(repository(&app), &before);
}
