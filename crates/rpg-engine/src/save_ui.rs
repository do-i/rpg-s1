//! Shared native slot catalog and title-screen load picker.

use bevy::{
    ecs::{hierarchy::ChildSpawnerCommands, system::SystemParam},
    prelude::*,
};

use crate::{
    action_input::{ActionState, AppAction},
    app_state::{AppState, AppStateTransitionRequest},
    gameplay_rng::GameplayRng,
    menu_chrome::{
        location_display_name, spawn_header_bars, spawn_section_rule, spawn_status_text,
        status_border_active, status_ember, status_faint, status_gold, status_ink, status_muted,
        status_teal, status_violet,
    },
    playtime::Playtime,
    save_store::{SAVE_SLOT_COUNT, SaveSlot, SaveSlotState, SaveStore},
    scenario_balance::BalanceData,
    scenario_dialogue::CutsceneDialogue,
    scenario_inventory::ScenarioInventory,
    scenario_manifest::Manifest,
    scenario_manifest_asset::ActiveManifestLoad,
    scenario_new_game_assets::{ActiveNewGameInputs, ActiveNewGameInputsStatus},
    scenario_party::PartyCatalog,
    scenario_root::ScenarioRoot,
};

/// Slot rows drawn per load-picker page.
const LOAD_VISIBLE_ROWS: usize = 7;

pub(crate) struct SaveUiPlugin;

impl Plugin for SaveUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SaveStore>()
            .init_resource::<SaveSlotCatalog>()
            .init_resource::<TitleLoadMenu>()
            .add_systems(OnEnter(AppState::Title), reset_title_save_ui)
            .add_systems(
                Update,
                (
                    refresh_save_slots,
                    handle_title_load_input,
                    sync_title_load_overlay,
                )
                    .chain()
                    .run_if(in_state(AppState::Title)),
            )
            .add_systems(OnExit(AppState::Title), cleanup_title_load_overlay)
            .add_systems(Update, refresh_save_slots.run_if(in_state(AppState::World)));
    }
}

/// Requests that the title screen immediately open its native load picker.
#[derive(Resource)]
pub(crate) struct OpenTitleLoadPicker;

#[derive(Debug, Default, Resource)]
pub(crate) struct SaveSlotCatalog {
    slots: Vec<SaveSlot>,
    scenario_id: String,
    scenario_version: String,
    balance: Option<BalanceData>,
    ready: bool,
    refresh_requested: bool,
    failure: Option<String>,
}

impl SaveSlotCatalog {
    pub(crate) fn has_valid(&self) -> bool {
        self.slots.iter().any(SaveSlot::is_valid)
    }

    pub(crate) fn slots(&self) -> &[SaveSlot] {
        &self.slots
    }

    pub(crate) fn request_refresh(&mut self) {
        self.refresh_requested = true;
    }

    #[cfg(test)]
    pub(crate) fn ready_for_test(
        scenario_id: impl Into<String>,
        scenario_version: impl Into<String>,
        balance: BalanceData,
    ) -> Self {
        Self {
            scenario_id: scenario_id.into(),
            scenario_version: scenario_version.into(),
            balance: Some(balance),
            ready: true,
            ..Self::default()
        }
    }

    pub(crate) fn context(&self) -> Option<SaveContext<'_>> {
        Some(SaveContext {
            scenario_id: &self.scenario_id,
            scenario_version: &self.scenario_version,
            balance: self.balance.as_ref()?,
        })
    }
}

#[derive(Clone, Copy)]
pub(crate) struct SaveContext<'a> {
    pub(crate) scenario_id: &'a str,
    pub(crate) scenario_version: &'a str,
    pub(crate) balance: &'a BalanceData,
}

#[derive(Debug, Default, Resource)]
pub(crate) struct TitleLoadMenu {
    pub(crate) open: bool,
    just_opened: bool,
    selected: usize,
    message: String,
}

impl TitleLoadMenu {
    pub(crate) fn open(&mut self, slots: &[SaveSlot]) {
        let latest = latest_valid_slot(slots).unwrap_or(0);
        *self = Self {
            open: true,
            just_opened: true,
            selected: latest,
            message: String::new(),
        };
    }
}

#[derive(Component)]
struct TitleLoadRoot;

#[derive(Component)]
struct TitleLoadSlotRow;

#[derive(Component)]
struct SelectedTitleLoadSlotRow;

#[derive(SystemParam)]
struct SaveInputs<'w> {
    active: Res<'w, ActiveNewGameInputs>,
    manifest_load: Res<'w, ActiveManifestLoad>,
    manifests: Res<'w, Assets<Manifest>>,
    parties: Res<'w, Assets<PartyCatalog>>,
    balances: Res<'w, Assets<BalanceData>>,
    intros: Res<'w, Assets<CutsceneDialogue>>,
}

