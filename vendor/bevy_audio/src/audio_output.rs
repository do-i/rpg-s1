use crate::{
    AudioPlayer, Decodable, DefaultSpatialScale, GlobalVolume, PlaybackMode, PlaybackSettings,
    SpatialAudioSink, SpatialListener,
};
use bevy_asset::{Asset, Assets};
use bevy_ecs::{prelude::*, system::SystemParam};
use bevy_math::Vec3;
use bevy_transform::prelude::GlobalTransform;
use rodio::{MixerDeviceSink, Player, Source, SpatialPlayer};
use tracing::{info, warn};

use crate::audio_recovery::{OutputChange, RecoveringOutput, open_output};
use bevy_platform::time::Instant;

use crate::{AudioSink, AudioSinkPlayback};

/// Used internally to play audio on the current "audio device"
#[derive(Resource)]
pub(crate) struct AudioOutput {
    device: RecoveringOutput<MixerDeviceSink>,
}

impl Default for AudioOutput {
    fn default() -> Self {
        let now = Instant::now();
        let mut device = RecoveringOutput::new(now);
        if let OutputChange::Unavailable(error) = device.update(now, open_output) {
            warn!("Audio output unavailable; will retry: {error}");
        }
        Self { device }
    }
}

#[derive(Component)]
pub(crate) struct OutputInterrupted;

/// Drop a failed stream before retrying, and detach looping players from its mixer.
pub(crate) fn recover_audio_output(
    mut output: ResMut<AudioOutput>,
    mut players: Query<
        (
            Entity,
            &mut PlaybackSettings,
            Option<&AudioSink>,
            Option<&SpatialAudioSink>,
        ),
        Or<(With<AudioSink>, With<SpatialAudioSink>)>,
    >,
    mut commands: Commands,
) {
    match output.device.update(Instant::now(), open_output) {
        OutputChange::Lost(error) => {
            warn!("Audio output failed; will reopen the device: {error}");
            for (entity, mut settings, sink, spatial_sink) in &mut players {
                let sink: &dyn AudioSinkPlayback = match (sink, spatial_sink) {
                    (Some(sink), _) => sink,
                    (_, Some(sink)) => sink,
                    _ => continue,
                };
                interrupt_player(entity, &mut settings, sink, &mut commands);
            }
        }
        OutputChange::Unavailable(error) => {
            warn!("Audio output still unavailable; will retry: {error}")
        }
        OutputChange::Restored => info!("Audio output restored."),
        OutputChange::None => {}
    }
}

fn interrupt_player(
    entity: Entity,
    settings: &mut PlaybackSettings,
    sink: &dyn AudioSinkPlayback,
    commands: &mut Commands,
) {
    if matches!(settings.mode, PlaybackMode::Loop) {
        // Paused background tracks must stay paused after reconnecting.
        settings.paused = sink.is_paused();
        commands
            .entity(entity)
            .remove::<(AudioSink, SpatialAudioSink)>();
    } else {
        // Finish stale one-shot cues instead of replaying them on recovery.
        sink.stop();
        commands.entity(entity).insert(OutputInterrupted);
    }
}

/// Cues belong to the moment they were requested, even when no device is available.
pub(crate) fn discard_unavailable_audio<T: Decodable + Asset>(
    output: Res<AudioOutput>,
    mut commands: Commands,
    players: Query<
        (Entity, &PlaybackSettings, Has<OutputInterrupted>),
        (
            With<AudioPlayer<T>>,
            Or<(
                With<OutputInterrupted>,
                (Without<AudioSink>, Without<SpatialAudioSink>),
            )>,
        ),
    >,
) {
    for (entity, settings, interrupted) in &players {
        if !interrupted && output.device.stream.is_some() {
            continue;
        }
        match settings.mode {
            PlaybackMode::Loop => {}
            PlaybackMode::Despawn => {
                commands.entity(entity).despawn();
            }
            PlaybackMode::Remove => {
                commands.entity(entity).remove::<(
                    AudioPlayer<T>,
                    AudioSink,
                    SpatialAudioSink,
                    PlaybackSettings,
                    PlaybackRemoveMarker,
                    OutputInterrupted,
                )>();
            }
            PlaybackMode::Once => {
                commands.entity(entity).remove::<(
                    AudioPlayer<T>,
                    AudioSink,
                    SpatialAudioSink,
                    OutputInterrupted,
                )>();
            }
        }
    }
}

