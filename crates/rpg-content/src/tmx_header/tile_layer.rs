//! Tile layer parsing: CSV data decoding and GID transform handling.

use super::*;

pub(super) fn parse_tile_layer(
    reader: &mut Reader<&[u8]>,
    element: &quick_xml::events::BytesStart<'_>,
    offset: u64,
    map_header: TmxMapHeader,
) -> Result<TmxTileLayer, TmxMapDocumentError> {
    let attributes = collect_document_attributes(reader, element, offset)?;
    let id = positive_document_u32(&attributes, "id", offset)?;
    let name = required_document_attribute(&attributes, "name", offset)?;
    if name.trim().is_empty() {
        return Err(TmxMapDocumentError::new(
            offset,
            "invalid `name` attribute: layer name must not be empty",
        ));
    }
    let width = positive_document_u32(&attributes, "width", offset)?;
    let height = positive_document_u32(&attributes, "height", offset)?;
    let visible = document_visibility(&attributes, offset)?;
    if width != map_header.width || height != map_header.height {
        return Err(TmxMapDocumentError::new(
            offset,
            format!(
                "tile layer `{name}` dimensions {width}x{height} do not match map dimensions {}x{}",
                map_header.width, map_header.height
            ),
        ));
    }

    let mut csv = None;
    let mut properties = None;
    let mut depth = 1_u32;
    loop {
        let child_offset = reader.buffer_position();
        match reader.read_event().map_err(|error| {
            TmxMapDocumentError::new(child_offset, format!("malformed XML: {error}"))
        })? {
            Event::Start(child) if child.name().as_ref() == b"properties" && depth == 1 => {
                if properties.is_some() {
                    return Err(TmxMapDocumentError::new(
                        child_offset,
                        "tile layer has multiple `properties` elements",
                    ));
                }
                properties = Some(parse_typed_properties(reader, &child, child_offset)?);
            }
            Event::Empty(child) if child.name().as_ref() == b"properties" && depth == 1 => {
                require_no_attributes(reader, &child, child_offset, "properties")?;
                if properties.replace(Vec::new()).is_some() {
                    return Err(TmxMapDocumentError::new(
                        child_offset,
                        "tile layer has multiple `properties` elements",
                    ));
                }
            }
            Event::Start(child) if child.name().as_ref() == b"data" => {
                if depth != 1 {
                    return Err(TmxMapDocumentError::new(
                        child_offset,
                        "`data` must be a direct child of tile `layer`",
                    ));
                }
                if csv.is_some() {
                    return Err(TmxMapDocumentError::new(
                        child_offset,
                        "tile `layer` must contain exactly one `data` element",
                    ));
                }
                validate_csv_data_attributes(reader, &child, child_offset)?;
                csv = Some(read_csv_data(reader, child_offset)?);
            }
            Event::Empty(child) if child.name().as_ref() == b"data" => {
                if depth != 1 {
                    return Err(TmxMapDocumentError::new(
                        child_offset,
                        "`data` must be a direct child of tile `layer`",
                    ));
                }
                if csv.is_some() {
                    return Err(TmxMapDocumentError::new(
                        child_offset,
                        "tile `layer` must contain exactly one `data` element",
                    ));
                }
                validate_csv_data_attributes(reader, &child, child_offset)?;
                csv = Some(String::new());
            }
            Event::Start(_) => depth += 1,
            Event::End(_) if depth == 1 => {
                let csv = csv.ok_or_else(|| {
                    TmxMapDocumentError::new(
                        offset,
                        "tile `layer` must contain one CSV `data` element",
                    )
                })?;
                let gids = parse_csv_gids(&csv, width, height, offset, name)?;
                let sort_base_y_overrides = parse_sort_base_y_overrides(
                    properties.as_deref().unwrap_or_default(),
                    width,
                    height,
                    &gids,
                    offset,
                )?;
                return Ok(TmxTileLayer {
                    id,
                    name: name.to_owned(),
                    width,
                    height,
                    visible,
                    sort_base_y_overrides,
                    gids,
                });
            }
            Event::End(_) => depth -= 1,
            Event::Eof => {
                return Err(TmxMapDocumentError::new(
                    child_offset,
                    "unexpected end of XML before tile `layer` closed",
                ));
            }
            _ => {}
        }
    }
}

