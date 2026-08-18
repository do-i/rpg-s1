//! In-world field-menu shell over the shared M6 runtime domain.

use bevy::{
    ecs::{hierarchy::ChildSpawnerCommands, schedule::ApplyDeferred},
    prelude::*,
};

use crate::{
    action_input::{ActionState, AppAction},
    app_state::AppState,
    field_menu_domain::{
        CatalogStatus, FieldMenuCatalog, InventoryTab, can_equip, cast_heal, derived_stats,
        discard_item, equip_item, inventory_ids, item_description, item_name,
        learned_field_abilities, preview_stats, unequip_item, use_field_item,
    },
    game_state::GameState,
    runtime_map::RuntimeMapId,
    runtime_member::EquipmentSlot,
    save_data::NativeSaveEnvelope,
    save_store::{
        FIRST_PLAYER_SLOT, LAST_PLAYER_SLOT, SaveSlot, SaveSlotState, SaveStore, unix_timestamp_now,
    },
    save_ui::SaveSlotCatalog,
    scenario_class::{Ability, AbilityKind, UtilityAbility},
    scenario_item::ItemDefinition,
    scenario_path::ScenarioRelativePath,
    scenario_root::ScenarioRoot,
    scenario_spatial::CardinalDirection,
    ui_theme::UiTheme,
    world_interaction::WorldInteractionState,
    world_transition::WorldTransition,
};

mod ui;

use ui::{
    cleanup_field_menu, large_status_portrait_path, load_status_image, profile_portrait_path,
    sync_custom_field_menu_content_visibility, sync_equipment_page, sync_field_menu_generic_text,
    sync_field_menu_overlay_lifecycle, sync_items_page, sync_main_menu_page, sync_save_page,
    sync_spells_page, sync_status_page,
};

const INVENTORY_PAGE_ROWS: usize = 10;
const EQUIPMENT_PICKER_VISIBLE_ROWS: usize = 4;
const SPELLBOOK_VISIBLE_ROWS: usize = 7;
const SAVE_VISIBLE_ROWS: usize = 6;
const SAVE_COMMAND_INDEX: usize = 4;
const QUIT_COMMAND_INDEX: usize = 5;

#[derive(Clone, Copy)]
struct MainCommand {
    label: &'static str,
    badge: &'static str,
    description: &'static str,
    screen: Option<FieldMenuScreen>,
}

const MAIN_COMMANDS: [MainCommand; 6] = [
    MainCommand {
        label: "Status",
        badge: "ST",
        description: "review health, rows, and growth",
        screen: Some(FieldMenuScreen::Status),
    },
    MainCommand {
        label: "Spells",
        badge: "SP",
        description: "cast field magic and utilities",
        screen: Some(FieldMenuScreen::Spells),
    },
    MainCommand {
        label: "Items",
        badge: "IT",
        description: "use, sort, and inspect supplies",
        screen: Some(FieldMenuScreen::Items),
    },
    MainCommand {
        label: "Equipment",
        badge: "EQ",
        description: "tune gear and compare stats",
        screen: Some(FieldMenuScreen::Equipment),
    },
    MainCommand {
        label: "Save",
        badge: "SV",
        description: "record the current journey",
        screen: Some(FieldMenuScreen::Save),
    },
    MainCommand {
        label: "Quit",
        badge: "QT",
        description: "exit the game to desktop",
        screen: None,
    },
];

const STATUS_PARTY_WIDTH: f32 = 316.0;
const STATUS_DETAIL_WIDTH: f32 = 404.0;
const STATUS_COLUMN_GAP: f32 = 18.0;
const STATUS_CATEGORIES: [&str; 2] = ["Spells", "Position"];
const ITEMS_POUCH_WIDTH: f32 = 286.0;
const ITEMS_DETAIL_WIDTH: f32 = 378.0;
const ITEMS_COLUMN_GAP: f32 = 18.0;
const EQUIPMENT_SLOT_WIDTH: f32 = 306.0;

pub(crate) struct FieldMenuPlugin;

