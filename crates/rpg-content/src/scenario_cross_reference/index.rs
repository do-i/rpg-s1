//! The index of every id a scenario defines, built once before validation.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};

use super::context::{ScenarioCatalogs, Validator, probe_path};
use crate::{
    manifest_path_validation::ScenarioPathProbeResult,
    scenario_class::AbilityElement,
    scenario_duplicate_id::{CatalogId, CatalogIdLocation, CatalogNamespace},
    scenario_enemy::{EnemyAi, EnemyBehavior, EnemyMove},
    scenario_item::ItemDefinition,
    scenario_path::ScenarioRelativePath,
};

#[derive(Default)]
pub(super) struct ReferenceIndex {
    pub(super) party: BTreeSet<String>,
    pub(super) protagonists: BTreeSet<String>,
    pub(super) classes: BTreeSet<String>,
    pub(super) class_abilities: BTreeMap<String, BTreeSet<String>>,
    pub(super) items: BTreeSet<String>,
    pub(super) consumables: BTreeSet<String>,
    pub(super) weapons: BTreeSet<String>,
    pub(super) shields: BTreeSet<String>,
    pub(super) helmets: BTreeSet<String>,
    pub(super) bodies: BTreeSet<String>,
    pub(super) accessories: BTreeSet<String>,
    pub(super) armor_equipment: BTreeSet<String>,
    pub(super) maps: BTreeSet<String>,
    pub(super) npcs: BTreeMap<String, BTreeSet<String>>,
    pub(super) dialogue: BTreeSet<String>,
    pub(super) enemies: BTreeSet<String>,
    pub(super) boss_enemies: BTreeSet<String>,
    pub(super) enemy_abilities: BTreeSet<String>,
    pub(super) boss_move_sets: BTreeSet<String>,
    pub(super) backgrounds: BTreeSet<String>,
    pub(super) bgm: BTreeSet<String>,
    pub(super) sfx: BTreeSet<String>,
    pub(super) enemy_root: String,
    pub(super) tmx_root: String,
    pub(super) catalog_ids: Vec<CatalogId>,
}

