//! Shared native slot catalog and title-screen load picker.

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    action_input::{ActionState, AppAction},
    app_state::{AppState, AppStateTransitionRequest},
    gameplay_rng::GameplayRng,
    playtime::Playtime,
    save_store::{SAVE_SLOT_COUNT, SaveSlot, SaveSlotState, SaveStore},
    scenario_balance::BalanceData,
    scenario_dialogue::CutsceneDialogue,
    scenario_manifest::Manifest,
    scenario_manifest_asset::ActiveManifestLoad,
    scenario_new_game_assets::{ActiveNewGameInputs, ActiveNewGameInputsStatus},
    scenario_party::PartyCatalog,
    ui_theme::UiTheme,
};

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
        let latest = slots
            .iter()
            .filter(|slot| slot.is_valid())
            .max_by_key(|slot| slot.saved_at_unix_seconds.unwrap_or(0))
            .map_or(0, |slot| slot.index);
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
struct TitleLoadBody;

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
    theme: Res<UiTheme>,
    catalog: Res<SaveSlotCatalog>,
    load_menu: Res<TitleLoadMenu>,
    roots: Query<Entity, With<TitleLoadRoot>>,
    mut bodies: Query<&mut Text, With<TitleLoadBody>>,
) {
    if !load_menu.open {
        for entity in &roots {
            commands.entity(entity).despawn();
        }
        return;
    }
    if roots.is_empty() {
        let font = asset_server.load("fonts/Philosopher-Regular.ttf");
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(100),
                    height: percent(100),
                    padding: UiRect::all(px(32)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.02, 0.02, 0.08, 0.96)),
                GlobalZIndex(5_000),
                TitleLoadRoot,
            ))
            .with_children(|root| {
                root.spawn((
                    Text::new(""),
                    TextFont {
                        font: font.into(),
                        font_size: FontSize::Px(21.0),
                        ..default()
                    },
                    TextColor(theme.name_entry_input_color),
                    TitleLoadBody,
                ));
            });
        return;
    }
    let Ok(mut body) = bodies.single_mut() else {
        return;
    };
    body.0 = render_load_menu(&catalog, &load_menu);
}

fn render_load_menu(catalog: &SaveSlotCatalog, menu: &TitleLoadMenu) -> String {
    if !catalog.ready {
        return format!(
            "LOAD GAME\n\n{}\n\nESC Back",
            catalog
                .failure
                .as_deref()
                .unwrap_or("Discovering native save slots...")
        );
    }
    let page_start = (menu.selected / 7) * 7;
    let latest = catalog
        .slots
        .iter()
        .filter(|slot| slot.is_valid())
        .max_by_key(|slot| slot.saved_at_unix_seconds.unwrap_or(0))
        .map(|slot| slot.index);
    let rows = catalog
        .slots
        .iter()
        .skip(page_start)
        .take(7)
        .map(|slot| {
            let cursor = if slot.index == menu.selected {
                ">"
            } else {
                " "
            };
            let latest = if Some(slot.index) == latest {
                " [LATEST]"
            } else {
                ""
            };
            match (&slot.state, &slot.metadata) {
                (SaveSlotState::Empty, _) => {
                    format!("{cursor} {:<8} --- Empty ---", slot.label())
                }
                (SaveSlotState::Valid, Some(metadata)) => format!(
                    "{cursor} {:<8} {} Lv{}  {}  {}{latest}",
                    slot.label(),
                    metadata.protagonist_name,
                    metadata.protagonist_level,
                    Playtime::format(metadata.playtime_seconds),
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
    format!(
        "LOAD GAME\n\n{rows}\n\n{}\nUP/DOWN Select  ENTER Load/inspect  ESC Back",
        menu.message
    )
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
