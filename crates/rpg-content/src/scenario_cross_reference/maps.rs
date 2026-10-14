//! Map metadata and transport references, including landing tiles.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    context::{Validator, probe_path},
    index::ReferenceIndex,
};
use crate::{
    manifest_path_validation::ScenarioPathProbeResult, scenario_path::ScenarioRelativePath,
};

/// True when `candidate` names a numbered TMX segment of `parent`: `parent` followed by `_` and
/// one or more ASCII digits, e.g. `zone_05_mountain_foothills_02` under
/// `zone_05_mountain_foothills`. Kept in sync with the identical helper in
/// `scenario_map_report`, which reports the same convention in detail; duplicated rather than
/// shared because it is a single pure predicate and the two modules otherwise have no coupling.
fn is_numeric_tmx_segment(parent: &str, candidate: &str) -> bool {
    candidate
        .strip_prefix(parent)
        .and_then(|rest| rest.strip_prefix('_'))
        .is_some_and(|suffix| {
            !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
        })
}

impl<'a> Validator<'a> {
    pub(super) fn validate_maps(&mut self, index: &ReferenceIndex) {
        let maps = self.catalogs.maps.clone();
        for file in maps {
            if let Some(bgm) = &file.value.bgm {
                self.checked("BGM", &index.bgm, bgm, &file.path, "bgm");
            }
            let tmx_path = format!("{}/{}.tmx", index.tmx_root, file.stem);
            self.report.checked_references += 1;
            let tmx_missing = ScenarioRelativePath::try_from(tmx_path.as_str()).is_ok_and(|path| {
                probe_path(self.physical_root, self.canonical_root.as_deref(), &path)
                    != ScenarioPathProbeResult::File
            });
            // Neither engine resets the track on a map change that names no BGM: the map simply
            // keeps playing whatever was already sounding. So an unscored playable map does not
            // fall silent, it inherits -- which is how the whole Ancient Ruins arc spent the
            // game playing town music, and how the marshland did the same (roadmap B1.3).
            // A map with no TMX of its own is reported separately below and is not playable, so
            // it is not held to this.
            if !tmx_missing && file.value.bgm.is_none() {
                self.warning(
                    "map.missing_bgm",
                    &file.path,
                    "bgm",
                    "map declares no `bgm`, so it inherits whatever track was already playing"
                        .to_owned(),
                );
            }

            // A map YAML with no same-stem TMX is unreadable at runtime either way: both engines
            // resolve per-map metadata by exact stem (`data/maps/<map_id>.yaml`), with no
            // parent-to-segment fallback anywhere.
            //
            // This previously stayed silent whenever numbered segment TMX files
            // (`<stem>_01.tmx`, `<stem>_02.tmx`, ...) named the file as their parent, treating
            // that as the source's multi-segment convention. It is not a convention the runtime
            // honours, and the suppression is exactly what hid `zone_05_mountain_foothills.yaml`
            // stranding a BGM key, a `warp_order`, and two unreachable chests (roadmap B2.1).
            // The pinned engine reaches the same conclusion: `_is_submap` builds its
            // id set from `media/maps/*.tmx` stems, so a parent with no TMX of its own is not a
            // submap parent there either.
            //
            // So the segment case is now the *louder* one -- it means authored content exists and
            // no map can load it -- and the repair is to name the file after the segment it
            // describes.
            let segments = index
                .maps
                .iter()
                .filter(|candidate| is_numeric_tmx_segment(&file.stem, candidate))
                .cloned()
                .collect::<Vec<_>>();
            if tmx_missing {
                if segments.is_empty() {
                    self.warning(
                        "source.unmatched_map_metadata",
                        &file.path,
                        "$same_stem_tmx",
                        format!("no same-stem TMX file exists at `{tmx_path}`"),
                    );
                } else {
                    self.error(
                        "source.unreadable_map_metadata",
                        &file.path,
                        "$same_stem_tmx",
                        format!(
                            "no same-stem TMX file exists at `{tmx_path}`, so nothing can load \
                             this metadata; its numbered segments ({}) each resolve their own \
                             `data/maps/<id>.yaml` and none of them is this file",
                            segments.join(", ")
                        ),
                    );
                }
            }
            for (section, shop) in [
                ("shop", file.value.shop.as_ref()),
                ("weapon_shop", file.value.weapon_shop.as_ref()),
                ("armor_shop", file.value.armor_shop.as_ref()),
            ] {
                if let Some(shop) = shop {
                    let (namespace, known) = match section {
                        "shop" => ("consumable item", &index.consumables),
                        "weapon_shop" => ("weapon", &index.weapons),
                        "armor_shop" => ("armor equipment", &index.armor_equipment),
                        _ => unreachable!("the map schema has exactly three shop sections"),
                    };
                    for (item_index, item) in shop.items.iter().enumerate() {
                        self.checked(
                            namespace,
                            known,
                            item.id(),
                            &file.path,
                            format!("{section}.items[{item_index}].id"),
                        );
                    }
                }
            }
            for (npc_index, npc) in file.value.npcs.iter().enumerate() {
                self.checked(
                    "dialogue",
                    &index.dialogue,
                    npc.effective_dialogue_id(),
                    &file.path,
                    format!("npcs[{npc_index}].dialogue"),
                );
                if let Some(excuses) = &npc.excuses {
                    self.checked(
                        "dialogue",
                        &index.dialogue,
                        excuses,
                        &file.path,
                        format!("npcs[{npc_index}].excuses"),
                    );
                }
                if let Some(sprite) = &npc.sprite {
                    self.checked_path(
                        &file.path,
                        format!("npcs[{npc_index}].sprite"),
                        sprite.as_str(),
                    );
                }
            }
            for (box_index, item_box) in file.value.item_boxes.iter().enumerate() {
                if item_box.loot.items.is_empty() && item_box.loot.magic_cores.is_empty() {
                    self.warning(
                        "content.empty_loot",
                        &file.path,
                        format!("item_boxes[{box_index}].loot"),
                        format!("item box `{}` has empty loot", item_box.id),
                    );
                }
                for (item_index, item) in item_box.loot.items.iter().enumerate() {
                    self.checked(
                        "item",
                        &index.items,
                        &item.id,
                        &file.path,
                        format!("item_boxes[{box_index}].loot.items[{item_index}].id"),
                    );
                }
            }
        }
    }

