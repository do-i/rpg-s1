//! Filesystem inventory for one selected scenario package.

use std::{
    fmt, fs,
    path::{Path, PathBuf},
};

use bevy::prelude::Resource;

use crate::{
    scenario_enemy::{BossMoveSet, EnemyCatalogFile},
    scenario_item::{FieldUseCatalogFile, ItemCatalogFile},
    scenario_manifest::Manifest,
    scenario_path::ScenarioRelativePath,
    scenario_root::ScenarioRoot,
    scenario_yaml,
};

#[derive(Clone, Debug, Default, Resource)]
pub(crate) struct ScenarioInventory {
    pub(crate) font: Option<ScenarioRelativePath>,
    pub(crate) menu_backdrop: Option<ScenarioRelativePath>,
    /// Keeper faces and recipe-status icons the service overlays draw.
    pub(crate) service_art: ServiceArt,
    pub(crate) map_directory: Option<ScenarioRelativePath>,
    pub(crate) encounter_directory: Option<ScenarioRelativePath>,
    pub(crate) tmx_directory: Option<ScenarioRelativePath>,
    pub(crate) battle_backgrounds: Option<ScenarioRelativePath>,
    pub(crate) party: Option<ScenarioRelativePath>,
    pub(crate) balance: Option<ScenarioRelativePath>,
    pub(crate) item_catalogs: Vec<ScenarioRelativePath>,
    pub(crate) field_use: Option<ScenarioRelativePath>,
    pub(crate) classes: Vec<ScenarioRelativePath>,
    pub(crate) maps: Vec<(String, ScenarioRelativePath, Option<ScenarioRelativePath>)>,
    pub(crate) recipes: Vec<ScenarioRelativePath>,
    pub(crate) enemy_catalogs: Vec<ScenarioRelativePath>,
    pub(crate) boss_move_sets: Vec<(String, ScenarioRelativePath)>,
    pub(crate) quests: Option<ScenarioRelativePath>,
    pub(crate) failure: Option<String>,
}

/// The manifest-declared presentation assets belonging to the field services.
///
/// The manifest has always named these; nothing consumed them until the service overlays were
/// rebuilt as drawn screens. Each is optional so a scenario package that omits a service still
/// loads — the overlay falls back to its lettered placeholder frame.
#[derive(Clone, Debug, Default)]
pub(crate) struct ServiceArt {
    pub(crate) apothecary_keeper: Option<ScenarioRelativePath>,
    pub(crate) inn_keeper: Option<ScenarioRelativePath>,
    pub(crate) item_shop_keeper: Option<ScenarioRelativePath>,
    pub(crate) weapon_shop_keeper: Option<ScenarioRelativePath>,
    pub(crate) armor_shop_keeper: Option<ScenarioRelativePath>,
    pub(crate) recipe_locked_icon: Option<ScenarioRelativePath>,
    pub(crate) recipe_ready_icon: Option<ScenarioRelativePath>,
    pub(crate) recipe_missing_icon: Option<ScenarioRelativePath>,
}

impl ScenarioInventory {
    pub(crate) fn discover(asset_base: &Path, root: &ScenarioRoot) -> Self {
        match discover(asset_base, root) {
            Ok(inventory) => inventory,
            Err(error) => Self {
                failure: Some(error.to_string()),
                ..Default::default()
            },
        }
    }

    pub(crate) fn map_metadata_path(&self, map_id: &str) -> Option<ScenarioRelativePath> {
        self.file_in(self.map_directory.as_ref()?, &format!("{map_id}.yaml"))
    }

    pub(crate) fn encounter_path(&self, map_id: &str) -> Option<ScenarioRelativePath> {
        self.file_in(
            self.encounter_directory.as_ref()?,
            &format!("{map_id}.yaml"),
        )
    }

    pub(crate) fn tmx_path(&self, map_id: &str) -> Option<ScenarioRelativePath> {
        self.file_in(self.tmx_directory.as_ref()?, &format!("{map_id}.tmx"))
    }

