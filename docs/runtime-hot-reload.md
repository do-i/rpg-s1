# Runtime scenario hot changes

Set `RPG_S1_HOT_RELOAD` when launching the desktop game to enable coordinated
scenario reloads. Any present value enables it; leave the variable unset to disable
it. No Rust rebuild is needed between content edits.

```sh
RPG_S1_HOT_RELOAD=1 cargo run -- play rusted_kingdoms
```

The Bevy 0.19 features selected by this workspace do **not** include `file_watcher`.
Its native watcher is explicitly disabled in the game. Instead, the runtime polls
the selected package every 0.5 seconds while World is idle. This lets it retain the
published assets and World before asking `AssetServer` to reload changed files.
Native autonomous watching must remain disabled: it can replace a shared image or
atlas before the source generation has been retained.

The poll includes YAML, TMX, TSX, PNG/WebP, audio and font files; it ignores symlinks
and backup/temporary file extensions. Revisions include size and modification time,
plus inode and change time on Unix, so an atomic Linux replacement preserving size
and modification time is still detected. It performs metadata inspection, not a
full content hash or filesystem walk every frame. Watching is a development option:
rebuilding retains copies of loaded typed assets, image data and atlas layouts,
and can temporarily increase memory use and pause the game. No performance benefit
is claimed.

Reload waits until overlays, dialogue, battle and map transitions have finished.
It then uses the normal World publication barrier under a black fade. The replacement
must finish asset loading and rebuild rendering, collision, player/NPC/object/enemy
state, and field/transport catalogs before play resumes. The active BGM reference
must load; a changed loop restarts after publication. One-shot SFX changes apply to
subsequent playback. Existing session progress, inventory, levels and flags remain
session data: changing party or bootstrap YAML affects the next new game, not a
migration of the current save. The save-slot balance context refreshes after commit.

Invalid edits or removed loaded dependencies reject the replacement and report the
failure through the same map recovery screen. The previous entities, collision,
assets, catalogs and RNG are restored. `Esc` resumes that generation; `R` retries
all files from the rejected edit; `T` returns to title. After returning to World,
a further file edit also retries the entire rejected set. There is no automatic
retry loop for unchanged invalid content. A 30-second timeout retains an asset guard
against outstanding IO: the screen explains when retry/source return are temporarily
unavailable, continues to allow title return, and updates its key hints when IO
settles. Complete a group of related edits before resuming the game; a partially
saved group may be rejected and can then be retried.

Scenario selection is fixed at process startup. Select another package by launching
`rpg-s1 play PACKAGE_KEY`; there is no in-process scenario-switch UI. Attempts to
mutate the live root restore the original root and inventory before consumers run,
and editing the manifest's scenario ID/version rejects the hot transaction. Restart
to select a different identity. Two packages can use the same map IDs without
sharing asset paths or session state.

## Disposable acceptance procedure

The canonical owner remains [Scenario and validator acceptance](../../rpg-s1-plans/manual-verification.md#scenario-and-validator-acceptance)
(`RK-DBG-006`–`RK-DBG-016`). Automated loader/ECS tests support that task; they do
not close its visual/listening or validator-owner acceptance.

1. Build once with `cargo build`. Copy `target/debug/rpg-s1` and the entire `assets`
   directory into a writable disposable directory outside the checkout (for example under `/tmp`), with `assets` beside the binary.
   Use an isolated `XDG_CONFIG_HOME` and `XDG_DATA_HOME` there. Never edit the canonical
   package during this procedure.
2. Copy the disposable scenario to a second package directory. Give it a distinct
   manifest ID/name/window title and visibly different map/NPC content before launch.
   Run each package with the same copied executable using `play PACKAGE_KEY` and
   `RPG_S1_HOT_RELOAD=1`. Check the title, map, NPCs and save identity belong to the
   selected package. Relaunching is the supported package-switch operation.
3. Launch one copy with `--start-map town_01_ardel --start-position 14,5` and
   `RPG_S1_DEBUG_COLLISION=1`. On that same map, edit a visible ground tile and a
   collision cell in its TMX. After publication, inspect both the changed tile and
   collision overlay; walk against the new blocked cell. Edit the referenced TSX
   or PNG, then the map YAML's NPC definitions and `data/item_boxes.yaml` chest
   definitions, and verify the corresponding
   sprites/entities change without leaving the map or rebuilding Rust.
4. Edit an item/recipe/quest/class or transport catalog. After publication, open the
   relevant menu and inspect the revised data. Close menus before expecting another
   reload. Change the active BGM index reference or replace its MP3 and listen for
   the new loop after publication; change an SFX and trigger it again.
5. Combine a valid map/image edit with invalid YAML, then remove a referenced image.
   Check that each failed attempt reports its owner/path where available, preserves
   the previous map/appearance/collision/session, and does not autosave the candidate.
   Repair the files and press `R`; check the entire replacement appears together.
   Change the manifest ID during play and verify rejection instead of identity reuse.
6. Run the existing validator success/failure fixtures from the acceptance task.
   Hot loading validates the runtime's required assets; it is not a replacement for
   the complete cross-reference validator or a campaign playthrough.

Native window rendering, movement feel, audible playback, non-Linux filesystem
semantics, and the complete acceptance fixture matrix remain manual checks.
