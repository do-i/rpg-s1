use bevy::{input::keyboard::KeyboardInput, prelude::*};

use super::{
    assets::*, battle::*, equipment::*, input::*, main_deck::*, markers::*, state::*, ui::*,
};
use crate::battle_tactics::AutoCommand;
use crate::save_data::tests::{fixture_balance, fixture_game};
use crate::{
    action_input::ActionState,
    battle_log::LastBattleLog,
    battle_tactics::{TACTIC_CHOICES, TurnPause},
    engine_settings::EngineSettings,
    field_menu_domain::{
        FieldMenuCatalog, InventoryTab, inventory_ids, item_name, learned_field_abilities,
    },
    game_state::GameState,
    options_store::{OptionsLoadStatus, OptionsStore, PlayerOptions},
    persistence_status::PersistenceStatus,
    runtime_member::EquipmentSlot,
    save_store::{FIRST_PLAYER_SLOT, SaveSlot, SaveSlotState, SaveStore},
    save_ui::SaveSlotCatalog,
    scenario_class::{AbilityKind, UtilityAbility},
    service_ui::ServiceUiState,
    sfx_cue::PlaySfx,
    world_interaction::WorldInteractionState,
    world_transition::WorldTransition,
};

/// A new game starts with the protagonist alone; the switch overlay needs someone to switch to.
pub(super) fn fixture_game_with_recruit() -> GameState {
    let catalog: crate::scenario_party::PartyCatalog =
        crate::scenario_yaml::from_str(include_str!(scenario_file!("data/party.yaml"))).unwrap();
    let elise = catalog
        .party
        .iter()
        .find(|member| member.data().id == "elise")
        .expect("the shipped party has elise");
    let mut game = fixture_game();
    game.party_mut()
        .try_add(
            crate::runtime_member::RuntimeMember::try_from_catalog(
                elise,
                &crate::runtime_member::test_class("cleric"),
                &fixture_balance().progression,
            )
            .unwrap(),
        )
        .unwrap();
    game
}

fn spawn_fixture_main_page(
    mut commands: Commands,
    state: Res<FieldMenuState>,
    game: Res<GameState>,
) {
    commands.spawn(Node::default()).with_children(|parent| {
        spawn_main_menu_page(
            parent,
            &Handle::<Font>::default(),
            None,
            &state,
            &game,
            &PartyWalkThumbnails::default(),
            false,
        );
    });
}

fn spawn_fixture_status_page(
    mut commands: Commands,
    state: Res<FieldMenuState>,
    game: Res<GameState>,
    catalog: Res<FieldMenuCatalog>,
) {
    commands.spawn(Node::default()).with_children(|parent| {
        spawn_status_page(
            parent,
            &Handle::<Font>::default(),
            &state,
            &game,
            &catalog,
            &StatusPortraitAssets::default(),
        );
    });
}

fn spawn_fixture_items_page(
    mut commands: Commands,
    state: Res<FieldMenuState>,
    game: Res<GameState>,
    catalog: Res<FieldMenuCatalog>,
) {
    commands.spawn(Node::default()).with_children(|parent| {
        spawn_items_page(parent, &Handle::<Font>::default(), &state, &game, &catalog);
    });
}

fn spawn_fixture_recipes_page(
    mut commands: Commands,
    state: Res<FieldMenuState>,
    game: Res<GameState>,
    catalog: Res<FieldMenuCatalog>,
) {
    commands.spawn(Node::default()).with_children(|parent| {
        spawn_recipes_page(parent, &Handle::<Font>::default(), &state, &game, &catalog);
    });
}

fn spawn_fixture_battle_page(
    mut commands: Commands,
    state: Res<FieldMenuState>,
    game: Res<GameState>,
    catalog: Res<FieldMenuCatalog>,
    battle_log: Option<Res<LastBattleLog>>,
) {
    commands.spawn(Node::default()).with_children(|parent| {
        spawn_battle_page(
            parent,
            &Handle::<Font>::default(),
            &state,
            &game,
            &catalog,
            &PlayerOptions::default(),
            battle_log.as_deref().unwrap_or(LastBattleLog::empty()),
        );
    });
}

fn spawn_fixture_equipment_page(
    mut commands: Commands,
    state: Res<FieldMenuState>,
    game: Res<GameState>,
    catalog: Res<FieldMenuCatalog>,
) {
    commands.spawn(Node::default()).with_children(|parent| {
        spawn_equipment_page(
            parent,
            &Handle::<Font>::default(),
            &state,
            &game,
            &catalog,
            &StatusPortraitAssets::default(),
        );
    });
}

fn spawn_fixture_spells_page(
    mut commands: Commands,
    state: Res<FieldMenuState>,
    game: Res<GameState>,
    catalog: Res<FieldMenuCatalog>,
) {
    commands.spawn(Node::default()).with_children(|parent| {
        spawn_spells_page(
            parent,
            &Handle::<Font>::default(),
            &state,
            &game,
            &catalog,
            &StatusPortraitAssets::default(),
        );
    });
}

fn spawn_fixture_save_page(
    mut commands: Commands,
    state: Res<FieldMenuState>,
    game: Res<GameState>,
    saves: Res<SaveSlotCatalog>,
) {
    commands.spawn(Node::default()).with_children(|parent| {
        spawn_save_page(parent, &Handle::<Font>::default(), &state, &game, &saves);
    });
}

fn spawn_fixture_valid_save_row(mut commands: Commands) {
    let slot = SaveSlot {
        index: 3,
        state: SaveSlotState::Valid,
        metadata: Some(crate::save_data::SaveMetadata {
            protagonist_name: "Aric".to_owned(),
            protagonist_level: 7,
            location: "Ardel".to_owned(),
            playtime_seconds: 3_661,
        }),
        saved_at_unix_seconds: Some(1_700_000_000),
    };
    commands.spawn(Node::default()).with_children(|parent| {
        spawn_field_save_slot_row(parent, &Handle::<Font>::default(), &slot, true);
    });
}

/// The field-menu content box: the 766px logical canvas less the menu root's 30px padding
/// on each edge (`spawn_field_menu_overlay`). A centred modal gets all of it.
const MENU_CONTENT_HEIGHT: usize = 766 - 30 * 2;

/// What a page's columns row is left with: the content box less the fixed 64px page header,
/// the ~18px hint line under it, and the page's two 14px row gaps.
const PAGE_COLUMN_HEIGHT: usize = MENU_CONTENT_HEIGHT - 64 - 18 - 14 * 2;

/// Chrome a `spawn_status_panel` adds around its rows: padding and border from the caller's
/// node, then the panel's own title line (~1.2x its font size) and the 1px rule under it with
/// its 2px bottom margin, plus one row gap before and after that pair.
const fn panel_chrome(padding: usize, border: usize, title: usize, row_gap: usize) -> usize {
    padding * 2 + border * 2 + title + 3 + row_gap * 2
}