/// Marker for internal use, to despawn entities when playback finishes.
#[derive(Component, Default)]
pub struct PlaybackDespawnMarker;

/// Marker for internal use, to remove audio components when playback finishes.
#[derive(Component, Default)]
pub struct PlaybackRemoveMarker;

#[derive(SystemParam)]
pub(crate) struct EarPositions<'w, 's> {
    pub(crate) query: Query<'w, 's, (Entity, &'static GlobalTransform, &'static SpatialListener)>,
}

impl<'w, 's> EarPositions<'w, 's> {
    /// Gets a set of transformed ear positions.
    ///
    /// If there are no listeners, use the default values. If a user has added multiple
    /// listeners for whatever reason, we will return the first value.
    pub(crate) fn get(&self) -> (Vec3, Vec3) {
        let (left_ear, right_ear) = self
            .query
            .iter()
            .next()
            .map(|(_, transform, settings)| {
                (
                    transform.transform_point(settings.left_ear_offset),
                    transform.transform_point(settings.right_ear_offset),
                )
            })
            .unwrap_or_else(|| {
                let settings = SpatialListener::default();
                (settings.left_ear_offset, settings.right_ear_offset)
            });

        (left_ear, right_ear)
    }

    pub(crate) fn multiple_listeners(&self) -> bool {
        self.query.iter().len() > 1
    }
}

