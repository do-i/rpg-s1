//! In-world field-menu shell over the shared M6 runtime domain.

use bevy::{
    ecs::{hierarchy::ChildSpawnerCommands, schedule::ApplyDeferred},
    input::{ButtonState, keyboard::KeyboardInput},
    prelude::*,
};

use crate::{
    action_input::{ActionState, AppAction},
    app_state::AppState,
    engine_settings::EngineSettings,
    field_menu_domain::{
        CUSTOM_TAG_MAX_LENGTH, CatalogStatus, EDITABLE_SYSTEM_TAGS, FieldMenuCatalog, InventoryTab,
        can_equip, cast_heal, custom_tags, derived_stats, discard_item, equip_item, inventory_ids,
        item_description, item_name, learned_field_abilities, normalize_custom_tag, preview_stats,
        targets_whole_party, unequip_item, use_field_item, use_field_item_on_party,
    },
    game_state::GameState,
    menu_chrome::{
        location_display_name, spawn_header_bars, spawn_meter, spawn_section_rule,
        spawn_status_panel, spawn_status_text, status_border, status_border_active, status_ember,
        status_faint, status_gold, status_ink, status_muted, status_teal, status_violet,
        window_start,
    },
    runtime_map::RuntimeMapId,
    runtime_member::EquipmentSlot,
    runtime_quest::{QuestStatus, quest_status},
    save_data::NativeSaveEnvelope,
    save_store::{
        FIRST_PLAYER_SLOT, LAST_PLAYER_SLOT, SaveSlot, SaveSlotState, SaveStore, unix_timestamp_now,
    },
    save_ui::{SaveSlotCatalog, save_slot_state_color, save_slot_state_label},
    scenario_class::{Ability, AbilityKind, UtilityAbility},
    scenario_inventory::ScenarioInventory,
    scenario_item::ItemDefinition,
    scenario_party::PartyRow,
    scenario_path::ScenarioRelativePath,
    scenario_quest::{QuestDefinition, QuestKind},
    scenario_root::ScenarioRoot,
    scenario_spatial::CardinalDirection,
    service_ui::ServiceUiState,
    sfx_cue::{MenuSfx, PlaySfx},
    tsx_atlas_asset::TsxAtlasAsset,
    ui_theme::UiTheme,
    world_interaction::WorldInteractionState,
    world_transition::WorldTransition,
};

fn scenario_font(
    asset_server: &AssetServer,
    root: &ScenarioRoot,
    inventory: &ScenarioInventory,
) -> Option<Handle<Font>> {
    inventory
        .font
        .as_ref()
        .map(|path| asset_server.load(root.resolve(path)))
}

mod ui;

use ui::{
    cleanup_field_menu, large_status_portrait_path, load_status_image, profile_portrait_path,
    sync_custom_field_menu_content_visibility, sync_equipment_page, sync_field_menu_generic_text,
    sync_field_menu_overlay_lifecycle, sync_items_page, sync_main_menu_page, sync_quests_page,
    sync_save_page, sync_spells_page, sync_status_page,
};

const INVENTORY_PAGE_ROWS: usize = 10;
const EQUIPMENT_PICKER_VISIBLE_ROWS: usize = 4;
const SPELLBOOK_VISIBLE_ROWS: usize = 7;
/// Rows per column on the two-column main command deck.
const MAIN_COMMAND_ROWS: usize = 4;
const QUEST_VISIBLE_ROWS: usize = 7;
const SAVE_VISIBLE_ROWS: usize = 6;
const ITEM_MANAGE_VISIBLE_ROWS: usize = 8;
/// Item-action rows, in the order the modal lists them.
const ITEM_ACTIONS: [(&str, &str); 4] = [
    ("Use", "apply this item"),
    ("Discard", "remove from pouch"),
    ("Hide", "hide for this session"),
    ("Edit Tags", "curate and add tags"),
];
const ITEM_ACTION_TAGS: usize = 3;
const QUIT_COMMAND_INDEX: usize = 3;
const SAVE_COMMAND_INDEX: usize = 4;
const CHARACTER_COMMAND_INDEX: usize = 7;

#[derive(Clone, Copy)]
struct MainCommand {
    label: &'static str,
    badge: &'static str,
    description: &'static str,
    screen: Option<FieldMenuScreen>,
}

const MAIN_COMMANDS: [MainCommand; 8] = [
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
        label: "Quit",
        badge: "QT",
        description: "exit the game to desktop",
        screen: None,
    },
    MainCommand {
        label: "Save",
        badge: "SV",
        description: "record the current journey",
        screen: Some(FieldMenuScreen::Save),
    },
    MainCommand {
        label: "Equipment",
        badge: "EQ",
        description: "tune gear and compare stats",
        screen: Some(FieldMenuScreen::Equipment),
    },
    MainCommand {
        label: "Quests",
        badge: "QU",
        description: "review active and completed quests",
        screen: Some(FieldMenuScreen::Quests),
    },
    MainCommand {
        label: "Character",
        badge: "CH",
        description: "control a different party member",
        screen: None,
    },
];

