//! In-world field-menu shell over the shared M6 runtime domain.

use bevy::{ecs::hierarchy::ChildSpawnerCommands, prelude::*};

use crate::{
    action_input::{ActionState, AppAction},
    app_state::{AppState, AppStateTransitionRequest},
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
        FIRST_PLAYER_SLOT, LAST_PLAYER_SLOT, SaveSlotState, SaveStore, unix_timestamp_now,
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

const INVENTORY_PAGE_ROWS: usize = 12;
const MAIN_COMMANDS: [&str; 6] = ["Status", "Items", "Equipment", "Spells", "Save", "Quit"];

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
                    sync_field_menu_overlay,
                    sync_custom_field_menu_content_visibility,
                    sync_status_page,
                    sync_items_page,
                    sync_equipment_page,
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
    mut commands: Commands,
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
    mut transitions: MessageWriter<AppStateTransitionRequest>,
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
                if state.selected == 4 {
                    state.screen = FieldMenuScreen::Save;
                    state.selected = FIRST_PLAYER_SLOT;
                } else if state.selected == 5 {
                    state.mode = FieldMenuMode::QuitConfirm;
                    state.message =
                        "Return to the title screen? Unsaved progress will be lost.".to_owned();
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
                commands.remove_resource::<GameState>();
                transitions.write(AppStateTransitionRequest::new(AppState::Title));
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
    [
        FieldMenuScreen::Status,
        FieldMenuScreen::Items,
        FieldMenuScreen::Equipment,
        FieldMenuScreen::Spells,
        FieldMenuScreen::Save,
    ]
    .get(index)
    .copied()
}

fn inventory_page_range(len: usize, selected: usize) -> std::ops::Range<usize> {
    let start = ((selected / INVENTORY_PAGE_ROWS) * INVENTORY_PAGE_ROWS).min(len);
    start..(start + INVENTORY_PAGE_ROWS).min(len)
}

#[expect(
    clippy::too_many_arguments,
    clippy::type_complexity,
    reason = "the overlay updates three independently styled text roles"
)]
fn sync_field_menu_overlay(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    theme: Res<UiTheme>,
    state: Res<FieldMenuState>,
    catalog: Res<FieldMenuCatalog>,
    saves: Res<SaveSlotCatalog>,
    game: Option<Res<GameState>>,
    roots: Query<Entity, With<FieldMenuRoot>>,
    mut titles: Query<
        &mut Text,
        (
            With<FieldMenuTitle>,
            Without<FieldMenuBody>,
            Without<FieldMenuHint>,
        ),
    >,
    mut bodies: Query<
        &mut Text,
        (
            With<FieldMenuBody>,
            Without<FieldMenuTitle>,
            Without<FieldMenuHint>,
        ),
    >,
    mut hints: Query<
        &mut Text,
        (
            With<FieldMenuHint>,
            Without<FieldMenuTitle>,
            Without<FieldMenuBody>,
        ),
    >,
) {
    if !state.open {
        for entity in &roots {
            commands.entity(entity).despawn();
        }
        return;
    }
    if roots.is_empty() {
        spawn_field_menu_overlay(&mut commands, &asset_server, &root, &theme);
        return;
    }
    let Some(game) = game else { return };
    if let Ok(mut title) = titles.single_mut() {
        title.0 = screen_title(&state).to_owned();
    }
    if let Ok(mut body) = bodies.single_mut() {
        body.0 = render_body(&state, &game, &catalog, &saves);
    }
    if let Ok(mut hint) = hints.single_mut() {
        hint.0 = render_hint(&state);
    }
}

fn spawn_field_menu_overlay(
    commands: &mut Commands,
    asset_server: &AssetServer,
    root: &ScenarioRoot,
    theme: &UiTheme,
) {
    let font = asset_server.load(
        root.resolve(
            &ScenarioRelativePath::try_from("assets/fonts/Philosopher-Regular.ttf")
                .expect("field font path"),
        ),
    );
    let backdrop = asset_server.load(
        root.resolve(
            &ScenarioRelativePath::try_from(
                "assets/images/battle_bg/zone4-sanctum-bg-1280x468.webp",
            )
            .expect("field backdrop path"),
        ),
    );
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(30)),
                row_gap: px(14),
                ..default()
            },
            ImageNode::new(backdrop)
                .with_mode(NodeImageMode::Stretch)
                .with_color(Color::srgba(0.18, 0.18, 0.24, 0.36)),
            BackgroundColor(Color::srgba(0.02, 0.02, 0.08, 0.97)),
            GlobalZIndex(4_000),
            Pickable::IGNORE,
            Name::new("Field menu"),
            FieldMenuRoot,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new(""),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(34.0),
                    ..default()
                },
                TextColor(theme.name_entry_input_color),
                FieldMenuTitle,
                FieldMenuGenericContent,
            ));
            panel.spawn((
                Text::new(""),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(21.0),
                    ..default()
                },
                TextColor(Color::srgb_u8(235, 225, 190)),
                TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
                Node {
                    width: percent(100),
                    flex_grow: 1.0,
                    ..default()
                },
                FieldMenuBody,
                FieldMenuGenericContent,
            ));
            panel.spawn((
                Text::new(""),
                TextFont {
                    font: font.into(),
                    font_size: FontSize::Px(17.0),
                    ..default()
                },
                TextColor(theme.name_entry_hint_color),
                FieldMenuHint,
                FieldMenuGenericContent,
            ));
        });
}

fn sync_custom_field_menu_content_visibility(
    state: Res<FieldMenuState>,
    catalog: Res<FieldMenuCatalog>,
    game: Option<Res<GameState>>,
    mut generic_nodes: Query<&mut Node, With<FieldMenuGenericContent>>,
) {
    let show_custom_page = state.open
        && catalog.status() == CatalogStatus::Ready
        && game.is_some()
        && matches!(
            state.screen,
            FieldMenuScreen::Status | FieldMenuScreen::Items | FieldMenuScreen::Equipment
        );
    for mut node in &mut generic_nodes {
        node.display = if show_custom_page {
            Display::None
        } else {
            Display::Flex
        };
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "the status page coordinates the shared menu root, loaded scenario data, and UI assets"
)]
fn sync_status_page(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    state: Res<FieldMenuState>,
    catalog: Res<FieldMenuCatalog>,
    game: Option<Res<GameState>>,
    menu_roots: Query<Entity, With<FieldMenuRoot>>,
    pages: Query<Entity, With<FieldMenuStatusPage>>,
) {
    let show_status = state.open
        && state.screen == FieldMenuScreen::Status
        && catalog.status() == CatalogStatus::Ready
        && game.is_some();

    if !show_status {
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

    let font = asset_server.load(
        root.resolve(
            &ScenarioRelativePath::try_from("assets/fonts/Philosopher-Regular.ttf")
                .expect("status font path"),
        ),
    );
    let portraits = StatusPortraitAssets::load(&asset_server, &root, &game);
    commands.entity(menu_root).with_children(|parent| {
        spawn_status_page(parent, &font, &state, &game, &catalog, &portraits);
    });
}

#[expect(
    clippy::too_many_arguments,
    reason = "the items page coordinates the shared menu root and live inventory catalog"
)]
fn sync_items_page(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    state: Res<FieldMenuState>,
    catalog: Res<FieldMenuCatalog>,
    game: Option<Res<GameState>>,
    menu_roots: Query<Entity, With<FieldMenuRoot>>,
    pages: Query<Entity, With<FieldMenuItemsPage>>,
) {
    let show_items = state.open
        && state.screen == FieldMenuScreen::Items
        && catalog.status() == CatalogStatus::Ready
        && game.is_some();
    if !show_items {
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

    let font = asset_server.load(
        root.resolve(
            &ScenarioRelativePath::try_from("assets/fonts/Philosopher-Regular.ttf")
                .expect("items font path"),
        ),
    );
    commands.entity(menu_root).with_children(|parent| {
        spawn_items_page(parent, &font, &state, &game, &catalog);
    });
}

fn spawn_items_page(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
) {
    let tab = InventoryTab::ALL[state.tab_index];
    let ids = inventory_ids(game, catalog, tab);
    let list_index = selected_inventory_index(state, &ids);

    parent
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(14),
                ..default()
            },
            FieldMenuItemsPage,
            Name::new("Items page"),
        ))
        .with_children(|page| {
            spawn_items_header(page, font, tab, game.repository().gp());
            page.spawn(Node {
                width: percent(100),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Row,
                column_gap: px(ITEMS_COLUMN_GAP),
                ..default()
            })
            .with_children(|columns| {
                spawn_pouch_column(columns, font, state, game, catalog);
                spawn_item_list_column(columns, font, game, catalog, &ids, list_index);
                spawn_item_detail_column(columns, font, state, game, catalog, &ids, list_index);
            });

            let hint = if state.mode == FieldMenuMode::Browse {
                "←/→   CHANGE POUCH      ↑/↓   BROWSE      ENTER   ACTIONS      ESC   BACK"
            } else {
                "↑/↓   CHOOSE      ENTER   CONFIRM      ESC   CANCEL      I   CLOSE"
            };
            spawn_status_text(page, hint, font, 15.0, status_muted());
            if !state.message.is_empty() {
                spawn_items_message(page, font, &state.message);
            }
            spawn_item_modal(page, font, state, game, catalog);
        });
}

