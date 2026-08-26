//! Source-authored apothecary recipe schemas.
//!
//! The pinned `0897035` Rusted Kingdoms scenario has one list-root recipe catalog:
//! `data/recipe/all_recipe.yaml`. It currently contains eleven recipes: ten ordinary outputs and
//! one `unique_output` recipe. A recipe is an ordered output, optional ordered item and magic-core
//! ingredient lists, a GP cost, and an optional unlock-flag id. The catalog filename is its
//! location only; recipe identity always comes from the required `id` field.
//!
//! The Python apothecary treats absent `unique_output` as false, absent `inputs`/ingredient lists
//! as empty, and an absent output quantity as one. It treats an absent or null `unlock_flag` as
//! unlocked. This module records those data-loading defaults only; availability, ownership,
//! crafting, catalog uniqueness, and cross-reference validation belong to later milestones.

use bevy::{asset::Asset, reflect::TypePath};

use crate::scenario_yaml::deserialize_string;
use serde::{Deserialize, Deserializer};
use std::num::NonZeroU32;

/// The list-root `data/recipe/all_recipe.yaml` document.
#[derive(Asset, Clone, Debug, Deserialize, PartialEq, TypePath)]
#[serde(transparent)]
pub struct RecipeCatalogFile(pub Vec<RecipeDefinition>);

impl RecipeCatalogFile {
    pub fn entries(&self) -> &[RecipeDefinition] {
        &self.0
    }
}

/// One source-authored recipe. Item ids and unlock flags remain logical identifiers until the
/// dedicated catalog cross-reference validation task.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RecipeDefinition {
    #[serde(deserialize_with = "deserialize_string")]
    pub id: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub scroll_name: String,
    pub output: RecipeOutput,
    /// `false` is the Python default and denotes a repeatable ordinary recipe.
    #[serde(default)]
    pub unique_output: bool,
    /// Missing inputs are an empty ingredient set in the Python apothecary.
    #[serde(default)]
    pub inputs: RecipeInputs,
    /// GP is non-negative; zero is a meaningful free-recipe cost.
    pub gp_cost: u32,
    /// Missing or null means no unlock gate (the recipe is visible and unlocked).
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub unlock_flag: Option<String>,
}

/// The produced item and quantity. The apothecary defaults an omitted output quantity to one.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RecipeOutput {
    #[serde(deserialize_with = "deserialize_string")]
    pub item: String,
    #[serde(default = "one")]
    pub qty: NonZeroU32,
}

/// Ordered ingredient groups. Both groups may be empty and duplicate requirements are preserved.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RecipeInputs {
    #[serde(default)]
    pub items: Vec<ItemIngredient>,
    #[serde(default)]
    pub mc: Vec<MagicCoreIngredient>,
}

/// One ordinary-item ingredient.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ItemIngredient {
    #[serde(deserialize_with = "deserialize_string")]
    pub id: String,
    pub qty: NonZeroU32,
}

/// One magic-core ingredient, selected by the five source item catalog sizes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MagicCoreIngredient {
    pub size: MagicCoreSize,
    pub qty: NonZeroU32,
}

/// The complete fixed source magic-core size vocabulary.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum MagicCoreSize {
    XS,
    S,
    M,
    L,
    XL,
}

fn deserialize_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<StrictString>::deserialize(deserializer).map(|value| value.map(|value| value.0))
}

struct StrictString(String);

impl<'de> Deserialize<'de> for StrictString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_string(deserializer).map(Self)
    }
}

fn one() -> NonZeroU32 {
    NonZeroU32::new(1).expect("one is nonzero")
}

#[cfg(test)]
mod tests {
    use super::{MagicCoreSize, RecipeCatalogFile};
    use crate::scenario_yaml;

    #[test]
    fn loads_regular_and_unique_output_recipe_shapes() {
        let catalog: RecipeCatalogFile = scenario_yaml::from_str(include_str!(
            "../../../tests/fixtures/recipe-catalog-shapes.yaml"
        ))
        .expect("source-shaped recipe fixture should deserialize");

        assert_eq!(catalog.entries().len(), 2);
        let ordinary = &catalog.entries()[0];
        assert_eq!(ordinary.id, "recipe_invented_remedy");
        assert!(!ordinary.unique_output);
        assert_eq!(ordinary.output.item, "invented_remedy");
        assert_eq!(ordinary.output.qty.get(), 2);
        assert_eq!(ordinary.inputs.items[0].id, "invented_herb");
        assert_eq!(ordinary.inputs.items[0].qty.get(), 3);
        assert_eq!(ordinary.inputs.mc[0].size, MagicCoreSize::S);
        assert_eq!(ordinary.gp_cost, 80);
        assert_eq!(
            ordinary.unlock_flag.as_deref(),
            Some("story_invented_started")
        );

        let unique = &catalog.entries()[1];
        assert!(unique.unique_output);
        assert_eq!(unique.output.qty.get(), 1);
        assert_eq!(unique.inputs.mc[0].size, MagicCoreSize::XL);
    }

