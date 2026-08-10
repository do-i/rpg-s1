//! Strict, renderer-independent metadata from a finite orthogonal TMX document.
//!
//! The header-only API intentionally stops after the `<map>` start tag. The owned document API
//! additionally scans direct external tileset references, finite CSV tile layers, and direct
//! rectangle object groups, decoding orthogonal tile transforms from each raw GID.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "M4.01-M4.08 establish parser APIs before later M4 map loading consumes them"
    )
)]

use std::{collections::HashSet, fmt, str};

use quick_xml::{Reader, XmlVersion, events::Event};

use crate::scenario_path::ScenarioRelativePath;
use crate::tsx_metadata::TsxTilesetMetadata;

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

const TILED_HORIZONTAL_FLIP_FLAG: u32 = 0x8000_0000;
const TILED_VERTICAL_FLIP_FLAG: u32 = 0x4000_0000;
const TILED_DIAGONAL_FLIP_FLAG: u32 = 0x2000_0000;
const TILED_120_DEGREE_ROTATION_FLAG: u32 = 0x1000_0000;
const TILED_TRANSFORM_FLAGS: u32 = TILED_HORIZONTAL_FLIP_FLAG
    | TILED_VERTICAL_FLIP_FLAG
    | TILED_DIAGONAL_FLIP_FLAG
    | TILED_120_DEGREE_ROTATION_FLAG;

/// One decoded orthogonal Tiled global ID and its independent transform flags.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TmxTileGid {
    global_id: u32,
    flip_horizontally: bool,
    flip_vertically: bool,
    flip_diagonally: bool,
}

impl TmxTileGid {
    fn decode_orthogonal(raw_gid: u32) -> Result<Self, TmxTileGidError> {
        if raw_gid & TILED_120_DEGREE_ROTATION_FLAG != 0 {
            return Err(TmxTileGidError::Unsupported120DegreeRotation);
        }
        Ok(Self {
            global_id: raw_gid & !TILED_TRANSFORM_FLAGS,
            flip_horizontally: raw_gid & TILED_HORIZONTAL_FLIP_FLAG != 0,
            flip_vertically: raw_gid & TILED_VERTICAL_FLIP_FLAG != 0,
            flip_diagonally: raw_gid & TILED_DIAGONAL_FLIP_FLAG != 0,
        })
    }

    pub(crate) const fn global_id(self) -> u32 {
        self.global_id
    }

    pub(crate) const fn is_empty(self) -> bool {
        self.global_id == 0
    }

    pub(crate) const fn flip_horizontally(self) -> bool {
        self.flip_horizontally
    }

    pub(crate) const fn flip_vertically(self) -> bool {
        self.flip_vertically
    }

    pub(crate) const fn flip_diagonally(self) -> bool {
        self.flip_diagonally
    }

    #[cfg(test)]
    const fn raw_gid(self) -> u32 {
        self.global_id
            | if self.flip_horizontally {
                TILED_HORIZONTAL_FLIP_FLAG
            } else {
                0
            }
            | if self.flip_vertically {
                TILED_VERTICAL_FLIP_FLAG
            } else {
                0
            }
            | if self.flip_diagonally {
                TILED_DIAGONAL_FLIP_FLAG
            } else {
                0
            }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TmxTileGidError {
    Unsupported120DegreeRotation,
}

/// One finite tile layer with decoded Tiled global IDs in row-major order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TmxTileLayer {
    id: u32,
    name: String,
    width: u32,
    height: u32,
    gids: Vec<TmxTileGid>,
}

/// One ordered direct TMX object group from the supported game-data allowlist.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TmxObjectGroup {
    id: u32,
    name: String,
    objects: Vec<TmxRectangleObject>,
}

impl TmxObjectGroup {
    pub(crate) const fn id(&self) -> u32 {
        self.id
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn objects(&self) -> &[TmxRectangleObject] {
        &self.objects
    }
}

/// Renderer-independent bounds and typed ordered properties for one rectangle object.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TmxRectangleObject {
    id: u32,
    name: Option<String>,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    properties: Vec<TmxProperty>,
}

impl TmxRectangleObject {
    pub(crate) const fn id(&self) -> u32 {
        self.id
    }

    pub(crate) fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub(crate) const fn x(&self) -> f64 {
        self.x
    }

    pub(crate) const fn y(&self) -> f64 {
        self.y
    }

    pub(crate) const fn width(&self) -> f64 {
        self.width
    }

    pub(crate) const fn height(&self) -> f64 {
        self.height
    }

    pub(crate) fn properties(&self) -> &[TmxProperty] {
        &self.properties
    }
}

/// One named Tiled property in authored source order.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TmxProperty {
    name: String,
    value: TmxPropertyValue,
}

impl TmxProperty {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) const fn value(&self) -> &TmxPropertyValue {
        &self.value
    }
}

/// The strict scalar property types accepted by the TMX profile.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum TmxPropertyValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

impl TmxTileLayer {
    pub(crate) const fn id(&self) -> u32 {
        self.id
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) const fn width(&self) -> u32 {
        self.width
    }

    pub(crate) const fn height(&self) -> u32 {
        self.height
    }

    pub(crate) fn gids(&self) -> &[TmxTileGid] {
        &self.gids
    }

    pub(crate) fn gid_at(&self, column: u32, row: u32) -> Option<TmxTileGid> {
        if column >= self.width || row >= self.height {
            return None;
        }
        let index = usize::try_from(row.checked_mul(self.width)?.checked_add(column)?).ok()?;
        self.gids.get(index).copied()
    }
}

impl TmxExternalTileset {
    pub(crate) const fn first_gid(&self) -> u32 {
        self.first_gid
    }

    pub(crate) fn source(&self) -> &ScenarioRelativePath {
        &self.source
    }
}

/// Validated ordered external-tileset ranges for resolving decoded TMX tile GIDs.
#[derive(Clone, Debug)]
pub(crate) struct TmxTilesetRanges<'a> {
    ranges: Vec<TmxTilesetRange<'a>>,
}

#[derive(Clone, Copy, Debug)]
struct TmxTilesetRange<'a> {
    tileset: &'a TmxExternalTileset,
    exclusive_end: u32,
}