fn spawn_items_header(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    tab: InventoryTab,
    gp: u32,
) {
    parent
        .spawn(Node {
            width: percent(100),
            height: px(64),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|header| {
            header.spawn((
                Node {
                    width: px(7),
                    height: px(46),
                    margin: UiRect::right(px(3)),
                    ..default()
                },
                BackgroundColor(status_ember()),
            ));
            header.spawn((
                Node {
                    width: px(2),
                    height: px(46),
                    margin: UiRect::right(px(15)),
                    ..default()
                },
                BackgroundColor(status_gold()),
            ));
            header
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    ..default()
                })
                .with_children(|title| {
                    spawn_status_text(title, "ITEMS", font, 31.0, status_ink());
                    spawn_status_text(
                        title,
                        format!("{} POUCH", tab.label().to_uppercase()),
                        font,
                        14.0,
                        status_muted(),
                    );
                });
            spawn_status_text(header, format!("GP  {gp}"), font, 18.0, status_gold());
        });
}

fn spawn_pouch_column(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
) {
    spawn_status_panel(
        parent,
        Node {
            width: px(ITEMS_POUCH_WIDTH),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(7),
            padding: UiRect::all(px(14)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        "POUCH",
        font,
        |panel| {
            for (index, tab) in InventoryTab::ALL.into_iter().enumerate() {
                let selected = index == state.tab_index;
                let count = inventory_ids(game, catalog, tab).len();
                panel
                    .spawn((
                        Node {
                            width: percent(100),
                            height: px(48),
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            padding: UiRect::horizontal(px(10)),
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
                            Color::srgba_u8(126, 98, 55, 96)
                        }),
                        ItemPouchRow,
                    ))
                    .with_children(|row| {
                        row.spawn((
                            Node {
                                width: px(5),
                                height: px(28),
                                margin: UiRect::right(px(10)),
                                ..default()
                            },
                            BackgroundColor(if selected {
                                status_ember()
                            } else {
                                status_border()
                            }),
                        ));
                        row.spawn(Node {
                            flex_grow: 1.0,
                            ..default()
                        })
                        .with_children(|label| {
                            spawn_status_text(
                                label,
                                tab.label(),
                                font,
                                17.0,
                                if count == 0 {
                                    Color::srgb_u8(116, 108, 90)
                                } else {
                                    status_ink()
                                },
                            );
                        });
                        spawn_status_text(row, count.to_string(), font, 14.0, status_muted());
                    });
            }
        },
    );
}

fn spawn_item_list_column(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    game: &GameState,
    catalog: &FieldMenuCatalog,
    ids: &[&str],
    list_index: usize,
) {
    spawn_status_panel(
        parent,
        Node {
            height: percent(100),
            flex_basis: px(0),
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(5),
            padding: UiRect::all(px(14)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(6)),
            overflow: Overflow::clip(),
            ..default()
        },
        "ITEMS",
        font,
        |panel| {
            if ids.is_empty() {
                spawn_status_text(panel, "Pouch is empty.", font, 18.0, status_muted());
                spawn_status_text(
                    panel,
                    "Use ←/→ to inspect another pouch.",
                    font,
                    14.0,
                    Color::srgb_u8(116, 108, 90),
                );
                return;
            }
            let page = inventory_page_range(ids.len(), list_index);
            for (index, id) in ids.iter().enumerate().skip(page.start).take(page.len()) {
                let item = catalog.item(id).expect("filtered inventory item exists");
                let selected = index == list_index;
                let mut row = panel.spawn((
                    Node {
                        width: percent(100),
                        min_height: px(40),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        padding: UiRect::horizontal(px(9)),
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
                        Color::srgba_u8(126, 98, 55, 80)
                    }),
                    ItemListRow,
                ));
                if selected {
                    row.insert(SelectedItemListRow);
                }
                row.with_children(|row| {
                    row.spawn((
                        Node {
                            width: px(5),
                            height: px(24),
                            margin: UiRect::right(px(9)),
                            ..default()
                        },
                        BackgroundColor(item_accent(item)),
                    ));
                    row.spawn(Node {
                        flex_grow: 1.0,
                        ..default()
                    })
                    .with_children(|name| {
                        spawn_status_text(name, item_name(item), font, 16.0, status_ink());
                    });
                    if game.repository().is_new_item(id) {
                        spawn_status_text(row, "NEW", font, 11.0, status_ember());
                    }
                    spawn_status_text(
                        row,
                        format!("  x{}", game.repository().item_count(id)),
                        font,
                        14.0,
                        status_muted(),
                    );
                });
            }
            let page_number = list_index / INVENTORY_PAGE_ROWS + 1;
            let page_count = ids.len().div_ceil(INVENTORY_PAGE_ROWS);
            spawn_status_text(
                panel,
                format!("PAGE {page_number:02} / {page_count:02}"),
                font,
                12.0,
                Color::srgb_u8(116, 108, 90),
            );
        },
    );
}

fn spawn_item_detail_column(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
    ids: &[&str],
    list_index: usize,
) {
    spawn_status_panel(
        parent,
        Node {
            width: px(ITEMS_DETAIL_WIDTH),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(11),
            padding: UiRect::all(px(16)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        "DETAIL",
        font,
        |panel| {
            let Some(id) = selected_inventory_id(state, ids, list_index) else {
                spawn_status_text(
                    panel,
                    "Select a pouch containing items to view details.",
                    font,
                    16.0,
                    status_muted(),
                );
                return;
            };
            let Some(item) = catalog.item(id) else {
                return;
            };
            panel
                .spawn(Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(12),
                    ..default()
                })
                .with_children(|heading| {
                    heading
                        .spawn((
                            Node {
                                width: px(56),
                                height: px(56),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(px(2)),
                                border_radius: BorderRadius::all(px(6)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba_u8(10, 10, 14, 188)),
                            BorderColor::all(item_accent(item)),
                        ))
                        .with_children(|icon| {
                            spawn_status_text(
                                icon,
                                item_kind_abbreviation(item),
                                font,
                                15.0,
                                item_accent(item),
                            );
                        });
                    heading
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            flex_grow: 1.0,
                            ..default()
                        })
                        .with_children(|title| {
                            spawn_status_text(title, item_name(item), font, 23.0, status_gold());
                            spawn_status_text(
                                title,
                                format!(
                                    "{}    QUANTITY  x{}",
                                    item_kind_label(item),
                                    game.repository().item_count(id)
                                ),
                                font,
                                13.0,
                                status_muted(),
                            );
                        });
                });
            spawn_section_rule(panel);
            spawn_status_text(panel, "DESCRIPTION", font, 13.0, status_gold());
            spawn_status_text(panel, item_description(item), font, 16.0, status_ink());
            spawn_section_rule(panel);
            spawn_item_chips(panel, font, game, catalog, id, item);
            spawn_status_text(panel, "ENTER  →  ACTIONS", font, 14.0, status_muted());
        },
    );
}

fn spawn_item_chips(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    game: &GameState,
    catalog: &FieldMenuCatalog,
    id: &str,
    item: &ItemDefinition,
) {
    parent
        .spawn(Node {
            width: percent(100),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(7),
            row_gap: px(7),
            ..default()
        })
        .with_children(|chips| {
            spawn_item_chip(chips, font, item_kind_label(item), item_accent(item));
            if catalog.field_use(id).is_some() {
                spawn_item_chip(chips, font, "FIELD USE", status_teal());
            }
            if game.repository().is_new_item(id) {
                spawn_item_chip(chips, font, "NEW", status_ember());
            }
            if game.repository().is_locked(id) {
                spawn_item_chip(chips, font, "LOCKED", status_violet());
            }
        });
}