impl<'w> SaveInputs<'w> {
    fn current(&self) -> Option<(&Manifest, &BalanceData)> {
        self.active
            .inputs(
                &self.manifest_load,
                &self.manifests,
                &self.parties,
                &self.balances,
                &self.intros,
            )
            .map(|inputs| (inputs.manifest, inputs.balance))
    }
}

fn reset_title_save_ui(
    mut commands: Commands,
    mut catalog: ResMut<SaveSlotCatalog>,
    mut load_menu: ResMut<TitleLoadMenu>,
    open_picker: Option<Res<OpenTitleLoadPicker>>,
) {
    catalog.request_refresh();
    *load_menu = TitleLoadMenu::default();
    if open_picker.is_some() {
        load_menu.open(catalog.slots());
        commands.remove_resource::<OpenTitleLoadPicker>();
    }
}

fn refresh_save_slots(
    store: Res<SaveStore>,
    inputs: SaveInputs,
    mut catalog: ResMut<SaveSlotCatalog>,
) {
    if !catalog.refresh_requested {
        return;
    }
    let Some((manifest, balance)) = inputs.current() else {
        if inputs.active.status() == ActiveNewGameInputsStatus::Failed {
            catalog.ready = false;
            catalog.failure = inputs.active.failure().map(ToString::to_string);
            catalog.refresh_requested = false;
        }
        return;
    };
    catalog.slots = store.enumerate(&manifest.id, &manifest.version, balance);
    catalog.scenario_id.clone_from(&manifest.id);
    catalog.scenario_version.clone_from(&manifest.version);
    catalog.balance = Some(balance.clone());
    catalog.ready = true;
    catalog.failure = None;
    catalog.refresh_requested = false;
}

#[expect(
    clippy::too_many_arguments,
    reason = "title load coordinates input, assets, storage, session installation, and state transition"
)]
fn handle_title_load_input(
    actions: Res<ActionState>,
    store: Res<SaveStore>,
    inputs: SaveInputs,
    catalog: Res<SaveSlotCatalog>,
    time: Res<Time<Real>>,
    mut load_menu: ResMut<TitleLoadMenu>,
    mut commands: Commands,
    mut transitions: MessageWriter<AppStateTransitionRequest>,
) {
    if !load_menu.open {
        return;
    }
    if load_menu.just_opened {
        load_menu.just_opened = false;
        return;
    }
    if actions.just_pressed(AppAction::Back) {
        *load_menu = TitleLoadMenu::default();
        return;
    }
    if let Some(delta) = actions.menu_navigation() {
        load_menu.selected =
            (load_menu.selected as isize + delta).clamp(0, (SAVE_SLOT_COUNT - 1) as isize) as usize;
        load_menu.message.clear();
    }
    if !actions.just_pressed(AppAction::Confirm) {
        return;
    }
    let Some(slot) = catalog.slots.get(load_menu.selected) else {
        load_menu.message = "Save slots are still loading.".to_owned();
        return;
    };
    match &slot.state {
        SaveSlotState::Empty => load_menu.message = "That slot is empty.".to_owned(),
        SaveSlotState::Corrupt(reason) | SaveSlotState::Incompatible(reason) => {
            load_menu.message = reason.clone()
        }
        SaveSlotState::Valid => {
            let Some((manifest, balance)) = inputs.current() else {
                load_menu.message = "Scenario data is still loading.".to_owned();
                return;
            };
            match store.load(slot.index, &manifest.id, &manifest.version, balance) {
                Ok((_, mut game)) => {
                    game.playtime_mut().start_session(time.elapsed());
                    commands.queue(move |world: &mut World| {
                        world.insert_resource(game);
                        world.remove_resource::<GameplayRng>();
                        world.remove_resource::<Playtime>();
                    });
                    transitions.write(AppStateTransitionRequest::new(AppState::World));
                    *load_menu = TitleLoadMenu::default();
                }
                Err(error) => load_menu.message = error.to_string(),
            }
        }
    }
}

fn sync_title_load_overlay(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
    catalog: Res<SaveSlotCatalog>,
    load_menu: Res<TitleLoadMenu>,
    roots: Query<Entity, With<TitleLoadRoot>>,
) {
    if !load_menu.open {
        for entity in &roots {
            commands.entity(entity).despawn();
        }
        return;
    }
    if !roots.is_empty() && !load_menu.is_changed() && !catalog.is_changed() {
        return;
    }
    for entity in &roots {
        commands.entity(entity).despawn();
    }

    let Some(font_path) = inventory.font.as_ref() else {
        return;
    };
    let font = asset_server.load(root.resolve(font_path));
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(px(28)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.02, 0.08, 0.96)),
            GlobalZIndex(5_000),
            Name::new("Load picker"),
            TitleLoadRoot,
        ))
        .with_children(|root| {
            spawn_load_panel(root, &font, &catalog, &load_menu);
        });
}

