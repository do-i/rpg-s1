"""Apply portal target changes back to TMX files.

Each call either rewrites the target properties of an existing portal object
or creates a brand-new portal object at a given tile. The first time a
particular TMX is edited within this run, a sibling .bak file is created so
the original can be restored manually if needed.
"""

from __future__ import annotations

import os
import shutil
import tempfile
from hashlib import sha256
from pathlib import Path
from xml.etree import ElementTree as ET


def save_portal_target(
    tmx_path: Path,
    portal_obj_id: int,
    new_target_map: str,
    new_target_tile: tuple[int, int],
    new_source_rect_px: tuple[int, int, int, int] | None,
    *, revisions: dict[Path, str] | None = None,
) -> int:
    """Update the named portal's target properties.

    `new_source_rect_px` is the portal's geometry (x, y, w, h) in map pixels.
    Pass it to resize/move the portal area; pass None to leave the existing
    geometry untouched (a plain retarget).

    Returns the portal's obj_id (unchanged for existing portals).
    """
    tree, revisions = _read_tmx(tmx_path, revisions)
    root = tree.getroot()
    obj = _find_portal_object(root, portal_obj_id)
    if obj is None:
        raise ValueError(
            f"Portal object id={portal_obj_id} not found in {tmx_path}"
        )
    if new_source_rect_px is not None:
        _set_object_rect(obj, new_source_rect_px)
    props = obj.find("properties")
    if props is None:
        props = ET.SubElement(obj, "properties")
    _set_prop(props, "target_map", new_target_map, None)
    _set_prop(props, "target_position_x", str(int(new_target_tile[0])), "int")
    _set_prop(props, "target_position_y", str(int(new_target_tile[1])), "int")
    _publish_tmx(tree, tmx_path, revisions)
    return portal_obj_id


def create_portal(
    tmx_path: Path,
    source_rect_px: tuple[float, float, float, float],
    new_target_map: str,
    new_target_tile: tuple[int, int],
    *, revisions: dict[Path, str] | None = None,
) -> int:
    """Append a new portal object on `tmx_path` covering the given pixel rect.

    `source_rect_px` is (x, y, w, h) in map pixels, allowing partial-tile and
    multi-tile portal areas.

    Returns the Tiled object id of the newly created portal.
    """
    tree, revisions = _read_tmx(tmx_path, revisions)
    root = tree.getroot()

    portals = _find_or_create_portals_group(root)
    new_id = _next_object_id(root)

    obj = ET.SubElement(portals, "object", attrib={"id": str(new_id)})
    _set_object_rect(obj, source_rect_px)
    props = ET.SubElement(obj, "properties")
    _set_prop(props, "target_map", new_target_map, None)
    _set_prop(props, "target_position_x", str(int(new_target_tile[0])), "int")
    _set_prop(props, "target_position_y", str(int(new_target_tile[1])), "int")

    # Tiled tracks the next-available object id on the root map element.
    root.set("nextobjectid", str(new_id + 1))
    _publish_tmx(tree, tmx_path, revisions)
    return new_id


def delete_portal(tmx_path: Path, portal_obj_id: int, *, revisions: dict[Path, str] | None = None) -> None:
    """Remove the portal object with the given id from `tmx_path`.

    Raises ValueError if no portal with that id exists.
    """
    tree, revisions = _read_tmx(tmx_path, revisions)
    root = tree.getroot()
    for group in root.findall("objectgroup"):
        if group.get("name") != "portals":
            continue
        for obj in group.findall("object"):
            if obj.get("id") == str(portal_obj_id):
                group.remove(obj)
                _publish_tmx(tree, tmx_path, revisions)
                return
    raise ValueError(f"Portal object id={portal_obj_id} not found in {tmx_path}")


def _set_object_rect(
    obj: ET.Element, rect_px: tuple[float, float, float, float]
) -> None:
    """Write the object's x/y/width/height geometry from a pixel rect."""
    x, y, w, h = rect_px
    obj.set("x", str(x))
    obj.set("y", str(y))
    obj.set("width", str(w))
    obj.set("height", str(h))


class TMXPublishedDurabilityError(OSError):
    """The new map is visible, but syncing its directory failed."""


class TMXConflictError(ValueError):
    """A map changed since the snapshot used by an edit."""


def map_revision(path: Path) -> str:
    return sha256(path.read_bytes()).hexdigest()


def check_revisions(revisions: dict[Path, str]) -> None:
    for path, expected in revisions.items():
        try:
            current = map_revision(path)
        except FileNotFoundError:
            current = None
        if current != expected:
            raise TMXConflictError(
                f"Map {path.name} changed while editing. Refresh the graph "
                "and review the current portal before making a new edit."
            )


