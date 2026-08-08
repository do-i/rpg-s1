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
| Available graphics path | PASS | The real X11 window/input run used the machine's best available Vulkan adapter, llvmpipe with CPU device type. Software rendering is explicitly accepted because this development machine has no GPU. A targeted `WGPU_BACKEND=gl cargo run` probe found no GPU and exited 101; see `/tmp/rpg-s1-m0-17-runtime-gl.log`. |
| Initial presentation | PASS | X11 reported one mapped game window at exactly 1280x766. `/tmp/rpg-s1-m0-17-final-initial.png` shows the title art, dark surround, three bottom-centered menu entries, selected New Game, disabled-looking Load Game, and an empty status line. |
| Resize | PASS | The same window remained mapped and responsive at 900x600, 1600x900, and restored 1280x766. Art and menu remained visible without corruption; see `/tmp/rpg-s1-m0-17-final-{small,large,restored}.png`. |
| Title music and repeat | PASS | The game owned ALSA playback device `hw:0,0` in `RUNNING` state while a built-in ALSA file wrapper mirrored the hardware-bound PCM to `/tmp/rpg-s1-m0-17-alsa.raw`. The 48 kHz stereo S16_LE capture contains 211.71 seconds and nonzero signal (first 120 seconds: RMS -17.4 dB, peak -4.0 dB). A 10-second title-only sample repeated bit-for-bit after 25.5925417 seconds (correlation 1.0, maximum sample difference 0), proving multiple natural repeats before input. |
| Navigation, wrap, and hover SFX | PASS | Ordered Up, Down, Down produced Quit, New Game, and disabled Load selection states in `/tmp/rpg-s1-m0-17-nav-{quit,new,load}.png`. The PCM repeat-residual contained distinct hover events at 112.78, 127.46, 141.15, 185.31, and 208.31 seconds for all five successful selection moves in the full sequence. |
| Disabled Load | PASS | Enter and Space left the mapped 1280x766 title window unchanged and usable with no status line or transition; compare `/tmp/rpg-s1-m0-17-nav-load.png` and `/tmp/rpg-s1-m0-17-load-disabled.png`. The event-local PCM residual contained no new confirm event. |
| New Game status and confirm SFX | PASS | `/tmp/rpg-s1-m0-17-new-game.png` shows `New Game is the next migration slice.` while the app remains open. The PCM repeat-residual contains a distinct confirm event at 188.30-188.82 seconds. |
| Quit confirm SFX and clean exit | FAIL | `/tmp/rpg-s1-m0-17-quit-selected.png` shows Quit selected and Enter produced a clean exit status 0, but no distinct confirm event reached hardware PCM before the capture ended at 211.71 seconds. The 210.90-211.07 residual is the expected one-period echo of the 185.31 hover, not a new confirm. |
| Terminal/error review | PASS | `/tmp/rpg-s1-m0-17-runtime-final.log` ends with command exit code 0 and has no panic, fatal error, asset error, or audio error. The only warnings are non-fatal XSETTINGS reload and expected llvmpipe software-rendering warnings. The separate GL probe exited 101 because no GPU exists; that environment limitation is recorded above. |

Record the run date, commit under test, display/session, audio output device,
launch path, screenshot/log locations, and any BLOCKED prerequisite here:

```text
Date: 2026-08-07
Commit: d8ab18e6e6bed24b3c0875979f094a4fffe0c5b7
Display/session: X11 DISPLAY=:0; X.Org 21.1.24; Virtual-1 2206x1025; Vulkan llvmpipe CPU software renderer; game window 0x3400004; GL recovery found no GPU
Audio output: ALSA card 0 HDA Intel, hw:0,0 Generic Analog; 48 kHz stereo S16_LE; application title volume 0.65
Launch path: cargo run (final evidence run added ALSA_CONFIG_PATH=target/m0_17_asound.conf to mirror the same hw:0,0 PCM to /tmp)
Screenshot/log locations: /tmp/rpg-s1-m0-17-*.png, /tmp/rpg-s1-m0-17-*.log, /tmp/rpg-s1-m0-17-alsa.raw
Environment limitations: Vulkan exposes only llvmpipe CPU software rendering, and the GL backend reports no GPU. Software rendering is accepted under the available-hardware policy because the machine has no GPU. PulseAudio monitor capture was unavailable because the uninstrumented app opened ALSA hw:0,0 directly; the evidence run used ALSA's built-in file wrapper to preserve the real hardware-bound PCM instead.
```

The evidence run followed the ordered checklist from initial presentation
through Quit on the best graphics adapter available to this no-GPU machine.
The test-only X11 input/resize helper and ALSA mirror configuration live under
ignored `target/` paths and do not change the shipped runtime. Audio repeat and
event detection compared the captured PCM with itself one measured title-loop
period earlier, so continuous title music cancels and new menu SFX remain
visible. The failed Quit SFX is an observed baseline defect at the commit under
test, not a blocked assertion. The PCM evidence proves nonzero game samples were
delivered through the application's live ALSA hardware playback path; it is not
a human-listener report or an acoustic microphone measurement of loudspeaker
output.