fn spawn_load_panel(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    catalog: &SaveSlotCatalog,
    menu: &TitleLoadMenu,
) {
    parent
        .spawn((
            Node {
                width: px(820),
                max_height: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                padding: UiRect::all(px(16)),
                border: UiRect::all(px(2)),
                border_radius: BorderRadius::all(px(8)),
                ..default()
            },
            BackgroundColor(Color::srgba_u8(22, 22, 28, 240)),
            BorderColor::all(status_border_active()),
        ))
        .with_children(|panel| {
            spawn_load_header(panel, font, catalog, menu);
            spawn_section_rule(panel);
            if catalog.ready {
                let page_start = load_page_start(menu.selected);
                let latest = latest_valid_slot(&catalog.slots);
                for slot in catalog
                    .slots
                    .iter()
                    .skip(page_start)
                    .take(LOAD_VISIBLE_ROWS)
                {
                    spawn_load_slot_row(
                        panel,
                        font,
                        slot,
                        slot.index == menu.selected,
                        Some(slot.index) == latest,
                    );
                }
            } else {
                panel
                    .spawn(Node {
                        flex_grow: 1.0,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|pending| {
                        spawn_status_text(
                            pending,
                            catalog
                                .failure
                                .as_deref()
                                .unwrap_or("Discovering native save slots..."),
                            font,
                            18.0,
                            if catalog.failure.is_some() {
                                status_ember()
                            } else {
                                status_muted()
                            },
                        );
                    });
            }
            spawn_section_rule(panel);
            spawn_status_text(
                panel,
                "UP/DOWN   SELECT SLOT      ENTER   LOAD      ESC   BACK",
                font,
                14.0,
                status_muted(),
            );
            if !menu.message.is_empty() {
                spawn_load_message(panel, font, &menu.message);
            }
        });
}

fn spawn_load_header(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    catalog: &SaveSlotCatalog,
    menu: &TitleLoadMenu,
) {
    let page = menu.selected / LOAD_VISIBLE_ROWS + 1;
    let page_count = SAVE_SLOT_COUNT.div_ceil(LOAD_VISIBLE_ROWS);
    let recorded = catalog.slots.iter().filter(|slot| slot.is_valid()).count();
    parent
        .spawn(Node {
            width: percent(100),
            min_height: px(56),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|header| {
            spawn_header_bars(header, 44.0, 14.0);
            header
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    ..default()
                })
                .with_children(|title| {
                    spawn_status_text(title, "LOAD GAME", font, 30.0, status_gold());
                    spawn_status_text(
                        title,
                        "CONTINUE A RECORDED CHRONICLE",
                        font,
                        13.0,
                        status_muted(),
                    );
                });
            header
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::End,
                    ..default()
                })
                .with_children(|counts| {
                    spawn_status_text(
                        counts,
                        if catalog.ready {
                            format!("{recorded:02} RECORDED")
                        } else {
                            "-- RECORDED".to_owned()
                        },
                        font,
                        13.0,
                        if recorded == 0 {
                            status_muted()
                        } else {
                            status_teal()
                        },
                    );
                    spawn_status_text(
                        counts,
                        format!("PAGE {page:02} / {page_count:02}"),
                        font,
                        13.0,
                        status_muted(),
                    );
                });
        });
}

