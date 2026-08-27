//! Construction and asset-loading sweep for every authored encounter formation and boss.

use std::{
    collections::BTreeSet,
    fs,
    path::Path,
    thread,
    time::{Duration, Instant},
};

use bevy::{
    asset::{AssetApp, AssetMetaCheck, AssetPlugin, AssetServer, LoadState},
    image::{CompressedImageFormats, ImageLoader, ImagePlugin},
    prelude::{App, Image, MinimalPlugins},
};

use crate::{
    encounter::{
        BattleSide, EncounterCatalog, EnemyCatalog, PreBattleReturnContext, build_battle_entry,
    },
    field_menu_domain::FieldMenuCatalog,
    new_game::{NewGameScenario, build_new_game_state},
    scenario_balance::BalanceData,
    scenario_battle_background::BattleBackgroundCatalog,
    scenario_class::ClassDefinition,
    scenario_encounter::EncounterZone,
    scenario_enemy::{BossMoveSet, EnemyCatalogFile},
    scenario_inventory::ScenarioInventory,
    scenario_item::ItemCatalogFile,
    scenario_manifest::Manifest,
    scenario_party::PartyCatalog,
    scenario_path::ScenarioRelativePath,
    scenario_root::ScenarioRoot,
    scenario_spatial::{CardinalDirection, Position},
    scenario_yaml,
    tsx_atlas_asset::{TsxAtlasAsset, TsxAtlasAssetPlugin},
};

const ASSET_LOAD_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct EncounterSweepReport {
    pub(crate) scenario_id: Option<String>,
    pub(crate) scenario_name: Option<String>,
    pub(crate) zones: Vec<EncounterSweepZone>,
    pub(crate) assets_checked: usize,
    pub(crate) asset_failures: Vec<String>,
    pub(crate) load_error: Option<String>,
}

impl EncounterSweepReport {
    pub(crate) fn with_load_error(message: impl Into<String>) -> Self {
        Self {
            load_error: Some(message.into()),
            ..Self::default()
        }
    }

    pub(crate) fn constructions(&self) -> usize {
        self.zones.iter().map(|zone| zone.constructions.len()).sum()
    }

    pub(crate) fn construction_failures(&self) -> usize {
        self.zones
            .iter()
            .flat_map(|zone| &zone.constructions)
            .filter(|entry| entry.failure.is_some())
            .count()
    }

