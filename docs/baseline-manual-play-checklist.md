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
| Preflight format, test, and Clippy | NOT RUN | |
| Developer-menu entry / direct launch path | NOT RUN | |
| Initial presentation | NOT RUN | |
| Resize | NOT RUN | |
| Title music and repeat | NOT RUN | |
| Navigation, wrap, and hover SFX | NOT RUN | |
| Disabled Load | NOT RUN | |
| New Game status and confirm SFX | NOT RUN | |
| Quit confirm SFX and clean exit | NOT RUN | |
| Terminal/error review | NOT RUN | |

Record the run date, commit under test, display/session, audio output device,
launch path, screenshot/log locations, and any BLOCKED prerequisite here:

```text
Date:
Commit:
Display/session:
Audio output:
Launch path:
Screenshot/log locations:
Blocked prerequisites:
```
