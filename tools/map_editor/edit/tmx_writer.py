"""Apply portal target changes back to TMX files.

Each call either rewrites the target properties of an existing portal object
or creates a brand-new portal object at a given tile. The first time a
particular TMX is edited within this run, a sibling .bak file is created so
the original can be restored manually if needed.
"""

from __future__ import annotations

import shutil
from pathlib import Path
from xml.etree import ElementTree as ET


def save_portal_target(
    tmx_path: Path,
    portal_obj_id: int,
    new_target_map: str,
    new_target_tile: tuple[int, int],
    new_source_rect_px: tuple[int, int, int, int] | None,
) -> int:
    """Update the named portal's target properties.

    `new_source_rect_px` is the portal's geometry (x, y, w, h) in map pixels.
    Pass it to resize/move the portal area; pass None to leave the existing
    geometry untouched (a plain retarget).

    Returns the portal's obj_id (unchanged for existing portals).
    """
    _ensure_backup(tmx_path)
    tree = ET.parse(tmx_path)
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
    tree.write(tmx_path, encoding="utf-8", xml_declaration=True)
    return portal_obj_id


def create_portal(
    tmx_path: Path,
    source_rect_px: tuple[int, int, int, int],
    new_target_map: str,
    new_target_tile: tuple[int, int],
) -> int:
    """Append a new portal object on `tmx_path` covering the given pixel rect.

    `source_rect_px` is (x, y, w, h) in map pixels, allowing partial-tile and
    multi-tile portal areas.

    Returns the Tiled object id of the newly created portal.
    """
    _ensure_backup(tmx_path)
    tree = ET.parse(tmx_path)
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
    tree.write(tmx_path, encoding="utf-8", xml_declaration=True)
    return new_id


def delete_portal(tmx_path: Path, portal_obj_id: int) -> None:
    """Remove the portal object with the given id from `tmx_path`.

    Raises ValueError if no portal with that id exists.
    """
    _ensure_backup(tmx_path)
    tree = ET.parse(tmx_path)
    root = tree.getroot()
    for group in root.findall("objectgroup"):
        if group.get("name") != "portals":
            continue
        for obj in group.findall("object"):
            if obj.get("id") == str(portal_obj_id):
                group.remove(obj)
                tree.write(tmx_path, encoding="utf-8", xml_declaration=True)
                return
    raise ValueError(f"Portal object id={portal_obj_id} not found in {tmx_path}")


def _set_object_rect(
    obj: ET.Element, rect_px: tuple[int, int, int, int]
) -> None:
    """Write the object's x/y/width/height geometry from a pixel rect."""
    x, y, w, h = rect_px
    obj.set("x", str(int(x)))
    obj.set("y", str(int(y)))
    obj.set("width", str(int(w)))
    obj.set("height", str(int(h)))


def _ensure_backup(tmx_path: Path) -> None:
    bak = tmx_path.with_suffix(tmx_path.suffix + ".bak")
    if not bak.exists():
        shutil.copy2(tmx_path, bak)


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