/// Every scrolling list has to fit the box it is drawn in, or it paints over its neighbour.
///
/// This is arithmetic rather than a layout assertion because none of these lists is measured
/// anywhere — the row heights, gaps and paddings are literals spread across the page modules,
/// and the only thing tying them to the canvas is the visible-row constant each list windows
/// by. Each row below states its chrome explicitly so a changed padding shows up as a failure
/// here instead of as content drawn outside a border.
#[test]
fn custom_menu_row_budgets_fit_the_baseline_canvas() {
    // Pouch list: 40px rows over 5px gaps plus a "PAGE nn/nn" footer, in a 14px-padded panel.
    let items =
        INVENTORY_PAGE_ROWS * 40 + (INVENTORY_PAGE_ROWS - 1) * 5 + 15 + panel_chrome(14, 1, 17, 5);
    // Equipment picker: 52px rows over 8px gaps, under the worn-item summary it sits below.
    let equipment = EQUIPMENT_PICKER_VISIBLE_ROWS * 52
        + (EQUIPMENT_PICKER_VISIBLE_ROWS - 1) * 8
        + 180
        + panel_chrome(16, 1, 17, 8);
    // Spellbook: 58px rows over 7px gaps in a 16px-padded panel.
    let spellbook =
        SPELLBOOK_VISIBLE_ROWS * 58 + (SPELLBOOK_VISIBLE_ROWS - 1) * 7 + panel_chrome(16, 1, 17, 8);
    // Quest register: 54px rows over 7px gaps plus the "QUEST nn OF nn" footer.
    let quests =
        QUEST_VISIBLE_ROWS * 54 + (QUEST_VISIBLE_ROWS - 1) * 7 + 13 + panel_chrome(16, 1, 17, 7);
    assert!(items <= PAGE_COLUMN_HEIGHT, "pouch list: {items}px");
    assert!(
        equipment <= PAGE_COLUMN_HEIGHT,
        "equipment picker: {equipment}px"
    );
    assert!(spellbook <= PAGE_COLUMN_HEIGHT, "spellbook: {spellbook}px");
    assert!(quests <= PAGE_COLUMN_HEIGHT, "quest register: {quests}px");
    // Recipe book: the quest register's geometry.
    let recipes =
        RECIPE_VISIBLE_ROWS * 54 + (RECIPE_VISIBLE_ROWS - 1) * 7 + 13 + panel_chrome(16, 1, 17, 7);
    assert!(recipes <= PAGE_COLUMN_HEIGHT, "recipe book: {recipes}px");
    // Battle party list: a full party's 54px rows, the 5px rule, four 46px settings rows,
    // all over 7px gaps.
    let battle_rows = crate::runtime_party::RuntimeParty::MAX_MEMBERS + 1 + 4;
    let battle = crate::runtime_party::RuntimeParty::MAX_MEMBERS * 54
        + 5
        + 4 * 46
        + (battle_rows - 1) * 7
        + panel_chrome(16, 1, 17, 7);
    assert!(
        battle <= PAGE_COLUMN_HEIGHT,
        "battle party list: {battle}px"
    );
    // Battle log: the range line, then one 14px line (~17px) per visible entry, 4px apart.
    let log = 15
        + BATTLE_LOG_VISIBLE_LINES * 17
        + BATTLE_LOG_VISIBLE_LINES * 4
        + panel_chrome(16, 1, 17, 4);
    assert!(log <= PAGE_COLUMN_HEIGHT, "battle log: {log}px");

    // Centred modals are drawn over the whole content box rather than inside a column.
    let save = SAVE_VISIBLE_ROWS * 54 + (SAVE_VISIBLE_ROWS - 1) * 6 + 146;
    // Item management and the tag editor share the 48px modal row and its 9px gap.
    let manage = ITEM_MANAGE_VISIBLE_ROWS * 48
        + (ITEM_MANAGE_VISIBLE_ROWS - 1) * 9
        + 16
        + panel_chrome(16, 2, 17, 9);
    let tags = TAG_EDITOR_VISIBLE_ROWS * 48
        + (TAG_EDITOR_VISIBLE_ROWS - 1) * 9
        + 16
        + panel_chrome(16, 2, 17, 9);
    assert!(save <= MENU_CONTENT_HEIGHT, "save slots: {save}px");
    assert!(manage <= MENU_CONTENT_HEIGHT, "item management: {manage}px");
    assert!(tags <= MENU_CONTENT_HEIGHT, "tag editor: {tags}px");

    // The teleport picker caps itself at 520px rather than filling the content box.
    let teleport = TELEPORT_VISIBLE_ROWS * 48
        + (TELEPORT_VISIBLE_ROWS - 1) * 9
        + 17
        + 15
        + panel_chrome(16, 2, 17, 9);
    assert!(teleport <= 520, "teleport picker: {teleport}px");
}

#[test]
fn status_visual_helpers_handle_empty_names_and_portrait_paths() {
    assert_eq!(member_emblem("Aric"), "AR");
    assert_eq!(member_emblem("Aric Vale"), "AV");
    assert_eq!(member_emblem("  "), "?");
    assert_eq!(
        profile_portrait_path("aric"),
        "media/images/aric_profile.png"
    );
    assert_eq!(
        large_status_portrait_path("aric"),
        "media/images/party_portraits_large/aric_status_portrait.webp"
    );
}

#[test]
fn status_page_builds_one_card_per_member_and_one_selection() {
    let game = fixture_game();
    let party_len = game.party().len();
    let mut app = App::new();
    app.insert_resource(game)
        .insert_resource(FieldMenuCatalog::default())
        .insert_resource(FieldMenuState {
            open: true,
            screen: FieldMenuScreen::Status,
            ..default()
        })
        .add_systems(Update, spawn_fixture_status_page);

    app.update();

    let world = app.world_mut();
    let cards = world.query::<&StatusMemberCard>().iter(world).count();
    let selected = world
        .query::<&SelectedStatusMemberCard>()
        .iter(world)
        .count();
    assert_eq!(cards, party_len);
    assert_eq!(selected, usize::from(party_len > 0));
}

#[test]
fn the_tag_and_manage_modals_render_their_rows() {
    let mut game = fixture_game();
    game.repository_mut()
        .add_tags("potion", ["favorite"])
        .unwrap();
    let mut app = App::new();
    app.insert_resource(game)
        .insert_resource(crate::field_menu_domain::tests::catalog())
        .insert_resource(FieldMenuState {
            open: true,
            screen: FieldMenuScreen::Items,
            mode: FieldMenuMode::ItemTags,
            pending_id: Some("potion".to_owned()),
            ..default()
        })
        .add_systems(Update, spawn_fixture_items_page);

    app.update();

    let world = app.world_mut();
    let labels = world
        .query::<&Text>()
        .iter(world)
        .map(|text| text.0.clone())
        .collect::<Vec<_>>();
    assert!(labels.iter().any(|label| label == "EDIT TAGS"));
    assert!(labels.iter().any(|label| label == "rare"));
    assert!(labels.iter().any(|label| label == "favorite"));
    assert!(labels.iter().any(|label| label == "New tag…"));
    assert!(labels.iter().any(|label| label == "ON"));

    app.world_mut().resource_mut::<FieldMenuState>().mode = FieldMenuMode::ItemManage;
    app.update();
    let world = app.world_mut();
    let labels = world
        .query::<&Text>()
        .iter(world)
        .map(|text| text.0.clone())
        .collect::<Vec<_>>();
    assert!(labels.iter().any(|label| label == "SHOW / HIDE"));
    // The fixture's potion is hidden, so the manager is the only place it still appears.
    assert!(labels.iter().any(|label| label == "HIDDEN"));
    assert!(labels.iter().any(|label| label == "Potion"));
}

#[test]
fn items_page_builds_the_original_three_column_pouch_structure() {
    let mut app = App::new();
    app.insert_resource(fixture_game())
        .insert_resource(FieldMenuCatalog::default())
        .insert_resource(FieldMenuState {
            open: true,
            screen: FieldMenuScreen::Items,
            ..default()
        })
        .add_systems(Update, spawn_fixture_items_page);

    app.update();

    let world = app.world_mut();
    assert_eq!(
        world.query::<&ItemPouchRow>().iter(world).count(),
        InventoryTab::ALL.len()
    );
    assert_eq!(world.query::<&ItemListRow>().iter(world).count(), 0);
    let labels = world
        .query::<&Text>()
        .iter(world)
        .map(|text| text.0.as_str())
        .collect::<Vec<_>>();
    assert!(labels.contains(&"POUCH"));
    assert!(labels.contains(&"ITEMS"));
    assert!(labels.contains(&"DETAIL"));
    assert!(labels.contains(&"Pouch is empty."));
}

#[test]
fn items_action_mode_uses_a_focused_modal_without_losing_the_columns() {
    let mut app = App::new();
    app.insert_resource(fixture_game())
        .insert_resource(FieldMenuCatalog::default())
        .insert_resource(FieldMenuState {
            open: true,
            screen: FieldMenuScreen::Items,
            mode: FieldMenuMode::ItemActions,
            pending_id: Some("potion".to_owned()),
            ..default()
        })
        .add_systems(Update, spawn_fixture_items_page);

    app.update();

    let world = app.world_mut();
    assert_eq!(world.query::<&ItemActionModal>().iter(world).count(), 1);
    assert_eq!(
        world.query::<&ItemPouchRow>().iter(world).count(),
        InventoryTab::ALL.len()
    );
    let labels = world
        .query::<&Text>()
        .iter(world)
        .map(|text| text.0.as_str())
        .collect::<Vec<_>>();
    assert!(labels.contains(&"Use"));
    assert!(labels.contains(&"Discard"));
    assert!(labels.contains(&"Hide"));
    assert!(labels.contains(&"Unlock"));
}

#[test]
fn item_details_show_repository_tags_and_locked_actions_offer_unlock() {
    let mut game = fixture_game();
    game.repository_mut()
        .add_tags("potion", ["travel"])
        .unwrap();
    game.repository_mut().set_locked("potion", true);
    let mut app = App::new();
    app.insert_resource(game)
        .insert_resource(crate::field_menu_domain::tests::catalog())
        .insert_resource(FieldMenuState {
            open: true,
            screen: FieldMenuScreen::Items,
            mode: FieldMenuMode::ItemActions,
            pending_id: Some("potion".to_owned()),
            ..default()
        })
        .add_systems(Update, spawn_fixture_items_page);

    app.update();

    let world = app.world_mut();
    let labels = world
        .query::<&Text>()
        .iter(world)
        .map(|text| text.0.as_str())
        .collect::<Vec<_>>();
    assert!(labels.contains(&"TRAVEL"));
    assert!(labels.contains(&"LOCKED"));
    assert!(labels.contains(&"Unlock"));
}

