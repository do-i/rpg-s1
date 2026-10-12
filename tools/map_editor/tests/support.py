"""Reusable isolated scenario fixture; no shipped maps are modified."""

from pathlib import Path
from tempfile import TemporaryDirectory

from tools.map_editor.service.editor_service import EditorService


MAP_XML = (Path(__file__).parent / "fixtures/source.tmx").read_text(encoding="utf-8")


class DisposableScenario:
    def __init__(self):
        self._temporary = TemporaryDirectory(prefix="rpg-editor-test-")
        self.root = Path(self._temporary.name)
        self.maps = self.root / "media/maps"
        self.maps.mkdir(parents=True)
        (self.root / "data/maps").mkdir(parents=True)
        (self.root / "manifest.yaml").write_text(
            "refs:\n  maps: data/maps\n  tmx: media/maps\n", encoding="utf-8"
        )
        for name in ("source", "target"):
            (self.maps / f"{name}.tmx").write_text(MAP_XML, encoding="utf-8")
        self.service = EditorService(self.root)

    @property
    def source(self):
        return self.maps / "source.tmx"

    def close(self):
        self._temporary.cleanup()
