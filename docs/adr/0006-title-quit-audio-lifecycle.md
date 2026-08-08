# ADR 0006: Wait for title Quit confirmation before exiting

- Status: Accepted
- Date: 2026-08-07
- Decision owner: M0.17a

## Context

The title screen currently queues `menu_confirm.mp3` and immediately writes
`AppExit::Success` when Quit is confirmed. The process can therefore tear down
the audio device before a confirm sample reaches it. The M0.17 hardware-audio
capture observed precisely that failure.

The shipped confirm clip is 1.333333 seconds long (`ffprobe` reports the MP3
container duration). The game must leave after the audible confirmation when
playback is available, but it must not hang if the asset cannot load or Bevy
has no audio output.

## Decision

M0.17b will model Quit as a small, single-owner lifecycle resource. A Quit
activation is accepted only while that resource is `Idle`; it immediately
becomes `Waiting` and title input is ignored until exit. The same transition
queues exactly one entity containing:

```rust
AudioPlayer::new(confirm_handle)
PlaybackSettings::DESPAWN
QuitConfirmSound
```

`QuitConfirmSound` is a private marker. `PlaybackSettings::DESPAWN` is the
completion signal: Bevy despawns its entity only after its `AudioSink` has
drained. The lifecycle observer records `Started` only after it has seen an
`AudioSink` on the marked entity. It records `Completed` only when that entity
is absent **after** `Started` was recorded. It must not infer completion merely
because a newly queued entity has no sink.

This distinction gives the following meanings:

| Observation | Lifecycle meaning | Exit action |
| --- | --- | --- |
| Marked entity exists without `AudioSink`; asset state is `NotLoaded`, `Loading`, or unknown | Not yet started | Continue waiting, subject to deadline. |
| Marked entity has `AudioSink` | Playback started | Continue waiting for despawn. |
| Marked entity is absent after a previously observed sink | Playback completed | Emit the one success exit. |
| Marked entity exists and `AssetServer::get_load_state(confirm_handle.id())` is `Failed(_)` | Asset-load failure | Emit the one success exit immediately. |
| No completion before deadline, including unavailable audio output (which never inserts a sink) | Bounded playback/start failure | Emit the one success exit. |

The deadline is **3.0 seconds from accepting Quit**: the measured 1.333333 s
clip duration plus 1.666667 s for asset availability, sink creation, schedule
turnover, and a small platform/audio-buffer margin. It is deliberately a
fallback rather than a replacement for the sink/despawn observation. A delayed
but valid playback that begins before the deadline is still allowed to finish;
once `Started` is observed, use a separate 3.0-second *completion* deadline
from that observation. This preserves a full clip while bounding a stuck sink.

The lifecycle owns a terminal `ExitSent` state. Only its single transition from
`Waiting` to `ExitSent` may write `AppExit::Success` (using
`MessageWriter<AppExit>` or the equivalent command). Repeated Enter/Space,
multiple input systems, load failure, timeout, and normal completion are all
ignored after the first accepted request. The input system must not spawn a
second marked entity once the resource is non-idle.

## Bevy 0.19 basis and scheduling

Bevy 0.19's audio implementation provides the required lifecycle directly:

- `PlaybackSettings::DESPAWN` selects `PlaybackMode::Despawn`.
- The audio queue system inserts `AudioSink` only after both an audio output
  stream and the loaded `AudioSource` are available.
- The cleanup system despawns a `Despawn` entity when `AudioSink::empty()` is
  true.
- `AudioPlugin` runs those queue and cleanup systems in `PostUpdate`; an
  unavailable output makes that set not run, so it cannot produce a sink or a
  despawn completion.
- `AssetServer::get_load_state` exposes `LoadState::Failed(_)`, allowing a
  known asset failure to be treated differently from a still-loading asset.

The title lifecycle observer will run in `Update`, after input handling in an
explicit ordered chain. Commands spawned by input are deferred, so the marked
entity first becomes observable on a later update. That is intentional: the
observer starts in `Waiting`, never treats initial absence as completion, and
only accepts entity absence after it recorded a sink in a prior update.
`PostUpdate` audio cleanup may queue its despawn command after the observer;
the observer sees that completed despawn on the next `Update`. This one-frame
latency is safe and avoids depending on Bevy's private audio system set or
system ordering.

## Deterministic test seam

M0.17b will keep the state transition independent of Bevy audio hardware. A
pure, crate-private reducer accepts a controlled elapsed duration and this
observation enum:

```rust
enum QuitPlaybackObservation {
    AwaitingStart,
    Started,
    CompletedAfterStart,
    AssetLoadFailed,
}
```

The Bevy adapter derives these values from the marked entity, `AudioSink`, and
`AssetServer` as specified above. Unit tests feed the reducer directly and
assert its emitted effects (`SpawnConfirm` once and `EmitExit` once). They will
cover duplicate Quit activation, normal start then completion, load failure,
start timeout, and completion timeout with exact synthetic durations. An ECS
integration test can use the existing headless title app, `TimeUpdateStrategy::ManualDuration`,
and the same adapter-facing observation hook; it must not add `AudioPlugin`,
open an output device, decode the MP3, or require graphics hardware.

## Consequences

- Quit waits for the actual Bevy completion lifecycle whenever audio starts.
- Load and output failures cannot leave the title screen stuck indefinitely.
- The direct immediate `AppExit` in the input match is removed in M0.17b.
- This decision does not change New Game or hover sound lifecycles.
