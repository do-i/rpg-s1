//! Renderer-free collision occupancy derived from one strict TMX collision layer.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "M4.14 establishes collision data before M4.19 movement consumes it"
    )
)]

use std::fmt;

use crate::tmx_header::TmxMapDocument;

/// Row-major blocked/open occupancy for one finite tile map.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CollisionOccupancy {
    width: u32,
    height: u32,
    tile_width: u32,
    tile_height: u32,
    blocked: Vec<bool>,
}

impl CollisionOccupancy {
    /// Builds occupancy from the document's one exact `collision` layer.
    pub(crate) fn from_tmx_document(
        document: &TmxMapDocument,
    ) -> Result<Self, CollisionOccupancyError> {
        let mut collision_layers = document
            .tile_layers()
            .iter()
            .filter(|layer| layer.name() == "collision");
        let layer = collision_layers
            .next()
            .ok_or(CollisionOccupancyError::MissingCollisionLayer)?;
        if collision_layers.next().is_some() {
            return Err(CollisionOccupancyError::MultipleCollisionLayers);
        }

        let width = document.header().width();
        let height = document.header().height();
        let expected_cells = usize::try_from(width)
            .ok()
            .and_then(|width| {
                usize::try_from(height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .ok_or(CollisionOccupancyError::DimensionsTooLarge { width, height })?;
        if layer.width() != width
            || layer.height() != height
            || layer.gids().len() != expected_cells
        {
            return Err(CollisionOccupancyError::CollisionLayerShapeMismatch {
                map_width: width,
                map_height: height,
                layer_width: layer.width(),
                layer_height: layer.height(),
                cells: layer.gids().len(),
            });
        }

        Ok(Self {
            width,
            height,
            tile_width: document.header().tile_width(),
            tile_height: document.header().tile_height(),
            blocked: layer.gids().iter().map(|gid| !gid.is_empty()).collect(),
        })
    }

    pub(crate) const fn width(&self) -> u32 {
        self.width
    }

    pub(crate) const fn height(&self) -> u32 {
        self.height
    }

    /// Returns `None` outside the finite map and otherwise whether the cell is blocked.
    pub(crate) fn is_blocked(&self, column: i32, row: i32) -> Option<bool> {
        let column = u32::try_from(column).ok()?;
        let row = u32::try_from(row).ok()?;
        if column >= self.width || row >= self.height {
            return None;
        }
        let index = usize::try_from(row.checked_mul(self.width)?.checked_add(column)?).ok()?;
        self.blocked.get(index).copied()
    }

    /// Returns `None` outside the finite map and otherwise whether the cell is open.
    pub(crate) fn is_open(&self, column: i32, row: i32) -> Option<bool> {
        self.is_blocked(column, row).map(|blocked| !blocked)
    }

    /// Checks the four corners of a source-pixel collision rectangle.
    pub(crate) fn is_rect_blocked(&self, x: f32, y: f32, width: f32, height: f32) -> bool {
        let right = x + width - 1.0;
        let bottom = y + height - 1.0;
        [[x, y], [right, y], [x, bottom], [right, bottom]]
            .into_iter()
            .any(|[px, py]| {
                self.is_blocked(
                    (px / self.tile_width as f32).floor() as i32,
                    (py / self.tile_height as f32).floor() as i32,
                ) != Some(false)
            })
    }
}

/// A strict reserved-layer contract failure while building collision occupancy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum CollisionOccupancyError {
    MissingCollisionLayer,
    MultipleCollisionLayers,
    DimensionsTooLarge {
        width: u32,
        height: u32,
    },
    CollisionLayerShapeMismatch {
        map_width: u32,
        map_height: u32,
        layer_width: u32,
        layer_height: u32,
        cells: usize,
    },
}

impl fmt::Display for CollisionOccupancyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCollisionLayer => {
                formatter.write_str("TMX document must contain one `collision` tile layer")
            }
            Self::MultipleCollisionLayers => {
                formatter.write_str("TMX document contains more than one `collision` tile layer")
            }
            Self::DimensionsTooLarge { width, height } => {
                write!(formatter, "map dimensions {width}x{height} are too large")
            }
            Self::CollisionLayerShapeMismatch {
                map_width,
                map_height,
                layer_width,
                layer_height,
                cells,
            } => write!(
                formatter,
                "collision layer {layer_width}x{layer_height} with {cells} cells does not match map {map_width}x{map_height}"
            ),
        }
    }
}

