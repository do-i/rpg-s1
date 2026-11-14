//! Object group parsing: rectangle objects and their typed properties.

use super::*;

pub(super) fn parse_empty_object_group(
    reader: &Reader<&[u8]>,
    element: &quick_xml::events::BytesStart<'_>,
    offset: u64,
) -> Result<TmxObjectGroup, TmxMapDocumentError> {
    let (id, name, _) = parse_object_group_attributes(reader, element, offset)?;
    Ok(TmxObjectGroup {
        id,
        name,
        objects: Vec::new(),
        tile_layer_index: 0,
    })
}

pub(super) fn parse_object_group(
    reader: &mut Reader<&[u8]>,
    element: &quick_xml::events::BytesStart<'_>,
    offset: u64,
    object_ids: &mut HashSet<u32>,
) -> Result<TmxObjectGroup, TmxMapDocumentError> {
    let (id, name, [offset_x, offset_y]) = parse_object_group_attributes(reader, element, offset)?;
    let mut objects = Vec::new();
    loop {
        let child_offset = reader.buffer_position();
        match reader.read_event().map_err(|error| {
            TmxMapDocumentError::new(child_offset, format!("malformed XML: {error}"))
        })? {
            Event::Start(child) if child.name().as_ref() == b"object" => {
                let object = parse_rectangle_object(reader, &child, child_offset)?;
                if name != "item_boxes" && object.gid.is_some() {
                    return Err(TmxMapDocumentError::new(
                        child_offset,
                        "unsupported rectangle `object` attribute `gid`",
                    ));
                }
                validate_object_id(object.id, object_ids, child_offset)?;
                if name == "portals" {
                    validate_portal_properties(&object, child_offset)?;
                }
                objects.push(object);
            }
            Event::Empty(child) if child.name().as_ref() == b"object" => {
                let object = parse_empty_rectangle_object(reader, &child, child_offset)?;
                if name != "item_boxes" && object.gid.is_some() {
                    return Err(TmxMapDocumentError::new(
                        child_offset,
                        "unsupported rectangle `object` attribute `gid`",
                    ));
                }
                validate_object_id(object.id, object_ids, child_offset)?;
                if name == "portals" {
                    validate_portal_properties(&object, child_offset)?;
                }
                objects.push(object);
            }
            Event::Start(child) | Event::Empty(child) => {
                let child_name = str::from_utf8(child.name().as_ref())
                    .unwrap_or("non-UTF-8")
                    .to_owned();
                return Err(TmxMapDocumentError::new(
                    child_offset,
                    format!("unsupported `{child_name}` child in `objectgroup`"),
                ));
            }
            Event::Text(text) if text.as_ref().iter().all(u8::is_ascii_whitespace) => {}
            Event::Comment(_) => {}
            Event::End(end) if end.name().as_ref() == b"objectgroup" => {
                // Normalize once so every consumer uses the same map-space coordinates.
                for object in &mut objects {
                    object.x += offset_x;
                    object.y += offset_y;
                    if !object.x.is_finite() || !object.y.is_finite() {
                        return Err(TmxMapDocumentError::new(
                            offset,
                            format!(
                                "object {} position overflows after object-group offset",
                                object.id
                            ),
                        ));
                    }
                }
                return Ok(TmxObjectGroup {
                    id,
                    name,
                    objects,
                    tile_layer_index: 0,
                });
            }
            Event::End(_) => {
                return Err(TmxMapDocumentError::new(
                    child_offset,
                    "unexpected closing element in `objectgroup`",
                ));
            }
            Event::Eof => {
                return Err(TmxMapDocumentError::new(
                    child_offset,
                    "unexpected end of XML before `objectgroup` closed",
                ));
            }
            _ => {
                return Err(TmxMapDocumentError::new(
                    child_offset,
                    "`objectgroup` may contain rectangle `object` elements only",
                ));
            }
        }
    }
}

fn parse_object_group_attributes(
    reader: &Reader<&[u8]>,
    element: &quick_xml::events::BytesStart<'_>,
    offset: u64,
) -> Result<(u32, String, [f64; 2]), TmxMapDocumentError> {
    let attributes = collect_document_attributes(reader, element, offset)?;
    if let Some(name) = attributes.keys().find(|name| {
        !matches!(
            name.as_str(),
            "id" | "name" | "visible" | "offsetx" | "offsety"
        )
    }) {
        return Err(TmxMapDocumentError::new(
            offset,
            format!("unsupported `objectgroup` attribute `{name}`"),
        ));
    }
    // Editor visibility does not disable gameplay objects such as portal triggers.
    document_visibility(&attributes, offset)?;
    let id = positive_document_u32(&attributes, "id", offset)?;
    let name = required_document_attribute(&attributes, "name", offset)?;
    if !matches!(name, "portals" | "boss_enemy" | "item_boxes") {
        return Err(TmxMapDocumentError::new(
            offset,
            format!("unsupported object-group name `{name}`"),
        ));
    }
    let mut translation = [0.0; 2];
    for (axis, attribute) in ["offsetx", "offsety"].into_iter().enumerate() {
        if attributes.contains_key(attribute) {
            translation[axis] = finite_document_f64(&attributes, attribute, offset)?;
        }
    }
    Ok((id, name.to_owned(), translation))
}

fn validate_object_id(
    id: u32,
    object_ids: &mut HashSet<u32>,
    offset: u64,
) -> Result<(), TmxMapDocumentError> {
    if object_ids.insert(id) {
        Ok(())
    } else {
        Err(TmxMapDocumentError::new(
            offset,
            format!("duplicate object ID {id}"),
        ))
    }
}