impl<'a> TmxTilesetRanges<'a> {
    /// Couples each ordered TMX reference to its parsed external TSX metadata.
    pub(crate) fn try_new(
        tilesets: impl IntoIterator<Item = (&'a TmxExternalTileset, &'a TsxTilesetMetadata)>,
    ) -> Result<Self, TmxGidResolutionError> {
        let mut ranges: Vec<TmxTilesetRange<'a>> = Vec::new();
        for (tileset, metadata) in tilesets {
            let exclusive_end = tileset.first_gid.checked_add(metadata.tile_count()).ok_or(
                TmxGidResolutionError::TilesetRangeOverflow {
                    first_gid: tileset.first_gid,
                    tile_count: metadata.tile_count(),
                },
            )?;
            if let Some(previous) = ranges.last() {
                if tileset.first_gid <= previous.tileset.first_gid {
                    return Err(TmxGidResolutionError::TilesetsNotStrictlyOrdered {
                        previous_first_gid: previous.tileset.first_gid,
                        first_gid: tileset.first_gid,
                    });
                }
                if tileset.first_gid < previous.exclusive_end {
                    return Err(TmxGidResolutionError::OverlappingTilesets {
                        previous_first_gid: previous.tileset.first_gid,
                        previous_exclusive_end: previous.exclusive_end,
                        first_gid: tileset.first_gid,
                    });
                }
            }
            ranges.push(TmxTilesetRange {
                tileset,
                exclusive_end,
            });
        }
        Ok(Self { ranges })
    }

    /// Resolves a decoded, non-empty global ID while retaining its transform flags.
    pub(crate) fn resolve(
        &self,
        gid: TmxTileGid,
    ) -> Result<Option<TmxResolvedTile<'a>>, TmxGidResolutionError> {
        if gid.is_empty() {
            return Ok(None);
        }
        let range_index = self
            .ranges
            .partition_point(|range| range.tileset.first_gid <= gid.global_id());
        let Some(range) = range_index
            .checked_sub(1)
            .and_then(|index| self.ranges.get(index))
        else {
            return Err(TmxGidResolutionError::UnmappedGlobalId(gid.global_id()));
        };
        if gid.global_id() >= range.exclusive_end {
            return Err(TmxGidResolutionError::UnmappedGlobalId(gid.global_id()));
        }
        Ok(Some(TmxResolvedTile {
            gid,
            tileset: range.tileset,
            local_id: gid.global_id() - range.tileset.first_gid,
        }))
    }
}

/// One decoded map tile resolved to its external tileset and zero-based local ID.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TmxResolvedTile<'a> {
    gid: TmxTileGid,
    tileset: &'a TmxExternalTileset,
    local_id: u32,
}

impl<'a> TmxResolvedTile<'a> {
    pub(crate) const fn gid(self) -> TmxTileGid {
        self.gid
    }

    pub(crate) const fn tileset(self) -> &'a TmxExternalTileset {
        self.tileset
    }

    pub(crate) const fn local_id(self) -> u32 {
        self.local_id
    }
}

/// Failure to construct or query strict ordered external-tileset ranges.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TmxGidResolutionError {
    TilesetRangeOverflow {
        first_gid: u32,
        tile_count: u32,
    },
    TilesetsNotStrictlyOrdered {
        previous_first_gid: u32,
        first_gid: u32,
    },
    OverlappingTilesets {
        previous_first_gid: u32,
        previous_exclusive_end: u32,
        first_gid: u32,
    },
    UnmappedGlobalId(u32),
}

impl fmt::Display for TmxGidResolutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TilesetRangeOverflow {
                first_gid,
                tile_count,
            } => write!(
                formatter,
                "tileset range firstgid {first_gid} plus tilecount {tile_count} exceeds u32"
            ),
            Self::TilesetsNotStrictlyOrdered {
                previous_first_gid,
                first_gid,
            } => write!(
                formatter,
                "tileset firstgid {first_gid} must be greater than preceding firstgid {previous_first_gid}"
            ),
            Self::OverlappingTilesets {
                previous_first_gid,
                previous_exclusive_end,
                first_gid,
            } => write!(
                formatter,
                "tileset firstgid {first_gid} overlaps range {previous_first_gid}..{previous_exclusive_end}"
            ),
            Self::UnmappedGlobalId(global_id) => {
                write!(
                    formatter,
                    "global tile ID {global_id} is outside every tileset range"
                )
            }
        }
    }
}

impl std::error::Error for TmxGidResolutionError {}

/// The single owned TMX parse result extended by later M4 milestones.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TmxMapDocument {
    header: TmxMapHeader,
    external_tilesets: Vec<TmxExternalTileset>,
    tile_layers: Vec<TmxTileLayer>,
    object_groups: Vec<TmxObjectGroup>,
}

impl TmxMapDocument {
    pub(crate) const fn header(&self) -> TmxMapHeader {
        self.header
    }

    pub(crate) fn external_tilesets(&self) -> &[TmxExternalTileset] {
        &self.external_tilesets
    }

    pub(crate) fn tile_layers(&self) -> &[TmxTileLayer] {
        &self.tile_layers
    }

    pub(crate) fn object_groups(&self) -> &[TmxObjectGroup] {
        &self.object_groups
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
                    validate_object_group_identity(
                        &group,
                        &mut object_group_ids,
                        &mut object_group_names,
                        offset,
                    )?;
                    scan.document.object_groups.push(group);
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
                    validate_object_group_identity(
                        &group,
                        &mut object_group_ids,
                        &mut object_group_names,
                        offset,
                    )?;
                    scan.document.object_groups.push(group);
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

fn parse_empty_object_group(
    reader: &Reader<&[u8]>,
    element: &quick_xml::events::BytesStart<'_>,
    offset: u64,
) -> Result<TmxObjectGroup, TmxMapDocumentError> {
    let (id, name) = parse_object_group_attributes(reader, element, offset)?;
    Ok(TmxObjectGroup {
        id,
        name,
        objects: Vec::new(),
    })
}

fn parse_object_group(
    reader: &mut Reader<&[u8]>,
    element: &quick_xml::events::BytesStart<'_>,
    offset: u64,
    object_ids: &mut HashSet<u32>,
) -> Result<TmxObjectGroup, TmxMapDocumentError> {
    let (id, name) = parse_object_group_attributes(reader, element, offset)?;
    let mut objects = Vec::new();
    loop {
        let child_offset = reader.buffer_position();
        match reader.read_event().map_err(|error| {
            TmxMapDocumentError::new(child_offset, format!("malformed XML: {error}"))
        })? {
            Event::Start(child) if child.name().as_ref() == b"object" => {
                let object = parse_rectangle_object(reader, &child, child_offset)?;
                validate_object_id(object.id, object_ids, child_offset)?;
                if name == "portals" {
                    validate_portal_properties(&object, child_offset)?;
                }
                objects.push(object);
            }
            Event::Empty(child) if child.name().as_ref() == b"object" => {
                let object = parse_empty_rectangle_object(reader, &child, child_offset)?;
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
                return Ok(TmxObjectGroup { id, name, objects });
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
) -> Result<(u32, String), TmxMapDocumentError> {
    let attributes = collect_document_attributes(reader, element, offset)?;
    if let Some(name) = attributes
        .keys()
        .find(|name| !matches!(name.as_str(), "id" | "name"))
    {
        return Err(TmxMapDocumentError::new(
            offset,
            format!("unsupported `objectgroup` attribute `{name}`"),
        ));
    }
    let id = positive_document_u32(&attributes, "id", offset)?;
    let name = required_document_attribute(&attributes, "name", offset)?;
    if !matches!(name, "portals" | "boss_enemy") {
        return Err(TmxMapDocumentError::new(
            offset,
            format!("unsupported object-group name `{name}`"),
        ));
    }
    Ok((id, name.to_owned()))
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
    // The pinned corpus contains seven editor markers in the `portals` group with no properties;
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
            "id" | "name" | "x" | "y" | "width" | "height"
        )
    }) {
        return Err(TmxMapDocumentError::new(
            offset,
            format!("unsupported rectangle `object` attribute `{name}`"),
        ));
    }
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
    })
}