    pub(super) fn validate_transport(&mut self, index: &ReferenceIndex) {
        let Some(file) = self.catalogs.transport.clone() else {
            return;
        };
        self.checked_path(&file.path, "map_image", file.value.map_image.as_str());
        let mut regions = BTreeSet::new();
        let mut assigned_maps = BTreeMap::<String, usize>::new();
        for (region_index, region) in file.value.regions.iter().enumerate() {
            if !regions.insert(region.id.clone()) {
                self.error(
                    "id.duplicate",
                    &file.path,
                    format!("regions[{region_index}].id"),
                    format!("duplicate atlas region id `{}`", region.id),
                );
            }
            for (map_index, map) in region.maps.iter().enumerate() {
                self.checked(
                    "map",
                    &index.maps,
                    &map.id,
                    &file.path,
                    format!("regions[{region_index}].maps[{map_index}].id"),
                );
                *assigned_maps.entry(map.id.clone()).or_default() += 1;
            }
        }
        for map_id in &index.maps {
            match assigned_maps.get(map_id).copied().unwrap_or_default() {
                1 => {}
                0 => self.error(
                    "transport.region_missing",
                    &file.path,
                    "regions",
                    format!("map `{map_id}` belongs to no atlas region"),
                ),
                count => self.error(
                    "transport.region_multiple",
                    &file.path,
                    "regions",
                    format!("map `{map_id}` belongs to {count} atlas regions"),
                ),
            }
        }
        let mut berth_ids = BTreeSet::new();
        for (kind, anchors) in [
            ("fly_anchors", &file.value.fly_anchors),
            ("sail_berths", &file.value.sail_berths),
        ] {
            let mut ids = BTreeSet::new();
            for (anchor_index, anchor) in anchors.iter().enumerate() {
                if !ids.insert(anchor.id.clone()) {
                    self.error(
                        "id.duplicate",
                        &file.path,
                        format!("{kind}[{anchor_index}].id"),
                        format!("duplicate transport anchor id `{}`", anchor.id),
                    );
                }
                if kind == "sail_berths" {
                    berth_ids.insert(anchor.id.clone());
                }
                self.checked(
                    "map",
                    &index.maps,
                    &anchor.map,
                    &file.path,
                    format!("{kind}[{anchor_index}].map"),
                );
                if !regions.contains(&anchor.region) {
                    self.error(
                        "reference.missing",
                        &file.path,
                        format!("{kind}[{anchor_index}].region"),
                        format!("unknown atlas region `{}`", anchor.region),
                    );
                } else if file
                    .value
                    .regions
                    .iter()
                    .find(|region| region.id == anchor.region)
                    .is_some_and(|region| !region.maps.iter().any(|map| map.id == anchor.map))
                {
                    self.error(
                        "transport.anchor_region",
                        &file.path,
                        format!("{kind}[{anchor_index}].map"),
                        format!(
                            "transport anchor `{}` lands on map `{}` outside region `{}`",
                            anchor.id, anchor.map, anchor.region
                        ),
                    );
                }
                self.validate_transport_landing(
                    &file.path,
                    kind,
                    anchor_index,
                    &anchor.map,
                    anchor.position,
                );
            }
        }
        let mut edges = BTreeSet::new();
        for (edge_index, edge) in file.value.sail_edges.iter().enumerate() {
            if let [a, b] = edge.between.as_slice() {
                for (endpoint, berth) in [(0, a), (1, b)] {
                    if !berth_ids.contains(berth) {
                        self.error(
                            "reference.missing",
                            &file.path,
                            format!("sail_edges[{edge_index}].between[{endpoint}]"),
                            format!("unknown Sail berth `{berth}`"),
                        );
                    }
                }
                if a == b {
                    self.error(
                        "transport.edge_self",
                        &file.path,
                        format!("sail_edges[{edge_index}].between"),
                        format!("Sail berth `{a}` cannot connect to itself"),
                    );
                }
                let canonical = if a <= b {
                    (a.clone(), b.clone())
                } else {
                    (b.clone(), a.clone())
                };
                if !edges.insert(canonical) {
                    self.error(
                        "id.duplicate",
                        &file.path,
                        format!("sail_edges[{edge_index}].between"),
                        format!("duplicate Sail edge `{a}` to `{b}`"),
                    );
                }
            } else {
                self.error(
                    "transport.edge_shape",
                    &file.path,
                    format!("sail_edges[{edge_index}].between"),
                    "Sail edges require exactly two berth ids",
                );
            }
        }
    }

