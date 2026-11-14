//! Document scan: walks the `<map>` children and dispatches to the tile layer and object group parsers.

use super::*;

pub(super) fn scan_tmx_map_document(
    xml: &str,
    logical_tmx_path: &ScenarioRelativePath,
) -> Result<TmxMapScan, TmxMapDocumentError> {
    let mut reader = Reader::from_str(xml);
    let (root, root_offset, root_is_empty) = loop {
        let offset = reader.buffer_position();
        match reader
            .read_event()
            .map_err(|error| TmxMapDocumentError::new(offset, format!("malformed XML: {error}")))?
        {
            Event::Decl(_) | Event::Comment(_) | Event::PI(_) | Event::DocType(_) => {}
            Event::Text(text) if text.as_ref().iter().all(u8::is_ascii_whitespace) => {}
            Event::Start(element) => break (element, offset, false),
            Event::Empty(element) => break (element, offset, true),
            Event::Eof => {
                return Err(TmxMapDocumentError::new(
                    offset,
                    "missing `map` root element",
                ));
            }
            _ => {
                return Err(TmxMapDocumentError::new(
                    offset,
                    "root element must be `map`",
                ));
            }
        }
    };
    if root.name().as_ref() != b"map" {
        return Err(TmxMapDocumentError::new(
            root_offset,
            "root element must be `map`",
        ));
    }
    let header = parse_map_attributes(&reader, &root, root_offset)
        .map_err(TmxMapDocumentError::from_header)?;
    let mut scan = TmxMapScan {
        document: TmxMapDocument {
            header,
            external_tilesets: Vec::new(),
            tile_layers: Vec::new(),
            object_groups: Vec::new(),
        },
        first_inline_error: None,
        inline_tilesets: 0,
    };
    if root_is_empty {
        require_document_end(&mut reader)?;
        return Ok(scan);
    }

    let mut depth = 1_u32;
    let mut normalized_sources = HashSet::new();
    let mut object_group_ids = HashSet::new();
    let mut object_group_names = HashSet::new();
    let mut object_ids = HashSet::new();
    loop {
        let offset = reader.buffer_position();
        match reader
            .read_event()
            .map_err(|error| TmxMapDocumentError::new(offset, format!("malformed XML: {error}")))?
        {
            Event::Start(element) => {
                if element.name().as_ref() == b"objectgroup" {
                    require_root_object_group(depth, offset)?;
                    let group = parse_object_group(&mut reader, &element, offset, &mut object_ids)?;
                    push_object_group(
                        &mut scan.document,
                        group,
                        &mut object_group_ids,
                        &mut object_group_names,
                        offset,
                    )?;
                    continue;
                }
                if element.name().as_ref() == b"object" {
                    return Err(TmxMapDocumentError::new(
                        offset,
                        "`object` must be a direct child of `objectgroup`",
                    ));
                }
                if element.name().as_ref() == b"layer" {
                    require_root_layer(depth, offset)?;
                    let layer = parse_tile_layer(&mut reader, &element, offset, header)?;
                    scan.document.tile_layers.push(layer);
                    continue;
                }
                if element.name().as_ref() == b"tileset" {
                    require_root_tileset(depth, offset)?;
                    match parse_external_tileset(&reader, &element, offset, logical_tmx_path) {
                        Ok(reference) => {
                            validate_tileset_order(
                                &scan.document,
                                &normalized_sources,
                                &reference,
                                offset,
                            )?;
                            return Err(TmxMapDocumentError::new(
                                offset,
                                "external `tileset` reference must be an empty element",
                            ));
                        }
                        Err(error) if error.is_inline_tileset() => {
                            scan.inline_tilesets += 1;
                            scan.first_inline_error.get_or_insert(error);
                        }
                        Err(error) => return Err(error),
                    }
                }
                depth += 1;
            }
            Event::Empty(element) => {
                if element.name().as_ref() == b"objectgroup" {
                    require_root_object_group(depth, offset)?;
                    let group = parse_empty_object_group(&reader, &element, offset)?;
                    push_object_group(
                        &mut scan.document,
                        group,
                        &mut object_group_ids,
                        &mut object_group_names,
                        offset,
                    )?;
                    continue;
                }
                if element.name().as_ref() == b"object" {
                    return Err(TmxMapDocumentError::new(
                        offset,
                        "`object` must be a direct child of `objectgroup`",
                    ));
                }
                if element.name().as_ref() == b"layer" {
                    require_root_layer(depth, offset)?;
                    return Err(TmxMapDocumentError::new(
                        offset,
                        "tile `layer` must contain one CSV `data` element",
                    ));
                }
                if element.name().as_ref() == b"tileset" {
                    require_root_tileset(depth, offset)?;
                    match parse_external_tileset(&reader, &element, offset, logical_tmx_path) {
                        Ok(reference) => {
                            validate_tileset_order(
                                &scan.document,
                                &normalized_sources,
                                &reference,
                                offset,
                            )?;
                            normalized_sources.insert(reference.source.clone());
                            scan.document.external_tilesets.push(reference);
                        }
                        Err(error) if error.is_inline_tileset() => {
                            scan.inline_tilesets += 1;
                            scan.first_inline_error.get_or_insert(error);
                        }
                        Err(error) => return Err(error),
                    }
                }
            }
            Event::End(_) if depth == 1 => {
                require_document_end(&mut reader)?;
                return Ok(scan);
            }
            Event::End(_) => depth -= 1,
            Event::Eof => {
                return Err(TmxMapDocumentError::new(
                    offset,
                    "unexpected end of XML before `map` closed",
                ));
            }
            _ => {}
        }
    }
}

