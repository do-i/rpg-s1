//! Settings page actions: the player's preferences that are not tied to battle.

use bevy::prelude::*;

use super::{
    battle::MenuSettings,
    input::MenuInput,
    state::{FieldMenuMode, FieldMenuState, wrapped_or_zero},
};
use crate::options_store::PlayerOptions;

/// Rows on the settings page: the master sound switch, then music and sound effects.
pub(super) const SETTINGS_ROW_COUNT: usize = 3;

pub(super) fn handle_input(
    state: &mut ResMut<'_, FieldMenuState>,
    input: &MenuInput<'_>,
    menu_settings: &mut MenuSettings<'_>,
) {
    if state.mode != FieldMenuMode::Browse {
        return;
    }
    if let Some(delta) = input.vertical {
        state.selected = wrapped_or_zero(state.selected, SETTINGS_ROW_COUNT, delta);
    }
    if input.confirm && state.selected < SETTINGS_ROW_COUNT {
        let Some(options) = menu_settings.options.as_deref_mut() else {
            return;
        };
        match state.selected {
            SOUND_ROW => toggle_sound(options),
            MUSIC_ROW => options.music_enabled = !options.music_enabled,
            _ => options.sfx_enabled = !options.sfx_enabled,
        }
        menu_settings.save_options(state);
    }
}

/// The settings row that switches all game audio on and off.
pub(super) const SOUND_ROW: usize = 0;
/// The settings row for background music alone.
pub(super) const MUSIC_ROW: usize = 1;
/// The settings row for sound effects alone.
pub(super) const SFX_ROW: usize = 2;

/// Flips the Sound setting. Applying it to the audio output is the sound system's job.
pub(super) fn toggle_sound(options: &mut PlayerOptions) {
    options.sound_enabled = !options.sound_enabled;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggling_sound_flips_the_setting_and_back() {
        let mut options = PlayerOptions::default();
        assert!(options.sound_enabled);
        toggle_sound(&mut options);
        assert!(!options.sound_enabled);
        toggle_sound(&mut options);
        assert!(options.sound_enabled);
    }
}