    #[test]
    fn applies_only_the_documented_apothecary_defaults() {
        let catalog: RecipeCatalogFile = scenario_yaml::from_str(
            "- id: recipe_free\n  scroll_name: Free Draft\n  output: { item: invented_draft }\n  gp_cost: 0\n- id: recipe_null_gate\n  scroll_name: Null Gate\n  output: { item: invented_key, qty: 1 }\n  inputs: {}\n  gp_cost: 1\n  unlock_flag: null\n",
        )
        .expect("source loader defaults should deserialize");

        assert!(!catalog.entries()[0].unique_output);
        assert_eq!(catalog.entries()[0].output.qty.get(), 1);
        assert!(catalog.entries()[0].inputs.items.is_empty());
        assert!(catalog.entries()[0].inputs.mc.is_empty());
        assert_eq!(catalog.entries()[0].unlock_flag, None);
        assert_eq!(catalog.entries()[1].unlock_flag, None);
    }

    #[test]
    fn preserves_ingredient_order_and_duplicates_without_crafting_them() {
        let catalog: RecipeCatalogFile = scenario_yaml::from_str(
            "- id: recipe_ordered\n  scroll_name: Ordered\n  output: { item: invented_output, qty: 1 }\n  inputs:\n    items: [{ id: second, qty: 1 }, { id: first, qty: 2 }, { id: second, qty: 1 }]\n    mc: [{ size: M, qty: 2 }, { size: S, qty: 1 }]\n  gp_cost: 0\n",
        )
        .expect("ordered ingredients should deserialize");
        let inputs = &catalog.entries()[0].inputs;
        assert_eq!(
            inputs
                .items
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            ["second", "first", "second"]
        );
        assert_eq!(
            inputs.mc.iter().map(|core| core.size).collect::<Vec<_>>(),
            [MagicCoreSize::M, MagicCoreSize::S]
        );
    }

    #[test]
    fn rejects_unknown_coerced_null_and_invalid_numeric_shapes() {
        let valid = include_str!("../../../tests/fixtures/recipe-catalog-shapes.yaml");
        for document in [
            valid.replacen("id: recipe_invented_remedy", "id: 42", 1),
            valid.replacen("scroll_name: Invented Remedy", "scroll_name: true", 1),
            valid.replacen("item: invented_remedy", "item: null", 1),
            valid.replacen("qty: 2", "qty: 0", 1),
            valid.replacen("qty: 2", "qty: -1", 1),
            valid.replacen("qty: 2", "qty: 1.5", 1),
            valid.replacen("size: S", "size: XXL", 1),
            valid.replacen("gp_cost: 80", "gp_cost: -1", 1),
            valid.replacen("unlock_flag: story_invented_started", "unlock_flag: 10", 1),
            valid.replacen("unique_output: true", "unique_output: null", 1),
            valid.replacen("gp_cost: 80", "gp_cost: 80\n  mystery: nope", 1),
            valid.replacen("  gp_cost: 80", "  gp_cost: 80\n    stray: nope", 1),
        ] {
            assert!(
                scenario_yaml::from_str::<RecipeCatalogFile>(&document).is_err(),
                "accepted:\n{document}"
            );
        }

        for document in [
            valid.replacen("  output: { item: invented_remedy, qty: 2 }\n", "", 1),
            valid.replacen("  gp_cost: 80\n", "", 1),
            valid.replacen("inputs:\n", "inputs: null\n", 1),
            valid.replacen("items:\n", "items: null\n", 1),
        ] {
            assert!(
                scenario_yaml::from_str::<RecipeCatalogFile>(&document).is_err(),
                "accepted:\n{document}"
            );
        }
    }

    #[test]
    #[ignore = "requires the separately licensed pinned Python source checkout"]
    fn audits_the_complete_pinned_recipe_catalog_when_requested() {
        let root = std::env::var_os("RPG_S1_PINNED_RECIPES_DIR")
            .map(std::path::PathBuf::from)
            .expect("RPG_S1_PINNED_RECIPES_DIR must name the pinned data/recipe directory");
        let mut files = std::fs::read_dir(&root)
            .expect("pinned recipe directory should be readable")
            .map(|entry| {
                entry
                    .expect("recipe directory entry should be readable")
                    .path()
            })
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "yaml")
            })
            .collect::<Vec<_>>();
        files.sort();

        let mut recipe_count = 0;
        let mut unique_count = 0;
        let mut item_ingredients = 0;
        let mut magic_core_ingredients = 0;
        for path in &files {
            let document = std::fs::read_to_string(path)
                .unwrap_or_else(|error| panic!("{} should be readable: {error}", path.display()));
            let catalog: RecipeCatalogFile = scenario_yaml::from_str(&document)
                .unwrap_or_else(|error| panic!("{} should load: {error}", path.display()));
            recipe_count += catalog.0.len();
            unique_count += catalog
                .0
                .iter()
                .filter(|recipe| recipe.unique_output)
                .count();
            item_ingredients += catalog
                .0
                .iter()
                .map(|recipe| recipe.inputs.items.len())
                .sum::<usize>();
            magic_core_ingredients += catalog
                .0
                .iter()
                .map(|recipe| recipe.inputs.mc.len())
                .sum::<usize>();
            assert!(catalog.0.iter().all(|recipe| !recipe.id.is_empty()
                && !recipe.scroll_name.is_empty()
                && !recipe.output.item.is_empty()));
        }

        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].file_name().and_then(|name| name.to_str()),
            Some("all_recipe.yaml")
        );
        assert_eq!(recipe_count, 11);
        assert_eq!(unique_count, 1);
        assert_eq!(item_ingredients, 20);
        assert_eq!(magic_core_ingredients, 8);
    }
}
