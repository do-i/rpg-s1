"""Reusable isolated scenario fixture; no shipped maps are modified."""

import asyncio
import json
from pathlib import Path
from tempfile import TemporaryDirectory
from urllib.parse import urlsplit
from xml.etree import ElementTree as ET

from tools.map_editor.service.editor_service import EditorService


MAP_XML = (Path(__file__).parent / "fixtures/source.tmx").read_text(encoding="utf-8")


class DisposableScenario:
    def __init__(self, source_xml=MAP_XML, target_xml=MAP_XML):
        self._temporary = TemporaryDirectory(prefix="rpg-editor-test-")
        self.root = Path(self._temporary.name)
        self.maps = self.root / "media/maps"
        self.maps.mkdir(parents=True)
        (self.root / "data/maps").mkdir(parents=True)
        (self.root / "manifest.yaml").write_text(
            "refs:\n  maps: data/maps\n  tmx: media/maps\n", encoding="utf-8"
        )
        for name, xml in (("source", source_xml), ("target", target_xml)):
            (self.maps / f"{name}.tmx").write_text(xml, encoding="utf-8")
        self.service = EditorService(self.root)

    @property
    def source(self):
        return self.maps / "source.tmx"

    def close(self):
        self._temporary.cleanup()

    def snapshot(self, service=None):
        """Canonical/backup/temp bytes and serialized graph for rejection checks."""
        return (
            {str(path.relative_to(self.root)): path.read_bytes()
             for path in self.root.rglob("*") if path.is_file()},
            json.dumps((service or self.service).graph_dict(), sort_keys=True),
        )


def map_xml(width=4, height=4, tile_width=32, tile_height=32, rect=(32, 32, 32, 32)):
    """Make a finite fixture with matching layers and authored tile dimensions."""
    root = ET.fromstring(MAP_XML)
    for key, value in (("width", width), ("height", height),
                       ("tilewidth", tile_width), ("tileheight", tile_height)):
        root.set(key, str(value))
    for layer in root.findall("layer"):
        layer.set("width", str(width))
        layer.set("height", str(height))
        layer.find("data").text = "\n" + ",\n".join(
            ",".join(["0"] * width) for _ in range(height)
        ) + "\n"
    obj = root.find("objectgroup/object")
    for key, value in zip(("x", "y", "width", "height"), rect):
        obj.set(key, str(value))
    return ET.tostring(root, encoding="unicode")


def api_request(app, method, path, body=None, *, headers=None, include_headers=False, raw=False):
    """Drive the real ASGI request/response stack without an extra HTTP client dependency."""
    async def request():
        url = urlsplit(path)
        messages = []
        payload = json.dumps(body).encode() if body is not None else b""
        delivered = False

        async def receive():
            nonlocal delivered
            if delivered:
                await asyncio.Event().wait()
            delivered = True
            return {"type": "http.request", "body": payload, "more_body": False}

        async def send(message):
            messages.append(message)

        await asyncio.wait_for(app({
            "type": "http", "asgi": {"version": "3.0", "spec_version": "2.4"}, "http_version": "1.1",
            "method": method, "scheme": "http", "path": url.path,
            "raw_path": url.path.encode(), "query_string": url.query.encode(),
            "headers": [(b"content-type", b"application/json"), *(headers or [])],
            "server": ("test", 80), "client": ("test", 1234), "root_path": "",
        }, receive, send), timeout=5)
        status = next(message["status"] for message in messages if message["type"] == "http.response.start")
        response = b"".join(message.get("body", b"") for message in messages
                            if message["type"] == "http.response.body")
        result = (status, response if raw else (json.loads(response) if response else None))
        if include_headers:
            start = next(message for message in messages if message["type"] == "http.response.start")
            return (*result, dict(start["headers"]))
        return result

    return asyncio.run(request())