impl Plugin for FieldMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FieldMenuState>()
            .add_systems(OnEnter(AppState::World), reset_field_menu)
            .add_systems(
                Update,
                (
                    handle_field_menu_input,
                    sync_field_menu_overlay_lifecycle,
                    ApplyDeferred,
                    sync_field_menu_generic_text,
                    sync_custom_field_menu_content_visibility,
                    sync_main_menu_page,
                    sync_status_page,
                    sync_items_page,
                    sync_equipment_page,
                    sync_spells_page,
                    sync_save_page,
                )
                    .chain()
                    .run_if(in_state(AppState::World)),
            )
            .add_systems(OnExit(AppState::World), cleanup_field_menu);
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum FieldMenuScreen {
    #[default]
    Main,
    Status,
    Items,
    Equipment,
    Spells,
    Save,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum FieldMenuMode {
    #[default]
    Browse,
    ItemActions,
    DiscardQuantity,
    ItemTarget,
    EquipmentPicker,
    SpellTarget,
    TeleportPicker,
    SaveConfirm,
    QuitConfirm,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum StatusPage {
    #[default]
    Roster,
    Details,
}

#[derive(Debug, Default, Resource)]
pub(crate) struct FieldMenuState {
    open: bool,
    screen: FieldMenuScreen,
    mode: FieldMenuMode,
    selected: usize,
    member_index: usize,
    status_page: StatusPage,
    tab_index: usize,
    quantity: u32,
    pending_id: Option<String>,
    message: String,
}

impl FieldMenuState {
    pub(crate) const fn input_locked(&self) -> bool {
        self.open
    }

    fn open(&mut self, screen: FieldMenuScreen) {
        *self = Self {
            open: true,
            screen,
            ..default()
        };
    }

    fn close(&mut self) {
        *self = Self::default();
    }

    fn back(&mut self) {
        if self.screen == FieldMenuScreen::Status && self.status_page == StatusPage::Details {
            self.status_page = StatusPage::Roster;
            self.selected = 0;
            self.message.clear();
            return;
        }
        match self.mode {
            FieldMenuMode::Browse => {
                if self.screen == FieldMenuScreen::Main {
                    self.close();
                } else {
                    self.screen = FieldMenuScreen::Main;
                    self.selected = 0;
                    self.message.clear();
                }
            }
            FieldMenuMode::ItemActions | FieldMenuMode::EquipmentPicker => {
                self.mode = FieldMenuMode::Browse;
                self.selected = 0;
                self.pending_id = None;
                self.message.clear();
            }
            FieldMenuMode::DiscardQuantity
            | FieldMenuMode::ItemTarget
            | FieldMenuMode::SpellTarget
            | FieldMenuMode::TeleportPicker => {
                self.mode = match self.screen {
                    FieldMenuScreen::Items => FieldMenuMode::ItemActions,
                    FieldMenuScreen::Spells => FieldMenuMode::Browse,
                    _ => FieldMenuMode::Browse,
                };
                self.selected = 0;
                self.quantity = 1;
                self.message.clear();
            }
            FieldMenuMode::SaveConfirm => {
                self.mode = FieldMenuMode::Browse;
                self.message.clear();
            }
            FieldMenuMode::QuitConfirm => {
                self.mode = FieldMenuMode::Browse;
                self.message.clear();
            }
        }
    }
}

#[derive(Component)]
struct FieldMenuRoot;

#[derive(Component)]
struct FieldMenuTitle;

#[derive(Component)]
struct FieldMenuBody;

#[derive(Component)]
struct FieldMenuHint;

#[derive(Component)]
struct FieldMenuGenericContent;

#[derive(Component)]
struct FieldMenuMainPage;

#[derive(Component)]
struct MainCommandRow;

#[derive(Component)]
struct SelectedMainCommandRow;

#[derive(Component)]
struct FieldMenuQuitModal;

#[derive(Component)]
struct FieldMenuStatusPage;

#[derive(Component)]
struct FieldMenuItemsPage;

#[derive(Component)]
struct ItemPouchRow;

#[derive(Component)]
struct ItemListRow;

#[derive(Component)]
struct SelectedItemListRow;

#[derive(Component)]
struct ItemActionModal;

#[derive(Component)]
struct FieldMenuEquipmentPage;

#[derive(Component)]
struct EquipmentSlotRow;

#[derive(Component)]
struct SelectedEquipmentSlotRow;

#[derive(Component)]
struct EquipmentPickerRow;

#[derive(Component)]
struct FieldMenuSpellsPage;

#[derive(Component)]
struct SpellbookRow;

#[derive(Component)]
struct SelectedSpellbookRow;

#[derive(Component)]
struct SpellTargetOverlay;

#[derive(Component)]
struct FieldMenuSavePage;

#[derive(Component)]
struct FieldSaveSlotRow;

#[derive(Component)]
struct SelectedFieldSaveSlotRow;

#[derive(Component)]
struct SaveOverwriteModal;

#[derive(Component)]
struct StatusMemberCard;

#[derive(Component)]
struct SelectedStatusMemberCard;

#[derive(Default)]
struct StatusPortraitAssets {
    profiles: std::collections::BTreeMap<String, Handle<Image>>,
    large: std::collections::BTreeMap<String, Handle<Image>>,
}

impl StatusPortraitAssets {
    fn load(asset_server: &AssetServer, root: &ScenarioRoot, game: &GameState) -> Self {
        let mut portraits = Self::default();
        for member in game.party().members() {
            if let Some(handle) =
                load_status_image(asset_server, root, &profile_portrait_path(member.id()))
            {
                portraits.profiles.insert(member.id().to_owned(), handle);
            }
            if let Some(handle) =
                load_status_image(asset_server, root, &large_status_portrait_path(member.id()))
            {
                portraits.large.insert(member.id().to_owned(), handle);
            }
        }
        portraits
    }

    fn profile(&self, member_id: &str) -> Option<Handle<Image>> {
        self.profiles.get(member_id).cloned()
    }

    fn large(&self, member_id: &str) -> Option<Handle<Image>> {
        self.large.get(member_id).cloned()
    }
}

fn reset_field_menu(mut state: ResMut<FieldMenuState>) {
    state.close();
}

#[expect(
    clippy::too_many_arguments,
    reason = "the field menu coordinates input, world locks, save storage, session state, and transitions"
)]
fn handle_field_menu_input(
    keys: Res<ButtonInput<KeyCode>>,
    actions: Res<ActionState>,
    catalog: Res<FieldMenuCatalog>,
    interaction: Res<WorldInteractionState>,
    mut transition: ResMut<WorldTransition>,
    game: Option<ResMut<GameState>>,
    store: Res<SaveStore>,
    mut saves: ResMut<SaveSlotCatalog>,
    time: Res<Time<Real>>,
    mut state: ResMut<FieldMenuState>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(mut game) = game else { return };

    if !state.open {
        if interaction.input_locked() || transition.input_locked() {
            return;
        }
        if keys.just_pressed(KeyCode::KeyM) {
            state.open(FieldMenuScreen::Main);
        } else if keys.just_pressed(KeyCode::KeyI) {
            state.open(FieldMenuScreen::Items);
        } else if keys.just_pressed(KeyCode::KeyS) {
            state.open(FieldMenuScreen::Status);
        }
        return;
    }

    if keys.just_pressed(KeyCode::KeyM) {
        state.close();
        return;
    }
    if actions.just_pressed(AppAction::Back) {
        state.back();
        return;
    }
    if !state.message.is_empty()
        && !matches!(
            state.mode,
            FieldMenuMode::SaveConfirm | FieldMenuMode::QuitConfirm
        )
    {
        state.message.clear();
    }

    let horizontal = if keys.just_pressed(KeyCode::ArrowLeft) {
        Some(-1)
    } else if keys.just_pressed(KeyCode::ArrowRight) {
        Some(1)
    } else {
        None
    };
    let vertical = actions.menu_navigation();

    match (state.screen, state.mode) {
        (FieldMenuScreen::Main, FieldMenuMode::Browse) => {
            if let Some(delta) = vertical {
                state.selected = wrapped(state.selected, MAIN_COMMANDS.len(), delta);
            }
            if actions.just_pressed(AppAction::Confirm) {
                if state.selected == SAVE_COMMAND_INDEX {
                    state.screen = FieldMenuScreen::Save;
                    state.selected = FIRST_PLAYER_SLOT;
                } else if state.selected == QUIT_COMMAND_INDEX {
                    state.mode = FieldMenuMode::QuitConfirm;
                    state.message = "Exit to desktop? Unsaved progress will be lost.".to_owned();
                } else if let Some(screen) = main_command_screen(state.selected) {
                    state.screen = screen;
                    state.selected = 0;
                }
            }
        }
        (FieldMenuScreen::Main, FieldMenuMode::QuitConfirm) => {
            if keys.just_pressed(KeyCode::KeyN) {
                state.mode = FieldMenuMode::Browse;
                state.message.clear();
            } else if keys.just_pressed(KeyCode::KeyY) || actions.just_pressed(AppAction::Confirm) {
                state.close();
                exit.write(AppExit::Success);
            }
        }
        (FieldMenuScreen::Save, FieldMenuMode::Browse) => {
            if let Some(delta) = vertical {
                state.selected = (state.selected as isize + delta)
                    .clamp(FIRST_PLAYER_SLOT as isize, LAST_PLAYER_SLOT as isize)
                    as usize;
            }
            if actions.just_pressed(AppAction::Confirm) {
                let Some(slot) = saves.slots().get(state.selected) else {
                    state.message = "Save slots are still loading.".to_owned();
                    return;
                };
                if slot.is_empty() {
                    save_game(&mut game, &store, &mut saves, &time, &mut state, false);
                } else {
                    state.mode = FieldMenuMode::SaveConfirm;
                    state.message = format!("Overwrite {}?", slot.label());
                }
            }
        }
        (FieldMenuScreen::Save, FieldMenuMode::SaveConfirm) => {
            if keys.just_pressed(KeyCode::KeyN) {
                state.mode = FieldMenuMode::Browse;
                state.message.clear();
            } else if keys.just_pressed(KeyCode::KeyY) || actions.just_pressed(AppAction::Confirm) {
                save_game(&mut game, &store, &mut saves, &time, &mut state, true);
            }
        }
        (FieldMenuScreen::Status, FieldMenuMode::Browse) => {
            if state.status_page == StatusPage::Roster {
                if let Some(delta) = horizontal.or(vertical) {
                    cycle_member(&mut state, game.party().len(), delta);
                }
                if actions.just_pressed(AppAction::Confirm) && !game.party().is_empty() {
                    state.status_page = StatusPage::Details;
                    state.selected = 0;
                }
            } else if let Some(delta) = vertical {
                state.selected = wrapped(state.selected, STATUS_CATEGORIES.len(), delta);
            }
        }
        (FieldMenuScreen::Items, FieldMenuMode::Browse) => {
            if let Some(delta) = horizontal {
                state.tab_index = wrapped(state.tab_index, InventoryTab::ALL.len(), delta);
                state.selected = 0;
            }
            let ids = inventory_ids(&game, &catalog, InventoryTab::ALL[state.tab_index]);
            if let Some(delta) = vertical {
                state.selected = wrapped_or_zero(state.selected, ids.len(), delta);
            }
            if actions.just_pressed(AppAction::Confirm) {
                if let Some(id) = ids.get(state.selected) {
                    state.pending_id = Some((*id).to_owned());
                    state.mode = FieldMenuMode::ItemActions;
                    state.selected = 0;
                } else {
                    state.message = "This tab is empty.".to_owned();
                }
            }
        }
        (FieldMenuScreen::Items, FieldMenuMode::ItemActions) => {
            if let Some(delta) = vertical {
                state.selected = wrapped(state.selected, 3, delta);
            }
            if actions.just_pressed(AppAction::Confirm) {
                let id = state
                    .pending_id
                    .clone()
                    .expect("item action requires an item");
                match state.selected {
                    0 if catalog.field_use(&id).is_some() => {
                        state.mode = FieldMenuMode::ItemTarget;
                        state.selected = 0;
                    }
                    0 => state.message = "That item cannot be used in the field.".to_owned(),
                    1 => {
                        if matches!(catalog.item(&id), Some(ItemDefinition::Key(_))) {
                            state.message = "Key items cannot be discarded.".to_owned();
                        } else if game.repository().is_locked(&id) {
                            state.message = "That item is locked.".to_owned();
                        } else {
                            state.mode = FieldMenuMode::DiscardQuantity;
                            state.selected = 0;
                            state.quantity = 1;
                        }
                    }
                    2 => {
                        game.repository_mut().set_hidden(id, true);
                        state.mode = FieldMenuMode::Browse;
                        state.selected = 0;
                        state.pending_id = None;
                        state.message = "Hidden for this session.".to_owned();
                    }
                    _ => unreachable!(),
                }
            }
        }
        (FieldMenuScreen::Items, FieldMenuMode::DiscardQuantity) => {
            let id = state
                .pending_id
                .as_deref()
                .expect("discard requires item")
                .to_owned();
            let max = game.repository().item_count(&id).max(1);
            if let Some(delta) = vertical {
                state.quantity = wrapped_quantity(state.quantity, max, delta);
            }
            if horizontal == Some(-1) {
                state.quantity = 1;
            }
            if horizontal == Some(1) {
                state.quantity = max;
            }
            if actions.just_pressed(AppAction::Confirm) {
                match discard_item(&mut game, &catalog, &id, state.quantity) {
                    Ok(()) => {
                        state.mode = FieldMenuMode::Browse;
                        state.selected = 0;
                        state.pending_id = None;
                        state.message = "Items discarded.".to_owned();
                    }
                    Err(error) => state.message = error.to_string(),
                }
            }
        }
        (FieldMenuScreen::Items, FieldMenuMode::ItemTarget) => {
            if let Some(delta) = vertical {
                state.selected = wrapped_or_zero(state.selected, game.party().len(), delta);
            }
            if actions.just_pressed(AppAction::Confirm) {
                let Some(target_id) = member_id_at(&game, state.selected).map(str::to_owned) else {
                    return;
                };
                let item_id = state
                    .pending_id
                    .clone()
                    .expect("item targeting requires item");
                match use_field_item(&mut game, &catalog, &item_id, &target_id) {
                    Ok(changed) => {
                        state.mode = FieldMenuMode::Browse;
                        state.selected = 0;
                        state.pending_id = None;
                        state.message = format!("Applied {changed} points/effects.");
                    }
                    Err(error) => state.message = error.to_string(),
                }
            }
        }
        (FieldMenuScreen::Equipment, FieldMenuMode::Browse) => {
            if let Some(delta) = horizontal {
                cycle_member(&mut state, game.party().len(), delta);
            }
            if let Some(delta) = vertical {
                state.selected = wrapped(state.selected, EquipmentSlot::ALL.len(), delta);
            }
            if actions.just_pressed(AppAction::Confirm) {
                state.quantity = state.selected as u32 + 1;
                state.mode = FieldMenuMode::EquipmentPicker;
                state.selected = 0;
            }
        }
        (FieldMenuScreen::Equipment, FieldMenuMode::EquipmentPicker) => {
            let Some(member_id) = member_id_at(&game, state.member_index).map(str::to_owned) else {
                return;
            };
            let slot = EquipmentSlot::ALL[state_slot_index(&state)];
            let candidates = equipment_candidates(&game, &catalog, slot);
            if let Some(delta) = vertical {
                state.selected = wrapped_or_zero(state.selected, candidates.len() + 1, delta);
            }
            if actions.just_pressed(AppAction::Confirm) {
                if state.selected == 0 {
                    match unequip_item(&mut game, &member_id, slot) {
                        Ok(Some(_)) => {
                            state.message = "Equipment returned to the repository.".to_owned()
                        }
                        Ok(None) => state.message = "That slot is already empty.".to_owned(),
                        Err(error) => state.message = error.to_string(),
                    }
                } else if let Some(id) = candidates.get(state.selected - 1) {
                    match equip_item(&mut game, &catalog, &member_id, id) {
                        Ok(_) => {
                            state.mode = FieldMenuMode::Browse;
                            state.selected = slot_index(slot);
                            state.message = "Equipment changed.".to_owned();
                        }
                        Err(error) => state.message = error.to_string(),
                    }
                }
            }
        }
        (FieldMenuScreen::Spells, FieldMenuMode::Browse) => {
            if let Some(delta) = horizontal {
                cycle_member(&mut state, game.party().len(), delta);
            }
            let abilities = member_at(&game, state.member_index).map_or_else(Vec::new, |member| {
                learned_field_abilities(member, &game, &catalog)
            });
            if let Some(delta) = vertical {
                state.selected = wrapped_or_zero(state.selected, abilities.len(), delta);
            }
            if actions.just_pressed(AppAction::Confirm) {
                let Some(ability) = abilities.get(state.selected).cloned().cloned() else {
                    state.message = "No learned field abilities.".to_owned();
                    return;
                };
                let Some(caster) = member_at(&game, state.member_index) else {
                    return;
                };
                if caster.mana() < ability.mp_cost {
                    state.message = "Not enough MP.".to_owned();
                    return;
                }
                match &ability.kind {
                    AbilityKind::Heal(_) => {
                        state.pending_id = Some(ability.id.clone());
                        state.mode = FieldMenuMode::SpellTarget;
                        state.selected = 0;
                    }
                    AbilityKind::Utility(UtilityAbility::Warp { .. }) => {
                        if catalog.eligible_warp_destinations(game.map()).is_empty() {
                            state.message = "Nowhere eligible to teleport to yet.".to_owned();
                        } else {
                            state.pending_id = Some(ability.id.clone());
                            state.mode = FieldMenuMode::TeleportPicker;
                            state.selected = 0;
                        }
                    }
                    _ => {
                        state.message =
                            "This field ability is display-only in the current slice.".to_owned()
                    }
                }
            }
        }
        (FieldMenuScreen::Spells, FieldMenuMode::SpellTarget) => {
            if let Some(delta) = vertical {
                state.selected = wrapped_or_zero(state.selected, game.party().len(), delta);
            }
            if actions.just_pressed(AppAction::Confirm) {
                let Some(caster_id) = member_id_at(&game, state.member_index).map(str::to_owned)
                else {
                    return;
                };
                let Some(target_id) = member_id_at(&game, state.selected).map(str::to_owned) else {
                    return;
                };
                let Some(ability) = ability_by_id(
                    &game,
                    &catalog,
                    &caster_id,
                    state
                        .pending_id
                        .as_deref()
                        .expect("spell target requires ability"),
                )
                .cloned() else {
                    return;
                };
                match cast_heal(&mut game, &ability, &caster_id, &target_id) {
                    Ok(amount) => {
                        state.mode = FieldMenuMode::Browse;
                        state.selected = 0;
                        state.pending_id = None;
                        state.message = format!("Restored {amount} HP.");
                    }
                    Err(error) => state.message = error.to_string(),
                }
            }
        }
        (FieldMenuScreen::Spells, FieldMenuMode::TeleportPicker) => {
            let destinations = catalog.eligible_warp_destinations(game.map());
            if let Some(delta) = vertical {
                state.selected = wrapped_or_zero(state.selected, destinations.len(), delta);
            }
            if actions.just_pressed(AppAction::Confirm) {
                let Some(destination) = destinations.get(state.selected).cloned() else {
                    return;
                };
                let Some(caster_id) = member_id_at(&game, state.member_index).map(str::to_owned)
                else {
                    return;
                };
                let ability_id = state
                    .pending_id
                    .as_deref()
                    .expect("teleport requires ability");
                let Some(ability) = ability_by_id(&game, &catalog, &caster_id, ability_id).cloned()
                else {
                    return;
                };
                if game
                    .party()
                    .member(&caster_id)
                    .is_none_or(|member| member.mana() < ability.mp_cost)
                {
                    state.message = "Not enough MP.".to_owned();
                    return;
                }
                let map_id = RuntimeMapId::try_new(destination.map_id.clone())
                    .expect("loaded warp id is valid");
                if transition.request_destination(
                    map_id,
                    destination.position,
                    CardinalDirection::Down,
                ) {
                    game.party_mut()
                        .member_mut(&caster_id)
                        .expect("validated")
                        .spend_mana(ability.mp_cost);
                    state.close();
                } else {
                    state.message = "A map transition is already active.".to_owned();
                }
            }
        }
        _ => {}
    }
}

