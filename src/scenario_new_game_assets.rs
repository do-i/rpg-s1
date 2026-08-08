//! Transactional AssetServer loading for the complete new-game input set.

use std::{error::Error, fmt};

use bevy::{
    asset::{
        AssetApp, AssetLoadError, AssetLoader, AssetServer, Assets, Handle, LoadContext, LoadState,
        io::{AssetReaderError, Reader},
    },
    ecs::schedule::IntoScheduleConfigs,
    prelude::{App, Plugin, Res, ResMut, Resource, Update},
    reflect::TypePath,
};

use crate::{
    scenario_balance::BalanceData,
    scenario_manifest::Manifest,
    scenario_manifest_asset::{
        ActiveManifestLoad, ActiveManifestStatus, ManifestLoadFailure, track_active_manifest_load,
    },
    scenario_party::PartyCatalog,
    scenario_root::ScenarioRoot,
    scenario_yaml::{self, ScenarioYamlError},
};

pub struct ScenarioNewGameAssetsPlugin;

impl Plugin for ScenarioNewGameAssetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<PartyCatalog>()
            .init_asset::<BalanceData>()
            .init_asset_loader::<PartyCatalogAssetLoader>()
            .init_asset_loader::<BalanceDataAssetLoader>()
            .init_resource::<ActiveNewGameInputs>()
            .add_systems(
                Update,
                track_new_game_inputs.after(track_active_manifest_load),
            );
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActiveNewGameInputsStatus {
    Loading,
    Ready,
    Failed,
}

#[derive(Debug, Resource)]
pub struct ActiveNewGameInputs {
    status: ActiveNewGameInputsStatus,
    handles: Option<NewGameHandles>,
    failure: Option<NewGameInputsLoadFailure>,
}

#[derive(Clone, Debug)]
struct NewGameHandles {
    party: Handle<PartyCatalog>,
    balance: Handle<BalanceData>,
    party_path: String,
    balance_path: String,
}

impl Default for ActiveNewGameInputs {
    fn default() -> Self {
        Self {
            status: ActiveNewGameInputsStatus::Loading,
            handles: None,
            failure: None,
        }
    }
}

pub struct NewGameInputs<'a> {
    pub manifest: &'a Manifest,
    pub party: &'a PartyCatalog,
    pub balance: &'a BalanceData,
}

impl ActiveNewGameInputs {
    pub const fn status(&self) -> ActiveNewGameInputsStatus {
        self.status
    }
    pub fn failure(&self) -> Option<&NewGameInputsLoadFailure> {
        self.failure.as_ref()
    }
    pub fn inputs<'a>(
        &self,
        active_manifest: &ActiveManifestLoad,
        manifests: &'a Assets<Manifest>,
        parties: &'a Assets<PartyCatalog>,
        balances: &'a Assets<BalanceData>,
    ) -> Option<NewGameInputs<'a>> {
        let handles = self.handles.as_ref()?;
        if self.status != ActiveNewGameInputsStatus::Ready {
            return None;
        }
        let manifest = active_manifest.manifest(manifests)?;
        if manifest.refs.party.as_str() != handles.party_path
            || manifest.refs.balance.as_str() != handles.balance_path
        {
            return None;
        }
        Some(NewGameInputs {
            manifest,
            party: parties.get(&handles.party)?,
            balance: balances.get(&handles.balance)?,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NewGameInputsLoadFailure {
    package_key: String,
    scenario_relative_path: String,
    cause: String,
}
impl NewGameInputsLoadFailure {
    fn new(root: &ScenarioRoot, path: &str, error: &AssetLoadError) -> Self {
        Self {
            package_key: root.package_key().to_owned(),
            scenario_relative_path: path.to_owned(),
            cause: safe_cause(error),
        }
    }
    fn manifest_failure(failure: &ManifestLoadFailure) -> Self {
        Self {
            package_key: failure.package_key().to_owned(),
            scenario_relative_path: failure.scenario_relative_path().to_owned(),
            cause: failure.cause().to_owned(),
        }
    }
    pub fn package_key(&self) -> &str {
        &self.package_key
    }
    pub fn scenario_relative_path(&self) -> &str {
        &self.scenario_relative_path
    }
    pub fn cause(&self) -> &str {
        &self.cause
    }
}
impl fmt::Display for NewGameInputsLoadFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}: {}",
            self.package_key, self.scenario_relative_path, self.cause
        )
    }
}
impl Error for NewGameInputsLoadFailure {}

