//! In-world field-menu shell over the shared M6 runtime domain.

use bevy::prelude::*;

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

pub(crate) struct FieldMenuPlugin;

impl Plugin for FieldMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FieldMenuState>()
            .add_systems(OnEnter(AppState::World), reset_field_menu)
            .add_systems(
                Update,
                (handle_field_menu_input, sync_field_menu_overlay)
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
}

#[derive(Debug, Default, Resource)]
pub(crate) struct FieldMenuState {
    open: bool,
    screen: FieldMenuScreen,
    mode: FieldMenuMode,
    selected: usize,
    member_index: usize,
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

fn reset_field_menu(mut state: ResMut<FieldMenuState>) {
    state.close();
}

fn handle_field_menu_input(
    keys: Res<ButtonInput<KeyCode>>,
    actions: Res<ActionState>,
    catalog: Res<FieldMenuCatalog>,
    interaction: Res<WorldInteractionState>,
    mut transition: ResMut<WorldTransition>,
    game: Option<ResMut<GameState>>,
    mut state: ResMut<FieldMenuState>,
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
    state.message.clear();

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
                if let Some(screen) = main_command_screen(state.selected) {
                    state.screen = screen;
                    state.selected = 0;
                } else {
                    state.message = disabled_main_command_message(state.selected)
                        .expect("every main command is enabled or visibly disabled")
                        .to_owned();
                }
            }
        }
        (FieldMenuScreen::Status, FieldMenuMode::Browse) => {
            if let Some(delta) = horizontal.or(vertical) {
                cycle_member(&mut state, game.party().len(), delta);
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
    ]
    .get(index)
    .copied()
}

fn disabled_main_command_message(index: usize) -> Option<&'static str> {
    match index {
        4 => Some("Save becomes available in Milestone 7."),
        5 => Some("Return-to-title confirmation becomes available with saves."),
        _ => None,
    }
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
        body.0 = render_body(&state, &game, &catalog);
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
            ));
        });
}

fn screen_title(state: &FieldMenuState) -> &'static str {
    match state.screen {
        FieldMenuScreen::Main => "Field Menu",
        FieldMenuScreen::Status => "Status",
        FieldMenuScreen::Items => "Items",
        FieldMenuScreen::Equipment => "Equipment",
        FieldMenuScreen::Spells => "Spells",
    }
}

fn render_body(state: &FieldMenuState, game: &GameState, catalog: &FieldMenuCatalog) -> String {
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
            let disabled = if index >= 4 { " [disabled]" } else { "" };
            format!("{cursor} {label}{disabled}")
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

fn render_hint(state: &FieldMenuState) -> String {
    match (state.screen, state.mode) {
        (FieldMenuScreen::Main, _) => "UP/DOWN choose  ENTER open  M/ESC close",
        (FieldMenuScreen::Status, _) => "LEFT/RIGHT or UP/DOWN member  ESC back  M close",
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
    fn main_commands_distinguish_enabled_disabled_and_cancel_paths() {
        assert_eq!(main_command_screen(0), Some(FieldMenuScreen::Status));
        assert_eq!(main_command_screen(3), Some(FieldMenuScreen::Spells));
        assert!(main_command_screen(4).is_none());
        assert!(
            disabled_main_command_message(4)
                .unwrap()
                .contains("Milestone 7")
        );
        assert!(
            disabled_main_command_message(5)
                .unwrap()
                .contains("confirmation")
        );

        let mut state = FieldMenuState::default();
        state.open(FieldMenuScreen::Items);
        state.back();
        assert!(state.open);
        assert_eq!(state.screen, FieldMenuScreen::Main);
        state.back();
        assert!(!state.open);
    }
}