fn state_slot_index(state: &FieldMenuState) -> usize {
    if state.mode == FieldMenuMode::EquipmentPicker {
        // Picker selection replaces the slot cursor, so retain the chosen slot in quantity.
        state.quantity.saturating_sub(1) as usize
    } else {
        state.selected.min(EquipmentSlot::ALL.len() - 1)
    }
}

fn save_game(
    game: &mut GameState,
    store: &SaveStore,
    saves: &mut SaveSlotCatalog,
    time: &Time<Real>,
    state: &mut FieldMenuState,
    overwrite: bool,
) {
    let Some(context) = saves.context() else {
        state.message = "Scenario data is still loading.".to_owned();
        return;
    };
    let scenario_id = context.scenario_id.to_owned();
    let scenario_version = context.scenario_version.to_owned();
    let balance = context.balance.clone();
    game.playtime_mut().commit_session(time.elapsed());
    let location = game
        .map()
        .current()
        .map_or("Unknown", RuntimeMapId::as_str)
        .to_owned();
    let timestamp = match unix_timestamp_now() {
        Ok(timestamp) => timestamp,
        Err(error) => {
            state.message = error.to_string();
            return;
        }
    };
    let result = NativeSaveEnvelope::from_game_state(
        game,
        scenario_id,
        scenario_version,
        timestamp,
        location,
    )
    .map_err(|error| error.to_string())
    .and_then(|envelope| {
        store
            .write(state.selected, &envelope, overwrite, &balance)
            .map_err(|error| error.to_string())
    });
    match result {
        Ok(_) => {
            state.mode = FieldMenuMode::Browse;
            state.message = format!("{} saved.", slot_label(state.selected));
            saves.request_refresh();
        }
        Err(error) => state.message = error,
    }
}

