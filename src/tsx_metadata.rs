//! Strict metadata for one external, single-image TSX atlas.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "M4.03 establishes typed TSX metadata before later M4 atlas loading consumes it"
    )
)]

use std::{collections::HashSet, fmt, str};

use quick_xml::{Reader, XmlVersion, events::Event};

use crate::scenario_path::ScenarioRelativePath;

/// Validated external image metadata for the M4 single-atlas TSX profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TsxTilesetMetadata {
    tile_width: u32,
    tile_height: u32,
    columns: u32,
    tile_count: u32,
    image: TsxImageMetadata,
}

impl TsxTilesetMetadata {
    pub(crate) const fn tile_width(&self) -> u32 {
        self.tile_width
    }

    pub(crate) const fn tile_height(&self) -> u32 {
        self.tile_height
    }

    pub(crate) const fn columns(&self) -> u32 {
        self.columns
    }

    pub(crate) const fn tile_count(&self) -> u32 {
        self.tile_count
    }

    pub(crate) const fn image(&self) -> &TsxImageMetadata {
        &self.image
    }
}

/// One safely resolved external atlas image.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TsxImageMetadata {
    source: ScenarioRelativePath,
    width: u32,
    height: u32,
}

impl TsxImageMetadata {
    pub(crate) const fn source(&self) -> &ScenarioRelativePath {
        &self.source
    }

    pub(crate) const fn width(&self) -> u32 {
        self.width
    }

    pub(crate) const fn height(&self) -> u32 {
        self.height
    }
}

/// A location-safe TSX metadata failure. Byte offsets are relative to the supplied XML.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TsxMetadataError {
    offset: u64,
    detail: String,
}

impl TsxMetadataError {
    fn new(offset: u64, detail: impl Into<String>) -> Self {
        Self {
            offset,
            detail: detail.into(),
        }
    }

    #[cfg(test)]
    fn offset(&self) -> u64 {
        self.offset
    }
}

impl fmt::Display for TsxMetadataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "TSX metadata at byte {}: {}",
            self.offset, self.detail
        )
    }
}

impl std::error::Error for TsxMetadataError {}

/// Parses a complete external TSX atlas document owned by `logical_tsx_path`.
pub(crate) fn parse_tsx_tileset_metadata(
    xml: &str,
    logical_tsx_path: &ScenarioRelativePath,
) -> Result<TsxTilesetMetadata, TsxMetadataError> {
    if !logical_tsx_path.as_str().ends_with(".tsx") {
        return Err(TsxMetadataError::new(
            0,
            "logical owner path must identify a `.tsx` file",
        ));
    }
    let mut reader = Reader::from_str(xml);
    let (root, root_offset, root_is_empty) = read_root(&mut reader)?;
    if root.name().as_ref() != b"tileset" {
        return Err(TsxMetadataError::new(
            root_offset,
            "root element must be `tileset`",
        ));
    }
    let (tile_width, tile_height, columns, tile_count) =
        parse_tileset_attributes(&reader, &root, root_offset)?;
    if columns > tile_count {
        return Err(TsxMetadataError::new(
            root_offset,
            "`columns` must not exceed `tilecount`",
        ));
    }
    if root_is_empty {
        require_document_end(&mut reader)?;
        return Err(TsxMetadataError::new(
            root_offset,
            "missing required root-level `image` element",
        ));
    }

    let mut image = None;
    let mut depth = 1_u32;
    loop {
        let offset = reader.buffer_position();
        match reader
            .read_event()
            .map_err(|error| TsxMetadataError::new(offset, format!("malformed XML: {error}")))?
        {
            Event::Start(element) => {
                if element.name().as_ref() == b"image" {
                    if depth != 1 {
                        return Err(TsxMetadataError::new(
                            offset,
                            "`image` must be a direct child of `tileset`",
                        ));
                    }
                    return Err(TsxMetadataError::new(
                        offset,
                        "root-level `image` must be an empty element",
                    ));
                }
                depth += 1;
            }
            Event::Empty(element) if element.name().as_ref() == b"image" => {
                if depth != 1 {
                    return Err(TsxMetadataError::new(
                        offset,
                        "`image` must be a direct child of `tileset`",
                    ));
                }
                if image.is_some() {
                    return Err(TsxMetadataError::new(
                        offset,
                        "multiple root-level `image` elements are unsupported",
                    ));
                }
                image = Some(parse_image_attributes(
                    &reader,
                    &element,
                    offset,
                    logical_tsx_path,
                )?);
            }
            Event::Empty(_) => {}
            Event::End(_) if depth == 1 => {
                require_document_end(&mut reader)?;
                let image = image.ok_or_else(|| {
                    TsxMetadataError::new(offset, "missing required root-level `image` element")
                })?;
                validate_atlas_geometry(
                    tile_width,
                    tile_height,
                    columns,
                    tile_count,
                    &image,
                    root_offset,
                )?;
                return Ok(TsxTilesetMetadata {
                    tile_width,
                    tile_height,
                    columns,
                    tile_count,
                    image,
                });
            }
            Event::End(_) => depth -= 1,
            Event::Eof => {
                return Err(TsxMetadataError::new(
                    offset,
                    "unexpected end of XML before `tileset` closed",
                ));
            }
            _ => {}
        }
    }
}

