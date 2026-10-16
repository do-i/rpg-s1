//! Typed Bevy loaders for encounter, enemy-stream, and battle-background YAML.

use bevy::{
    asset::AssetApp,
    prelude::{App, Plugin},
};

use crate::{
    scenario_battle_background::BattleBackgroundCatalog,
    scenario_encounter::EncounterZone,
    scenario_enemy::{BossMoveSet, EnemyCatalogFile},
    scenario_yaml_asset::ScenarioYamlLoader,
};

pub(crate) struct EncounterAssetPlugin;

impl Plugin for EncounterAssetPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<EncounterZone>()
            .init_asset::<EnemyCatalogFile>()
            .init_asset::<BossMoveSet>()
            .init_asset::<BattleBackgroundCatalog>()
            .register_asset_loader(ScenarioYamlLoader::<EncounterZone>::new("encounter zone"))
            .register_asset_loader(ScenarioYamlLoader::with_parser(
                "enemy catalog",
                EnemyCatalogFile::from_yaml_stream,
            ))
            .register_asset_loader(ScenarioYamlLoader::<BossMoveSet>::new("boss move set"))
            .register_asset_loader(ScenarioYamlLoader::<BattleBackgroundCatalog>::new(
                "battle background catalog",
            ));
    }
}
