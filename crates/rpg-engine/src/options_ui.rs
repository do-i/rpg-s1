//! The options screen's Bevy wiring: layout, input routing, and persistence.
//!
//! Every rule about what an input *means* lives in [`crate::options_domain`]. This module only
//! turns raw Bevy input into calls on that state machine, redraws the result, and writes the file
//! when the machine says something changed.
//!
//! # Why raw input is read here at all
//!
//! Every other screen consults [`ActionState`], which is already resolved through the player's
//! bindings. That is exactly wrong for a rebinding screen: while capturing, the screen needs the
//! physical key, not the action it currently maps to. So input routing forks — resolved actions
//! while idle, raw device state while capturing — and the two paths are mutually exclusive so a
//! keypress can never do both.

use bevy::{
    ecs::hierarchy::ChildSpawnerCommands,
    input::{ButtonInput, gamepad::Gamepad, keyboard::KeyCode},
    prelude::*,
    ui::{ComputedNode, UiGlobalTransform, percent, px},
};

use crate::{
    action_input::{ActionState, AppAction},
    app_state::{AppState, AppStateTransitionRequest},
    gameplay_canvas::{fixed_gameplay_camera, pointer::CanvasPointer},
    input_names::{ALL_BINDABLE_GAMEPAD_BUTTONS, ALL_BINDABLE_KEYS},
    menu_chrome::{
        spawn_header_bars, spawn_section_rule, spawn_status_text, status_border_active,
        status_faint, status_gold, status_ink, status_muted, status_teal,
    },
    options_domain::{Device, OptionsEffect, OptionsRow, OptionsScreen},
    options_store::{OptionsLoadStatus, OptionsStore, PlayerOptions, load_player_options},
    scenario_inventory::ScenarioInventory,
    scenario_root::ScenarioRoot,
};

const PANEL_WIDTH: f32 = 820.0;
const ROW_HEIGHT: f32 = 26.0;
const ROW_PADDING: f32 = 10.0;
const COLUMN_GAP: f32 = 10.0;
const LABEL_WIDTH: f32 = 200.0;
const CELL_WIDTH: f32 = 270.0;
const ROW_FONT_SIZE: f32 = 15.0;
const HINT_FONT_SIZE: f32 = 13.0;

/// Which state leaving the options screen should return to.
///
/// Only the title screen opens options today, so this always holds [`AppState::Title`] in
/// practice. It is a resource rather than a constant because the opener sets it: the field menu is
/// the obvious second entry point, and a screen that hardcoded its way home would have to be
/// rewritten rather than extended when that lands.
#[derive(Resource, Clone, Copy, Debug)]
pub(crate) struct OptionsReturn(pub(crate) AppState);

impl Default for OptionsReturn {
    fn default() -> Self {
        Self(AppState::Title)
    }
}

/// Set on entry so the frame that opened the screen cannot also act inside it.
///
/// Entering costs a Confirm. `ActionState` is edge-triggered and the state change lands a frame
/// later, so in principle the edge is already spent — but this screen also reads *raw* keys while
/// capturing, where no such edge logic protects it, and a held Enter would otherwise be captured
/// the instant a rebind row opened. One dead frame is cheaper than reasoning about that every time.
#[derive(Resource, Default)]
struct OptionsEntryLatch(bool);

/// Marks the options screen as needing a write.
#[derive(Resource, Default)]
struct OptionsDirty(bool);

#[derive(Resource, Default)]
struct OptionsRecovery {
    confirming: bool,
    message: String,
}

impl OptionsRecovery {
    fn status(&self, load: &OptionsLoadStatus) -> String {
        if self.confirming {
            "Replace preserved options? Confirm backs up the original to \
             options.yaml.rejected-###.bak, then saves shipped defaults. \
             Back cancels and keeps your session changes. Click Reset To Defaults again to confirm."
                .to_owned()
        } else if !self.message.is_empty() {
            self.message.clone()
        } else {
            load.rejection_message().unwrap_or_default()
        }
    }
}

