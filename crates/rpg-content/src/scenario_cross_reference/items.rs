//! Item and recipe references.

use super::{context::Validator, index::ReferenceIndex};
use crate::scenario_item::{AccessoryStats, ItemDefinition};

impl<'a> Validator<'a> {
    pub(super) fn validate_items(&mut self, index: &ReferenceIndex) {
        let items = self.catalogs.items.clone();
        for file in items {
            for (item_index, item) in file.value.entries().iter().enumerate() {
                let base = format!("[{item_index}]");
                match item {
                    ItemDefinition::Key(_) => {}
                    ItemDefinition::Accessory(item) => {
                        for (class_index, class_id) in item.equippable.iter().enumerate() {
                            if class_id == "all" {
                                continue;
                            }
                            self.checked(
                                "class",
                                &index.classes,
                                class_id,
                                &file.path,
                                format!("{base}.equippable[{class_index}]"),
                            );
                        }
                        if let AccessoryStats::AbilityBlock(stats) = &item.stats {
                            self.checked(
                                "enemy ability",
                                &index.enemy_abilities,
                                &stats.blocks_ability,
                                &file.path,
                                format!("{base}.stats.blocks_ability"),
                            );
                        }
                    }
                    _ => {}
                }
            }
        }
        if let Some(field_use) = self.catalogs.field_use.clone() {
            for (item_index, item) in field_use.value.entries().iter().enumerate() {
                self.checked(
                    "item",
                    &index.items,
                    item.id(),
                    &field_use.path,
                    format!("[{item_index}].id"),
                );
            }
        }
    }

    pub(super) fn validate_recipes(&mut self, index: &ReferenceIndex) {
        let files = self.catalogs.recipes.clone();
        for file in files {
            for (recipe_index, recipe) in file.value.entries().iter().enumerate() {
                self.checked(
                    "item",
                    &index.items,
                    &recipe.output.item,
                    &file.path,
                    format!("[{recipe_index}].output.item"),
                );
                for (item_index, item) in recipe.inputs.items.iter().enumerate() {
                    self.checked(
                        "item",
                        &index.items,
                        &item.id,
                        &file.path,
                        format!("[{recipe_index}].inputs.items[{item_index}].id"),
                    );
                }
            }
        }
    }
}