/// Plays "queued" audio through the [`AudioOutput`] resource.
///
/// "Queued" audio is any audio entity (with an [`AudioPlayer`] component) that does not have an
/// [`AudioSink`]/[`SpatialAudioSink`] component.
///
/// This system detects such entities, checks if their source asset
/// data is available, and creates/inserts the sink.
pub(crate) fn play_queued_audio_system<Source: Asset + Decodable>(
    audio_output: Res<AudioOutput>,
    audio_sources: Res<Assets<Source>>,
    global_volume: Res<GlobalVolume>,
    query_nonplaying: Query<
        (
            Entity,
            &AudioPlayer<Source>,
            &PlaybackSettings,
            Option<&GlobalTransform>,
        ),
        (Without<AudioSink>, Without<SpatialAudioSink>),
    >,
    ear_positions: EarPositions,
    default_spatial_scale: Res<DefaultSpatialScale>,
    mut commands: Commands,
) where
    f32: rodio::cpal::FromSample<rodio::Sample>,
{
    let Some(stream) = audio_output.device.stream.as_ref() else {
        // audio output unavailable; cannot play sound
        return;
    };
    let mixer = stream.mixer();

    for (entity, source_handle, settings, maybe_emitter_transform) in &query_nonplaying {
        let Some(audio_source) = audio_sources.get(&source_handle.0) else {
            continue;
        };
        // audio data is available (has loaded), begin playback and insert sink component
        if settings.spatial {
            let (left_ear, right_ear) = ear_positions.get();

            // We can only use one `SpatialListener`. If there are more than that, then
            // the user may have made a mistake.
            if ear_positions.multiple_listeners() {
                warn!(
                    "Multiple SpatialListeners found. Using {}.",
                    ear_positions.query.iter().next().unwrap().0
                );
            }

            let scale = settings.spatial_scale.unwrap_or(default_spatial_scale.0).0;

            let emitter_translation = if let Some(emitter_transform) = maybe_emitter_transform {
                (emitter_transform.translation() * scale).into()
            } else {
                warn!("Spatial AudioPlayer with no GlobalTransform component. Using zero.");
                Vec3::ZERO.into()
            };

            let sink = SpatialPlayer::connect_new(
                mixer,
                emitter_translation,
                (left_ear * scale).into(),
                (right_ear * scale).into(),
            );

            let decoder = audio_source.decoder();

            match settings.mode {
                PlaybackMode::Loop => match (settings.start_position, settings.duration) {
                    // custom start position and duration
                    (Some(start_position), Some(duration)) => sink.append(
                        decoder
                            .skip_duration(start_position)
                            .take_duration(duration)
                            .repeat_infinite(),
                    ),

                    // custom start position
                    (Some(start_position), None) => {
                        sink.append(decoder.skip_duration(start_position).repeat_infinite());
                    }

                    // custom duration
                    (None, Some(duration)) => {
                        sink.append(decoder.take_duration(duration).repeat_infinite());
                    }

                    // full clip
                    (None, None) => sink.append(decoder.repeat_infinite()),
                },
                PlaybackMode::Once | PlaybackMode::Despawn | PlaybackMode::Remove => {
                    match (settings.start_position, settings.duration) {
                        (Some(start_position), Some(duration)) => sink.append(
                            decoder
                                .skip_duration(start_position)
                                .take_duration(duration),
                        ),

                        (Some(start_position), None) => {
                            sink.append(decoder.skip_duration(start_position));
                        }

                        (None, Some(duration)) => sink.append(decoder.take_duration(duration)),

                        (None, None) => sink.append(decoder),
                    }
                }
            }

            let mut sink = SpatialAudioSink::new(sink);

            if settings.muted {
                sink.mute();
            }

            sink.set_speed(settings.speed);
            sink.set_volume(settings.volume * global_volume.volume);

            if settings.paused {
                sink.pause();
            }

            match settings.mode {
                PlaybackMode::Loop | PlaybackMode::Once => commands.entity(entity).insert(sink),
                PlaybackMode::Despawn => commands
                    .entity(entity)
                    // PERF: insert as bundle to reduce archetype moves
                    .insert((sink, PlaybackDespawnMarker)),
                PlaybackMode::Remove => commands
                    .entity(entity)
                    // PERF: insert as bundle to reduce archetype moves
                    .insert((sink, PlaybackRemoveMarker)),
            };
        } else {
            let sink = Player::connect_new(mixer);
            let decoder = audio_source.decoder();

            match settings.mode {
                PlaybackMode::Loop => match (settings.start_position, settings.duration) {
                    // custom start position and duration
                    (Some(start_position), Some(duration)) => sink.append(
                        decoder
                            .skip_duration(start_position)
                            .take_duration(duration)
                            .repeat_infinite(),
                    ),

                    // custom start position
                    (Some(start_position), None) => {
                        sink.append(decoder.skip_duration(start_position).repeat_infinite());
                    }

                    // custom duration
                    (None, Some(duration)) => {
                        sink.append(decoder.take_duration(duration).repeat_infinite());
                    }

                    // full clip
                    (None, None) => sink.append(decoder.repeat_infinite()),
                },
                PlaybackMode::Once | PlaybackMode::Despawn | PlaybackMode::Remove => {
                    match (settings.start_position, settings.duration) {
                        (Some(start_position), Some(duration)) => sink.append(
                            decoder
                                .skip_duration(start_position)
                                .take_duration(duration),
                        ),

                        (Some(start_position), None) => {
                            sink.append(decoder.skip_duration(start_position));
                        }

                        (None, Some(duration)) => sink.append(decoder.take_duration(duration)),

                        (None, None) => sink.append(decoder),
                    }
                }
            }

            let mut sink = AudioSink::new(sink);

            if settings.muted {
                sink.mute();
            }

            sink.set_speed(settings.speed);
            sink.set_volume(settings.volume * global_volume.volume);

            if settings.paused {
                sink.pause();
            }

            match settings.mode {
                PlaybackMode::Loop | PlaybackMode::Once => commands.entity(entity).insert(sink),
                PlaybackMode::Despawn => commands
                    .entity(entity)
                    // PERF: insert as bundle to reduce archetype moves
                    .insert((sink, PlaybackDespawnMarker)),
                PlaybackMode::Remove => commands
                    .entity(entity)
                    // PERF: insert as bundle to reduce archetype moves
                    .insert((sink, PlaybackRemoveMarker)),
            };
        }
    }
}