fn confirm_option(
    screen: &mut OptionsScreen,
    options: &mut PlayerOptions,
    load: &OptionsLoadStatus,
    recovery: &mut OptionsRecovery,
) -> OptionsEffect {
    if matches!(load, OptionsLoadStatus::Rejected(_))
        && screen.rows()[screen.selected()] == OptionsRow::ResetDefaults
    {
        recovery.confirming = true;
        recovery.message.clear();
        OptionsEffect::None
    } else {
        screen.confirm(options)
    }
}

#[derive(Component)]
struct OptionsScreenEntity;

#[derive(Component)]
struct OptionsRowNode(usize);

/// A clickable binding cell, so a mouse can aim at one device column.
#[derive(Component, Clone, Copy)]
struct OptionsCellNode {
    row: usize,
    device: Device,
}

#[derive(Component)]
struct OptionsLabelText(usize);

#[derive(Component)]
struct OptionsCellText {
    row: usize,
    device: Device,
}

#[derive(Component)]
struct OptionsStatusText;

#[derive(Component)]
struct OptionsHintText;

pub(crate) struct OptionsScreenPlugin;

impl Plugin for OptionsScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OptionsScreen>()
            .init_resource::<OptionsReturn>()
            .init_resource::<OptionsEntryLatch>()
            .init_resource::<OptionsDirty>()
            .init_resource::<OptionsLoadStatus>()
            .init_resource::<OptionsRecovery>()
            .init_resource::<OptionsStore>()
            .add_systems(Startup, load_options_at_startup)
            .add_systems(OnEnter(AppState::Options), setup_options_screen)
            .add_systems(OnExit(AppState::Options), cleanup_options_screen)
            .add_systems(
                Update,
                (handle_options_input, persist_options, refresh_options_text)
                    .chain()
                    .run_if(in_state(AppState::Options)),
            );
    }
}

/// Reads the player's saved options before the first frame is drawn.
///
/// Bindings decide what the title screen's very first keypress does, so they cannot arrive as an
/// asset a few frames in.
fn load_options_at_startup(
    store: Res<OptionsStore>,
    mut options: ResMut<PlayerOptions>,
    mut load: ResMut<OptionsLoadStatus>,
) {
    let (loaded, status) = load_player_options(&store);
    *options = loaded;
    *load = status;
}

#[expect(
    clippy::too_many_arguments,
    reason = "screen entry initializes recovery and input latch alongside layout resources"
)]
fn setup_options_screen(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
    screen: Res<OptionsScreen>,
    options: Res<PlayerOptions>,
    mut latch: ResMut<OptionsEntryLatch>,
    mut recovery: ResMut<OptionsRecovery>,
) {
    latch.0 = true;
    recovery.confirming = false;
    commands.spawn((fixed_gameplay_camera(), OptionsScreenEntity));

    // The same face the load picker and field menu draw with; Bevy's built-in monospace is only
    // the fallback for a scenario that ships no font.
    let font = inventory
        .font
        .as_ref()
        .map(|path| asset_server.load(root.resolve(path)))
        .unwrap_or_default();

    commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(px(28)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.02, 0.08, 0.96)),
            OptionsScreenEntity,
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: px(PANEL_WIDTH),
                    max_height: percent(100),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4),
                    padding: UiRect::all(px(16)),
                    border: UiRect::all(px(2)),
                    border_radius: BorderRadius::all(px(8)),
                    ..default()
                },
                BackgroundColor(Color::srgba_u8(22, 22, 28, 240)),
                BorderColor::all(status_border_active()),
            ))
            .with_children(|panel| {
                spawn_options_header(panel, &font);
                spawn_section_rule(panel);
                spawn_column_headings(panel, &font);

                let mut previous_was_binding = true;
                for (index, row) in screen.rows().iter().copied().enumerate() {
                    if previous_was_binding && !row.is_binding() {
                        spawn_section_rule(panel);
                    }
                    previous_was_binding = row.is_binding();
                    spawn_options_row(panel, &font, index, row.label(), |device| {
                        screen.binding_text(row, device, &options)
                    });
                }

                spawn_section_rule(panel);
                panel.spawn((
                    Text::new(""),
                    text_font(&font, HINT_FONT_SIZE),
                    TextColor(status_muted()),
                    OptionsHintText,
                ));
                panel.spawn((
                    Text::new(""),
                    text_font(&font, HINT_FONT_SIZE),
                    TextColor(status_teal()),
                    OptionsStatusText,
                ));
            });
        });
}