fn validate_portal_properties(
    object: &TmxRectangleObject,
    offset: u64,
) -> Result<(), TmxMapDocumentError> {
    // Editor markers in the `portals` group carry no properties and are skipped;
    // property-bearing entries are the runtime portals and must have the exact domain schema.
    if object.properties.is_empty() {
        return Ok(());
    }
    let target_map = object
        .properties
        .iter()
        .find(|property| property.name == "target_map");
    let target_x = object
        .properties
        .iter()
        .find(|property| property.name == "target_position_x");
    let target_y = object
        .properties
        .iter()
        .find(|property| property.name == "target_position_y");
    if object.properties.len() != 3
        || !target_map.is_some_and(|property| {
            matches!(property.value, TmxPropertyValue::String(ref value) if !value.is_empty())
        })
        || !target_x.is_some_and(|property| {
            matches!(property.value, TmxPropertyValue::Integer(_))
        })
        || !target_y.is_some_and(|property| {
            matches!(property.value, TmxPropertyValue::Integer(_))
        })
    {
        return Err(TmxMapDocumentError::new(
            offset,
            format!(
                "portal object {} must have exactly string `target_map` and integer `target_position_x`/`target_position_y` properties",
                object.id
            ),
        ));
    }
    Ok(())
}

fn parse_empty_rectangle_object(
    reader: &Reader<&[u8]>,
    element: &quick_xml::events::BytesStart<'_>,
    offset: u64,
) -> Result<TmxRectangleObject, TmxMapDocumentError> {
    parse_rectangle_object_attributes(reader, element, offset, Vec::new())
}

fn parse_rectangle_object(
    reader: &mut Reader<&[u8]>,
    element: &quick_xml::events::BytesStart<'_>,
    offset: u64,
) -> Result<TmxRectangleObject, TmxMapDocumentError> {
    let mut properties = None;
    loop {
        let child_offset = reader.buffer_position();
        match reader.read_event().map_err(|error| {
            TmxMapDocumentError::new(child_offset, format!("malformed XML: {error}"))
        })? {
            Event::Start(child) if child.name().as_ref() == b"properties" => {
                if properties.is_some() {
                    return Err(TmxMapDocumentError::new(
                        child_offset,
                        "rectangle `object` may contain at most one `properties` element",
                    ));
                }
                properties = Some(parse_typed_properties(reader, &child, child_offset)?);
            }
            Event::Empty(child) if child.name().as_ref() == b"properties" => {
                if properties.is_some() {
                    return Err(TmxMapDocumentError::new(
                        child_offset,
                        "rectangle `object` may contain at most one `properties` element",
                    ));
                }
                require_no_attributes(reader, &child, child_offset, "properties")?;
                properties = Some(Vec::new());
            }
            Event::Start(child) | Event::Empty(child) => {
                let child_name = str::from_utf8(child.name().as_ref())
                    .unwrap_or("non-UTF-8")
                    .to_owned();
                return Err(TmxMapDocumentError::new(
                    child_offset,
                    format!("unsupported rectangle-object child `{child_name}`"),
                ));
            }
            Event::Text(text) if text.as_ref().iter().all(u8::is_ascii_whitespace) => {}
            Event::Comment(_) => {}
            Event::End(end) if end.name().as_ref() == b"object" => {
                return parse_rectangle_object_attributes(
                    reader,
                    element,
                    offset,
                    properties.unwrap_or_default(),
                );
            }
            Event::End(_) => {
                return Err(TmxMapDocumentError::new(
                    child_offset,
                    "unexpected closing element in rectangle `object`",
                ));
            }
            Event::Eof => {
                return Err(TmxMapDocumentError::new(
                    child_offset,
                    "unexpected end of XML before rectangle `object` closed",
                ));
            }
            _ => {
                return Err(TmxMapDocumentError::new(
                    child_offset,
                    "rectangle `object` may contain `properties` only",
                ));
            }
        }
    }
}

fn parse_rectangle_object_attributes(
    reader: &Reader<&[u8]>,
    element: &quick_xml::events::BytesStart<'_>,
    offset: u64,
    properties: Vec<TmxProperty>,
) -> Result<TmxRectangleObject, TmxMapDocumentError> {
    let attributes = collect_document_attributes(reader, element, offset)?;
    if let Some(name) = attributes.keys().find(|name| {
        !matches!(
            name.as_str(),
            "id" | "name" | "x" | "y" | "width" | "height" | "visible" | "gid"
        )
    }) {
        return Err(TmxMapDocumentError::new(
            offset,
            format!("unsupported rectangle `object` attribute `{name}`"),
        ));
    }
    document_visibility(&attributes, offset)?;
    let id = positive_document_u32(&attributes, "id", offset)?;
    let x = finite_document_f64(&attributes, "x", offset)?;
    let y = finite_document_f64(&attributes, "y", offset)?;
    let width = optional_nonnegative_document_f64(&attributes, "width", offset)?;
    let height = optional_nonnegative_document_f64(&attributes, "height", offset)?;
    Ok(TmxRectangleObject {
        id,
        name: attributes.get("name").cloned(),
        x,
        y,
        width,
        height,
        properties,
        gid: attributes
            .get("gid")
            .map(|raw| {
                let raw = raw
                    .parse::<u32>()
                    .map_err(|_| TmxMapDocumentError::new(offset, "invalid tile-object gid"))?;
                let gid = TmxTileGid::decode_orthogonal(raw).map_err(|_| {
                    TmxMapDocumentError::new(offset, "invalid tile-object gid flags")
                })?;
                if gid.is_empty() {
                    return Err(TmxMapDocumentError::new(
                        offset,
                        "tile-object gid must be nonzero",
                    ));
                }
                Ok(gid)
            })
            .transpose()?,
    })
}