fn parse_typed_properties(
    reader: &mut Reader<&[u8]>,
    element: &quick_xml::events::BytesStart<'_>,
    offset: u64,
) -> Result<Vec<TmxProperty>, TmxMapDocumentError> {
    require_no_attributes(reader, element, offset, "properties")?;
    let mut properties = Vec::new();
    let mut names = HashSet::new();
    loop {
        let child_offset = reader.buffer_position();
        match reader.read_event().map_err(|error| {
            TmxMapDocumentError::new(child_offset, format!("malformed XML: {error}"))
        })? {
            Event::Empty(child) if child.name().as_ref() == b"property" => {
                let property = parse_typed_property(reader, &child, child_offset, None)?;
                if !names.insert(property.name.clone()) {
                    return Err(TmxMapDocumentError::new(
                        child_offset,
                        format!("duplicate property name `{}`", property.name),
                    ));
                }
                properties.push(property);
            }
            Event::Start(child) if child.name().as_ref() == b"property" => {
                let text = read_property_text(reader, child_offset)?;
                let property = parse_typed_property(reader, &child, child_offset, Some(text))?;
                if !names.insert(property.name.clone()) {
                    return Err(TmxMapDocumentError::new(
                        child_offset,
                        format!("duplicate property name `{}`", property.name),
                    ));
                }
                properties.push(property);
            }
            Event::Start(child) | Event::Empty(child) => {
                let child_name = str::from_utf8(child.name().as_ref())
                    .unwrap_or("non-UTF-8")
                    .to_owned();
                return Err(TmxMapDocumentError::new(
                    child_offset,
                    format!("unsupported `{child_name}` child in `properties`"),
                ));
            }
            Event::Text(text) if text.as_ref().iter().all(u8::is_ascii_whitespace) => {}
            Event::Comment(_) => {}
            Event::End(end) if end.name().as_ref() == b"properties" => return Ok(properties),
            Event::End(_) => {
                return Err(TmxMapDocumentError::new(
                    child_offset,
                    "unexpected closing element in `properties`",
                ));
            }
            Event::Eof => {
                return Err(TmxMapDocumentError::new(
                    child_offset,
                    "unexpected end of XML before `properties` closed",
                ));
            }
            _ => {
                return Err(TmxMapDocumentError::new(
                    child_offset,
                    "`properties` may contain `property` elements only",
                ));
            }
        }
    }
}

fn parse_typed_property(
    reader: &Reader<&[u8]>,
    element: &quick_xml::events::BytesStart<'_>,
    offset: u64,
    text: Option<String>,
) -> Result<TmxProperty, TmxMapDocumentError> {
    let attributes = collect_document_attributes(reader, element, offset)?;
    if let Some(attribute) = attributes
        .keys()
        .find(|attribute| !matches!(attribute.as_str(), "name" | "type" | "value"))
    {
        return Err(TmxMapDocumentError::new(
            offset,
            format!("unsupported `property` attribute `{attribute}`"),
        ));
    }
    let name = required_document_attribute(&attributes, "name", offset)?;
    if name.trim().is_empty() {
        return Err(TmxMapDocumentError::new(
            offset,
            "invalid `name` attribute: property name must not be empty",
        ));
    }
    let property_type = attributes
        .get("type")
        .map(String::as_str)
        .unwrap_or("string");
    if !matches!(property_type, "string" | "int" | "float" | "bool") {
        return Err(TmxMapDocumentError::new(
            offset,
            format!("unsupported property type `{property_type}` for `{name}`"),
        ));
    }
    if attributes.contains_key("value") && text.as_ref().is_some_and(|text| !text.is_empty()) {
        return Err(TmxMapDocumentError::new(
            offset,
            format!("property `{name}` must use either the `value` attribute or text, not both"),
        ));
    }
    let source_value = attributes
        .get("value")
        .cloned()
        .or(text)
        .unwrap_or_default();
    if source_value.is_empty() && property_type != "string" {
        return Err(TmxMapDocumentError::new(
            offset,
            format!("property `{name}` of type `{property_type}` requires a value"),
        ));
    }
    let value = match property_type {
        "string" => TmxPropertyValue::String(source_value),
        "int" => TmxPropertyValue::Integer(source_value.parse::<i64>().map_err(|_| {
            TmxMapDocumentError::new(
                offset,
                format!("property `{name}` has invalid integer value `{source_value}`"),
            )
        })?),
        "float" => {
            let parsed = source_value.parse::<f64>().map_err(|_| {
                TmxMapDocumentError::new(
                    offset,
                    format!("property `{name}` has invalid finite float value `{source_value}`"),
                )
            })?;
            if !parsed.is_finite() {
                return Err(TmxMapDocumentError::new(
                    offset,
                    format!("property `{name}` has invalid finite float value `{source_value}`"),
                ));
            }
            TmxPropertyValue::Float(parsed)
        }
        "bool" => match source_value.as_str() {
            "true" => TmxPropertyValue::Boolean(true),
            "false" => TmxPropertyValue::Boolean(false),
            _ => {
                return Err(TmxMapDocumentError::new(
                    offset,
                    format!("property `{name}` has invalid boolean value `{source_value}`"),
                ));
            }
        },
        _ => unreachable!("property type allowlist checked above"),
    };
    Ok(TmxProperty {
        name: name.to_owned(),
        value,
    })
}

