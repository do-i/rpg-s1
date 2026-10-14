//! Battle-entry presentation: the battle camera and the authored battle BGM with its fade.

use bevy::{
    asset::{AssetServer, Assets, Handle, LoadState},
    audio::{AudioSink, AudioSinkPlayback, PlaybackMode, Volume},
    prelude::*,
};

use crate::{
    encounter::BattleEntry,
    gameplay_canvas::fixed_gameplay_camera,
    scenario_audio::{BGM_INDEX_PATH, BgmIndex},
    scenario_path::ScenarioRelativePath,
    scenario_root::ScenarioRoot,
    world_audio::LogicalBgmPlayer,
};

pub(super) const BATTLE_BGM_VOLUME: f32 = 0.3;
pub(super) const BATTLE_BGM_FADE_SECONDS: f32 = 0.8;

/// Sent by the battle UI on the first frame after the final enemy reaches zero HP.
#[derive(Clone, Copy, Debug, Default, Message)]
pub(crate) struct BattleBgmFadeOut;

#[derive(Debug, Default, Resource)]
pub(super) struct BattlePresentationState {
    bgm_index: Option<Handle<BgmIndex>>,
    audio_started: bool,
    fade_requested: bool,
    fade_elapsed: f32,
}

#[derive(Component)]
pub(super) struct BattlePresentation;

#[derive(Component)]
pub(super) struct BattleBgm;

pub(super) fn begin_battle_presentation(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    entry: Option<Res<BattleEntry>>,
    mut state: ResMut<BattlePresentationState>,
    logical_bgm: Query<Entity, With<LogicalBgmPlayer>>,
) {
    *state = BattlePresentationState::default();
    for entity in &logical_bgm {
        commands.entity(entity).despawn();
    }
    let Some(_entry) = entry else {
        return;
    };
    commands.spawn((fixed_gameplay_camera(), BattlePresentation));
    let bgm_index = ScenarioRelativePath::try_from(BGM_INDEX_PATH)
        .expect("canonical BGM index path is scenario-relative");
    state.bgm_index = Some(asset_server.load(root.resolve(&bgm_index)));
}

#[expect(
    clippy::too_many_arguments,
    reason = "battle audio coordinates the authored track, fade request, and sink lifecycle"
)]
pub(super) fn drive_battle_audio(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    indexes: Res<Assets<BgmIndex>>,
    entry: Option<Res<BattleEntry>>,
    time: Res<Time>,
    mut fade_requests: MessageReader<BattleBgmFadeOut>,
    mut players: Query<(&mut PlaybackSettings, Option<&mut AudioSink>), With<BattleBgm>>,
    mut state: ResMut<BattlePresentationState>,
) {
    if fade_requests.read().next().is_some() {
        state.fade_requested = true;
    }
    if !state.audio_started
        && let (Some(entry), Some(handle)) = (entry, state.bgm_index.as_ref())
        && !matches!(asset_server.load_state(handle.id()), LoadState::Failed(_))
        && let Some(index) = indexes.get(handle)
        && let Some(path) = index.resolve_key(&root, &entry.bgm_key)
    {
        commands.spawn((
            AudioPlayer::new(asset_server.load(path)),
            PlaybackSettings {
                mode: PlaybackMode::Loop,
                volume: Volume::Linear(battle_bgm_volume(&state)),
                ..default()
            },
            LogicalBgmPlayer,
            BattleBgm,
            BattlePresentation,
        ));
        state.audio_started = true;
    }
    if state.fade_requested {
        state.fade_elapsed = (state.fade_elapsed + time.delta_secs()).min(BATTLE_BGM_FADE_SECONDS);
        let volume = battle_bgm_volume(&state);
        for (mut settings, sink) in &mut players {
            settings.volume = Volume::Linear(volume);
            if let Some(mut sink) = sink {
                sink.set_volume(Volume::Linear(volume));
            }
        }
    }
}

pub(super) fn battle_bgm_volume(state: &BattlePresentationState) -> f32 {
    if !state.fade_requested {
        return BATTLE_BGM_VOLUME;
    }
    BATTLE_BGM_VOLUME * (1.0 - state.fade_elapsed / BATTLE_BGM_FADE_SECONDS)
}

pub(super) fn cleanup_battle_presentation(
    mut commands: Commands,
    entities: Query<Entity, With<BattlePresentation>>,
) {
    for entity in &entities {
        commands.entity(entity).despawn();
    }
}