    fn file_in(
        &self,
        directory: &ScenarioRelativePath,
        filename: &str,
    ) -> Option<ScenarioRelativePath> {
        ScenarioRelativePath::try_from(format!("{}/{filename}", directory.as_str())).ok()
    }
}

#[derive(Debug)]
struct InventoryError(String);

impl fmt::Display for InventoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

fn discover(asset_base: &Path, root: &ScenarioRoot) -> Result<ScenarioInventory, InventoryError> {
    let package = asset_base.join("scenarios").join(root.package_key());
    let manifest_document = fs::read_to_string(package.join("manifest.yaml"))
        .map_err(|_| InventoryError("scenario manifest is unavailable for inventory".into()))?;
    let manifest: Manifest = scenario_yaml::from_str(&manifest_document).map_err(|error| {
        InventoryError(format!("scenario manifest inventory parse failed: {error}"))
    })?;

    let mut inventory = ScenarioInventory {
        font: Some(manifest.font.path.clone()),
        menu_backdrop: Some(manifest.ui.menu_backdrop.clone()),
        service_art: ServiceArt {
            apothecary_keeper: Some(manifest.apothecary.sprite.clone()),
            inn_keeper: Some(manifest.inn.sprite.clone()),
            item_shop_keeper: Some(manifest.item_shop.sprite.clone()),
            weapon_shop_keeper: Some(manifest.weapon_shop.sprite.clone()),
            armor_shop_keeper: Some(manifest.armor_shop.sprite.clone()),
            recipe_locked_icon: Some(manifest.apothecary.icons.locked.clone()),
            recipe_ready_icon: Some(manifest.apothecary.icons.ready.clone()),
            recipe_missing_icon: Some(manifest.apothecary.icons.missing.clone()),
        },
        map_directory: Some(manifest.refs.maps.as_relative_path().clone()),
        encounter_directory: Some(manifest.refs.encount.as_relative_path().clone()),
        tmx_directory: Some(manifest.refs.tmx.as_relative_path().clone()),
        battle_backgrounds: Some(manifest.refs.battle_backgrounds.clone()),
        party: Some(manifest.refs.party.clone()),
        balance: Some(manifest.refs.balance.clone()),
        ..Default::default()
    };
    for path in yaml_files(&package, manifest.refs.items.as_relative_path())? {
        let document = fs::read_to_string(package.join(path.as_str()))
            .map_err(|_| InventoryError(format!("{} is unreadable", path.as_str())))?;
        if scenario_yaml::from_str::<ItemCatalogFile>(&document).is_ok() {
            inventory.item_catalogs.push(path);
        } else if scenario_yaml::from_str::<FieldUseCatalogFile>(&document).is_ok()
            && inventory.field_use.replace(path).is_some()
        {
            return Err(InventoryError(
                "scenario has multiple field-use catalogs".into(),
            ));
        }
    }
    inventory.classes = yaml_files(&package, manifest.refs.classes.as_relative_path())?;
    inventory.recipes = yaml_files(&package, manifest.refs.recipe.as_relative_path())?;

    let metadata = yaml_files(&package, manifest.refs.maps.as_relative_path())?;
    let tmx = extension_files(&package, manifest.refs.tmx.as_relative_path(), "tmx")?;
    let mut tmx_by_stem = std::collections::BTreeMap::new();
    for path in tmx {
        if let Some(stem) = Path::new(path.as_str())
            .file_stem()
            .and_then(|stem| stem.to_str())
        {
            tmx_by_stem.insert(stem.to_owned(), path);
        }
    }
    for path in metadata {
        let stem = Path::new(path.as_str())
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| InventoryError(format!("{} has no portable stem", path.as_str())))?
            .to_owned();
        inventory
            .maps
            .push((stem.clone(), path, tmx_by_stem.remove(&stem)));
    }

    for path in yaml_files_recursive(&package, manifest.refs.enemies.as_relative_path())? {
        let document = fs::read_to_string(package.join(path.as_str()))
            .map_err(|_| InventoryError(format!("{} is unreadable", path.as_str())))?;
        if EnemyCatalogFile::from_yaml_stream(&document).is_ok() {
            inventory.enemy_catalogs.push(path);
        } else if scenario_yaml::from_str::<BossMoveSet>(&document).is_ok() {
            let prefix = format!("{}/", manifest.refs.enemies.as_str());
            let logical = path
                .as_str()
                .strip_prefix(&prefix)
                .unwrap_or(path.as_str())
                .to_owned();
            inventory.boss_move_sets.push((logical, path));
        }
    }
    inventory.quests = Some(manifest.refs.quests.clone());
    Ok(inventory)
}