fn read_property_text(
    reader: &mut Reader<&[u8]>,
    property_offset: u64,
) -> Result<String, TmxMapDocumentError> {
    let mut text_value = String::new();
    loop {
        let offset = reader.buffer_position();
        match reader
            .read_event()
            .map_err(|error| TmxMapDocumentError::new(offset, format!("malformed XML: {error}")))?
        {
            Event::Text(text) => text_value.push_str(
                &text.xml_content(XmlVersion::Implicit1_0).map_err(|error| {
                    TmxMapDocumentError::new(offset, format!("invalid property text: {error}"))
                })?,
            ),
            Event::CData(text) => text_value.push_str(
                &text.xml_content(XmlVersion::Implicit1_0).map_err(|error| {
                    TmxMapDocumentError::new(offset, format!("invalid property CDATA: {error}"))
                })?,
            ),
            Event::GeneralRef(reference) => {
                if let Some(character) = reference.resolve_char_ref().map_err(|error| {
                    TmxMapDocumentError::new(
                        offset,
                        format!("invalid property character reference: {error}"),
                    )
                })? {
                    text_value.push(character);
                } else {
                    let reference = reference.decode().map_err(|error| {
                        TmxMapDocumentError::new(
                            offset,
                            format!("invalid property reference: {error}"),
                        )
                    })?;
                    text_value.push(match reference.as_ref() {
                        "lt" => '<',
                        "gt" => '>',
                        "amp" => '&',
                        "apos" => '\'',
                        "quot" => '"',
                        _ => {
                            return Err(TmxMapDocumentError::new(
                                offset,
                                format!("unsupported property entity reference `&{reference};`"),
                            ));
                        }
                    });
                }
            }
            Event::Comment(_) => {}
            Event::End(end) if end.name().as_ref() == b"property" => return Ok(text_value),
            Event::Eof => {
                return Err(TmxMapDocumentError::new(
                    offset,
                    "unexpected end of XML before `property` closed",
                ));
            }
            _ => {
                return Err(TmxMapDocumentError::new(
                    property_offset,
                    "`property` value must contain text only",
                ));
            }
        }
    }
}

fn require_no_attributes(
    reader: &Reader<&[u8]>,
    element: &quick_xml::events::BytesStart<'_>,
    offset: u64,
    element_name: &str,
) -> Result<(), TmxMapDocumentError> {
    let attributes = collect_document_attributes(reader, element, offset)?;
    if let Some(name) = attributes.keys().next() {
        Err(TmxMapDocumentError::new(
            offset,
            format!("unsupported `{element_name}` attribute `{name}`"),
        ))
    } else {
        Ok(())
    }
}

fn finite_document_f64(
    attributes: &std::collections::BTreeMap<String, String>,
    name: &str,
    offset: u64,
) -> Result<f64, TmxMapDocumentError> {
    let value = required_document_attribute(attributes, name, offset)?;
    let parsed = value.parse::<f64>().map_err(|_| {
        TmxMapDocumentError::new(
            offset,
            format!("invalid `{name}` attribute `{value}`; expected finite number"),
        )
    })?;
    if parsed.is_finite() {
        Ok(parsed)
    } else {
        Err(TmxMapDocumentError::new(
            offset,
            format!("invalid `{name}` attribute `{value}`; expected finite number"),
        ))
    }
}

fn optional_nonnegative_document_f64(
    attributes: &std::collections::BTreeMap<String, String>,
    name: &str,
    offset: u64,
) -> Result<f64, TmxMapDocumentError> {
    let Some(value) = attributes.get(name) else {
        return Ok(0.0);
    };
    let parsed = value.parse::<f64>().map_err(|_| {
        TmxMapDocumentError::new(
            offset,
            format!("invalid `{name}` attribute `{value}`; expected non-negative finite number"),
        )
    })?;
    if parsed.is_finite() && parsed >= 0.0 {
        Ok(parsed)
    } else {
        Err(TmxMapDocumentError::new(
            offset,
            format!("invalid `{name}` attribute `{value}`; expected non-negative finite number"),
        ))
    }
}

