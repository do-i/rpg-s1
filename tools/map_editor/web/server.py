"""FastAPI backend for the web map editor.

Serves the portal graph as JSON, map renders as PNG, and portal mutations
(create / retarget / delete) over a small REST API. All domain logic lives in
`service/editor_service.py`; this module is transport only.

The built frontend (tools/map_editor_web/dist) is mounted at "/" when present,
so `python -m tools.map_editor --web --scenario ./rusted_kingdoms` is a
one-command launch. During frontend development the Vite dev server proxies to
this backend instead (see tools/map_editor_web/vite.config.ts), which is why
localhost CORS origins are allowed.
"""

from __future__ import annotations

import os
from pathlib import Path

import pygame
from fastapi import FastAPI, HTTPException
from fastapi.exceptions import RequestValidationError
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import FileResponse, JSONResponse
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel, StrictInt

from tools.map_editor.edit.tmx_writer import TMXPublishedDurabilityError
from tools.map_editor.graph.thumbnails import ThumbnailCache
from tools.map_editor.service.editor_service import EditorLookupError, EditorService
from tools.map_editor.service.portal_validation import PortalValidationError

FRONTEND_DIST = (
    Path(__file__).resolve().parent.parent.parent / "map_editor_web" / "dist"
)
DEV_CORS_ORIGINS = [
    "http://localhost:5173",
    "http://127.0.0.1:5173",
]


class PortalCreate(BaseModel):
    source_map: str
    source_rect_px: tuple[StrictInt, StrictInt, StrictInt, StrictInt]
    target_map: str
    target_tile: tuple[StrictInt, StrictInt]


class PortalPatch(BaseModel):
    target_map: str
    target_tile: tuple[StrictInt, StrictInt]
    # (x, y, w, h) in map pixels to move/resize the portal; null keeps the
    # existing on-disk geometry (a plain retarget).
    source_rect_px: tuple[StrictInt, StrictInt, StrictInt, StrictInt] | None


def _init_headless_pygame() -> None:
    """Bring up pygame with an offscreen display so TMX maps can be rendered
    to PNG. The server never opens a window, so the dummy driver is forced."""
    os.environ["SDL_VIDEODRIVER"] = "dummy"
    if not pygame.get_init():
        pygame.init()
    if pygame.display.get_surface() is None:
        pygame.display.set_mode((1, 1))


def create_app(scenario_root: Path) -> FastAPI:
    _init_headless_pygame()
    service = EditorService(scenario_root)
    thumbnails = ThumbnailCache(service.scenario_root)

    app = FastAPI(title="Map Editor", version="0.1.0")

    @app.exception_handler(RequestValidationError)
    async def invalid_request(request, error: RequestValidationError):
        # Field text is useful to the browser's error display and avoids echoing
        # nonfinite input values, which cannot be serialized as JSON responses.
        details = []
        for item in error.errors():
            location = ""
            for part in item["loc"]:
                if part in ("body", "path", "query"):
                    continue
                location += f"[{part}]" if isinstance(part, int) else ("." if location else "") + part
            details.append(f"{location or 'request'}: {item['msg']}")
        return JSONResponse(status_code=422, content={"detail": "; ".join(details)})

    app.add_middleware(
        CORSMiddleware,
        allow_origins=DEV_CORS_ORIGINS,
        allow_methods=["*"],
        allow_headers=["*"],
    )

    @app.get("/api/graph")
    def get_graph() -> dict:
        return service.graph_dict()

    @app.get("/api/maps/{map_id}/thumbnail.png")
    def get_thumbnail(map_id: str) -> FileResponse:
        return _png_response(map_id, thumbnails.thumbnail_file)

    @app.get("/api/maps/{map_id}/full.png")
    def get_full_render(map_id: str) -> FileResponse:
        return _png_response(map_id, thumbnails.full_file)

    def _png_response(map_id: str, render) -> FileResponse:
        node = _or_404(lambda: service.node(map_id))
        png = render(node.tmx_path)
        if png is None:
            raise HTTPException(
                status_code=422,
                detail=f"Map '{map_id}' failed to render ({node.tmx_path.name}).",
            )
        return FileResponse(png, media_type="image/png")

    @app.post("/api/portals", status_code=201)
    def post_portal(body: PortalCreate) -> dict:
        return _or_404(
            lambda: service.create_portal(
                source_map=body.source_map,
                source_rect_px=body.source_rect_px,
                target_map=body.target_map,
                target_tile=body.target_tile,
            )
        )

    @app.patch("/api/portals/{source_map}/{portal_obj_id}")
    def patch_portal(source_map: str, portal_obj_id: int, body: PortalPatch) -> dict:
        return _or_404(
            lambda: service.retarget_portal(
                source_map=source_map,
                portal_obj_id=portal_obj_id,
                target_map=body.target_map,
                target_tile=body.target_tile,
                source_rect_px=body.source_rect_px,
            )
        )

    @app.delete("/api/portals/{source_map}/{portal_obj_id}", status_code=204)
    def delete_portal(source_map: str, portal_obj_id: int) -> None:
        _or_404(lambda: service.delete_portal(source_map, portal_obj_id))

    if FRONTEND_DIST.is_dir():
        app.mount("/", StaticFiles(directory=FRONTEND_DIST, html=True), name="frontend")

    return app


def _or_404(action):
    """Translate domain lookup/input errors without misclassifying writer failures."""
    try:
        return action()
    except TMXPublishedDurabilityError as exc:
        raise HTTPException(status_code=500, detail=str(exc)) from exc
    except PortalValidationError as exc:
        raise HTTPException(status_code=422, detail=str(exc)) from exc
    except EditorLookupError as exc:
        raise HTTPException(status_code=404, detail=str(exc)) from exc
