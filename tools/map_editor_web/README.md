# Map Editor — Web Frontend

React Flow UI for browsing and editing the scenario's portal graph. The
Python backend (`tools/map_editor/web/server.py`) owns all TMX reading and
writing; this app is presentation only.

## One-time build

```sh
scripts/map-editor.sh setup   # tools/.venv, Python deps, and this dist/ build
```

That is the whole prerequisite step; `dist/` is served by the Python backend at "/".

## Run

```sh
scripts/map-editor.sh web     # or: lazymenu-cli -> "Map editor - Web"
```

## Frontend development

```sh
scripts/map-editor.sh web --no-browser &
cd tools/map_editor_web && npm run dev   # Vite on :5173, proxies /api to :8017
```

## Using the editor

- **Create a portal**: drag the green dot on a map's right edge onto another
  map. Pick the door tile and the arrival tile in the dialog; leave "also
  create return portal" checked for a two-way door.
- **Edit a portal**: click its edge, then "Move arrival tile…" or "Delete
  portal" in the right panel.
- **Undo / redo**: Ctrl+Z / Ctrl+Shift+Z (or the toolbar buttons).
- Edits write to the TMX files immediately; the first edit per file creates a
  sibling `.bak` backup.

## Conflicts and editing policy

Run one editor process with one `EditorService` for a scenario (the launcher
uses one Uvicorn worker). Multiple browser tabs may use that process. Do not run
multiple backend workers or separate editors writing the same scenario: the
mutation lock is in-process. Direct Python clients may omit an expected revision
for serialized operations; browser requests send `If-Match` with the source
map's SHA-256 revision from the last graph or successful mutation.

Source and validated destination revisions are captured before validation and
checked again just before publication. An external change observed during that
interval rejects the edit with HTTP 409 and preserves the external file. This
check is not filesystem compare-and-swap: another process could write between
the final check and replacement. Coordinate Tiled saves with browser editing;
finish a Tiled save and refresh the graph before making a browser edit. Pause
browser mutations while saving in Tiled. Graph refresh rereads map metadata and
portals; preview cache handling is a separate editor concern.

On a conflict the UI discards stale dialogs, clears undo/redo history and refreshes
the graph. It does not replay the failed edit. Review the current portal before
editing again. Reciprocal portals are separate saves: if the second request fails,
the UI reports how many portals were already saved, refreshes, and clears partial
history. The same rule applies to a partially completed undo or redo.

Undo of a deletion uses a server-held receipt containing the original edge,
including fractional authored geometry. Restoration revalidates map contact and
arrival collision and receives a fresh object ID. Receipts are session-local,
single-use, and bounded to the most recent 128 deletions; an expired receipt or
server restart returns a conflict and clears obsolete history. Ordinary create
and resize request coordinates still require integer pixels.

Run the frontend behavioral checks with `npm test` (Node's built-in test runner;
no additional test dependencies), alongside `npm ci` and `npm run build`.