fn spawn_options_header(parent: &mut ChildSpawnerCommands<'_>, font: &Handle<Font>) {
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
                    ..default()
                })
                .with_children(|title| {
                    spawn_status_text(title, "OPTIONS", font, 30.0, status_gold());
                    spawn_status_text(
                        title,
                        "CONTROLS AND PREFERENCES",
                        font,
                        13.0,
                        status_muted(),
                    );
                });
        });
}

fn spawn_column_headings(parent: &mut ChildSpawnerCommands<'_>, font: &Handle<Font>) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: px(COLUMN_GAP),
            padding: UiRect::horizontal(px(ROW_PADDING)),
            ..default()
        })
        .with_children(|header| {
            let headings = [
                ("Action", LABEL_WIDTH),
                (Device::Keyboard.label(), CELL_WIDTH),
                (Device::Gamepad.label(), CELL_WIDTH),
            ];
            for (heading, width) in headings {
                header
                    .spawn(Node {
                        width: px(width),
                        ..default()
                    })
                    .with_children(|cell| {
                        spawn_status_text(cell, heading.to_uppercase(), font, 12.0, status_faint());
                    });
            }
        });
}

fn spawn_options_row(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    index: usize,
    label: &str,
    binding_text: impl Fn(Device) -> String,
) {
    parent
        .spawn((
            Node {
                min_height: px(ROW_HEIGHT),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(COLUMN_GAP),
                padding: UiRect::horizontal(px(ROW_PADDING)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            BorderColor::all(Color::NONE),
            OptionsRowNode(index),
        ))
        .with_children(|line| {
            line.spawn(Node {
                width: px(LABEL_WIDTH),
                ..default()
            })
            .with_child((
                Text::new(label),
                text_font(font, ROW_FONT_SIZE),
                TextColor(status_ink()),
                OptionsLabelText(index),
            ));
            for device in [Device::Keyboard, Device::Gamepad] {
                line.spawn((
                    Node {
                        width: px(CELL_WIDTH),
                        ..default()
                    },
                    OptionsCellNode { row: index, device },
                ))
                .with_child((
                    Text::new(binding_text(device)),
                    text_font(font, ROW_FONT_SIZE),
                    TextColor(status_muted()),
                    OptionsCellText { row: index, device },
                ));
            }
        });
}

fn text_font(font: &Handle<Font>, size: f32) -> TextFont {
    TextFont {
        font: font.clone().into(),
        font_size: FontSize::Px(size),
        ..default()
    }
}

fn cleanup_options_screen(
    mut commands: Commands,
    entities: Query<Entity, With<OptionsScreenEntity>>,
) {
    for entity in &entities {
        commands.entity(entity).despawn();
    }
}

/// Routes one frame of input into the state machine.
#[expect(
    clippy::too_many_arguments,
    reason = "a rebinding screen must see resolved actions, raw keys, raw buttons, and the mouse"
)]
fn handle_options_input(
    actions: Res<ActionState>,
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    pointer: Res<CanvasPointer>,
    rows: Query<(&OptionsRowNode, &ComputedNode, &UiGlobalTransform)>,
    cells: Query<(&OptionsCellNode, &ComputedNode, &UiGlobalTransform)>,
    mut screen: ResMut<OptionsScreen>,
    mut options: ResMut<PlayerOptions>,
    mut latch: ResMut<OptionsEntryLatch>,
    mut dirty: ResMut<OptionsDirty>,
    store: Option<Res<OptionsStore>>,
    mut load: ResMut<OptionsLoadStatus>,
    mut recovery: ResMut<OptionsRecovery>,
    mut transitions: MessageWriter<AppStateTransitionRequest>,
    return_to: Res<OptionsReturn>,
) {
    if latch.0 {
        latch.0 = false;
        return;
    }

    if recovery.confirming {
        let clicked_reset = pointer.just_clicked()
            && rows.iter().any(|(row, node, transform)| {
                screen.rows()[row.0] == OptionsRow::ResetDefaults
                    && pointer.is_over(node, transform)
            });
        if actions.just_pressed(AppAction::Back) || keys.just_pressed(KeyCode::Escape) {
            recovery.confirming = false;
        } else if actions.just_pressed(AppAction::Confirm) || clicked_reset {
            recovery.confirming = false;
            if let Some(store) = store {
                let defaults = PlayerOptions::default();
                match store.replace_rejected(&defaults) {
                    Ok(backup) => {
                        *options = defaults;
                        *load = OptionsLoadStatus::Loaded;
                        dirty.0 = false;
                        recovery.message = format!(
                            "Original options backed up to {}. Shipped defaults saved.",
                            backup.display()
                        );
                    }
                    Err(error) => {
                        recovery.message = format!(
                            "Could not replace options: {error}. Session choices remain active; \
                             the original or its verified backup is preserved. Choose Reset To Defaults to try again."
                        );
                    }
                }
            }
        }
        return;
    }

    let mut effect = OptionsEffect::None;

    if screen.is_capturing() {
        // Raw device state only. Resolved actions are deliberately not consulted, so binding Up
        // cannot also scroll the list.
        if let Some(key) = ALL_BINDABLE_KEYS
            .iter()
            .copied()
            .find(|key| keys.just_pressed(*key))
        {
            effect = screen.capture_key(key, &mut options);
        } else if keys.just_pressed(KeyCode::Escape) {
            effect = screen.back();
        } else if let Some(button) = ALL_BINDABLE_GAMEPAD_BUTTONS
            .iter()
            .copied()
            .find(|button| gamepads.iter().any(|gamepad| gamepad.just_pressed(*button)))
        {
            effect = screen.capture_button(button, &mut options);
        }
    } else {
        // Mouse hover moves the highlight, so a click always lands where the player is looking.
        for (row, node, transform) in &rows {
            if pointer.is_over(node, transform) {
                screen.select(row.0);
            }
        }

        if pointer.just_clicked() {
            let clicked_cell = cells
                .iter()
                .find(|(_, node, transform)| pointer.is_over(node, transform))
                .map(|(cell, _, _)| *cell);
            if let Some(cell) = clicked_cell {
                screen.select(cell.row);
                screen.set_device(cell.device);
                effect = confirm_option(&mut screen, &mut options, &load, &mut recovery);
            } else if rows
                .iter()
                .any(|(_, node, transform)| pointer.is_over(node, transform))
            {
                effect = confirm_option(&mut screen, &mut options, &load, &mut recovery);
            }
        } else if actions.just_pressed(AppAction::Confirm) {
            effect = confirm_option(&mut screen, &mut options, &load, &mut recovery);
        } else if actions.just_pressed(AppAction::Back) {
            effect = screen.back();
        } else if keys.just_pressed(KeyCode::Delete) || keys.just_pressed(KeyCode::Backspace) {
            effect = screen.unbind(&mut options);
        } else if let Some(delta) = actions.menu_navigation() {
            screen.navigate(delta);
        } else if let Some(delta) = actions.menu_navigation_horizontal() {
            effect = screen.horizontal(delta, &mut options);
        }
    }

    match effect {
        OptionsEffect::None => {}
        OptionsEffect::Persist => dirty.0 = true,
        OptionsEffect::Leave => {
            transitions.write(AppStateTransitionRequest::new(return_to.0));
        }
    }
}

