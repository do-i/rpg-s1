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
    input::{ButtonInput, gamepad::Gamepad, keyboard::KeyCode},
    prelude::*,
    ui::{ComputedNode, UiGlobalTransform, percent, px},
};

use crate::{
    action_input::{ActionState, AppAction},
    app_state::{AppState, AppStateTransitionRequest},
    gameplay_canvas::{fixed_gameplay_camera, pointer::CanvasPointer},
    input_names::{ALL_BINDABLE_GAMEPAD_BUTTONS, ALL_BINDABLE_KEYS},
    options_domain::{Device, OptionsEffect, OptionsScreen},
    options_store::{OptionsStore, PlayerOptions, load_player_options},
    ui_theme::UiTheme,
};

const ROW_HEIGHT: f32 = 30.0;
const LABEL_WIDTH: f32 = 200.0;
const CELL_WIDTH: f32 = 250.0;

/// Where Confirm on the options row should return to.
///
/// The screen is reachable from the title and from the field menu, and it must go back to
/// whichever one opened it rather than to a fixed state.
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
            .init_resource::<OptionsStore>()
            .init_resource::<UiTheme>()
            .add_systems(Startup, load_options_at_startup)
            .add_systems(OnEnter(AppState::Options), setup_options_screen)
            .add_systems(OnExit(AppState::Options), cleanup_options_screen)
            .add_systems(
                Update,
                (handle_options_input, refresh_options_text, persist_options)
                    .chain()
                    .run_if(in_state(AppState::Options)),
            );
    }
}

/// Reads the player's saved options before the first frame is drawn.
///
/// Bindings decide what the title screen's very first keypress does, so they cannot arrive as an
/// asset a few frames in.
fn load_options_at_startup(store: Res<OptionsStore>, mut options: ResMut<PlayerOptions>) {
    *options = load_player_options(&store);
}

fn setup_options_screen(
    mut commands: Commands,
    theme: Res<UiTheme>,
    screen: Res<OptionsScreen>,
    options: Res<PlayerOptions>,
    mut latch: ResMut<OptionsEntryLatch>,
) {
    latch.0 = true;
    commands.spawn((fixed_gameplay_camera(), OptionsScreenEntity));

    commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: px(6),
                ..default()
            },
            OptionsScreenEntity,
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("Options"),
                text_font(theme.menu_font_size + 6.0),
                TextColor(theme.menu_selected_color),
            ));

            root.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: px(10),
                    padding: UiRect::axes(px(20), px(4)),
                    ..default()
                },
                BackgroundColor(theme.panel_color),
            ))
            .with_children(|header| {
                header.spawn((
                    Node {
                        width: px(LABEL_WIDTH),
                        ..default()
                    },
                    children![(
                        Text::new("Action"),
                        text_font(theme.status_font_size),
                        TextColor(theme.status_color),
                    )],
                ));
                for device in [Device::Keyboard, Device::Gamepad] {
                    header.spawn((
                        Node {
                            width: px(CELL_WIDTH),
                            ..default()
                        },
                        children![(
                            Text::new(device.label()),
                            text_font(theme.status_font_size),
                            TextColor(theme.status_color),
                        )],
                    ));
                }
            });

            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::axes(px(20), px(10)),
                    border_radius: BorderRadius::all(px(12)),
                    ..default()
                },
                BackgroundColor(theme.panel_color),
            ))
            .with_children(|panel| {
                for (index, row) in screen.rows().iter().copied().enumerate() {
                    panel
                        .spawn((
                            Node {
                                height: px(ROW_HEIGHT),
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                column_gap: px(10),
                                ..default()
                            },
                            OptionsRowNode(index),
                        ))
                        .with_children(|line| {
                            line.spawn((
                                Node {
                                    width: px(LABEL_WIDTH),
                                    ..default()
                                },
                                children![(
                                    Text::new(row.label()),
                                    text_font(theme.menu_font_size),
                                    TextColor(theme.menu_normal_color),
                                    OptionsLabelText(index),
                                )],
                            ));
                            for device in [Device::Keyboard, Device::Gamepad] {
                                line.spawn((
                                    Node {
                                        width: px(CELL_WIDTH),
                                        ..default()
                                    },
                                    OptionsCellNode { row: index, device },
                                    children![(
                                        Text::new(screen.binding_text(row, device, &options)),
                                        text_font(theme.menu_font_size),
                                        TextColor(theme.menu_normal_color),
                                        OptionsCellText { row: index, device },
                                    )],
                                ));
                            }
                        });
                }
            });

            root.spawn((
                Text::new(""),
                text_font(theme.status_font_size),
                TextColor(theme.status_color),
                OptionsStatusText,
            ));
            root.spawn((
                Text::new(""),
                text_font(theme.status_font_size),
                TextColor(theme.menu_disabled_color),
                OptionsHintText,
            ));
        });
}

fn text_font(size: f32) -> TextFont {
    TextFont {
        font: Handle::<Font>::default().into(),
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
    mut transitions: MessageWriter<AppStateTransitionRequest>,
    return_to: Res<OptionsReturn>,
) {
    if latch.0 {
        latch.0 = false;
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
                effect = screen.confirm(&mut options);
            } else if rows
                .iter()
                .any(|(_, node, transform)| pointer.is_over(node, transform))
            {
                effect = screen.confirm(&mut options);
            }
        } else if actions.just_pressed(AppAction::Confirm) {
            effect = screen.confirm(&mut options);
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
    theme: Res<UiTheme>,
    screen: Res<OptionsScreen>,
    options: Res<PlayerOptions>,
    mut texts: OptionsTextQuery,
) {
    for (mut text, mut color, label, cell, is_status, is_hint) in &mut texts {
        if let Some(label) = label {
            color.0 = if label.0 == screen.selected() {
                theme.menu_selected_color
            } else {
                theme.menu_normal_color
            };
        } else if let Some(cell) = cell {
            let row = screen.rows()[cell.row];
            text.0 = screen.binding_text(row, cell.device, &options);
            let selected_cell = cell.row == screen.selected()
                && (!row.is_binding() || cell.device == screen.device());
            color.0 = if selected_cell {
                theme.menu_selected_color
            } else {
                theme.menu_normal_color
            };
        } else if is_status {
            text.0 = screen.status().to_owned();
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
) {
    if !dirty.0 {
        return;
    }
    dirty.0 = false;
    if let Err(error) = store.store(&options) {
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
}