impl ReferenceIndex {
    pub(super) fn from_catalogs(catalogs: &ScenarioCatalogs) -> Self {
        let mut index = Self::default();
        if let Some(manifest) = &catalogs.manifest {
            index.enemy_root = manifest.refs.enemies.as_str().to_owned();
            index.tmx_root = manifest.refs.tmx.as_str().to_owned();
        }
        if let Some(file) = &catalogs.party {
            for (position, member) in file.value.party.iter().enumerate() {
                add_id(
                    &mut index.party,
                    &mut index.catalog_ids,
                    CatalogNamespace::Party,
                    "",
                    &member.data().id,
                    &file.path,
                    format!("party[{position}].id"),
                );
                if member.is_protagonist() {
                    index.protagonists.insert(member.data().id.clone());
                }
            }
        }
        for file in &catalogs.classes {
            add_id(
                &mut index.classes,
                &mut index.catalog_ids,
                CatalogNamespace::Classes,
                "",
                &file.value.class_id,
                &file.path,
                "class",
            );
            for (position, ability) in file.value.abilities.iter().enumerate() {
                index
                    .class_abilities
                    .entry(file.value.class_id.clone())
                    .or_default()
                    .insert(ability.id.clone());
                add_catalog_id(
                    &mut index.catalog_ids,
                    CatalogNamespace::Abilities,
                    "",
                    &ability.id,
                    &file.path,
                    format!("abilities[{position}].id"),
                );
            }
        }
        for file in &catalogs.items {
            for (position, item) in file.value.entries().iter().enumerate() {
                add_id(
                    &mut index.items,
                    &mut index.catalog_ids,
                    CatalogNamespace::Items,
                    "",
                    item.id(),
                    &file.path,
                    format!("[{position}].id"),
                );
                match item {
                    ItemDefinition::Consumable(item) => {
                        index.consumables.insert(item.id.clone());
                    }
                    ItemDefinition::Weapon(item) => {
                        index.weapons.insert(item.id.clone());
                    }
                    ItemDefinition::Shield(item) => {
                        index.shields.insert(item.id.clone());
                        index.armor_equipment.insert(item.id.clone());
                    }
                    ItemDefinition::Helmet(item) => {
                        index.helmets.insert(item.id.clone());
                        index.armor_equipment.insert(item.id.clone());
                    }
                    ItemDefinition::Body(item) => {
                        index.bodies.insert(item.id.clone());
                        index.armor_equipment.insert(item.id.clone());
                    }
                    ItemDefinition::Accessory(item) => {
                        index.accessories.insert(item.id.clone());
                        index.armor_equipment.insert(item.id.clone());
                    }
                    ItemDefinition::Material(_)
                    | ItemDefinition::Key(_)
                    | ItemDefinition::MagicCore(_) => {}
                }
            }
        }
        if let Some(file) = &catalogs.field_use {
            for (position, item) in file.value.entries().iter().enumerate() {
                add_catalog_id(
                    &mut index.catalog_ids,
                    CatalogNamespace::FieldUseItems,
                    "",
                    item.id(),
                    &file.path,
                    format!("[{position}].id"),
                );
            }
        }
        for file in &catalogs.maps {
            let id = file.value.effective_id(&file.stem);
            add_catalog_id(
                &mut index.catalog_ids,
                CatalogNamespace::Maps,
                "",
                id,
                &file.path,
                "id",
            );
            // Runtime map identity comes from the TMX stem. Keep the authored metadata id in
            // duplicate diagnostics above, but scope children to the map they can inhabit.
            let scope = file.stem.clone();
            for (position, npc) in file.value.npcs.iter().enumerate() {
                index
                    .npcs
                    .entry(scope.clone())
                    .or_default()
                    .insert(npc.id.clone());
                add_catalog_id(
                    &mut index.catalog_ids,
                    CatalogNamespace::Npcs,
                    &scope,
                    &npc.id,
                    &file.path,
                    format!("npcs[{position}].id"),
                );
            }
            for (position, item_box) in file.value.item_boxes.iter().enumerate() {
                add_catalog_id(
                    &mut index.catalog_ids,
                    CatalogNamespace::ItemBoxes,
                    &scope,
                    &item_box.id,
                    &file.path,
                    format!("item_boxes[{position}].id"),
                );
            }
        }
        for file in &catalogs.dialogue {
            let id = file.value.effective_id(&file.stem);
            add_id(
                &mut index.dialogue,
                &mut index.catalog_ids,
                CatalogNamespace::Dialogue,
                "",
                id,
                &file.path,
                "id",
            );
        }
        for file in &catalogs.enemies {
            for (position, enemy) in file.value.entries().iter().enumerate() {
                add_id(
                    &mut index.enemies,
                    &mut index.catalog_ids,
                    CatalogNamespace::Enemies,
                    "",
                    &enemy.id,
                    &file.path,
                    format!("documents[{position}].id"),
                );
                if enemy.boss {
                    index.boss_enemies.insert(enemy.id.clone());
                }
                if let EnemyBehavior::Inline { ai, .. } = &enemy.behavior {
                    collect_enemy_abilities(ai, &mut index.enemy_abilities);
                }
            }
        }
        for file in &catalogs.boss_move_sets {
            add_id(
                &mut index.boss_move_sets,
                &mut index.catalog_ids,
                CatalogNamespace::BossMoveSets,
                "",
                &file.stem,
                &file.path,
                "$filename_stem",
            );
            collect_enemy_abilities(&file.value.ai, &mut index.enemy_abilities);
        }
        for file in &catalogs.encounters {
            add_catalog_id(
                &mut index.catalog_ids,
                CatalogNamespace::Encounters,
                "",
                file.value.effective_id(&file.stem),
                &file.path,
                "id",
            );
        }
        if let Some(file) = &catalogs.backgrounds {
            for (position, background) in file.value.0.iter().enumerate() {
                add_id(
                    &mut index.backgrounds,
                    &mut index.catalog_ids,
                    CatalogNamespace::BattleBackgrounds,
                    "",
                    &background.id,
                    &file.path,
                    format!("[{position}].id"),
                );
            }
        }
        for file in &catalogs.recipes {
            for (position, recipe) in file.value.entries().iter().enumerate() {
                add_catalog_id(
                    &mut index.catalog_ids,
                    CatalogNamespace::Recipes,
                    "",
                    &recipe.id,
                    &file.path,
                    format!("[{position}].id"),
                );
            }
        }
        if let Some(file) = &catalogs.quests {
            for (position, quest) in file.value.entries().iter().enumerate() {
                add_catalog_id(
                    &mut index.catalog_ids,
                    CatalogNamespace::Quests,
                    "",
                    &quest.id,
                    &file.path,
                    format!("[{position}].id"),
                );
            }
        }
        if let Some(file) = &catalogs.bgm {
            for (category_position, category) in file.value.categories.iter().enumerate() {
                for (entry_position, entry) in category.entries.iter().enumerate() {
                    let id = format!("{}.{}", category.id, entry.id);
                    add_id(
                        &mut index.bgm,
                        &mut index.catalog_ids,
                        CatalogNamespace::Bgm,
                        "",
                        &id,
                        &file.path,
                        format!("categories[{category_position}].entries[{entry_position}]"),
                    );
                }
            }
        }
        if let Some(file) = &catalogs.sfx {
            for (category_position, category) in file.value.categories.iter().enumerate() {
                for (entry_position, entry) in category.entries.iter().enumerate() {
                    index.sfx.insert(entry.id.clone());
                    add_catalog_id(
                        &mut index.catalog_ids,
                        CatalogNamespace::Sfx,
                        "",
                        &entry.id,
                        &file.path,
                        format!("categories[{category_position}].entries[{entry_position}]"),
                    );
                }
            }
        }
        index
    }
}