fn read_root(
    reader: &mut Reader<&[u8]>,
) -> Result<(quick_xml::events::BytesStart<'static>, u64, bool), TsxMetadataError> {
    loop {
        let offset = reader.buffer_position();
        match reader
            .read_event()
            .map_err(|error| TsxMetadataError::new(offset, format!("malformed XML: {error}")))?
        {
            Event::Decl(_) | Event::Comment(_) | Event::PI(_) | Event::DocType(_) => {}
            Event::Text(text) if text.as_ref().iter().all(u8::is_ascii_whitespace) => {}
            Event::Start(element) => return Ok((element.into_owned(), offset, false)),
            Event::Empty(element) => return Ok((element.into_owned(), offset, true)),
            Event::Eof => {
                return Err(TsxMetadataError::new(
                    offset,
                    "missing `tileset` root element",
                ));
            }
            _ => {
                return Err(TsxMetadataError::new(
                    offset,
                    "root element must be `tileset`",
                ));
            }
        }
    }
}

fn parse_tileset_attributes(
    reader: &Reader<&[u8]>,
    element: &quick_xml::events::BytesStart<'_>,
    offset: u64,
) -> Result<(u32, u32, u32, u32), TsxMetadataError> {
    let attributes = collect_attributes(reader, element, offset)?;
    Ok((
        positive_u32(&attributes, "tilewidth", offset)?,
        positive_u32(&attributes, "tileheight", offset)?,
        positive_u32(&attributes, "columns", offset)?,
        positive_u32(&attributes, "tilecount", offset)?,
    ))
}

fn parse_image_attributes(
    reader: &Reader<&[u8]>,
    element: &quick_xml::events::BytesStart<'_>,
    offset: u64,
    logical_tsx_path: &ScenarioRelativePath,
) -> Result<TsxImageMetadata, TsxMetadataError> {
    let attributes = collect_attributes(reader, element, offset)?;
    let source = required(&attributes, "source", offset)?;
    if source.trim().is_empty() {
        return Err(TsxMetadataError::new(
            offset,
            "invalid `source` attribute: image path must not be empty",
        ));
    }
    let source = logical_tsx_path
        .resolve_from_file(source)
        .map_err(|error| {
            TsxMetadataError::new(offset, format!("invalid `source` attribute: {error}"))
        })?;
    if !source.as_str().ends_with(".png") {
        return Err(TsxMetadataError::new(
            offset,
            format!("invalid `source` attribute `{source}`; expected a `.png` path"),
        ));
    }
    Ok(TsxImageMetadata {
        source,
        width: positive_u32(&attributes, "width", offset)?,
        height: positive_u32(&attributes, "height", offset)?,
    })
}

fn collect_attributes(
    reader: &Reader<&[u8]>,
    element: &quick_xml::events::BytesStart<'_>,
    offset: u64,
) -> Result<std::collections::BTreeMap<String, String>, TsxMetadataError> {
    let mut attributes = std::collections::BTreeMap::new();
    let mut seen = HashSet::new();
    for attribute in element.attributes().with_checks(false) {
        let attribute = attribute.map_err(|error| {
            TsxMetadataError::new(offset, format!("invalid attribute: {error}"))
        })?;
        let name = str::from_utf8(attribute.key.as_ref())
            .map_err(|_| TsxMetadataError::new(offset, "attribute names must be UTF-8"))?;
        if !seen.insert(name.to_owned()) {
            return Err(TsxMetadataError::new(
                offset,
                format!("duplicate `{name}` attribute"),
            ));
        }
        let value = attribute
            .decoded_and_normalized_value(XmlVersion::Implicit1_0, reader.decoder())
            .map_err(|error| {
                TsxMetadataError::new(offset, format!("invalid `{name}` attribute: {error}"))
            })?
            .into_owned();
        attributes.insert(name.to_owned(), value);
    }
    Ok(attributes)
}