fn spawn_load_slot_row(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    slot: &SaveSlot,
    selected: bool,
    latest: bool,
) {
    let mut row = parent.spawn((
        Node {
            width: percent(100),
            min_height: px(56),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            padding: UiRect::axes(px(12), px(7)),
            border: UiRect::all(px(if selected { 2 } else { 1 })),
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        BackgroundColor(if selected {
            Color::srgba_u8(72, 49, 25, 224)
        } else {
            Color::srgba_u8(10, 10, 14, 148)
        }),
        BorderColor::all(if selected {
            status_border_active()
        } else {
            Color::srgba_u8(126, 98, 55, 95)
        }),
        TitleLoadSlotRow,
    ));
    if selected {
        row.insert(SelectedTitleLoadSlotRow);
    }
    row.with_children(|row| {
        row.spawn(Node {
            width: px(106),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|label| {
            spawn_status_text(
                label,
                slot.label().to_uppercase(),
                font,
                15.0,
                if slot.is_valid() {
                    status_ink()
                } else {
                    status_muted()
                },
            );
            if latest {
                spawn_status_text(label, "LATEST", font, 10.0, status_gold());
            }
        });
        row.spawn(Node {
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|content| {
            spawn_load_slot_content(content, font, slot);
        });
        spawn_status_text(
            row,
            save_slot_state_label(slot),
            font,
            11.0,
            save_slot_state_color(slot),
        );
    });
}

fn spawn_load_slot_content(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    slot: &SaveSlot,
) {
    match (&slot.state, &slot.metadata) {
        (SaveSlotState::Empty, _) => {
            spawn_status_text(parent, "—  EMPTY  —", font, 17.0, status_faint());
        }
        (SaveSlotState::Valid, Some(metadata)) => {
            spawn_status_text(
                parent,
                format!(
                    "{}    ({})",
                    location_display_name(&metadata.location),
                    metadata.protagonist_name
                ),
                font,
                17.0,
                status_ink(),
            );
            spawn_status_text(
                parent,
                format!(
                    "LV {}      PLAYTIME {}",
                    metadata.protagonist_level,
                    Playtime::format(metadata.playtime_seconds)
                ),
                font,
                12.0,
                status_muted(),
            );
        }
        (SaveSlotState::Corrupt(reason), _) => {
            spawn_status_text(parent, "CORRUPT SAVE", font, 16.0, status_ember());
            spawn_status_text(parent, reason, font, 11.0, status_muted());
        }
        (SaveSlotState::Incompatible(reason), _) => {
            spawn_status_text(parent, "INCOMPATIBLE SAVE", font, 16.0, status_violet());
            spawn_status_text(parent, reason, font, 11.0, status_muted());
        }
        _ => {
            spawn_status_text(parent, "INVALID METADATA", font, 16.0, status_ember());
        }
    }
}

fn spawn_load_message(parent: &mut ChildSpawnerCommands<'_>, font: &Handle<Font>, message: &str) {
    parent
        .spawn((
            Node {
                width: percent(100),
                justify_content: JustifyContent::Center,
                padding: UiRect::axes(px(10), px(6)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(Color::srgba_u8(10, 10, 14, 180)),
            BorderColor::all(status_border_active()),
        ))
        .with_children(|banner| {
            spawn_status_text(banner, message, font, 13.0, status_ink());
        });
}

/// Reports the newest recorded slot so the picker can badge the row it opens on.
pub(crate) fn latest_valid_slot(slots: &[SaveSlot]) -> Option<usize> {
    slots
        .iter()
        .filter(|slot| slot.is_valid())
        .max_by_key(|slot| slot.saved_at_unix_seconds.unwrap_or(0))
        .map(|slot| slot.index)
}

pub(crate) fn load_page_start(selected: usize) -> usize {
    selected / LOAD_VISIBLE_ROWS * LOAD_VISIBLE_ROWS
}

pub(crate) fn save_slot_state_label(slot: &SaveSlot) -> &'static str {
    match slot.state {
        SaveSlotState::Empty => "OPEN",
        SaveSlotState::Valid => "SAVED",
        SaveSlotState::Corrupt(_) => "CORRUPT",
        SaveSlotState::Incompatible(_) => "VERSION",
    }
}

pub(crate) fn save_slot_state_color(slot: &SaveSlot) -> Color {
    match slot.state {
        SaveSlotState::Empty => status_muted(),
        SaveSlotState::Valid => status_teal(),
        SaveSlotState::Corrupt(_) => status_ember(),
        SaveSlotState::Incompatible(_) => status_violet(),
    }
}

fn cleanup_title_load_overlay(
    mut commands: Commands,
    roots: Query<Entity, With<TitleLoadRoot>>,
    mut load_menu: ResMut<TitleLoadMenu>,
) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    *load_menu = TitleLoadMenu::default();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_picker_selects_latest_valid_slot_and_clamps_navigation() {
        let slots = vec![
            SaveSlot {
                index: 0,
                state: SaveSlotState::Valid,
                metadata: None,
                saved_at_unix_seconds: Some(10),
            },
            SaveSlot {
                index: 1,
                state: SaveSlotState::Corrupt("bad".to_owned()),
                metadata: None,
                saved_at_unix_seconds: None,
            },
            SaveSlot {
                index: 2,
                state: SaveSlotState::Valid,
                metadata: None,
                saved_at_unix_seconds: Some(20),
            },
        ];
        let mut menu = TitleLoadMenu::default();
        menu.open(&slots);
        assert_eq!(menu.selected, 2);
        assert!(menu.open && menu.just_opened);
    }
}