fn collect_enemy_abilities(ai: &EnemyAi, target: &mut BTreeSet<String>) {
    for move_ in &ai.moves {
        if let EnemyMove::Ability { id, .. } = move_ {
            target.insert(id.clone());
        }
    }
}

pub(super) fn ability_element_id(element: AbilityElement) -> &'static str {
    match element {
        AbilityElement::Fire => "fire",
        AbilityElement::Water => "water",
        AbilityElement::Wind => "wind",
        AbilityElement::Earth => "earth",
        AbilityElement::Holy => "holy",
    }
}

fn add_id(
    set: &mut BTreeSet<String>,
    ids: &mut Vec<CatalogId>,
    namespace: CatalogNamespace,
    scope: &str,
    id: &str,
    path: &str,
    field: impl Into<String>,
) {
    set.insert(id.to_owned());
    add_catalog_id(ids, namespace, scope, id, path, field);
}

fn add_catalog_id(
    ids: &mut Vec<CatalogId>,
    namespace: CatalogNamespace,
    scope: &str,
    id: &str,
    path: &str,
    field: impl Into<String>,
) {
    let location = CatalogIdLocation::new(path, field.into())
        .expect("loaded paths and fields are valid diagnostic locations");
    ids.push(if scope.is_empty() {
        CatalogId::scenario_wide(namespace, id, location)
    } else {
        CatalogId::scoped(namespace, scope, id, location)
    });
}

impl<'a> Validator<'a> {
    pub(super) fn add_tmx_map_ids(&mut self, index: &mut ReferenceIndex) {
        let directory = self.physical_root.join(&index.tmx_root);
        let Some(canonical_root) = self.canonical_root.as_deref() else {
            return;
        };
        let Ok(canonical_directory) = directory.canonicalize() else {
            return;
        };
        if !canonical_directory.starts_with(canonical_root) || !canonical_directory.is_dir() {
            return;
        }
        let Ok(entries) = fs::read_dir(directory) else {
            return;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let contained_file = path
                .strip_prefix(self.physical_root)
                .ok()
                .and_then(|relative| relative.to_str())
                .and_then(|relative| ScenarioRelativePath::try_from(relative).ok())
                .is_some_and(|relative| {
                    probe_path(
                        self.physical_root,
                        self.canonical_root.as_deref(),
                        &relative,
                    ) == ScenarioPathProbeResult::File
                });
            if path.extension().is_some_and(|extension| extension == "tmx")
                && contained_file
                && let Some(stem) = path.file_stem().and_then(|stem| stem.to_str())
            {
                index.maps.insert(stem.to_owned());
            }
        }
    }
}
