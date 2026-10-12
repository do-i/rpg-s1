"""Portal input checks shared by create and retarget, before publication."""

from math import isfinite
from pathlib import Path
from xml.etree import ElementTree as ET


I32_MIN = -(2**31)
I32_MAX = 2**31 - 1
U32_MAX = 2**32 - 1


class PortalValidationError(ValueError):
    def __init__(self, field: str, message: str):
        self.field = field
        super().__init__(f"{field}: {message}")


def _integers(values, field: str, count: int) -> tuple[int, ...]:
    if not isinstance(values, (tuple, list)) or len(values) != count:
        raise PortalValidationError(field, f"expected {count} integer coordinates")
    for index, value in enumerate(values):
        if type(value) is not int:
            raise PortalValidationError(f"{field}[{index}]", "expected an integer")
        if not I32_MIN <= value <= I32_MAX:
            raise PortalValidationError(f"{field}[{index}]", "outside the i32 coordinate range")
    return tuple(values)


def _map(path: Path, field: str):
    try:
        root = ET.parse(path).getroot()
        dimensions = tuple(int(root.attrib[key]) for key in (
            "width", "height", "tilewidth", "tileheight",
        ))
    except (OSError, ET.ParseError, KeyError, ValueError) as exc:
        raise PortalValidationError(field, "map dimensions could not be read") from exc
    if (root.tag != "map" or root.get("orientation") != "orthogonal"
            or root.get("infinite", "0") != "0"
            or any(not 0 < value <= U32_MAX for value in dimensions)):
        raise PortalValidationError(field, "expected a finite orthogonal map with positive dimensions")
    width, height, tw, th = dimensions
    if width * tw > I32_MAX or height * th > I32_MAX:
        raise PortalValidationError(field, "map pixels exceed the i32 coordinate range")
    return root, dimensions


def validate_source(path: Path, rect, *, authored: bool = False) -> None:
    _, (width, height, tw, th) = _map(path, "source_map")
    if authored:
        # Plain retargets keep the original finite fractional/edge-crossing TMX
        # rectangle. Request geometry is integer pixels, like the writer.
        for index, value in enumerate(rect):
            if not isfinite(value) or not I32_MIN <= value <= I32_MAX:
                raise PortalValidationError(f"source_rect_px[{index}]", "outside the finite i32 pixel range")
    else:
        rect = _integers(rect, "source_rect_px", 4)
    x, y, w, h = rect
    for index, extent in ((2, w), (3, h)):
        if extent < 0:
            raise PortalValidationError(f"source_rect_px[{index}]", "extent must be nonnegative")
    for index, end in ((0, x + w), (1, y + h)):
        if not I32_MIN <= end <= I32_MAX:
            raise PortalValidationError(f"source_rect_px[{index}]", "rectangle end exceeds the i32 pixel range")
    # Runtime portal contact is inclusive; points and edge triggers touching
    # the map are valid. Many production rectangles straddle a map boundary.
    for index, start, extent, limit in ((0, x, w, width * tw), (1, y, h, height * th)):
        if start > limit or start + extent < 0:
            raise PortalValidationError(f"source_rect_px[{index}]", f"rectangle must touch map pixels 0..{limit}")


def validate_destination(path: Path, tile) -> None:
    x, y = _integers(tile, "target_tile", 2)
    root, (width, height, _, _) = _map(path, "target_map")
    for index, value, limit in ((0, x, width), (1, y, height)):
        if not 0 <= value < limit:
            raise PortalValidationError(f"target_tile[{index}]", f"must be within map tiles 0..{limit - 1}")
    # CollisionOccupancy requires one exact CSV collision layer. As in the
    # runtime map sweep, an arrival must be in bounds and on an open tile.
    layers = [layer for layer in root.findall("layer") if layer.get("name") == "collision"]
    if len(layers) != 1:
        raise PortalValidationError("target_map", "arrival requires exactly one collision layer")
    layer = layers[0]
    data = layer.find("data")
    try:
        if (int(layer.get("width", "0")), int(layer.get("height", "0"))) != (width, height):
            raise ValueError("collision dimensions differ")
        if (data is None or data.get("encoding") != "csv"
                or set(data.attrib) != {"encoding"} or list(data)):
            raise ValueError("collision data must be uncompressed CSV")
        rows = []
        for row in (data.text or "").splitlines():
            if not row.strip():
                continue
            cells = [cell.strip() for cell in row.strip().split(",")]
            if cells[-1] == "":
                cells.pop()
            rows.append(cells)
        if len(rows) != height or any(len(row) != width for row in rows):
            raise ValueError("collision CSV shape differs")
        if any(not value.isascii() or not value.removeprefix("+").isdigit()
               for row in rows for value in row):
            raise ValueError("collision GIDs must be decimal u32 values")
        cells = [int(value) for row in rows for value in row]
        # Match the Rust GID domain, including its unsupported rotation flag.
        if any(not 0 <= cell <= U32_MAX or cell & 0x10000000 for cell in cells):
            raise ValueError("unsupported collision GID")
    except ValueError as exc:
        raise PortalValidationError("target_map", "collision layer is unsupported or malformed") from exc
    if cells[y * width + x] & 0x0FFFFFFF:
        raise PortalValidationError("target_tile", f"arrival tile ({x}, {y}) is blocked by collision")