#[test]
fn moving_focus_between_equipment_columns_never_changes_a_frame_border() {
    for mode in [
        FieldMenuMode::Browse,
        FieldMenuMode::EquipmentSlots,
        FieldMenuMode::EquipmentPicker,
    ] {
        let mut app = App::new();
        app.insert_resource(fixture_game())
            .insert_resource(FieldMenuCatalog::default())
            .insert_resource(FieldMenuState {
                open: true,
                screen: FieldMenuScreen::Equipment,
                mode,
                quantity: 1,
                ..default()
            })
            .add_systems(Update, spawn_fixture_equipment_page);
        app.update();

        let world = app.world_mut();
        let frames = world
            .query::<(&Node, &Outline)>()
            .iter(world)
            .map(|(node, outline)| (node.border.top, outline.color))
            .collect::<Vec<_>>();
        assert_eq!(frames.len(), 3, "{mode:?}: party, slots, inventory");
        // Focus is the outline alone: every frame keeps the same 1px border, so nothing
        // inside reflows, and exactly one frame carries the bright rim.
        assert!(
            frames.iter().all(|(border, _)| *border == px(1)),
            "{mode:?}"
        );
        assert_eq!(
            frames
                .iter()
                .filter(|(_, color)| *color != Color::NONE)
                .count(),
            1,
            "{mode:?}"
        );
    }
}

#[test]
fn the_slot_cursor_appears_once_the_slot_column_has_focus() {
    let mut app = App::new();
    app.insert_resource(fixture_game())
        .insert_resource(FieldMenuCatalog::default())
        .insert_resource(FieldMenuState {
            open: true,
            screen: FieldMenuScreen::Equipment,
            mode: FieldMenuMode::EquipmentSlots,
            selected: 2,
            ..default()
        })
        .add_systems(Update, spawn_fixture_equipment_page);

    app.update();

    let world = app.world_mut();
    assert_eq!(
        world
            .query::<&SelectedEquipmentSlotRow>()
            .iter(world)
            .count(),
        1
    );
}

#[test]
fn equipment_page_reuses_party_cards_and_builds_slots_and_inventory_columns() {
    let game = fixture_game();
    let party_len = game.party().len();
    let mut app = App::new();
    app.insert_resource(game)
        .insert_resource(FieldMenuCatalog::default())
        .insert_resource(FieldMenuState {
            open: true,
            screen: FieldMenuScreen::Equipment,
            ..default()
        })
        .add_systems(Update, spawn_fixture_equipment_page);

    app.update();

    let world = app.world_mut();
    assert_eq!(
        world.query::<&StatusMemberCard>().iter(world).count(),
        party_len
    );
    assert_eq!(
        world.query::<&EquipmentSlotRow>().iter(world).count(),
        EquipmentSlot::ALL.len()
    );
    // The party column has focus, so no slot shows a cursor yet.
    assert_eq!(
        world
            .query::<&SelectedEquipmentSlotRow>()
            .iter(world)
            .count(),
        0
    );
    let labels = world
        .query::<&Text>()
        .iter(world)
        .map(|text| text.0.as_str())
        .collect::<Vec<_>>();
    assert!(labels.contains(&"PARTY"));
    assert!(labels.contains(&"SLOTS"));
    assert!(labels.contains(&"INVENTORY"));
    assert!(labels.contains(&"TOTALS"));
}

#[test]
fn equipment_picker_keeps_the_slot_context_and_renders_a_preview() {
    let mut app = App::new();
    app.insert_resource(fixture_game())
        .insert_resource(FieldMenuCatalog::default())
        .insert_resource(FieldMenuState {
            open: true,
            screen: FieldMenuScreen::Equipment,
            mode: FieldMenuMode::EquipmentPicker,
            quantity: 1,
            ..default()
        })
        .add_systems(Update, spawn_fixture_equipment_page);

    app.update();

    let world = app.world_mut();
    assert_eq!(world.query::<&EquipmentPickerRow>().iter(world).count(), 1);
    assert_eq!(
        world
            .query::<&SelectedEquipmentSlotRow>()
            .iter(world)
            .count(),
        1
    );
    let labels = world
        .query::<&Text>()
        .iter(world)
        .map(|text| text.0.as_str())
        .collect::<Vec<_>>();
    assert!(labels.contains(&"(Unequip)"));
    assert!(labels.contains(&"PREVIEW"));
}

#[test]
fn spells_page_reuses_party_cards_and_builds_the_spellbook_detail_layout() {
    let game = fixture_game();
    let party_len = game.party().len();
    let mut app = App::new();
    app.insert_resource(game)
        .insert_resource(FieldMenuCatalog::default())
        .insert_resource(FieldMenuState {
            open: true,
            screen: FieldMenuScreen::Spells,
            ..default()
        })
        .add_systems(Update, spawn_fixture_spells_page);

    app.update();

    let world = app.world_mut();
    assert_eq!(
        world.query::<&StatusMemberCard>().iter(world).count(),
        party_len
    );
    assert_eq!(world.query::<&SpellbookRow>().iter(world).count(), 0);
    let labels = world
        .query::<&Text>()
        .iter(world)
        .map(|text| text.0.as_str())
        .collect::<Vec<_>>();
    assert!(labels.contains(&"PARTY"));
    assert!(labels.contains(&"SPELLBOOK"));
    assert!(labels.contains(&"DETAIL"));
    assert!(labels.contains(&"No learned field abilities."));
}

#[test]
fn spell_target_mode_uses_a_party_overlay_without_hiding_the_spellbook() {
    let party_len = fixture_game().party().len();
    let mut app = App::new();
    app.insert_resource(fixture_game())
        .insert_resource(FieldMenuCatalog::default())
        .insert_resource(FieldMenuState {
            open: true,
            screen: FieldMenuScreen::Spells,
            mode: FieldMenuMode::SpellTarget,
            pending_id: Some("heal".to_owned()),
            ..default()
        })
        .add_systems(Update, spawn_fixture_spells_page);

    app.update();

    let world = app.world_mut();
    assert_eq!(world.query::<&SpellTargetOverlay>().iter(world).count(), 1);
    assert_eq!(
        world.query::<&StatusMemberCard>().iter(world).count(),
        party_len
    );
    let labels = world
        .query::<&Text>()
        .iter(world)
        .map(|text| text.0.as_str())
        .collect::<Vec<_>>();
    assert!(labels.contains(&"CHOOSE A TARGET"));
    assert!(labels.contains(&"SPELLBOOK"));
}

#[test]
fn teleport_mode_uses_a_destination_overlay_with_an_empty_state() {
    let mut app = App::new();
    app.insert_resource(fixture_game())
        .insert_resource(FieldMenuCatalog::default())
        .insert_resource(FieldMenuState {
            open: true,
            screen: FieldMenuScreen::Spells,
            mode: FieldMenuMode::TeleportPicker,
            pending_id: Some("teleport".to_owned()),
            ..default()
        })
        .add_systems(Update, spawn_fixture_spells_page);

    app.update();

    let world = app.world_mut();
    assert_eq!(world.query::<&SpellTargetOverlay>().iter(world).count(), 1);
    let labels = world
        .query::<&Text>()
        .iter(world)
        .map(|text| text.0.as_str())
        .collect::<Vec<_>>();
    assert!(labels.contains(&"CHOOSE A DESTINATION"));
    assert!(labels.contains(&"No eligible visited destinations."));
    assert!(labels.contains(&"SPELLBOOK"));
}

#[test]
fn save_page_builds_the_centered_modal_and_loading_state() {
    let mut app = App::new();
    app.insert_resource(fixture_game())
        .insert_resource(SaveSlotCatalog::default())
        .insert_resource(FieldMenuState {
            open: true,
            screen: FieldMenuScreen::Save,
            selected: FIRST_PLAYER_SLOT,
            ..default()
        })
        .add_systems(Update, spawn_fixture_save_page);

    app.update();

    let world = app.world_mut();
    assert_eq!(world.query::<&FieldMenuSavePage>().iter(world).count(), 1);
    assert_eq!(world.query::<&FieldSaveSlotRow>().iter(world).count(), 0);
    let labels = world
        .query::<&Text>()
        .iter(world)
        .map(|text| text.0.as_str())
        .collect::<Vec<_>>();
    assert!(labels.contains(&"SAVE GAME"));
    assert!(labels.contains(&"Discovering native save slots..."));
    assert!(labels.contains(&"SLOT 01 / 100"));
}

#[test]
fn save_slot_card_renders_metadata_and_selection() {
    let mut app = App::new();
    app.add_systems(Update, spawn_fixture_valid_save_row);

    app.update();

    let world = app.world_mut();
    assert_eq!(world.query::<&FieldSaveSlotRow>().iter(world).count(), 1);
    assert_eq!(
        world
            .query::<&SelectedFieldSaveSlotRow>()
            .iter(world)
            .count(),
        1
    );
    let labels = world
        .query::<&Text>()
        .iter(world)
        .map(|text| text.0.as_str())
        .collect::<Vec<_>>();
    assert!(labels.contains(&"SLOT 03"));
    assert!(labels.contains(&"Ardel    (Aric)"));
    assert!(labels.contains(&"LV 7      PLAYTIME 00d 01h 01m 01s"));
    assert!(labels.contains(&"SAVED"));
}

