//! Typed Bevy loaders for encounter, enemy-stream, and battle-background YAML.

use std::{error::Error, fmt};

use bevy::{
    asset::{AssetApp, AssetLoader, LoadContext, io::Reader},
    prelude::{App, Plugin},
    reflect::TypePath,
};

use crate::{
    scenario_battle_background::BattleBackgroundCatalog,
    scenario_encounter::EncounterZone,
    scenario_enemy::EnemyCatalogFile,
    scenario_yaml::{self, ScenarioYamlError},
};

pub(crate) struct EncounterAssetPlugin;

impl Plugin for EncounterAssetPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<EncounterZone>()
            .init_asset::<EnemyCatalogFile>()
            .init_asset::<BattleBackgroundCatalog>()
            .init_asset_loader::<EncounterZoneLoader>()
            .init_asset_loader::<EnemyCatalogFileLoader>()
            .init_asset_loader::<BattleBackgroundCatalogLoader>();
    }
}

#[derive(Default, TypePath)]
struct EncounterZoneLoader;

#[derive(Default, TypePath)]
struct EnemyCatalogFileLoader;

#[derive(Default, TypePath)]
struct BattleBackgroundCatalogLoader;

async fn read_document(reader: &mut dyn Reader) -> Result<String, EncounterAssetLoaderError> {
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .await
        .map_err(EncounterAssetLoaderError::Io)?;
    String::from_utf8(bytes).map_err(EncounterAssetLoaderError::Utf8)
}

impl AssetLoader for EncounterZoneLoader {
    type Asset = EncounterZone;
    type Settings = ();
    type Error = EncounterAssetLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _: &(),
        _: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        scenario_yaml::from_str(&read_document(reader).await?)
            .map_err(EncounterAssetLoaderError::Yaml)
    }

    fn extensions(&self) -> &[&str] {
        &["yaml", "yml"]
    }
}

impl AssetLoader for EnemyCatalogFileLoader {
    type Asset = EnemyCatalogFile;
    type Settings = ();
    type Error = EncounterAssetLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _: &(),
        _: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        EnemyCatalogFile::from_yaml_stream(&read_document(reader).await?)
            .map_err(EncounterAssetLoaderError::Yaml)
    }

    fn extensions(&self) -> &[&str] {
        &["yaml", "yml"]
    }
}

impl AssetLoader for BattleBackgroundCatalogLoader {
    type Asset = BattleBackgroundCatalog;
    type Settings = ();
    type Error = EncounterAssetLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _: &(),
        _: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        scenario_yaml::from_str(&read_document(reader).await?)
            .map_err(EncounterAssetLoaderError::Yaml)
    }

    fn extensions(&self) -> &[&str] {
        &["yaml", "yml"]
    }
}

#[derive(Debug)]
enum EncounterAssetLoaderError {
    Io(std::io::Error),
    Utf8(std::string::FromUtf8Error),
    Yaml(ScenarioYamlError),
}

impl fmt::Display for EncounterAssetLoaderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "encounter asset read failed: {error}"),
            Self::Utf8(error) => write!(formatter, "encounter asset is not UTF-8: {error}"),
            Self::Yaml(error) => write!(formatter, "encounter asset YAML is invalid: {error}"),
        }
    }
}

impl Error for EncounterAssetLoaderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Utf8(error) => Some(error),
            Self::Yaml(error) => Some(error),
        }
    }
}
