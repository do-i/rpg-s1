//! Strict, renderer-independent metadata from a finite orthogonal TMX map root.
//!
//! This intentionally stops after the `<map>` start tag. Tilesets, layers, and objects are
//! separate M4 concerns and their child content is not interpreted here.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "M4.01 establishes the parser API before later M4 map loading consumes it"
    )
)]

use std::{collections::HashSet, fmt, str};

use quick_xml::{Reader, XmlVersion, events::Event};

/// The only map orientation supported by the current fixed-grid runtime profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TmxOrientation {
    Orthogonal,
}

/// Validated dimensions and orientation of a finite TMX map.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TmxMapHeader {
    orientation: TmxOrientation,
    width: u32,
    height: u32,
    tile_width: u32,
    tile_height: u32,
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "M4.01 exposes typed map metadata for the later map loader and renderer"
    )
)]
impl TmxMapHeader {
    pub(crate) const fn orientation(self) -> TmxOrientation {
        self.orientation
    }

    pub(crate) const fn width(self) -> u32 {
        self.width
    }

    pub(crate) const fn height(self) -> u32 {
        self.height
    }

    pub(crate) const fn tile_width(self) -> u32 {
        self.tile_width
    }

    pub(crate) const fn tile_height(self) -> u32 {
        self.tile_height
    }
}

/// A location-safe TMX root-header failure. Byte offsets are relative to the supplied XML.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TmxHeaderError {
    offset: u64,
    detail: String,
}

impl TmxHeaderError {
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

impl fmt::Display for TmxHeaderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "TMX map header at byte {}: {}",
            self.offset, self.detail
        )
    }
}

impl std::error::Error for TmxHeaderError {}

/// Parses only the root `<map>` header.
///
/// Tiled defines an omitted `infinite` attribute as finite (`0`); this parser pins that default.
/// Explicit `infinite="1"` is rejected because chunked infinite maps are outside M4's profile.
pub(crate) fn parse_tmx_map_header(xml: &str) -> Result<TmxMapHeader, TmxHeaderError> {
    let mut reader = Reader::from_str(xml);
    loop {
        let offset = reader.buffer_position();
        match reader
            .read_event()
            .map_err(|error| TmxHeaderError::new(offset, format!("malformed XML: {error}")))?
        {
            Event::Decl(_) | Event::Comment(_) | Event::PI(_) | Event::DocType(_) => {}
            Event::Text(text) if text.as_ref().iter().all(u8::is_ascii_whitespace) => {}
            Event::Start(element) | Event::Empty(element) => {
                if element.name().as_ref() != b"map" {
                    return Err(TmxHeaderError::new(offset, "root element must be `map`"));
                }
                return parse_map_attributes(&reader, &element, offset);
            }
            Event::Eof => return Err(TmxHeaderError::new(offset, "missing `map` root element")),
            _ => return Err(TmxHeaderError::new(offset, "root element must be `map`")),
        }
    }
}

fn parse_map_attributes(
    reader: &Reader<&[u8]>,
    element: &quick_xml::events::BytesStart<'_>,
    offset: u64,
) -> Result<TmxMapHeader, TmxHeaderError> {
    let mut attributes = std::collections::BTreeMap::new();
    let mut seen = HashSet::new();
    for attribute in element.attributes().with_checks(false) {
        let attribute = attribute
            .map_err(|error| TmxHeaderError::new(offset, format!("invalid attribute: {error}")))?;
        let name = str::from_utf8(attribute.key.as_ref())
            .map_err(|_| TmxHeaderError::new(offset, "attribute names must be UTF-8"))?;
        if !seen.insert(name.to_owned()) {
            return Err(TmxHeaderError::new(
                offset,
                format!("duplicate `{name}` attribute"),
            ));
        }
        let value = attribute
            .decoded_and_normalized_value(XmlVersion::Implicit1_0, reader.decoder())
            .map_err(|error| {
                TmxHeaderError::new(offset, format!("invalid `{name}` attribute: {error}"))
            })?
            .into_owned();
        attributes.insert(name.to_owned(), value);
    }

    let orientation = required(&attributes, "orientation", offset)?;
    if orientation != "orthogonal" {
        return Err(TmxHeaderError::new(
            offset,
            format!("unsupported orientation `{orientation}`; expected `orthogonal`"),
        ));
    }
    let infinite = attributes
        .get("infinite")
        .map(String::as_str)
        .unwrap_or("0");
    match infinite {
        "0" => {}
        "1" => return Err(TmxHeaderError::new(offset, "infinite maps are unsupported")),
        _ => {
            return Err(TmxHeaderError::new(
                offset,
                format!("invalid `infinite` attribute `{infinite}`; expected `0` or `1`"),
            ));
        }
    }

    Ok(TmxMapHeader {
        orientation: TmxOrientation::Orthogonal,
        width: positive_u32(&attributes, "width", offset)?,
        height: positive_u32(&attributes, "height", offset)?,
        tile_width: positive_u32(&attributes, "tilewidth", offset)?,
        tile_height: positive_u32(&attributes, "tileheight", offset)?,
    })
}

