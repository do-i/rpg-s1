# Baseline manual play checklist

Use this protocol in M0.17 to record a real graphics-and-audio run of the
current title-screen prototype. Run every command from the repository root.
This checklist is a test procedure, not a result record: do not mark a step
passed until the M0.17 run supplies evidence.

## Result vocabulary

- **PASS**: the observed behavior matches the expectation.
- **FAIL**: the game ran and the behavior was observable, but it differed from
  the expectation (including a crash, error dialog, or unexpected exit).
- **BLOCKED**: an external prerequisite prevented observation. State the
  prerequisite and evidence, such as no usable graphics display or no audio
  output device/mixer. Do not use BLOCKED for an observed wrong behavior.
- **NOT RUN**: the step was not attempted.

If audio output is blocked, continue with all visual and input steps. Record
each audio assertion as BLOCKED rather than inferring a pass from successful
startup. A mixer/device warning in the terminal is evidence for a blocked
audio assertion; it is a FAIL only when it prevents the application from
starting or otherwise contradicts the expected clean run.

## Setup and launch

1. Start from a clean, visible desktop session with a working graphics display.
   Connect or select a known-good audio output device, unmute it, and set a
   safe audible volume. Record the display/session and output device used.
2. Check the baseline before launching:

   ```sh
   cargo fmt -- --check
   cargo test
   cargo clippy --all-targets -- -D warnings
   ```

   Each command must exit successfully. Record the command output or a log
   location; a failure here is a FAIL for the run precondition.
3. Confirm the developer-menu entry resolves to the intended launch command:

   ```sh
   lazymenu-cli --config menu.toml --print
   ```

   Expect `r` / `Run title-screen prototype` / `cargo run`. If
   `lazymenu-cli` is unavailable, record that menu-path check as BLOCKED and
   use the direct launch below; that does not block the game play check.
4. Launch one instance using either path:

   ```sh
   lazymenu-cli --config menu.toml
   ```

   Select `Run title-screen prototype` (key `r`), or use the direct equivalent:

   ```sh
   cargo run
   ```

   Keep the terminal visible after launch to capture warnings or errors.

## Ordered in-game checks

Perform these in order. The Quit action is deliberately last.

1. **Initial presentation.** Expect one resizable window titled
   `Chronicles of the Lost Flame`, initially 1280 by 766, with the title art,
   dark background, and a bottom-centered menu containing `New Game`,
   `Load Game`, and `Quit`. `New Game` is selected; `Load Game` is visibly
   disabled; the status line is empty. Record a screenshot.
2. **Resize.** Resize the window smaller and larger, then restore it. Expect
   the window to remain responsive, the artwork and menu to remain visible,
   and no crash, corruption, or terminal error.
3. **Title music.** With a working audio path, expect title music shortly
   after startup and verify it repeats by listening through one natural track
   ending (record the observed start and repeat times). If no audio output can
   be observed because of hardware, routing, or mixer availability, mark only
   this assertion BLOCKED and preserve the terminal evidence.
4. **Navigation and wrap.** Press Up once from the initial selection. Expect
   the selection to wrap from `New Game` to `Quit`. Press Down once. Expect it
   to wrap back to `New Game`. Press Down once more. Expect `Load Game` to be
   selected but retain its disabled appearance. Each successful selection move
   should play the hover SFX when audio is available; otherwise mark the hover
   audio assertion BLOCKED while recording the visible selection result.
5. **Disabled Load.** With `Load Game` selected, press Enter, then Space.
   Expect no status message, no confirm SFX, no save/load screen, and no exit;
   the title screen remains usable with `Load Game` selected.
6. **New Game.** Press Up once to select `New Game`, then press Enter. Expect
   the status line to read `New Game is the next migration slice.` and the
   application to stay open. Expect one confirm SFX when audio is available;
   otherwise mark that audio assertion BLOCKED.
7. **Quit.** Press Up once to wrap to `Quit`, then press Enter. Expect one
   confirm SFX when audio is available and a clean application exit. This is
   the final input step.
8. **Exit and errors.** After the process ends, inspect the terminal. Expect
   a successful return to the shell without panic, fatal graphics/audio error,
   or unhandled-error output. Record the exit status and relevant terminal
   lines.

## M0.17 evidence record

Fill this table during the run; leave untested assertions as NOT RUN.

