//! The options screen as a pure state machine.
//!
//! Split from [`crate::options_ui`] the way [`crate::field_menu_domain`] is split from the field
//! menu: every rule about what a keypress means lives here and is tested without a Bevy app, and
//! the UI module only draws the result and forwards raw input.
//!
//! # Capture mode
//!
//! Rebinding needs the screen to stop interpreting input and start recording it. While a row is
//! capturing, the caller must feed raw keys and buttons to [`OptionsScreen::capture_key`] and
//! [`OptionsScreen::capture_button`] and must not act on [`crate::action_input::ActionState`] at
//! all — otherwise pressing Up to bind Up would also scroll the list.
//!
//! Escape always cancels capture and is therefore the one key a player cannot newly bind. That is
//! a deliberate trade: an unconditional escape hatch matters more than the ability to move a key
//! that already defaults to Back, because a capture with no way out is unrecoverable without
//! editing the options file by hand.

use bevy::{
    input::{gamepad::GamepadButton, keyboard::KeyCode},
    prelude::Resource,
};

use crate::{
    engine_config::TextSpeed,
    input_bindings::BindingTarget,
    input_names::{gamepad_button_name, key_name},
    options_store::PlayerOptions,
};

/// Which device's column an edit applies to.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum Device {
    #[default]
    Keyboard,
    Gamepad,
}

impl Device {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Keyboard => "Keyboard",
            Self::Gamepad => "Gamepad",
        }
    }

    const fn toggled(self) -> Self {
        match self {
            Self::Keyboard => Self::Gamepad,
            Self::Gamepad => Self::Keyboard,
        }
    }
}

/// One line of the screen.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OptionsRow {
    Binding(BindingTarget),
    TextSpeed,
    ResetDefaults,
    Back,
}

impl OptionsRow {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Binding(target) => target.label(),
            Self::TextSpeed => "Text Speed",
            Self::ResetDefaults => "Reset To Defaults",
            Self::Back => "Back",
        }
    }

    pub(crate) const fn is_binding(self) -> bool {
        matches!(self, Self::Binding(_))
    }
}

/// What the caller must do after handing the screen an input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OptionsEffect {
    /// Nothing outside the screen changed.
    None,
    /// Options changed and must be written to disk.
    Persist,
    /// The player asked to leave the screen.
    Leave,
}

/// The four text-speed choices the row cycles through.
///
/// `None` means "whatever `assets/settings.yaml` says", which is a real and distinct choice from
/// pinning the same value: a player who never touches the row keeps following the shipped file
/// even if that file later changes.
const TEXT_SPEEDS: [Option<TextSpeed>; 4] = [
    None,
    Some(TextSpeed::Slow),
    Some(TextSpeed::Fast),
    Some(TextSpeed::VeryFast),
];

fn text_speed_label(speed: Option<TextSpeed>) -> &'static str {
    match speed {
        None => "From settings file",
        Some(TextSpeed::Slow) => "Slow",
        Some(TextSpeed::Fast) => "Fast",
        Some(TextSpeed::VeryFast) => "Instant",
    }
}

#[derive(Debug, Resource)]
pub(crate) struct OptionsScreen {
    rows: Vec<OptionsRow>,
    selected: usize,
    device: Device,
    capturing: Option<BindingTarget>,
    status: String,
}

impl Default for OptionsScreen {
    fn default() -> Self {
        let rows = BindingTarget::all()
            .into_iter()
            .map(OptionsRow::Binding)
            .chain([
                OptionsRow::TextSpeed,
                OptionsRow::ResetDefaults,
                OptionsRow::Back,
            ])
            .collect();
        Self {
            rows,
            selected: 0,
            device: Device::Keyboard,
            capturing: None,
            status: String::new(),
        }
    }
}

impl OptionsScreen {
    pub(crate) fn rows(&self) -> &[OptionsRow] {
        &self.rows
    }

    pub(crate) fn selected(&self) -> usize {
        self.selected
    }

    pub(crate) fn device(&self) -> Device {
        self.device
    }

    pub(crate) fn status(&self) -> &str {
        &self.status
    }

    pub(crate) fn is_capturing(&self) -> bool {
        self.capturing.is_some()
    }

    fn selected_row(&self) -> OptionsRow {
        self.rows[self.selected]
    }

    /// Moves the highlight, wrapping at both ends the way every other menu in the port does.
    pub(crate) fn navigate(&mut self, delta: isize) {
        if self.capturing.is_some() {
            return;
        }
        let length = self.rows.len() as isize;
        self.selected = (self.selected as isize + delta).rem_euclid(length) as usize;
        self.status.clear();
    }