fn track_new_game_inputs(
    mut active: ResMut<ActiveNewGameInputs>,
    active_manifest: Res<ActiveManifestLoad>,
    manifests: Res<Assets<Manifest>>,
    parties: Res<Assets<PartyCatalog>>,
    balances: Res<Assets<BalanceData>>,
    asset_server: Res<AssetServer>,
) {
    let root = active_manifest.root();
    let Some(manifest) = active_manifest.manifest(&manifests) else {
        active.handles = None;
        active.failure = if active_manifest.status() == ActiveManifestStatus::Failed {
            active_manifest
                .failure()
                .map(NewGameInputsLoadFailure::manifest_failure)
        } else {
            None
        };
        active.status = if active.failure.is_some() {
            ActiveNewGameInputsStatus::Failed
        } else {
            ActiveNewGameInputsStatus::Loading
        };
        return;
    };
    let party_path = manifest.refs.party.as_str().to_owned();
    let balance_path = manifest.refs.balance.as_str().to_owned();
    if active.handles.as_ref().is_some_and(|handles| {
        handles.party_path != party_path || handles.balance_path != balance_path
    }) {
        active.handles = None;
        active.failure = None;
        active.status = ActiveNewGameInputsStatus::Loading;
    }
    if active.handles.is_none() {
        active.handles = Some(NewGameHandles {
            party: asset_server.load(root.resolve(&manifest.refs.party)),
            party_path,
            balance: asset_server.load(root.resolve(&manifest.refs.balance)),
            balance_path,
        });
        active.status = ActiveNewGameInputsStatus::Loading;
        active.failure = None;
        return;
    }
    let handles = active.handles.as_ref().expect("set above");
    if let LoadState::Failed(error) = asset_server.load_state(handles.party.id()) {
        active.failure = Some(NewGameInputsLoadFailure::new(
            root,
            manifest.refs.party.as_str(),
            &error,
        ));
        active.status = ActiveNewGameInputsStatus::Failed;
        return;
    }
    if let LoadState::Failed(error) = asset_server.load_state(handles.balance.id()) {
        active.failure = Some(NewGameInputsLoadFailure::new(
            root,
            manifest.refs.balance.as_str(),
            &error,
        ));
        active.status = ActiveNewGameInputsStatus::Failed;
        return;
    }
    if matches!(
        asset_server.load_state(handles.party.id()),
        LoadState::Loaded
    ) && matches!(
        asset_server.load_state(handles.balance.id()),
        LoadState::Loaded
    ) && parties.contains(&handles.party)
        && balances.contains(&handles.balance)
    {
        active.status = ActiveNewGameInputsStatus::Ready;
        active.failure = None;
    } else {
        active.status = ActiveNewGameInputsStatus::Loading;
        active.failure = None;
    }
}

fn safe_cause(error: &AssetLoadError) -> String {
    match error {
        AssetLoadError::AssetReaderError(AssetReaderError::NotFound(_)) => {
            "asset was not found".to_owned()
        }
        AssetLoadError::AssetReaderError(AssetReaderError::Io(error)) => {
            format!("asset I/O failed ({:?})", error.kind())
        }
        AssetLoadError::AssetLoaderError(error) => error
            .error()
            .downcast_ref::<NewGameAssetLoaderError>()
            .map(ToString::to_string)
            .unwrap_or_else(|| "asset loader failed".to_owned()),
        _ => "asset loading failed".to_owned(),
    }
}