/// Test-only tolerant scan used by shipped-content audits that must inventory external references
/// even in the one map that also contains an unsupported inline tileset.
#[cfg(test)]
pub fn scan_tmx_external_tilesets_for_test(
    xml: &str,
    logical_tmx_path: &ScenarioRelativePath,
) -> Result<TmxMapDocument, TmxMapDocumentError> {
    Ok(scan_tmx_map_document(xml, logical_tmx_path)?.document)
}

fn require_root_tileset(depth: u32, offset: u64) -> Result<(), TmxMapDocumentError> {
    if depth == 1 {
        Ok(())
    } else {
        Err(TmxMapDocumentError::new(
            offset,
            "`tileset` must be a direct child of `map`",
        ))
    }
}

fn require_root_layer(depth: u32, offset: u64) -> Result<(), TmxMapDocumentError> {
    if depth == 1 {
        Ok(())
    } else {
        Err(TmxMapDocumentError::new(
            offset,
            "tile `layer` must be a direct child of `map`",
        ))
    }
}

fn require_root_object_group(depth: u32, offset: u64) -> Result<(), TmxMapDocumentError> {
    if depth == 1 {
        Ok(())
    } else {
        Err(TmxMapDocumentError::new(
            offset,
            "`objectgroup` must be a direct child of `map`",
        ))
    }
}

fn push_object_group(
    document: &mut TmxMapDocument,
    mut group: TmxObjectGroup,
    ids: &mut HashSet<u32>,
    names: &mut HashSet<String>,
    offset: u64,
) -> Result<(), TmxMapDocumentError> {
    validate_object_group_identity(&group, ids, names, offset)?;
    group.tile_layer_index = document.tile_layers.len();
    document.object_groups.push(group);
    Ok(())
}

fn validate_object_group_identity(
    group: &TmxObjectGroup,
    ids: &mut HashSet<u32>,
    names: &mut HashSet<String>,
    offset: u64,
) -> Result<(), TmxMapDocumentError> {
    if !ids.insert(group.id) {
        return Err(TmxMapDocumentError::new(
            offset,
            format!("duplicate object-group ID {}", group.id),
        ));
    }
    if !names.insert(group.name.clone()) {
        return Err(TmxMapDocumentError::new(
            offset,
            format!("duplicate object-group name `{}`", group.name),
        ));
    }
    Ok(())
}