fn spawn_item_chip(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    label: &str,
    color: Color,
) {
    parent
        .spawn((
            Node {
                padding: UiRect::axes(px(9), px(4)),
                border_radius: BorderRadius::all(px(11)),
                ..default()
            },
            BackgroundColor(color),
        ))
        .with_children(|chip| {
            spawn_status_text(chip, label, font, 11.0, Color::srgb_u8(20, 17, 12));
        });
}

fn spawn_items_message(parent: &mut ChildSpawnerCommands<'_>, font: &Handle<Font>, message: &str) {
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(360),
                right: px(360),
                bottom: px(40),
                justify_content: JustifyContent::Center,
                padding: UiRect::axes(px(14), px(8)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(5)),
                ..default()
            },
            BackgroundColor(Color::srgba_u8(22, 22, 28, 244)),
            BorderColor::all(status_border_active()),
        ))
        .with_children(|banner| {
            spawn_status_text(banner, message, font, 14.0, status_ink());
        });
}

fn spawn_item_modal(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
) {
    if state.mode == FieldMenuMode::Browse {
        return;
    }
    let title = state
        .pending_id
        .as_deref()
        .and_then(|id| catalog.item(id))
        .map_or("ITEM ACTION", item_name);
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(0),
                bottom: px(0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba_u8(2, 2, 6, 150)),
        ))
        .with_children(|overlay| {
            spawn_status_panel(
                overlay,
                Node {
                    width: px(390),
                    min_height: px(190),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(9),
                    padding: UiRect::all(px(16)),
                    border: UiRect::all(px(2)),
                    border_radius: BorderRadius::all(px(7)),
                    ..default()
                },
                &title.to_uppercase(),
                font,
                |modal| {
                    modal.spawn(ItemActionModal);
                    match state.mode {
                        FieldMenuMode::ItemActions => {
                            for (index, (label, subtitle)) in [
                                ("Use", "apply this item"),
                                ("Discard", "remove from pouch"),
                                ("Hide", "hide for this session"),
                            ]
                            .into_iter()
                            .enumerate()
                            {
                                spawn_item_modal_row(
                                    modal,
                                    font,
                                    label,
                                    subtitle,
                                    index == state.selected,
                                );
                            }
                        }
                        FieldMenuMode::DiscardQuantity => {
                            spawn_status_text(
                                modal,
                                "DISCARD QUANTITY",
                                font,
                                13.0,
                                status_muted(),
                            );
                            spawn_status_text(
                                modal,
                                format!("{:02}", state.quantity),
                                font,
                                34.0,
                                status_gold(),
                            );
                            spawn_status_text(
                                modal,
                                "← one    → whole stack    ↑/↓ adjust",
                                font,
                                14.0,
                                status_muted(),
                            );
                        }
                        FieldMenuMode::ItemTarget => {
                            spawn_status_text(modal, "CHOOSE A TARGET", font, 13.0, status_muted());
                            for (index, member) in game.party().members().enumerate() {
                                spawn_item_modal_row(
                                    modal,
                                    font,
                                    member.name(),
                                    &format!(
                                        "HP {}/{}    MP {}/{}",
                                        member.health(),
                                        member.max_health(),
                                        member.mana(),
                                        member.max_mana()
                                    ),
                                    index == state.selected,
                                );
                            }
                        }
                        _ => {}
                    }
                },
            );
        });
}

fn spawn_item_modal_row(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    label: &str,
    subtitle: &str,
    selected: bool,
) {
    parent
        .spawn((
            Node {
                width: percent(100),
                min_height: px(48),
                flex_direction: FlexDirection::Column,
                padding: UiRect::axes(px(11), px(5)),
                border: UiRect::all(px(if selected { 2 } else { 1 })),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(if selected {
                Color::srgba_u8(72, 49, 25, 230)
            } else {
                Color::srgba_u8(10, 10, 14, 170)
            }),
            BorderColor::all(if selected {
                status_border_active()
            } else {
                status_border()
            }),
        ))
        .with_children(|row| {
            spawn_status_text(row, label, font, 17.0, status_ink());
            spawn_status_text(row, subtitle, font, 12.0, status_muted());
        });
}

#[expect(
    clippy::too_many_arguments,
    reason = "the equipment page coordinates the shared menu root, portraits, and live inventory catalog"
)]
fn sync_equipment_page(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    state: Res<FieldMenuState>,
    catalog: Res<FieldMenuCatalog>,
    game: Option<Res<GameState>>,
    menu_roots: Query<Entity, With<FieldMenuRoot>>,
    pages: Query<Entity, With<FieldMenuEquipmentPage>>,
) {
    let show_equipment = state.open
        && state.screen == FieldMenuScreen::Equipment
        && catalog.status() == CatalogStatus::Ready
        && game.is_some();
    if !show_equipment {
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

    let font = asset_server.load(
        root.resolve(
            &ScenarioRelativePath::try_from("assets/fonts/Philosopher-Regular.ttf")
                .expect("equipment font path"),
        ),
    );
    let portraits = StatusPortraitAssets::load(&asset_server, &root, &game);
    commands.entity(menu_root).with_children(|parent| {
        spawn_equipment_page(parent, &font, &state, &game, &catalog, &portraits);
    });
}

fn spawn_equipment_page(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
    portraits: &StatusPortraitAssets,
) {
    parent
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(14),
                ..default()
            },
            FieldMenuEquipmentPage,
            Name::new("Equipment page"),
        ))
        .with_children(|page| {
            spawn_equipment_header(page, font, state, game);
            page.spawn(Node {
                width: percent(100),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Row,
                column_gap: px(STATUS_COLUMN_GAP),
                ..default()
            })
            .with_children(|columns| {
                spawn_party_column(columns, font, state, game, catalog, portraits);
                spawn_equipment_slots_column(columns, font, state, game, catalog);
                spawn_equipment_inventory_column(columns, font, state, game, catalog);
            });
            spawn_status_text(
                page,
                if state.mode == FieldMenuMode::EquipmentPicker {
                    "↑/↓   PREVIEW ITEM      ENTER   EQUIP      ESC   SLOTS      M   CLOSE"
                } else {
                    "←/→   CHANGE MEMBER      ↑/↓   SELECT SLOT      ENTER   INVENTORY      ESC   BACK"
                },
                font,
                15.0,
                status_muted(),
            );
            if !state.message.is_empty() {
                spawn_items_message(page, font, &state.message);
            }
        });
}

fn spawn_equipment_header(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
) {
    let member_name =
        member_at(game, state.member_index).map_or("NO MEMBER", |member| member.name());
    parent
        .spawn(Node {
            width: percent(100),
            height: px(64),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|header| {
            header.spawn((
                Node {
                    width: px(7),
                    height: px(46),
                    margin: UiRect::right(px(3)),
                    ..default()
                },
                BackgroundColor(status_ember()),
            ));
            header.spawn((
                Node {
                    width: px(2),
                    height: px(46),
                    margin: UiRect::right(px(15)),
                    ..default()
                },
                BackgroundColor(status_gold()),
            ));
            header
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    ..default()
                })
                .with_children(|title| {
                    spawn_status_text(title, "EQUIPMENT", font, 31.0, status_ink());
                    spawn_status_text(title, "GEAR, COMPARE, COMMIT", font, 14.0, status_muted());
                });
            spawn_status_text(
                header,
                member_name.to_uppercase(),
                font,
                16.0,
                status_gold(),
            );
        });
}