## M0.17c targeted Quit audio recheck

This focused recheck supersedes the earlier `Quit confirm SFX and clean exit`
FAIL for the repaired commit only. The historical result above remains the
baseline evidence for the defect at commit `d8ab18e`. This recheck did not
repeat the complete checklist; the completion determination below explains how
the full baseline run and this post-fix targeted runtime check combine.

| Field | Result / evidence |
| --- | --- |
| Date and commit | 2026-08-07; `1cf1713c4f8be8ffe8013f3a75e849940ed54439` (`Play title confirm before quitting`). |
| Preflight | PASS: `cargo fmt -- --check`, `cargo test` (13 passed), and `cargo clippy --all-targets -- -D warnings` each exited 0 before launch. |
| Runtime path | PASS: actual `cargo run` on X11 `DISPLAY=:0`; the title window mapped at 1280x766. ALSA used the ignored M0.17 file wrapper with live slave `hw:0,0`, mirroring the same hardware-bound 48 kHz stereo S16_LE PCM to `/tmp/rpg-s1-m0-17c-alsa.raw`. |
| Inputs | PASS: one Up selected Quit, then one Return activated it. Immediately before Return injection the capture contained 2,572,288 frames (53.589333 seconds). |
| Quit confirm in output | PASS: the one-title-loop residual contains a new event from 54.154375 through 55.487708 seconds, starting 0.565042 seconds after the pre-Return capture marker. It matches the prior run's independently identified New Game confirm output at normalized correlation 0.999999783 and least-squares gain 1.000005679; fitted residual RMS is 0.475 sample counts versus event RMS 721.549. The same event's best correlation with the prior hover output is 0.005204271. |
| Clean exit | PASS: the PCM contains the complete 1.333333-second confirm event plus a 0.064292-second capture tail. The terminal log completed 2.125950 seconds after the timestamp immediately before Return injection; because the helper completed injection within 0.504371 seconds, actual key-to-exit time is bounded to 1.621579-2.125950 seconds. `cargo run` exited 0, the log recorded `COMMAND_EXIT_CODE="0"`, and no `target/debug/rpg-s1` or `cargo run` process remained. |
| Evidence | `/tmp/rpg-s1-m0-17c-runtime.log`, `/tmp/rpg-s1-m0-17c-alsa.raw` (10,665,984 bytes; SHA-256 `40d009b26465181650daafa85f73ff08cbf587536cdd9ad72e007a7121825595`), and `/tmp/rpg-s1-m0-17c-analysis.log`. |

The signal comparison subtracts the sample stream exactly 1,228,442 frames
(25.592541667 seconds, one measured title loop) earlier. The 50-52-second
pre-Quit control cancels to zero RMS and peak, while the prior captured New
Game confirm supplies an output-path reference independent of the source MP3
decoder. This proves that the new confirm event reached the captured
hardware-bound PCM before clean process exit; it is not a claim that a human
listener acoustically heard a loudspeaker.

## M0.17 graphics-hardware blocker recheck

The graphics prerequisite was rechecked on 2026-08-08. The guest exposes a
Virtio 1.0 GPU plus `/dev/dri/card1` and `/dev/dri/renderD128`, but
`vulkaninfo --summary` still enumerates only llvmpipe with Vulkan device type
`PHYSICAL_DEVICE_TYPE_CPU`. The installed Vulkan ICD directory contains only
the llvmpipe manifest.

To distinguish a missing guest driver from a missing host capability, the
official Arch `vulkan-virtio` package (`1:26.1.6-1`) was downloaded and
extracted under `/tmp` without installation. Vulkan was pointed exclusively at
that disposable `virtio_icd.json` and `libvulkan_virtio.so`. The driver loaded,
but physical-device enumeration failed with:

```text
setup_loader_term_phys_devs: Failed to detect any valid GPUs in the current config
vkEnumeratePhysicalDevices failed with ERROR_INITIALIZATION_FAILED
```

The guest package alone therefore cannot provide a hardware-backed Vulkan
device. This remains useful environment evidence: Virtio Venus, GPU passthrough,
or another host-provided adapter would be required to exercise a physical GPU.
It is not an M0.17 blocker because this development machine has no GPU and the
available-hardware policy explicitly accepts software rendering.

## M0.17 completion determination

M0.17 is **PASS** as of 2026-08-08 under the available-hardware policy. The
2026-08-07 full ordered run exercised the real X11 window and input path,
multiple resize sizes, the complete menu sequence, and the live ALSA output
path on Vulkan llvmpipe, the best adapter available to this no-GPU development
machine. Its original Quit-audio FAIL at commit `d8ab18e` remains recorded above
as the defect that prompted M0.17a-b.

The M0.17c run at repaired commit `1cf1713` was intentionally focused rather
than a second full checklist replay. It provides post-fix runtime evidence that
Quit emits the complete confirm event and that the application then exits with
status 0 and no lingering process. Together, the full baseline run and the
targeted post-fix recheck cover every M0.17 assertion without claiming that the
entire checklist was replayed after the fix. Gate 0 is therefore complete; the
historical hardware-driver failures above document the environment but do not
override this result.