fn slot_label(index: usize) -> String {
    format!("Slot {index:02}")
}

fn slot_index(slot: EquipmentSlot) -> usize {
    EquipmentSlot::ALL
        .iter()
        .position(|candidate| *candidate == slot)
        .expect("known slot")
}

fn equipment_candidates(
    game: &GameState,
    catalog: &FieldMenuCatalog,
    slot: EquipmentSlot,
) -> Vec<String> {
    game.repository()
        .item_counts()
        .filter_map(|(id, _)| {
            let candidate_slot = match catalog.item(id)? {
                ItemDefinition::Weapon(_) => EquipmentSlot::Weapon,
                ItemDefinition::Shield(_) => EquipmentSlot::Shield,
                ItemDefinition::Helmet(_) => EquipmentSlot::Helmet,
                ItemDefinition::Body(_) => EquipmentSlot::Body,
                ItemDefinition::Accessory(_) => EquipmentSlot::Accessory,
                _ => return None,
            };
            (candidate_slot == slot).then(|| id.to_owned())
        })
        .collect()
}

fn ability_by_id<'a>(
    game: &GameState,
    catalog: &'a FieldMenuCatalog,
    member_id: &str,
    ability_id: &str,
) -> Option<&'a Ability> {
    let member = game.party().member(member_id)?;
    learned_field_abilities(member, game, catalog)
        .into_iter()
        .find(|ability| ability.id == ability_id)
}

