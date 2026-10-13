//! Opt-in, coordinated desktop hot reload. Bevy's autonomous watcher stays disabled:
//! the source generation must be retained before any asset can be replaced in place.

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};

use bevy::{
    asset::{UntypedAssetId, UntypedAssetLoadFailedEvent},
    ecs::{message::MessageCursor, system::RunSystemOnce},
    prelude::*,
};

use super::recovery;
use crate::{
    app_state::AppState,
    field_menu_domain::{FieldMenuCatalog, FieldMenuCatalogLoad},
    gameplay_rng::GameplayRng,
    scenario_inventory::ScenarioInventory,
    scenario_manifest::Manifest,
    scenario_manifest_asset::ActiveManifestLoad,
    scenario_new_game_assets::ActiveNewGameInputs,
    scenario_root::ScenarioRoot,
    transport_domain::{TransportCatalogLoad, TransportDomain},
    world_publication::PublicationStatus,
};

pub(crate) struct ScenarioHotReloadPlugin {
    asset_base: PathBuf,
    enabled: bool,
}

impl ScenarioHotReloadPlugin {
    pub(crate) fn new(asset_base: PathBuf, enabled: bool) -> Self {
        Self {
            asset_base,
            enabled,
        }
    }
}

// Both snapshots and completion tracking must cover the same typed asset boundaries.
macro_rules! scenario_assets {
    ($action:ident, $target:expr) => {
        $action::<crate::tmx_ground_asset::TmxGroundAsset>($target);
        $action::<crate::tsx_atlas_asset::TsxAtlasAsset>($target);
        $action::<Image>($target);
        $action::<TextureAtlasLayout>($target);
        $action::<AudioSource>($target);
        $action::<Font>($target);
        $action::<Manifest>($target);
        $action::<crate::scenario_map::MapMetadata>($target);
        $action::<crate::scenario_audio::BgmIndex>($target);
        $action::<crate::scenario_audio::SfxIndex>($target);
        $action::<crate::scenario_party::PartyCatalog>($target);
        $action::<crate::scenario_balance::BalanceData>($target);
        $action::<crate::scenario_dialogue::CutsceneDialogue>($target);
        $action::<crate::scenario_dialogue::DialogueDocument>($target);
        $action::<crate::scenario_item::ItemCatalogFile>($target);
        $action::<crate::scenario_item::FieldUseCatalogFile>($target);
        $action::<crate::scenario_class::ClassDefinition>($target);
        $action::<crate::scenario_recipe::RecipeCatalogFile>($target);
        $action::<crate::scenario_quest::QuestCatalogFile>($target);
        $action::<crate::scenario_enemy::EnemyCatalogFile>($target);
        $action::<crate::scenario_enemy::BossMoveSet>($target);
        $action::<crate::scenario_encounter::EncounterZone>($target);
        $action::<crate::scenario_battle_background::BattleBackgroundCatalog>($target);
        $action::<crate::scenario_transport::TransportCatalog>($target);
    };
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct CollectReloads;

impl Plugin for ScenarioHotReloadPlugin {
    fn build(&self, app: &mut App) {
        let root = app.world().resource::<ScenarioRoot>().clone();
        let inventory = app.world().resource::<ScenarioInventory>().clone();
        let files = if self.enabled {
            scan(&self.asset_base, &root).unwrap_or_default()
        } else {
            BTreeMap::new()
        };
        app.insert_resource(HotReload {
            asset_base: self.asset_base.clone(),
            root,
            inventory,
            enabled: self.enabled,
            files,
            elapsed: 0.0,
            revision: 0,
            active: false,
            prepared: false,
            rejected: false,
            pending: HashSet::new(),
            requested: Vec::new(),
            failure: None,
            audio: None,
            bgm: None,
            late_assets: Vec::new(),
            synthetic_events: HashMap::new(),
        })
        .add_systems(PreUpdate, guard_scenario_identity.before(CollectReloads));
        if !self.enabled {
            return;
        }
        app.configure_sets(
            PreUpdate,
            CollectReloads.after(bevy::asset::AssetTrackingSystems),
        )
        .add_systems(PreUpdate, collect_failures.in_set(CollectReloads))
        .add_systems(
            PreUpdate,
            restore_late_assets
                .after(CollectReloads)
                .before(poll_changes),
        )
        .add_systems(
            PreUpdate,
            poll_changes
                .after(CollectReloads)
                .run_if(in_state(AppState::World))
                .run_if(crate::world_pause::world_simulation_running),
        );
        scenario_assets!(register_completion, app);
    }
}

fn register_completion<A: Asset>(app: &mut App) {
    app.add_systems(PreUpdate, collect_completed::<A>.in_set(CollectReloads));
}

#[derive(Resource)]
pub(crate) struct HotReload {
    asset_base: PathBuf,
    root: ScenarioRoot,
    inventory: ScenarioInventory,
    enabled: bool,
    files: BTreeMap<String, FileRevision>,
    elapsed: f32,
    pub(super) revision: u64,
    active: bool,
    prepared: bool,
    rejected: bool,
    pending: HashSet<UntypedAssetId>,
    requested: Vec<String>,
    failure: Option<String>,
    audio: Option<Handle<AudioSource>>,
    bgm: Option<Handle<crate::scenario_audio::BgmIndex>>,
    late_assets: Vec<AssetRestore>,
    synthetic_events: HashMap<UntypedAssetId, usize>,
}

/// Freeze global asset-derived state until the candidate is complete, or while serving a
/// rolled-back generation whose AssetServer load states still describe the rejected files.
pub(crate) fn published_inputs_enabled(reload: Option<Res<HotReload>>) -> bool {
    reload.is_none_or(|reload| !reload.active && !reload.rejected)
}

pub(super) fn loaders_ready(reload: Option<Res<HotReload>>) -> bool {
    reload.is_none_or(|r| !r.active || r.prepared)
}

fn guard_scenario_identity(world: &mut World) {
    let reload = world.resource::<HotReload>();
    if world.resource::<ScenarioRoot>() == &reload.root {
        return;
    }
    let root = reload.root.clone();
    let inventory = reload.inventory.clone();
    error!(
        "Scenario selection is fixed for this process; launch with play PACKAGE_KEY to select another package"
    );
    world.insert_resource(root);
    world.insert_resource(inventory);
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FileRevision {
    size: u64,
    modified: SystemTime,
    #[cfg(unix)]
    identity: (u64, i64, i64),
}

fn scan(base: &Path, root: &ScenarioRoot) -> std::io::Result<BTreeMap<String, FileRevision>> {
    let mut files = BTreeMap::new();
    let mut directories = vec![base.join(root.logical_root())];
    while let Some(directory) = directories.pop() {
        let entries = match fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        };
        for entry in entries {
            let entry = entry?;
            let kind = entry.file_type()?;
            let path = entry.path();
            if kind.is_dir() {
                directories.push(path);
            } else if kind.is_file()
                && matches!(
                    path.extension().and_then(|s| s.to_str()),
                    Some(
                        "yaml"
                            | "yml"
                            | "tmx"
                            | "tsx"
                            | "png"
                            | "webp"
                            | "mp3"
                            | "ogg"
                            | "wav"
                            | "ttf"
                            | "otf"
                    )
                )
            {
                let metadata = entry.metadata()?;
                let relative = path.strip_prefix(base).expect("scanned beneath asset base");
                files.insert(
                    relative.to_string_lossy().replace('\\', "/"),
                    FileRevision {
                        size: metadata.len(),
                        modified: metadata.modified()?,
                        #[cfg(unix)]
                        identity: {
                            use std::os::unix::fs::MetadataExt;
                            (metadata.ino(), metadata.ctime(), metadata.ctime_nsec())
                        },
                    },
                );
            }
        }
    }
    Ok(files)
}

fn poll_changes(world: &mut World) {
    let delta = world.resource::<Time<Real>>().delta_secs();
    let mut reload = world.resource_mut::<HotReload>();
    if !reload.enabled || reload.active || !reload.late_assets.is_empty() {
        return;
    }
    reload.elapsed += delta;
    if reload.elapsed < 0.5 {
        return;
    }
    reload.elapsed = 0.0;
    let files = match scan(&reload.asset_base, &reload.root) {
        Ok(files) => files,
        Err(error) => {
            warn!("Unable to inspect scenario hot changes: {error}");
            return;
        }
    };
    let mut changed = files
        .keys()
        .chain(reload.files.keys())
        .filter(|path| files.get(*path) != reload.files.get(*path))
        .cloned()
        .collect::<Vec<_>>();
    if changed.is_empty() {
        return;
    }
    if reload.rejected {
        changed.extend(reload.requested.iter().cloned());
    }
    changed.sort();
    changed.dedup();
    reload.files = files;
    if changed.is_empty() {
        return;
    }
    reload.requested = changed;
    begin(world);
}

fn collect_completed<A: Asset>(
    messages: Option<Res<Messages<AssetEvent<A>>>>,
    mut cursor: Local<MessageCursor<AssetEvent<A>>>,
    mut reload: ResMut<HotReload>,
) {
    let Some(messages) = messages else {
        return;
    };
    for event in cursor.read(&messages) {
        if let AssetEvent::Added { id } | AssetEvent::Modified { id } = event {
            let id = id.untyped();
            if let Some(count) = reload.synthetic_events.get_mut(&id) {
                *count -= 1;
                if *count == 0 {
                    reload.synthetic_events.remove(&id);
                }
            } else {
                reload.pending.remove(&id);
            }
        }
    }
}

fn collect_failures(
    mut events: MessageReader<UntypedAssetLoadFailedEvent>,
    mut reload: ResMut<HotReload>,
) {
    for event in events.read() {
        if reload.pending.remove(&event.id) && reload.active {
            reload.failure = Some(format!(
                "hot reload failed for {}: {}",
                event.path, event.error
            ));
        }
    }
}

type Restore = Box<dyn FnOnce(&mut World) + Send + Sync>;
type AssetRestore = Arc<dyn Fn(&mut World) + Send + Sync>;

pub(super) fn recovering_late_assets(world: &World) -> bool {
    world
        .get_resource::<HotReload>()
        .is_some_and(|reload| !reload.late_assets.is_empty())
}

fn restore_late_assets(world: &mut World) {
    // Bevy applies completed IO in PreUpdate. Reassert the retained generation after that
    // boundary and before any Update consumer or renderer can see a late candidate value.
    let restores = world.resource::<HotReload>().late_assets.clone();
    for restore in restores {
        restore(world);
    }
    if world.resource::<HotReload>().pending.is_empty() {
        world.resource_mut::<HotReload>().late_assets.clear();
    }
}

/// The generation retained alongside RF-02's dormant World bundles. Strong handles keep
/// replaced TSX layouts and images alive until the candidate commits or is rolled back.
#[derive(Default)]
pub(super) struct ReloadSnapshot {
    resources: Vec<Restore>,
    assets: Vec<AssetRestore>,
}

impl ReloadSnapshot {
    pub(super) fn take(world: &mut World) -> Self {
        let mut snapshot = Self::default();
        scenario_assets!(snapshot_assets, (&mut snapshot, &mut *world));
        snapshot.resource::<ScenarioInventory>(world);
        snapshot.resource::<ActiveManifestLoad>(world);
        snapshot.resource::<ActiveNewGameInputs>(world);
        snapshot.resource::<FieldMenuCatalog>(world);
        snapshot.resource::<FieldMenuCatalogLoad>(world);
        snapshot.resource::<TransportDomain>(world);
        snapshot.resource::<TransportCatalogLoad>(world);
        snapshot.resource::<GameplayRng>(world);
        snapshot
    }

    fn resource<R: Resource + Clone>(&mut self, world: &World) {
        if let Some(value) = world.get_resource::<R>().cloned() {
            self.resources.push(Box::new(move |world| {
                world.insert_resource(value);
            }));
        }
    }

    pub(super) fn restore(self, world: &mut World) {
        for restore in &self.assets {
            restore(world);
        }
        if world.resource::<HotReload>().pending.is_empty() {
            drop(self.assets);
        } else {
            world.resource_mut::<HotReload>().late_assets = self.assets;
        }
        for restore in self.resources {
            restore(world);
        }
    }
}

fn snapshot_assets<A: Asset + Clone>((snapshot, world): (&mut ReloadSnapshot, &mut World)) {
    let Some(mut assets) = world.get_resource_mut::<Assets<A>>() else {
        return;
    };
    let values = assets
        .iter()
        .map(|(id, asset)| (id, asset.clone()))
        .collect::<Vec<_>>();
    let values = values
        .into_iter()
        .map(|(id, value)| {
            // UUID assets are permanently addressed and cannot have strong handles.
            let handle = assets.get_strong_handle(id);
            (id, handle, value)
        })
        .collect::<Vec<_>>();
    snapshot.assets.push(Arc::new(move |world| {
        let mut assets = world.resource_mut::<Assets<A>>();
        for (id, _handle, value) in &values {
            assets
                .insert(*id, value.clone())
                .expect("snapshot retains asset generation");
        }
        // Assets::insert emits one event per write. Do not mistake our rollback writes for
        // completion of an outstanding AssetServer request (especially after a timeout).
        let mut reload = world.resource_mut::<HotReload>();
        for (id, _, _) in &values {
            *reload.synthetic_events.entry(id.untyped()).or_default() += 1;
        }
    }));
}

pub(super) fn begin(world: &mut World) {
    let (base, root, paths) = {
        let reload = world.resource::<HotReload>();
        (
            reload.asset_base.clone(),
            reload.root.clone(),
            reload.requested.clone(),
        )
    };
    let snapshot = ReloadSnapshot::take(world);
    recovery::begin_hot_reload(world, snapshot);
    let inventory = ScenarioInventory::discover(&base, &root);
    let mut failure = inventory.failure.clone();
    // A running save/session cannot change its scenario identity by editing manifest.yaml.
    let manifest_path = base.join(root.manifest_asset_path());
    if let Ok(document) = fs::read_to_string(manifest_path)
        && let Ok(candidate) = crate::scenario_yaml::from_str::<Manifest>(&document)
        && let Some(active) = world.get_resource::<ActiveManifestLoad>()
        && let Some(published) = active.manifest(world.resource::<Assets<Manifest>>())
        && (candidate.id != published.id || candidate.version != published.version)
    {
        failure = Some("scenario manifest identity changed; restart the selected scenario".into());
    }
    let server = world.resource::<AssetServer>();
    let pending = if failure.is_none() {
        paths
            .iter()
            .flat_map(|path| server.get_path_ids(path.clone()))
            .collect()
    } else {
        HashSet::new()
    };
    if failure.is_none() {
        for path in &paths {
            server.reload(path.clone());
        }
    }
    world.insert_resource(inventory);
    let files = scan(&base, &root).ok();
    let mut reload = world.resource_mut::<HotReload>();
    if let Some(files) = files {
        reload.files = files;
    }
    reload.revision += 1;
    reload.active = true;
    reload.prepared = false;
    reload.rejected = false;
    reload.pending = pending;
    reload.failure = failure;
    reload.audio = None;
    reload.bgm = None;
    info!(
        revision = reload.revision,
        "Rebuilding scenario hot changes through World publication"
    );
}

/// Called only by the existing publication driver. Completion events, rather than a fixed
/// sleep, establish that each explicit reload has replaced its cached asset or failed.
pub(super) fn prepare(world: &mut World) -> PublicationStatus {
    let Some(reload) = world.get_resource::<HotReload>() else {
        return PublicationStatus::Ready;
    };
    if !reload.active {
        return PublicationStatus::Ready;
    }
    if !reload.pending.is_empty() {
        return PublicationStatus::Pending;
    }
    if let Some(failure) = &reload.failure {
        return PublicationStatus::Failed(failure.clone());
    }
    if !reload.prepared {
        if world.contains_resource::<FieldMenuCatalogLoad>() {
            world.insert_resource(FieldMenuCatalog::default());
            world
                .run_system_once(crate::field_menu_domain::begin_catalog_load)
                .expect("catalog restart system");
        }
        if world.contains_resource::<TransportCatalogLoad>() {
            world
                .run_system_once(crate::transport_domain::begin_transport_load)
                .expect("transport restart system");
        }
        world.resource_mut::<HotReload>().prepared = true;
        return PublicationStatus::Pending;
    }
    if world.contains_resource::<FieldMenuCatalogLoad>() {
        let catalog = world.resource::<FieldMenuCatalog>();
        match catalog.status() {
            crate::field_menu_domain::CatalogStatus::Loading => return PublicationStatus::Pending,
            crate::field_menu_domain::CatalogStatus::Failed => {
                return PublicationStatus::Failed(
                    catalog
                        .failure()
                        .unwrap_or("catalog hot reload failed")
                        .into(),
                );
            }
            _ => {}
        }
    }
    if let Some(domain) = world.get_resource::<TransportDomain>() {
        match domain.status() {
            crate::transport_domain::TransportStatus::Loading => return PublicationStatus::Pending,
            crate::transport_domain::TransportStatus::Failed => {
                return PublicationStatus::Failed("transport catalog hot reload failed".into());
            }
            _ => {}
        }
    }
    if let Some(map) = world
        .resource::<crate::tmx_ground_asset::StaticMapRenderState>()
        .map(world.resource::<Assets<crate::tmx_ground_asset::TmxGroundAsset>>())
        && let Err(error) = super::runtime_portals(map.document())
    {
        return PublicationStatus::Failed(format!("hot map portals are invalid: {error}"));
    }
    audio_status(world)
}

fn audio_status(world: &mut World) -> PublicationStatus {
    use crate::{
        game_state::GameState,
        scenario_audio::{BGM_INDEX_PATH, BgmIndex},
        scenario_map::MapMetadata,
    };
    let Some(map_id) = world.resource::<GameState>().map().current() else {
        return PublicationStatus::Failed("current map is missing".into());
    };
    let root = world.resource::<ScenarioRoot>();
    let Some(path) = world
        .resource::<ScenarioInventory>()
        .map_metadata_path(map_id.as_str())
    else {
        return PublicationStatus::Failed("map metadata path is missing".into());
    };
    let server = world.resource::<AssetServer>();
    let Some(handle) = server.get_handle::<MapMetadata>(root.resolve(&path)) else {
        return PublicationStatus::Pending;
    };
    let Some(metadata) = world.resource::<Assets<MapMetadata>>().get(&handle) else {
        // TMX-only maps legitimately have no YAML. Actor/object loaders own that check.
        return PublicationStatus::Ready;
    };
    let Some(key) = &metadata.bgm else {
        return PublicationStatus::Ready;
    };
    let index_handle: Handle<BgmIndex> = world
        .resource::<HotReload>()
        .bgm
        .clone()
        .unwrap_or_else(|| server.load(format!("{}/{BGM_INDEX_PATH}", root.logical_root())));
    if crate::world_publication::asset_failed(server, index_handle.id()) {
        return PublicationStatus::Failed("BGM index failed to load".into());
    }
    let Some(index) = world.resource::<Assets<BgmIndex>>().get(&index_handle) else {
        world.resource_mut::<HotReload>().bgm = Some(index_handle);
        return PublicationStatus::Pending;
    };
    let Some(path) = index.resolve_key(root, key) else {
        return PublicationStatus::Failed(format!("unknown BGM key {key}"));
    };
    let handle: Handle<AudioSource> = world
        .resource::<HotReload>()
        .audio
        .clone()
        .unwrap_or_else(|| server.load(path));
    let status = if crate::world_publication::asset_failed(server, handle.id()) {
        PublicationStatus::Failed("BGM audio dependency failed to load".into())
    } else if server.is_loaded_with_dependencies(handle.id()) {
        PublicationStatus::Ready
    } else {
        PublicationStatus::Pending
    };
    let mut reload = world.resource_mut::<HotReload>();
    reload.audio = Some(handle);
    reload.bgm = Some(index_handle);
    status
}

pub(super) fn finish(world: &mut World, success: bool) {
    let Some(mut reload) = world.get_resource_mut::<HotReload>() else {
        return;
    };
    if !reload.active {
        return;
    }
    reload.active = false;
    reload.rejected = !success;
    if success {
        reload.requested.clear();
        let inventory = world.resource::<ScenarioInventory>().clone();
        world.resource_mut::<HotReload>().inventory = inventory;
        crate::world_audio::reload_world_audio(world);
        if let Some(mut slots) = world.get_resource_mut::<crate::save_ui::SaveSlotCatalog>() {
            slots.request_refresh();
        }
        info!("Scenario hot changes published");
    } else {
        error!("Scenario hot changes rejected; restored the published generation");
    }
}

#[cfg(test)]
pub(super) fn hold_completion_for_test(world: &mut World, id: UntypedAssetId) {
    world.resource_mut::<HotReload>().pending.insert(id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(unix)]
    fn file_revision_detects_atomic_same_size_replacement_with_preserved_mtime() {
        let directory =
            std::env::temp_dir().join(format!("rpg-hot-revision-{}", std::process::id()));
        let root = ScenarioRoot::try_for_package_key("revision").unwrap();
        let package = directory.join(root.logical_root());
        fs::create_dir_all(&package).unwrap();
        let target = package.join("map.yaml");
        fs::write(&target, b"name: first\n").unwrap();
        let original = fs::metadata(&target).unwrap().modified().unwrap();
        let before = scan(&directory, &root).unwrap();
        let replacement = package.join("map.yaml.tmp");
        fs::write(&replacement, b"name: other\n").unwrap();
        fs::File::options()
            .write(true)
            .open(&replacement)
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(original))
            .unwrap();
        fs::rename(&replacement, &target).unwrap();
        assert_ne!(before, scan(&directory, &root).unwrap());
        fs::remove_dir_all(directory).unwrap();
    }
}