def _read_tmx(path: Path, revisions: dict[Path, str] | None):
    data = path.read_bytes()
    revision = sha256(data).hexdigest()
    revisions = dict(revisions or {path: revision})
    if revisions.get(path, revision) != revision:
        raise TMXConflictError(f"Map {path.name} changed before the edit could be read.")
    revisions[path] = revision
    return ET.ElementTree(ET.fromstring(data)), revisions


def _publish_tmx(tree: ET.ElementTree, tmx_path: Path, revisions: dict[Path, str]) -> None:
    """Verify and sync a sibling temporary file before replacing the map."""
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="w+b", dir=tmx_path.parent,
            prefix=f".{tmx_path.name}.", suffix=".tmp", delete=False,
        ) as output:
            temporary = Path(output.name)
            tree.write(output, encoding="utf-8", xml_declaration=True)
            output.flush()
            output.seek(0)
            verified = ET.parse(output)
            if _xml_contents(verified.getroot()) != _xml_contents(tree.getroot()):
                raise ValueError("Serialized TMX does not match the intended map")
            os.fchmod(output.fileno(), tmx_path.stat().st_mode & 0o7777)
            os.fsync(output.fileno())
        check_revisions(revisions)
        _ensure_backup(tmx_path)
        check_revisions(revisions)
        os.replace(temporary, tmx_path)
        try:
            _sync_directory(tmx_path.parent)
        except OSError as exc:
            raise TMXPublishedDurabilityError(
                f"Map {tmx_path.name} was saved, but directory synchronization "
                "failed; durability after power loss is uncertain. Reload the "
                "graph before deciding whether to repeat the edit."
            ) from exc
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def _xml_contents(element: ET.Element) -> tuple:
    """Compare parsed XML independently of the serializer being verified."""
    return (
        element.tag, element.attrib, element.text, element.tail,
        tuple(_xml_contents(child) for child in element),
    )


def _ensure_backup(tmx_path: Path) -> None:
    bak = tmx_path.with_suffix(tmx_path.suffix + ".bak")
    if bak.exists():
        return
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(
            dir=tmx_path.parent, prefix=f".{bak.name}.", suffix=".tmp", delete=False,
        ) as output:
            temporary = Path(output.name)
        shutil.copy2(tmx_path, temporary)
        with temporary.open("rb") as backup:
            if backup.read() != tmx_path.read_bytes():
                raise ValueError("TMX backup does not match the original map")
            os.fsync(backup.fileno())
        # Keep the first backup, including if its name was claimed meanwhile.
        try:
            os.link(temporary, bak)
        except FileExistsError:
            pass
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)
    _sync_directory(tmx_path.parent)


def _sync_directory(directory: Path) -> None:
    descriptor = os.open(directory, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def _find_portal_object(root: ET.Element, obj_id: int) -> ET.Element | None:
    for group in root.findall("objectgroup"):
        if group.get("name") != "portals":
            continue
        for obj in group.findall("object"):
            if obj.get("id") == str(obj_id):
                return obj
    return None


def _find_or_create_portals_group(root: ET.Element) -> ET.Element:
    for group in root.findall("objectgroup"):
        if group.get("name") == "portals":
            return group
    return ET.SubElement(root, "objectgroup", attrib={"name": "portals"})


def _next_object_id(root: ET.Element) -> int:
    existing_max = 0
    for group in root.findall("objectgroup"):
        for obj in group.findall("object"):
            try:
                existing_max = max(existing_max, int(obj.get("id", "0")))
            except (TypeError, ValueError):
                pass
    declared = int(root.get("nextobjectid", "0") or 0)
    return max(declared, existing_max + 1)


def _set_prop(
    props_elem: ET.Element, name: str, value: str, prop_type: str | None
) -> None:
    """Set (creating if needed) a Tiled custom property.

    `prop_type` is the Tiled property type written as the `type` attribute
    (e.g. "int"). Pass None for the default string type, which Tiled stores
    with no `type` attribute. Integer props such as target_position_x/_y MUST
    be written with type="int" — without it pytmx reads them back as strings,
    which crashes portal traversal.
    """
    for prop in props_elem.findall("property"):
        if prop.get("name") == name:
            prop.set("value", value)
            if prop_type is None:
                prop.attrib.pop("type", None)
            else:
                prop.set("type", prop_type)
            return
    attrib = {"name": name}
    if prop_type is not None:
        attrib["type"] = prop_type
    attrib["value"] = value
    ET.SubElement(props_elem, "property", attrib=attrib)