#[derive(Default, TypePath)]
struct PartyCatalogAssetLoader;
#[derive(Default, TypePath)]
struct BalanceDataAssetLoader;
macro_rules! loader {
    ($type:ty, $loader:ident) => {
        impl AssetLoader for $loader {
            type Asset = $type;
            type Settings = ();
            type Error = NewGameAssetLoaderError;
            async fn load(
                &self,
                reader: &mut dyn Reader,
                _: &(),
                _: &mut LoadContext<'_>,
            ) -> Result<$type, Self::Error> {
                let mut bytes = Vec::new();
                reader
                    .read_to_end(&mut bytes)
                    .await
                    .map_err(NewGameAssetLoaderError::Io)?;
                let document =
                    std::str::from_utf8(&bytes).map_err(NewGameAssetLoaderError::Utf8)?;
                scenario_yaml::from_str(document).map_err(NewGameAssetLoaderError::Yaml)
            }
            fn extensions(&self) -> &[&str] {
                &["yaml", "yml"]
            }
        }
    };
}
loader!(PartyCatalog, PartyCatalogAssetLoader);
loader!(BalanceData, BalanceDataAssetLoader);
#[derive(Debug)]
enum NewGameAssetLoaderError {
    Io(std::io::Error),
    Utf8(std::str::Utf8Error),
    Yaml(ScenarioYamlError),
}
impl fmt::Display for NewGameAssetLoaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "read failed: {e}"),
            Self::Utf8(e) => write!(f, "asset is not UTF-8: {e}"),
            Self::Yaml(e) => write!(f, "YAML is invalid: {e}"),
        }
    }
}
impl Error for NewGameAssetLoaderError {}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::*;
    use crate::{
        app_state::AppState, scenario_root::ScenarioRoot,
        test_support::headless_title_app_with_asset_base,
    };

    static NEXT_TEMPORARY_ID: AtomicU64 = AtomicU64::new(0);

    struct InventedAssetBase(PathBuf);
    impl InventedAssetBase {
        fn new(package_key: &str, party: Option<&str>, balance: Option<&str>) -> Self {
            let unique = NEXT_TEMPORARY_ID.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "rpg-s1-new-game-inputs-{}-{unique}",
                std::process::id()
            ));
            let package = root.join("scenarios").join(package_key);
            fs::create_dir_all(package.join("data")).expect("invented data directory");
            let manifest = include_str!("../tests/fixtures/rusted-kingdoms-manifest-complete.yaml")
                .replacen("data/party.yaml", "data/party-alt.yaml", 1)
                .replacen("data/balance.yaml", "data/balance-alt.yaml", 1);
            fs::write(package.join("manifest.yaml"), manifest).expect("invented manifest");
            if let Some(party) = party {
                fs::write(package.join("data/party-alt.yaml"), party).expect("invented party");
            }
            if let Some(balance) = balance {
                fs::write(package.join("data/balance-alt.yaml"), balance)
                    .expect("invented balance");
            }
            Self(root)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for InventedAssetBase {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("temporary assets removed");
        }
    }

    fn party() -> &'static str {
        include_str!("../tests/fixtures/party-catalog-shapes.yaml")
    }
    fn balance() -> &'static str {
        include_str!("../tests/fixtures/balance-complete.yaml")
    }
    fn app_for(base: &InventedAssetBase, key: &str) -> App {
        headless_title_app_with_asset_base(
            AppState::Title,
            base.path().to_string_lossy().into_owned(),
            ScenarioRoot::try_for_package_key(key).expect("valid invented key"),
        )
    }
    fn update_until(app: &mut App, wanted: ActiveNewGameInputsStatus) {
        for _ in 0..1_000 {
            app.update();
            if app.world().resource::<ActiveNewGameInputs>().status() == wanted {
                return;
            }
            std::thread::yield_now();
        }
        panic!("new-game inputs did not reach {wanted:?}");
    }
    fn inputs_present(app: &App) -> bool {
        let world = app.world();
        world
            .resource::<ActiveNewGameInputs>()
            .inputs(
                world.resource::<ActiveManifestLoad>(),
                world.resource::<Assets<Manifest>>(),
                world.resource::<Assets<PartyCatalog>>(),
                world.resource::<Assets<BalanceData>>(),
            )
            .is_some()
    }

    #[test]
    fn publishes_all_invented_inputs_together_with_exact_selected_paths() {
        let assets = InventedAssetBase::new("invented_campaign", Some(party()), Some(balance()));
        let mut app = app_for(&assets, "invented_campaign");
        app.update();
        assert!(
            !inputs_present(&app),
            "loading must not expose a partial view"
        );
        update_until(&mut app, ActiveNewGameInputsStatus::Ready);
        let world = app.world();
        let active = world.resource::<ActiveNewGameInputs>();
        let inputs = active
            .inputs(
                world.resource::<ActiveManifestLoad>(),
                world.resource::<Assets<Manifest>>(),
                world.resource::<Assets<PartyCatalog>>(),
                world.resource::<Assets<BalanceData>>(),
            )
            .expect("all inputs together");
        assert!(!inputs.party.party.is_empty());
        assert_eq!(inputs.balance.economy.gp_cap.get(), 600_000);
        let handles = active.handles.as_ref().expect("ready handles retained");
        let server = world.resource::<AssetServer>();
        assert_eq!(
            server
                .get_path(handles.party.id())
                .unwrap()
                .path()
                .to_string_lossy(),
            "scenarios/invented_campaign/data/party-alt.yaml"
        );
        assert_eq!(
            server
                .get_path(handles.balance.id())
                .unwrap()
                .path()
                .to_string_lossy(),
            "scenarios/invented_campaign/data/balance-alt.yaml"
        );
    }

    #[test]
    fn dependent_failures_never_publish_partial_or_host_paths() {
        let cases = [
            (
                "missing_party",
                None,
                Some(balance()),
                "data/party-alt.yaml",
            ),
            (
                "bad_party",
                Some("invalid: ["),
                Some(balance()),
                "data/party-alt.yaml",
            ),
            (
                "missing_balance",
                Some(party()),
                None,
                "data/balance-alt.yaml",
            ),
            (
                "bad_balance",
                Some(party()),
                Some("invalid: ["),
                "data/balance-alt.yaml",
            ),
        ];
        for (key, party, balance, path) in cases {
            let assets = InventedAssetBase::new(key, party, balance);
            let mut app = app_for(&assets, key);
            update_until(&mut app, ActiveNewGameInputsStatus::Failed);
            let failure = app
                .world()
                .resource::<ActiveNewGameInputs>()
                .failure()
                .expect("failure published")
                .to_string();
            assert!(failure.starts_with(&format!("{key}:{path}:")));
            assert!(!failure.contains(assets.path().to_string_lossy().as_ref()));
            assert!(!inputs_present(&app));
        }
    }

    #[test]
    fn removing_a_ready_selected_asset_revokes_the_complete_view() {
        let assets = InventedAssetBase::new("invented_campaign", Some(party()), Some(balance()));
        let mut app = app_for(&assets, "invented_campaign");
        update_until(&mut app, ActiveNewGameInputsStatus::Ready);
        let party_handle = app
            .world()
            .resource::<ActiveNewGameInputs>()
            .handles
            .as_ref()
            .unwrap()
            .party
            .clone();
        app.world_mut()
            .resource_mut::<Assets<PartyCatalog>>()
            .remove(party_handle.id());
        app.update();
        assert_eq!(
            app.world().resource::<ActiveNewGameInputs>().status(),
            ActiveNewGameInputsStatus::Loading
        );
        assert!(!inputs_present(&app));

        let assets = InventedAssetBase::new("second_campaign", Some(party()), Some(balance()));
        let mut app = app_for(&assets, "second_campaign");
        update_until(&mut app, ActiveNewGameInputsStatus::Ready);
        let balance_handle = app
            .world()
            .resource::<ActiveNewGameInputs>()
            .handles
            .as_ref()
            .unwrap()
            .balance
            .clone();
        app.world_mut()
            .resource_mut::<Assets<BalanceData>>()
            .remove(balance_handle.id());
        app.update();
        assert_eq!(
            app.world().resource::<ActiveNewGameInputs>().status(),
            ActiveNewGameInputsStatus::Loading
        );
        assert!(!inputs_present(&app));
        app.update();
        assert_eq!(
            app.world().resource::<ActiveNewGameInputs>().status(),
            ActiveNewGameInputsStatus::Loading
        );
    }

    #[test]
    fn changed_manifest_references_replace_handles_without_misassociation() {
        let assets = InventedAssetBase::new("invented_campaign", Some(party()), Some(balance()));
        fs::write(
            assets
                .path()
                .join("scenarios/invented_campaign/data/party-next.yaml"),
            party(),
        )
        .expect("next party fixture");
        let mut app = app_for(&assets, "invented_campaign");
        update_until(&mut app, ActiveNewGameInputsStatus::Ready);
        let manifest_handle = app
            .world()
            .resource::<ActiveManifestLoad>()
            .handle()
            .clone();
        app.world_mut()
            .resource_mut::<Assets<Manifest>>()
            .get_mut(manifest_handle.id())
            .expect("active manifest")
            .refs
            .party = "data/party-next.yaml"
            .try_into()
            .expect("valid changed ref");
        assert!(
            !inputs_present(&app),
            "the view must reject changed refs before tracker update"
        );
        app.update();
        assert_eq!(
            app.world().resource::<ActiveNewGameInputs>().status(),
            ActiveNewGameInputsStatus::Loading
        );
        assert!(!inputs_present(&app));
        update_until(&mut app, ActiveNewGameInputsStatus::Ready);
        assert_eq!(
            app.world()
                .resource::<ActiveNewGameInputs>()
                .handles
                .as_ref()
                .unwrap()
                .party_path,
            "data/party-next.yaml"
        );
    }

    #[test]
    fn active_manifest_root_owns_dependent_failure_provenance() {
        let assets = InventedAssetBase::new("old_campaign", None, Some(balance()));
        let mut app = app_for(&assets, "old_campaign");
        app.insert_resource(ScenarioRoot::try_for_package_key("new_campaign").unwrap());
        update_until(&mut app, ActiveNewGameInputsStatus::Failed);
        let failure = app
            .world()
            .resource::<ActiveNewGameInputs>()
            .failure()
            .unwrap();
        assert_eq!(failure.package_key(), "old_campaign");
        assert_eq!(failure.scenario_relative_path(), "data/party-alt.yaml");
    }
}
