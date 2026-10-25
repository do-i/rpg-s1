//! Applies the player's sound settings to every playing sink.
//!
//! Bevy's `GlobalVolume` only reaches a sink when it is created, so it cannot silence music that
//! is already looping. Instead each sink's volume is its own `PlaybackSettings` volume times a
//! gain worked out from the options: the master Sound switch, then the Music or Sound effects
//! switch depending on whether the entity is a [`LogicalBgmPlayer`]. The gain is re-applied every
//! frame, so fades that write the sink directly and toggles made mid-track both hold.

use bevy::{
    audio::{AudioSink, AudioSinkPlayback, Volume},
    prelude::*,
};

use crate::{options_store::PlayerOptions, world_audio::LogicalBgmPlayer};

/// Set by the launcher to silence the game regardless of the player's sound settings.
const MUTE_AUDIO_VARIABLE: &str = "RPG_S1_MUTE_AUDIO";

pub(crate) struct SoundOptionPlugin;

impl Plugin for SoundOptionPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(apply_to_new_sink)
            .add_systems(Last, apply_sound_options);
    }
}

/// The gain for music (`music`) or sound effects given `options`.
pub(crate) fn channel_gain(options: &PlayerOptions, music: bool, forced_mute: bool) -> f32 {
    let channel = if music {
        options.music_enabled
    } else {
        options.sfx_enabled
    };
    if forced_mute || !options.sound_enabled || !channel {
        0.0
    } else {
        1.0
    }
}

fn forced_mute() -> bool {
    std::env::var_os(MUTE_AUDIO_VARIABLE).is_some()
}

fn apply_to_new_sink(
    added: On<Add, AudioSink>,
    options: Res<PlayerOptions>,
    mut sinks: Query<(&PlaybackSettings, &mut AudioSink, Has<LogicalBgmPlayer>)>,
) {
    if let Ok((settings, mut sink, music)) = sinks.get_mut(added.entity) {
        let gain = channel_gain(&options, music, forced_mute());
        sink.set_volume(settings.volume * Volume::Linear(gain));
    }
}

fn apply_sound_options(
    options: Res<PlayerOptions>,
    mut sinks: Query<(&PlaybackSettings, &mut AudioSink, Has<LogicalBgmPlayer>)>,
) {
    let forced_mute = forced_mute();
    for (settings, mut sink, music) in &mut sinks {
        let gain = channel_gain(&options, music, forced_mute);
        sink.set_volume(settings.volume * Volume::Linear(gain));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_master_switch_silences_both_channels_and_each_channel_has_its_own_switch() {
        let on = PlayerOptions::default();
        assert_eq!(channel_gain(&on, true, false), 1.0);
        assert_eq!(channel_gain(&on, false, false), 1.0);

        let master_off = PlayerOptions {
            sound_enabled: false,
            ..PlayerOptions::default()
        };
        assert_eq!(channel_gain(&master_off, true, false), 0.0);
        assert_eq!(channel_gain(&master_off, false, false), 0.0);

        let no_music = PlayerOptions {
            music_enabled: false,
            ..PlayerOptions::default()
        };
        assert_eq!(channel_gain(&no_music, true, false), 0.0);
        assert_eq!(channel_gain(&no_music, false, false), 1.0);

        let no_sfx = PlayerOptions {
            sfx_enabled: false,
            ..PlayerOptions::default()
        };
        assert_eq!(channel_gain(&no_sfx, true, false), 1.0);
        assert_eq!(channel_gain(&no_sfx, false, false), 0.0);

        assert_eq!(channel_gain(&on, true, true), 0.0);
    }
}