#[test]
fn save_overwrite_confirmation_uses_a_focused_modal() {
    let mut app = App::new();
    app.insert_resource(fixture_game())
        .insert_resource(SaveSlotCatalog::default())
        .insert_resource(FieldMenuState {
            open: true,
            screen: FieldMenuScreen::Save,
            mode: FieldMenuMode::SaveConfirm,
            selected: 4,
            ..default()
        })
        .add_systems(Update, spawn_fixture_save_page);

    app.update();

    let world = app.world_mut();
    assert_eq!(world.query::<&SaveOverwriteModal>().iter(world).count(), 1);
    let labels = world
        .query::<&Text>()
        .iter(world)
        .map(|text| text.0.as_str())
        .collect::<Vec<_>>();
    assert!(labels.contains(&"OVERWRITE SAVE?"));
    assert!(labels.contains(&"Slot 04"));
}

#[test]
fn save_window_scrolls_one_slot_at_a_time() {
    assert_eq!(save_window_start(1), 1);
    assert_eq!(save_window_start(SAVE_VISIBLE_ROWS), 1);
    assert_eq!(save_window_start(SAVE_VISIBLE_ROWS + 1), 2);
    // The last slot sits at the bottom of a full window.
    assert_eq!(save_window_start(100), 100 - SAVE_VISIBLE_ROWS + 1);
}

#[test]
fn status_details_replace_portrait_with_stats_and_categories() {
    let mut app = App::new();
    app.insert_resource(fixture_game())
        .insert_resource(FieldMenuCatalog::default())
        .insert_resource(FieldMenuState {
            open: true,
            screen: FieldMenuScreen::Status,
            status_page: StatusPage::Details,
            ..default()
        })
        .add_systems(Update, spawn_fixture_status_page);

    app.update();

    let world = app.world_mut();
    let labels = world
        .query::<&Text>()
        .iter(world)
        .map(|text| text.0.clone())
        .collect::<Vec<_>>();
    assert!(labels.iter().any(|label| label == "EXP"));
    assert!(labels.iter().any(|label| label == "HP"));
    assert!(labels.iter().any(|label| label == "Spells"));
    assert!(labels.iter().any(|label| label == "Position"));
    assert!(
        world
            .query::<&Name>()
            .iter(world)
            .all(|name| name.as_str() != "Full status portrait")
    );
}

#[test]
fn the_position_page_lists_both_rows_and_tags_the_current_one() {
    let mut app = App::new();
    app.insert_resource(fixture_game())
        .insert_resource(FieldMenuCatalog::default())
        .insert_resource(FieldMenuState {
            open: true,
            screen: FieldMenuScreen::Status,
            status_page: StatusPage::Position,
            ..default()
        })
        .add_systems(Update, spawn_fixture_status_page);

    app.update();

    let world = app.world_mut();
    let labels = world
        .query::<&Text>()
        .iter(world)
        .map(|text| text.0.clone())
        .collect::<Vec<_>>();
    assert!(labels.iter().any(|label| label == "Front"));
    assert!(labels.iter().any(|label| label == "Back"));
    assert!(labels.iter().any(|label| label == "CURRENT"));
    // The profile column yields to the picker while Position is open.
    assert!(!labels.iter().any(|label| label == "FIELD ARTS"));
}

#[test]
fn the_switch_overlay_lists_the_party_and_badges_the_controlled_member() {
    let mut app = App::new();
    app.insert_resource(fixture_game_with_recruit())
        .insert_resource(FieldMenuState {
            open: true,
            selected: 1,
            mode: FieldMenuMode::CharacterSwitch,
            ..default()
        })
        .add_systems(Update, spawn_fixture_main_page);

    app.update();

    let world = app.world_mut();
    assert_eq!(
        world
            .query::<&FieldMenuCharacterModal>()
            .iter(world)
            .count(),
        1
    );
    let labels = world
        .query::<&Text>()
        .iter(world)
        .map(|text| text.0.clone())
        .collect::<Vec<_>>();
    assert!(labels.iter().any(|label| label == "SWITCH CHARACTER"));
    assert!(labels.iter().any(|label| label == "Golden Aric"));
    assert!(labels.iter().any(|label| label == "Elise"));
    // Only the protagonist is controlled, so exactly one row carries the badge.
    assert_eq!(labels.iter().filter(|label| *label == "ACTIVE").count(), 1);
}

/// The reported bug: the deck showing through the switch overlay's translucent backdrop moved
/// too. `selected` is one cursor shared by every mode, so a roster index of 1 lit up command 1
/// behind the modal, and walking the party walked the deck's highlight down with it.
#[test]
fn the_deck_behind_the_switch_overlay_stays_on_the_character_command() {
    let highlighted = |member: usize| {
        let mut app = App::new();
        app.insert_resource(fixture_game_with_recruit())
            .insert_resource(FieldMenuState {
                open: true,
                selected: member,
                mode: FieldMenuMode::CharacterSwitch,
                ..default()
            })
            .add_systems(Update, spawn_fixture_main_page);
        app.update();

        let world = app.world_mut();
        world
            .query::<(&SelectedMainCommandRow, &Name)>()
            .iter(world)
            .map(|(_, name)| name.to_string())
            .collect::<Vec<_>>()
    };

    for member in 0..2 {
        assert_eq!(
            highlighted(member),
            vec!["Character command".to_owned()],
            "roster row {member} must not drag the deck's highlight with it"
        );
    }

    // Browsing still steers the deck itself — the pin is scoped to the modal.
    let mut app = App::new();
    app.insert_resource(fixture_game_with_recruit())
        .insert_resource(FieldMenuState {
            open: true,
            selected: 1,
            ..default()
        })
        .add_systems(Update, spawn_fixture_main_page);
    app.update();
    let world = app.world_mut();
    let browsing = world
        .query::<(&SelectedMainCommandRow, &Name)>()
        .iter(world)
        .map(|(_, name)| name.to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        browsing,
        vec![format!("{} command", MAIN_COMMANDS[1].label)]
    );
}

#[test]
fn main_page_matches_the_source_command_deck_structure() {
    // The first eight are the source deck in source order. `Recipes` is appended rather than
    // inserted (roadmap B4.4) so every index the deck constants pin stays where it was.
    assert_eq!(
        MAIN_COMMANDS.map(|command| command.label),
        [
            "Status",
            "Spells",
            "Items",
            "Quit",
            "Save",
            "Equipment",
            "Quests",
            "Character",
            "Recipes",
            "Battle"
        ]
    );
    assert_eq!(MAIN_COMMANDS[QUIT_COMMAND_INDEX].label, "Quit");
    assert_eq!(MAIN_COMMANDS[SAVE_COMMAND_INDEX].label, "Save");
    assert_eq!(MAIN_COMMANDS[CHARACTER_COMMAND_INDEX].label, "Character");

    let mut app = App::new();
    app.insert_resource(fixture_game())
        .insert_resource(FieldMenuState {
            open: true,
            selected: 1,
            ..default()
        })
        .add_systems(Update, spawn_fixture_main_page);

    app.update();

    let world = app.world_mut();
    assert_eq!(world.query::<&FieldMenuMainPage>().iter(world).count(), 1);
    assert_eq!(
        world.query::<&MainCommandRow>().iter(world).count(),
        MAIN_COMMANDS.len()
    );
    assert_eq!(
        world.query::<&SelectedMainCommandRow>().iter(world).count(),
        1
    );
    let labels = world
        .query::<&Text>()
        .iter(world)
        .map(|text| text.0.as_str())
        .collect::<Vec<_>>();
    assert!(labels.contains(&"FIELD MENU"));
    assert!(labels.contains(&"PARTY COMMAND DECK"));
    assert!(labels.contains(&"cast field magic and utilities"));
    assert!(labels.contains(&"exit the game to desktop"));
    // No icon sheet: every command keeps its letter badge.
    assert!(labels.contains(&"ST"));
    assert!(labels.contains(&"BT"));
    assert_eq!(world.query::<&MainCommandIcon>().iter(world).count(), 0);
}

fn spawn_fixture_main_page_with_icons(
    mut commands: Commands,
    state: Res<FieldMenuState>,
    game: Res<GameState>,
) {
    commands.spawn(Node::default()).with_children(|parent| {
        spawn_main_menu_page(
            parent,
            &Handle::<Font>::default(),
            Some(&MainCommandIcons::test_stub()),
            &state,
            &game,
            &PartyWalkThumbnails::default(),
            false,
        );
    });
}

#[test]
fn an_icon_sheet_replaces_every_letter_badge_with_its_own_tinted_cell() {
    // Each command owns a distinct cell, in deck order, so the sheet is authored once.
    assert_eq!(
        MAIN_COMMANDS.map(|command| command.icon),
        std::array::from_fn(|index| index)
    );

    let mut app = App::new();
    app.insert_resource(fixture_game())
        .insert_resource(FieldMenuState {
            open: true,
            ..default()
        })
        .add_systems(Update, spawn_fixture_main_page_with_icons);

    app.update();

    let world = app.world_mut();
    let mut cells = world
        .query_filtered::<&ImageNode, With<MainCommandIcon>>()
        .iter(world)
        .map(|image| {
            (
                image.texture_atlas.as_ref().map(|atlas| atlas.index),
                image.color,
            )
        })
        .collect::<Vec<_>>();
    cells.sort_by_key(|(index, _)| *index);
    assert_eq!(cells.len(), MAIN_COMMANDS.len());
    for (index, (cell, color)) in cells.into_iter().enumerate() {
        assert_eq!(cell, Some(index));
        assert_eq!(color, main_command_accent(index));
    }
    let labels = world
        .query::<&Text>()
        .iter(world)
        .map(|text| text.0.as_str())
        .collect::<Vec<_>>();
    assert!(!labels.contains(&"ST"));
    assert!(!labels.contains(&"BT"));
}