/// Every text role the screen redraws, gathered into one query.
type OptionsTextQuery<'world, 'state> = Query<
    'world,
    'state,
    (
        &'static mut Text,
        &'static mut TextColor,
        Option<&'static OptionsLabelText>,
        Option<&'static OptionsCellText>,
        Has<OptionsStatusText>,
        Has<OptionsHintText>,
    ),
>;

/// Redraws every piece of text the screen owns.
///
/// One query rather than four. Splitting by marker would need each to exclude all the others so
/// Bevy can prove the `&mut` borrows are disjoint, and that chain of `Without` filters grows
/// quadratically with the number of text roles; matching on the optional markers instead keeps the
/// disjointness obvious without stating it.
fn refresh_options_text(
    screen: Res<OptionsScreen>,
    options: Res<PlayerOptions>,
    load: Res<OptionsLoadStatus>,
    recovery: Res<OptionsRecovery>,
    mut rows: Query<(&OptionsRowNode, &mut BackgroundColor, &mut BorderColor)>,
    mut texts: OptionsTextQuery,
) {
    // The load picker's selected-slot plate, so the cursor reads the same on every menu.
    for (row, mut background, mut border) in &mut rows {
        let selected = row.0 == screen.selected();
        background.0 = if selected {
            Color::srgba_u8(72, 49, 25, 224)
        } else {
            Color::NONE
        };
        *border = BorderColor::all(if selected {
            status_border_active()
        } else {
            Color::NONE
        });
    }
    for (mut text, mut color, label, cell, is_status, is_hint) in &mut texts {
        if let Some(label) = label {
            color.0 = if label.0 == screen.selected() {
                status_gold()
            } else {
                status_ink()
            };
        } else if let Some(cell) = cell {
            let row = screen.rows()[cell.row];
            text.0 = screen.binding_text(row, cell.device, &options);
            let selected_cell = cell.row == screen.selected()
                && (!row.is_binding() || cell.device == screen.device());
            color.0 = if selected_cell {
                status_gold()
            } else {
                status_muted()
            };
        } else if is_status {
            let persistence = recovery.status(&load);
            text.0 = if persistence.is_empty() {
                screen.status().to_owned()
            } else {
                format!("{}\n{persistence}", screen.status())
            };
        } else if is_hint {
            text.0 = screen.hint().to_owned();
        }
    }
}

