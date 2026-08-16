//! Source-authored battle-background ground-placement schemas.
//!
//! The pinned `0897035` scenario has one sequence-root `data/battle_backgrounds.yaml` catalog
//! with thirteen records. Each record identifies a battle background and its rectangular visible
//! ground area in that background image's pixel coordinate space. The Python loader derives the
//! image bounds from an optional `-<width>x<height>` suffix on the id and rejects rectangles that
//! leave those bounds. It does not define defaults, load assets, or render from this catalog.

use std::fmt;

use bevy::{asset::Asset, reflect::TypePath};
use serde::{Deserialize, Deserializer};

use crate::scenario_yaml::deserialize_string;

/// The sequence-root battle-background catalog selected by `manifest.refs.battle_backgrounds`.
///
/// Order and duplicate ids are retained as authored. The Python runtime later indexes records by
/// id; lookup, encounter cross-reference checks, and rendering remain separate concerns.
#[derive(Asset, Clone, Debug, Deserialize, Eq, PartialEq, TypePath)]
#[serde(transparent)]
pub struct BattleBackgroundCatalog(pub Vec<BattleBackground>);

/// One battle background's authored identifier and visible ground rectangle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BattleBackground {
    /// A renderer-facing background id, not a filesystem path.
    pub id: String,
    pub ground_rect: GroundRect,
}

/// A rectangle in a battle background image's native pixel coordinate space.
///
/// Python accepts signed integer components until the id encodes a canvas size, at which point
/// [`BattleBackground`] rejects negative origins and overshoot. Zero or negative extents without
/// such a suffix are retained because the source loader does not otherwise constrain them.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GroundRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl GroundRect {
    pub fn right(self) -> i64 {
        i64::from(self.x) + i64::from(self.width)
    }

    pub fn bottom(self) -> i64 {
        i64::from(self.y) + i64::from(self.height)
    }
}

impl<'de> Deserialize<'de> for GroundRect {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Document {
            x: i32,
            y: i32,
            width: i32,
            height: i32,
        }

        let document = Document::deserialize(deserializer)?;
        Ok(Self {
            x: document.x,
            y: document.y,
            width: document.width,
            height: document.height,
        })
    }
}

impl<'de> Deserialize<'de> for BattleBackground {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Document {
            #[serde(deserialize_with = "deserialize_string")]
            id: String,
            ground_rect: GroundRect,
        }

        let document = Document::deserialize(deserializer)?;
        let background = Self {
            id: document.id,
            ground_rect: document.ground_rect,
        };
        background
            .validate_id_canvas()
            .map_err(serde::de::Error::custom)?;
        Ok(background)
    }
}

impl BattleBackground {
    /// Returns the dimensions encoded by the source id suffix, when it has one.
    pub fn encoded_canvas_size(&self) -> Option<(u32, u32)> {
        let suffix = self.id.rsplit_once('-')?.1;
        let (width, height) = suffix.split_once('x')?;
        if width.is_empty()
            || height.is_empty()
            || !width.bytes().all(|byte| byte.is_ascii_digit())
            || !height.bytes().all(|byte| byte.is_ascii_digit())
        {
            return None;
        }
        Some((width.parse().ok()?, height.parse().ok()?))
    }

    fn validate_id_canvas(&self) -> Result<(), GroundRectBoundsError> {
        let Some((canvas_width, canvas_height)) = self.encoded_canvas_size() else {
            return Ok(());
        };
        if self.ground_rect.x < 0
            || self.ground_rect.y < 0
            || self.ground_rect.right() > i64::from(canvas_width)
            || self.ground_rect.bottom() > i64::from(canvas_height)
        {
            return Err(GroundRectBoundsError {
                id: self.id.clone(),
                rect: self.ground_rect,
                canvas_width,
                canvas_height,
            });
        }
        Ok(())
    }
}

/// A suffix-described canvas cannot contain an authored ground rectangle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GroundRectBoundsError {
    id: String,
    rect: GroundRect,
    canvas_width: u32,
    canvas_height: u32,
}

impl fmt::Display for GroundRectBoundsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "background {:?}: ground_rect {:?} falls outside the {}x{} image bounds (right={}, bottom={})",
            self.id,
            self.rect,
            self.canvas_width,
            self.canvas_height,
            self.rect.right(),
            self.rect.bottom(),
        )
    }
}

impl std::error::Error for GroundRectBoundsError {}

#[cfg(test)]
mod tests {
    use super::{BattleBackgroundCatalog, GroundRect};
    use crate::scenario_yaml;
    use std::fs;