    pub(crate) fn is_valid(&self) -> bool {
        self.load_error.is_none()
            && self.construction_failures() == 0
            && self.asset_failures.is_empty()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct EncounterSweepZone {
    pub(crate) id: String,
    pub(crate) formations: usize,
    pub(crate) bosses: usize,
    pub(crate) constructions: Vec<EncounterSweepConstruction>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EncounterSweepConstruction {
    pub(crate) label: String,
    pub(crate) enemies: usize,
    pub(crate) failure: Option<String>,
}

pub(crate) fn build_encounter_sweep(
    asset_base: &Path,
    root: &ScenarioRoot,
    physical_root: &Path,
) -> EncounterSweepReport {
    match try_build_encounter_sweep(asset_base, root, physical_root) {
        Ok(report) => report,
        Err(error) => EncounterSweepReport::with_load_error(error),
    }
}

fn try_build_encounter_sweep(
    asset_base: &Path,
    root: &ScenarioRoot,
    physical_root: &Path,
) -> Result<EncounterSweepReport, String> {
    let manifest: Manifest = read_yaml(&physical_root.join("manifest.yaml"), "manifest")?;
    let inventory = ScenarioInventory::discover(asset_base, root);
    if let Some(error) = inventory.failure.as_ref() {
        return Err(format!("scenario inventory failed: {error}"));
    }
    let party_path = inventory
        .party
        .as_ref()
        .ok_or_else(|| "scenario inventory has no party catalog".to_owned())?;
    let balance_path = inventory
        .balance
        .as_ref()
        .ok_or_else(|| "scenario inventory has no balance catalog".to_owned())?;
    let party: PartyCatalog = read_yaml(
        &physical_root.join(party_path.as_str()),
        party_path.as_str(),
    )?;
    let balance: BalanceData = read_yaml(
        &physical_root.join(balance_path.as_str()),
        balance_path.as_str(),
    )?;
    let mut game = build_new_game_state(
        NewGameScenario {
            manifest: &manifest,
            party: &party,
            balance: &balance,
        },
        std::time::Duration::ZERO,
    )
    .map_err(|error| format!("normal new-game construction failed: {error}"))?;

    let item_files = inventory
        .item_catalogs
        .iter()
        .map(|path| read_yaml(&physical_root.join(path.as_str()), path.as_str()))
        .collect::<Result<Vec<ItemCatalogFile>, _>>()?;
    let class_files = inventory
        .classes
        .iter()
        .map(|path| read_yaml(&physical_root.join(path.as_str()), path.as_str()))
        .collect::<Result<Vec<ClassDefinition>, _>>()?;
    let field_catalog = FieldMenuCatalog::for_encounter_sweep(item_files, class_files);

    let enemy_files = inventory
        .enemy_catalogs
        .iter()
        .map(|path| {
            let text = read_text(&physical_root.join(path.as_str()), path.as_str())?;
            EnemyCatalogFile::from_yaml_stream(&text)
                .map_err(|error| format!("{} is invalid: {error}", path.as_str()))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let move_sets = inventory
        .boss_move_sets
        .iter()
        .map(|(logical, path)| {
            read_yaml::<BossMoveSet>(&physical_root.join(path.as_str()), path.as_str())
                .map(|move_set| (logical.clone(), move_set))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut enemy_catalog = EnemyCatalog::try_from_definitions(
        enemy_files
            .iter()
            .flat_map(|file| file.entries().iter().cloned()),
    )
    .map_err(|error| error.to_string())?;
    enemy_catalog
        .resolve_boss_move_sets(
            move_sets
                .iter()
                .map(|(logical, move_set)| (logical.as_str(), move_set)),
        )
        .map_err(|error| error.to_string())?;

    let backgrounds_path = inventory
        .battle_backgrounds
        .as_ref()
        .ok_or_else(|| "scenario inventory has no battle-background catalog".to_owned())?;
    let backgrounds: BattleBackgroundCatalog = read_yaml(
        &physical_root.join(backgrounds_path.as_str()),
        backgrounds_path.as_str(),
    )?;
    let background_ids = backgrounds
        .0
        .iter()
        .map(|background| background.id.as_str())
        .collect::<BTreeSet<_>>();

    let encounter_directory = inventory
        .encounter_directory
        .as_ref()
        .ok_or_else(|| "scenario inventory has no encounter directory".to_owned())?;
    let zone_paths = yaml_paths(&physical_root.join(encounter_directory.as_str()))?;
    let mut zone_documents = Vec::new();
    for path in zone_paths {
        let stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| "encounter filename is not portable UTF-8".to_owned())?
            .to_owned();
        let zone: EncounterZone = read_yaml(&path, &format!("encounter `{stem}`"))?;
        zone_documents.push((stem, zone));
    }
    EncounterCatalog::try_from_zones(zone_documents.iter().cloned())
        .map_err(|error| error.to_string())?;

    for (_, zone) in &zone_documents {
        for barrier in &zone.barrier_enemies {
            let _ = game
                .repository_mut()
                .add_item(barrier.requires_item.clone(), 1)
                .map_err(|error| {
                    format!(
                        "barrier item `{}` could not be installed: {error}",
                        barrier.requires_item
                    )
                })?;
        }
    }

    let mut zones = Vec::new();
    let mut atlas_paths = BTreeSet::new();
    let mut background_paths = BTreeSet::new();
    for (stem, zone) in &zone_documents {
        let zone_id = zone.effective_id(stem).to_owned();
        let mut constructions = Vec::new();
        if !background_ids.contains(zone.background.as_str()) {
            constructions.push(EncounterSweepConstruction {
                label: "background".to_owned(),
                enemies: 0,
                failure: Some(format!(
                    "encounter references unknown background `{}`",
                    zone.background
                )),
            });
        }
        for (index, formation) in zone.entries.iter().enumerate() {
            let label = format!("formation[{index}]");
            let result = construct(
                &format!("{zone_id}:{label}"),
                &zone_id,
                &formation.enemy_ids,
                zone,
                &enemy_catalog,
                &field_catalog,
                &game,
                false,
            );
            collect_assets(&result, &mut atlas_paths, &mut background_paths);
            constructions.push(EncounterSweepConstruction {
                label,
                enemies: result.as_ref().map_or(0, |entry| {
                    entry
                        .participants
                        .iter()
                        .filter(|participant| participant.side == BattleSide::Enemy)
                        .count()
                }),
                failure: result.err(),
            });
        }
        let bosses = usize::from(zone.boss.is_some());
        if let Some(boss) = &zone.boss {
            let result = construct(
                &format!("{zone_id}:boss"),
                &zone_id,
                std::slice::from_ref(&boss.enemy_id),
                zone,
                &enemy_catalog,
                &field_catalog,
                &game,
                true,
            );
            collect_assets(&result, &mut atlas_paths, &mut background_paths);
            constructions.push(EncounterSweepConstruction {
                label: "boss".to_owned(),
                enemies: result.as_ref().map_or(0, |entry| {
                    entry
                        .participants
                        .iter()
                        .filter(|participant| participant.side == BattleSide::Enemy)
                        .count()
                }),
                failure: result.err(),
            });
        }
        zones.push(EncounterSweepZone {
            id: zone_id,
            formations: zone.entries.len(),
            bosses,
            constructions,
        });
    }

    let asset_failures = load_battle_assets(asset_base, root, &atlas_paths, &background_paths);
    Ok(EncounterSweepReport {
        scenario_id: Some(manifest.id),
        scenario_name: Some(manifest.name),
        zones,
        assets_checked: atlas_paths.len() + background_paths.len(),
        asset_failures,
        load_error: None,
    })
}

fn construct(
    encounter_id: &str,
    map_id: &str,
    formation: &[String],
    zone: &EncounterZone,
    enemies: &EnemyCatalog,
    items: &FieldMenuCatalog,
    game: &crate::game_state::GameState,
    boss: bool,
) -> Result<crate::encounter::BattleEntry, String> {
    let entry = build_battle_entry(
        encounter_id,
        formation,
        zone,
        enemies,
        items,
        game.party(),
        game.repository(),
        game.flags(),
        boss,
        PreBattleReturnContext {
            map_id: map_id.to_owned(),
            position: Position::new(0, 0),
            facing: CardinalDirection::Down,
            world_bgm_key: None,
            world_enemies: Vec::new(),
        },
    )
    .map_err(|error| error.to_string())?;
    let actual = entry
        .participants
        .iter()
        .filter(|participant| participant.side == BattleSide::Enemy)
        .map(|participant| participant.id.as_str())
        .collect::<Vec<_>>();
    let expected = formation.iter().map(String::as_str).collect::<Vec<_>>();
    if actual != expected {
        return Err(format!(
            "constructed enemies {actual:?} differ from formation {expected:?}"
        ));
    }
    if entry
        .participants
        .iter()
        .filter(|participant| participant.side == BattleSide::Enemy)
        .any(|participant| {
            participant.name.is_empty()
                || participant.health == 0
                || participant.sprite_id.is_empty()
                || participant.behavior.is_none()
        })
    {
        return Err("constructed enemy has incomplete combatant state".to_owned());
    }
    Ok(entry)
}

fn collect_assets(
    result: &Result<crate::encounter::BattleEntry, String>,
    atlases: &mut BTreeSet<ScenarioRelativePath>,
    backgrounds: &mut BTreeSet<ScenarioRelativePath>,
) {
    let Ok(entry) = result else {
        return;
    };
    if let Ok(path) = ScenarioRelativePath::try_from(entry.background_asset.as_str()) {
        backgrounds.insert(path);
    }
    for participant in entry
        .participants
        .iter()
        .filter(|participant| participant.side == BattleSide::Enemy)
    {
        if let Ok(path) = ScenarioRelativePath::try_from(
            format!(
                "assets/sprites/enemies/{}_battle.tsx",
                participant.sprite_id
            )
            .as_str(),
        ) {
            atlases.insert(path);
        }
    }
}

fn load_battle_assets(
    asset_base: &Path,
    root: &ScenarioRoot,
    atlas_paths: &BTreeSet<ScenarioRelativePath>,
    background_paths: &BTreeSet<ScenarioRelativePath>,
) -> Vec<String> {
    if atlas_paths.is_empty() && background_paths.is_empty() {
        return Vec::new();
    }
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin {
            file_path: asset_base.to_string_lossy().into_owned(),
            meta_check: AssetMetaCheck::Never,
            ..Default::default()
        })
        .add_plugins(ImagePlugin::default_nearest())
        .register_asset_loader(ImageLoader::new(CompressedImageFormats::empty()))
        .insert_resource(root.clone())
        .add_plugins(TsxAtlasAssetPlugin);
    let atlas_handles = {
        let server = app.world().resource::<AssetServer>();
        atlas_paths
            .iter()
            .map(|path| (path, server.load::<TsxAtlasAsset>(root.resolve(path))))
            .collect::<Vec<_>>()
    };
    let background_handles = {
        let server = app.world().resource::<AssetServer>();
        background_paths
            .iter()
            .map(|path| (path, server.load::<Image>(root.resolve(path))))
            .collect::<Vec<_>>()
    };
    let deadline = Instant::now() + ASSET_LOAD_TIMEOUT;
    while Instant::now() < deadline {
        app.update();
        let server = app.world().resource::<AssetServer>();
        let settled = atlas_handles
            .iter()
            .all(|(_, handle)| asset_settled(server, handle.id()))
            && background_handles
                .iter()
                .all(|(_, handle)| asset_settled(server, handle.id()));
        if settled {
            break;
        }
        thread::yield_now();
    }
    let server = app.world().resource::<AssetServer>();
    atlas_handles
        .iter()
        .map(|(path, handle)| (path, handle.id().untyped()))
        .chain(
            background_handles
                .iter()
                .map(|(path, handle)| (path, handle.id().untyped())),
        )
        .filter_map(|(path, id)| {
            (!server.is_loaded_with_dependencies(id)).then(|| {
                format!(
                    "{}: root={:?}, dependencies={:?}",
                    path.as_str(),
                    server.load_state(id),
                    server.recursive_dependency_load_state(id)
                )
            })
        })
        .collect()
}

fn asset_settled<A: bevy::asset::Asset>(server: &AssetServer, id: bevy::asset::AssetId<A>) -> bool {
    server.is_loaded_with_dependencies(id)
        || matches!(server.load_state(id), LoadState::Failed(_))
        || server.recursive_dependency_load_state(id).is_failed()
}

fn read_text(path: &Path, label: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("{label} could not be read: {error}"))
}

fn read_yaml<T: serde::de::DeserializeOwned>(path: &Path, label: &str) -> Result<T, String> {
    scenario_yaml::from_str(&read_text(path, label)?)
        .map_err(|error| format!("{label} is invalid: {error}"))
}

fn yaml_paths(directory: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("encounter directory could not be read: {error}"))?
    {
        let path = entry
            .map_err(|error| format!("encounter directory entry could not be read: {error}"))?
            .path();
        if path.extension().and_then(|extension| extension.to_str()) == Some("yaml") {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_package_returns_a_bounded_load_error() {
        let report = build_encounter_sweep(
            Path::new("/nonexistent/assets"),
            &ScenarioRoot::default(),
            Path::new("/nonexistent/assets/scenarios/rusted_kingdoms"),
        );

        assert!(!report.is_valid());
        assert!(report.zones.is_empty());
        assert!(report.load_error.unwrap().contains("manifest"));
    }
}