#[test]
fn the_battle_page_lists_each_member_with_mode_and_command() {
    let mut game = fixture_game_with_recruit();
    game.party_mut().member_mut("elise").unwrap().set_tactics(
        crate::battle_tactics::BattleTactics {
            auto: true,
            command: crate::battle_tactics::AutoCommand::Smart,
        },
    );
    let mut app = App::new();
    app.insert_resource(game)
        .insert_resource(crate::field_menu_domain::tests::catalog())
        .insert_resource(FieldMenuState {
            open: true,
            screen: FieldMenuScreen::Battle,
            selected: 1,
            mode: FieldMenuMode::BattleTacticPicker,
            battle_choice: 2,
            ..default()
        })
        .add_systems(Update, spawn_fixture_battle_page);

    app.update();

    let world = app.world_mut();
    assert_eq!(world.query::<&FieldMenuBattlePage>().iter(world).count(), 1);
    // Two members, then Enemy turns, Turn pause, After victory, and Last battle.
    assert_eq!(world.query::<&BattleTacticsRow>().iter(world).count(), 6);
    assert_eq!(
        world
            .query::<&SelectedBattleTacticsRow>()
            .iter(world)
            .count(),
        1
    );
    let labels = world
        .query::<&Text>()
        .iter(world)
        .map(|text| text.0.clone())
        .collect::<Vec<_>>();
    assert!(labels.iter().any(|label| label == "BATTLE TACTICS"));
    assert!(labels.iter().any(|label| label == "Enemy turns"));
    assert!(labels.iter().any(|label| label == "Turn pause"));
    assert!(labels.iter().any(|label| label == "1S"));
    assert!(labels.iter().any(|label| label == "SHOW REWARDS"));
    assert!(labels.iter().any(|label| label == "NO BATTLE FOUGHT YET"));
    assert!(labels.iter().any(|label| label == "MANUAL"));
    assert!(labels.iter().any(|label| label == "AUTO - SMART"));
    // The picker lists every choice, with the highlighted one's explanation beneath.
    for choice in TACTIC_CHOICES {
        assert!(labels.contains(&crate::battle_tactics::choice_label(choice)));
    }
    assert!(
        labels
            .iter()
            .any(|label| label == AutoCommand::Spell.description())
    );
}

fn quest_board_fades(selected: usize) -> (usize, Vec<crate::menu_chrome::ScrollFade>) {
    fn spawn_board(
        mut commands: Commands,
        state: Res<FieldMenuState>,
        game: Res<GameState>,
        catalog: Res<FieldMenuCatalog>,
    ) {
        commands.spawn(Node::default()).with_children(|parent| {
            spawn_quests_page(parent, &Handle::<Font>::default(), &state, &game, &catalog);
        });
    }
    let mut app = App::new();
    app.insert_resource(fixture_game())
        .insert_resource(crate::field_menu_domain::tests::catalog_with_quests())
        .insert_resource(FieldMenuState {
            open: true,
            screen: FieldMenuScreen::Quests,
            selected,
            ..default()
        })
        .add_systems(Update, spawn_board);
    app.update();
    let world = app.world_mut();
    let rows = world.query::<&QuestBoardRow>().iter(world).count();
    let mut fades = world
        .query::<&crate::menu_chrome::ScrollFade>()
        .iter(world)
        .copied()
        .collect::<Vec<_>>();
    fades.sort_by_key(|fade| *fade as u8);
    (rows, fades)
}

#[test]
fn a_scrolled_quest_board_fills_its_window_and_fades_the_edges_with_hidden_cards() {
    use crate::menu_chrome::ScrollFade::{Bottom, Top};
    let len = crate::field_menu_domain::tests::catalog_with_quests()
        .quests()
        .len();
    assert!(len > QUEST_VISIBLE_ROWS, "the shipped register must scroll");

    // Top of the list: a full window, shaded only toward the cards below.
    assert_eq!(quest_board_fades(0), (QUEST_VISIBLE_ROWS, vec![Bottom]));
    // Mid-list: cards hidden on both sides.
    assert_eq!(
        quest_board_fades(QUEST_VISIBLE_ROWS),
        (QUEST_VISIBLE_ROWS, vec![Top, Bottom])
    );
    // End of the list: only the cards above are hidden.
    assert_eq!(quest_board_fades(len - 1), (QUEST_VISIBLE_ROWS, vec![Top]));
}

fn fixture_battle_log(lines: usize) -> LastBattleLog {
    let mut log = LastBattleLog::default();
    for index in 0..lines {
        log.push(&format!("Line {}", index + 1), index % 2 == 1);
    }
    log
}

#[test]
fn the_last_battle_row_previews_the_end_of_the_log() {
    let game = fixture_game_with_recruit();
    let last_row = battle_row_count(&game) - 1;
    let mut app = App::new();
    app.insert_resource(game)
        .insert_resource(crate::field_menu_domain::tests::catalog())
        .insert_resource(fixture_battle_log(25))
        .insert_resource(FieldMenuState {
            open: true,
            screen: FieldMenuScreen::Battle,
            selected: last_row,
            ..default()
        })
        .add_systems(Update, spawn_fixture_battle_page);

    app.update();

    let world = app.world_mut();
    let labels = world
        .query::<&Text>()
        .iter(world)
        .map(|text| text.0.clone())
        .collect::<Vec<_>>();
    assert!(labels.iter().any(|label| label == "LAST BATTLE"));
    assert!(labels.iter().any(|label| label == "25 LOG LINES"));
    assert!(labels.iter().any(|label| label == "LINES 2-25 OF 25"));
    assert!(labels.iter().any(|label| label == "Line 25"));
    assert!(!labels.iter().any(|label| label == "Line 1"));
    // The log replaces the orders panel rather than sitting beside it.
    assert!(!labels.iter().any(|label| label == "ORDERS"));
}

#[test]
fn reading_the_log_starts_at_its_top_and_scrolls_no_further_than_its_end() {
    let game = fixture_game_with_recruit();
    let last_row = battle_row_count(&game) - 1;
    let mut app = App::new();
    app.insert_resource(game)
        .insert_resource(crate::field_menu_domain::tests::catalog())
        .insert_resource(fixture_battle_log(25))
        .insert_resource(FieldMenuState {
            open: true,
            screen: FieldMenuScreen::Battle,
            selected: last_row,
            mode: FieldMenuMode::BattleLog,
            ..default()
        })
        .add_systems(Update, spawn_fixture_battle_page);

    app.update();

    let world = app.world_mut();
    let labels = world
        .query::<&Text>()
        .iter(world)
        .map(|text| text.0.clone())
        .collect::<Vec<_>>();
    assert!(labels.iter().any(|label| label == "LINES 1-24 OF 25"));
    assert!(labels.iter().any(|label| label == "Line 1"));
    assert!(!labels.iter().any(|label| label == "Line 25"));

    assert_eq!(battle_log_last_scroll(25), 1);
    assert_eq!(battle_log_last_scroll(BATTLE_LOG_VISIBLE_LINES), 0);
    assert_eq!(battle_log_last_scroll(0), 0);
    let mut state = FieldMenuState {
        open: true,
        screen: FieldMenuScreen::Battle,
        mode: FieldMenuMode::BattleLog,
        ..default()
    };
    state.back();
    assert_eq!(state.mode, FieldMenuMode::Browse);
    assert_eq!(state.screen, FieldMenuScreen::Battle);
}

/// Every recipe is listed so the tally is honest, but an unmet unlock flag keeps the formula
/// itself unreadable. A new game bootstraps `story_quest_started`, so the Act 1 recipes are
/// already legible while the later acts stay sealed.
#[test]
fn the_recipe_book_lists_the_catalog_and_withholds_sealed_formulae() {
    let catalog = crate::field_menu_domain::tests::catalog();
    let recipe_count = catalog.recipes().len();
    assert!(recipe_count > 0, "the shipped catalog must carry recipes");

    let mut app = App::new();
    app.insert_resource(fixture_game())
        .insert_resource(catalog)
        .insert_resource(FieldMenuState {
            open: true,
            screen: FieldMenuScreen::Recipes,
            ..default()
        })
        .add_systems(Update, spawn_fixture_recipes_page);

    app.update();

    let world = app.world_mut();
    assert_eq!(
        world.query::<&FieldMenuRecipesPage>().iter(world).count(),
        1
    );
    assert_eq!(
        world.query::<&RecipeBookRow>().iter(world).count(),
        RECIPE_VISIBLE_ROWS.min(recipe_count)
    );
    assert_eq!(
        world.query::<&SelectedRecipeBookRow>().iter(world).count(),
        1
    );
    let labels = world
        .query::<&Text>()
        .iter(world)
        .map(|text| text.0.clone())
        .collect::<Vec<_>>();
    assert!(labels.iter().any(|label| label == "RECIPE BOOK"));
    assert!(labels.iter().any(|label| label == "Sealed Formula"));
    assert!(labels.iter().any(|label| label == "SEALED"));
    assert!(
        labels
            .iter()
            .any(|label| label == "Brewing is done at an apothecary."),
        "the book must say where crafting actually happens"
    );
    // Act 1 is bootstrapped, so its formula reads normally.
    assert!(labels.iter().any(|label| label == "Purging Salve"));
    // Act 2 is not, so its scroll name must not appear anywhere on the page.
    assert!(
        !labels.iter().any(|label| label == "Vital Brew"),
        "a sealed formula must not leak its scroll name: {labels:?}"
    );
}

