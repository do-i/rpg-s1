//! Windowless AssetServer loading for every migrated TMX in a scenario package.

use std::{fs, path::Path, thread};

use bevy::{
    asset::{AssetApp, AssetMetaCheck, AssetPlugin, AssetServer, Assets, LoadState},
    image::{CompressedImageFormats, ImageLoader, ImagePlugin},
    prelude::{App, MinimalPlugins},
    state::app::{AppExtStates, StatesPlugin},
};

use crate::{
    app_state::AppState,
    scenario_manifest::Manifest,
    scenario_path::ScenarioRelativePath,
    scenario_root::ScenarioRoot,
    scenario_yaml,
    tmx_ground_asset::{TmxGroundAsset, TmxGroundAssetPlugin},
    tsx_atlas_asset::{TsxAtlasAsset, TsxAtlasAssetPlugin},
};

const HEADLESS_FRAMES_PER_MAP: u32 = 5;
const MAX_LOAD_UPDATES: usize = 20_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RuntimeMapSweepEntry {
    pub(crate) id: String,
    pub(crate) frames: u32,
    pub(crate) failure: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct RuntimeMapSweepReport {
    pub(crate) entries: Vec<RuntimeMapSweepEntry>,
    pub(crate) skipped_fixtures: Vec<String>,
    pub(crate) load_error: Option<String>,
}

impl RuntimeMapSweepReport {
    pub(crate) fn with_load_error(message: impl Into<String>) -> Self {
        Self {
            load_error: Some(message.into()),
            ..Self::default()
        }
    }

    pub(crate) fn passed(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.failure.is_none())
            .count()
    }

    pub(crate) fn failed(&self) -> usize {
        self.entries.len().saturating_sub(self.passed())
    }

    pub(crate) fn is_valid(&self) -> bool {
        self.load_error.is_none() && self.failed() == 0
    }
}

/// Loads each migrated map through the production TMX/TSX/Image AssetServer chain, verifies its
/// visible tile projection, and advances five additional headless Bevy frames while retaining the
/// loaded map. The two `sample_*` maps are engine fixtures, not migrated scenario maps.
pub(crate) fn build_runtime_map_sweep(
    asset_base: &Path,
    root: &ScenarioRoot,
    physical_root: &Path,
) -> RuntimeMapSweepReport {
    let manifest_text = match fs::read_to_string(physical_root.join("manifest.yaml")) {
        Ok(text) => text,
        Err(error) => {
            return RuntimeMapSweepReport::with_load_error(format!(
                "manifest.yaml could not be read: {error}"
            ));
        }
    };
    let manifest: Manifest = match scenario_yaml::from_str(&manifest_text) {
        Ok(manifest) => manifest,
        Err(error) => {
            return RuntimeMapSweepReport::with_load_error(format!(
                "manifest.yaml is invalid: {error}"
            ));
        }
    };
    let mut ids = match read_tmx_ids(&physical_root.join(manifest.refs.tmx.as_str())) {
        Ok(ids) => ids,
        Err(error) => return RuntimeMapSweepReport::with_load_error(error),
    };
    let skipped_fixtures = ids
        .iter()
        .filter(|id| id.starts_with("sample_"))
        .cloned()
        .collect::<Vec<_>>();
    ids.retain(|id| !id.starts_with("sample_"));
    if ids.is_empty() {
        return RuntimeMapSweepReport {
            skipped_fixtures,
            ..RuntimeMapSweepReport::default()
        };
    }

    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin))
        .add_plugins(AssetPlugin {
            file_path: asset_base.to_string_lossy().into_owned(),
            meta_check: AssetMetaCheck::Never,
            ..Default::default()
        })
        .add_plugins(ImagePlugin::default_nearest())
        .register_asset_loader(ImageLoader::new(CompressedImageFormats::empty()))
        .insert_resource(root.clone())
        .insert_state(AppState::Title)
        .add_plugins(TsxAtlasAssetPlugin)
        .add_plugins(TmxGroundAssetPlugin);

    let paths = ids
        .iter()
        .map(|id| {
            ScenarioRelativePath::try_from(format!("{}/{id}.tmx", manifest.refs.tmx.as_str()))
                .expect("a portable map id beneath a validated manifest directory remains valid")
        })
        .collect::<Vec<_>>();
    let handles = {
        let server = app.world().resource::<AssetServer>();
        paths
            .iter()
            .map(|path| server.load::<TmxGroundAsset>(root.resolve(path)))
            .collect::<Vec<_>>()
    };

    for _ in 0..MAX_LOAD_UPDATES {
        app.update();
        let server = app.world().resource::<AssetServer>();
        if handles.iter().all(|handle| {
            server.is_loaded_with_dependencies(handle.id())
                || matches!(server.load_state(handle.id()), LoadState::Failed(_))
                || server
                    .recursive_dependency_load_state(handle.id())
                    .is_failed()
        }) {
            break;
        }
        thread::yield_now();
    }

    let mut entries = Vec::with_capacity(ids.len());
    for (id, handle) in ids.into_iter().zip(handles) {
        let load_failure = {
            let server = app.world().resource::<AssetServer>();
            if server.is_loaded_with_dependencies(handle.id()) {
                None
            } else {
                Some(format!(
                    "asset load did not complete: root={:?}, dependencies={:?}",
                    server.load_state(handle.id()),
                    server.recursive_dependency_load_state(handle.id())
                ))
            }
        };
        let projection_failure = if load_failure.is_none() {
            let maps = app.world().resource::<Assets<TmxGroundAsset>>();
            let atlases = app.world().resource::<Assets<TsxAtlasAsset>>();
            maps.get(&handle)
                .ok_or_else(|| "loaded TMX was not published".to_owned())
                .and_then(|map| {
                    map.visible_bundles(atlases)
                        .map(|_| ())
                        .map_err(|error| format!("visible tile projection failed: {error}"))
                })
                .err()
        } else {
            None
        };
        let failure = load_failure.or(projection_failure);
        let frames = if failure.is_none() {
            for _ in 0..HEADLESS_FRAMES_PER_MAP {
                app.update();
            }
            HEADLESS_FRAMES_PER_MAP
        } else {
            0
        };
        entries.push(RuntimeMapSweepEntry {
            id,
            frames,
            failure,
        });
    }

    RuntimeMapSweepReport {
        entries,
        skipped_fixtures,
        load_error: None,
    }
}

fn read_tmx_ids(directory: &Path) -> Result<Vec<String>, String> {
    let mut ids = Vec::new();
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("TMX directory could not be read: {error}"))?;
    for entry in entries {
        let path = entry
            .map_err(|error| format!("TMX directory entry could not be read: {error}"))?
            .path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("tmx") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
            return Err("TMX filename is not portable UTF-8".to_owned());
        };
        ids.push(stem.to_owned());
    }
    ids.sort();
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_package_returns_a_bounded_load_error() {
        let report = build_runtime_map_sweep(
            Path::new("/nonexistent/assets"),
            &ScenarioRoot::default(),
            Path::new("/nonexistent/assets/scenarios/rusted_kingdoms"),
        );

        assert!(!report.is_valid());
        assert!(report.entries.is_empty());
        assert!(report.load_error.unwrap().contains("manifest.yaml"));
    }
}
