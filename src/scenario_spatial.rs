//! Shared spatial values used by source-authored scenario schemas.
//!
//! Rusted Kingdoms writes every authored tile coordinate as a two-element YAML sequence,
//! `[x, y]`. The pinned corpus has non-negative map coordinates, while Python's `Position`
//! value object and its tests intentionally permit negative coordinates, so this shared value
//! preserves the complete signed `i32` domain and leaves map-bound validation to map loading.
//!
//! Authored NPC `default_facing` values are lowercase cardinal strings. The Python field
//! renderer also accepts simultaneous horizontal and vertical input, which creates eight
//! movement vectors but still renders with a cardinal facing. Keeping [`CardinalDirection`]
//! and [`EightWayDirection`] distinct prevents a diagonal movement value from being accepted
//! accidentally where a sprite-facing value is required.

use serde::{Deserialize, Serialize};

pub(crate) mod aric_atlas;
pub(crate) mod cardinal_movement;
pub(crate) mod collision_occupancy;

/// An immutable two-dimensional integer coordinate serialized as source-shaped `[x, y]` YAML.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(from = "[i32; 2]", into = "[i32; 2]")]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    /// Creates a coordinate from its horizontal and vertical components.
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

impl From<[i32; 2]> for Position {
    fn from([x, y]: [i32; 2]) -> Self {
        Self { x, y }
    }
}

impl From<Position> for [i32; 2] {
    fn from(position: Position) -> Self {
        [position.x, position.y]
    }
}

/// One of the four sprite-facing and cardinal movement directions used by the source.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CardinalDirection {
    Up,
    Left,
    Down,
    Right,
}

/// One of the eight simultaneous-input movement directions.
///
/// Diagonal source movement arises from an `(dx, dy)` pair, not an authored YAML field. This
/// type gives later runtime and schema work a strict, serializable domain with canonical
/// snake-case spellings such as `up_right`, without widening cardinal-only facing fields.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EightWayDirection {
    Up,
    UpRight,
    Right,
    DownRight,
    Down,
    DownLeft,
    Left,
    UpLeft,
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::{CardinalDirection, EightWayDirection, Position};
    use crate::scenario_yaml;

    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(deny_unknown_fields)]
    struct SpatialFixture {
        positions: Vec<Position>,
        cardinal_directions: Vec<CardinalDirection>,
        eight_way_directions: Vec<EightWayDirection>,
    }

    #[test]
    fn source_shaped_spatial_values_round_trip_without_loss() {
        let fixture: SpatialFixture =
            scenario_yaml::from_str(include_str!("../tests/fixtures/shared-spatial-values.yaml"))
                .expect("the spatial fixture should deserialize");

        assert_eq!(
            fixture.positions,
            [Position::new(14, 5), Position::new(-5, -3)]
        );
        assert_eq!(
            fixture.cardinal_directions,
            [
                CardinalDirection::Up,
                CardinalDirection::Left,
                CardinalDirection::Down,
                CardinalDirection::Right,
            ]
        );
        assert_eq!(
            fixture.eight_way_directions,
            [
                EightWayDirection::Up,
                EightWayDirection::UpRight,
                EightWayDirection::Right,
                EightWayDirection::DownRight,
                EightWayDirection::Down,
                EightWayDirection::DownLeft,
                EightWayDirection::Left,
                EightWayDirection::UpLeft,
            ]
        );

        let serialized =
            serde_yaml_ng::to_string(&fixture).expect("spatial fixture should serialize to YAML");
        let reparsed: SpatialFixture = scenario_yaml::from_str(&serialized)
            .expect("serialized spatial fixture should deserialize");
        assert_eq!(reparsed, fixture);
    }

    #[test]
    fn position_rejects_non_pair_and_non_integer_shapes() {
        for document in [
            "[14]\n",
            "[14, 5, 2]\n",
            "[14.5, 5]\n",
            "[14, five]\n",
            "{ x: 14, y: 5 }\n",
        ] {
            assert!(
                scenario_yaml::from_str::<Position>(document).is_err(),
                "{document}"
            );
        }
    }

    #[test]
    fn directions_reject_unknown_and_wrong_domain_spellings() {
        for document in ["north\n", "UP\n", "up_right\n", "2\n"] {
            assert!(scenario_yaml::from_str::<CardinalDirection>(document).is_err());
        }
        for document in ["north_east\n", "UP_RIGHT\n", "center\n", "2\n"] {
            assert!(scenario_yaml::from_str::<EightWayDirection>(document).is_err());
        }
    }
}