#[test]
fn an_unsealed_recipe_shows_its_ingredients_against_what_the_party_carries() {
    let mut game = fixture_game();
    game.flags_mut().set("story_quest_started");
    let mut app = App::new();
    app.insert_resource(game)
        .insert_resource(crate::field_menu_domain::tests::catalog())
        .insert_resource(FieldMenuState {
            open: true,
            screen: FieldMenuScreen::Recipes,
            ..default()
        })
        .add_systems(Update, spawn_fixture_recipes_page);

    app.update();

    let world = app.world_mut();
    let labels = world
        .query::<&Text>()
        .iter(world)
        .map(|text| text.0.clone())
        .collect::<Vec<_>>();
    assert!(labels.iter().any(|label| label == "Purging Salve"));
    // The fixture carries none of the salve's inputs, so both read as carried zero.
    for expected in ["Venom Sac", "NEED 2", "HAVE 0"] {
        assert!(
            labels.iter().any(|label| label == expected),
            "ingredient rows must show the required and carried counts: {labels:?}"
        );
    }
}

/// The acceptance condition for roadmap B4.4: the book is a reference, not a second workshop.
#[test]
fn confirming_in_the_recipe_book_never_crafts_anything() {
    use crate::action_input::ActionInputPlugin;

    let mut game = fixture_game();
    // Unseal an Act 1 recipe and stock every input, so the only thing standing between the
    // player and a craft is the absence of a crafting path on this screen.
    game.flags_mut().set("story_quest_started");
    let batch = game.repository_mut().start_loot_batch();
    let _ = game
        .repository_mut()
        .add_item_in_batch("venom_sac", 9, batch)
        .unwrap();
    let _ = game
        .repository_mut()
        .add_item_in_batch("herb_red", 9, batch)
        .unwrap();
    let before_gp = game.repository().gp();
    let before_remedy = game.repository().item_count("remedy");
    let before_venom = game.repository().item_count("venom_sac");

    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<ButtonInput<KeyCode>>()
        .add_plugins(ActionInputPlugin)
        .insert_resource(crate::field_menu_domain::tests::catalog())
        .insert_resource(WorldInteractionState::default())
        .init_resource::<EngineSettings>()
        .insert_resource(WorldTransition::idle_for_test())
        .insert_resource(game)
        .insert_resource(SaveStore::new(
            std::env::temp_dir().join("rpg-s1-recipe-book-test-unused"),
        ))
        .insert_resource(SaveSlotCatalog::default())
        .insert_resource(Time::<Real>::default())
        .init_resource::<ServiceUiState>()
        .insert_resource(FieldMenuState {
            open: true,
            screen: FieldMenuScreen::Recipes,
            ..default()
        })
        .add_message::<AppExit>()
        .add_message::<PlaySfx>()
        .add_message::<KeyboardInput>()
        .add_systems(Update, handle_field_menu_input);

    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    app.update();

    let state = app.world().resource::<FieldMenuState>();
    assert_eq!(
        state.screen,
        FieldMenuScreen::Recipes,
        "Confirm must not navigate out of the book"
    );
    assert_eq!(
        state.mode,
        FieldMenuMode::Browse,
        "Confirm must not open a crafting mode"
    );
    let repository = app.world().resource::<GameState>().repository();
    assert_eq!(repository.gp(), before_gp, "Confirm must not spend GP");
    assert_eq!(
        repository.item_count("remedy"),
        before_remedy,
        "Confirm must not produce the recipe output"
    );
    assert_eq!(
        repository.item_count("venom_sac"),
        before_venom,
        "Confirm must not consume ingredients"
    );
}

#[test]
fn the_recipe_book_cursor_wraps_across_the_whole_catalog() {
    use crate::action_input::ActionInputPlugin;

    let catalog = crate::field_menu_domain::tests::catalog();
    let last = catalog.recipes().len() - 1;
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<ButtonInput<KeyCode>>()
        .add_plugins(ActionInputPlugin)
        .insert_resource(catalog)
        .insert_resource(WorldInteractionState::default())
        .init_resource::<EngineSettings>()
        .insert_resource(WorldTransition::idle_for_test())
        .insert_resource(fixture_game())
        .insert_resource(SaveStore::new(
            std::env::temp_dir().join("rpg-s1-recipe-cursor-test-unused"),
        ))
        .insert_resource(SaveSlotCatalog::default())
        .insert_resource(Time::<Real>::default())
        .init_resource::<ServiceUiState>()
        .insert_resource(FieldMenuState {
            open: true,
            screen: FieldMenuScreen::Recipes,
            ..default()
        })
        .add_message::<AppExit>()
        .add_message::<PlaySfx>()
        .add_message::<KeyboardInput>()
        .add_systems(Update, handle_field_menu_input);

    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowUp);
    app.update();

    assert_eq!(
        app.world().resource::<FieldMenuState>().selected,
        last,
        "stepping up from the first recipe wraps to the last"
    );
}

/// The field menu's input system alone, over `game`, starting from `state`.
fn menu_input_app(game: GameState, state: FieldMenuState) -> App {
    use crate::action_input::ActionInputPlugin;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<ButtonInput<KeyCode>>()
        .add_plugins(ActionInputPlugin)
        .insert_resource(crate::field_menu_domain::tests::catalog())
        .insert_resource(WorldInteractionState::default())
        .init_resource::<EngineSettings>()
        .insert_resource(WorldTransition::idle_for_test())
        .insert_resource(game)
        .insert_resource(SaveStore::new(
            std::env::temp_dir().join("rpg-s1-menu-nav-test-unused"),
        ))
        .insert_resource(SaveSlotCatalog::default())
        .insert_resource(Time::<Real>::default())
        .init_resource::<ServiceUiState>()
        .insert_resource(state)
        .add_message::<AppExit>()
        .add_message::<PlaySfx>()
        .add_message::<KeyboardInput>()
        .add_systems(Update, handle_field_menu_input);
    app.update();
    app
}