fn spawn_equipment_slots_column(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
) {
    let Some(member) = member_at(game, state.member_index) else {
        return;
    };
    let selected_slot = state_slot_index(state);
    spawn_status_panel(
        parent,
        Node {
            width: px(EQUIPMENT_SLOT_WIDTH),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            padding: UiRect::all(px(14)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        "SLOTS",
        font,
        |panel| {
            for (index, slot) in EquipmentSlot::ALL.into_iter().enumerate() {
                let selected = index == selected_slot;
                let equipped = member.equipment().get(slot).and_then(|id| catalog.item(id));
                let mut row = panel.spawn((
                    Node {
                        width: percent(100),
                        min_height: px(58),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
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
                    EquipmentSlotRow,
                ));
                if selected {
                    row.insert(SelectedEquipmentSlotRow);
                }
                row.with_children(|row| {
                    spawn_status_text(
                        row,
                        slot.as_str().to_uppercase(),
                        font,
                        12.0,
                        status_muted(),
                    );
                    spawn_status_text(
                        row,
                        equipped.map_or("—", item_name),
                        font,
                        16.0,
                        if equipped.is_some() {
                            status_ink()
                        } else {
                            Color::srgb_u8(116, 108, 90)
                        },
                    );
                });
            }
            spawn_section_rule(panel);
            spawn_status_text(panel, "TOTALS", font, 13.0, status_gold());
            spawn_equipment_stat_grid(panel, font, derived_stats(member, catalog));
        },
    );
}

fn spawn_equipment_stat_grid(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    stats: crate::field_menu_domain::DerivedStats,
) {
    parent
        .spawn(Node {
            width: percent(100),
            flex_direction: FlexDirection::Row,
            column_gap: px(8),
            ..default()
        })
        .with_children(|grid| {
            for (label, value) in [
                ("STR", stats.strength),
                ("DEX", stats.dexterity),
                ("CON", stats.constitution),
                ("INT", stats.intelligence),
            ] {
                grid.spawn((
                    Node {
                        flex_basis: px(0),
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::axes(px(3), px(6)),
                        border: UiRect::all(px(1)),
                        border_radius: BorderRadius::all(px(4)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba_u8(10, 10, 14, 150)),
                    BorderColor::all(status_border()),
                ))
                .with_children(|stat| {
                    spawn_status_text(stat, label, font, 11.0, status_muted());
                    spawn_status_text(stat, value.to_string(), font, 18.0, status_gold());
                });
            }
        });
}

fn spawn_equipment_inventory_column(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
) {
    let Some(member) = member_at(game, state.member_index) else {
        return;
    };
    let slot = EquipmentSlot::ALL[state_slot_index(state)];
    spawn_status_panel(
        parent,
        Node {
            height: percent(100),
            flex_basis: px(0),
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            padding: UiRect::all(px(16)),
            border: UiRect::all(px(if state.mode == FieldMenuMode::EquipmentPicker {
                2
            } else {
                1
            })),
            border_radius: BorderRadius::all(px(6)),
            overflow: Overflow::clip(),
            ..default()
        },
        "INVENTORY",
        font,
        |panel| {
            spawn_status_text(
                panel,
                format!("{}  /  {}", member.name(), slot.as_str().to_uppercase()),
                font,
                13.0,
                status_muted(),
            );
            if state.mode == FieldMenuMode::EquipmentPicker {
                spawn_equipment_picker(panel, font, state, game, catalog, member, slot);
            } else {
                spawn_current_equipment_detail(panel, font, member, slot, catalog);
            }
        },
    );
}

fn spawn_current_equipment_detail(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    member: &crate::runtime_member::RuntimeMember,
    slot: EquipmentSlot,
    catalog: &FieldMenuCatalog,
) {
    let equipped = member.equipment().get(slot).and_then(|id| catalog.item(id));
    spawn_section_rule(parent);
    spawn_status_text(
        parent,
        equipped.map_or("EMPTY SLOT", item_name),
        font,
        24.0,
        if equipped.is_some() {
            status_gold()
        } else {
            status_muted()
        },
    );
    spawn_status_text(
        parent,
        equipped.map_or(
            "No item is currently equipped in this slot.",
            item_description,
        ),
        font,
        16.0,
        status_ink(),
    );
    spawn_section_rule(parent);
    spawn_status_text(
        parent,
        "Press ENTER to compare compatible inventory items.",
        font,
        14.0,
        status_muted(),
    );
}

fn spawn_equipment_picker(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
    member: &crate::runtime_member::RuntimeMember,
    slot: EquipmentSlot,
) {
    let candidates = equipment_candidates(game, catalog, slot);
    let total = candidates.len() + 1;
    let visible_rows = 6;
    let first = state
        .selected
        .saturating_sub(visible_rows - 1)
        .min(total.saturating_sub(visible_rows));
    for index in first..(first + visible_rows).min(total) {
        let candidate_id = index
            .checked_sub(1)
            .and_then(|candidate| candidates.get(candidate).map(String::as_str));
        let item = candidate_id.and_then(|id| catalog.item(id));
        let selected = index == state.selected;
        parent
            .spawn((
                Node {
                    width: percent(100),
                    min_height: px(52),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    padding: UiRect::axes(px(11), px(5)),
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
                EquipmentPickerRow,
            ))
            .with_children(|row| {
                spawn_status_text(
                    row,
                    item.map_or("(Unequip)", item_name),
                    font,
                    16.0,
                    if item.is_some() {
                        status_ink()
                    } else {
                        status_muted()
                    },
                );
                spawn_status_text(
                    row,
                    candidate_id.map_or("return current item to pouch".to_owned(), |id| {
                        equipment_preview_summary(member, catalog, slot, Some(id))
                    }),
                    font,
                    12.0,
                    status_muted(),
                );
            });
    }
    spawn_section_rule(parent);
    spawn_equipment_preview(parent, font, state, catalog, member, slot, &candidates);
}

fn spawn_equipment_preview(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    catalog: &FieldMenuCatalog,
    member: &crate::runtime_member::RuntimeMember,
    slot: EquipmentSlot,
    candidates: &[String],
) {
    let candidate_id = state
        .selected
        .checked_sub(1)
        .and_then(|index| candidates.get(index).map(String::as_str));
    let before = derived_stats(member, catalog);
    let after = preview_stats(member, catalog, slot, candidate_id);
    spawn_status_text(parent, "PREVIEW", font, 13.0, status_gold());
    parent
        .spawn(Node {
            width: percent(100),
            flex_direction: FlexDirection::Row,
            column_gap: px(8),
            ..default()
        })
        .with_children(|preview| {
            for (label, old, new) in [
                ("STR", before.strength, after.strength),
                ("DEX", before.dexterity, after.dexterity),
                ("CON", before.constitution, after.constitution),
                ("INT", before.intelligence, after.intelligence),
            ] {
                let color = stat_change_color(old, new);
                preview
                    .spawn((
                        Node {
                            flex_basis: px(0),
                            flex_grow: 1.0,
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            padding: UiRect::axes(px(3), px(6)),
                            border: UiRect::all(px(1)),
                            border_radius: BorderRadius::all(px(4)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba_u8(10, 10, 14, 150)),
                        BorderColor::all(color),
                    ))
                    .with_children(|stat| {
                        spawn_status_text(stat, label, font, 11.0, status_muted());
                        spawn_status_text(stat, format!("{old}→{new}"), font, 14.0, color);
                    });
            }
        });
    if let Some(item) = candidate_id.and_then(|id| catalog.item(id)) {
        spawn_status_text(parent, item_description(item), font, 13.0, status_muted());
    }
}

fn equipment_preview_summary(
    member: &crate::runtime_member::RuntimeMember,
    catalog: &FieldMenuCatalog,
    slot: EquipmentSlot,
    candidate_id: Option<&str>,
) -> String {
    let before = derived_stats(member, catalog);
    let after = preview_stats(member, catalog, slot, candidate_id);
    [
        ("STR", before.strength, after.strength),
        ("DEX", before.dexterity, after.dexterity),
        ("CON", before.constitution, after.constitution),
        ("INT", before.intelligence, after.intelligence),
    ]
    .into_iter()
    .filter(|(_, old, new)| old != new)
    .map(|(label, old, new)| format!("{label} {old}→{new}"))
    .collect::<Vec<_>>()
    .join("    ")
}

fn stat_change_color(old: i32, new: i32) -> Color {
    if new > old {
        Color::srgb_u8(120, 220, 120)
    } else if new < old {
        Color::srgb_u8(220, 110, 110)
    } else {
        status_muted()
    }
}

fn spawn_status_page(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
    portraits: &StatusPortraitAssets,
) {
    parent
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(14),
                ..default()
            },
            FieldMenuStatusPage,
            Name::new("Status page"),
        ))
        .with_children(|page| {
            spawn_status_header(page, font, state.member_index, game.party().len());

            page.spawn(Node {
                width: percent(100),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Row,
                column_gap: px(STATUS_COLUMN_GAP),
                ..default()
            })
            .with_children(|columns| {
                spawn_party_column(columns, font, state, game, catalog, portraits);
                spawn_member_column(columns, font, state, game, catalog, portraits);
                spawn_profile_column(columns, font, state, game, catalog);
            });

            spawn_status_text(
                page,
                if state.status_page == StatusPage::Roster {
                    "↑/↓   SELECT MEMBER      ENTER   STATS      ESC   BACK"
                } else {
                    "↑/↓   SELECT ACTION      ESC   PORTRAIT      M   CLOSE"
                },
                font,
                15.0,
                status_muted(),
            );
        });
}

fn spawn_status_header(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    member_index: usize,
    party_len: usize,
) {
    parent
        .spawn(Node {
            width: percent(100),
            height: px(64),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|header| {
            header.spawn((
                Node {
                    width: px(7),
                    height: px(46),
                    margin: UiRect::right(px(3)),
                    ..default()
                },
                BackgroundColor(status_ember()),
            ));
            header.spawn((
                Node {
                    width: px(2),
                    height: px(46),
                    margin: UiRect::right(px(15)),
                    ..default()
                },
                BackgroundColor(status_gold()),
            ));
            header
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    ..default()
                })
                .with_children(|title| {
                    spawn_status_text(title, "STATUS", font, 31.0, status_ink());
                    spawn_status_text(title, "PARTY ROSTER AND GROWTH", font, 14.0, status_muted());
                });
            spawn_status_text(
                header,
                format!(
                    "{:02}  /  {:02}",
                    member_index.saturating_add(1).min(party_len.max(1)),
                    party_len
                ),
                font,
                16.0,
                status_gold(),
            );
        });
}

fn spawn_party_column(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
    portraits: &StatusPortraitAssets,
) {
    spawn_status_panel(
        parent,
        Node {
            width: px(STATUS_PARTY_WIDTH),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(10),
            padding: UiRect::all(px(14)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        "PARTY",
        font,
        |panel| {
            for (index, member) in game.party().members().enumerate() {
                let selected = index == state.member_index;
                let mut card = panel.spawn((
                    Node {
                        width: percent(100),
                        min_height: px(84),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: px(12),
                        padding: UiRect::all(px(10)),
                        border: UiRect::all(px(if selected { 2 } else { 1 })),
                        border_radius: BorderRadius::all(px(5)),
                        ..default()
                    },
                    BackgroundColor(if selected {
                        Color::srgba_u8(79, 51, 38, 214)
                    } else {
                        Color::srgba_u8(30, 30, 38, 164)
                    }),
                    BorderColor::all(if selected {
                        status_border_active()
                    } else {
                        status_border()
                    }),
                    StatusMemberCard,
                    Name::new(format!("Party card: {}", member.name())),
                ));
                if selected {
                    card.insert(SelectedStatusMemberCard);
                }
                card.with_children(|card| {
                    spawn_portrait_frame(
                        card,
                        font,
                        member.name(),
                        portraits.profile(member.id()),
                        56.0,
                        56.0,
                        selected,
                        "Party portrait",
                    );
                    card.spawn(Node {
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::Column,
                        row_gap: px(2),
                        ..default()
                    })
                    .with_children(|summary| {
                        spawn_status_text(
                            summary,
                            format!("{}     LV {}", member.name(), member.level()),
                            font,
                            18.0,
                            status_ink(),
                        );
                        spawn_status_text(
                            summary,
                            class_name(member.class_id(), catalog).to_uppercase(),
                            font,
                            13.0,
                            status_gold(),
                        );
                        spawn_status_text(
                            summary,
                            format!(
                                "HP {:>3}/{:<3}     MP {:>3}/{:<3}",
                                member.health(),
                                member.max_health(),
                                member.mana(),
                                member.max_mana()
                            ),
                            font,
                            13.0,
                            status_muted(),
                        );
                    });
                });
            }
        },
    );
}

fn spawn_member_column(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
    portraits: &StatusPortraitAssets,
) {
    if state.status_page == StatusPage::Roster {
        spawn_full_portrait_column(parent, font, state, game, portraits);
    } else {
        spawn_member_details_column(parent, font, state, game, catalog);
    }
}

fn spawn_full_portrait_column(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    portraits: &StatusPortraitAssets,
) {
    let Some(member) = member_at(game, state.member_index) else {
        return;
    };
    parent
        .spawn((
            Node {
                width: px(STATUS_DETAIL_WIDTH),
                height: percent(100),
                flex_shrink: 0.0,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(px(5)),
                border: UiRect::all(px(2)),
                border_radius: BorderRadius::all(px(6)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgba_u8(8, 8, 12, 210)),
            BorderColor::all(status_border_active()),
            Name::new("Full status portrait"),
        ))
        .with_children(|portrait_panel| {
            if let Some(portrait) = portraits.large(member.id()) {
                portrait_panel.spawn((
                    ImageNode::new(portrait).with_mode(NodeImageMode::Stretch),
                    Node {
                        height: percent(100),
                        aspect_ratio: Some(418.0 / 570.0),
                        max_width: percent(100),
                        ..default()
                    },
                ));
            } else {
                spawn_status_text(
                    portrait_panel,
                    member_emblem(member.name()),
                    font,
                    42.0,
                    status_gold(),
                );
            }
        });
}

fn spawn_member_details_column(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
) {
    let Some(member) = member_at(game, state.member_index) else {
        return;
    };
    let stats = derived_stats(member, catalog);
    let equipment = EquipmentSlot::ALL
        .into_iter()
        .map(|slot| {
            let name = member
                .equipment()
                .get(slot)
                .and_then(|id| catalog.item(id))
                .map_or("—", item_name);
            format!("{:<5}  {name}", status_slot_label(slot))
        })
        .collect::<Vec<_>>()
        .join("\n");
    spawn_status_panel(
        parent,
        Node {
            width: px(STATUS_DETAIL_WIDTH),
            height: percent(100),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            padding: UiRect::all(px(16)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        member.name(),
        font,
        |panel| {
            panel
                .spawn(Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                })
                .with_children(|level| {
                    spawn_status_text(
                        level,
                        format!("LV {}", member.level()),
                        font,
                        16.0,
                        status_ink(),
                    );
                    spawn_status_text(
                        level,
                        format!("EXP {} / {}", member.experience(), member.experience_next()),
                        font,
                        14.0,
                        status_muted(),
                    );
                });

            spawn_meter(
                panel,
                "EXP",
                member.experience(),
                member.experience_next(),
                font,
                status_violet(),
            );
            spawn_meter(
                panel,
                "HP",
                member.health(),
                member.max_health(),
                font,
                if member.health().saturating_mul(4) < member.max_health() {
                    status_ember()
                } else {
                    Color::srgb_u8(132, 196, 111)
                },
            );
            spawn_meter(
                panel,
                "MP",
                member.mana(),
                member.max_mana(),
                font,
                status_teal(),
            );

            panel
                .spawn(Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    row_gap: px(4),
                    ..default()
                })
                .with_children(|grid| {
                    for (label, value) in [
                        ("STR", stats.strength),
                        ("DEX", stats.dexterity),
                        ("CON", stats.constitution),
                        ("INT", stats.intelligence),
                    ] {
                        grid.spawn(Node {
                            width: percent(50),
                            flex_direction: FlexDirection::Row,
                            column_gap: px(14),
                            ..default()
                        })
                        .with_children(|attribute| {
                            spawn_status_text(attribute, label, font, 14.0, status_muted());
                            spawn_status_text(
                                attribute,
                                format!("{value:03}"),
                                font,
                                16.0,
                                status_ink(),
                            );
                        });
                    }
                });

            spawn_status_text(panel, equipment, font, 14.0, status_ink());

            panel.spawn(Node {
                flex_grow: 1.0,
                ..default()
            });
            spawn_section_rule(panel);
            for (index, label) in STATUS_CATEGORIES.into_iter().enumerate() {
                let selected = index == state.selected;
                panel
                    .spawn((
                        Node {
                            width: percent(100),
                            height: px(52),
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: px(12),
                            padding: UiRect::axes(px(10), px(7)),
                            border: UiRect::all(px(if selected { 2 } else { 1 })),
                            border_radius: BorderRadius::all(px(5)),
                            ..default()
                        },
                        BackgroundColor(if selected {
                            Color::srgba_u8(79, 51, 38, 214)
                        } else {
                            Color::srgba_u8(30, 30, 38, 164)
                        }),
                        BorderColor::all(if selected {
                            status_border_active()
                        } else {
                            status_border()
                        }),
                    ))
                    .with_children(|category| {
                        category
                            .spawn((
                                Node {
                                    width: px(34),
                                    height: px(34),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(px(1)),
                                    border_radius: BorderRadius::all(percent(50)),
                                    ..default()
                                },
                                BackgroundColor(if index == 0 {
                                    status_violet()
                                } else {
                                    Color::srgb_u8(100, 180, 95)
                                }),
                                BorderColor::all(status_ink()),
                            ))
                            .with_children(|icon| {
                                spawn_status_text(icon, "C", font, 15.0, Color::BLACK);
                            });
                        spawn_status_text(category, label, font, 18.0, status_ink());
                        category.spawn(Node {
                            flex_grow: 1.0,
                            ..default()
                        });
                        if index == 1 {
                            spawn_status_text(
                                category,
                                format!("{:?}", member.row()),
                                font,
                                14.0,
                                status_muted(),
                            );
                        }
                    });
            }
        },
    );
}

fn spawn_profile_column(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
) {
    let Some(member) = member_at(game, state.member_index) else {
        return;
    };
    let description = catalog
        .class(member.class_id())
        .map_or("No class profile is available.", |class| {
            class.description.as_str()
        });
    let equipment = EquipmentSlot::ALL
        .into_iter()
        .map(|slot| {
            let name = member
                .equipment()
                .get(slot)
                .and_then(|id| catalog.item(id))
                .map_or("—", item_name);
            format!("{:<10}  {name}", slot.as_str().to_uppercase())
        })
        .collect::<Vec<_>>()
        .join("\n");
    let abilities = learned_field_abilities(member, game, catalog)
        .into_iter()
        .map(|ability| format!("•  {}", ability.name))
        .collect::<Vec<_>>();
    let statuses = member
        .status_effects()
        .map(|effect| format!("{effect:?}"))
        .collect::<Vec<_>>();

    spawn_status_panel(
        parent,
        Node {
            height: percent(100),
            flex_basis: px(0),
            flex_grow: 0.92,
            flex_direction: FlexDirection::Column,
            row_gap: px(10),
            padding: UiRect::all(px(16)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        "PROFILE & LOADOUT",
        font,
        |panel| {
            spawn_status_text(panel, "PROFILE", font, 14.0, status_gold());
            spawn_status_text(panel, description, font, 16.0, status_ink());

            spawn_section_rule(panel);
            spawn_status_text(panel, "EQUIPMENT", font, 14.0, status_gold());
            panel
                .spawn((
                    Node {
                        width: percent(100),
                        padding: UiRect::all(px(10)),
                        border: UiRect::all(px(1)),
                        border_radius: BorderRadius::all(px(4)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba_u8(10, 10, 14, 150)),
                    BorderColor::all(status_border()),
                ))
                .with_children(|box_node| {
                    spawn_status_text(box_node, &equipment, font, 14.0, status_ink());
                });

            spawn_section_rule(panel);
            spawn_status_text(panel, "FIELD ARTS", font, 14.0, status_gold());
            spawn_status_text(
                panel,
                if abilities.is_empty() {
                    "None learned".to_owned()
                } else {
                    abilities.join("\n")
                },
                font,
                15.0,
                status_teal(),
            );
            spawn_status_text(
                panel,
                format!(
                    "STATUS    {}",
                    if statuses.is_empty() {
                        "CLEAR".to_owned()
                    } else {
                        statuses.join(", ").to_uppercase()
                    }
                ),
                font,
                14.0,
                if statuses.is_empty() {
                    Color::srgb_u8(132, 196, 111)
                } else {
                    status_ember()
                },
            );
        },
    );
}

#[expect(
    clippy::too_many_arguments,
    reason = "the shared portrait frame keeps profile and status art styling identical"
)]
fn spawn_portrait_frame(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    member_name: &str,
    portrait: Option<Handle<Image>>,
    width: f32,
    height: f32,
    selected: bool,
    label: &'static str,
) {
    parent
        .spawn((
            Node {
                width: px(width),
                height: px(height),
                flex_shrink: 0.0,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(px(if selected { 2 } else { 1 })),
                border_radius: BorderRadius::all(px(5)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgba_u8(10, 10, 14, 188)),
            BorderColor::all(if selected {
                status_border_active()
            } else {
                status_border()
            }),
            Name::new(label),
        ))
        .with_children(|frame| {
            spawn_status_text(
                frame,
                member_emblem(member_name),
                font,
                if height > 100.0 { 34.0 } else { 18.0 },
                if selected {
                    status_gold()
                } else {
                    status_ink()
                },
            );
            if let Some(portrait) = portrait {
                frame.spawn((
                    ImageNode::new(portrait).with_mode(NodeImageMode::Stretch),
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(0),
                        top: px(0),
                        width: percent(100),
                        height: percent(100),
                        ..default()
                    },
                ));
            }
        });
}

fn load_status_image(
    asset_server: &AssetServer,
    root: &ScenarioRoot,
    relative: &str,
) -> Option<Handle<Image>> {
    let relative = ScenarioRelativePath::try_from(relative).ok()?;
    Some(asset_server.load(root.resolve(&relative)))
}

fn profile_portrait_path(member_id: &str) -> String {
    format!("assets/images/{member_id}_profile.png")
}

fn large_status_portrait_path(member_id: &str) -> String {
    format!("assets/images/party_portraits_large/{member_id}_status_portrait.webp")
}

fn spawn_status_panel(
    parent: &mut ChildSpawnerCommands<'_>,
    node: Node,
    title: &str,
    font: &Handle<Font>,
    content: impl FnOnce(&mut ChildSpawnerCommands<'_>),
) {
    parent
        .spawn((
            node,
            BackgroundColor(Color::srgba_u8(22, 22, 28, 228)),
            BorderColor::all(status_border()),
        ))
        .with_children(|panel| {
            spawn_status_text(panel, title, font, 14.0, status_gold());
            panel.spawn((
                Node {
                    width: percent(100),
                    height: px(1),
                    margin: UiRect::bottom(px(2)),
                    ..default()
                },
                BackgroundColor(Color::srgba_u8(126, 98, 55, 150)),
            ));
            content(panel);
        });
}

fn spawn_meter(
    parent: &mut ChildSpawnerCommands<'_>,
    label: &str,
    value: u32,
    maximum: u32,
    font: &Handle<Font>,
    fill: Color,
) {
    parent
        .spawn(Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(3),
            ..default()
        })
        .with_children(|meter| {
            meter
                .spawn(Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                })
                .with_children(|label_row| {
                    spawn_status_text(label_row, label, font, 13.0, status_muted());
                    spawn_status_text(
                        label_row,
                        format!("{value} / {maximum}"),
                        font,
                        13.0,
                        status_ink(),
                    );
                });
            meter
                .spawn((
                    Node {
                        width: percent(100),
                        height: px(8),
                        border_radius: BorderRadius::all(px(4)),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(Color::srgb_u8(17, 17, 22)),
                ))
                .with_children(|track| {
                    track.spawn((
                        Node {
                            width: percent(meter_percent(value, maximum)),
                            height: percent(100),
                            border_radius: BorderRadius::all(px(4)),
                            ..default()
                        },
                        BackgroundColor(fill),
                    ));
                });
        });
}

fn spawn_section_rule(parent: &mut ChildSpawnerCommands<'_>) {
    parent.spawn((
        Node {
            width: percent(100),
            height: px(1),
            margin: UiRect::axes(px(0), px(2)),
            ..default()
        },
        BackgroundColor(Color::srgba_u8(126, 98, 55, 100)),
    ));
}

fn spawn_status_text(
    parent: &mut ChildSpawnerCommands<'_>,
    text: impl Into<String>,
    font: &Handle<Font>,
    size: f32,
    color: Color,
) {
    parent.spawn((
        Text::new(text),
        TextFont {
            font: font.clone().into(),
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
        TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
    ));
}

fn meter_percent(value: u32, maximum: u32) -> f32 {
    if maximum == 0 {
        0.0
    } else {
        (value as f32 / maximum as f32 * 100.0).clamp(0.0, 100.0)
    }
}

fn selected_inventory_index(state: &FieldMenuState, ids: &[&str]) -> usize {
    if ids.is_empty() {
        return 0;
    }
    if state.mode == FieldMenuMode::Browse {
        return state.selected.min(ids.len() - 1);
    }
    state
        .pending_id
        .as_deref()
        .and_then(|pending| ids.iter().position(|id| *id == pending))
        .unwrap_or(0)
}

fn selected_inventory_id<'a>(
    state: &'a FieldMenuState,
    ids: &'a [&'a str],
    list_index: usize,
) -> Option<&'a str> {
    if state.mode == FieldMenuMode::Browse {
        ids.get(list_index).copied()
    } else {
        state.pending_id.as_deref()
    }
}

fn item_kind_label(item: &ItemDefinition) -> &'static str {
    match item {
        ItemDefinition::Consumable(_) => "CONSUMABLE",
        ItemDefinition::Material(_) => "MATERIAL",
        ItemDefinition::Key(_) => "KEY ITEM",
        ItemDefinition::MagicCore(_) => "MAGIC CORE",
        ItemDefinition::Weapon(_) => "WEAPON",
        ItemDefinition::Shield(_) => "SHIELD",
        ItemDefinition::Helmet(_) => "HELMET",
        ItemDefinition::Body(_) => "BODY ARMOR",
        ItemDefinition::Accessory(_) => "ACCESSORY",
    }
}

fn item_kind_abbreviation(item: &ItemDefinition) -> &'static str {
    match item {
        ItemDefinition::Consumable(_) => "USE",
        ItemDefinition::Material(_) => "MAT",
        ItemDefinition::Key(_) => "KEY",
        ItemDefinition::MagicCore(_) => "CORE",
        ItemDefinition::Weapon(_) => "WPN",
        ItemDefinition::Shield(_) => "SHLD",
        ItemDefinition::Helmet(_) => "HELM",
        ItemDefinition::Body(_) => "BODY",
        ItemDefinition::Accessory(_) => "ACC",
    }
}

fn item_accent(item: &ItemDefinition) -> Color {
    match item {
        ItemDefinition::Consumable(_) => status_teal(),
        ItemDefinition::Material(_) => Color::srgb_u8(157, 139, 101),
        ItemDefinition::Key(_) => status_gold(),
        ItemDefinition::MagicCore(_) => status_violet(),
        ItemDefinition::Weapon(_) => status_ember(),
        ItemDefinition::Shield(_) => Color::srgb_u8(91, 143, 183),
        ItemDefinition::Helmet(_) => Color::srgb_u8(126, 151, 174),
        ItemDefinition::Body(_) => Color::srgb_u8(112, 126, 148),
        ItemDefinition::Accessory(_) => Color::srgb_u8(196, 116, 168),
    }
}

fn member_emblem(name: &str) -> String {
    let words = name.split_whitespace().collect::<Vec<_>>();
    let letters = if words.len() > 1 {
        words
            .iter()
            .filter_map(|word| word.chars().next())
            .take(2)
            .collect::<String>()
    } else {
        name.chars()
            .filter(|character| !character.is_whitespace())
            .take(2)
            .collect()
    };
    let emblem = letters.to_uppercase();
    if emblem.is_empty() {
        "?".to_owned()
    } else {
        emblem
    }
}

fn class_name<'a>(class_id: &'a str, catalog: &'a FieldMenuCatalog) -> &'a str {
    catalog
        .class(class_id)
        .map_or(class_id, |class| class.name.as_str())
}

fn status_slot_label(slot: EquipmentSlot) -> &'static str {
    match slot {
        EquipmentSlot::Weapon => "WPN",
        EquipmentSlot::Shield => "SHLD",
        EquipmentSlot::Helmet => "HELM",
        EquipmentSlot::Body => "BODY",
        EquipmentSlot::Accessory => "ACC",
    }
}

fn status_ink() -> Color {
    Color::srgb_u8(242, 236, 211)
}

fn status_muted() -> Color {
    Color::srgb_u8(184, 174, 142)
}

fn status_gold() -> Color {
    Color::srgb_u8(231, 184, 86)
}

fn status_ember() -> Color {
    Color::srgb_u8(203, 82, 47)
}

fn status_teal() -> Color {
    Color::srgb_u8(67, 166, 160)
}

fn status_violet() -> Color {
    Color::srgb_u8(126, 101, 204)
}

fn status_border() -> Color {
    Color::srgb_u8(126, 98, 55)
}

fn status_border_active() -> Color {
    Color::srgb_u8(235, 190, 89)
}

fn screen_title(state: &FieldMenuState) -> &'static str {
    match state.screen {
        FieldMenuScreen::Main => "Field Menu",
        FieldMenuScreen::Status => "Status",
        FieldMenuScreen::Items => "Items",
        FieldMenuScreen::Equipment => "Equipment",
        FieldMenuScreen::Spells => "Spells",
        FieldMenuScreen::Save => "Save Game",
    }
}

