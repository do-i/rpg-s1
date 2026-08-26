//! Transactional runtime loading for the selected scenario manifest.
//!
//! Runtime bytes enter through Bevy's approved asset source. The loader delegates YAML parsing
//! to the same strict typed boundary as validation, while [`ActiveManifestLoad`] retains the
//! selected handle and publishes a manifest only after Bevy reports that exact typed asset ready.

use std::{error::Error, fmt};

use bevy::{
    asset::{
        AssetApp, AssetLoadError, AssetLoader, AssetServer, Assets, Handle, LoadContext, LoadState,
        io::{AssetReaderError, Reader},
    },
    prelude::{App, Plugin, Res, ResMut, Resource, Update},
    reflect::TypePath,
};

use crate::{
    scenario_manifest::Manifest,
    scenario_root::{SCENARIO_MANIFEST_PATH, ScenarioRoot},
    scenario_yaml::{self, ScenarioYamlError},
};

/// Registers direct YAML loading and begins loading the selected package manifest.
pub struct ScenarioManifestAssetPlugin;

impl Plugin for ScenarioManifestAssetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScenarioRoot>()
            .init_asset::<Manifest>()
            .init_asset_loader::<ManifestAssetLoader>();

        let root = app.world().resource::<ScenarioRoot>().clone();
        let handle = app
            .world()
            .resource::<AssetServer>()
            .load(root.manifest_asset_path());
        app.insert_resource(ActiveManifestLoad::new(root, handle))
            .add_systems(Update, track_active_manifest_load);
    }
}

/// Observable lifecycle of the selected manifest request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActiveManifestStatus {
    Loading,
    Ready,
    Failed,
}

/// Strong selected handle plus transactional publication state.
#[derive(Debug, Resource)]
pub struct ActiveManifestLoad {
    root: ScenarioRoot,
    handle: Handle<Manifest>,
    status: ActiveManifestStatus,
    failure: Option<ManifestLoadFailure>,
}

impl ActiveManifestLoad {
    fn new(root: ScenarioRoot, handle: Handle<Manifest>) -> Self {
        Self {
            root,
            handle,
            status: ActiveManifestStatus::Loading,
            failure: None,
        }
    }

    /// Returns the selected package's current manifest lifecycle.
    pub const fn status(&self) -> ActiveManifestStatus {
        self.status
    }

    /// Returns a package-qualified failure only after the selected request fails.
    pub fn failure(&self) -> Option<&ManifestLoadFailure> {
        self.failure.as_ref()
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "tests verify hot-reload handle identity")
    )]
    pub(crate) fn handle(&self) -> &Handle<Manifest> {
        &self.handle
    }

    pub(crate) fn root(&self) -> &ScenarioRoot {
        &self.root
    }

    /// Returns the selected typed manifest only while that exact handle is ready and present.
    ///
    /// A failed, pending, or unloaded request cannot expose a stale asset value.
    pub fn manifest<'a>(&self, manifests: &'a Assets<Manifest>) -> Option<&'a Manifest> {
        (self.status == ActiveManifestStatus::Ready)
            .then(|| manifests.get(&self.handle))
            .flatten()
    }
}

/// Stable runtime diagnostic for the selected manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManifestLoadFailure {
    package_key: String,
    scenario_relative_path: String,
    cause: String,
}

impl ManifestLoadFailure {
    fn from_asset_error(root: &ScenarioRoot, error: &AssetLoadError) -> Self {
        Self {
            package_key: root.package_key().to_owned(),
            scenario_relative_path: SCENARIO_MANIFEST_PATH.to_owned(),
            cause: safe_asset_error_cause(error),
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

impl fmt::Display for ManifestLoadFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}:{}: {}",
            self.package_key, self.scenario_relative_path, self.cause
        )
    }
}

impl Error for ManifestLoadFailure {}

pub(crate) fn track_active_manifest_load(
    mut active: ResMut<ActiveManifestLoad>,
    asset_server: Res<AssetServer>,
    manifests: Res<Assets<Manifest>>,
) {
    match asset_server.load_state(active.handle.id()) {
        LoadState::Loaded if manifests.contains(&active.handle) => {
            active.status = ActiveManifestStatus::Ready;
            active.failure = None;
        }
        LoadState::Failed(error) => {
            active.failure = Some(ManifestLoadFailure::from_asset_error(&active.root, &error));
            active.status = ActiveManifestStatus::Failed;
        }
        LoadState::NotLoaded | LoadState::Loading | LoadState::Loaded => {
            active.status = ActiveManifestStatus::Loading;
            active.failure = None;
        }
    }
}