const STATUS_PARTY_WIDTH: f32 = 316.0;
const STATUS_DETAIL_WIDTH: f32 = 404.0;
const STATUS_COLUMN_GAP: f32 = 18.0;
const STATUS_CATEGORIES: [&str; 2] = ["Spells", "Position"];
/// Index of `Position` within [`STATUS_CATEGORIES`].
const STATUS_POSITION_CATEGORY: usize = 1;
/// Battle rows in picker order, matching Python's `status_renderer.ROWS`.
const STATUS_ROWS: [PartyRow; 2] = [PartyRow::Front, PartyRow::Back];
const ITEMS_POUCH_WIDTH: f32 = 286.0;
const ITEMS_DETAIL_WIDTH: f32 = 378.0;
const ITEMS_COLUMN_GAP: f32 = 18.0;
const EQUIPMENT_SLOT_WIDTH: f32 = 306.0;
const MAIN_DECK_WIDTH: f32 = 772.0;
const MAIN_DECK_COLUMN_GAP: f32 = 14.0;
const QUEST_LIST_WIDTH: f32 = 512.0;
const QUEST_COLUMN_GAP: f32 = 18.0;

pub(crate) struct FieldMenuPlugin;

impl Plugin for FieldMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PlaySfx>()
            .add_message::<KeyboardInput>()
            .init_resource::<FieldMenuState>()
            .init_resource::<PartyWalkSheets>()
            .add_systems(
                OnEnter(AppState::World),
                (reset_field_menu, load_party_walk_sheets),
            )
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
                    sync_quests_page,
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
    Quests,
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
    /// Roster overlay for choosing which member the World sprite follows.
    CharacterSwitch,
    /// Tag editor for one item: toggle curatorial tags, drop custom ones, add a new one.
    ItemTags,
    /// Free-text entry for a new custom tag.
    ItemNewTag,
    /// Show/hide manager listing every owned item, hidden ones included.
    ItemManage,
    /// Y/N guard before an item is spent on the whole party. Gated on `item.use_aoe_confirm`.
    ItemAoeConfirm,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum StatusPage {
    #[default]
    Roster,
    Details,
    /// Front/back row picker, reached from the Position category.
    Position,
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
    /// Draft text for the new-custom-tag prompt.
    text_input: String,
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
        if self.screen == FieldMenuScreen::Status {
            match self.status_page {
                StatusPage::Roster => {}
                StatusPage::Details => {
                    self.status_page = StatusPage::Roster;
                    self.selected = 0;
                    self.message.clear();
                    return;
                }
                StatusPage::Position => {
                    self.status_page = StatusPage::Details;
                    self.selected = STATUS_POSITION_CATEGORY;
                    self.message.clear();
                    return;
                }
            }
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
            FieldMenuMode::ItemTags => {
                self.mode = FieldMenuMode::ItemActions;
                self.selected = ITEM_ACTION_TAGS;
                self.message.clear();
            }
            FieldMenuMode::ItemNewTag => {
                self.mode = FieldMenuMode::ItemTags;
                self.text_input.clear();
                self.message.clear();
            }
            FieldMenuMode::ItemManage => {
                self.mode = FieldMenuMode::Browse;
                self.selected = 0;
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
            // Backing out of the guard returns to the item's action list, not to browsing, so
            // declining to spend a Tent does not also lose the player's place.
            FieldMenuMode::ItemAoeConfirm => {
                self.mode = FieldMenuMode::ItemActions;
                self.message.clear();
            }
            FieldMenuMode::QuitConfirm => {
                self.mode = FieldMenuMode::Browse;
                self.message.clear();
            }
            FieldMenuMode::CharacterSwitch => {
                self.mode = FieldMenuMode::Browse;
                self.selected = CHARACTER_COMMAND_INDEX;
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
struct FieldMenuCharacterModal;

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
struct FieldMenuQuestsPage;

#[derive(Component)]
struct QuestBoardRow;

#[derive(Component)]
struct SelectedQuestBoardRow;

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

/// Tile row holding the downward-facing idle frame on a cardinal walk sheet.
const WALK_SHEET_IDLE_DOWN_TILE: usize = 18;

/// Strong handles to each party member's walk sheet, kept so the switch overlay can draw a real
/// sprite frame instead of a lettered placeholder.
#[derive(Debug, Default, Resource)]
pub(crate) struct PartyWalkSheets {
    sheets: Vec<(String, Handle<TsxAtlasAsset>)>,
}

impl PartyWalkSheets {
    fn sheet(&self, member_id: &str) -> Option<&Handle<TsxAtlasAsset>> {
        self.sheets
            .iter()
            .find(|(id, _)| id == member_id)
            .map(|(_, handle)| handle)
    }
}

fn load_party_walk_sheets(
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
    mut sheets: ResMut<PartyWalkSheets>,
) {
    if !sheets.sheets.is_empty() {
        return;
    }
    sheets.sheets = inventory
        .party_sprites
        .iter()
        .map(|(member_id, path)| (member_id.clone(), asset_server.load(root.resolve(path))))
        .collect();
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
    settings: Res<EngineSettings>,
    interaction: Res<WorldInteractionState>,
    service: Option<Res<ServiceUiState>>,
    mut transition: ResMut<WorldTransition>,
    game: Option<ResMut<GameState>>,
    store: Res<SaveStore>,
    mut saves: ResMut<SaveSlotCatalog>,
    time: Res<Time<Real>>,
    mut state: ResMut<FieldMenuState>,
    mut keyboard: MessageReader<KeyboardInput>,
    mut exit: MessageWriter<AppExit>,
    mut menu_sfx: MenuSfx,
) {
    let Some(mut game) = game else { return };

    if !state.open {
        if interaction.input_locked()
            || service.as_deref().is_some_and(ServiceUiState::input_locked)
            || transition.input_locked()
        {
            return;
        }
        if keys.just_pressed(KeyCode::KeyM) || actions.just_pressed(AppAction::Back) {
            state.open(FieldMenuScreen::Main);
            menu_sfx.confirm();
        } else if keys.just_pressed(KeyCode::KeyI) {
            state.open(FieldMenuScreen::Items);
            menu_sfx.confirm();
        } else if keys.just_pressed(KeyCode::KeyS) {
            state.open(FieldMenuScreen::Status);
            menu_sfx.confirm();
        } else if keys.just_pressed(KeyCode::KeyQ) {
            state.open(FieldMenuScreen::Quests);
            menu_sfx.confirm();
        }
        return;
    }

    if keys.just_pressed(KeyCode::KeyM) {
        state.close();
        menu_sfx.cancel();
        return;
    }
    if actions.just_pressed(AppAction::Back) {
        state.back();
        menu_sfx.cancel();
        return;
    }
    // Drained every frame, not only while the tag prompt is open, so entering the prompt never
    // replays the keypress that opened it.
    let typed = keyboard
        .read()
        .filter(|input| input.state == ButtonState::Pressed)
        .cloned()
        .collect::<Vec<_>>();

    let horizontal = if keys.just_pressed(KeyCode::ArrowLeft) {
        Some(-1)
    } else if keys.just_pressed(KeyCode::ArrowRight) {
        Some(1)
    } else {
        None
    };
    let vertical = actions.menu_navigation();

    // Messages persist until the player's next input rather than one frame. Clearing them
    // unconditionally made every refusal ("This tab is empty", "Invalid tag") unreadable: the
    // page rebuilds on state change, so the banner appeared and vanished inside one frame.
    if !state.message.is_empty()
        && !matches!(
            state.mode,
            FieldMenuMode::SaveConfirm | FieldMenuMode::QuitConfirm | FieldMenuMode::ItemAoeConfirm
        )
        && (vertical.is_some()
            || horizontal.is_some()
            || !typed.is_empty()
            || actions.just_pressed(AppAction::Confirm))
    {
        state.message.clear();
    }

    // The four menu beats are wired here rather than inside the match below. That match has
    // thirteen early returns across seventeen confirm branches, so there is no single point
    // after it where "what actually changed" could be compared; hooking the shared entry points
    // instead keeps the cue wiring in one readable place.
    //
    // The trade-off is that Confirm sounds on every press while the menu is open, including a
    // press a branch ignores. Per-branch `blocked()` cues for the refusals are follow-up work.
    if vertical.is_some() || horizontal.is_some() {
        menu_sfx.hover();
    }
    if actions.just_pressed(AppAction::Confirm) {
        menu_sfx.confirm();
    }

    match (state.screen, state.mode) {
        (FieldMenuScreen::Main, FieldMenuMode::Browse) => {
            if vertical.is_some() || horizontal.is_some() {
                state.selected = stepped_main_command(state.selected, vertical, horizontal);
            }
            if actions.just_pressed(AppAction::Confirm) {
                if state.selected == CHARACTER_COMMAND_INDEX {
                    state.mode = FieldMenuMode::CharacterSwitch;
                    state.selected = controlled_member_index(&game);
                } else if state.selected == SAVE_COMMAND_INDEX {
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
        (FieldMenuScreen::Main, FieldMenuMode::CharacterSwitch) => {
            if let Some(delta) = vertical {
                state.selected = wrapped_or_zero(state.selected, game.party().len(), delta);
            }
            if actions.just_pressed(AppAction::Confirm) {
                switch_controlled_member(&mut game, &mut state);
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
        (FieldMenuScreen::Items, FieldMenuMode::ItemAoeConfirm) => {
            if keys.just_pressed(KeyCode::KeyN) {
                state.mode = FieldMenuMode::ItemActions;
                state.message.clear();
            } else if keys.just_pressed(KeyCode::KeyY) || actions.just_pressed(AppAction::Confirm) {
                let id = state
                    .pending_id
                    .clone()
                    .expect("a party-wide confirm requires an item");
                apply_party_item(&mut state, &mut game, &catalog, &id);
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
        (FieldMenuScreen::Status, FieldMenuMode::Browse) => match state.status_page {
            StatusPage::Roster => {
                if let Some(delta) = horizontal.or(vertical) {
                    cycle_member(&mut state, game.party().len(), delta);
                }
                if actions.just_pressed(AppAction::Confirm) && !game.party().is_empty() {
                    state.status_page = StatusPage::Details;
                    state.selected = 0;
                }
            }
            StatusPage::Details => {
                if let Some(delta) = vertical {
                    state.selected = wrapped(state.selected, STATUS_CATEGORIES.len(), delta);
                }
                if actions.just_pressed(AppAction::Confirm) {
                    if state.selected == STATUS_POSITION_CATEGORY {
                        let current =
                            member_at(&game, state.member_index).map(|member| member.row());
                        state.status_page = StatusPage::Position;
                        state.selected = current
                            .and_then(|row| STATUS_ROWS.iter().position(|entry| *entry == row))
                            .unwrap_or_default();
                        state.message.clear();
                    } else {
                        // The spellbook is its own screen in this port; carry the member across.
                        state.screen = FieldMenuScreen::Spells;
                        state.status_page = StatusPage::Roster;
                        state.selected = 0;
                        state.message.clear();
                    }
                }
            }
            StatusPage::Position => {
                if let Some(delta) = vertical {
                    state.selected = wrapped(state.selected, STATUS_ROWS.len(), delta);
                }
                if actions.just_pressed(AppAction::Confirm) {
                    set_member_row(&mut game, &mut state);
                }
            }
        },
        (FieldMenuScreen::Items, FieldMenuMode::Browse) => {
            if let Some(delta) = horizontal {
                state.tab_index = wrapped(state.tab_index, InventoryTab::ALL.len(), delta);
                state.selected = 0;
            }
            let ids = inventory_ids(&game, &catalog, InventoryTab::ALL[state.tab_index]);
            if let Some(delta) = vertical {
                state.selected = wrapped_or_zero(state.selected, ids.len(), delta);
            }
            // Python opens the manage modal with M; here M closes the whole field menu, so the
            // show/hide manager takes H instead.
            if keys.just_pressed(KeyCode::KeyH) {
                if manage_ids(&game, &catalog).is_empty() {
                    state.message = "The pouch is empty.".to_owned();
                } else {
                    state.mode = FieldMenuMode::ItemManage;
                    state.selected = 0;
                }
            } else if actions.just_pressed(AppAction::Confirm) {
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
                state.selected = wrapped(state.selected, ITEM_ACTIONS.len(), delta);
            }
            if actions.just_pressed(AppAction::Confirm) {
                let id = state
                    .pending_id
                    .clone()
                    .expect("item action requires an item");
                match state.selected {
                    // A party-wide item never reaches the target picker: the source branches on
                    // the effect's target before offering one, and routing a Tent through the
                    // picker is what made it unusable -- every member was an invalid target.
                    0 if targets_whole_party(&catalog, &id) => {
                        if settings.use_aoe_confirm {
                            state.mode = FieldMenuMode::ItemAoeConfirm;
                            state.selected = 0;
                        } else {
                            apply_party_item(&mut state, &mut game, &catalog, &id);
                        }
                    }
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
                    ITEM_ACTION_TAGS => {
                        state.mode = FieldMenuMode::ItemTags;
                        state.selected = 0;
                    }
                    _ => unreachable!(),
                }
            }
        }
        (FieldMenuScreen::Items, FieldMenuMode::ItemTags) => {
            let id = state
                .pending_id
                .clone()
                .expect("the tag editor requires an item");
            let rows = tag_editor_rows(&game, &id);
            if let Some(delta) = vertical {
                state.selected = wrapped(state.selected, rows.len(), delta);
            }
            if actions.just_pressed(AppAction::Confirm) {
                match rows.get(state.selected) {
                    Some(TagEditorRow::New) => {
                        state.mode = FieldMenuMode::ItemNewTag;
                        state.text_input.clear();
                        state.message.clear();
                    }
                    Some(TagEditorRow::Tag(tag)) => {
                        let tag = tag.clone();
                        toggle_item_tag(&mut game, &mut state, &id, &tag);
                    }
                    None => {}
                }
            }
        }
        (FieldMenuScreen::Items, FieldMenuMode::ItemNewTag) => {
            let id = state
                .pending_id
                .clone()
                .expect("the tag editor requires an item");
            for input in typed {
                match input.key_code {
                    KeyCode::Backspace => {
                        state.text_input.pop();
                    }
                    KeyCode::Enter | KeyCode::NumpadEnter if !input.repeat => {
                        commit_new_tag(&mut game, &mut state, &id);
                    }
                    KeyCode::Escape => {}
                    _ => {
                        if let Some(text) = input.text.as_deref() {
                            append_tag_text(&mut state.text_input, text);
                        }
                    }
                }
            }
        }
        (FieldMenuScreen::Items, FieldMenuMode::ItemManage) => {
            let ids = manage_ids(&game, &catalog);
            if let Some(delta) = vertical {
                state.selected = wrapped_or_zero(state.selected, ids.len(), delta);
            }
            if keys.just_pressed(KeyCode::KeyH) {
                state.back();
            } else if actions.just_pressed(AppAction::Confirm)
                && let Some(id) = ids.get(state.selected).map(|id| (*id).to_owned())
            {
                let hidden = game.repository_mut().toggle_hidden(&id);
                state.message = if hidden {
                    "Hidden from the pouch.".to_owned()
                } else {
                    "Shown in the pouch again.".to_owned()
                };
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
        (FieldMenuScreen::Quests, FieldMenuMode::Browse) => {
            if let Some(delta) = vertical {
                state.selected = wrapped_or_zero(state.selected, catalog.quests().len(), delta);
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

/// Spends a party-wide item and returns the menu to browsing, from either route into it.
fn apply_party_item(
    state: &mut FieldMenuState,
    game: &mut GameState,
    catalog: &FieldMenuCatalog,
    item_id: &str,
) {
    match use_field_item_on_party(game, catalog, item_id) {
        Ok(changed) => {
            state.mode = FieldMenuMode::Browse;
            state.selected = 0;
            state.pending_id = None;
            state.message = format!("Applied {changed} points/effects to the party.");
        }
        Err(error) => {
            state.mode = FieldMenuMode::ItemActions;
            state.message = error.to_string();
        }
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

/// Applies the Position picker's selection to the focused member.
///
/// Mirrors `status_scene._confirm_position`: re-picking the row the member already holds is a
/// no-op rather than an error, and the picker stays open either way.
fn set_member_row(game: &mut GameState, state: &mut FieldMenuState) {
    let Some(row) = STATUS_ROWS.get(state.selected).copied() else {
        return;
    };
    let Some(member_id) = member_id_at(game, state.member_index).map(ToOwned::to_owned) else {
        return;
    };
    let name = member_at(game, state.member_index)
        .map_or_else(|| member_id.clone(), |member| member.name().to_owned());
    match game.party_mut().set_row(&member_id, row) {
        Ok(previous) if previous == row => {
            state.message = format!("{name} already holds the {} row.", row_label(row));
        }
        Ok(_) => {
            state.message = format!("{name} moved to the {} row.", row_label(row));
        }
        Err(_) => {
            state.message = "That party member is no longer available.".to_owned();
        }
    }
}

/// One row of the tag editor: a toggleable tag, or the prompt that opens free-text entry.
#[derive(Clone, Debug, Eq, PartialEq)]
enum TagEditorRow {
    Tag(String),
    New,
}

/// The editor's rows for one item: the curatorial set, then that item's own tags, then `New tag`.
///
/// Ports `item_scene._editor_rows`. Type-driven tags are catalog-owned and never listed, so a row
/// here is always something the player may add or drop.
fn tag_editor_rows(game: &GameState, item_id: &str) -> Vec<TagEditorRow> {
    let mut rows = EDITABLE_SYSTEM_TAGS
        .into_iter()
        .map(|tag| TagEditorRow::Tag(tag.to_owned()))
        .collect::<Vec<_>>();
    rows.extend(
        custom_tags(game, item_id)
            .into_iter()
            .map(|tag| TagEditorRow::Tag(tag.to_owned())),
    );
    rows.push(TagEditorRow::New);
    rows
}

/// Adds a tag the item lacks, or removes one it has. Ports `item_scene._activate_editor_row`.
fn toggle_item_tag(game: &mut GameState, state: &mut FieldMenuState, item_id: &str, tag: &str) {
    if game.repository().item_tags(item_id).any(|held| held == tag) {
        game.repository_mut().remove_tag(item_id, tag);
        state.message = format!("Removed `{tag}`.");
        return;
    }
    let cap = game.repository().max_tags_per_item();
    if game.repository_mut().add_tags(item_id, [tag]).is_err() {
        state.message = format!("Max tags ({cap}) reached.");
    } else {
        state.message = format!("Added `{tag}`.");
    }
}

/// Validates and stores the drafted custom tag, keeping the prompt open on rejection.
///
/// Ports `item_scene._commit_new_tag`, which distinguishes an invalid tag, a duplicate, and the
/// per-item cap so the player learns which one blocked them.
fn commit_new_tag(game: &mut GameState, state: &mut FieldMenuState, item_id: &str) {
    let Some(tag) = normalize_custom_tag(&state.text_input) else {
        state.message = "Invalid tag.".to_owned();
        return;
    };
    if game.repository().item_tags(item_id).any(|held| held == tag) {
        state.message = "Tag already added.".to_owned();
        return;
    }
    let cap = game.repository().max_tags_per_item();
    if game
        .repository_mut()
        .add_tags(item_id, [tag.as_str()])
        .is_err()
    {
        state.message = format!("Max tags ({cap}) reached.");
        return;
    }
    state.mode = FieldMenuMode::ItemTags;
    state.text_input.clear();
    state.message = format!("Added `{tag}`.");
}

/// Appends one scalar at a time so a multi-scalar platform text event cannot cross the cap.
fn append_tag_text(draft: &mut String, text: &str) {
    let remaining = CUSTOM_TAG_MAX_LENGTH.saturating_sub(draft.chars().count());
    draft.extend(
        text.chars()
            .filter(|character| !character.is_control())
            .take(remaining),
    );
}

/// Every owned item the manage modal lists, hidden ones included.
///
/// Ports `item_scene._manage_entries`, which sorts by id and deliberately ignores the tab filter
/// so a hidden item is always reachable again.
fn manage_ids<'a>(game: &'a GameState, catalog: &FieldMenuCatalog) -> Vec<&'a str> {
    let mut ids = game
        .repository()
        .item_counts()
        .map(|(id, _)| id)
        .filter(|id| catalog.item(id).is_some())
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids
}

/// Row of the member the player currently controls, so the switch overlay opens on them.
fn controlled_member_index(game: &GameState) -> usize {
    game.party()
        .members()
        .position(|member| member.id() == game.controlled_member_id())
        .unwrap_or_default()
}

/// Applies the switch overlay's selection and closes it.
///
/// Ports `switch_character_scene._confirm_selection`: the overlay closes on confirm whether or not
/// the selection changed, and the World sprite follows through `controlled_member_id`.
fn switch_controlled_member(game: &mut GameState, state: &mut FieldMenuState) {
    let Some(member_id) = member_id_at(game, state.selected).map(ToOwned::to_owned) else {
        return;
    };
    let name = member_at(game, state.selected)
        .map_or_else(|| member_id.clone(), |member| member.name().to_owned());
    state.mode = FieldMenuMode::Browse;
    state.selected = CHARACTER_COMMAND_INDEX;
    state.message = if game.set_controlled_member(member_id).is_ok() {
        format!("Now controlling {name}.")
    } else {
        "That party member cannot be controlled.".to_owned()
    };
}

/// Player-facing name for a battle row. Keeps `{:?}` out of the status page.
const fn row_label(row: PartyRow) -> &'static str {
    match row {
        PartyRow::Front => "Front",
        PartyRow::Back => "Back",
    }
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

/// Columns the command deck needs to hold every command at [`MAIN_COMMAND_ROWS`] per column.
const fn main_command_columns() -> usize {
    MAIN_COMMANDS.len().div_ceil(MAIN_COMMAND_ROWS)
}

/// Commands in one deck column; the last column is short when the count does not divide evenly.
const fn main_command_column_len(column: usize) -> usize {
    let remaining = MAIN_COMMANDS
        .len()
        .saturating_sub(column * MAIN_COMMAND_ROWS);
    if remaining < MAIN_COMMAND_ROWS {
        remaining
    } else {
        MAIN_COMMAND_ROWS
    }
}

/// Moves the deck cursor across the command grid, wrapping on both axes.
///
/// Commands fill each column top to bottom before the next one starts, so the index alone carries
/// the column. Crossing into a short column clamps the row rather than landing on nothing.
fn stepped_main_command(
    selected: usize,
    vertical: Option<isize>,
    horizontal: Option<isize>,
) -> usize {
    let selected = selected.min(MAIN_COMMANDS.len() - 1);
    let mut column = selected / MAIN_COMMAND_ROWS;
    let mut row = selected % MAIN_COMMAND_ROWS;
    if let Some(delta) = horizontal {
        column = wrapped(column, main_command_columns(), delta);
        row = row.min(main_command_column_len(column) - 1);
    }
    if let Some(delta) = vertical {
        row = wrapped(row, main_command_column_len(column), delta);
    }
    column * MAIN_COMMAND_ROWS + row
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
    use crate::save_data::tests::{fixture_balance, fixture_game};

    /// A new game starts with the protagonist alone; the switch overlay needs someone to switch to.
    fn fixture_game_with_recruit() -> GameState {
        let catalog: crate::scenario_party::PartyCatalog = crate::scenario_yaml::from_str(
            include_str!("../../../assets/scenarios/rusted_kingdoms/data/party.yaml"),
        )
        .unwrap();
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
                &state,
                &game,
                &PartyWalkThumbnails::default(),
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
    fn status_visual_helpers_handle_empty_names_and_portrait_paths() {
        assert_eq!(member_emblem("Aric"), "AR");
        assert_eq!(member_emblem("Aric Vale"), "AV");
        assert_eq!(member_emblem("  "), "?");
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
    fn the_tag_editor_lists_the_curatorial_set_then_custom_tags_then_the_new_prompt() {
        let mut game = fixture_game();
        game.repository_mut().add_tags("potion", ["mine"]).unwrap();

        let rows = tag_editor_rows(&game, "potion");

        assert_eq!(
            rows,
            vec![
                TagEditorRow::Tag("rare".to_owned()),
                TagEditorRow::Tag("sell_soon".to_owned()),
                TagEditorRow::Tag("favorite".to_owned()),
                TagEditorRow::Tag("mine".to_owned()),
                TagEditorRow::New,
            ]
        );
        // Type-driven catalog tags stay out of the editor: the player does not own them.
        game.repository_mut()
            .add_tags("potion", ["consumable"])
            .unwrap();
        assert!(
            !tag_editor_rows(&game, "potion").contains(&TagEditorRow::Tag("consumable".to_owned()))
        );
    }

    #[test]
    fn toggling_a_tag_adds_it_once_and_removes_it_again() {
        let mut game = fixture_game();
        let mut state = FieldMenuState::default();

        toggle_item_tag(&mut game, &mut state, "potion", "favorite");
        assert!(
            game.repository()
                .item_tags("potion")
                .any(|tag| tag == "favorite")
        );
        assert!(state.message.contains("Added"));

        toggle_item_tag(&mut game, &mut state, "potion", "favorite");
        assert!(
            !game
                .repository()
                .item_tags("potion")
                .any(|tag| tag == "favorite")
        );
        assert!(state.message.contains("Removed"));
    }

    #[test]
    fn a_custom_tag_is_normalized_and_rejected_reasons_are_distinguished() {
        assert_eq!(
            normalize_custom_tag("  Sell Later "),
            Some("sell_later".to_owned())
        );
        assert_eq!(normalize_custom_tag("KEEP"), Some("keep".to_owned()));
        assert!(normalize_custom_tag("   ").is_none());
        assert!(normalize_custom_tag("boss-drop").is_none());
        assert!(normalize_custom_tag(&"a".repeat(CUSTOM_TAG_MAX_LENGTH + 1)).is_none());

        let mut game = fixture_game();
        let mut state = FieldMenuState {
            mode: FieldMenuMode::ItemNewTag,
            text_input: "boss-drop".to_owned(),
            ..default()
        };

        commit_new_tag(&mut game, &mut state, "potion");
        assert_eq!(
            state.mode,
            FieldMenuMode::ItemNewTag,
            "an invalid tag keeps the prompt open"
        );
        assert_eq!(state.message, "Invalid tag.");

        state.text_input = "Boss Drop".to_owned();
        commit_new_tag(&mut game, &mut state, "potion");
        assert_eq!(state.mode, FieldMenuMode::ItemTags);
        assert!(state.text_input.is_empty());
        assert!(
            game.repository()
                .item_tags("potion")
                .any(|tag| tag == "boss_drop")
        );

        state.mode = FieldMenuMode::ItemNewTag;
        state.text_input = "boss_drop".to_owned();
        commit_new_tag(&mut game, &mut state, "potion");
        assert_eq!(state.message, "Tag already added.");
    }

    #[test]
    fn the_tag_draft_never_exceeds_the_cap_even_from_one_multi_scalar_event() {
        let mut draft = String::new();
        append_tag_text(&mut draft, &"x".repeat(CUSTOM_TAG_MAX_LENGTH + 8));
        assert_eq!(draft.chars().count(), CUSTOM_TAG_MAX_LENGTH);

        let mut short = "ab".to_owned();
        append_tag_text(&mut short, "\u{7}c");
        assert_eq!(short, "abc", "control scalars are dropped, not stored");
    }

    #[test]
    fn the_manage_list_keeps_hidden_items_that_the_pouch_filters_out() {
        // The fixture already hides its potion, which is exactly the state the manager exists for.
        let mut game = fixture_game();
        let catalog = crate::field_menu_domain::tests::catalog();

        assert!(game.repository().is_hidden("potion"));
        assert!(!inventory_ids(&game, &catalog, InventoryTab::All).contains(&"potion"));
        assert!(
            manage_ids(&game, &catalog).contains(&"potion"),
            "a hidden item must stay reachable so it can be shown again"
        );

        assert!(!game.repository_mut().toggle_hidden("potion"));
        assert!(inventory_ids(&game, &catalog, InventoryTab::All).contains(&"potion"));
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
    fn status_back_steps_out_of_the_position_picker_one_page_at_a_time() {
        let mut state = FieldMenuState::default();
        state.open(FieldMenuScreen::Status);
        state.status_page = StatusPage::Position;
        state.selected = 1;

        state.back();

        assert_eq!(state.status_page, StatusPage::Details);
        assert_eq!(state.selected, STATUS_POSITION_CATEGORY);

        state.back();
        assert_eq!(state.status_page, StatusPage::Roster);
    }

    #[test]
    fn confirming_a_row_moves_the_member_and_repeating_it_reports_no_change() {
        let mut game = fixture_game();
        let member_id = game
            .party()
            .members()
            .next()
            .expect("fixture party has members")
            .id()
            .to_owned();
        assert_eq!(game.party().row_of(&member_id), Some(PartyRow::Front));

        let mut state = FieldMenuState::default();
        state.open(FieldMenuScreen::Status);
        state.status_page = StatusPage::Position;
        state.selected = 1;

        set_member_row(&mut game, &mut state);
        assert_eq!(game.party().row_of(&member_id), Some(PartyRow::Back));
        assert!(state.message.contains("moved to the Back row"));

        set_member_row(&mut game, &mut state);
        assert_eq!(game.party().row_of(&member_id), Some(PartyRow::Back));
        assert!(state.message.contains("already holds the Back row"));

        state.selected = 0;
        set_member_row(&mut game, &mut state);
        assert_eq!(game.party().row_of(&member_id), Some(PartyRow::Front));
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

    #[test]
    fn confirming_the_switch_overlay_changes_control_and_returns_to_the_deck() {
        let mut game = fixture_game_with_recruit();
        assert_eq!(game.controlled_member_id(), "aric");

        let mut state = FieldMenuState::default();
        state.open(FieldMenuScreen::Main);
        state.mode = FieldMenuMode::CharacterSwitch;
        state.selected = 1;

        switch_controlled_member(&mut game, &mut state);

        assert_eq!(game.controlled_member_id(), "elise");
        assert_eq!(state.mode, FieldMenuMode::Browse);
        assert_eq!(state.selected, CHARACTER_COMMAND_INDEX);
        assert!(state.message.contains("Now controlling Elise"));
    }

    #[test]
    fn cancelling_the_switch_overlay_returns_the_cursor_to_the_character_command() {
        let mut state = FieldMenuState::default();
        state.open(FieldMenuScreen::Main);
        state.mode = FieldMenuMode::CharacterSwitch;
        state.selected = 3;

        state.back();

        assert!(state.open);
        assert_eq!(state.mode, FieldMenuMode::Browse);
        assert_eq!(state.selected, CHARACTER_COMMAND_INDEX);
    }

    #[test]
    fn the_switch_overlay_opens_on_the_member_already_being_controlled() {
        let mut game = fixture_game_with_recruit();
        game.set_controlled_member("elise").unwrap();

        assert_eq!(controlled_member_index(&game), 1);
    }

    #[test]
    fn main_commands_distinguish_enabled_disabled_and_cancel_paths() {
        assert_eq!(main_command_screen(0), Some(FieldMenuScreen::Status));
        assert_eq!(main_command_screen(1), Some(FieldMenuScreen::Spells));
        assert_eq!(main_command_screen(2), Some(FieldMenuScreen::Items));
        assert_eq!(main_command_screen(5), Some(FieldMenuScreen::Equipment));
        assert_eq!(main_command_screen(6), Some(FieldMenuScreen::Quests));
        // Character and Quit open modals rather than screens.
        assert!(main_command_screen(CHARACTER_COMMAND_INDEX).is_none());
        assert_eq!(
            main_command_screen(SAVE_COMMAND_INDEX),
            Some(FieldMenuScreen::Save)
        );
        assert!(main_command_screen(QUIT_COMMAND_INDEX).is_none());

        let mut state = FieldMenuState::default();
        state.open(FieldMenuScreen::Items);
        state.back();
        assert!(state.open);
        assert_eq!(state.screen, FieldMenuScreen::Main);
        state.back();
        assert!(!state.open);
    }

    #[test]
    fn deck_navigation_wraps_within_a_column_and_crosses_between_them() {
        // Left column holds Status, Spells, Items, Quit; right holds Save, Equipment, Quests,
        // Character.
        assert_eq!(stepped_main_command(0, Some(1), None), 1);
        assert_eq!(stepped_main_command(3, Some(1), None), 0);
        assert_eq!(stepped_main_command(0, Some(-1), None), 3);
        assert_eq!(stepped_main_command(4, Some(-1), None), 7);

        assert_eq!(stepped_main_command(1, None, Some(1)), 5);
        assert_eq!(stepped_main_command(5, None, Some(1)), 1);
        assert_eq!(stepped_main_command(5, None, Some(-1)), 1);
    }

    #[test]
    fn crossing_into_the_short_column_clamps_to_its_last_command() {
        // Adding Character filled the deck: eight commands are two whole columns, so the clamp has
        // nothing to trim today. It still guards the cursor if a command is ever added or removed.
        assert_eq!(main_command_columns(), 2);
        assert_eq!(main_command_column_len(0), 4);
        assert_eq!(main_command_column_len(1), 4);

        assert_eq!(stepped_main_command(3, None, Some(1)), 7);
        for row in 0..MAIN_COMMAND_ROWS {
            let crossed = stepped_main_command(row, None, Some(1));
            assert_eq!(
                crossed % MAIN_COMMAND_ROWS,
                row.min(main_command_column_len(1) - 1)
            );
        }
    }

    #[test]
    fn the_deck_cursor_never_leaves_the_command_list() {
        for selected in 0..MAIN_COMMANDS.len() {
            for vertical in [None, Some(-1), Some(1)] {
                for horizontal in [None, Some(-1), Some(1)] {
                    let stepped = stepped_main_command(selected, vertical, horizontal);
                    assert!(
                        stepped < MAIN_COMMANDS.len(),
                        "{selected} + {vertical:?}/{horizontal:?} left the deck at {stepped}"
                    );
                }
            }
        }
    }

    #[test]
    fn main_page_matches_the_source_command_deck_structure() {
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
                "Character"
            ]
        );

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
                .open(ServiceRequest::Inn);
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
}