fn render_body(
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
    saves: &SaveSlotCatalog,
) -> String {
    if catalog.status() == CatalogStatus::Loading {
        return "Loading class and item catalogs...".to_owned();
    }
    if catalog.status() == CatalogStatus::Failed {
        return format!(
            "Catalog load failed:\n{}",
            catalog.failure().unwrap_or("unknown failure")
        );
    }
    let mut text = match state.screen {
        FieldMenuScreen::Main => render_main(state, game),
        FieldMenuScreen::Status => render_status(state, game, catalog),
        FieldMenuScreen::Items => render_items(state, game, catalog),
        FieldMenuScreen::Equipment => render_equipment(state, game, catalog),
        FieldMenuScreen::Spells => render_spells(state, game, catalog),
        FieldMenuScreen::Save => render_save(state, saves),
    };
    if !state.message.is_empty() {
        text.push_str("\n\n");
        text.push_str(&state.message);
    }
    text
}

fn render_main(state: &FieldMenuState, game: &GameState) -> String {
    let commands = MAIN_COMMANDS
        .iter()
        .enumerate()
        .map(|(index, label)| {
            let cursor = if index == state.selected { ">" } else { " " };
            format!("{cursor} {label}")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let party = game
        .party()
        .members()
        .map(|member| {
            format!(
                "{:<12} Lv {:>2}  HP {:>3}/{:<3}  MP {:>3}/{:<3}  {:?}",
                member.name(),
                member.level(),
                member.health(),
                member.max_health(),
                member.mana(),
                member.max_mana(),
                member.row()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "{commands}\n\nParty                                      GP {:>8}\n{party}",
        game.repository().gp()
    )
}

fn render_status(state: &FieldMenuState, game: &GameState, catalog: &FieldMenuCatalog) -> String {
    let Some(member) = member_at(game, state.member_index) else {
        return "No party members.".to_owned();
    };
    let stats = derived_stats(member, catalog);
    let statuses = member
        .status_effects()
        .map(|status| format!("{status:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    let equipment = EquipmentSlot::ALL
        .into_iter()
        .map(|slot| {
            let name = member
                .equipment()
                .get(slot)
                .and_then(|id| catalog.item(id))
                .map_or("(Empty)", item_name);
            format!("  {:<10} {name}", slot.as_str())
        })
        .collect::<Vec<_>>()
        .join("\n");
    let abilities = learned_field_abilities(member, game, catalog)
        .into_iter()
        .map(|ability| ability.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Member {}/{}  {} — {}  Lv {}  {:?}\nHP {}/{}    MP {}/{}    EXP {}\n\nBase     STR {:>3}  DEX {:>3}  CON {:>3}  INT {:>3}\nDerived  STR {:>3}  DEX {:>3}  CON {:>3}  INT {:>3}\n\nEquipment\n{}\n\nField abilities: {}\nStatus effects: {}",
        state.member_index + 1,
        game.party().len(),
        member.name(),
        member.class_id(),
        member.level(),
        member.row(),
        member.health(),
        member.max_health(),
        member.mana(),
        member.max_mana(),
        member.experience(),
        member.stats().strength(),
        member.stats().dexterity(),
        member.stats().constitution(),
        member.stats().intelligence(),
        stats.strength,
        stats.dexterity,
        stats.constitution,
        stats.intelligence,
        equipment,
        if abilities.is_empty() {
            "None"
        } else {
            &abilities
        },
        if statuses.is_empty() {
            "None"
        } else {
            &statuses
        }
    )
}

fn render_items(state: &FieldMenuState, game: &GameState, catalog: &FieldMenuCatalog) -> String {
    let tabs = InventoryTab::ALL
        .iter()
        .enumerate()
        .map(|(index, tab)| {
            if index == state.tab_index {
                format!("[{}]", tab.label())
            } else {
                tab.label().to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("  ");
    let ids = inventory_ids(game, catalog, InventoryTab::ALL[state.tab_index]);
    let page = inventory_page_range(ids.len(), state.selected);
    let rows = ids
        .iter()
        .enumerate()
        .skip(page.start)
        .take(page.len())
        .map(|(index, id)| {
            let cursor = if state.mode == FieldMenuMode::Browse && index == state.selected {
                ">"
            } else {
                " "
            };
            let item = catalog.item(id).expect("filtered catalog item");
            format!(
                "{cursor} {:<30} x{:>3}",
                item_name(item),
                game.repository().item_count(id)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let detail = state
        .pending_id
        .as_deref()
        .or_else(|| ids.get(state.selected).copied())
        .and_then(|id| catalog.item(id))
        .map(item_description)
        .unwrap_or("No items in this tab.");
    let overlay = match state.mode {
        FieldMenuMode::ItemActions => ["Use", "Discard", "Hide"]
            .iter()
            .enumerate()
            .map(|(index, label)| {
                format!(
                    "{} {label}",
                    if index == state.selected { ">" } else { " " }
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
        FieldMenuMode::DiscardQuantity => format!("Discard how many?  {}", state.quantity),
        FieldMenuMode::ItemTarget => render_targets(state, game),
        _ => String::new(),
    };
    format!(
        "{tabs}\n\n{}\n\n{}\n{}",
        if rows.is_empty() { "(empty)" } else { &rows },
        detail,
        overlay
    )
}

fn render_equipment(
    state: &FieldMenuState,
    game: &GameState,
    catalog: &FieldMenuCatalog,
) -> String {
    let Some(member) = member_at(game, state.member_index) else {
        return "No party members.".to_owned();
    };
    let slot_index = state_slot_index(state);
    let slots = EquipmentSlot::ALL
        .into_iter()
        .enumerate()
        .map(|(index, slot)| {
            let cursor = if state.mode == FieldMenuMode::Browse && index == state.selected {
                ">"
            } else {
                " "
            };
            let name = member
                .equipment()
                .get(slot)
                .and_then(|id| catalog.item(id))
                .map_or("(Empty)", item_name);
            format!("{cursor} {:<10} {name}", slot.as_str())
        })
        .collect::<Vec<_>>()
        .join("\n");
    if state.mode != FieldMenuMode::EquipmentPicker {
        return format!(
            "Member {}/{}  {} — {}\n\n{slots}",
            state.member_index + 1,
            game.party().len(),
            member.name(),
            member.class_id()
        );
    }
    let slot = EquipmentSlot::ALL[slot_index];
    let candidates = equipment_candidates(game, catalog, slot);
    let rows = std::iter::once(None)
        .chain(candidates.iter().map(|id| Some(id.as_str())))
        .enumerate()
        .map(|(index, id)| {
            let cursor = if index == state.selected { ">" } else { " " };
            match id {
                None => format!("{cursor} (Unequip)"),
                Some(id) => {
                    let item = catalog.item(id).expect("candidate exists");
                    let blocked = can_equip(member, item, catalog)
                        .err()
                        .map(|error| format!("  [{}]", error))
                        .unwrap_or_default();
                    format!("{cursor} {}{blocked}", item_name(item))
                }
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let selected_id = state
        .selected
        .checked_sub(1)
        .and_then(|index| candidates.get(index).map(String::as_str));
    let before = derived_stats(member, catalog);
    let after = preview_stats(member, catalog, slot, selected_id);
    format!(
        "Member {}/{}  {} — {}\n\n{slots}\n\nChoose {}\n{}\n\nPreview  STR {}->{}, DEX {}->{}, CON {}->{}, INT {}->{}",
        state.member_index + 1,
        game.party().len(),
        member.name(),
        member.class_id(),
        slot.as_str(),
        rows,
        before.strength,
        after.strength,
        before.dexterity,
        after.dexterity,
        before.constitution,
        after.constitution,
        before.intelligence,
        after.intelligence
    )
}

fn render_spells(state: &FieldMenuState, game: &GameState, catalog: &FieldMenuCatalog) -> String {
    let Some(member) = member_at(game, state.member_index) else {
        return "No party members.".to_owned();
    };
    let abilities = learned_field_abilities(member, game, catalog);
    let rows = abilities
        .iter()
        .enumerate()
        .map(|(index, ability)| {
            format!(
                "{} {:<24} MP {:>3}  {}",
                if state.mode == FieldMenuMode::Browse && index == state.selected {
                    ">"
                } else {
                    " "
                },
                ability.name,
                ability.mp_cost,
                ability.description
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let overlay = match state.mode {
        FieldMenuMode::SpellTarget => render_targets(state, game),
        FieldMenuMode::TeleportPicker => catalog
            .eligible_warp_destinations(game.map())
            .iter()
            .enumerate()
            .map(|(index, destination)| {
                format!(
                    "{} {}",
                    if index == state.selected { ">" } else { " " },
                    destination.name
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    };
    format!(
        "Member {}/{}  {} — MP {}/{}\n\n{}\n\n{}",
        state.member_index + 1,
        game.party().len(),
        member.name(),
        member.mana(),
        member.max_mana(),
        if rows.is_empty() {
            "(No learned field abilities)"
        } else {
            &rows
        },
        overlay
    )
}

fn render_targets(state: &FieldMenuState, game: &GameState) -> String {
    game.party()
        .members()
        .enumerate()
        .map(|(index, member)| {
            format!(
                "{} {:<12} HP {}/{}  MP {}/{}",
                if index == state.selected { ">" } else { " " },
                member.name(),
                member.health(),
                member.max_health(),
                member.mana(),
                member.max_mana()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_save(state: &FieldMenuState, saves: &SaveSlotCatalog) -> String {
    let page_start =
        ((state.selected.saturating_sub(FIRST_PLAYER_SLOT)) / 7) * 7 + FIRST_PLAYER_SLOT;
    let rows = saves
        .slots()
        .iter()
        .skip(page_start)
        .take(7)
        .map(|slot| {
            let cursor = if slot.index == state.selected {
                ">"
            } else {
                " "
            };
            match (&slot.state, &slot.metadata) {
                (SaveSlotState::Empty, _) => {
                    format!("{cursor} {:<8} --- Empty ---", slot.label())
                }
                (SaveSlotState::Valid, Some(metadata)) => format!(
                    "{cursor} {:<8} {} Lv{}  {}  {}",
                    slot.label(),
                    metadata.protagonist_name,
                    metadata.protagonist_level,
                    crate::playtime::Playtime::format(metadata.playtime_seconds),
                    metadata.location,
                ),
                (SaveSlotState::Corrupt(_), _) => {
                    format!("{cursor} {:<8} [CORRUPT]", slot.label())
                }
                (SaveSlotState::Incompatible(_), _) => {
                    format!("{cursor} {:<8} [INCOMPATIBLE]", slot.label())
                }
                _ => format!("{cursor} {:<8} [INVALID METADATA]", slot.label()),
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    if rows.is_empty() {
        "Discovering native save slots...".to_owned()
    } else {
        rows
    }
}

fn render_hint(state: &FieldMenuState) -> String {
    match (state.screen, state.mode) {
        (FieldMenuScreen::Main, FieldMenuMode::QuitConfirm) => {
            "Y/ENTER return to title  N/ESC cancel"
        }
        (FieldMenuScreen::Main, _) => "UP/DOWN choose  ENTER open  M/ESC close",
        (FieldMenuScreen::Status, _) if state.status_page == StatusPage::Roster => {
            "UP/DOWN member  ENTER stats  ESC back  M close"
        }
        (FieldMenuScreen::Status, _) => "UP/DOWN action  ESC portrait  M close",
        (FieldMenuScreen::Items, FieldMenuMode::Browse) => {
            "LEFT/RIGHT tab  UP/DOWN item  ENTER actions  ESC back"
        }
        (FieldMenuScreen::Items, FieldMenuMode::DiscardQuantity) => {
            "UP/DOWN quantity  LEFT one  RIGHT whole stack  ENTER discard  ESC cancel"
        }
        (FieldMenuScreen::Equipment, FieldMenuMode::Browse) => {
            "LEFT/RIGHT member  UP/DOWN slot  ENTER choose  ESC back"
        }
        (FieldMenuScreen::Spells, FieldMenuMode::Browse) => {
            "LEFT/RIGHT member  UP/DOWN spell  ENTER cast  ESC back"
        }
        (FieldMenuScreen::Save, FieldMenuMode::Browse) => {
            "UP/DOWN slot  ENTER save  ESC back  M close"
        }
        (FieldMenuScreen::Save, FieldMenuMode::SaveConfirm) => "Y/ENTER overwrite  N/ESC cancel",
        (_, FieldMenuMode::ItemTarget | FieldMenuMode::SpellTarget) => {
            "UP/DOWN target  ENTER confirm  ESC cancel"
        }
        (_, FieldMenuMode::TeleportPicker) => "UP/DOWN destination  ENTER teleport  ESC cancel",
        _ => "UP/DOWN choose  ENTER confirm  ESC cancel  M close",
    }
    .to_owned()
}

fn cleanup_field_menu(
    mut commands: Commands,
    roots: Query<Entity, With<FieldMenuRoot>>,
    mut state: ResMut<FieldMenuState>,
) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    state.close();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save_data::tests::fixture_game;

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
        assert_eq!(inventory_page_range(25, 24), 24..25);
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
        assert_eq!(main_command_screen(3), Some(FieldMenuScreen::Spells));
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
    fn confirmed_quit_discards_the_session_before_returning_to_title() {
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
            .add_message::<AppStateTransitionRequest>()
            .add_systems(Update, handle_field_menu_input);

        app.update();

        assert!(!app.world().contains_resource::<GameState>());
        assert!(!app.world().resource::<FieldMenuState>().open);
    }
}
