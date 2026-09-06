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