    /// Points the highlight at a row the mouse is over.
    pub(crate) fn select(&mut self, index: usize) {
        if self.capturing.is_some() || index >= self.rows.len() || index == self.selected {
            return;
        }
        self.selected = index;
        self.status.clear();
    }

    /// Switches which device column an edit applies to.
    ///
    /// On the text-speed row the same Left/Right instead cycles the value, because that is what a
    /// player pressing Right on a value row expects; the device column means nothing there.
    pub(crate) fn horizontal(
        &mut self,
        delta: isize,
        options: &mut PlayerOptions,
    ) -> OptionsEffect {
        if self.capturing.is_some() || delta == 0 {
            return OptionsEffect::None;
        }
        match self.selected_row() {
            OptionsRow::TextSpeed => self.cycle_text_speed(delta, options),
            OptionsRow::Binding(_) => {
                self.device = self.device.toggled();
                self.status.clear();
                OptionsEffect::None
            }
            OptionsRow::ResetDefaults | OptionsRow::Back => OptionsEffect::None,
        }
    }

    /// Sets the device column directly, for a mouse click that lands in one of them.
    pub(crate) fn set_device(&mut self, device: Device) {
        if self.capturing.is_none() {
            self.device = device;
        }
    }

    fn cycle_text_speed(&mut self, delta: isize, options: &mut PlayerOptions) -> OptionsEffect {
        let current = TEXT_SPEEDS
            .iter()
            .position(|speed| *speed == options.text_speed)
            .unwrap_or(0) as isize;
        let next = (current + delta).rem_euclid(TEXT_SPEEDS.len() as isize) as usize;
        options.text_speed = TEXT_SPEEDS[next];
        self.status = format!("Text speed: {}", text_speed_label(options.text_speed));
        OptionsEffect::Persist
    }

    /// Activates the selected row.
    pub(crate) fn confirm(&mut self, options: &mut PlayerOptions) -> OptionsEffect {
        if self.capturing.is_some() {
            return OptionsEffect::None;
        }
        match self.selected_row() {
            OptionsRow::Binding(target) => {
                self.capturing = Some(target);
                self.status.clear();
                OptionsEffect::None
            }
            OptionsRow::TextSpeed => self.cycle_text_speed(1, options),
            OptionsRow::ResetDefaults => {
                if *options == PlayerOptions::default() {
                    self.status = "Already at the shipped defaults.".to_owned();
                    return OptionsEffect::None;
                }
                *options = PlayerOptions::default();
                self.status = "Every option restored to its shipped default.".to_owned();
                OptionsEffect::Persist
            }
            OptionsRow::Back => OptionsEffect::Leave,
        }
    }

    /// Handles Back: it cancels a capture first, and only leaves the screen when idle.
    pub(crate) fn back(&mut self) -> OptionsEffect {
        if self.capturing.take().is_some() {
            self.status = "Rebinding cancelled.".to_owned();
            return OptionsEffect::None;
        }
        OptionsEffect::Leave
    }

    /// Removes the selected row's most recent binding for the selected device.
    pub(crate) fn unbind(&mut self, options: &mut PlayerOptions) -> OptionsEffect {
        if self.capturing.is_some() {
            return OptionsEffect::None;
        }
        let OptionsRow::Binding(target) = self.selected_row() else {
            return OptionsEffect::None;
        };
        let result = match self.device {
            Device::Keyboard => options
                .bindings
                .keys(target)
                .last()
                .copied()
                .map(|key| options.bindings.unbind_key(target, key)),
            Device::Gamepad => options
                .bindings
                .buttons(target)
                .last()
                .copied()
                .map(|button| options.bindings.unbind_button(target, button)),
        };
        match result {
            Some(Ok(())) => {
                self.status = format!("Removed a binding from {}.", target.label());
                OptionsEffect::Persist
            }
            Some(Err(error)) => {
                self.status = error.message();
                OptionsEffect::None
            }
            None => OptionsEffect::None,
        }
    }

    /// Feeds a raw key to a capturing row.
    ///
    /// Escape is spent on cancelling rather than bound; see the module docs.
    pub(crate) fn capture_key(
        &mut self,
        key: KeyCode,
        options: &mut PlayerOptions,
    ) -> OptionsEffect {
        let Some(target) = self.capturing else {
            return OptionsEffect::None;
        };
        if key == KeyCode::Escape {
            return self.back();
        }
        if self.device != Device::Keyboard {
            return OptionsEffect::None;
        }
        self.capturing = None;
        match options.bindings.bind_key(target, key) {
            Ok(outcome) => {
                let name = key_name(key).unwrap_or("that key");
                self.status = match outcome.stolen_from {
                    Some(previous) => {
                        format!(
                            "{name} bound to {}, taken from {}.",
                            target.label(),
                            previous.label()
                        )
                    }
                    None => format!("{name} bound to {}.", target.label()),
                };
                OptionsEffect::Persist
            }
            Err(error) => {
                self.status = error.message();
                OptionsEffect::None
            }
        }
    }