impl std::error::Error for CollisionOccupancyError {}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::*;
    use crate::{scenario_path::ScenarioRelativePath, tmx_header::parse_tmx_map_document};

    fn invented_document(layers: &str) -> String {
        format!(
            r#"<map orientation="orthogonal" width="2" height="2" tilewidth="32" tileheight="32">{layers}</map>"#
        )
    }

    fn invented_path() -> ScenarioRelativePath {
        ScenarioRelativePath::try_from("assets/maps/invented.tmx").unwrap()
    }

    #[test]
    fn builds_row_major_occupancy_from_nonzero_decoded_collision_gids() {
        let xml = invented_document(
            r#"
                <layer id="1" name="ground" width="2" height="2">
                    <data encoding="csv">1,1,
                    1,1</data>
                </layer>
                <layer id="2" name="collision" width="2" height="2" visible="0">
                    <data encoding="csv">1,0,
                    0,2147483649</data>
                </layer>
            "#,
        );
        let document = parse_tmx_map_document(&xml, &invented_path()).unwrap();
        let occupancy = CollisionOccupancy::from_tmx_document(&document).unwrap();

        assert_eq!((occupancy.width(), occupancy.height()), (2, 2));
        assert_eq!(occupancy.is_blocked(0, 0), Some(true));
        assert_eq!(occupancy.is_blocked(1, 0), Some(false));
        assert_eq!(occupancy.is_blocked(0, 1), Some(false));
        assert_eq!(occupancy.is_blocked(1, 1), Some(true));
        assert_eq!(occupancy.is_open(0, 0), Some(false));
        assert_eq!(occupancy.is_open(1, 0), Some(true));
        assert!(!occupancy.is_rect_blocked(36.0, 4.0, 20.0, 18.0));
        assert!(occupancy.is_rect_blocked(20.0, 20.0, 20.0, 18.0));
    }

    #[test]
    fn all_four_outside_boundaries_are_safe() {
        let xml = invented_document(
            r#"<layer id="1" name="collision" width="2" height="2"><data encoding="csv">0,0,
            0,0</data></layer>"#,
        );
        let document = parse_tmx_map_document(&xml, &invented_path()).unwrap();
        let occupancy = CollisionOccupancy::from_tmx_document(&document).unwrap();

        for coordinate in [(-1, 0), (2, 0), (0, -1), (0, 2)] {
            assert_eq!(occupancy.is_blocked(coordinate.0, coordinate.1), None);
            assert_eq!(occupancy.is_open(coordinate.0, coordinate.1), None);
        }
    }

    #[test]
    fn requires_exactly_one_collision_layer() {
        let missing = parse_tmx_map_document(
            &invented_document(
                r#"<layer id="1" name="ground" width="2" height="2"><data encoding="csv">0,0,
                0,0</data></layer>"#,
            ),
            &invented_path(),
        )
        .unwrap();
        assert_eq!(
            CollisionOccupancy::from_tmx_document(&missing),
            Err(CollisionOccupancyError::MissingCollisionLayer)
        );

        let duplicate = parse_tmx_map_document(
            &invented_document(
                r#"
                    <layer id="1" name="collision" width="2" height="2"><data encoding="csv">0,0,
                    0,0</data></layer>
                    <layer id="2" name="collision" width="2" height="2"><data encoding="csv">0,0,
                    0,0</data></layer>
                "#,
            ),
            &invented_path(),
        )
        .unwrap();
        assert_eq!(
            CollisionOccupancy::from_tmx_document(&duplicate),
            Err(CollisionOccupancyError::MultipleCollisionLayers)
        );
    }

    #[test]
    #[ignore = "requires the separately pinned Python scenario checkout"]
    fn pinned_ardel_known_blocked_and_open_cells_match_source() {
        let maps = std::env::var_os("RPG_S1_PINNED_TMX_DIR")
            .expect("RPG_S1_PINNED_TMX_DIR must name the pinned assets/maps directory");
        let maps = Path::new(&maps);
        let scenario_root = maps
            .parent()
            .and_then(Path::parent)
            .expect("TMX directory should be nested below the scenario root");
        let path = maps.join("town_01_ardel.tmx");
        let logical = path
            .strip_prefix(scenario_root)
            .expect("Ardel TMX should be inside scenario root")
            .to_str()
            .expect("pinned scenario paths should be UTF-8");
        let logical = ScenarioRelativePath::try_from(logical).unwrap();
        let xml = fs::read_to_string(path).expect("Ardel TMX should be readable");
        let document = parse_tmx_map_document(&xml, &logical).unwrap();
        let occupancy = CollisionOccupancy::from_tmx_document(&document).unwrap();

        assert_eq!((occupancy.width(), occupancy.height()), (30, 20));
        for coordinate in [(0, 0), (28, 1), (1, 6), (7, 7)] {
            assert_eq!(
                occupancy.is_blocked(coordinate.0, coordinate.1),
                Some(true),
                "expected {coordinate:?} to be blocked"
            );
        }
        for coordinate in [(10, 0), (29, 1), (2, 6), (8, 7)] {
            assert_eq!(
                occupancy.is_open(coordinate.0, coordinate.1),
                Some(true),
                "expected {coordinate:?} to be open"
            );
        }
    }
}