fn parse_tile_layer(
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
    let mut depth = 1_u32;
    loop {
        let child_offset = reader.buffer_position();
        match reader.read_event().map_err(|error| {
            TmxMapDocumentError::new(child_offset, format!("malformed XML: {error}"))
        })? {
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
                return Ok(TmxTileLayer {
                    id,
                    name: name.to_owned(),
                    width,
                    height,
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

fn collect_document_attributes(
    reader: &Reader<&[u8]>,
    element: &quick_xml::events::BytesStart<'_>,
    offset: u64,
) -> Result<std::collections::BTreeMap<String, String>, TmxMapDocumentError> {
    let mut attributes = std::collections::BTreeMap::new();
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
        attributes.insert(name.to_owned(), value);
    }
    Ok(attributes)
}

fn required_document_attribute<'a>(
    attributes: &'a std::collections::BTreeMap<String, String>,
    name: &str,
    offset: u64,
) -> Result<&'a str, TmxMapDocumentError> {
    attributes.get(name).map(String::as_str).ok_or_else(|| {
        TmxMapDocumentError::new(offset, format!("missing required `{name}` attribute"))
    })
}

fn positive_document_u32(
    attributes: &std::collections::BTreeMap<String, String>,
    name: &str,
    offset: u64,
) -> Result<u32, TmxMapDocumentError> {
    let value = required_document_attribute(attributes, name, offset)?;
    let parsed = value.parse::<u32>().map_err(|_| {
        TmxMapDocumentError::new(
            offset,
            format!("invalid `{name}` attribute `{value}`; expected u32"),
        )
    })?;
    if parsed == 0 {
        return Err(TmxMapDocumentError::new(
            offset,
            format!("invalid `{name}` attribute `0`; expected positive u32"),
        ));
    }
    Ok(parsed)
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
    use crate::tsx_metadata::parse_tsx_tileset_metadata;

    const VALID: &str = include_str!("../tests/fixtures/tmx-header/finite-orthogonal.tmx");

    fn invented_document(children: &str) -> String {
        format!(
            r#"<map orientation="orthogonal" width="9" height="7" tilewidth="32" tileheight="32">{children}</map>"#
        )
    }

    fn invented_path() -> ScenarioRelativePath {
        ScenarioRelativePath::try_from("assets/maps/region/invented.tmx").unwrap()
    }

    fn invented_tileset_metadata(tile_count: u32) -> TsxTilesetMetadata {
        let xml = format!(
            r#"<tileset tilewidth="1" tileheight="1" columns="1" tilecount="{tile_count}"><image source="invented.png" width="1" height="{tile_count}"/></tileset>"#
        );
        parse_tsx_tileset_metadata(
            &xml,
            &ScenarioRelativePath::try_from("assets/tilesets/invented.tsx").unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn decodes_all_orthogonal_gid_flip_combinations_and_empty_tiles() {
        const GLOBAL_ID: u32 = 0x0012_3456;
        for (flags, horizontal, vertical, diagonal) in [
            (0, false, false, false),
            (TILED_HORIZONTAL_FLIP_FLAG, true, false, false),
            (TILED_VERTICAL_FLIP_FLAG, false, true, false),
            (
                TILED_HORIZONTAL_FLIP_FLAG | TILED_VERTICAL_FLIP_FLAG,
                true,
                true,
                false,
            ),
            (TILED_DIAGONAL_FLIP_FLAG, false, false, true),
            (
                TILED_HORIZONTAL_FLIP_FLAG | TILED_DIAGONAL_FLIP_FLAG,
                true,
                false,
                true,
            ),
            (
                TILED_VERTICAL_FLIP_FLAG | TILED_DIAGONAL_FLIP_FLAG,
                false,
                true,
                true,
            ),
            (
                TILED_HORIZONTAL_FLIP_FLAG | TILED_VERTICAL_FLIP_FLAG | TILED_DIAGONAL_FLIP_FLAG,
                true,
                true,
                true,
            ),
        ] {
            let raw_gid = GLOBAL_ID | flags;
            let gid = TmxTileGid::decode_orthogonal(raw_gid).unwrap();
            assert_eq!(gid.global_id(), GLOBAL_ID);
            assert_eq!(gid.flip_horizontally(), horizontal);
            assert_eq!(gid.flip_vertically(), vertical);
            assert_eq!(gid.flip_diagonally(), diagonal);
            assert!(!gid.is_empty());
            assert_eq!(gid.raw_gid(), raw_gid);
        }

        let empty = TmxTileGid::decode_orthogonal(0).unwrap();
        assert!(empty.is_empty());
        assert_eq!(empty.raw_gid(), 0);
    }

    #[test]
    fn rejects_tiled_120_degree_rotation_flag_for_orthogonal_gids() {
        for raw_gid in [
            TILED_120_DEGREE_ROTATION_FLAG | 1,
            TILED_120_DEGREE_ROTATION_FLAG | TILED_HORIZONTAL_FLIP_FLAG | 17,
        ] {
            assert_eq!(
                TmxTileGid::decode_orthogonal(raw_gid),
                Err(TmxTileGidError::Unsupported120DegreeRotation)
            );
        }
    }

    #[test]
    fn resolves_ordered_tileset_boundaries_to_typed_local_ids() {
        let document = parse_tmx_map_document(
            &invented_document(
                r#"<tileset firstgid="1" source="../../tilesets/ground.tsx"/><tileset firstgid="4" source="../../tilesets/walls.tsx"/>"#,
            ),
            &invented_path(),
        )
        .unwrap();
        let ground = invented_tileset_metadata(3);
        let walls = invented_tileset_metadata(2);
        let ranges = TmxTilesetRanges::try_new([
            (&document.external_tilesets()[0], &ground),
            (&document.external_tilesets()[1], &walls),
        ])
        .unwrap();

        for (raw_gid, expected_source, expected_local_id) in [
            (
                TILED_HORIZONTAL_FLIP_FLAG | 1,
                "assets/tilesets/ground.tsx",
                0,
            ),
            (3, "assets/tilesets/ground.tsx", 2),
            (4, "assets/tilesets/walls.tsx", 0),
            (5, "assets/tilesets/walls.tsx", 1),
        ] {
            let gid = TmxTileGid::decode_orthogonal(raw_gid).unwrap();
            let resolved = ranges.resolve(gid).unwrap().unwrap();
            assert_eq!(resolved.gid(), gid);
            assert_eq!(resolved.tileset().source().as_str(), expected_source);
            assert_eq!(resolved.local_id(), expected_local_id);
        }

        let flipped = ranges
            .resolve(TmxTileGid::decode_orthogonal(TILED_HORIZONTAL_FLIP_FLAG | 1).unwrap())
            .unwrap()
            .unwrap();
        assert!(flipped.gid().flip_horizontally());
    }

    #[test]
    fn treats_empty_gid_as_no_tile_without_requiring_a_tileset() {
        let ranges = TmxTilesetRanges::try_new([]).unwrap();
        for raw_gid in [0, TILED_HORIZONTAL_FLIP_FLAG | TILED_DIAGONAL_FLIP_FLAG] {
            let empty = TmxTileGid::decode_orthogonal(raw_gid).unwrap();
            assert!(empty.is_empty());
            assert_eq!(ranges.resolve(empty), Ok(None));
        }
    }

    #[test]
    fn rejects_gids_before_between_and_at_the_end_of_declared_ranges() {
        let document = parse_tmx_map_document(
            &invented_document(
                r#"<tileset firstgid="5" source="../../tilesets/ground.tsx"/><tileset firstgid="10" source="../../tilesets/walls.tsx"/>"#,
            ),
            &invented_path(),
        )
        .unwrap();
        let ground = invented_tileset_metadata(2);
        let walls = invented_tileset_metadata(3);
        let ranges = TmxTilesetRanges::try_new([
            (&document.external_tilesets()[0], &ground),
            (&document.external_tilesets()[1], &walls),
        ])
        .unwrap();

        for global_id in [1, 7, 9, 13] {
            let gid = TmxTileGid::decode_orthogonal(global_id).unwrap();
            assert_eq!(
                ranges.resolve(gid),
                Err(TmxGidResolutionError::UnmappedGlobalId(global_id))
            );
        }
    }

    #[test]
    fn rejects_unordered_overlapping_and_overflowing_tileset_ranges() {
        let document = parse_tmx_map_document(
            &invented_document(
                r#"<tileset firstgid="1" source="../../tilesets/ground.tsx"/><tileset firstgid="4" source="../../tilesets/walls.tsx"/>"#,
            ),
            &invented_path(),
        )
        .unwrap();
        let five_tiles = invented_tileset_metadata(5);
        let one_tile = invented_tileset_metadata(1);

        assert_eq!(
            TmxTilesetRanges::try_new([
                (&document.external_tilesets()[1], &one_tile),
                (&document.external_tilesets()[0], &one_tile),
            ])
            .unwrap_err(),
            TmxGidResolutionError::TilesetsNotStrictlyOrdered {
                previous_first_gid: 4,
                first_gid: 1,
            }
        );
        assert_eq!(
            TmxTilesetRanges::try_new([
                (&document.external_tilesets()[0], &five_tiles),
                (&document.external_tilesets()[1], &one_tile),
            ])
            .unwrap_err(),
            TmxGidResolutionError::OverlappingTilesets {
                previous_first_gid: 1,
                previous_exclusive_end: 6,
                first_gid: 4,
            }
        );

        let overflowing = parse_tmx_map_document(
            &invented_document(
                r#"<tileset firstgid="4294967295" source="../../tilesets/huge.tsx"/>"#,
            ),
            &invented_path(),
        )
        .unwrap();
        assert_eq!(
            TmxTilesetRanges::try_new([(&overflowing.external_tilesets()[0], &one_tile)])
                .unwrap_err(),
            TmxGidResolutionError::TilesetRangeOverflow {
                first_gid: u32::MAX,
                tile_count: 1,
            }
        );
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
                <objectgroup id="1" name="portals"/>
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
        assert!(document.tile_layers().is_empty());
        assert_eq!(document.object_groups().len(), 1);
    }

    #[test]
    fn parses_direct_rectangle_groups_in_source_order_with_typed_properties() {
        let xml = invented_document(
            r#"
                <objectgroup id="5" name="portals">
                    <object id="7" name="exit" x="-4.5" y="3.25" width="12" height="0.5">
                        <properties>
                            <property name="target_map" value="next_map"/>
                            <property name="target_position_x" type="int" value="-7"/>
                            <property name="target_position_y" type="int">12</property>
                        </properties>
                    </object>
                    <object id="8" x="5" y="6"/>
                </objectgroup>
                <objectgroup id="6" name="boss_enemy">
                    <object id="9" name="boss" x="32" y="64" width="32" height="32"/>
                </objectgroup>
            "#,
        );
        let document = parse_tmx_map_document(&xml, &invented_path()).unwrap();

        assert_eq!(document.object_groups().len(), 2);
        let portals = &document.object_groups()[0];
        assert_eq!((portals.id(), portals.name()), (5, "portals"));
        assert_eq!(portals.objects().len(), 2);
        let portal = &portals.objects()[0];
        assert_eq!((portal.id(), portal.name()), (7, Some("exit")));
        assert_eq!(
            (portal.x(), portal.y(), portal.width(), portal.height()),
            (-4.5, 3.25, 12.0, 0.5)
        );
        assert_eq!(portal.properties().len(), 3);
        assert_eq!(
            portal.properties()[0],
            TmxProperty {
                name: "target_map".to_owned(),
                value: TmxPropertyValue::String("next_map".to_owned()),
            }
        );
        assert_eq!(
            portal.properties()[1],
            TmxProperty {
                name: "target_position_x".to_owned(),
                value: TmxPropertyValue::Integer(-7),
            }
        );
        assert_eq!(
            portal.properties()[2],
            TmxProperty {
                name: "target_position_y".to_owned(),
                value: TmxPropertyValue::Integer(12),
            }
        );

        let zero_sized = &portals.objects()[1];
        assert_eq!(zero_sized.name(), None);
        assert_eq!((zero_sized.width(), zero_sized.height()), (0.0, 0.0));
        assert!(zero_sized.properties().is_empty());
        assert_eq!(document.object_groups()[1].name(), "boss_enemy");
    }

    #[test]
    fn parses_string_float_and_boolean_property_sources_without_coercion() {
        let xml = invented_document(
            r#"
                <objectgroup id="1" name="boss_enemy">
                    <object id="1" x="0" y="0">
                        <properties>
                            <property name="implicit_empty"/>
                            <property name="explicit_string" type="string">hello &amp; goodbye</property>
                            <property name="float_value" type="float" value="-1.25"/>
                            <property name="bool_true" type="bool">true</property>
                            <property name="bool_false" type="bool" value="false"/>
                        </properties>
                    </object>
                </objectgroup>
            "#,
        );
        let document = parse_tmx_map_document(&xml, &invented_path()).unwrap();
        let properties = document.object_groups()[0].objects()[0].properties();
        assert_eq!(
            properties.iter().map(TmxProperty::name).collect::<Vec<_>>(),
            [
                "implicit_empty",
                "explicit_string",
                "float_value",
                "bool_true",
                "bool_false",
            ]
        );
        assert_eq!(
            properties[0].value(),
            &TmxPropertyValue::String(String::new())
        );
        assert_eq!(
            properties[1].value(),
            &TmxPropertyValue::String("hello & goodbye".to_owned())
        );
        assert_eq!(properties[2].value(), &TmxPropertyValue::Float(-1.25));
        assert_eq!(properties[3].value(), &TmxPropertyValue::Boolean(true));
        assert_eq!(properties[4].value(), &TmxPropertyValue::Boolean(false));
    }

    #[test]
    fn rejects_invalid_typed_property_forms_and_values() {
        for (property, expected) in [
            (
                r#"<property value="x"/>"#,
                "missing required `name` attribute",
            ),
            (r#"<property name=" "/>"#, "property name must not be empty"),
            (
                r#"<property name="x" invented="no"/>"#,
                "unsupported `property` attribute `invented`",
            ),
            (
                r##"<property name="x" type="color" value="#ffffff"/>"##,
                "unsupported property type `color`",
            ),
            (
                r#"<property name="x" type="int" value="1.0"/>"#,
                "invalid integer value `1.0`",
            ),
            (
                r#"<property name="x" type="float" value="NaN"/>"#,
                "invalid finite float value `NaN`",
            ),
            (
                r#"<property name="x" type="float" value="inf"/>"#,
                "invalid finite float value `inf`",
            ),
            (
                r#"<property name="x" type="bool" value="1"/>"#,
                "invalid boolean value `1`",
            ),
            (
                r#"<property name="x" type="bool" value="TRUE"/>"#,
                "invalid boolean value `TRUE`",
            ),
            (
                r#"<property name="x" type="int"/>"#,
                "type `int` requires a value",
            ),
            (
                r#"<property name="x" value="attribute">text</property>"#,
                "either the `value` attribute or text, not both",
            ),
        ] {
            let xml = invented_document(&format!(
                r#"<objectgroup id="1" name="boss_enemy"><object id="1" x="0" y="0"><properties>{property}</properties></object></objectgroup>"#
            ));
            let error = parse_tmx_map_document(&xml, &invented_path()).unwrap_err();
            assert!(
                error.detail.contains(expected),
                "{property}: expected {expected:?}, got {error:?}"
            );
        }

        let duplicate = invented_document(
            r#"<objectgroup id="1" name="boss_enemy"><object id="1" x="0" y="0"><properties><property name="same"/><property name="same" value="again"/></properties></object></objectgroup>"#,
        );
        assert_eq!(
            parse_tmx_map_document(&duplicate, &invented_path())
                .unwrap_err()
                .detail,
            "duplicate property name `same`"
        );
    }

    #[test]
    fn rejects_incomplete_or_mistyped_property_bearing_portals() {
        for properties in [
            r#"<property name="target_map" value="next"/>"#,
            r#"<property name="target_map" type="int" value="1"/><property name="target_position_x" type="int" value="2"/><property name="target_position_y" type="int" value="3"/>"#,
            r#"<property name="target_map" value="next"/><property name="target_position_x" value="2"/><property name="target_position_y" type="int" value="3"/>"#,
            r#"<property name="target_map" value="next"/><property name="target_position_x" type="int" value="2"/><property name="invented" type="int" value="3"/>"#,
        ] {
            let xml = invented_document(&format!(
                r#"<objectgroup id="1" name="portals"><object id="1" x="0" y="0"><properties>{properties}</properties></object></objectgroup>"#
            ));
            assert!(
                parse_tmx_map_document(&xml, &invented_path())
                    .unwrap_err()
                    .detail
                    .contains("must have exactly string `target_map` and integer")
            );
        }
    }

    #[test]
    fn rejects_unsupported_object_geometry_and_attributes() {
        for child in [
            "<point/>",
            "<ellipse/>",
            "<polygon points=\"0,0 1,1\"/>",
            "<polyline points=\"0,0 1,1\"/>",
            "<text>invented</text>",
        ] {
            let xml = invented_document(&format!(
                r#"<objectgroup id="1" name="portals"><object id="1" x="0" y="0">{child}</object></objectgroup>"#
            ));
            let error = parse_tmx_map_document(&xml, &invented_path()).unwrap_err();
            assert!(
                error.detail.contains("unsupported rectangle-object child"),
                "{child}: {error:?}"
            );
        }

        for attribute in [
            r#"gid="1""#,
            r#"template="invented.tx""#,
            r#"rotation="90""#,
            r#"class="Invented""#,
        ] {
            let xml = invented_document(&format!(
                r#"<objectgroup id="1" name="portals"><object id="1" x="0" y="0" {attribute}/></objectgroup>"#
            ));
            let error = parse_tmx_map_document(&xml, &invented_path()).unwrap_err();
            assert!(
                error
                    .detail
                    .contains("unsupported rectangle `object` attribute"),
                "{attribute}: {error:?}"
            );
        }
    }

    #[test]
    fn rejects_nested_object_structures_and_unknown_groups() {
        for (children, expected) in [
            (
                r#"<group><objectgroup id="1" name="portals"/></group>"#,
                "`objectgroup` must be a direct child of `map`",
            ),
            (
                r#"<objectgroup id="1" name="portals"><objectgroup id="2" name="boss_enemy"/></objectgroup>"#,
                "unsupported `objectgroup` child in `objectgroup`",
            ),
            (
                r#"<object id="1" x="0" y="0"/>"#,
                "`object` must be a direct child of `objectgroup`",
            ),
            (
                r#"<objectgroup id="1" name="invented"/>"#,
                "unsupported object-group name `invented`",
            ),
        ] {
            let error =
                parse_tmx_map_document(&invented_document(children), &invented_path()).unwrap_err();
            assert_eq!(error.detail, expected, "{children}");
        }
    }

    #[test]
    fn rejects_duplicate_object_group_and_object_identities() {
        for (children, expected) in [
            (
                r#"<objectgroup id="1" name="portals"/><objectgroup id="1" name="boss_enemy"/>"#,
                "duplicate object-group ID 1",
            ),
            (
                r#"<objectgroup id="1" name="portals"/><objectgroup id="2" name="portals"/>"#,
                "duplicate object-group name `portals`",
            ),
            (
                r#"<objectgroup id="1" name="portals"><object id="7" x="0" y="0"/></objectgroup><objectgroup id="2" name="boss_enemy"><object id="7" x="1" y="1"/></objectgroup>"#,
                "duplicate object ID 7",
            ),
        ] {
            let error =
                parse_tmx_map_document(&invented_document(children), &invented_path()).unwrap_err();
            assert_eq!(error.detail, expected, "{children}");
        }
    }

    #[test]
    fn rejects_malformed_duplicate_and_invalid_object_attributes() {
        for (children, expected) in [
            (
                r#"<objectgroup id="1" id="2" name="portals"/>"#,
                "duplicate `id` attribute",
            ),
            (
                r#"<objectgroup id="1" name="portals"><object id="1" id="2" x="0" y="0"/></objectgroup>"#,
                "duplicate `id` attribute",
            ),
            (
                r#"<objectgroup id="1" name="portals"><object id="1" x="0" y="0"><properties><property name="a" name="b"/></properties></object></objectgroup>"#,
                "duplicate `name` attribute",
            ),
            (
                r#"<objectgroup name="portals"/>"#,
                "missing required `id` attribute",
            ),
            (
                r#"<objectgroup id="1" name="portals"><object x="0" y="0"/></objectgroup>"#,
                "missing required `id` attribute",
            ),
            (
                r#"<objectgroup id="1" name="portals"><object id="1" y="0"/></objectgroup>"#,
                "missing required `x` attribute",
            ),
            (
                r#"<objectgroup id="1" name="portals"><object id="1" x="NaN" y="0"/></objectgroup>"#,
                "invalid `x` attribute `NaN`; expected finite number",
            ),
            (
                r#"<objectgroup id="1" name="portals"><object id="1" x="0" y="0" width="-1"/></objectgroup>"#,
                "invalid `width` attribute `-1`; expected non-negative finite number",
            ),
        ] {
            let error =
                parse_tmx_map_document(&invented_document(children), &invented_path()).unwrap_err();
            assert_eq!(error.detail, expected, "{children}");
            assert!(error.offset() <= children.len() as u64 + 100);
        }
    }

    #[test]
    fn parses_finite_csv_tile_layers_in_source_order_with_decoded_gids() {
        let xml = r#"
            <map orientation="orthogonal" width="3" height="2" tilewidth="32" tileheight="32">
                <layer id="7" name="ground" width="3" height="2" visible="0">
                    <data encoding="csv">
                        0,1,2147483650,
                        3,4,4026531839
                    </data>
                </layer>
                <layer id="8" name="decoration" width="3" height="2">
                    <data encoding="csv"><![CDATA[
                        6,5,4,
                        3,2,1
                    ]]></data>
                </layer>
            </map>
        "#;

        let document = parse_tmx_map_document(xml, &invented_path()).unwrap();
        assert_eq!(document.tile_layers().len(), 2);
        let ground = &document.tile_layers()[0];
        assert_eq!(ground.id(), 7);
        assert_eq!(ground.name(), "ground");
        assert_eq!(ground.width(), 3);
        assert_eq!(ground.height(), 2);
        assert_eq!(
            ground
                .gids()
                .iter()
                .copied()
                .map(TmxTileGid::raw_gid)
                .collect::<Vec<_>>(),
            &[0, 1, 2_147_483_650, 3, 4, 4_026_531_839]
        );
        let horizontal = ground.gid_at(2, 0).unwrap();
        assert_eq!(horizontal.global_id(), 2);
        assert!(horizontal.flip_horizontally());
        assert!(!horizontal.flip_vertically());
        assert!(!horizontal.flip_diagonally());
        assert_eq!(
            ground.gid_at(2, 1).map(TmxTileGid::raw_gid),
            Some(4_026_531_839)
        );
        assert_eq!(ground.gid_at(3, 1), None);
        assert_eq!(ground.gid_at(0, 2), None);
        assert_eq!(document.tile_layers()[1].name(), "decoration");
        assert_eq!(
            document.tile_layers()[1]
                .gids()
                .iter()
                .map(|gid| gid.global_id())
                .collect::<Vec<_>>(),
            &[6, 5, 4, 3, 2, 1]
        );
    }

    #[test]
    fn rejects_invalid_layer_shape_and_csv_encoding() {
        let cases = [
            (
                r#"<layer id="1" width="3" height="2"><data encoding="csv">0,0,0,
0,0,0</data></layer>"#,
                "missing required `name` attribute",
            ),
            (
                r#"<layer id="1" name="ground" width="2" height="2"><data encoding="csv">0,0,
0,0</data></layer>"#,
                "dimensions 2x2 do not match map dimensions 3x2",
            ),
            (
                r#"<layer id="1" name="ground" width="3" height="2"/>"#,
                "must contain one CSV `data` element",
            ),
            (
                r#"<layer id="1" name="ground" width="3" height="2"><data encoding="base64">AAAA</data></layer>"#,
                "unsupported tile data encoding `base64`",
            ),
            (
                r#"<layer id="1" name="ground" width="3" height="2"><data encoding="csv" compression="gzip">0,0,0,
0,0,0</data></layer>"#,
                "compression `gzip` is unsupported",
            ),
        ];

        for (layer, expected) in cases {
            let xml = format!(
                r#"<map orientation="orthogonal" width="3" height="2" tilewidth="32" tileheight="32">{layer}</map>"#
            );
            let error = parse_tmx_map_document(&xml, &invented_path()).unwrap_err();
            assert!(
                error.detail.contains(expected),
                "expected {expected:?}, got {error:?}"
            );
        }
    }

    #[test]
    fn rejects_wrong_csv_row_column_counts_and_invalid_gids() {
        for (csv, expected) in [
            ("0,0,0", "has 1 CSV rows; expected 2"),
            ("0,0,\n0,0,0", "CSV row 1 has 2 columns; expected 3"),
            ("0,0,0,0,\n0,0,0", "CSV row 1 has 4 columns; expected 3"),
            (
                "0,-1,0,\n0,0,0",
                "invalid GID `-1` at row 1, column 2; expected u32",
            ),
            (
                "0,,0,\n0,0,0",
                "invalid GID `` at row 1, column 2; expected u32",
            ),
            (
                "0,268435457,0,\n0,0,0",
                "unsupported 120-degree rotation flag at row 1, column 2",
            ),
        ] {
            let xml = format!(
                r#"<map orientation="orthogonal" width="3" height="2" tilewidth="32" tileheight="32"><layer id="1" name="ground" width="3" height="2"><data encoding="csv">{csv}</data></layer></map>"#
            );
            let error = parse_tmx_map_document(&xml, &invented_path()).unwrap_err();
            assert!(
                error.detail.contains(expected),
                "expected {expected:?}, got {error:?}"
            );
        }
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

    #[test]
    #[ignore = "requires the separately pinned Python scenario checkout"]
    fn audits_every_pinned_csv_tile_layer_when_source_is_available() {
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

        let mut layer_count = 0;
        let mut gid_count = 0;
        let mut flip_combination_counts = [0_usize; 8];
        for path in &files {
            let logical = path
                .strip_prefix(scenario_root)
                .expect("TMX should be inside scenario root")
                .to_str()
                .expect("pinned scenario paths should be UTF-8");
            let logical = ScenarioRelativePath::try_from(logical).unwrap();
            let xml = fs::read_to_string(path).expect("TMX file should be readable");
            let document = scan_tmx_map_document(&xml, &logical)
                .unwrap_or_else(|error| panic!("{logical}: {error}"))
                .document;
            for layer in document.tile_layers() {
                assert_eq!(layer.width(), document.header().width());
                assert_eq!(layer.height(), document.header().height());
                assert_eq!(
                    layer.gids().len(),
                    usize::try_from(layer.width() * layer.height()).unwrap()
                );
                layer_count += 1;
                gid_count += layer.gids().len();
                for gid in layer.gids() {
                    let combination = usize::from(gid.flip_horizontally())
                        | (usize::from(gid.flip_vertically()) << 1)
                        | (usize::from(gid.flip_diagonally()) << 2);
                    flip_combination_counts[combination] += 1;
                }
            }
        }

        assert_eq!(files.len(), 47);
        assert_eq!(layer_count, 170);
        assert_eq!(gid_count, 161_066);
        assert_eq!(flip_combination_counts, [160_864, 10, 2, 0, 0, 189, 1, 0]);
    }

    #[test]
    #[ignore = "requires the separately pinned Python scenario checkout"]
    fn audits_every_pinned_rectangle_object_group_when_source_is_available() {
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

        let mut portal_groups = 0;
        let mut boss_groups = 0;
        let mut objects = 0;
        let mut zero_sized_objects = 0;
        let mut properties = 0;
        let mut property_strings = 0;
        let mut property_integers = 0;
        let mut property_floats = 0;
        let mut property_booleans = 0;
        let mut runtime_portals = 0;
        for path in &files {
            let logical = path
                .strip_prefix(scenario_root)
                .expect("TMX should be inside scenario root")
                .to_str()
                .expect("pinned scenario paths should be UTF-8");
            let logical = ScenarioRelativePath::try_from(logical).unwrap();
            let xml = fs::read_to_string(path).expect("TMX file should be readable");
            let document = scan_tmx_map_document(&xml, &logical)
                .unwrap_or_else(|error| panic!("{logical}: {error}"))
                .document;
            for group in document.object_groups() {
                match group.name() {
                    "portals" => portal_groups += 1,
                    "boss_enemy" => boss_groups += 1,
                    name => panic!("unexpected accepted object-group name {name}"),
                }
                for object in group.objects() {
                    objects += 1;
                    zero_sized_objects +=
                        usize::from(object.width() == 0.0 && object.height() == 0.0);
                    properties += object.properties().len();
                    if group.name() == "portals" && !object.properties().is_empty() {
                        runtime_portals += 1;
                    }
                    for property in object.properties() {
                        match property.value() {
                            TmxPropertyValue::String(_) => property_strings += 1,
                            TmxPropertyValue::Integer(_) => property_integers += 1,
                            TmxPropertyValue::Float(_) => property_floats += 1,
                            TmxPropertyValue::Boolean(_) => property_booleans += 1,
                        }
                    }
                }
            }
        }

        assert_eq!(files.len(), 47);
        assert_eq!(portal_groups, 45);
        assert_eq!(boss_groups, 10);
        assert_eq!(objects, 109);
        assert_eq!(zero_sized_objects, 10);
        assert_eq!(runtime_portals, 92);
        assert_eq!(properties, 276);
        assert_eq!(property_strings, 92);
        assert_eq!(property_integers, 184);
        assert_eq!(property_floats, 0);
        assert_eq!(property_booleans, 0);
    }
}