fn parse_external_tileset(
    reader: &Reader<&[u8]>,
    element: &quick_xml::events::BytesStart<'_>,
    offset: u64,
    logical_tmx_path: &ScenarioRelativePath,
) -> Result<TmxExternalTileset, TmxMapDocumentError> {
    let mut first_gid = None;
    let mut source = None;
    let mut unsupported_attribute = None;
    let mut seen = HashSet::new();
    for attribute in element.attributes().with_checks(false) {
        let attribute = attribute.map_err(|error| {
            TmxMapDocumentError::new(offset, format!("invalid attribute: {error}"))
        })?;
        let name = str::from_utf8(attribute.key.as_ref())
            .map_err(|_| TmxMapDocumentError::new(offset, "attribute names must be UTF-8"))?;
        if !seen.insert(name.to_owned()) {
            return Err(TmxMapDocumentError::new(
                offset,
                format!("duplicate `{name}` attribute"),
            ));
        }
        let value = attribute
            .decoded_and_normalized_value(XmlVersion::Implicit1_0, reader.decoder())
            .map_err(|error| {
                TmxMapDocumentError::new(offset, format!("invalid `{name}` attribute: {error}"))
            })?
            .into_owned();
        match name {
            "firstgid" => first_gid = Some(value),
            "source" => source = Some(value),
            _ => unsupported_attribute = Some(name.to_owned()),
        }
    }

    let source = source.ok_or_else(|| {
        TmxMapDocumentError::new(offset, "inline `tileset` without `source` is unsupported")
    })?;
    if source.trim().is_empty() {
        return Err(TmxMapDocumentError::new(
            offset,
            "invalid `source` attribute: TSX path must not be empty",
        ));
    }
    if let Some(name) = unsupported_attribute {
        return Err(TmxMapDocumentError::new(
            offset,
            format!("unsupported `tileset` attribute `{name}`"),
        ));
    }
    let first_gid = first_gid
        .ok_or_else(|| TmxMapDocumentError::new(offset, "missing required `firstgid` attribute"))?;
    let first_gid = first_gid.parse::<u32>().map_err(|_| {
        TmxMapDocumentError::new(
            offset,
            format!("invalid `firstgid` attribute `{first_gid}`; expected u32"),
        )
    })?;
    if first_gid == 0 {
        return Err(TmxMapDocumentError::new(
            offset,
            "invalid `firstgid` attribute `0`; expected positive u32",
        ));
    }
    let source = logical_tmx_path
        .resolve_from_file(&source)
        .map_err(|error| {
            TmxMapDocumentError::new(offset, format!("invalid `source` attribute: {error}"))
        })?;
    if !source.as_str().ends_with(".tsx") {
        return Err(TmxMapDocumentError::new(
            offset,
            format!("invalid `source` attribute `{source}`; expected a `.tsx` path"),
        ));
    }
    Ok(TmxExternalTileset { first_gid, source })
}

fn validate_tileset_order(
    document: &TmxMapDocument,
    normalized_sources: &HashSet<ScenarioRelativePath>,
    reference: &TmxExternalTileset,
    offset: u64,
) -> Result<(), TmxMapDocumentError> {
    if let Some(previous) = document.external_tilesets.last() {
        if reference.first_gid == previous.first_gid {
            return Err(TmxMapDocumentError::new(
                offset,
                format!("duplicate `firstgid` {}", reference.first_gid),
            ));
        }
        if reference.first_gid < previous.first_gid {
            return Err(TmxMapDocumentError::new(
                offset,
                format!(
                    "`firstgid` {} must be greater than preceding `firstgid` {}",
                    reference.first_gid, previous.first_gid
                ),
            ));
        }
    }
    if normalized_sources.contains(&reference.source) {
        return Err(TmxMapDocumentError::new(
            offset,
            format!("duplicate external tileset source `{}`", reference.source),
        ));
    }
    Ok(())
}

fn require_document_end(reader: &mut Reader<&[u8]>) -> Result<(), TmxMapDocumentError> {
    loop {
        let offset = reader.buffer_position();
        match reader
            .read_event()
            .map_err(|error| TmxMapDocumentError::new(offset, format!("malformed XML: {error}")))?
        {
            Event::Comment(_) | Event::PI(_) => {}
            Event::Text(text) if text.as_ref().iter().all(u8::is_ascii_whitespace) => {}
            Event::Eof => return Ok(()),
            _ => {
                return Err(TmxMapDocumentError::new(
                    offset,
                    "unexpected content after `map` root element",
                ));
            }
        }
    }
}