    fn validate_transport_landing(
        &mut self,
        catalog_path: &str,
        kind: &str,
        index: usize,
        map_id: &str,
        position: crate::scenario_spatial::Position,
    ) {
        let Some(manifest) = self.catalogs.manifest.as_ref() else {
            return;
        };
        let logical = format!("{}/{map_id}.tmx", manifest.refs.tmx.as_str());
        let Some(xml) = self.read_text(&logical) else {
            return;
        };
        let logical_path = ScenarioRelativePath::try_from(logical.as_str())
            .expect("manifest TMX root and map id form a valid path");
        let Ok(document) = crate::tmx_header::parse_tmx_map_document(&xml, &logical_path) else {
            return;
        };
        let header = document.header();
        let valid = position.x >= 0
            && position.y >= 0
            && (position.x as u32) < header.width()
            && (position.y as u32) < header.height();
        if !valid {
            self.error(
                "transport.landing_bounds",
                catalog_path,
                format!("{kind}[{index}].position"),
                format!(
                    "landing [{}, {}] is outside map `{map_id}` bounds {}x{}",
                    position.x,
                    position.y,
                    header.width(),
                    header.height()
                ),
            );
            return;
        }
        if document
            .tile_layers()
            .iter()
            .find(|layer| layer.name() == "collision")
            .and_then(|layer| layer.gid_at(position.x as u32, position.y as u32))
            .is_some_and(|gid| !gid.is_empty())
        {
            self.error(
                "transport.landing_collision",
                catalog_path,
                format!("{kind}[{index}].position"),
                format!(
                    "landing [{}, {}] occupies collision on map `{map_id}`",
                    position.x, position.y
                ),
            );
        }
    }
}