fn member_at(game: &GameState, index: usize) -> Option<&crate::runtime_member::RuntimeMember> {
    game.party().members().nth(index)
}

fn member_id_at(game: &GameState, index: usize) -> Option<&str> {
    member_at(game, index).map(crate::runtime_member::RuntimeMember::id)
}

fn cycle_member(state: &mut FieldMenuState, count: usize, delta: isize) {
    state.member_index = wrapped_or_zero(state.member_index, count, delta);
    state.selected = 0;
}

fn wrapped_or_zero(current: usize, count: usize, delta: isize) -> usize {
    if count == 0 {
        0
    } else {
        wrapped(current.min(count - 1), count, delta)
    }
}

fn wrapped(current: usize, count: usize, delta: isize) -> usize {
    (current as isize + delta).rem_euclid(count as isize) as usize
}

fn wrapped_quantity(current: u32, max: u32, delta: isize) -> u32 {
    ((current.saturating_sub(1) as isize + delta).rem_euclid(max as isize) + 1) as u32
}

fn main_command_screen(index: usize) -> Option<FieldMenuScreen> {
    MAIN_COMMANDS.get(index).and_then(|command| command.screen)
}

fn inventory_page_range(len: usize, selected: usize) -> std::ops::Range<usize> {
    let start = ((selected / INVENTORY_PAGE_ROWS) * INVENTORY_PAGE_ROWS).min(len);
    start..(start + INVENTORY_PAGE_ROWS).min(len)
}