fn required<'a>(
    attributes: &'a std::collections::BTreeMap<String, String>,
    name: &str,
    offset: u64,
) -> Result<&'a str, TsxMetadataError> {
    attributes.get(name).map(String::as_str).ok_or_else(|| {
        TsxMetadataError::new(offset, format!("missing required `{name}` attribute"))
    })
}

fn positive_u32(
    attributes: &std::collections::BTreeMap<String, String>,
    name: &str,
    offset: u64,
) -> Result<u32, TsxMetadataError> {
    let value = required(attributes, name, offset)?;
    let parsed = value.parse::<u32>().map_err(|_| {
        TsxMetadataError::new(
            offset,
            format!("invalid `{name}` attribute `{value}`; expected u32"),
        )
    })?;
    if parsed == 0 {
        return Err(TsxMetadataError::new(
            offset,
            format!("invalid `{name}` attribute `0`; expected positive u32"),
        ));
    }
    Ok(parsed)
}

fn validate_atlas_geometry(
    tile_width: u32,
    tile_height: u32,
    columns: u32,
    tile_count: u32,
    image: &TsxImageMetadata,
    offset: u64,
) -> Result<(), TsxMetadataError> {
    let rows = tile_count.div_ceil(columns);
    let minimum_width = tile_width.checked_mul(columns).ok_or_else(|| {
        TsxMetadataError::new(offset, "tile width and columns overflow atlas geometry")
    })?;
    let minimum_height = tile_height.checked_mul(rows).ok_or_else(|| {
        TsxMetadataError::new(offset, "tile height and rows overflow atlas geometry")
    })?;
    if image.width < minimum_width || image.height < minimum_height {
        return Err(TsxMetadataError::new(
            offset,
            "image dimensions cannot contain the declared tile grid",
        ));
    }
    Ok(())
}