fn safe_asset_error_cause(error: &AssetLoadError) -> String {
    match error {
        AssetLoadError::AssetReaderError(AssetReaderError::NotFound(_)) => {
            "manifest asset was not found".to_owned()
        }
        AssetLoadError::AssetReaderError(AssetReaderError::Io(error)) => {
            format!("manifest asset I/O failed ({:?})", error.kind())
        }
        AssetLoadError::AssetReaderError(AssetReaderError::HttpError(status)) => {
            format!("manifest asset request returned HTTP status {status}")
        }
        AssetLoadError::AssetLoaderError(error) => error
            .error()
            .downcast_ref::<ManifestAssetLoaderError>()
            .map(ToString::to_string)
            .unwrap_or_else(|| "manifest asset loader failed".to_owned()),
        _ => "manifest asset loading failed".to_owned(),
    }
}

#[derive(Default, TypePath)]
struct ManifestAssetLoader;

impl AssetLoader for ManifestAssetLoader {
    type Asset = Manifest;
    type Settings = ();
    type Error = ManifestAssetLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .await
            .map_err(ManifestAssetLoaderError::Io)?;
        let document = std::str::from_utf8(&bytes).map_err(ManifestAssetLoaderError::Utf8)?;
        scenario_yaml::from_str(document).map_err(ManifestAssetLoaderError::Yaml)
    }

    fn extensions(&self) -> &[&str] {
        &["yaml", "yml"]
    }
}

#[derive(Debug)]
enum ManifestAssetLoaderError {
    Io(std::io::Error),
    Utf8(std::str::Utf8Error),
    Yaml(ScenarioYamlError),
}

impl fmt::Display for ManifestAssetLoaderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "manifest read failed: {error}"),
            Self::Utf8(error) => write!(formatter, "manifest is not UTF-8: {error}"),
            Self::Yaml(error) => write!(formatter, "manifest YAML is invalid: {error}"),
        }
    }
}