    /// Feeds a raw gamepad button to a capturing row.
    pub(crate) fn capture_button(
        &mut self,
        button: GamepadButton,
        options: &mut PlayerOptions,
    ) -> OptionsEffect {
        let Some(target) = self.capturing else {
            return OptionsEffect::None;
        };
        if self.device != Device::Gamepad {
            return OptionsEffect::None;
        }
        self.capturing = None;
        match options.bindings.bind_button(target, button) {
            Ok(outcome) => {
                let name = gamepad_button_name(button).unwrap_or("that button");
                self.status = match outcome.stolen_from {
                    Some(previous) => {
                        format!(
                            "{name} bound to {}, taken from {}.",
                            target.label(),
                            previous.label()
                        )
                    }
                    None => format!("{name} bound to {}.", target.label()),
                };
                OptionsEffect::Persist
            }
            Err(error) => {
                self.status = error.message();
                OptionsEffect::None
            }
        }
    }

    /// The text shown in a row's keyboard or gamepad column.
    pub(crate) fn binding_text(
        &self,
        row: OptionsRow,
        device: Device,
        options: &PlayerOptions,
    ) -> String {
        match row {
            OptionsRow::Binding(target) => {
                if self.capturing == Some(target) && self.device == device {
                    return "Press an input…".to_owned();
                }
                let names: Vec<&str> = match device {
                    Device::Keyboard => options
                        .bindings
                        .keys(target)
                        .iter()
                        .filter_map(|key| key_name(*key))
                        .collect(),
                    Device::Gamepad => options
                        .bindings
                        .buttons(target)
                        .iter()
                        .filter_map(|button| gamepad_button_name(*button))
                        .collect(),
                };
                if names.is_empty() {
                    // Unreachable while the never-unbind rule holds, but a blank cell reads as a
                    // rendering bug, so say it plainly instead.
                    "Unbound".to_owned()
                } else {
                    names.join(", ")
                }
            }
            OptionsRow::TextSpeed => match device {
                Device::Keyboard => text_speed_label(options.text_speed).to_owned(),
                Device::Gamepad => String::new(),
            },
            OptionsRow::ResetDefaults | OptionsRow::Back => String::new(),
        }
    }

