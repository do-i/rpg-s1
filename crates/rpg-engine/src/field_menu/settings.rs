//! Settings page actions: the player's preferences that are not tied to battle.

use bevy::prelude::*;

use super::{
    battle::MenuSettings,
    input::MenuInput,
    state::{FieldMenuMode, FieldMenuState, wrapped_or_zero},
};
use crate::options_store::PlayerOptions;

/// Cards in the left column of the settings page.
pub(super) const SETTINGS_CARD_COUNT: usize = 1;
/// The settings card that switches all game audio on and off.
pub(super) const SOUND_ROW: usize = 0;
/// Sub switch of the Sound card for background music alone.
pub(super) const MUSIC_ROW: usize = 0;
/// Sub switch of the Sound card for sound effects alone.
pub(super) const SFX_ROW: usize = 1;

/// Sub switches under a left-column card; zero means the card has no right column.
pub(super) const fn sub_count(card: usize) -> usize {
    match card {
        SOUND_ROW => 2,
        _ => 0,
    }
}

pub(super) fn handle_input(
    state: &mut ResMut<'_, FieldMenuState>,
    input: &MenuInput<'_>,
    menu_settings: &mut MenuSettings<'_>,
) {
    if state.mode != FieldMenuMode::Browse {
        return;
    }
    let card = state.selected;
    // Right enters the sub switches of the selected card; Left returns to the list.
    match input.horizontal {
        Some(1) if state.settings_sub.is_none() && sub_count(card) > 0 => {
            state.settings_sub = Some(0);
        }
        Some(-1) => state.settings_sub = None,
        _ => {}
    }
    if let Some(delta) = input.vertical {
        match state.settings_sub {
            Some(sub) => {
                state.settings_sub = Some(wrapped_or_zero(sub, sub_count(card), delta));
            }
            None => state.selected = wrapped_or_zero(card, SETTINGS_CARD_COUNT, delta),
        }
    }
    if input.confirm {
        let Some(options) = menu_settings.options.as_deref_mut() else {
            return;
        };
        match (card, state.settings_sub) {
            (SOUND_ROW, None) => toggle_sound(options),
            (SOUND_ROW, Some(MUSIC_ROW)) => options.music_enabled = !options.music_enabled,
            (SOUND_ROW, Some(_)) => options.sfx_enabled = !options.sfx_enabled,
            _ => return,
        }
        menu_settings.save_options(state);
    }
}

/// Flips the Sound setting. Applying it to the audio output is the sound system's job.
pub(super) fn toggle_sound(options: &mut PlayerOptions) {
    options.sound_enabled = !options.sound_enabled;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_sound_card_has_sub_switches() {
        assert_eq!(sub_count(SOUND_ROW), 2);
        assert_eq!(sub_count(SOUND_ROW + 1), 0);
    }

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
