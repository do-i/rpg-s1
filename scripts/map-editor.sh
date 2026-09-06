#!/usr/bin/env bash
#
# Runs the scenario map editor, which lives in this repository under tools/.
#
# It used to drive a pinned checkout of the sibling `agentic-rpg` project, and enforced an exact
# editor commit because the two repositories could drift apart. The editor is now in-tree, so
# there is no checkout to find and no commit to pin -- the version of the editor is the version
# of this repository.
#
# The editor is a browser UI. The pygame window the old script could also launch was not
# migrated.
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly project_root="$(cd -- "$script_dir/.." && pwd)"
readonly scenario_package="${RPG_S1_SCENARIO_PACKAGE:-rusted_kingdoms}"
readonly scenario_root="${RPG_S1_SCENARIO_ROOT:-$project_root/assets/scenarios/$scenario_package}"
readonly editor_venv="${RPG_S1_EDITOR_VENV:-$project_root/tools/.venv}"
readonly editor_python="$editor_venv/bin/python"
readonly frontend_dir="$project_root/tools/map_editor_web"

usage() {
    echo "Usage: scripts/map-editor.sh check|setup|web" >&2
    echo >&2
    echo "  setup  Create tools/.venv, install Python deps, build the web frontend." >&2
    echo "  check  Verify the editor can load the scenario; print what it found." >&2
    echo "  web    Serve the editor and open a browser tab." >&2
}

require_scenario() {
    if [[ ! -f "$scenario_root/manifest.yaml" ]]; then
        echo "Scenario manifest not found at: $scenario_root/manifest.yaml" >&2
        echo "Set RPG_S1_SCENARIO_ROOT to a scenario package root." >&2
        exit 2
    fi
}

require_python_environment() {
    if [[ ! -x "$editor_python" ]]; then
        echo "Map-editor Python environment is missing: $editor_python" >&2
        echo "Run: scripts/map-editor.sh setup" >&2
        exit 2
    fi
    if ! (
        cd -- "$project_root"
        PYTHONWARNINGS=ignore::RuntimeWarning PYGAME_HIDE_SUPPORT_PROMPT=1 \
            "$editor_python" -c 'import pygame, pytmx, yaml, fastapi, uvicorn'
    ); then
        echo "Map-editor Python dependencies are incomplete. Run: scripts/map-editor.sh setup" >&2
        exit 2
    fi
}

require_frontend_build() {
    if [[ ! -d "$frontend_dir/dist" ]]; then
        echo "Web frontend is not built. Run: scripts/map-editor.sh setup" >&2
        exit 2
    fi
}

setup_editor() {
    local bootstrap_python
    bootstrap_python="${RPG_S1_EDITOR_BOOTSTRAP_PYTHON:-python3}"
    if ! command -v "$bootstrap_python" >/dev/null 2>&1; then
        echo "Python 3 is required; set RPG_S1_EDITOR_BOOTSTRAP_PYTHON to its executable." >&2
        exit 2
    fi
    if [[ ! -x "$editor_python" ]]; then
        "$bootstrap_python" -m venv "$editor_venv"
    fi
    "$editor_python" -m pip install --quiet --upgrade pip
    "$editor_python" -m pip install --quiet -r "$project_root/tools/requirements.txt"
    if ! command -v npm >/dev/null 2>&1; then
        echo "npm is required to build the web editor frontend." >&2
        exit 2
    fi
    (cd -- "$frontend_dir" && npm ci && npm run build)
    echo "Map-editor prerequisites are ready."
}

# Loads the scenario through the editor's own service, so `check` fails for the same reasons the
# editor would rather than merely asserting that files exist.
check_editor() {
    require_python_environment
    require_frontend_build
    echo "Scenario root: $scenario_root"
    (
        cd -- "$project_root"
        PYTHONWARNINGS=ignore::RuntimeWarning PYGAME_HIDE_SUPPORT_PROMPT=1 \
            SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy \
            "$editor_python" -c '
from pathlib import Path
import sys
import pygame
pygame.init()
pygame.display.set_mode((1, 1))
from tools.map_editor.service.editor_service import EditorService
service = EditorService(Path(sys.argv[1]))
maps = len(service.graph.nodes)
print(f"Editor graph: maps={maps} portals={len(service.graph.edges)}")
if maps == 0:
    raise SystemExit("No maps loaded: check refs.tmx in the scenario manifest.")
' "$scenario_root"
    )
    echo "Prerequisites: ready"
}

readonly mode="${1:-}"
case "$mode" in
    check)
        require_scenario
        check_editor
        ;;
    setup)
        require_scenario
        setup_editor
        check_editor
        ;;
    web)
        require_scenario
        require_python_environment
        require_frontend_build
        cd -- "$project_root"
        shift
        exec "$editor_python" -m tools.map_editor --scenario "$scenario_root" "$@"
        ;;
    *)
        usage
        exit 2
        ;;
esac