/// Writes the options file whenever the state machine accepted a change.
///
/// The write happens on the spot rather than when leaving the screen, so a rebind survives a crash
/// or a hard quit that never runs an exit handler. A failed write is reported and the flag is
/// cleared: retrying every frame would bury the log without ever succeeding.
fn persist_options(
    store: Res<OptionsStore>,
    options: Res<PlayerOptions>,
    mut dirty: ResMut<OptionsDirty>,
    mut load: ResMut<OptionsLoadStatus>,
    mut recovery: ResMut<OptionsRecovery>,
) {
    if !dirty.0 {
        return;
    }
    dirty.0 = false;
    if let Err(error) = store.persist(&options, &load) {
        if !matches!(*load, OptionsLoadStatus::Rejected(_))
            && let (_, rejected @ OptionsLoadStatus::Rejected(_)) = load_player_options(&store)
        {
            *load = rejected;
            recovery.message.clear();
        }
        error!(
            "could not save options to {}: {error}",
            store.path().display()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input_bindings::BindingTarget;
    use bevy::math::Affine2;

    /// Places a row and its two cells where layout would have put them.
    ///
    /// `UiGlobalTransform` carries the node's center and `ComputedNode` its size, which is exactly
    /// what `contains_point` inverts, so a hand-built pair hit-tests identically to a laid-out one.
    fn spawn_row(app: &mut App, index: usize, center_y: f32) {
        for (device, center_x) in [(Device::Keyboard, 300.0), (Device::Gamepad, 600.0)] {
            app.world_mut().spawn((
                OptionsCellNode { row: index, device },
                ComputedNode {
                    size: Vec2::new(200.0, 20.0),
                    ..default()
                },
                UiGlobalTransform::from(Affine2::from_translation(Vec2::new(center_x, center_y))),
            ));
        }
        app.world_mut().spawn((
            OptionsRowNode(index),
            ComputedNode {
                size: Vec2::new(700.0, 20.0),
                ..default()
            },
            UiGlobalTransform::from(Affine2::from_translation(Vec2::new(400.0, center_y))),
        ));
    }

    /// A headless app with the screen's resources but no rendering, for input-routing tests.
    fn options_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ActionState>()
            .init_resource::<CanvasPointer>()
            .init_resource::<OptionsScreen>()
            .init_resource::<PlayerOptions>()
            .init_resource::<OptionsEntryLatch>()
            .init_resource::<OptionsDirty>()
            .init_resource::<OptionsLoadStatus>()
            .init_resource::<OptionsRecovery>()
            .init_resource::<OptionsReturn>()
            .add_message::<AppStateTransitionRequest>()
            .add_systems(Update, handle_options_input);
        app
    }

    fn press(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
    }

    #[test]
    fn rejected_future_options_survive_a_real_rebind_and_persistence() {
        let root = std::env::temp_dir().join(format!("rpg-s1-rf04-rebind-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let store = OptionsStore::new(root.clone());
        let original = b"# newer player settings\nversion: 99\n";
        std::fs::write(store.path(), original).unwrap();
        let mut app = options_app();
        app.insert_resource(store.clone())
            .add_systems(Startup, load_options_at_startup)
            .add_systems(Update, persist_options.after(handle_options_input));
        app.update();
        {
            let mut screen = app.world_mut().resource_mut::<OptionsScreen>();
            screen.navigate(6);
            screen.confirm(&mut PlayerOptions::default());
        }
        press(&mut app, KeyCode::KeyG);
        assert!(
            app.world()
                .resource::<PlayerOptions>()
                .bindings
                .keys(BindingTarget::Menu(AppAction::Travel))
                .contains(&KeyCode::KeyG)
        );
        assert_eq!(std::fs::read(store.path()).unwrap(), original);
        std::fs::remove_dir_all(root).unwrap();
    }

    fn rejected_reset_app(label: &str) -> (App, OptionsStore, Vec<u8>) {
        let root = std::env::temp_dir().join(format!("rpg-s1-rf04-{label}-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let store = OptionsStore::new(root);
        let original = b"# newer schema\nversion: 99\nnew_option: true\n".to_vec();
        std::fs::write(store.path(), &original).unwrap();
        let mut app = options_app();
        app.insert_resource(store.clone())
            .add_systems(Startup, load_options_at_startup)
            .add_systems(
                Update,
                (persist_options, refresh_options_text)
                    .chain()
                    .after(handle_options_input),
            );
        app.world_mut()
            .spawn((Text::new(""), TextColor::default(), OptionsStatusText));
        app.update();
        let reset = app
            .world()
            .resource::<OptionsScreen>()
            .rows()
            .iter()
            .position(|row| *row == OptionsRow::ResetDefaults)
            .unwrap();
        app.world_mut()
            .resource_mut::<OptionsScreen>()
            .select(reset);
        (app, store, original)
    }

    fn action(app: &mut App, value: crate::input_record::NormalizedAction) {
        app.world_mut()
            .resource_mut::<ActionState>()
            .replace_with_normalized(&[value]);
        app.update();
        app.world_mut()
            .resource_mut::<ActionState>()
            .replace_with_normalized(&[]);
        app.update();
    }

    fn visible_status(app: &mut App) -> String {
        app.world_mut()
            .query_filtered::<&Text, With<OptionsStatusText>>()
            .single(app.world())
            .unwrap()
            .0
            .clone()
    }

    #[test]
    fn rejected_reset_requires_confirmation_and_preserves_an_exact_backup() {
        let (mut app, store, original) = rejected_reset_app("reset-confirm");
        assert!(visible_status(&mut app).contains("Options file preserved"));
        app.world_mut().resource_mut::<PlayerOptions>().text_speed =
            Some(crate::engine_config::TextSpeed::Slow);
        action(&mut app, crate::input_record::NormalizedAction::Confirm);
        let recovery = app.world().resource::<OptionsRecovery>();
        assert!(recovery.confirming);
        assert!(
            recovery
                .status(app.world().resource::<OptionsLoadStatus>())
                .contains("backs up the original")
        );
        assert!(visible_status(&mut app).contains("Back cancels"));
        assert_eq!(std::fs::read(store.path()).unwrap(), original);
        assert_eq!(
            std::fs::read_dir(store.path().parent().unwrap())
                .unwrap()
                .count(),
            1
        );
        action(&mut app, crate::input_record::NormalizedAction::Back);
        assert!(!app.world().resource::<OptionsRecovery>().confirming);
        assert_eq!(
            app.world().resource::<PlayerOptions>().text_speed,
            Some(crate::engine_config::TextSpeed::Slow)
        );
        action(&mut app, crate::input_record::NormalizedAction::Confirm);
        action(&mut app, crate::input_record::NormalizedAction::Confirm);
        assert_eq!(
            *app.world().resource::<OptionsLoadStatus>(),
            OptionsLoadStatus::Loaded
        );
        assert_eq!(
            *app.world().resource::<PlayerOptions>(),
            PlayerOptions::default()
        );
        assert_eq!(
            std::fs::read(store.path().with_file_name("options.yaml.rejected-001.bak")).unwrap(),
            original
        );
        assert!(
            app.world()
                .resource::<OptionsRecovery>()
                .message
                .contains("Shipped defaults saved")
        );
        // Persistence becomes available after explicit recovery.
        app.world_mut()
            .resource_mut::<PlayerOptions>()
            .auto_enemy_turns = true;
        app.world_mut().resource_mut::<OptionsDirty>().0 = true;
        app.update();
        assert!(store.load().unwrap().0.auto_enemy_turns);
        std::fs::remove_dir_all(store.path().parent().unwrap()).unwrap();
    }

    #[test]
    fn failed_explicit_recovery_keeps_session_choices_and_shows_the_reason() {
        let (mut app, store, original) = rejected_reset_app("reset-failure");
        for suffix in 1..=999 {
            std::fs::write(
                store
                    .path()
                    .with_file_name(format!("options.yaml.rejected-{suffix:03}.bak")),
                b"occupied",
            )
            .unwrap();
        }
        app.world_mut()
            .resource_mut::<PlayerOptions>()
            .auto_enemy_turns = true;
        action(&mut app, crate::input_record::NormalizedAction::Confirm);
        action(&mut app, crate::input_record::NormalizedAction::Confirm);
        assert!(app.world().resource::<PlayerOptions>().auto_enemy_turns);
        assert!(matches!(
            *app.world().resource::<OptionsLoadStatus>(),
            OptionsLoadStatus::Rejected(_)
        ));
        assert_eq!(std::fs::read(store.path()).unwrap(), original);
        assert!(
            app.world()
                .resource::<OptionsRecovery>()
                .message
                .contains("999 backup names")
        );
        std::fs::remove_dir_all(store.path().parent().unwrap()).unwrap();
    }

    #[test]
    fn the_entry_latch_swallows_exactly_one_frame() {
        let mut app = options_app();
        app.world_mut().resource_mut::<OptionsEntryLatch>().0 = true;

        // The frame that opened the screen: a held Confirm must not immediately open a capture.
        app.world_mut()
            .resource_mut::<ActionState>()
            .replace_with_normalized(&[crate::input_record::NormalizedAction::Confirm]);
        app.update();
        assert!(!app.world().resource::<OptionsScreen>().is_capturing());
        assert!(!app.world().resource::<OptionsEntryLatch>().0);

        // The next frame behaves normally.
        app.update();
        assert!(
            app.world().resource::<OptionsScreen>().is_capturing(),
            "the latch must not persist past one frame"
        );
    }

    #[test]
    fn a_captured_key_is_bound_and_marks_the_options_dirty() {
        let mut app = options_app();
        {
            let mut screen = app.world_mut().resource_mut::<OptionsScreen>();
            screen.navigate(6); // Travel
            screen.confirm(&mut PlayerOptions::default());
        }
        assert!(app.world().resource::<OptionsScreen>().is_capturing());

        press(&mut app, KeyCode::KeyG);

        let options = app.world().resource::<PlayerOptions>();
        assert_eq!(
            options
                .bindings
                .keys(BindingTarget::Menu(crate::action_input::AppAction::Travel)),
            [KeyCode::KeyT, KeyCode::KeyG]
        );
        assert!(app.world().resource::<OptionsDirty>().0);
        assert!(!app.world().resource::<OptionsScreen>().is_capturing());
    }

    #[test]
    fn resolved_actions_are_ignored_while_capturing() {
        let mut app = options_app();
        {
            let mut screen = app.world_mut().resource_mut::<OptionsScreen>();
            screen.navigate(6);
            screen.confirm(&mut PlayerOptions::default());
        }

        // A resolved MenuDown alongside the raw key must not also scroll the list.
        app.world_mut()
            .resource_mut::<ActionState>()
            .replace_with_normalized(&[crate::input_record::NormalizedAction::MenuDown]);
        press(&mut app, KeyCode::KeyG);

        assert_eq!(
            app.world().resource::<OptionsScreen>().selected(),
            6,
            "the list must not move during a capture"
        );
    }

    #[test]
    fn back_leaves_to_the_recorded_origin_state() {
        let mut app = options_app();
        app.world_mut()
            .insert_resource(OptionsReturn(AppState::FieldMenu));
        app.update();

        app.world_mut()
            .resource_mut::<ActionState>()
            .replace_with_normalized(&[crate::input_record::NormalizedAction::Back]);
        app.update();

        let messages = app
            .world()
            .resource::<Messages<AppStateTransitionRequest>>();
        let mut cursor = messages.get_cursor();
        let sent: Vec<_> = cursor.read(messages).collect();
        assert_eq!(sent.len(), 1, "exactly one transition");
    }

    #[test]
    fn delete_unbinds_without_needing_a_capture() {
        let mut app = options_app();
        app.update();
        app.world_mut().resource_mut::<OptionsScreen>().navigate(1); // Confirm row

        press(&mut app, KeyCode::Delete);

        assert_eq!(
            app.world()
                .resource::<PlayerOptions>()
                .bindings
                .keys(BindingTarget::Menu(crate::action_input::AppAction::Confirm)),
            [KeyCode::Enter, KeyCode::Space]
        );
        assert!(app.world().resource::<OptionsDirty>().0);
    }

    #[test]
    fn hovering_a_row_moves_the_highlight_without_activating_it() {
        let mut app = options_app();
        spawn_row(&mut app, 3, 100.0);
        app.update();

        app.world_mut()
            .insert_resource(CanvasPointer::at_viewport_position(
                Vec2::new(400.0, 100.0),
                false,
            ));
        app.update();

        let screen = app.world().resource::<OptionsScreen>();
        assert_eq!(screen.selected(), 3, "the mouse moved the highlight");
        assert!(!screen.is_capturing(), "hovering must not start a rebind");
    }

    #[test]
    fn clicking_a_device_cell_selects_that_column_and_starts_a_rebind() {
        let mut app = options_app();
        spawn_row(&mut app, 3, 100.0);
        app.update();

        // 600 is the gamepad cell's center; the keyboard cell is at 300.
        app.world_mut()
            .insert_resource(CanvasPointer::at_viewport_position(
                Vec2::new(600.0, 100.0),
                true,
            ));
        app.update();

        let screen = app.world().resource::<OptionsScreen>();
        assert_eq!(screen.selected(), 3);
        assert_eq!(
            screen.device(),
            Device::Gamepad,
            "clicking a column picks that device"
        );
        assert!(screen.is_capturing());
    }

    #[test]
    fn a_click_outside_every_row_does_nothing() {
        let mut app = options_app();
        spawn_row(&mut app, 3, 100.0);
        app.update();

        // Well below the only row. A click that reaches the screen but lands on nothing must not
        // activate the nearest row.
        app.world_mut()
            .insert_resource(CanvasPointer::at_viewport_position(
                Vec2::new(400.0, 900.0),
                true,
            ));
        app.update();

        let screen = app.world().resource::<OptionsScreen>();
        assert_eq!(screen.selected(), 0, "the highlight never moved");
        assert!(!screen.is_capturing());
    }

    #[test]
    fn a_pointer_in_a_letterbox_bar_reaches_no_row_at_all() {
        let mut app = options_app();
        spawn_row(&mut app, 3, 100.0);
        app.update();

        // The default pointer is the off-canvas state the mapping produces for bar space: no
        // position at all, so no row can be hovered however the bars happen to line up.
        app.world_mut().insert_resource(CanvasPointer::default());
        app.update();

        assert_eq!(app.world().resource::<OptionsScreen>().selected(), 0);
    }
}
