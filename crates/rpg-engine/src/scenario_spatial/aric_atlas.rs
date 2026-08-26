//! Renderer-independent atlas slicing for Aric's four-direction walk sheet.
//!
//! The pinned Python renderer indexes sheet rows in `Up`, `Left`, `Down`, `Right` order.
//! Aric's TSX attaches the corresponding walk animations to tiles `0`, `9`, `18`, and `27`;
//! those owner tiles are the idle/base frames at column zero of each row. This adapter validates
//! that exact authored profile before exposing frame rectangles to later rendering work.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "M4.16/M4.20 establish Aric atlas selection and authored animation playback"
    )
)]

use std::{error::Error, fmt};

use super::CardinalDirection;
use crate::tsx_metadata::TsxTilesetMetadata;

const ARIC_FRAME_SIZE: u32 = 64;
const ARIC_COLUMNS: u32 = 9;
const ARIC_ROWS: u32 = 4;
const ARIC_TILE_COUNT: u32 = ARIC_COLUMNS * ARIC_ROWS;
const ARIC_IMAGE_WIDTH: u32 = ARIC_COLUMNS * ARIC_FRAME_SIZE;
const ARIC_IMAGE_HEIGHT: u32 = ARIC_ROWS * ARIC_FRAME_SIZE;
const ARIC_ANIMATION_OWNERS: [u32; 4] = [0, 9, 18, 27];

/// One tileset-local frame and its pixel rectangle within the atlas image.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct AtlasFrame {
    tile_id: u32,
    rectangle: AtlasRectangle,
}

/// One authored walk frame with its exact TSX duration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct AtlasAnimationFrame {
    tile_id: u32,
    duration_ms: u32,
}

impl AtlasAnimationFrame {
    pub(crate) const fn tile_id(self) -> u32 {
        self.tile_id
    }

    pub(crate) const fn duration_ms(self) -> u32 {
        self.duration_ms
    }
}

impl AtlasFrame {
    pub(crate) const fn tile_id(self) -> u32 {
        self.tile_id
    }

    pub(crate) const fn rectangle(self) -> AtlasRectangle {
        self.rectangle
    }
}

/// An integer pixel rectangle within one atlas image.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct AtlasRectangle {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl AtlasRectangle {
    pub(crate) const fn x(self) -> u32 {
        self.x
    }

    pub(crate) const fn y(self) -> u32 {
        self.y
    }

    pub(crate) const fn width(self) -> u32 {
        self.width
    }

    pub(crate) const fn height(self) -> u32 {
        self.height
    }
}

/// Validated slicing and directional base-frame selection for Aric's walk atlas.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AricAtlasLayout {
    frame_width: u32,
    frame_height: u32,
    columns: u32,
    rows: u32,
    tile_count: u32,
    directional_base_tiles: [u32; 4],
    directional_walk_frames: [Vec<AtlasAnimationFrame>; 4],
}

impl AricAtlasLayout {
    /// Validates and projects Aric's strict TSX metadata into renderer-independent rectangles.
    pub(crate) fn from_tsx_metadata(
        metadata: &TsxTilesetMetadata,
    ) -> Result<Self, AricAtlasLayoutError> {
        require_value("tile width", metadata.tile_width(), ARIC_FRAME_SIZE)?;
        require_value("tile height", metadata.tile_height(), ARIC_FRAME_SIZE)?;
        require_value("column count", metadata.columns(), ARIC_COLUMNS)?;
        require_value("tile count", metadata.tile_count(), ARIC_TILE_COUNT)?;
        require_value("image width", metadata.image().width(), ARIC_IMAGE_WIDTH)?;
        require_value("image height", metadata.image().height(), ARIC_IMAGE_HEIGHT)?;

        let animation_owners = metadata
            .animations()
            .iter()
            .map(|animation| animation.tile_id())
            .collect::<Vec<_>>();
        if animation_owners != ARIC_ANIMATION_OWNERS {
            return Err(AricAtlasLayoutError::AnimationOwners {
                actual: animation_owners,
            });
        }

        let directional_walk_frames = std::array::from_fn(|index| {
            metadata.animations()[index]
                .frames()
                .iter()
                .map(|frame| AtlasAnimationFrame {
                    tile_id: frame.tile_id(),
                    duration_ms: frame.duration_ms(),
                })
                .collect()
        });

        Ok(Self {
            frame_width: metadata.tile_width(),
            frame_height: metadata.tile_height(),
            columns: metadata.columns(),
            rows: metadata.image().height() / metadata.tile_height(),
            tile_count: metadata.tile_count(),
            directional_base_tiles: ARIC_ANIMATION_OWNERS,
            directional_walk_frames,
        })
    }

