//! Stable text names for every bindable key and gamepad button.
//!
//! Bevy's `serialize` feature is not enabled, so `KeyCode` and `GamepadButton` have no serde
//! impls to borrow. That absence is convenient rather than annoying: the options file is
//! hand-editable, so it wants names chosen for a reader ("Up", "LeftShift") rather than Bevy's
//! internal variant spelling ("ArrowUp", "ShiftLeft"), and owning the table means a Bevy rename
//! cannot silently invalidate every player's saved bindings.
//!
//! The set is deliberately a subset. Only keys a player could reasonably want on a menu action
//! appear here; media keys, IME keys, and the browser cluster are omitted, so binding them is
//! impossible rather than possible-but-broken.

use bevy::input::{gamepad::GamepadButton, keyboard::KeyCode};

/// Declares a two-way name table and the round-trip test that keeps it honest.
macro_rules! name_table {
    ($type:ty, $to_name:ident, $from_name:ident, $all:ident, $($name:literal => $variant:path),+ $(,)?) => {
        /// Every bindable value, in the order the options screen offers them.
        pub(crate) const $all: &[$type] = &[$($variant),+];

        /// The stable options-file name for a bindable value.
        pub(crate) fn $to_name(value: $type) -> Option<&'static str> {
            match value {
                $($variant => Some($name),)+
                _ => None,
            }
        }

        /// The bindable value a stable options-file name refers to.
        pub(crate) fn $from_name(name: &str) -> Option<$type> {
            match name {
                $($name => Some($variant),)+
                _ => None,
            }
        }
    };
}

name_table!(
    KeyCode,
    key_name,
    key_from_name,
    ALL_BINDABLE_KEYS,
    // Editing and navigation.
    "Escape" => KeyCode::Escape,
    "Enter" => KeyCode::Enter,
    "NumpadEnter" => KeyCode::NumpadEnter,
    "Space" => KeyCode::Space,
    "Tab" => KeyCode::Tab,
    "Backspace" => KeyCode::Backspace,
    "Delete" => KeyCode::Delete,
    "Insert" => KeyCode::Insert,
    "Home" => KeyCode::Home,
    "End" => KeyCode::End,
    "PageUp" => KeyCode::PageUp,
    "PageDown" => KeyCode::PageDown,
    // Arrows. The port names these without Bevy's `Arrow` prefix because the options screen shows
    // the same string it stores.
    "Up" => KeyCode::ArrowUp,
    "Down" => KeyCode::ArrowDown,
    "Left" => KeyCode::ArrowLeft,
    "Right" => KeyCode::ArrowRight,
    // Modifiers, named side-first the way a player reads them aloud.
    "LeftShift" => KeyCode::ShiftLeft,
    "RightShift" => KeyCode::ShiftRight,
    "LeftControl" => KeyCode::ControlLeft,
    "RightControl" => KeyCode::ControlRight,
    "LeftAlt" => KeyCode::AltLeft,
    "RightAlt" => KeyCode::AltRight,
    "LeftSuper" => KeyCode::SuperLeft,
    "RightSuper" => KeyCode::SuperRight,
    // Letters.
    "A" => KeyCode::KeyA,
    "B" => KeyCode::KeyB,
    "C" => KeyCode::KeyC,
    "D" => KeyCode::KeyD,
    "E" => KeyCode::KeyE,
    "F" => KeyCode::KeyF,
    "G" => KeyCode::KeyG,
    "H" => KeyCode::KeyH,
    "I" => KeyCode::KeyI,
    "J" => KeyCode::KeyJ,
    "K" => KeyCode::KeyK,
    "L" => KeyCode::KeyL,
    "M" => KeyCode::KeyM,
    "N" => KeyCode::KeyN,
    "O" => KeyCode::KeyO,
    "P" => KeyCode::KeyP,
    "Q" => KeyCode::KeyQ,
    "R" => KeyCode::KeyR,
    "S" => KeyCode::KeyS,
    "T" => KeyCode::KeyT,
    "U" => KeyCode::KeyU,
    "V" => KeyCode::KeyV,
    "W" => KeyCode::KeyW,
    "X" => KeyCode::KeyX,
    "Y" => KeyCode::KeyY,
    "Z" => KeyCode::KeyZ,
    // Number row.
    "0" => KeyCode::Digit0,
    "1" => KeyCode::Digit1,
    "2" => KeyCode::Digit2,
    "3" => KeyCode::Digit3,
    "4" => KeyCode::Digit4,
    "5" => KeyCode::Digit5,
    "6" => KeyCode::Digit6,
    "7" => KeyCode::Digit7,
    "8" => KeyCode::Digit8,
    "9" => KeyCode::Digit9,
    // Function row.
    "F1" => KeyCode::F1,
    "F2" => KeyCode::F2,
    "F3" => KeyCode::F3,
    "F4" => KeyCode::F4,
    "F5" => KeyCode::F5,
    "F6" => KeyCode::F6,
    "F7" => KeyCode::F7,
    "F8" => KeyCode::F8,
    "F9" => KeyCode::F9,
    "F10" => KeyCode::F10,
    "F11" => KeyCode::F11,
    "F12" => KeyCode::F12,
    // Numeric keypad.
    "Numpad0" => KeyCode::Numpad0,
    "Numpad1" => KeyCode::Numpad1,
    "Numpad2" => KeyCode::Numpad2,
    "Numpad3" => KeyCode::Numpad3,
    "Numpad4" => KeyCode::Numpad4,
    "Numpad5" => KeyCode::Numpad5,
    "Numpad6" => KeyCode::Numpad6,
    "Numpad7" => KeyCode::Numpad7,
    "Numpad8" => KeyCode::Numpad8,
    "Numpad9" => KeyCode::Numpad9,
    "NumpadAdd" => KeyCode::NumpadAdd,
    "NumpadSubtract" => KeyCode::NumpadSubtract,
    "NumpadMultiply" => KeyCode::NumpadMultiply,
    "NumpadDivide" => KeyCode::NumpadDivide,
    "NumpadDecimal" => KeyCode::NumpadDecimal,
    // Punctuation.
    "Minus" => KeyCode::Minus,
    "Equal" => KeyCode::Equal,
    "LeftBracket" => KeyCode::BracketLeft,
    "RightBracket" => KeyCode::BracketRight,
    "Backslash" => KeyCode::Backslash,
    "Semicolon" => KeyCode::Semicolon,
    "Quote" => KeyCode::Quote,
    "Comma" => KeyCode::Comma,
    "Period" => KeyCode::Period,
    "Slash" => KeyCode::Slash,
    "Backquote" => KeyCode::Backquote,
);