    #[test]
    fn loads_source_shaped_catalog_and_preserves_rectangles_in_order() {
        let catalog: BattleBackgroundCatalog = scenario_yaml::from_str(include_str!(
            "../tests/fixtures/battle-background-catalog.yaml"
        ))
        .expect("source-shaped background catalog should deserialize");

        assert_eq!(catalog.0.len(), 3);
        assert_eq!(catalog.0[0].id, "invented-grove-1280x468");
        assert_eq!(
            catalog.0[1].ground_rect,
            GroundRect {
                x: 120,
                y: 310,
                width: 910,
                height: 158
            }
        );
        assert_eq!(catalog.0[1].ground_rect.right(), 1030);
        assert_eq!(catalog.0[1].ground_rect.bottom(), 468);
        assert_eq!(catalog.0[2].encoded_canvas_size(), None);
    }

    #[test]
    fn accepts_only_source_loader_bounds_and_no_catalog_defaults() {
        let catalog: BattleBackgroundCatalog = scenario_yaml::from_str(
            "- id: opaque-runtime-key\n  ground_rect: { x: -9, y: 0, width: -1, height: 1 }\n- id: empty-ground-4x3\n  ground_rect: { x: 4, y: 3, width: 0, height: 0 }\n",
        )
        .expect("ids without a dimension suffix and zero-area contained rectangles are allowed");
        assert_eq!(catalog.0.len(), 2);

        for document in [
            "[]\n",
            "- id: invented-4x3\n  ground_rect: { x: 4, y: 0, width: 1, height: 1 }\n",
            "- id: invented-4x3\n  ground_rect: { x: 0, y: 3, width: 1, height: 1 }\n",
        ] {
            let result = scenario_yaml::from_str::<BattleBackgroundCatalog>(document);
            if document == "[]\n" {
                assert!(
                    result.is_ok(),
                    "empty sequence has no source-level default requirement"
                );
            } else {
                assert!(result.is_err(), "unexpectedly accepted:\n{document}");
            }
        }
    }

    #[test]
    fn rejects_missing_null_coerced_fractional_negative_and_unknown_shapes() {
        let valid = include_str!("../tests/fixtures/battle-background-catalog.yaml");
        for document in [
            valid.replacen("id: invented-grove-1280x468", "id: true", 1),
            valid.replacen("x: 0", "x: '0'", 1),
            valid.replacen("y: 0", "y: 1.5", 1),
            valid.replacen("width: 1280", "width: null", 1),
            valid.replacen("height: 468", "height: true", 1),
            valid.replacen("ground_rect:", "unknown: nope\n  ground_rect:", 1),
            "- id: absent-1280x468\n".to_owned(),
            "- ground_rect: { x: 0, y: 0, width: 1, height: 1 }\n".to_owned(),
            "- id: null\n  ground_rect: { x: 0, y: 0, width: 1, height: 1 }\n".to_owned(),
            "- id: test-2x2\n  ground_rect: { x: 0, y: 0, width: 1, height: 1, depth: 3 }\n"
                .to_owned(),
            "- id: test-2x2\n  ground_rect: null\n".to_owned(),
        ] {
            assert!(
                scenario_yaml::from_str::<BattleBackgroundCatalog>(&document).is_err(),
                "unexpectedly accepted:\n{document}"
            );
        }
    }

    #[test]
    #[ignore = "requires the separately licensed pinned Python source checkout"]
    fn audits_the_complete_pinned_battle_background_catalog_when_requested() {
        let path = std::env::var_os("RPG_S1_PINNED_BATTLE_BACKGROUNDS_FILE")
            .map(std::path::PathBuf::from)
            .expect("RPG_S1_PINNED_BATTLE_BACKGROUNDS_FILE must name data/battle_backgrounds.yaml");
        let document = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{} should be readable: {error}", path.display()));
        let catalog: BattleBackgroundCatalog = scenario_yaml::from_str(&document)
            .unwrap_or_else(|error| panic!("{} should load: {error}", path.display()));

        assert_eq!(catalog.0.len(), 13);
        assert_eq!(
            catalog
                .0
                .iter()
                .filter(|background| background.encoded_canvas_size().is_some())
                .count(),
            13
        );
        assert_eq!(
            catalog
                .0
                .iter()
                .filter(|background| background.ground_rect.x > 0)
                .count(),
            2
        );
        assert_eq!(
            catalog
                .0
                .iter()
                .filter(|background| background.ground_rect.y > 0)
                .count(),
            2
        );
        assert_eq!(
            catalog
                .0
                .iter()
                .filter(|background| background.ground_rect.width != 1280)
                .count(),
            2
        );
        assert_eq!(
            catalog
                .0
                .iter()
                .filter(|background| background.ground_rect.height != 468)
                .count(),
            2
        );
        assert!(catalog.0.iter().all(|background| {
            let (width, height) = background.encoded_canvas_size().unwrap();
            background.ground_rect.right() <= i64::from(width)
                && background.ground_rect.bottom() <= i64::from(height)
        }));
    }
}