fn required<'a>(
    attributes: &'a std::collections::BTreeMap<String, String>,
    name: &str,
    offset: u64,
) -> Result<&'a str, TmxHeaderError> {
    attributes
        .get(name)
        .map(String::as_str)
        .ok_or_else(|| TmxHeaderError::new(offset, format!("missing required `{name}` attribute")))
}

fn positive_u32(
    attributes: &std::collections::BTreeMap<String, String>,
    name: &str,
    offset: u64,
) -> Result<u32, TmxHeaderError> {
    let value = required(attributes, name, offset)?;
    let parsed = value.parse::<u32>().map_err(|_| {
        TmxHeaderError::new(
            offset,
            format!("invalid `{name}` attribute `{value}`; expected u32"),
        )
    })?;
    if parsed == 0 {
        return Err(TmxHeaderError::new(
            offset,
            format!("invalid `{name}` attribute `0`; expected positive u32"),
        ));
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::*;

    const VALID: &str = include_str!("../tests/fixtures/tmx-header/finite-orthogonal.tmx");

    #[test]
    fn parses_finite_orthogonal_header_without_interpreting_children() {
        assert_eq!(
            parse_tmx_map_header(VALID).unwrap(),
            TmxMapHeader {
                orientation: TmxOrientation::Orthogonal,
                width: 30,
                height: 20,
                tile_width: 32,
                tile_height: 32,
            }
        );
    }

    #[test]
    fn omitted_infinite_uses_tiled_finite_default() {
        let header = parse_tmx_map_header(
            r#"<map orientation="orthogonal" width="1" height="2" tilewidth="16" tileheight="24"/>"#,
        )
        .unwrap();
        assert_eq!(header.orientation(), TmxOrientation::Orthogonal);
        assert_eq!(header.width(), 1);
        assert_eq!(header.height(), 2);
        assert_eq!(header.tile_width(), 16);
        assert_eq!(header.tile_height(), 24);
    }

    #[test]
    fn rejects_missing_duplicate_malformed_and_invalid_header_attributes() {
        for (xml, expected) in [
            (
                r#"<map width="1" height="1" tilewidth="1" tileheight="1"/>"#,
                "missing required `orientation` attribute",
            ),
            (
                r#"<map orientation="orthogonal" orientation="isometric" width="1" height="1" tilewidth="1" tileheight="1"/>"#,
                "duplicate `orientation` attribute",
            ),
            (
                r#"<map orientation="orthogonal" width="many" height="1" tilewidth="1" tileheight="1"/>"#,
                "invalid `width` attribute `many`; expected u32",
            ),
            (
                r#"<map orientation="orthogonal" width="0" height="1" tilewidth="1" tileheight="1"/>"#,
                "invalid `width` attribute `0`; expected positive u32",
            ),
        ] {
            let error = parse_tmx_map_header(xml).unwrap_err();
            assert_eq!(error.offset(), 0);
            assert_eq!(error.detail, expected);
        }
    }

    #[test]
    fn rejects_non_orthogonal_and_infinite_maps() {
        for (xml, expected) in [
            (
                r#"<map orientation="isometric" width="1" height="1" tilewidth="1" tileheight="1"/>"#,
                "unsupported orientation `isometric`; expected `orthogonal`",
            ),
            (
                r#"<map orientation="orthogonal" width="1" height="1" tilewidth="1" tileheight="1" infinite="1"/>"#,
                "infinite maps are unsupported",
            ),
            (
                r#"<map orientation="orthogonal" width="1" height="1" tilewidth="1" tileheight="1" infinite="true"/>"#,
                "invalid `infinite` attribute `true`; expected `0` or `1`",
            ),
        ] {
            assert_eq!(parse_tmx_map_header(xml).unwrap_err().detail, expected);
        }
    }

    #[test]
    fn rejects_non_map_or_malformed_roots_with_a_byte_offset() {
        for xml in ["<tileset/>", "<map"] {
            let error = parse_tmx_map_header(xml).unwrap_err();
            assert_eq!(error.offset(), 0);
        }
    }

    #[test]
    #[ignore = "requires the separately pinned Python scenario checkout"]
    fn audits_every_pinned_tmx_header_when_source_is_available() {
        let maps = std::env::var_os("RPG_S1_PINNED_TMX_DIR")
            .expect("RPG_S1_PINNED_TMX_DIR must name the pinned assets/maps directory");
        let mut files = fs::read_dir(Path::new(&maps))
            .expect("TMX map directory should be readable")
            .map(|entry| entry.expect("directory entry should be readable").path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "tmx"))
            .collect::<Vec<_>>();
        files.sort();

        for path in &files {
            let xml = fs::read_to_string(path).expect("TMX file should be readable");
            parse_tmx_map_header(&xml)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        }

        assert_eq!(files.len(), 47);
    }
}