name_table!(
    GamepadButton,
    gamepad_button_name,
    gamepad_button_from_name,
    ALL_BINDABLE_GAMEPAD_BUTTONS,
    // Face buttons. Bevy names these by position rather than by any vendor's letter, and so does
    // the options file: the same pad reports South for Xbox A and PlayStation Cross.
    "South" => GamepadButton::South,
    "East" => GamepadButton::East,
    "North" => GamepadButton::North,
    "West" => GamepadButton::West,
    "DPadUp" => GamepadButton::DPadUp,
    "DPadDown" => GamepadButton::DPadDown,
    "DPadLeft" => GamepadButton::DPadLeft,
    "DPadRight" => GamepadButton::DPadRight,
    "LeftShoulder" => GamepadButton::LeftTrigger,
    "RightShoulder" => GamepadButton::RightTrigger,
    "LeftTrigger" => GamepadButton::LeftTrigger2,
    "RightTrigger" => GamepadButton::RightTrigger2,
    "LeftStick" => GamepadButton::LeftThumb,
    "RightStick" => GamepadButton::RightThumb,
    "Select" => GamepadButton::Select,
    "Start" => GamepadButton::Start,
    "Mode" => GamepadButton::Mode,
);

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_bindable_key_round_trips_through_its_name() {
        for key in ALL_BINDABLE_KEYS {
            let name = key_name(*key).expect("a listed key has a name");
            assert_eq!(key_from_name(name), Some(*key), "{name} must round-trip");
        }
    }

    #[test]
    fn every_bindable_gamepad_button_round_trips_through_its_name() {
        for button in ALL_BINDABLE_GAMEPAD_BUTTONS {
            let name = gamepad_button_name(*button).expect("a listed button has a name");
            assert_eq!(
                gamepad_button_from_name(name),
                Some(*button),
                "{name} must round-trip"
            );
        }
    }

    #[test]
    fn names_and_values_are_both_unique() {
        let mut names = HashSet::new();
        let mut values = HashSet::new();
        for key in ALL_BINDABLE_KEYS {
            assert!(
                names.insert(key_name(*key).expect("a name")),
                "duplicate name"
            );
            assert!(values.insert(*key), "duplicate key {key:?}");
        }

        let mut names = HashSet::new();
        let mut values = HashSet::new();
        for button in ALL_BINDABLE_GAMEPAD_BUTTONS {
            assert!(
                names.insert(gamepad_button_name(*button).expect("a name")),
                "duplicate name"
            );
            assert!(values.insert(*button), "duplicate button {button:?}");
        }
    }

    #[test]
    fn unlisted_keys_and_unknown_names_are_rejected() {
        // A real `KeyCode` the port deliberately does not offer.
        assert_eq!(key_name(KeyCode::MediaPlayPause), None);
        assert_eq!(key_from_name("MediaPlayPause"), None);
        assert_eq!(
            key_from_name("ArrowUp"),
            None,
            "Bevy's spelling is not the file's"
        );
        assert_eq!(key_from_name(""), None);
        assert_eq!(
            gamepad_button_from_name("A"),
            None,
            "face buttons are positional"
        );
        assert_eq!(gamepad_button_name(GamepadButton::Other(3)), None);
    }

    #[test]
    fn every_default_binding_is_expressible() {
        // The shipped defaults must survive a save/load cycle, so each must be in the table.
        for key in [
            KeyCode::Escape,
            KeyCode::Enter,
            KeyCode::Space,
            KeyCode::NumpadEnter,
            KeyCode::ArrowUp,
            KeyCode::ArrowDown,
            KeyCode::ArrowLeft,
            KeyCode::ArrowRight,
            KeyCode::KeyT,
        ] {
            assert!(key_name(key).is_some(), "{key:?} is a default binding");
        }
    }
}