impl Error for ManifestAssetLoaderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Utf8(error) => Some(error),
            Self::Yaml(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    use bevy::{
        asset::AssetPath,
        prelude::{Assets, State},
    };

    use super::*;
    use crate::{app_state::AppState, test_support::headless_title_app_with_asset_base};

    static NEXT_TEMPORARY_ID: AtomicU64 = AtomicU64::new(0);

    struct InventedAssetBase(PathBuf);

    impl InventedAssetBase {
        fn new(package_key: &str, manifest: &str) -> Self {
            let unique = NEXT_TEMPORARY_ID.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "rpg-s1-manifest-assets-{}-{unique}",
                std::process::id()
            ));
            let package = root.join("scenarios").join(package_key);
            fs::create_dir_all(&package).expect("invented package should be created");
            fs::write(package.join(SCENARIO_MANIFEST_PATH), manifest)
                .expect("invented manifest should be written");
            Self(root)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for InventedAssetBase {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("invented asset base should be removed");
        }
    }

    fn invented_manifest() -> &'static str {
        include_str!("../../../tests/fixtures/rusted-kingdoms-manifest-complete.yaml")
    }

    fn app_for(base: &InventedAssetBase, package_key: &str) -> App {
        headless_title_app_with_asset_base(
            AppState::Title,
            base.path().to_string_lossy().into_owned(),
            ScenarioRoot::try_for_package_key(package_key).expect("invented key should be valid"),
        )
    }

    fn update_until_terminal(app: &mut App) -> ActiveManifestStatus {
        for _ in 0..1_000 {
            app.update();
            let status = app.world().resource::<ActiveManifestLoad>().status();
            if status != ActiveManifestStatus::Loading {
                return status;
            }
            std::thread::yield_now();
        }
        panic!("manifest request did not finish");
    }

    #[test]
    fn loads_and_publishes_one_complete_typed_manifest_from_the_selected_package() {
        let base = InventedAssetBase::new("invented_campaign", invented_manifest());
        let mut app = app_for(&base, "invented_campaign");

        assert_eq!(
            app.world().resource::<ActiveManifestLoad>().status(),
            ActiveManifestStatus::Loading
        );
        assert_eq!(update_until_terminal(&mut app), ActiveManifestStatus::Ready);

        let world = app.world();
        let active = world.resource::<ActiveManifestLoad>();
        let manifests = world.resource::<Assets<Manifest>>();
        let selected_path = world
            .resource::<AssetServer>()
            .get_path(active.handle.id())
            .expect("selected handle should retain its logical AssetServer path");
        assert_eq!(
            selected_path.path().to_string_lossy(),
            "scenarios/invented_campaign/manifest.yaml"
        );
        let manifest = active
            .manifest(manifests)
            .expect("ready request should publish its typed manifest");
        assert_eq!(manifest.id, "my_rpg_story");
        assert_eq!(manifest.protagonist.id, "aric");
        assert!(active.failure().is_none());
    }

    #[test]
    fn malformed_manifest_fails_with_relative_context_and_publishes_nothing() {
        let malformed = invented_manifest().replacen(
            "  cursor_icon: assets/images/icons/arrow-head-right.webp\n",
            "",
            1,
        );
        let base = InventedAssetBase::new("broken_campaign", &malformed);
        let mut app = app_for(&base, "broken_campaign");

        assert_eq!(
            update_until_terminal(&mut app),
            ActiveManifestStatus::Failed
        );

        let world = app.world();
        let active = world.resource::<ActiveManifestLoad>();
        let manifests = world.resource::<Assets<Manifest>>();
        assert!(active.manifest(manifests).is_none());
        assert!(!manifests.contains(&active.handle));
        let failure = active.failure().expect("failure should retain diagnostics");
        assert_eq!(failure.package_key(), "broken_campaign");
        assert_eq!(failure.scenario_relative_path(), "manifest.yaml");
        assert!(failure.cause().contains("title.cursor_icon"));
        assert!(failure.cause().contains("line 8 column 3"));
        assert!(
            failure
                .to_string()
                .starts_with("broken_campaign:manifest.yaml:")
        );
        assert!(
            !failure
                .to_string()
                .contains(base.path().to_string_lossy().as_ref())
        );
        assert!(!failure.to_string().contains("assets/scenarios"));
    }

    #[test]
    fn removing_a_ready_asset_revokes_publication_instead_of_returning_stale_data() {
        let base = InventedAssetBase::new("unloaded_campaign", invented_manifest());
        let mut app = app_for(&base, "unloaded_campaign");
        assert_eq!(update_until_terminal(&mut app), ActiveManifestStatus::Ready);

        let handle = app.world().resource::<ActiveManifestLoad>().handle.clone();
        app.world_mut()
            .resource_mut::<Assets<Manifest>>()
            .remove(handle.id());
        app.update();

        let world = app.world();
        let active = world.resource::<ActiveManifestLoad>();
        assert_eq!(active.status(), ActiveManifestStatus::Loading);
        assert!(
            active
                .manifest(world.resource::<Assets<Manifest>>())
                .is_none()
        );
    }

    #[test]
    fn absent_default_package_is_nonfatal_and_title_state_remains_active() {
        let empty = InventedAssetBase::new("unrelated_campaign", invented_manifest());
        let mut app = headless_title_app_with_asset_base(
            AppState::Title,
            empty.path().to_string_lossy().into_owned(),
            ScenarioRoot::default(),
        );

        assert_eq!(
            update_until_terminal(&mut app),
            ActiveManifestStatus::Failed
        );
        assert_eq!(
            app.world().resource::<State<AppState>>().get(),
            &AppState::Title
        );
        let failure = app
            .world()
            .resource::<ActiveManifestLoad>()
            .failure()
            .expect("missing default package should be observable");
        assert_eq!(
            failure.to_string(),
            "rusted_kingdoms:manifest.yaml: manifest asset was not found"
        );
    }

    #[test]
    fn unclassified_bevy_failures_never_format_their_embedded_path() {
        let error = AssetLoadError::EmptyPath(AssetPath::from("private/host-shaped-detail"));

        assert_eq!(
            safe_asset_error_cause(&error),
            "manifest asset loading failed"
        );
    }
}
