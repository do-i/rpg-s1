//! Strict source-authored catalog for the Ember Atlas transport overlay.

use bevy::{asset::Asset, reflect::TypePath};
use serde::Deserialize;

use crate::{
    scenario_class::UnitInterval,
    scenario_path::ScenarioRelativePath,
    scenario_spatial::Position,
    scenario_yaml::{deserialize_string, deserialize_strings},
};

#[derive(Asset, Clone, Debug, Deserialize, PartialEq, TypePath)]
#[serde(deny_unknown_fields)]
pub struct TransportCatalog {
    pub map_image: ScenarioRelativePath,
    pub sail_unlock_flag: String,
    pub fly_unlock_flag: String,
    pub regions: Vec<AtlasRegion>,
    pub fly_anchors: Vec<TravelAnchor>,
    pub sail_berths: Vec<TravelAnchor>,
    pub sail_edges: Vec<SailEdge>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AtlasRegion {
    #[serde(deserialize_with = "deserialize_string")]
    pub id: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub name: String,
    pub position: AtlasPosition,
    pub maps: Vec<AtlasMap>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AtlasMap {
    #[serde(deserialize_with = "deserialize_string")]
    pub id: String,
    #[serde(default)]
    pub outdoor: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AtlasPosition {
    pub x: UnitInterval,
    pub y: UnitInterval,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TravelAnchor {
    #[serde(deserialize_with = "deserialize_string")]
    pub id: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub name: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub region: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub map: String,
    pub position: Position,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub reveal_flag: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SailEdge {
    #[serde(deserialize_with = "deserialize_strings")]
    pub between: Vec<String>,
}

fn deserialize_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserialize_string(deserializer).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario_yaml;

    #[test]
    fn transport_catalog_is_strict_and_coordinates_are_normalized() {
        let catalog: TransportCatalog = scenario_yaml::from_str(
            "map_image: assets/atlas.png\nsail_unlock_flag: sail\nfly_unlock_flag: fly\nregions:\n  - id: coast\n    name: Coast\n    position: { x: 0.2, y: 0.8 }\n    maps: [{ id: port, outdoor: true }]\nfly_anchors: [{ id: port_air, name: Port, region: coast, map: port, position: [1, 2] }]\nsail_berths: [{ id: port_dock, name: Port Dock, region: coast, map: port, position: [1, 2] }]\nsail_edges: []\n",
        )
        .expect("complete transport catalog should parse");
        assert_eq!(catalog.regions[0].position.x.get(), 0.2);

        for bad in [
            "map_image: assets/atlas.png\nsail_unlock_flag: sail\nfly_unlock_flag: fly\nregions: [{ id: coast, name: Coast, position: { x: 1.2, y: 0.5 }, maps: [] }]\nfly_anchors: []\nsail_berths: []\nsail_edges: []\n",
            "map_image: assets/atlas.png\nsail_unlock_flag: sail\nfly_unlock_flag: fly\nregions: []\nfly_anchors: []\nsail_berths: []\nsail_edges: []\ntypo: true\n",
        ] {
            assert!(scenario_yaml::from_str::<TransportCatalog>(bad).is_err());
        }
    }
}