#[cfg(test)]
mod tests {
    use super::{ui::*, *};
    use crate::save_data::tests::fixture_game;

    fn spawn_fixture_main_page(mut commands: Commands, state: Res<FieldMenuState>) {
        commands.spawn(Node::default()).with_children(|parent| {
            spawn_main_menu_page(parent, &Handle::<Font>::default(), &state);
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
            spawn_field_save_slot_row(parent, &Handle::<Font>::default(), &slot, true, false);
        });
    }

    #[test]
    fn wrap_navigation_and_quantity_bounds_are_deterministic() {
        assert_eq!(wrapped(0, 1, -1), 0);
        assert_eq!(wrapped(4, 5, 1), 0);
        assert_eq!(wrapped(0, 5, -1), 4);
        assert_eq!(wrapped(0, 6, -1), 5);
        assert_eq!(wrapped(5, 6, 1), 0);
        assert_eq!(wrapped_or_zero(3, 0, 1), 0);
        assert_eq!(wrapped_quantity(1, 4, -1), 4);
        assert_eq!(wrapped_quantity(4, 4, 1), 1);
    }

    #[test]
    fn inventory_pages_cover_empty_short_and_scrolling_lists() {
        assert_eq!(inventory_page_range(0, 0), 0..0);
        assert_eq!(inventory_page_range(5, 4), 0..5);
        assert_eq!(
            inventory_page_range(25, INVENTORY_PAGE_ROWS - 1),
            0..INVENTORY_PAGE_ROWS
        );
        assert_eq!(
            inventory_page_range(25, INVENTORY_PAGE_ROWS),
            INVENTORY_PAGE_ROWS..INVENTORY_PAGE_ROWS * 2
        );
        let final_page_start = (24 / INVENTORY_PAGE_ROWS) * INVENTORY_PAGE_ROWS;
        assert_eq!(inventory_page_range(25, 24), final_page_start..25);
    }