    pub(crate) const fn frame_width(&self) -> u32 {
        self.frame_width
    }

    pub(crate) const fn frame_height(&self) -> u32 {
        self.frame_height
    }

    pub(crate) const fn columns(&self) -> u32 {
        self.columns
    }

    pub(crate) const fn rows(&self) -> u32 {
        self.rows
    }

    pub(crate) const fn tile_count(&self) -> u32 {
        self.tile_count
    }

    /// Returns a tileset-local frame rectangle, or `None` when the ID is outside this atlas.
    pub(crate) const fn frame(&self, tile_id: u32) -> Option<AtlasFrame> {
        if tile_id >= self.tile_count {
            return None;
        }
        let column = tile_id % self.columns;
        let row = tile_id / self.columns;
        Some(AtlasFrame {
            tile_id,
            rectangle: AtlasRectangle {
                x: column * self.frame_width,
                y: row * self.frame_height,
                width: self.frame_width,
                height: self.frame_height,
            },
        })
    }

    /// Selects the TSX animation-owner tile used as the idle/base frame for one facing.
    pub(crate) fn base_frame(&self, direction: CardinalDirection) -> AtlasFrame {
        let index = direction_index(direction);
        let tile_id = self.directional_base_tiles[index];
        self.frame(tile_id)
            .expect("validated directional base tile must be inside the Aric atlas")
    }

    /// Returns the exact ordered TSX walk frames for one cardinal facing.
    pub(crate) fn walk_frames(&self, direction: CardinalDirection) -> &[AtlasAnimationFrame] {
        &self.directional_walk_frames[direction_index(direction)]
    }
}

const fn direction_index(direction: CardinalDirection) -> usize {
    match direction {
        CardinalDirection::Up => 0,
        CardinalDirection::Left => 1,
        CardinalDirection::Down => 2,
        CardinalDirection::Right => 3,
    }
}

/// A failure to match the strict authored Aric atlas profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum AricAtlasLayoutError {
    Geometry {
        field: &'static str,
        expected: u32,
        actual: u32,
    },
    AnimationOwners {
        actual: Vec<u32>,
    },
}

impl fmt::Display for AricAtlasLayoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Geometry {
                field,
                expected,
                actual,
            } => write!(
                formatter,
                "Aric atlas {field} must be {expected}, found {actual}"
            ),
            Self::AnimationOwners { actual } => write!(
                formatter,
                "Aric atlas animation owners must be [0, 9, 18, 27], found {actual:?}"
            ),
        }
    }
}

impl Error for AricAtlasLayoutError {}

