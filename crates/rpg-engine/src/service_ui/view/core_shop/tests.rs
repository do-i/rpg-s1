use super::*;
use crate::field_menu_domain::tests::{catalog, game};

fn view(focus: CoreFocus, review: bool, quantity: u32) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let mut game = game([]);
    let _ = game.repository_mut().add_item("mc_s", 100).unwrap();
    let mut state = ServiceUiState::default();
    state.open(ServiceRequest::Shop(DialogueShopKind::MagicCore), None);
    state.selected = 1;
    state.core_shop.mode = CoreMode::RankUp;
    state.core_shop.focus = focus;
    state.quantity = quantity;
    state.pending_id = Some("mc_s".to_owned());
    state.pending = Some(PendingTransaction::RankUp);
    let catalog = catalog();
    app.world_mut()
        .commands()
        .spawn(Node::default())
        .with_children(|root| {
            spawn_header(root, &Handle::default(), &game);
            spawn_page(root, &Handle::default(), &state, &catalog, &game);
            spawn_footer(root, &Handle::default(), &state);
            if review {
                spawn_review(root, &Handle::default(), &state, &catalog, &game);
            } else if focus == CoreFocus::Quantity {
                spawn_quantity_dialog(root, &Handle::default(), &state, &catalog, &game);
            }
        });
    app.update();
    app
}

#[test]
fn exchange_selector_has_larger_list_art_and_native_source_and_target_images() {
    let mut app = view(CoreFocus::Exchange, false, 1);
    let world = app.world_mut();
    let images = world
        .query::<(&Name, &Node)>()
        .iter(world)
        .filter(|(name, _)| name.as_str().starts_with("Item image:"))
        .collect::<Vec<_>>();
    assert_eq!(
        images
            .iter()
            .filter(|(_, node)| node.width == px(76))
            .count(),
        5
    );
    assert_eq!(
        images
            .iter()
            .filter(|(_, node)| node.width == px(128) && node.height == px(128))
            .count(),
        3
    );
    let controls = world
        .query::<&CoreControl>()
        .iter(world)
        .collect::<Vec<_>>();
    assert_eq!(controls.len(), 7); // five cores and two destinations; quantity is modal only
    assert!(
        controls
            .iter()
            .all(|control| matches!(control, CoreControl::Item(_) | CoreControl::Mode(_)))
    );
    assert!(
        !world
            .query::<&Name>()
            .iter(world)
            .any(|name| name.as_str() == "Core frame accent")
    );
    assert!(
        !world
            .query::<&Text>()
            .iter(world)
            .any(|text| text.contains("UP / DOWN") || text.contains("LEFT / RIGHT"))
    );
}

#[test]
fn quantity_dialog_has_simple_buttons_stable_width_and_an_explicit_review_action() {
    for quantity in [1, 9, 10, 99, 100] {
        let mut app = view(CoreFocus::Quantity, false, quantity);
        let world = app.world_mut();
        let texts = world
            .query::<&Text>()
            .iter(world)
            .map(|text| text.0.as_str())
            .collect::<Vec<_>>();
        assert!(texts.contains(&"+"));
        assert!(texts.contains(&"-"));
        assert!(texts.contains(&"BACK · ESC"));
        assert!(texts.contains(&"REVIEW EXCHANGE · ENTER"));
        assert!(!texts.iter().any(|text| text.contains("+ 1")
            || text.contains("ADJUST BY")
            || text.contains("CANCEL")));
        let picker = world
            .query::<(&Name, &Node)>()
            .iter(world)
            .find(|(name, _)| name.as_str() == "Vertical core quantity")
            .unwrap()
            .1;
        assert_eq!(picker.width, px(166));
        assert_eq!(picker.height, px(128));
        let layer = world
            .query::<(&Name, &ZIndex)>()
            .iter(world)
            .find(|(name, _)| name.as_str() == "Core quantity dialog")
            .unwrap()
            .1;
        assert!(layer.0 > 0);
    }
}

#[test]
fn review_shows_exact_exchange_and_only_back_and_confirm_controls() {
    let mut app = view(CoreFocus::Quantity, true, 2);
    let world = app.world_mut();
    let texts = world
        .query::<&Text>()
        .iter(world)
        .map(|text| text.0.as_str())
        .collect::<Vec<_>>();
    assert!(texts.contains(&"100 × S"));
    assert!(texts.contains(&"2 × M"));
    assert!(texts.contains(&"BACK · ESC"));
    assert!(texts.contains(&"CONFIRM EXCHANGE · ENTER"));
    assert!(!texts.contains(&"CHOOSE QUANTITY"));
    let controls = world
        .query::<&CoreControl>()
        .iter(world)
        .filter(|control| matches!(control, CoreControl::Back | CoreControl::Confirm))
        .count();
    assert_eq!(controls, 2);
}