/// Whitespace-separated `column,row:pixel_offset` entries on the existing foreground layer.
fn parse_sort_base_y_overrides(
    properties: &[TmxProperty],
    width: u32,
    height: u32,
    gids: &[TmxTileGid],
    offset: u64,
) -> Result<BTreeMap<(u32, u32), u32>, TmxMapDocumentError> {
    let mut overrides = BTreeMap::new();
    let Some(property) = properties
        .iter()
        .find(|p| p.name == "sort_base_y_overrides")
    else {
        return Ok(overrides);
    };
    let TmxPropertyValue::String(entries) = &property.value else {
        return Err(TmxMapDocumentError::new(
            offset,
            "layer `sort_base_y_overrides` must have type `string`",
        ));
    };
    for entry in entries.split_whitespace() {
        let invalid = || {
            TmxMapDocumentError::new(
                offset,
                format!(
                    "invalid `sort_base_y_overrides` entry `{entry}`: expected unique in-bounds column,row:offset with nonnegative u32 values"
                ),
            )
        };
        let (cell, baseline) = entry.split_once(':').ok_or_else(invalid)?;
        let (column, row) = cell.split_once(',').ok_or_else(invalid)?;
        let column = column.parse::<u32>().map_err(|_| invalid())?;
        let row = row.parse::<u32>().map_err(|_| invalid())?;
        let baseline = baseline.parse::<u32>().map_err(|_| invalid())?;
        if column >= width || row >= height || overrides.insert((column, row), baseline).is_some() {
            return Err(invalid());
        }
    }
    // Tiled preserves layer properties when tiles are erased or moved. Stale
    // overrides on empty cells have no effect and must not prevent map loading.
    overrides.retain(|&(column, row), _| {
        !gids[(u64::from(row) * u64::from(width) + u64::from(column)) as usize].is_empty()
    });
    Ok(overrides)
}

fn validate_csv_data_attributes(
    reader: &Reader<&[u8]>,
    element: &quick_xml::events::BytesStart<'_>,
    offset: u64,
) -> Result<(), TmxMapDocumentError> {
    let attributes = collect_document_attributes(reader, element, offset)?;
    let encoding = required_document_attribute(&attributes, "encoding", offset)?;
    if encoding != "csv" {
        return Err(TmxMapDocumentError::new(
            offset,
            format!("unsupported tile data encoding `{encoding}`; expected `csv`"),
        ));
    }
    if let Some(compression) = attributes.get("compression") {
        return Err(TmxMapDocumentError::new(
            offset,
            format!("compression `{compression}` is unsupported for CSV tile data"),
        ));
    }
    if let Some(name) = attributes.keys().find(|name| name.as_str() != "encoding") {
        return Err(TmxMapDocumentError::new(
            offset,
            format!("unsupported `data` attribute `{name}`"),
        ));
    }
    Ok(())
}

fn read_csv_data(
    reader: &mut Reader<&[u8]>,
    data_offset: u64,
) -> Result<String, TmxMapDocumentError> {
    let mut csv = String::new();
    loop {
        let offset = reader.buffer_position();
        match reader
            .read_event()
            .map_err(|error| TmxMapDocumentError::new(offset, format!("malformed XML: {error}")))?
        {
            Event::Text(text) => csv
                .push_str(str::from_utf8(text.as_ref()).map_err(|_| {
                    TmxMapDocumentError::new(offset, "CSV tile data must be UTF-8")
                })?),
            Event::CData(data) => csv
                .push_str(str::from_utf8(data.as_ref()).map_err(|_| {
                    TmxMapDocumentError::new(offset, "CSV tile data must be UTF-8")
                })?),
            Event::Comment(_) => {}
            Event::End(_) => return Ok(csv),
            Event::Eof => {
                return Err(TmxMapDocumentError::new(
                    offset,
                    "unexpected end of XML before tile `data` closed",
                ));
            }
            _ => {
                return Err(TmxMapDocumentError::new(
                    data_offset,
                    "CSV tile `data` must contain text only",
                ));
            }
        }
    }
}

fn parse_csv_gids(
    csv: &str,
    width: u32,
    height: u32,
    offset: u64,
    layer_name: &str,
) -> Result<Vec<TmxTileGid>, TmxMapDocumentError> {
    let rows = csv
        .lines()
        .map(str::trim)
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    if rows.len() != height as usize {
        return Err(TmxMapDocumentError::new(
            offset,
            format!(
                "tile layer `{layer_name}` has {} CSV rows; expected {height}",
                rows.len()
            ),
        ));
    }

    let capacity = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .ok_or_else(|| TmxMapDocumentError::new(offset, "tile layer dimensions are too large"))?;
    let mut gids = Vec::with_capacity(capacity);
    for (row_index, row) in rows.iter().enumerate() {
        let mut cells = row.split(',').map(str::trim).collect::<Vec<_>>();
        if cells.last() == Some(&"") {
            cells.pop();
        }
        if cells.len() != width as usize {
            return Err(TmxMapDocumentError::new(
                offset,
                format!(
                    "tile layer `{layer_name}` CSV row {} has {} columns; expected {width}",
                    row_index + 1,
                    cells.len()
                ),
            ));
        }
        for (column_index, cell) in cells.into_iter().enumerate() {
            let gid = cell.parse::<u32>().map_err(|_| {
                TmxMapDocumentError::new(
                    offset,
                    format!(
                        "tile layer `{layer_name}` has invalid GID `{cell}` at row {}, column {}; expected u32",
                        row_index + 1,
                        column_index + 1
                    ),
                )
            })?;
            let gid = TmxTileGid::decode_orthogonal(gid).map_err(|error| match error {
                TmxTileGidError::Unsupported120DegreeRotation => TmxMapDocumentError::new(
                    offset,
                    format!(
                        "tile layer `{layer_name}` has unsupported 120-degree rotation flag at row {}, column {}",
                        row_index + 1,
                        column_index + 1
                    ),
                ),
            })?;
            gids.push(gid);
        }
    }
    Ok(gids)
}