fn require_value(
    field: &'static str,
    actual: u32,
    expected: u32,
) -> Result<(), AricAtlasLayoutError> {
    if actual == expected {
        Ok(())
    } else {
        Err(AricAtlasLayoutError::Geometry {
            field,
            expected,
            actual,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{ARIC_ANIMATION_OWNERS, AricAtlasLayout, AricAtlasLayoutError, AtlasRectangle};
    use crate::{
        scenario_path::ScenarioRelativePath, scenario_spatial::CardinalDirection,
        tsx_metadata::parse_tsx_tileset_metadata,
    };

    const COPIED_ARIC_TSX: &str = include_str!(
        "../../../../assets/scenarios/rusted_kingdoms/assets/sprites/party/01_aric_walk.tsx"
    );

    fn metadata(xml: &str) -> crate::tsx_metadata::TsxTilesetMetadata {
        let owner = ScenarioRelativePath::try_from("assets/sprites/party/01_aric_walk.tsx")
            .expect("fixture owner should be safe");
        parse_tsx_tileset_metadata(xml, &owner).expect("fixture TSX should parse")
    }

    #[test]
    fn copied_aric_metadata_slices_576_by_256_into_nine_by_four_64_pixel_cells() {
        let layout = AricAtlasLayout::from_tsx_metadata(&metadata(COPIED_ARIC_TSX))
            .expect("copied Aric metadata should match its authored atlas");

        assert_eq!(layout.frame_width(), 64);
        assert_eq!(layout.frame_height(), 64);
        assert_eq!(layout.columns(), 9);
        assert_eq!(layout.rows(), 4);
        assert_eq!(layout.tile_count(), 36);
        assert_eq!(
            layout.frame(0).expect("first frame").rectangle(),
            AtlasRectangle {
                x: 0,
                y: 0,
                width: 64,
                height: 64,
            }
        );
        assert_eq!(
            layout.frame(35).expect("last frame").rectangle(),
            AtlasRectangle {
                x: 512,
                y: 192,
                width: 64,
                height: 64,
            }
        );
        assert!(layout.frame(36).is_none());
    }

    #[test]
    fn cardinal_directions_select_the_exact_tsx_animation_owner_base_frames() {
        let layout = AricAtlasLayout::from_tsx_metadata(&metadata(COPIED_ARIC_TSX)).unwrap();

        for ((direction, expected_tile), expected_y) in [
            (CardinalDirection::Up, 0_u32),
            (CardinalDirection::Left, 9),
            (CardinalDirection::Down, 18),
            (CardinalDirection::Right, 27),
        ]
        .into_iter()
        .zip([0_u32, 64, 128, 192])
        {
            let frame = layout.base_frame(direction);
            assert_eq!(frame.tile_id(), expected_tile);
            assert_eq!(frame.rectangle().x(), 0);
            assert_eq!(frame.rectangle().y(), expected_y);
            assert_eq!(frame.rectangle().width(), 64);
            assert_eq!(frame.rectangle().height(), 64);
        }
    }

    #[test]
    fn cardinal_walks_retain_exact_authored_frame_ids_and_durations() {
        let layout = AricAtlasLayout::from_tsx_metadata(&metadata(COPIED_ARIC_TSX)).unwrap();

        for (direction, expected_tiles) in [
            (CardinalDirection::Up, 1_u32..=8),
            (CardinalDirection::Left, 10..=17),
            (CardinalDirection::Down, 19..=26),
            (CardinalDirection::Right, 28..=35),
        ] {
            let frames = layout.walk_frames(direction);
            assert_eq!(
                frames
                    .iter()
                    .map(|frame| frame.tile_id())
                    .collect::<Vec<_>>(),
                expected_tiles.collect::<Vec<_>>()
            );
            assert!(frames.iter().all(|frame| frame.duration_ms() == 100));
        }
    }

    #[test]
    fn rejects_non_aric_geometry_before_exposing_slices() {
        let taller = COPIED_ARIC_TSX
            .replace("tilecount=\"36\"", "tilecount=\"45\"")
            .replace("height=\"256\"", "height=\"320\"");

        assert_eq!(
            AricAtlasLayout::from_tsx_metadata(&metadata(&taller)),
            Err(AricAtlasLayoutError::Geometry {
                field: "tile count",
                expected: 36,
                actual: 45,
            })
        );
    }

    #[test]
    fn rejects_direction_rows_that_do_not_match_the_tsx_animation_owners() {
        let wrong_owner = COPIED_ARIC_TSX.replacen("<tile id=\"9\">", "<tile id=\"10\">", 1);

        assert_eq!(
            AricAtlasLayout::from_tsx_metadata(&metadata(&wrong_owner)),
            Err(AricAtlasLayoutError::AnimationOwners {
                actual: vec![0, 10, 18, 27],
            })
        );
        assert_eq!(ARIC_ANIMATION_OWNERS, [0, 9, 18, 27]);
    }
}
