# Audio output recovery

This is `bevy_audio` 0.19.0 from crates.io, upstream Bevy revision
`c6f634ca9f406d68ba5109d921247b654cb42c10`, under the included MIT / Apache-2.0
licenses. The source is vendored because its audio output resource is private;
the application cannot replace the stream or its error callback through Bevy's
public API.

Local changes are in `src/audio_recovery.rs`, `src/audio_output.rs`, and
`src/lib.rs`:

- Capture the first backend error without logging from the audio callback.
- Drop a failed output stream on the next update, then reopen the default output
  with exponential delays of 1, 2, 4, 8, 16, and at most 30 seconds. A stream
  must remain healthy for 30 seconds before resetting the delay.
- Retain Rodio's fallback device/configuration selection and buffer settings.
- Restart looping players after recovery, preserving whether they were paused.
  Finish interrupted one-shot sounds and discard cues queued during an outage,
  so old sound effects do not replay in a burst when the device returns.
- Leave buffer underruns to CPAL's built-in recovery. Other backend failures
  produce one warning per failed attempt, and a successful recovery is logged.

Run the device-independent regression tests with:

```sh
cargo test -p bevy_audio --lib
cargo fmt --manifest-path vendor/bevy_audio/Cargo.toml -- --check
```

Review or remove this patch when upgrading Bevy. The normal workspace remains
on the same Bevy, Rodio, and CPAL versions.