    #[test]
    fn custom_menu_row_budgets_fit_the_baseline_canvas() {
        let items_rows = INVENTORY_PAGE_ROWS * 40 + (INVENTORY_PAGE_ROWS - 1) * 5 + 15;
        let equipment_rows =
            EQUIPMENT_PICKER_VISIBLE_ROWS * 52 + (EQUIPMENT_PICKER_VISIBLE_ROWS - 1) * 8 + 180;
        let spell_rows = SPELLBOOK_VISIBLE_ROWS * 58 + (SPELLBOOK_VISIBLE_ROWS - 1) * 7;
        let save_rows = SAVE_VISIBLE_ROWS * 54 + (SAVE_VISIBLE_ROWS - 1) * 6 + 206;

        assert!(items_rows <= 480);
        assert!(equipment_rows <= 480);
        assert!(spell_rows <= 480);
        assert!(save_rows <= 632);
    }

    #[test]
    fn status_visual_helpers_handle_empty_names_and_meter_bounds() {
        assert_eq!(member_emblem("Aric"), "AR");
        assert_eq!(member_emblem("Aric Vale"), "AV");
        assert_eq!(member_emblem("  "), "?");
        assert_eq!(meter_percent(3, 0), 0.0);
        assert_eq!(meter_percent(25, 100), 25.0);
        assert_eq!(meter_percent(125, 100), 100.0);
        assert_eq!(
            profile_portrait_path("aric"),
            "assets/images/aric_profile.png"
        );
        assert_eq!(
            large_status_portrait_path("aric"),
            "assets/images/party_portraits_large/aric_status_portrait.webp"
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
        assert_eq!(
            world
                .query::<&SelectedEquipmentSlotRow>()
                .iter(world)
                .count(),
            usize::from(party_len > 0)
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
        assert!(labels.contains(&"PAGE 01 / 17"));
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
    fn save_pages_keep_every_player_slot_reachable() {
        assert_eq!(save_page_start(1), 1);
        assert_eq!(save_page_start(6), 1);
        assert_eq!(save_page_start(7), 7);
        assert_eq!(save_page_start(100), 97);
    }

    #[test]
    fn status_back_returns_from_details_before_leaving_status() {
        let mut state = FieldMenuState::default();
        state.open(FieldMenuScreen::Status);
        state.status_page = StatusPage::Details;
        state.selected = 1;

        state.back();

        assert!(state.open);
        assert_eq!(state.screen, FieldMenuScreen::Status);
        assert_eq!(state.status_page, StatusPage::Roster);
        assert_eq!(state.selected, 0);

        state.back();
        assert_eq!(state.screen, FieldMenuScreen::Main);
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
    fn main_commands_distinguish_enabled_disabled_and_cancel_paths() {
        assert_eq!(main_command_screen(0), Some(FieldMenuScreen::Status));
        assert_eq!(main_command_screen(1), Some(FieldMenuScreen::Spells));
        assert_eq!(main_command_screen(2), Some(FieldMenuScreen::Items));
        assert_eq!(main_command_screen(3), Some(FieldMenuScreen::Equipment));
        assert_eq!(main_command_screen(4), Some(FieldMenuScreen::Save));
        assert!(main_command_screen(5).is_none());

        let mut state = FieldMenuState::default();
        state.open(FieldMenuScreen::Items);
        state.back();
        assert!(state.open);
        assert_eq!(state.screen, FieldMenuScreen::Main);
        state.back();
        assert!(!state.open);
    }

    #[test]
    fn main_page_matches_the_source_command_deck_structure() {
        assert_eq!(
            MAIN_COMMANDS.map(|command| command.label),
            ["Status", "Spells", "Items", "Equipment", "Save", "Quit"]
        );

        let mut app = App::new();
        app.insert_resource(FieldMenuState {
            open: true,
            selected: 1,
            ..default()
        })
        .add_systems(Update, spawn_fixture_main_page);

        app.update();

        let world = app.world_mut();
        assert_eq!(world.query::<&FieldMenuMainPage>().iter(world).count(), 1);
        assert_eq!(world.query::<&MainCommandRow>().iter(world).count(), 6);
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
    }

    #[test]
    fn quit_confirmation_is_a_focused_desktop_exit_modal() {
        let mut app = App::new();
        app.insert_resource(FieldMenuState {
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

    #[test]
    fn confirmed_field_menu_quit_emits_app_exit_without_discarding_the_session() {
        let mut app = App::new();
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::KeyY);
        app.insert_resource(keys)
            .insert_resource(ActionState::default())
            .insert_resource(FieldMenuCatalog::default())
            .insert_resource(WorldInteractionState::default())
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
}