fn require_document_end(reader: &mut Reader<&[u8]>) -> Result<(), TsxMetadataError> {
    loop {
        let offset = reader.buffer_position();
        match reader
            .read_event()
            .map_err(|error| TsxMetadataError::new(offset, format!("malformed XML: {error}")))?
        {
            Event::Comment(_) | Event::PI(_) => {}
            Event::Text(text) if text.as_ref().iter().all(u8::is_ascii_whitespace) => {}
            Event::Eof => return Ok(()),
            _ => {
                return Err(TsxMetadataError::new(
                    offset,
                    "unexpected content after `tileset` root element",
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, fs, path::Path};

    use super::*;
    use crate::tmx_header::scan_tmx_external_tilesets_for_test;

    const VALID: &str = include_str!("../tests/fixtures/tsx-metadata/invented-atlas.tsx");

    fn owner() -> ScenarioRelativePath {
        ScenarioRelativePath::try_from("assets/tilesets/ground/invented.tsx").unwrap()
    }

    #[test]
    fn parses_one_root_image_and_normalizes_its_containing_file_relative_path() {
        let metadata = parse_tsx_tileset_metadata(VALID, &owner()).unwrap();
        assert_eq!(metadata.tile_width(), 32);
        assert_eq!(metadata.tile_height(), 16);
        assert_eq!(metadata.columns(), 3);
        assert_eq!(metadata.tile_count(), 5);
        assert_eq!(
            metadata.image().source().as_str(),
            "assets/images/invented.png"
        );
        assert_eq!(
            (metadata.image().width(), metadata.image().height()),
            (100, 32)
        );
    }

    #[test]
    fn rejects_required_attribute_and_geometry_failures() {
        for (xml, expected) in [
            (
                r#"<tileset tileheight="1" columns="1" tilecount="1"><image source="a.png" width="1" height="1"/></tileset>"#,
                "missing required `tilewidth` attribute",
            ),
            (
                r#"<tileset tilewidth="1" tileheight="1" columns="2" tilecount="1"><image source="a.png" width="2" height="1"/></tileset>"#,
                "`columns` must not exceed `tilecount`",
            ),
            (
                r#"<tileset tilewidth="16" tileheight="16" columns="2" tilecount="3"><image source="a.png" width="31" height="32"/></tileset>"#,
                "image dimensions cannot contain the declared tile grid",
            ),
        ] {
            assert_eq!(
                parse_tsx_tileset_metadata(xml, &owner())
                    .unwrap_err()
                    .detail,
                expected
            );
        }
    }

    #[test]
    fn rejects_duplicate_nested_multiple_nonempty_and_trailing_images() {
        for (xml, expected) in [
            (
                r#"<tileset tilewidth="1" tilewidth="2" tileheight="1" columns="1" tilecount="1"><image source="a.png" width="1" height="1"/></tileset>"#,
                "duplicate `tilewidth` attribute",
            ),
            (
                r#"<tileset tilewidth="1" tileheight="1" columns="1" tilecount="1"><tile><image source="a.png" width="1" height="1"/></tile></tileset>"#,
                "`image` must be a direct child of `tileset`",
            ),
            (
                r#"<tileset tilewidth="1" tileheight="1" columns="1" tilecount="1"><image source="a.png" width="1" height="1"></image></tileset>"#,
                "root-level `image` must be an empty element",
            ),
            (
                r#"<tileset tilewidth="1" tileheight="1" columns="1" tilecount="1"><image source="a.png" width="1" height="1"/><image source="b.png" width="1" height="1"/></tileset>"#,
                "multiple root-level `image` elements are unsupported",
            ),
            (
                r#"<tileset tilewidth="1" tileheight="1" columns="1" tilecount="1"><image source="a.png" width="1" height="1"/></tileset><trailing/>"#,
                "unexpected content after `tileset` root element",
            ),
        ] {
            let error = parse_tsx_tileset_metadata(xml, &owner()).unwrap_err();
            assert_eq!(error.detail, expected);
            assert!(error.offset() <= xml.len() as u64);
        }
    }

    #[test]
    fn rejects_unsafe_or_non_external_image_sources() {
        for source in [
            "",
            "   ",
            "/image.png",
            "C:/image.png",
            "https://example.invalid/image.png",
            "..\\image.png",
            "../../../../outside.png",
            "data:image/png;base64,invented",
            "image.webp",
        ] {
            let xml = format!(
                r#"<tileset tilewidth="1" tileheight="1" columns="1" tilecount="1"><image source="{source}" width="1" height="1"/></tileset>"#
            );
            assert!(
                parse_tsx_tileset_metadata(&xml, &owner()).is_err(),
                "{source:?} must be rejected"
            );
        }
    }

    #[test]
    #[ignore = "requires the separately pinned Python scenario checkout"]
    fn audits_external_tsx_targets_discovered_from_pinned_tmx_references() {
        let maps = std::env::var_os("RPG_S1_PINNED_TMX_DIR")
            .expect("RPG_S1_PINNED_TMX_DIR must name the pinned assets/maps directory");
        let maps = Path::new(&maps);
        let scenario_root = maps
            .parent()
            .and_then(Path::parent)
            .expect("TMX directory should be nested below the scenario root");
        let mut maps = fs::read_dir(maps)
            .expect("TMX map directory should be readable")
            .map(|entry| entry.expect("directory entry should be readable").path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "tmx"))
            .collect::<Vec<_>>();
        maps.sort();

        let mut targets = BTreeSet::new();
        for map in &maps {
            let logical = map
                .strip_prefix(scenario_root)
                .expect("TMX should be inside scenario root")
                .to_str()
                .expect("pinned scenario paths should be UTF-8");
            let logical = ScenarioRelativePath::try_from(logical).unwrap();
            let xml = fs::read_to_string(map).expect("TMX should be readable");
            let document = scan_tmx_external_tilesets_for_test(&xml, &logical)
                .unwrap_or_else(|error| panic!("{logical}: {error}"));
            targets.extend(
                document
                    .external_tilesets()
                    .iter()
                    .map(|reference| reference.source().clone()),
            );
        }

        for target in &targets {
            let path = scenario_root.join(target.as_str());
            assert!(path.is_file(), "missing discovered TSX target {target}");
            let xml = fs::read_to_string(&path).expect("TSX target should be readable");
            let metadata = parse_tsx_tileset_metadata(&xml, target)
                .unwrap_or_else(|error| panic!("{target}: {error}"));
            assert!(
                scenario_root
                    .join(metadata.image().source().as_str())
                    .is_file()
            );
        }
        assert_eq!(maps.len(), 47);
        assert_eq!(targets.len(), 17);
    }
}
