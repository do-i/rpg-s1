"""Entry point for the scenario map editor.

Web only. The pygame window this tool also used to offer was left behind in the `agentic-rpg`
checkout when the editor moved into this repository: it duplicated the browser UI's features
against a second rendering path, and only one of the two is maintained.
"""

from __future__ import annotations

import argparse
import threading
import webbrowser
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(
        prog="map_editor", description="Scenario map viewer/editor (browser UI)."
    )
    parser.add_argument(
        "--scenario",
        required=True,
        type=Path,
        help="Path to a scenario root containing manifest.yaml, "
        "e.g. ./assets/scenarios/rusted_kingdoms",
    )
    parser.add_argument(
        "--port",
        type=int,
        default=8017,
        help="Port for the editor backend. Default: 8017.",
    )
    parser.add_argument(
        "--no-browser",
        action="store_true",
        help="Don't open a browser tab automatically.",
    )
    args = parser.parse_args()
    _run_web(args.scenario, args.port, open_browser=not args.no_browser)


def _run_web(scenario_root: Path, port: int, open_browser: bool) -> None:
    try:
        import uvicorn
    except ImportError:
        raise SystemExit(
            "The map editor's Python dependencies are missing.\n"
            "    Run: scripts/map-editor.sh setup"
        )
    from tools.map_editor.web.server import FRONTEND_DIST, create_app

    if not FRONTEND_DIST.is_dir():
        print(
            "Note: frontend not built -- serving the API only.\n"
            "Build it once with: scripts/map-editor.sh setup"
        )
    app = create_app(scenario_root)
    url = f"http://127.0.0.1:{port}/"
    print(f"Map editor: {url}  (Ctrl+C to stop)")
    if open_browser:
        threading.Timer(0.8, webbrowser.open, args=(url,)).start()
    uvicorn.run(app, host="127.0.0.1", port=port, log_level="warning")


if __name__ == "__main__":
    main()