#[test]
fn rejected_options_survive_every_field_menu_battle_setting_change() {
    let root = std::env::temp_dir().join(format!("rpg-s1-rf04-field-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let store = OptionsStore::new(root.clone());
    // A newer document with an unknown row is rejected as Parse by the strict schema.
    let original = b"version: 99\nfuture_option: preserve\n";
    std::fs::write(store.path(), original).unwrap();
    let (options, status) = crate::options_store::load_player_options(&store);
    let game = crate::field_menu_domain::tests::game([]);
    let members = game.party().len();
    let mut app = menu_input_app(game, FieldMenuState::default());
    app.insert_resource(store.clone())
        .insert_resource(options)
        .insert_resource(status);
    for (offset, choice) in [(0, 1), (1, 0), (2, 1)] {
        *app.world_mut().resource_mut::<FieldMenuState>() = FieldMenuState {
            open: true,
            screen: FieldMenuScreen::Battle,
            mode: FieldMenuMode::BattleTacticPicker,
            selected: members + offset,
            battle_choice: choice,
            ..default()
        };
        tap(&mut app, KeyCode::Enter);
        assert_eq!(std::fs::read(store.path()).unwrap(), original);
        let message = &app.world().resource::<FieldMenuState>().message;
        assert!(message.contains("Changes apply only for this session"));
        assert!(message.contains("Reset To Defaults"));
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release_all();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.update();
    }
    let options = app.world().resource::<PlayerOptions>();
    assert!(options.auto_enemy_turns);
    assert_eq!(options.turn_pause, TurnPause::Instant);
    assert!(options.skip_victory_screen);
    assert_eq!(
        std::fs::read_dir(&root).unwrap().count(),
        1,
        "normal edits must not even create a backup"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn battle_option_write_failure_reports_active_choice_in_persistent_status() {
    let root = std::env::temp_dir().join(format!("rpg-s1-rf05-field-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let mut store = OptionsStore::new(root.clone());
    store.store(&PlayerOptions::default()).unwrap();
    let original = std::fs::read(store.path()).unwrap();
    store.fail_operation = Some("write temporary options");
    let game = crate::field_menu_domain::tests::game([]);
    let members = game.party().len();
    let mut app = menu_input_app(game, FieldMenuState::default());
    app.insert_resource(store.clone())
        .insert_resource(PlayerOptions::default())
        .init_resource::<OptionsLoadStatus>()
        .init_resource::<PersistenceStatus>();
    for (offset, choice) in [(0, 1), (1, 0), (2, 1)] {
        *app.world_mut().resource_mut::<FieldMenuState>() = FieldMenuState {
            open: true,
            screen: FieldMenuScreen::Battle,
            mode: FieldMenuMode::BattleTacticPicker,
            selected: members + offset,
            battle_choice: choice,
            ..default()
        };
        tap(&mut app, KeyCode::Enter);
        assert_eq!(std::fs::read(store.path()).unwrap(), original);
        let notice = &app.world().resource::<PersistenceStatus>().options;
        assert!(notice.failed);
        assert!(notice.message.contains("active for this session"));
        assert!(notice.message.contains("write temporary options"));
        assert_eq!(
            app.world().resource::<FieldMenuState>().message,
            notice.message
        );
    }
    let options = app.world().resource::<PlayerOptions>();
    assert!(options.auto_enemy_turns);
    assert_eq!(options.turn_pause, TurnPause::Instant);
    assert!(options.skip_victory_screen);
    app.world_mut()
        .resource_mut::<FieldMenuState>()
        .message
        .clear();
    assert!(app.world().resource::<PersistenceStatus>().options.failed);
    std::fs::remove_dir_all(root).unwrap();
}

/// Presses `key` for one frame and reports where the menu landed.
fn tap(app: &mut App, key: KeyCode) -> (FieldMenuScreen, FieldMenuMode, usize, usize) {
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release_all();
        keys.clear();
        keys.press(key);
    }
    app.update();
    let state = app.world().resource::<FieldMenuState>();
    (state.screen, state.mode, state.member_index, state.selected)
}

#[test]
fn equipment_arrows_walk_party_then_slots_then_inventory_and_back() {
    use FieldMenuMode::{Browse, EquipmentPicker, EquipmentSlots};
    use FieldMenuScreen::{Equipment, Main};
    let mut app = menu_input_app(
        fixture_game_with_recruit(),
        FieldMenuState {
            open: true,
            screen: Equipment,
            ..default()
        },
    );
    let slot = |app: &App| state_slot_index(app.world().resource::<FieldMenuState>());

    // The party column takes Up/Down; Right steps into the slots.
    assert_eq!(tap(&mut app, KeyCode::ArrowDown), (Equipment, Browse, 1, 0));
    assert_eq!(
        tap(&mut app, KeyCode::ArrowRight),
        (Equipment, EquipmentSlots, 1, 0)
    );
    assert_eq!(
        tap(&mut app, KeyCode::ArrowDown),
        (Equipment, EquipmentSlots, 1, 1)
    );
    // Right opens that slot's inventory; Left comes back to the same slot, then the party.
    assert_eq!(tap(&mut app, KeyCode::ArrowRight).1, EquipmentPicker);
    assert_eq!(slot(&app), 1);
    assert_eq!(
        tap(&mut app, KeyCode::ArrowLeft),
        (Equipment, EquipmentSlots, 1, 1)
    );
    assert_eq!(tap(&mut app, KeyCode::ArrowLeft), (Equipment, Browse, 1, 1));
    // Enter also moves right, and Esc retraces the same columns before leaving the page.
    assert_eq!(
        tap(&mut app, KeyCode::Enter),
        (Equipment, EquipmentSlots, 1, 1)
    );
    assert_eq!(tap(&mut app, KeyCode::Escape), (Equipment, Browse, 1, 1));
    assert_eq!(tap(&mut app, KeyCode::Escape).0, Main);
}

#[test]
fn spells_arrows_walk_casters_then_the_spellbook_and_back() {
    use FieldMenuMode::{Browse, SpellList};
    use FieldMenuScreen::{Main, Spells};
    let mut app = menu_input_app(
        fixture_game_with_recruit(),
        FieldMenuState {
            open: true,
            screen: Spells,
            ..default()
        },
    );

    // Up/Down changes caster; Left/Right no longer do.
    assert_eq!(tap(&mut app, KeyCode::ArrowDown), (Spells, Browse, 1, 0));
    assert_eq!(tap(&mut app, KeyCode::ArrowLeft), (Spells, Browse, 1, 0));
    assert_eq!(
        tap(&mut app, KeyCode::ArrowRight),
        (Spells, SpellList, 1, 0)
    );
    assert_eq!(tap(&mut app, KeyCode::ArrowLeft), (Spells, Browse, 1, 0));
    assert_eq!(tap(&mut app, KeyCode::Enter), (Spells, SpellList, 1, 0));
    assert_eq!(tap(&mut app, KeyCode::Escape), (Spells, Browse, 1, 0));
    assert_eq!(tap(&mut app, KeyCode::Escape).0, Main);
}

/// Every `Text` under `entity`, depth first.
fn texts_under(world: &World, entity: Entity) -> Vec<String> {
    let mut texts = world
        .get::<Text>(entity)
        .map(|text| vec![text.0.clone()])
        .unwrap_or_default();
    if let Some(children) = world.get::<Children>(entity) {
        for child in children.iter() {
            texts.extend(texts_under(world, child));
        }
    }
    texts
}

#[test]
fn the_item_list_cursor_highlights_the_item_it_is_on_without_opening_a_modal() {
    let mut game = fixture_game();
    for id in ["potion", "hi_potion"] {
        let _outcome = game.repository_mut().add_item(id, 2).unwrap();
        game.repository_mut().set_hidden(id, false);
    }
    let catalog = crate::field_menu_domain::tests::catalog();
    let ids = inventory_ids(&game, &catalog, InventoryTab::ALL[0]);
    let second = item_name(catalog.item(ids[1]).unwrap()).to_owned();
    let mut app = App::new();
    app.insert_resource(game)
        .insert_resource(catalog)
        .insert_resource(FieldMenuState {
            open: true,
            screen: FieldMenuScreen::Items,
            mode: FieldMenuMode::ItemList,
            selected: 1,
            ..default()
        })
        .add_systems(Update, spawn_fixture_items_page);
    app.update();

    let world = app.world_mut();
    let selected = world
        .query_filtered::<Entity, With<SelectedItemListRow>>()
        .iter(world)
        .collect::<Vec<_>>();
    assert_eq!(selected.len(), 1);
    assert!(texts_under(world, selected[0]).contains(&second));
    let labels = world
        .query::<&Text>()
        .iter(world)
        .map(|text| text.0.clone())
        .collect::<Vec<_>>();
    // No actions modal: browsing the list must not open one for the item under the cursor.
    assert!(!labels.iter().any(|label| label == "remove from pouch"));
}

#[test]
fn the_spellbook_cursor_follows_the_list_until_a_modal_takes_it() {
    let catalog = crate::field_menu_domain::tests::catalog();
    let class = catalog.class("cleric").expect("the cleric ships");
    let abilities = class.abilities.iter().take(2).collect::<Vec<_>>();
    assert_eq!(abilities.len(), 2);
    let mut state = FieldMenuState {
        open: true,
        screen: FieldMenuScreen::Spells,
        mode: FieldMenuMode::SpellList,
        selected: 1,
        ..default()
    };
    assert_eq!(selected_spell_index(&state, &abilities), 1);
    // Browsing casters still reads the list cursor.
    state.mode = FieldMenuMode::Browse;
    assert_eq!(selected_spell_index(&state, &abilities), 1);
    // A target modal owns `selected`; the spellbook shows the spell being cast instead.
    state.mode = FieldMenuMode::SpellTarget;
    state.selected = 0;
    state.pending_id = Some(abilities[1].id.clone());
    assert_eq!(selected_spell_index(&state, &abilities), 1);
}

#[test]
fn status_arrows_step_between_the_roster_and_a_members_details() {
    let mut app = menu_input_app(
        fixture_game_with_recruit(),
        FieldMenuState {
            open: true,
            screen: FieldMenuScreen::Status,
            ..default()
        },
    );
    let page = |app: &App| app.world().resource::<FieldMenuState>().status_page;

    assert_eq!(tap(&mut app, KeyCode::ArrowDown).2, 1);
    assert_eq!(page(&app), StatusPage::Roster);
    tap(&mut app, KeyCode::ArrowRight);
    assert_eq!(page(&app), StatusPage::Details);
    tap(&mut app, KeyCode::ArrowLeft);
    assert_eq!(page(&app), StatusPage::Roster);
    assert_eq!(
        app.world().resource::<FieldMenuState>().screen,
        FieldMenuScreen::Status
    );
}

#[test]
fn items_arrows_walk_pouch_then_items_and_actions_return_to_the_same_item() {
    use FieldMenuMode::{Browse, ItemActions, ItemList};
    use FieldMenuScreen::{Items, Main};
    let mut game = fixture_game();
    for id in ["potion", "hi_potion"] {
        let _outcome = game.repository_mut().add_item(id, 2).unwrap();
        // The fixture save hides its potions; this pouch needs them listed.
        game.repository_mut().set_hidden(id, false);
    }
    let catalog = crate::field_menu_domain::tests::catalog();
    let stocked = inventory_ids(&game, &catalog, InventoryTab::ALL[0]).len();
    assert!(
        stocked >= 2,
        "the fixture pouch needs two items to move between"
    );
    let mut app = menu_input_app(
        game,
        FieldMenuState {
            open: true,
            screen: Items,
            ..default()
        },
    );
    let tab = |app: &App| app.world().resource::<FieldMenuState>().tab_index;

    // The pouch column takes Up/Down, which changes the tab.
    assert_eq!(tap(&mut app, KeyCode::ArrowDown).1, Browse);
    assert_eq!(tab(&app), 1);
    assert_eq!(tap(&mut app, KeyCode::ArrowUp).1, Browse);
    assert_eq!(tab(&app), 0);
    // Right enters the list; Up/Down then moves between items, not tabs.
    assert_eq!(tap(&mut app, KeyCode::ArrowRight), (Items, ItemList, 0, 0));
    assert_eq!(tap(&mut app, KeyCode::ArrowDown), (Items, ItemList, 0, 1));
    assert_eq!(tab(&app), 0);
    // Leaving the actions modal puts the cursor back on the item it was opened from.
    assert_eq!(tap(&mut app, KeyCode::Enter).1, ItemActions);
    assert_eq!(tap(&mut app, KeyCode::Escape), (Items, ItemList, 0, 1));
    assert_eq!(tap(&mut app, KeyCode::ArrowLeft), (Items, Browse, 0, 1));
    assert_eq!(tap(&mut app, KeyCode::Escape).0, Main);
}

#[test]
fn quit_confirmation_is_a_focused_desktop_exit_modal() {
    let mut app = App::new();
    app.insert_resource(fixture_game())
        .insert_resource(FieldMenuState {
            open: true,
            selected: QUIT_COMMAND_INDEX,
            mode: FieldMenuMode::QuitConfirm,
            ..default()
        })
        .add_systems(Update, spawn_fixture_main_page);

    app.update();

    let world = app.world_mut();
    assert_eq!(world.query::<&FieldMenuQuitModal>().iter(world).count(), 1);
    let labels = world
        .query::<&Text>()
        .iter(world)
        .map(|text| text.0.as_str())
        .collect::<Vec<_>>();
    assert!(labels.contains(&"QUIT GAME?"));
    assert!(labels.contains(&"Exit to desktop?"));
    assert!(labels.contains(&"ENTER / Y   CONFIRM      ESC / N   CANCEL"));
}

/// A Back press that closes an overlay is spent. Before the close latch, the same press
/// reached the world and popped the field menu, so cancelling any shop, inn, apothecary,
/// or dialogue left the player staring at the command deck.
#[test]
fn a_back_press_that_closes_a_service_never_reaches_the_field_menu() {
    use crate::{
        action_input::ActionInputPlugin,
        service_ui::{ServiceRequest, handle_service_input},
    };

    for service_runs_first in [true, false] {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .add_plugins(ActionInputPlugin)
            .insert_resource(FieldMenuCatalog::default())
            .insert_resource(WorldInteractionState::default())
            .init_resource::<EngineSettings>()
            .insert_resource(WorldTransition::idle_for_test())
            .insert_resource(fixture_game())
            .insert_resource(SaveStore::new(
                std::env::temp_dir().join("rpg-s1-service-back-test-unused"),
            ))
            .insert_resource(SaveSlotCatalog::default())
            .insert_resource(Time::<Real>::default())
            .init_resource::<ServiceUiState>()
            .init_resource::<FieldMenuState>()
            .add_message::<AppExit>()
            .add_message::<PlaySfx>()
            .add_message::<KeyboardInput>();
        if service_runs_first {
            app.add_systems(
                Update,
                (handle_service_input, handle_field_menu_input).chain(),
            );
        } else {
            app.add_systems(
                Update,
                (handle_field_menu_input, handle_service_input).chain(),
            );
        }
        app.world_mut()
            .resource_mut::<ServiceUiState>()
            .open(ServiceRequest::Inn, None);
        app.update();

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();

        assert!(
            !app.world().resource::<FieldMenuState>().open,
            "the Escape that closed the inn also opened the field menu \
             (service first: {service_runs_first})"
        );

        // MinimalPlugins carries no input plugin, so the per-frame clear is manual.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.update();

        assert!(
            !app.world().resource::<ServiceUiState>().input_locked(),
            "the inn stayed open (service first: {service_runs_first})"
        );
        assert!(!app.world().resource::<FieldMenuState>().open);
    }
}

/// The reported bug: casting Teleport showed the Ember Atlas but the destination list could
/// not be steered.
///
/// The Confirm that picked Teleport out of the spell list was still `just_pressed` when
/// `handle_transport_input` ran later in the same `Update`, so the atlas opened and was
/// immediately answered — landing in `TransportPhase::Confirm` on whichever destination
/// happened to be first, a phase that ignores the arrow keys entirely. Both orderings are
/// exercised because nothing between the two plugins declares one.
#[test]
fn the_confirm_that_casts_teleport_does_not_also_answer_the_atlas_it_opens() {
    use crate::{
        action_input::ActionInputPlugin,
        transport_domain::TransportDomain,
        transport_ui::{TransportPhase, TransportUiState, handle_transport_input},
    };

    for field_menu_runs_first in [true, false] {
        let mut game = fixture_game();
        game.flags_mut().set("aric_teleport_unlocked");
        let catalog = FieldMenuCatalog::atlas_warp_fixture();
        let teleport =
            learned_field_abilities(game.party().member("aric").unwrap(), &game, &catalog)
                .iter()
                .position(|ability| {
                    matches!(
                        ability.kind,
                        AbilityKind::Utility(UtilityAbility::Warp { .. })
                    )
                })
                .expect("aric knows Teleport once the flag is set");
        let domain: TransportDomain = TransportDomain::try_from_catalog(
            crate::scenario_yaml::from_str(include_str!(scenario_file!("data/transport.yaml")))
                .unwrap(),
        )
        .unwrap();

        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .add_plugins(ActionInputPlugin)
            .insert_resource(catalog)
            .insert_resource(domain)
            .insert_resource(WorldInteractionState::default())
            .init_resource::<EngineSettings>()
            .insert_resource(WorldTransition::idle_for_test())
            .insert_resource(game)
            .insert_resource(SaveStore::new(
                std::env::temp_dir().join("rpg-s1-atlas-open-test-unused"),
            ))
            .insert_resource(SaveSlotCatalog::default())
            .insert_resource(Time::<Real>::default())
            .init_resource::<TransportUiState>()
            .insert_resource(FieldMenuState {
                open: true,
                screen: FieldMenuScreen::Spells,
                mode: FieldMenuMode::SpellList,
                selected: teleport,
                ..default()
            })
            .add_message::<AppExit>()
            .add_message::<PlaySfx>()
            .add_message::<KeyboardInput>();
        if field_menu_runs_first {
            app.add_systems(
                Update,
                (handle_field_menu_input, handle_transport_input).chain(),
            );
        } else {
            app.add_systems(
                Update,
                (handle_transport_input, handle_field_menu_input).chain(),
            );
        }

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        app.update();

        let state = app.world().resource::<TransportUiState>();
        assert_eq!(
            state.phase,
            TransportPhase::Browse,
            "the atlas opened already answered (field menu first: {field_menu_runs_first})"
        );
        assert_eq!(state.selected, 0);
    }
}

#[test]
fn confirmed_field_menu_quit_emits_app_exit_without_discarding_the_session() {
    let mut app = App::new();
    let mut keys = ButtonInput::<KeyCode>::default();
    keys.press(KeyCode::KeyY);
    app.insert_resource(keys)
        .insert_resource(ActionState::default())
        .insert_resource(FieldMenuCatalog::default())
        .insert_resource(WorldInteractionState::default())
        .init_resource::<EngineSettings>()
        .insert_resource(WorldTransition::default())
        .insert_resource(fixture_game())
        .insert_resource(SaveStore::new(
            std::env::temp_dir().join("rpg-s1-field-menu-quit-test-unused"),
        ))
        .insert_resource(SaveSlotCatalog::default())
        .insert_resource(Time::<Real>::default())
        .insert_resource(FieldMenuState {
            open: true,
            mode: FieldMenuMode::QuitConfirm,
            ..default()
        })
        .add_message::<AppExit>()
        .add_message::<PlaySfx>()
        .add_message::<KeyboardInput>()
        .add_systems(Update, handle_field_menu_input);

    let mut exit_cursor = app.world().resource::<Messages<AppExit>>().get_cursor();

    app.update();

    assert!(app.world().contains_resource::<GameState>());
    assert!(!app.world().resource::<FieldMenuState>().open);
    assert_eq!(
        exit_cursor
            .read(app.world().resource::<Messages<AppExit>>())
            .collect::<Vec<_>>(),
        [&AppExit::Success]
    );
}