pub(crate) fn cleanup_finished_audio<T: Decodable + Asset>(
    mut commands: Commands,
    query_nonspatial_despawn: Query<
        (Entity, &AudioSink),
        (With<PlaybackDespawnMarker>, With<AudioPlayer<T>>),
    >,
    query_spatial_despawn: Query<
        (Entity, &SpatialAudioSink),
        (With<PlaybackDespawnMarker>, With<AudioPlayer<T>>),
    >,
    query_nonspatial_remove: Query<
        (Entity, &AudioSink),
        (With<PlaybackRemoveMarker>, With<AudioPlayer<T>>),
    >,
    query_spatial_remove: Query<
        (Entity, &SpatialAudioSink),
        (With<PlaybackRemoveMarker>, With<AudioPlayer<T>>),
    >,
) {
    for (entity, sink) in &query_nonspatial_despawn {
        if sink.sink.empty() {
            commands.entity(entity).despawn();
        }
    }
    for (entity, sink) in &query_spatial_despawn {
        if sink.sink.empty() {
            commands.entity(entity).despawn();
        }
    }
    for (entity, sink) in &query_nonspatial_remove {
        if sink.sink.empty() {
            commands.entity(entity).remove::<(
                AudioPlayer<T>,
                AudioSink,
                PlaybackSettings,
                PlaybackRemoveMarker,
            )>();
        }
    }
    for (entity, sink) in &query_spatial_remove {
        if sink.sink.empty() {
            commands.entity(entity).remove::<(
                AudioPlayer<T>,
                SpatialAudioSink,
                PlaybackSettings,
                PlaybackRemoveMarker,
            )>();
        }
    }
}

/// Run Condition to only play audio if the audio output is available
pub(crate) fn audio_output_available(audio_output: Res<AudioOutput>) -> bool {
    audio_output.device.stream.is_some()
}

/// Updates spatial audio sinks when emitter positions change.
pub(crate) fn update_emitter_positions(
    mut emitters: Query<
        (&GlobalTransform, &SpatialAudioSink, &PlaybackSettings),
        Or<(Changed<GlobalTransform>, Changed<PlaybackSettings>)>,
    >,
    default_spatial_scale: Res<DefaultSpatialScale>,
) {
    for (transform, sink, settings) in emitters.iter_mut() {
        let scale = settings.spatial_scale.unwrap_or(default_spatial_scale.0).0;

        let translation = transform.translation() * scale;
        sink.set_emitter_position(translation);
    }
}

/// Updates spatial audio sink ear positions when spatial listeners change.
pub(crate) fn update_listener_positions(
    mut emitters: Query<(&SpatialAudioSink, &PlaybackSettings)>,
    changed_listener: Query<
        (),
        (
            Or<(
                Changed<SpatialListener>,
                Changed<GlobalTransform>,
                Changed<PlaybackSettings>,
            )>,
            With<SpatialListener>,
        ),
    >,
    ear_positions: EarPositions,
    default_spatial_scale: Res<DefaultSpatialScale>,
) {
    if !default_spatial_scale.is_changed() && changed_listener.is_empty() {
        return;
    }

    let (left_ear, right_ear) = ear_positions.get();

    for (sink, settings) in emitters.iter_mut() {
        let scale = settings.spatial_scale.unwrap_or(default_spatial_scale.0).0;

        sink.set_ears_position(left_ear * scale, right_ear * scale);
    }
}

#[cfg(test)]
mod recovery_tests {
    use super::*;
    use bevy_ecs::system::RunSystemOnce;
    use std::num::NonZero;

    fn sink() -> AudioSink {
        let (mixer, _source) =
            rodio::mixer::mixer(NonZero::new(2).unwrap(), NonZero::new(48_000).unwrap());
        let player = Player::connect_new(&mixer);
        player.append(rodio::source::SineWave::new(440.0));
        AudioSink::new(player)
    }

    fn unavailable_output() -> AudioOutput {
        AudioOutput {
            // Keep schedule tests independent of the host's real audio devices.
            device: RecoveringOutput::new(Instant::now() + std::time::Duration::from_secs(3600)),
        }
    }