    /// The hint line describing what the current state does.
    pub(crate) fn hint(&self) -> &'static str {
        if self.is_capturing() {
            return "Press any key or button to bind it. Escape cancels.";
        }
        match self.selected_row() {
            OptionsRow::Binding(_) => {
                "Confirm rebinds. Left/Right switches device. Delete removes a binding."
            }
            OptionsRow::TextSpeed => "Left/Right or Confirm changes the speed.",
            OptionsRow::ResetDefaults => "Confirm restores every shipped default.",
            OptionsRow::Back => "Confirm returns.",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input_bindings::{AppAction, MovementAction};

    const CONFIRM_ROW: usize = 1;
    const TRAVEL_ROW: usize = 6;

    fn screen_and_options() -> (OptionsScreen, PlayerOptions) {
        (OptionsScreen::default(), PlayerOptions::default())
    }

    #[test]
    fn the_row_list_covers_every_binding_plus_the_value_and_command_rows() {
        let screen = OptionsScreen::default();

        assert_eq!(screen.rows().len(), 14);
        assert_eq!(
            screen.rows()[CONFIRM_ROW],
            OptionsRow::Binding(BindingTarget::Menu(AppAction::Confirm))
        );
        assert_eq!(
            screen.rows()[TRAVEL_ROW],
            OptionsRow::Binding(BindingTarget::Menu(AppAction::Travel))
        );
        assert_eq!(
            screen.rows()[7],
            OptionsRow::Binding(BindingTarget::Movement(MovementAction::Up))
        );
        assert_eq!(screen.rows()[11], OptionsRow::TextSpeed);
        assert_eq!(screen.rows()[13], OptionsRow::Back);
    }

    #[test]
    fn navigation_wraps_at_both_ends() {
        let (mut screen, _) = screen_and_options();

        screen.navigate(-1);
        assert_eq!(screen.selected(), 13);
        screen.navigate(1);
        assert_eq!(screen.selected(), 0);
    }

    #[test]
    fn a_rebind_takes_the_next_key_and_reports_what_it_took_it_from() {
        let (mut screen, mut options) = screen_and_options();
        screen.navigate(TRAVEL_ROW as isize);

        assert_eq!(screen.confirm(&mut options), OptionsEffect::None);
        assert!(screen.is_capturing());
        // Confirm ships with three keys, so Enter can move.
        let effect = screen.capture_key(KeyCode::Enter, &mut options);

        assert_eq!(effect, OptionsEffect::Persist);
        assert!(!screen.is_capturing());
        assert_eq!(
            screen.status(),
            "Enter bound to Travel, taken from Confirm."
        );
        assert_eq!(
            options
                .bindings
                .keys(BindingTarget::Menu(AppAction::Travel)),
            [KeyCode::KeyT, KeyCode::Enter]
        );
    }

    #[test]
    fn a_refused_rebind_leaves_capture_and_explains_itself() {
        let (mut screen, mut options) = screen_and_options();
        screen.navigate(TRAVEL_ROW as isize);
        screen.confirm(&mut options);

        // Escape is Back's only key, so it may not be taken.
        let effect = screen.capture_key(KeyCode::Escape, &mut options);

        // Escape is the cancel key, so it cancels rather than reporting the conflict.
        assert_eq!(effect, OptionsEffect::None);
        assert!(!screen.is_capturing());
        assert_eq!(screen.status(), "Rebinding cancelled.");
        assert_eq!(options, PlayerOptions::default());
    }

    #[test]
    fn a_conflicting_bind_that_is_not_escape_reports_the_owner() {
        let (mut screen, mut options) = screen_and_options();
        // Menu Up owns ArrowUp as its only key.
        screen.navigate(TRAVEL_ROW as isize);
        screen.confirm(&mut options);

        let effect = screen.capture_key(KeyCode::ArrowUp, &mut options);

        assert_eq!(effect, OptionsEffect::None);
        assert_eq!(screen.status(), "Menu Up would be left with no binding.");
        assert_eq!(options, PlayerOptions::default());
    }

    #[test]
    fn navigation_and_device_switching_are_inert_while_capturing() {
        let (mut screen, mut options) = screen_and_options();
        screen.navigate(TRAVEL_ROW as isize);
        screen.confirm(&mut options);

        screen.navigate(3);
        screen.select(0);
        screen.horizontal(1, &mut options);
        screen.set_device(Device::Gamepad);

        assert_eq!(
            screen.selected(),
            TRAVEL_ROW,
            "the capturing row must not move"
        );
        assert_eq!(screen.device(), Device::Keyboard);
        assert!(screen.is_capturing());
    }

    #[test]
    fn a_gamepad_capture_only_accepts_a_button_and_a_keyboard_capture_only_a_key() {
        let (mut screen, mut options) = screen_and_options();
        screen.navigate(TRAVEL_ROW as isize);
        screen.horizontal(1, &mut options);
        assert_eq!(screen.device(), Device::Gamepad);
        screen.confirm(&mut options);

        // A stray keypress must not land in the gamepad column.
        assert_eq!(
            screen.capture_key(KeyCode::KeyG, &mut options),
            OptionsEffect::None
        );
        assert!(
            screen.is_capturing(),
            "a wrong-device key is ignored, not accepted"
        );

        let effect = screen.capture_button(GamepadButton::Start, &mut options);
        assert_eq!(effect, OptionsEffect::Persist);
        assert_eq!(
            options
                .bindings
                .buttons(BindingTarget::Menu(AppAction::Travel)),
            [GamepadButton::North, GamepadButton::Start]
        );
        assert_eq!(
            options
                .bindings
                .keys(BindingTarget::Menu(AppAction::Travel)),
            [KeyCode::KeyT],
            "the keyboard column is untouched"
        );
    }

    #[test]
    fn escape_cancels_a_gamepad_capture_too() {
        let (mut screen, mut options) = screen_and_options();
        screen.navigate(TRAVEL_ROW as isize);
        screen.horizontal(1, &mut options);
        screen.confirm(&mut options);

        screen.capture_key(KeyCode::Escape, &mut options);

        assert!(
            !screen.is_capturing(),
            "the keyboard escape hatch is device-independent"
        );
    }

    #[test]
    fn back_cancels_a_capture_before_it_leaves_the_screen() {
        let (mut screen, mut options) = screen_and_options();
        screen.navigate(TRAVEL_ROW as isize);
        screen.confirm(&mut options);

        assert_eq!(screen.back(), OptionsEffect::None, "the first Back cancels");
        assert_eq!(screen.back(), OptionsEffect::Leave, "the second leaves");
    }

    #[test]
    fn the_text_speed_row_cycles_through_the_file_default_and_all_three_speeds() {
        let (mut screen, mut options) = screen_and_options();
        screen.navigate(11);

        let mut seen = Vec::new();
        for _ in 0..TEXT_SPEEDS.len() {
            assert_eq!(screen.horizontal(1, &mut options), OptionsEffect::Persist);
            seen.push(options.text_speed);
        }

        assert_eq!(
            seen,
            vec![
                Some(TextSpeed::Slow),
                Some(TextSpeed::Fast),
                Some(TextSpeed::VeryFast),
                None,
            ],
            "the cycle returns to following the settings file"
        );
        assert_eq!(screen.status(), "Text speed: From settings file");
    }

    #[test]
    fn left_and_right_switch_devices_on_a_binding_row_but_not_on_a_command_row() {
        let (mut screen, mut options) = screen_and_options();

        screen.horizontal(1, &mut options);
        assert_eq!(screen.device(), Device::Gamepad);
        screen.horizontal(1, &mut options);
        assert_eq!(
            screen.device(),
            Device::Keyboard,
            "it is a toggle, not a list"
        );

        screen.navigate(13);
        assert_eq!(screen.rows()[screen.selected()], OptionsRow::Back);
        assert_eq!(screen.horizontal(1, &mut options), OptionsEffect::None);
        assert_eq!(screen.device(), Device::Keyboard);
    }

    #[test]
    fn unbind_removes_the_last_binding_and_refuses_to_empty_a_row() {
        let (mut screen, mut options) = screen_and_options();
        screen.navigate(CONFIRM_ROW as isize);

        assert_eq!(screen.unbind(&mut options), OptionsEffect::Persist);
        assert_eq!(
            options
                .bindings
                .keys(BindingTarget::Menu(AppAction::Confirm)),
            [KeyCode::Enter, KeyCode::Space]
        );
        assert_eq!(screen.unbind(&mut options), OptionsEffect::Persist);
        assert_eq!(screen.unbind(&mut options), OptionsEffect::None);
        assert_eq!(screen.status(), "An action must keep at least one binding.");
        assert_eq!(
            options
                .bindings
                .keys(BindingTarget::Menu(AppAction::Confirm)),
            [KeyCode::Enter]
        );
    }

    #[test]
    fn reset_restores_defaults_and_says_so_only_when_something_changed() {
        let (mut screen, mut options) = screen_and_options();
        screen.navigate(12);

        assert_eq!(screen.confirm(&mut options), OptionsEffect::None);
        assert_eq!(screen.status(), "Already at the shipped defaults.");

        options.text_speed = Some(TextSpeed::Slow);
        options
            .bindings
            .bind_key(BindingTarget::Menu(AppAction::Travel), KeyCode::KeyG)
            .expect("a free key");

        assert_eq!(screen.confirm(&mut options), OptionsEffect::Persist);
        assert_eq!(options, PlayerOptions::default());
    }

    #[test]
    fn the_back_row_leaves_the_screen() {
        let (mut screen, mut options) = screen_and_options();
        screen.navigate(13);

        assert_eq!(screen.confirm(&mut options), OptionsEffect::Leave);
    }

    #[test]
    fn a_capturing_cell_shows_a_prompt_only_in_its_own_device_column() {
        let (mut screen, mut options) = screen_and_options();
        screen.navigate(TRAVEL_ROW as isize);
        screen.confirm(&mut options);
        let row = screen.rows()[TRAVEL_ROW];

        assert_eq!(
            screen.binding_text(row, Device::Keyboard, &options),
            "Press an input…"
        );
        assert_eq!(
            screen.binding_text(row, Device::Gamepad, &options),
            "North",
            "the other column keeps showing its binding"
        );
    }

    #[test]
    fn a_multi_key_row_lists_every_binding() {
        let (screen, options) = screen_and_options();

        assert_eq!(
            screen.binding_text(screen.rows()[CONFIRM_ROW], Device::Keyboard, &options),
            "Enter, Space, NumpadEnter"
        );
    }

    #[test]
    fn the_hint_describes_the_current_row_and_capture_state() {
        let (mut screen, mut options) = screen_and_options();

        assert!(screen.hint().contains("Left/Right switches device"));
        screen.navigate(11);
        assert!(screen.hint().contains("changes the speed"));
        screen.navigate(-11);
        screen.confirm(&mut options);
        assert!(screen.hint().contains("Escape cancels"));
    }
}
