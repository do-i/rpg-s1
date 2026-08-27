#!/usr/bin/env bash
set -euo pipefail

readonly expected_editor_commit="08970359d6cb03586948625d29b0d3351dbbf785"
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly project_root="$(cd -- "$script_dir/.." && pwd)"
readonly editor_repo="${RPG_S1_EDITOR_REPO:-$project_root/../agentic-rpg}"
readonly scenario_package="${RPG_S1_SCENARIO_PACKAGE:-rusted_kingdoms}"
readonly scenario_root="${RPG_S1_SCENARIO_ROOT:-$project_root/assets/scenarios/$scenario_package}"
readonly editor_python="${RPG_S1_EDITOR_PYTHON:-$editor_repo/.venv/bin/python}"

usage() {
    echo "Usage: scripts/map-editor.sh check|setup|pygame|web" >&2
}

require_editor_checkout() {
    if [[ ! -f "$editor_repo/tools/map_editor/__main__.py" ]]; then
        echo "Map-editor checkout not found at: $editor_repo" >&2
        echo "Clone do-i/agentic-rpg beside this repository (or set RPG_S1_EDITOR_REPO), then checkout:" >&2
        echo "  $expected_editor_commit" >&2
        exit 2
    fi
    local actual_commit
    actual_commit="$(git -C "$editor_repo" rev-parse HEAD)"
    if [[ "$actual_commit" != "$expected_editor_commit" && "${RPG_S1_EDITOR_ALLOW_UNPINNED:-0}" != "1" ]]; then
        echo "Map-editor checkout is at $actual_commit; expected $expected_editor_commit." >&2
        echo "Checkout the pinned commit or set RPG_S1_EDITOR_ALLOW_UNPINNED=1 for an intentional trial." >&2
        exit 2
    fi
    if [[ ! -f "$scenario_root/manifest.yaml" ]]; then
        echo "Scenario manifest not found at: $scenario_root/manifest.yaml" >&2
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
        cd -- "$editor_repo"
        PYTHONWARNINGS=ignore::RuntimeWarning PYGAME_HIDE_SUPPORT_PROMPT=1 \
            "$editor_python" -c 'import pygame, pytmx, yaml'
    ); then
        echo "Map-editor Python dependencies are incomplete. Run: scripts/map-editor.sh setup" >&2
        exit 2
    fi
}

setup_editor() {
    local bootstrap_python
    bootstrap_python="${RPG_S1_EDITOR_BOOTSTRAP_PYTHON:-python3}"
    if ! command -v "$bootstrap_python" >/dev/null 2>&1; then
        echo "Python 3.13+ is required; set RPG_S1_EDITOR_BOOTSTRAP_PYTHON to its executable." >&2
        exit 2
    fi
    if [[ ! -x "$editor_python" ]]; then
        "$bootstrap_python" -m venv "$editor_repo/.venv"
    fi
    (cd -- "$editor_repo" && "$editor_python" -m pip install -e '.[dev,editor]')
    if ! command -v npm >/dev/null 2>&1; then
        echo "npm is required to build the web editor frontend." >&2
        exit 2
    fi
    (cd -- "$editor_repo/tools/map_editor_web" && npm ci && npm run build)
    echo "Map-editor prerequisites are ready."
}

check_editor() {
    require_python_environment
    if [[ ! -d "$editor_repo/tools/map_editor_web/dist" ]]; then
        echo "Web frontend is not built. Run: scripts/map-editor.sh setup" >&2
        exit 2
    fi
    echo "Editor checkout: $editor_repo"
    echo "Scenario root: $scenario_root"
    echo "Editor commit: $(git -C "$editor_repo" rev-parse HEAD)"
    (
        cd -- "$editor_repo"
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
print(f"Editor graph: maps={len(service.graph.nodes)} portals={len(service.graph.edges)}")
' "$scenario_root"
    )
    echo "Prerequisites: ready"
}

readonly mode="${1:-}"
case "$mode" in
    check)
        require_editor_checkout
        check_editor
        ;;
    setup)
        require_editor_checkout
        setup_editor
        check_editor
        ;;
    pygame)
        require_editor_checkout
        require_python_environment
        cd -- "$editor_repo"
        exec "$editor_python" -m tools.map_editor --scenario "$scenario_root"
        ;;
    web)
        require_editor_checkout
        require_python_environment
        if [[ ! -d "$editor_repo/tools/map_editor_web/dist" ]]; then
            echo "Web frontend is not built. Run: scripts/map-editor.sh setup" >&2
            exit 2
        fi
        cd -- "$editor_repo"
        exec "$editor_python" -m tools.map_editor --web --scenario "$scenario_root"
        ;;
    *)
        usage
        exit 2
        ;;
esac
