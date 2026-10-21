//! Reusable chest definitions and TMX-owned placements.

use std::collections::BTreeMap;

use bevy::{asset::Asset, reflect::TypePath};
use serde::{
    Deserialize, Deserializer,
    de::{Error, MapAccess, Visitor},
};

use crate::{
    scenario_condition::FlagConditions,
    scenario_map::{ItemBoxLoot, ItemBoxTrap},
    scenario_spatial::Position,
    tmx_header::{TmxMapDocument, TmxTileGid},
};

#[derive(Asset, Clone, Debug, Default, Deserialize, PartialEq, TypePath)]
#[serde(deny_unknown_fields)]
pub struct ItemBoxCatalog {
    #[serde(deserialize_with = "definitions")]
    pub item_boxes: BTreeMap<String, ItemBoxDefinition>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ItemBoxDefinition {
    #[serde(default)]
    pub present: FlagConditions,
    #[serde(default)]
    pub loot: ItemBoxLoot,
    #[serde(default, deserialize_with = "present_trap")]
    pub trap: Option<ItemBoxTrap>,
}

fn present_trap<'de, D: Deserializer<'de>>(d: D) -> Result<Option<ItemBoxTrap>, D::Error> {
    ItemBoxTrap::deserialize(d).map(Some)
}

fn definitions<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<BTreeMap<String, ItemBoxDefinition>, D::Error> {
    struct Definitions;
    impl<'de> Visitor<'de> for Definitions {
        type Value = BTreeMap<String, ItemBoxDefinition>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("unique nonempty item-box definition keys")
        }
        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut result = BTreeMap::new();
            while let Some((key, value)) = map.next_entry::<String, ItemBoxDefinition>()? {
                if key.trim().is_empty() || result.insert(key.clone(), value).is_some() {
                    return Err(A::Error::custom(format!(
                        "empty or duplicate item-box definition `{key}`"
                    )));
                }
            }
            Ok(result)
        }
    }
    d.deserialize_map(Definitions)
}

#[derive(Clone, Debug, PartialEq)]
pub struct ItemBoxPlacement {
    /// Stable placement identity, unique within a map; saved separately from the definition.
    pub instance_id: String,
    pub definition_id: String,
    pub gid: TmxTileGid,
    /// Pixel-space top-left, normalized from Tiled's bottom-left tile-object anchor.
    pub x: f64,
    pub y: f64,
    pub position: Position,
    pub tile_layer_index: usize,
}

pub fn item_box_placements(map: &TmxMapDocument) -> Result<Vec<ItemBoxPlacement>, String> {
    let mut ids = std::collections::BTreeSet::new();
    let mut result = Vec::new();
    let h = map.header();
    for group in map
        .object_groups()
        .iter()
        .filter(|g| g.name() == "item_boxes")
    {
        for object in group.objects() {
            let instance_id = object
                .name()
                .filter(|id| !id.trim().is_empty())
                .ok_or_else(|| format!("item-box object {} needs a unique name", object.id()))?;
            if !ids.insert(instance_id.to_owned()) {
                return Err(format!("duplicate item-box placement `{instance_id}`"));
            }
            let definition_id = object
                .properties()
                .iter()
                .find(|p| p.name() == "item_box_id")
                .and_then(|p| match p.value() {
                    crate::tmx_header::TmxPropertyValue::String(s) if !s.trim().is_empty() => {
                        Some(s.clone())
                    }
                    _ => None,
                })
                .ok_or_else(|| format!("item-box `{instance_id}` needs a string item_box_id"))?;
            let gid = object
                .gid()
                .ok_or_else(|| format!("item-box `{instance_id}` must be a tile object"))?;
            if object.width() != f64::from(h.tile_width())
                || object.height() != f64::from(h.tile_height())
            {
                return Err(format!(
                    "item-box `{instance_id}` must have the map's tile dimensions"
                ));
            }
            let x = object.x();
            let y = object.y() - object.height();
            if x < 0.0
                || y < 0.0
                || x + object.width() > f64::from(h.width()) * f64::from(h.tile_width())
                || y + object.height() > f64::from(h.height()) * f64::from(h.tile_height())
            {
                return Err(format!("item-box `{instance_id}` is outside map bounds"));
            }
            result.push(ItemBoxPlacement {
                instance_id: instance_id.to_owned(),
                definition_id,
                gid,
                x,
                y,
                position: Position::new(
                    ((x + object.width() / 2.0) / f64::from(h.tile_width())).floor() as i32,
                    ((y + object.height() / 2.0) / f64::from(h.tile_height())).floor() as i32,
                ),
                tile_layer_index: group.tile_layer_index(),
            });
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(objects: &str) -> TmxMapDocument {
        let xml = format!(
            r#"<map version="1.10" orientation="orthogonal" width="4" height="4" tilewidth="32" tileheight="32" infinite="0"><tileset firstgid="1" source="../sprites/objects/item_box.tsx"/><objectgroup id="1" name="item_boxes">{objects}</objectgroup></map>"#
        );
        crate::tmx_header::parse_tmx_map_document(
            &xml,
            &"media/maps/example.tmx".try_into().unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn shared_definition_keeps_each_placement_identity_and_fractional_position() {
        let doc = map(
            r#"<object id="1" name="one" gid="1" x="0" y="32" width="32" height="32"><properties><property name="item_box_id" value="supplies"/></properties></object><object id="2" name="two" gid="1" x="63.5" y="97.25" width="32" height="32"><properties><property name="item_box_id" value="supplies"/></properties></object>"#,
        );
        let boxes = item_box_placements(&doc).unwrap();
        assert_eq!(boxes.len(), 2);
        assert_eq!(boxes[0].definition_id, boxes[1].definition_id);
        assert_ne!(boxes[0].instance_id, boxes[1].instance_id);
        assert_eq!((boxes[1].x, boxes[1].y), (63.5, 65.25));
        assert_eq!(boxes[1].position, Position::new(2, 2));
    }

    #[test]
    fn definitions_reject_positions_duplicate_keys_and_null_traps() {
        for yaml in [
            "item_boxes: {supplies: {position: [1, 2]}}",
            "item_boxes:\n  supplies: {}\n  supplies: {}\n",
            "item_boxes: {supplies: {trap: null}}",
            "item_boxes: {'': {}}",
        ] {
            assert!(
                crate::scenario_yaml::from_str::<ItemBoxCatalog>(yaml).is_err(),
                "{yaml}"
            );
        }
        let catalog: ItemBoxCatalog = crate::scenario_yaml::from_str(
            "item_boxes: {supplies: {loot: {items: [{id: potion, qty: 2}]}}}",
        )
        .unwrap();
        assert_eq!(catalog.item_boxes["supplies"].loot.items[0].qty.get(), 2);
    }

    #[test]
    fn placement_requires_definition_id_unique_name_and_in_bounds_image() {
        let valid = r#"<object id="1" name="one" gid="1" x="0" y="32" width="32" height="32"><properties><property name="item_box_id" value="supplies"/></properties></object>"#;
        for invalid in [
            valid.replace("name=\"one\"", ""),
            valid.replace("item_box_id", "other"),
            valid.replace("x=\"0\"", "x=\"128\""),
            format!("{valid}{}", valid.replace("id=\"1\"", "id=\"2\"")),
        ] {
            assert!(item_box_placements(&map(&invalid)).is_err(), "{invalid}");
        }
    }
}