    #[test]
    fn loops_restart_with_their_current_pause_state_and_interrupted_cues_are_cleaned_up() {
        let mut world = World::new();
        world.insert_resource(unavailable_output());
        let music = world
            .spawn((
                AudioPlayer::<crate::Pitch>(Default::default()),
                PlaybackSettings::LOOP,
                sink(),
            ))
            .id();
        // Simulate a track paused through its sink after initial playback began.
        world.get::<AudioSink>(music).unwrap().pause();
        let cue = world
            .spawn((
                AudioPlayer::<crate::Pitch>(Default::default()),
                PlaybackSettings::DESPAWN,
                sink(),
                PlaybackDespawnMarker,
            ))
            .id();
        let remove = world
            .spawn((
                AudioPlayer::<crate::Pitch>(Default::default()),
                PlaybackSettings::REMOVE,
                sink(),
                PlaybackRemoveMarker,
            ))
            .id();
        let once = world
            .spawn((
                AudioPlayer::<crate::Pitch>(Default::default()),
                PlaybackSettings::ONCE,
                sink(),
            ))
            .id();
        world
            .run_system_once(
                |mut players: Query<(Entity, &mut PlaybackSettings, &AudioSink)>,
                 mut commands: Commands| {
                    for (entity, mut settings, sink) in &mut players {
                        interrupt_player(entity, &mut settings, sink, &mut commands);
                    }
                },
            )
            .unwrap();
        world
            .run_system_once(discard_unavailable_audio::<crate::Pitch>)
            .unwrap();

        assert!(world.get::<AudioPlayer<crate::Pitch>>(music).is_some());
        assert!(world.get::<AudioSink>(music).is_none());
        assert!(world.get::<PlaybackSettings>(music).unwrap().paused);
        assert!(world.get_entity(cue).is_err());
        assert!(world.get_entity(remove).is_ok());
        assert!(world.get::<AudioPlayer<crate::Pitch>>(remove).is_none());
        assert!(world.get::<PlaybackSettings>(remove).is_none());
        assert!(world.get::<AudioSink>(remove).is_none());
        assert!(world.get_entity(once).is_ok());
        assert!(world.get::<AudioPlayer<crate::Pitch>>(once).is_none());
        assert!(world.get::<AudioSink>(once).is_none());
    }

    #[test]
    fn cues_requested_during_an_outage_do_not_accumulate_for_later_playback() {
        let mut world = World::new();
        world.insert_resource(unavailable_output());
        let music = world
            .spawn((
                AudioPlayer::<crate::Pitch>(Default::default()),
                PlaybackSettings::LOOP,
            ))
            .id();
        let cues: Vec<_> = (0..100)
            .map(|_| {
                world
                    .spawn((
                        AudioPlayer::<crate::Pitch>(Default::default()),
                        PlaybackSettings::DESPAWN,
                    ))
                    .id()
            })
            .collect();
        world
            .run_system_once(discard_unavailable_audio::<crate::Pitch>)
            .unwrap();
        assert!(world.get::<AudioPlayer<crate::Pitch>>(music).is_some());
        assert!(
            cues.into_iter()
                .all(|entity| world.get_entity(entity).is_err())
        );
    }

    #[test]
    fn audio_plugin_updates_without_a_device_and_cleans_up_interrupted_players_once() {
        use bevy_app::{App, TaskPoolPlugin};
        use bevy_asset::AssetPlugin;

        let mut app = App::new();
        app.insert_resource(unavailable_output()).add_plugins((
            TaskPoolPlugin::default(),
            AssetPlugin::default(),
            crate::AudioPlugin::default(),
        ));
        let music = app
            .world_mut()
            .spawn((
                AudioPlayer::<crate::Pitch>(Default::default()),
                PlaybackSettings::LOOP,
            ))
            .id();
        let cue = app
            .world_mut()
            .spawn((
                AudioPlayer::<crate::Pitch>(Default::default()),
                PlaybackSettings::DESPAWN,
            ))
            .id();
        // An empty interrupted sink could also match normal finished-audio cleanup.
        let finished = app
            .world_mut()
            .spawn((
                AudioPlayer::<crate::Pitch>(Default::default()),
                PlaybackSettings::DESPAWN,
                AudioSink::new(Player::new().0),
                PlaybackDespawnMarker,
                OutputInterrupted,
            ))
            .id();
        app.update();
        app.update();
        assert!(
            app.world()
                .get::<AudioPlayer<crate::Pitch>>(music)
                .is_some()
        );
        assert!(app.world().get_entity(cue).is_err());
        assert!(app.world().get_entity(finished).is_err());
    }
}