fn yaml_files(
    package: &Path,
    directory: &ScenarioRelativePath,
) -> Result<Vec<ScenarioRelativePath>, InventoryError> {
    extension_files(package, directory, "yaml")
}

fn yaml_files_recursive(
    package: &Path,
    directory: &ScenarioRelativePath,
) -> Result<Vec<ScenarioRelativePath>, InventoryError> {
    let mut files = Vec::new();
    collect_recursive(
        package,
        &package.join(directory.as_str()),
        "yaml",
        &mut files,
    )?;
    files.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    Ok(files)
}

fn extension_files(
    package: &Path,
    directory: &ScenarioRelativePath,
    extension: &str,
) -> Result<Vec<ScenarioRelativePath>, InventoryError> {
    let absolute = package.join(directory.as_str());
    let entries = fs::read_dir(&absolute)
        .map_err(|_| InventoryError(format!("{} directory is unavailable", directory.as_str())))?;
    let mut files = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|_| InventoryError("scenario directory entry is unreadable".into()))?
            .path();
        if path.is_file() && path.extension().and_then(|value| value.to_str()) == Some(extension) {
            files.push(relative_path(package, &path)?);
        }
    }
    files.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    Ok(files)
}

fn collect_recursive(
    package: &Path,
    directory: &Path,
    extension: &str,
    files: &mut Vec<ScenarioRelativePath>,
) -> Result<(), InventoryError> {
    for entry in fs::read_dir(directory)
        .map_err(|_| InventoryError("scenario catalog directory is unavailable".into()))?
    {
        let path = entry
            .map_err(|_| InventoryError("scenario directory entry is unreadable".into()))?
            .path();
        if path.is_dir() {
            collect_recursive(package, &path, extension, files)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
            files.push(relative_path(package, &path)?);
        }
    }
    Ok(())
}

fn relative_path(package: &Path, path: &Path) -> Result<ScenarioRelativePath, InventoryError> {
    let relative: PathBuf = path
        .strip_prefix(package)
        .map_err(|_| InventoryError("scenario inventory path escaped its package".into()))?
        .to_owned();
    let portable = relative
        .to_str()
        .ok_or_else(|| InventoryError("scenario inventory path is not UTF-8".into()))?;
    ScenarioRelativePath::try_from(portable)
        .map_err(|error| InventoryError(format!("invalid inventory path: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_an_invented_package_without_rusted_kingdoms_catalog_names() {
        let asset_base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
        let root = ScenarioRoot::try_for_package_key("minimal_demo").unwrap();

        let inventory = ScenarioInventory::discover(&asset_base, &root);

        assert!(inventory.failure.is_none(), "{:?}", inventory.failure);
        assert!(inventory.item_catalogs.is_empty());
        assert!(inventory.classes.is_empty());
        assert!(inventory.maps.is_empty());
        assert!(inventory.enemy_catalogs.is_empty());
        assert_eq!(
            inventory.font.as_ref().map(ScenarioRelativePath::as_str),
            Some("assets/font.ttf")
        );
        assert_eq!(
            inventory
                .map_metadata_path("invented_map")
                .as_ref()
                .map(ScenarioRelativePath::as_str),
            Some("records/places/invented_map.yaml")
        );
        assert_eq!(
            inventory.quests.as_ref().map(ScenarioRelativePath::as_str),
            Some("records/quests.yaml")
        );
    }
}