| Assertion | Result | Evidence / notes |
| --- | --- | --- |
| Preflight format, test, and Clippy | PASS | `cargo fmt -- --check`, `cargo test` (4 passed), and `cargo clippy --all-targets -- -D warnings` exited 0; see `/tmp/rpg-s1-m0-17-{fmt,test,clippy}.log`. |
| Developer-menu entry / direct launch path | PASS | `lazymenu-cli --config menu.toml --print` reported `r`, `Run title-screen prototype`, `cargo run`; see `/tmp/rpg-s1-m0-17-menu.log`. All evidence launches used that direct `cargo run` path, with test-only environment instrumentation where noted below. |
| Graphics hardware prerequisite | BLOCKED | The only Vulkan adapter is llvmpipe with CPU device type, so the successful visual run was software-rendered. A targeted `WGPU_BACKEND=gl cargo run` recovery found no GPU and exited 101; see `/tmp/rpg-s1-m0-17-runtime-gl.log`. No hardware-backed graphics adapter is available in this session. |
| Initial presentation | PASS | X11 reported one mapped game window at exactly 1280x766. `/tmp/rpg-s1-m0-17-final-initial.png` shows the title art, dark surround, three bottom-centered menu entries, selected New Game, disabled-looking Load Game, and an empty status line. |
| Resize | PASS | The same window remained mapped and responsive at 900x600, 1600x900, and restored 1280x766. Art and menu remained visible without corruption; see `/tmp/rpg-s1-m0-17-final-{small,large,restored}.png`. |
| Title music and repeat | PASS | The game owned ALSA playback device `hw:0,0` in `RUNNING` state while a built-in ALSA file wrapper mirrored the hardware-bound PCM to `/tmp/rpg-s1-m0-17-alsa.raw`. The 48 kHz stereo S16_LE capture contains 211.71 seconds and nonzero signal (first 120 seconds: RMS -17.4 dB, peak -4.0 dB). A 10-second title-only sample repeated bit-for-bit after 25.5925417 seconds (correlation 1.0, maximum sample difference 0), proving multiple natural repeats before input. |
| Navigation, wrap, and hover SFX | PASS | Ordered Up, Down, Down produced Quit, New Game, and disabled Load selection states in `/tmp/rpg-s1-m0-17-nav-{quit,new,load}.png`. The PCM repeat-residual contained distinct hover events at 112.78, 127.46, 141.15, 185.31, and 208.31 seconds for all five successful selection moves in the full sequence. |
| Disabled Load | PASS | Enter and Space left the mapped 1280x766 title window unchanged and usable with no status line or transition; compare `/tmp/rpg-s1-m0-17-nav-load.png` and `/tmp/rpg-s1-m0-17-load-disabled.png`. The event-local PCM residual contained no new confirm event. |
| New Game status and confirm SFX | PASS | `/tmp/rpg-s1-m0-17-new-game.png` shows `New Game is the next migration slice.` while the app remains open. The PCM repeat-residual contains a distinct confirm event at 188.30-188.82 seconds. |
| Quit confirm SFX and clean exit | FAIL | `/tmp/rpg-s1-m0-17-quit-selected.png` shows Quit selected and Enter produced a clean exit status 0, but no distinct confirm event reached hardware PCM before the capture ended at 211.71 seconds. The 210.90-211.07 residual is the expected one-period echo of the 185.31 hover, not a new confirm. |
| Terminal/error review | PASS | `/tmp/rpg-s1-m0-17-runtime-final.log` ends with command exit code 0 and has no panic, fatal error, asset error, or audio error. The only warnings are non-fatal XSETTINGS reload and expected llvmpipe software-rendering warnings. The separate GL recovery exited 101 because no GPU exists; that prerequisite failure is recorded as BLOCKED above. |

Record the run date, commit under test, display/session, audio output device,
launch path, screenshot/log locations, and any BLOCKED prerequisite here:

```text
Date: 2026-08-07
Commit: d8ab18e6e6bed24b3c0875979f094a4fffe0c5b7
Display/session: X11 DISPLAY=:0; X.Org 21.1.24; Virtual-1 2206x1025; Vulkan llvmpipe CPU software renderer; game window 0x3400004; GL recovery found no GPU
Audio output: ALSA card 0 HDA Intel, hw:0,0 Generic Analog; 48 kHz stereo S16_LE; application title volume 0.65
Launch path: cargo run (final evidence run added ALSA_CONFIG_PATH=target/m0_17_asound.conf to mirror the same hw:0,0 PCM to /tmp)
Screenshot/log locations: /tmp/rpg-s1-m0-17-*.png, /tmp/rpg-s1-m0-17-*.log, /tmp/rpg-s1-m0-17-alsa.raw
Blocked prerequisites: Real graphics hardware. Vulkan exposes only llvmpipe CPU software rendering, and the GL backend reports no GPU. PulseAudio monitor capture was unavailable because the uninstrumented app opened ALSA hw:0,0 directly; the evidence run used ALSA's built-in file wrapper to preserve the real hardware-bound PCM instead.
```

The partial evidence run followed the ordered checklist from initial
presentation through Quit, but it does not complete M0.17 because no
hardware-backed graphics adapter was available. The test-only X11 input/resize
helper and ALSA mirror configuration live under ignored `target/` paths and do
not change the shipped runtime. Audio repeat and event detection compared the
captured PCM with itself one measured title-loop period earlier, so continuous
title music cancels and new menu SFX remain visible. The failed Quit SFX is an
observed baseline defect,
not a blocked assertion. The PCM evidence proves nonzero game samples were
delivered through the application's live ALSA hardware playback path; it is not
a human-listener report or an acoustic microphone measurement of loudspeaker
output.
