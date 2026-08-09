//! Strict, renderer-independent metadata from a finite orthogonal TMX document.
//!
//! The header-only API intentionally stops after the `<map>` start tag. The owned document API
//! additionally scans direct external tileset references; layers and objects remain uninterpreted.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "M4.01-M4.02 establish parser APIs before later M4 map loading consumes them"
    )
)]

use std::{collections::HashSet, fmt, str};

use quick_xml::{Reader, XmlVersion, events::Event};

use crate::scenario_path::ScenarioRelativePath;

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

/// One ordered external TSX dependency declared directly below the TMX map root.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TmxExternalTileset {
    first_gid: u32,
    source: ScenarioRelativePath,
}

impl TmxExternalTileset {
    pub(crate) const fn first_gid(&self) -> u32 {
        self.first_gid
    }

    pub(crate) fn source(&self) -> &ScenarioRelativePath {
        &self.source
    }
}

/// The single owned TMX parse result extended by later M4 milestones.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TmxMapDocument {
    header: TmxMapHeader,
    external_tilesets: Vec<TmxExternalTileset>,
}

impl TmxMapDocument {
    pub(crate) const fn header(&self) -> TmxMapHeader {
        self.header
    }

    pub(crate) fn external_tilesets(&self) -> &[TmxExternalTileset] {
        &self.external_tilesets
    }
}

/// A location-safe failure while parsing an owned TMX document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TmxMapDocumentError {
    offset: u64,
    detail: String,
}

impl TmxMapDocumentError {
    fn new(offset: u64, detail: impl Into<String>) -> Self {
        Self {
            offset,
            detail: detail.into(),
        }
    }

    fn from_header(error: TmxHeaderError) -> Self {
        Self::new(error.offset, error.detail)
    }

    fn is_inline_tileset(&self) -> bool {
        self.detail == "inline `tileset` without `source` is unsupported"
    }

    #[cfg(test)]
    fn offset(&self) -> u64 {
        self.offset
    }
}

impl fmt::Display for TmxMapDocumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "TMX map document at byte {}: {}",
            self.offset, self.detail
        )
    }
}

impl std::error::Error for TmxMapDocumentError {}

struct TmxMapScan {
    document: TmxMapDocument,
    first_inline_error: Option<TmxMapDocumentError>,
    inline_tilesets: usize,
}

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

/// Parses the TMX root once into the shared document owned by later M4 stages.
///
/// External TSX paths are resolved lexically relative to `logical_tmx_path`. Other root children
/// remain uninterpreted until their owning milestone, matching the M4.01 header-only tolerance.
pub(crate) fn parse_tmx_map_document(
    xml: &str,
    logical_tmx_path: &ScenarioRelativePath,
) -> Result<TmxMapDocument, TmxMapDocumentError> {
    if !logical_tmx_path.as_str().ends_with(".tmx") {
        return Err(TmxMapDocumentError::new(
            0,
            "logical owner path must identify a `.tmx` file",
        ));
    }
    let scan = scan_tmx_map_document(xml, logical_tmx_path)?;
    if let Some(error) = scan.first_inline_error {
        Err(error)
    } else {
        Ok(scan.document)
    }
}

fn scan_tmx_map_document(
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
    loop {
        let offset = reader.buffer_position();
        match reader
            .read_event()
            .map_err(|error| TmxMapDocumentError::new(offset, format!("malformed XML: {error}")))?
        {
            Event::Start(element) => {
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

/// Test-only tolerant scan used by pinned corpus audits that must inventory external references
/// even in the one map that also contains an unsupported inline tileset.
#[cfg(test)]
pub(crate) fn scan_tmx_external_tilesets_for_test(
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
    use std::{collections::BTreeSet, fs, path::Path};

    use super::*;

    const VALID: &str = include_str!("../tests/fixtures/tmx-header/finite-orthogonal.tmx");

    fn invented_document(children: &str) -> String {
        format!(
            r#"<map orientation="orthogonal" width="9" height="7" tilewidth="32" tileheight="32">{children}</map>"#
        )
    }

    fn invented_path() -> ScenarioRelativePath {
        ScenarioRelativePath::try_from("assets/maps/region/invented.tmx").unwrap()
    }

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
    fn document_owns_header_and_exact_ordered_normalized_external_tilesets() {
        let xml = invented_document(
            r#"
                <properties><property name="invented" value="ignored-for-now"/></properties>
                <tileset firstgid="1" source="../../tilesets/./ground.tsx"/>
                <layer id="1"><data encoding="csv">0</data></layer>
                <tileset firstgid="257" source="../tilesets/../tilesets/walls.tsx"/>
            "#,
        );

        let document = parse_tmx_map_document(&xml, &invented_path()).unwrap();
        assert_eq!(document.header().width(), 9);
        assert_eq!(document.header().height(), 7);
        assert_eq!(document.external_tilesets().len(), 2);
        assert_eq!(document.external_tilesets()[0].first_gid(), 1);
        assert_eq!(
            document.external_tilesets()[0].source().as_str(),
            "assets/tilesets/ground.tsx"
        );
        assert_eq!(document.external_tilesets()[1].first_gid(), 257);
        assert_eq!(
            document.external_tilesets()[1].source().as_str(),
            "assets/maps/tilesets/walls.tsx"
        );
    }

    #[test]
    fn header_only_api_still_ignores_all_child_content() {
        let xml = invented_document(r#"<group><tileset firstgid="0"/></group><malformed"#);
        let header = parse_tmx_map_header(&xml).unwrap();
        assert_eq!(header.width(), 9);
        assert_eq!(header.height(), 7);
        assert!(parse_tmx_map_document(&xml, &invented_path()).is_err());
    }

    #[test]
    fn rejects_missing_empty_duplicate_malformed_and_inline_attributes() {
        for (tileset, expected) in [
            (
                r#"<tileset source="../../tilesets/a.tsx"/>"#,
                "missing required `firstgid` attribute",
            ),
            (
                r#"<tileset firstgid="1"/>"#,
                "inline `tileset` without `source` is unsupported",
            ),
            (
                r#"<tileset firstgid="0" source="../../tilesets/a.tsx"/>"#,
                "expected positive u32",
            ),
            (
                r#"<tileset firstgid="many" source="../../tilesets/a.tsx"/>"#,
                "expected u32",
            ),
            (
                r#"<tileset firstgid="1" source=""/>"#,
                "invalid `source` attribute",
            ),
            (
                r#"<tileset firstgid="1" source="   "/>"#,
                "TSX path must not be empty",
            ),
            (
                r#"<tileset firstgid="1" source="../../tilesets/a.tsj"/>"#,
                "expected a `.tsx` path",
            ),
            (
                r#"<tileset firstgid="1" firstgid="2" source="../../tilesets/a.tsx"/>"#,
                "duplicate `firstgid` attribute",
            ),
            (
                r#"<tileset firstgid="1" source="../../tilesets/a.tsx" invented="no"/>"#,
                "unsupported `tileset` attribute `invented`",
            ),
        ] {
            let error =
                parse_tmx_map_document(&invented_document(tileset), &invented_path()).unwrap_err();
            assert!(
                error.detail.contains(expected),
                "{tileset}: expected {expected:?}, got {error:?}"
            );
        }

        let malformed =
            invented_document(r#"<tileset firstgid="1" source="../../tilesets/a.tsx></tileset>"#);
        assert!(parse_tmx_map_document(&malformed, &invented_path()).is_err());
    }

    #[test]
    fn rejects_unsafe_external_sources_before_any_filesystem_lookup() {
        for source in [
            "/absolute.tsx",
            "C:/tiles.tsx",
            "https://example.invalid/tiles.tsx",
            "..\\tilesets\\walls.tsx",
            "../../../../outside.tsx",
        ] {
            let xml = invented_document(&format!(r#"<tileset firstgid="1" source="{source}"/>"#));
            let error = parse_tmx_map_document(&xml, &invented_path()).unwrap_err();
            assert!(
                error.detail.contains("invalid `source` attribute"),
                "{source}"
            );
        }

        let wrong_owner = ScenarioRelativePath::try_from("assets/maps/invented.xml").unwrap();
        let error = parse_tmx_map_document(&invented_document(""), &wrong_owner).unwrap_err();
        assert_eq!(
            error.detail,
            "logical owner path must identify a `.tmx` file"
        );
        assert_eq!(error.offset(), 0);
    }

    #[test]
    fn rejects_nested_nonempty_duplicate_and_nonincreasing_tilesets() {
        for (children, expected) in [
            (
                r#"<group><tileset firstgid="1" source="../../tilesets/a.tsx"/></group>"#,
                "must be a direct child",
            ),
            (
                r#"<tileset firstgid="1" source="../../tilesets/a.tsx"></tileset>"#,
                "must be an empty element",
            ),
            (
                r#"<tileset firstgid="1" source="../../tilesets/a.tsx"/><tileset firstgid="1" source="../../tilesets/b.tsx"/>"#,
                "duplicate `firstgid` 1",
            ),
            (
                r#"<tileset firstgid="9" source="../../tilesets/a.tsx"/><tileset firstgid="8" source="../../tilesets/b.tsx"/>"#,
                "must be greater than preceding",
            ),
            (
                r#"<tileset firstgid="1" source="../../tilesets/a.tsx"/><tileset firstgid="2" source="../../tilesets/./a.tsx"/>"#,
                "duplicate external tileset source",
            ),
        ] {
            let error =
                parse_tmx_map_document(&invented_document(children), &invented_path()).unwrap_err();
            assert!(
                error.detail.contains(expected),
                "expected {expected:?}, got {error:?}"
            );
            assert!(error.offset() > 0);
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

    #[test]
    #[ignore = "requires the separately pinned Python scenario checkout"]
    fn audits_every_pinned_tmx_external_tileset_reference_when_source_is_available() {
        let maps = std::env::var_os("RPG_S1_PINNED_TMX_DIR")
            .expect("RPG_S1_PINNED_TMX_DIR must name the pinned assets/maps directory");
        let maps = Path::new(&maps);
        let scenario_root = maps
            .parent()
            .and_then(Path::parent)
            .expect("TMX directory should be nested below the scenario root");
        let mut files = fs::read_dir(maps)
            .expect("TMX map directory should be readable")
            .map(|entry| entry.expect("directory entry should be readable").path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "tmx"))
            .collect::<Vec<_>>();
        files.sort();

        let mut parsed_documents = 0;
        let mut inline_tilesets = 0;
        let mut reference_count = 0;
        let mut distinct_sources = BTreeSet::new();
        for path in &files {
            let logical = path
                .strip_prefix(scenario_root)
                .expect("TMX should be inside scenario root")
                .to_str()
                .expect("pinned scenario paths should be UTF-8");
            let logical = ScenarioRelativePath::try_from(logical).unwrap();
            let xml = fs::read_to_string(path).expect("TMX file should be readable");
            let scan = scan_tmx_map_document(&xml, &logical)
                .unwrap_or_else(|error| panic!("{logical}: {error}"));
            inline_tilesets += scan.inline_tilesets;
            parsed_documents += usize::from(scan.inline_tilesets == 0);
            assert!(
                scan.document
                    .external_tilesets()
                    .windows(2)
                    .all(|pair| pair[0].first_gid() < pair[1].first_gid())
            );
            for reference in scan.document.external_tilesets() {
                assert_eq!(
                    Path::new(reference.source().as_str()).extension(),
                    Some(std::ffi::OsStr::new("tsx"))
                );
                assert!(
                    scenario_root.join(reference.source().as_str()).is_file(),
                    "{} has missing normalized target {}",
                    logical,
                    reference.source()
                );
                reference_count += 1;
                distinct_sources.insert(reference.source().clone());
            }
        }

        assert_eq!(files.len(), 47);
        assert_eq!(parsed_documents, 46);
        assert_eq!(inline_tilesets, 1);
        assert_eq!(reference_count, 263);
        assert_eq!(distinct_sources.len(), 17);
    }
}
