//! Pure eligibility and route logic for the Ember Atlas.

use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fmt,
};

use bevy::{
    asset::{
        AssetApp, AssetLoader, AssetServer, Assets, Handle, LoadContext, LoadState, io::Reader,
    },
    prelude::*,
    reflect::TypePath,
};

use crate::{
    app_state::AppState,
    runtime_flags::RuntimeFlags,
    runtime_map::{RuntimeMapId, RuntimeMapState},
    scenario_inventory::ScenarioInventory,
    scenario_root::ScenarioRoot,
    scenario_spatial::Position,
    scenario_transport::{AtlasRegion, TransportCatalog, TravelAnchor},
    scenario_yaml,
    world_audio::WorldYamlAssetLoaderError,
};

pub(crate) struct TransportDomainPlugin;

impl Plugin for TransportDomainPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<TransportCatalog>()
            .init_asset_loader::<TransportCatalogAssetLoader>()
            .init_resource::<TransportDomain>()
            .init_resource::<TransportCatalogLoad>()
            .add_systems(OnEnter(AppState::World), begin_transport_load)
            .add_systems(
                Update,
                track_transport_load.run_if(in_state(AppState::World)),
            );
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum TravelMode {
    Sail,
    Fly,
    Warp,
    #[default]
    Closed,
}

/// How the Atlas was opened.
///
/// Scripted rewards may deliberately open Fly from the interior where the reward is granted. In
/// that one Atlas session the script is also permission to depart; ordinary Travel-key use keeps
/// the open-ground rule.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum TravelDeparture {
    #[default]
    Standard,
    Scripted,
}

impl TravelMode {
    pub(crate) const ALL: [Self; 3] = [Self::Sail, Self::Fly, Self::Warp];

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Sail => "SAIL",
            Self::Fly => "FLY",
            Self::Warp => "WARP",
            Self::Closed => "",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TravelRequest {
    pub(crate) mode: TravelMode,
    pub(crate) destination: TravelDestination,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TravelDestination {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) region_id: String,
    pub(crate) map_id: String,
    pub(crate) position: Position,
    pub(crate) availability: TravelAvailability,
    /// Sail berth ids from origin through destination; empty for Fly and Warp.
    pub(crate) path: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum TravelAvailability {
    Available,
    Locked(String),
}

impl TravelAvailability {
    pub(crate) fn reason(&self) -> Option<&str> {
        match self {
            Self::Available => None,
            Self::Locked(reason) => Some(reason),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum TransportStatus {
    #[default]
    Loading,
    Ready,
    Failed,
}

#[derive(Debug, Default, Resource)]
pub(crate) struct TransportDomain {
    status: TransportStatus,
    catalog: Option<TransportCatalog>,
    map_regions: BTreeMap<String, usize>,
    berth_edges: BTreeMap<String, BTreeSet<String>>,
    failure: Option<String>,
}

impl TransportDomain {
    pub(crate) fn try_from_catalog(
        catalog: TransportCatalog,
    ) -> Result<Self, TransportCatalogError> {
        validate_catalog(&catalog)?;
        let mut map_regions = BTreeMap::new();
        for (index, region) in catalog.regions.iter().enumerate() {
            for map in &region.maps {
                map_regions.insert(map.id.clone(), index);
            }
        }
        let mut berth_edges = BTreeMap::<String, BTreeSet<String>>::new();
        for berth in &catalog.sail_berths {
            berth_edges.entry(berth.id.clone()).or_default();
        }
        for edge in &catalog.sail_edges {
            let [a, b] = edge.between.as_slice() else {
                unreachable!("validated sail edge shape")
            };
            berth_edges.entry(a.clone()).or_default().insert(b.clone());
            berth_edges.entry(b.clone()).or_default().insert(a.clone());
        }
        Ok(Self {
            status: TransportStatus::Ready,
            catalog: Some(catalog),
            map_regions,
            berth_edges,
            failure: None,
        })
    }

    pub(crate) const fn status(&self) -> TransportStatus {
        self.status
    }
    pub(crate) fn catalog(&self) -> Option<&TransportCatalog> {
        self.catalog.as_ref()
    }

    pub(crate) fn region_for_map(&self, map_id: &str) -> Option<&AtlasRegion> {
        self.map_regions
            .get(map_id)
            .and_then(|index| self.catalog.as_ref()?.regions.get(*index))
    }

    pub(crate) fn is_outdoor(&self, map_id: &str) -> bool {
        self.region_for_map(map_id)
            .and_then(|region| region.maps.iter().find(|map| map.id == map_id))
            .is_some_and(|map| map.outdoor)
    }

    pub(crate) fn mode_unlocked(&self, mode: TravelMode, flags: &RuntimeFlags) -> bool {
        let Some(catalog) = &self.catalog else {
            return false;
        };
        match mode {
            TravelMode::Sail => flags.is_set(&catalog.sail_unlock_flag),
            TravelMode::Fly => flags.is_set(&catalog.fly_unlock_flag),
            TravelMode::Warp => true,
            TravelMode::Closed => false,
        }
    }

    pub(crate) fn any_convenience_unlocked(&self, flags: &RuntimeFlags) -> bool {
        self.mode_unlocked(TravelMode::Sail, flags) || self.mode_unlocked(TravelMode::Fly, flags)
    }

    pub(crate) fn availability(
        &self,
        mode: TravelMode,
        map: &RuntimeMapState,
        flags: &RuntimeFlags,
    ) -> TravelAvailability {
        self.availability_for(mode, map, flags, TravelDeparture::Standard)
    }

    pub(crate) fn availability_for(
        &self,
        mode: TravelMode,
        map: &RuntimeMapState,
        flags: &RuntimeFlags,
        departure: TravelDeparture,
    ) -> TravelAvailability {
        let Some(current) = map.current().map(RuntimeMapId::as_str) else {
            return TravelAvailability::Locked("Current location is unavailable.".to_owned());
        };
        if let Some(reason) = self.mode_block_reason(mode, current, flags, departure) {
            return TravelAvailability::Locked(reason);
        }
        if matches!(mode, TravelMode::Sail | TravelMode::Fly)
            && !self
                .destinations_for(mode, map, flags, departure)
                .iter()
                .any(|destination| destination.availability == TravelAvailability::Available)
        {
            return TravelAvailability::Locked(
                "No other visited destination is available.".to_owned(),
            );
        }
        TravelAvailability::Available
    }

    #[cfg(test)]
    pub(crate) fn destinations(
        &self,
        mode: TravelMode,
        map: &RuntimeMapState,
        flags: &RuntimeFlags,
    ) -> Vec<TravelDestination> {
        self.destinations_for(mode, map, flags, TravelDeparture::Standard)
    }

    pub(crate) fn destinations_for(
        &self,
        mode: TravelMode,
        map: &RuntimeMapState,
        flags: &RuntimeFlags,
        departure: TravelDeparture,
    ) -> Vec<TravelDestination> {
        let Some(catalog) = &self.catalog else {
            return Vec::new();
        };
        let current = map.current().map(RuntimeMapId::as_str);
        let anchors = match mode {
            TravelMode::Sail => &catalog.sail_berths,
            TravelMode::Fly => &catalog.fly_anchors,
            _ => return Vec::new(),
        };
        let origin = current.and_then(|id| self.origin_berth(id));
        let current_region =
            current.and_then(|id| self.region_for_map(id).map(|region| &region.id));
        let mode_reason = current
            .and_then(|id| self.mode_block_reason(mode, id, flags, departure))
            .or_else(|| {
                current
                    .is_none()
                    .then(|| "Current location is unavailable.".to_owned())
            });
        anchors
            .iter()
            .filter(|anchor| current_region != Some(&anchor.region))
            .filter(|anchor| {
                anchor
                    .reveal_flag
                    .as_ref()
                    .is_none_or(|flag| flags.is_set(flag))
            })
            .filter(|anchor| {
                RuntimeMapId::try_new(anchor.map.clone()).is_ok_and(|id| map.has_visited(&id))
            })
            .map(|anchor| {
                let path = if mode == TravelMode::Sail {
                    origin
                        .and_then(|from| self.sail_path(&from.id, &anchor.id))
                        .unwrap_or_default()
                } else {
                    Vec::new()
                };
                let availability = if let Some(reason) = &mode_reason {
                    TravelAvailability::Locked(reason.clone())
                } else if mode == TravelMode::Sail && path.is_empty() {
                    TravelAvailability::Locked("No charted waterway reaches this berth.".to_owned())
                } else {
                    TravelAvailability::Available
                };
                destination(anchor, availability, path)
            })
            .collect()
    }

    fn mode_block_reason(
        &self,
        mode: TravelMode,
        current: &str,
        flags: &RuntimeFlags,
        departure: TravelDeparture,
    ) -> Option<String> {
        if !self.mode_unlocked(mode, flags) {
            return Some(
                match mode {
                    TravelMode::Sail => "Sail has not been charted yet.",
                    TravelMode::Fly => "The Sky Crystal has not awakened.",
                    _ => "This travel mode is unavailable.",
                }
                .to_owned(),
            );
        }
        match mode {
            TravelMode::Sail if self.origin_berth(current).is_none() => {
                Some("Reach a charted berth.".to_owned())
            }
            TravelMode::Fly
                if !self.is_outdoor(current) && departure == TravelDeparture::Standard =>
            {
                Some(
                    if self
                        .region_for_map(current)
                        .is_some_and(|region| region.id == "hearth")
                    {
                        "The Hearth seals away the open sky."
                    } else {
                        "Reach open ground before taking flight."
                    }
                    .to_owned(),
                )
            }
            _ => None,
        }
    }

    pub(crate) fn sail_path(&self, origin: &str, destination: &str) -> Option<Vec<String>> {
        if origin == destination {
            return Some(vec![origin.to_owned()]);
        }
        let mut queue = VecDeque::from([origin.to_owned()]);
        let mut previous = BTreeMap::<String, String>::new();
        let mut seen = BTreeSet::from([origin.to_owned()]);
        while let Some(node) = queue.pop_front() {
            for next in self.berth_edges.get(&node).into_iter().flatten() {
                if !seen.insert(next.clone()) {
                    continue;
                }
                previous.insert(next.clone(), node.clone());
                if next == destination {
                    let mut path = vec![destination.to_owned()];
                    let mut cursor = destination;
                    while let Some(parent) = previous.get(cursor) {
                        path.push(parent.clone());
                        cursor = parent;
                    }
                    path.reverse();
                    return Some(path);
                }
                queue.push_back(next.clone());
            }
        }
        None
    }

    fn origin_berth(&self, map_id: &str) -> Option<&TravelAnchor> {
        let region = self.region_for_map(map_id)?;
        self.catalog
            .as_ref()?
            .sail_berths
            .iter()
            .find(|berth| berth.region == region.id)
    }
}

fn destination(
    anchor: &TravelAnchor,
    availability: TravelAvailability,
    path: Vec<String>,
) -> TravelDestination {
    TravelDestination {
        id: anchor.id.clone(),
        name: anchor.name.clone(),
        region_id: anchor.region.clone(),
        map_id: anchor.map.clone(),
        position: anchor.position,
        availability,
        path,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TransportCatalogError(String);

impl fmt::Display for TransportCatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for TransportCatalogError {}

fn validate_catalog(catalog: &TransportCatalog) -> Result<(), TransportCatalogError> {
    let mut region_ids = BTreeSet::new();
    let mut maps = BTreeSet::new();
    for region in &catalog.regions {
        if !region_ids.insert(region.id.as_str()) {
            return Err(TransportCatalogError(format!(
                "duplicate atlas region `{}`",
                region.id
            )));
        }
        for map in &region.maps {
            if !maps.insert(map.id.as_str()) {
                return Err(TransportCatalogError(format!(
                    "map `{}` belongs to multiple atlas regions",
                    map.id
                )));
            }
        }
    }
    let mut anchors = BTreeSet::new();
    for (kind, values) in [
        ("Fly anchor", &catalog.fly_anchors),
        ("Sail berth", &catalog.sail_berths),
    ] {
        for anchor in values {
            if !anchors.insert((kind, anchor.id.as_str())) {
                return Err(TransportCatalogError(format!(
                    "duplicate {kind} `{}`",
                    anchor.id
                )));
            }
            if !region_ids.contains(anchor.region.as_str()) {
                return Err(TransportCatalogError(format!(
                    "{kind} `{}` names unknown region `{}`",
                    anchor.id, anchor.region
                )));
            }
            let Some(region) = catalog.regions.iter().find(|r| r.id == anchor.region) else {
                unreachable!()
            };
            if !region.maps.iter().any(|map| map.id == anchor.map) {
                return Err(TransportCatalogError(format!(
                    "{kind} `{}` landing map is outside its region",
                    anchor.id
                )));
            }
        }
    }
    let berth_ids = catalog
        .sail_berths
        .iter()
        .map(|b| b.id.as_str())
        .collect::<BTreeSet<_>>();
    for edge in &catalog.sail_edges {
        let [a, b] = edge.between.as_slice() else {
            return Err(TransportCatalogError(
                "each sail edge must contain exactly two berth ids".to_owned(),
            ));
        };
        if a == b || !berth_ids.contains(a.as_str()) || !berth_ids.contains(b.as_str()) {
            return Err(TransportCatalogError(format!(
                "invalid sail edge `{a}` to `{b}`"
            )));
        }
    }
    Ok(())
}

#[derive(Debug, Default, Resource)]
struct TransportCatalogLoad(Option<Handle<TransportCatalog>>);

fn begin_transport_load(
    root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
    server: Res<AssetServer>,
    mut load: ResMut<TransportCatalogLoad>,
    mut domain: ResMut<TransportDomain>,
) {
    *domain = TransportDomain::default();
    load.0 = inventory
        .transport
        .as_ref()
        .map(|path| server.load(root.resolve(path)));
    if load.0.is_none() {
        domain.status = TransportStatus::Failed;
        domain.failure = Some("scenario manifest has no transport catalog".to_owned());
    }
}

fn track_transport_load(
    server: Res<AssetServer>,
    assets: Res<Assets<TransportCatalog>>,
    load: Res<TransportCatalogLoad>,
    mut domain: ResMut<TransportDomain>,
) {
    if domain.status != TransportStatus::Loading {
        return;
    }
    let Some(handle) = &load.0 else { return };
    match server.load_state(handle.id()) {
        LoadState::Loaded => {
            let Some(catalog) = assets.get(handle).cloned() else {
                return;
            };
            match TransportDomain::try_from_catalog(catalog) {
                Ok(ready) => *domain = ready,
                Err(error) => {
                    domain.status = TransportStatus::Failed;
                    domain.failure = Some(error.to_string());
                }
            }
        }
        LoadState::Failed(error) => {
            domain.status = TransportStatus::Failed;
            domain.failure = Some(format!("transport catalog failed to load: {error}"));
        }
        _ => {}
    }
}

#[derive(Default, TypePath)]
struct TransportCatalogAssetLoader;

impl AssetLoader for TransportCatalogAssetLoader {
    type Asset = TransportCatalog;
    type Settings = ();
    type Error = WorldYamlAssetLoaderError;
    async fn load(
        &self,
        reader: &mut dyn Reader,
        _: &Self::Settings,
        _: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .await
            .map_err(WorldYamlAssetLoaderError::Io)?;
        let text = std::str::from_utf8(&bytes).map_err(WorldYamlAssetLoaderError::Utf8)?;
        scenario_yaml::from_str(text).map_err(WorldYamlAssetLoaderError::Yaml)
    }
    fn extensions(&self) -> &[&str] {
        &["yaml", "yml"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{runtime_map::RuntimeMapState, scenario_spatial::CardinalDirection, scenario_yaml};

    fn domain() -> TransportDomain {
        let catalog: TransportCatalog =
            scenario_yaml::from_str(include_str!(scenario_file!("data/transport.yaml"))).unwrap();
        TransportDomain::try_from_catalog(catalog).unwrap()
    }
    fn map(current: &str, visited: &[&str]) -> RuntimeMapState {
        RuntimeMapState::try_from_saved(
            RuntimeMapId::try_new(current).unwrap(),
            Position::new(1, 1),
            CardinalDirection::Down,
            visited.iter().map(|id| RuntimeMapId::try_new(*id).unwrap()),
        )
        .unwrap()
    }

    #[test]
    fn sail_is_visited_only_and_finds_deterministic_multi_edge_paths() {
        let domain = domain();
        let flags = RuntimeFlags::from_bootstrap(["transport_sail_unlocked"]);
        let state = map(
            "town_01_ardel_inn_01",
            &["town_02_millhaven", "town_05_ashenveil"],
        );
        let destinations = domain.destinations(TravelMode::Sail, &state, &flags);
        assert_eq!(
            destinations
                .iter()
                .map(|d| d.id.as_str())
                .collect::<Vec<_>>(),
            ["millhaven_lock", "ashenveil_quay"]
        );
        assert_eq!(
            destinations[1].path,
            [
                "ardel_landing",
                "millhaven_lock",
                "harborgate",
                "ruinwatch_ferry",
                "ashenveil_quay"
            ]
        );
    }

    #[test]
    fn sail_rejects_non_berth_regions_and_fly_requires_outdoors() {
        let domain = domain();
        let flags =
            RuntimeFlags::from_bootstrap(["transport_sail_unlocked", "transport_fly_unlocked"]);
        let cave = map("zone_07_sunken_cave", &["town_01_ardel"]);
        assert_eq!(
            domain
                .availability(TravelMode::Sail, &cave, &flags)
                .reason(),
            Some("Reach a charted berth.")
        );
        assert_eq!(
            domain.availability(TravelMode::Fly, &cave, &flags).reason(),
            Some("Reach open ground before taking flight.")
        );
        let forest = map("zone_08_corrupted_forest", &["town_01_ardel"]);
        assert_eq!(
            domain.availability(TravelMode::Fly, &forest, &flags),
            TravelAvailability::Available
        );
    }

    #[test]
    fn a_scripted_fly_reward_can_depart_from_its_interior() {
        let domain = domain();
        let flags = RuntimeFlags::from_bootstrap(["transport_fly_unlocked"]);
        let vault = map("town_04_frostholm_vault", &["town_01_ardel"]);

        assert_eq!(
            domain
                .availability(TravelMode::Fly, &vault, &flags)
                .reason(),
            Some("Reach open ground before taking flight.")
        );
        assert_eq!(
            domain.availability_for(TravelMode::Fly, &vault, &flags, TravelDeparture::Scripted,),
            TravelAvailability::Available
        );
        assert!(
            domain
                .destinations_for(TravelMode::Fly, &vault, &flags, TravelDeparture::Scripted,)
                .iter()
                .all(|destination| destination.availability == TravelAvailability::Available)
        );
    }

    #[test]
    fn current_and_unvisited_destinations_are_excluded_in_authored_order() {
        let domain = domain();
        let flags = RuntimeFlags::from_bootstrap(["transport_fly_unlocked"]);
        let state = map(
            "town_01_ardel",
            &[
                "town_01_ardel",
                "town_04_frostholm",
                "zone_09_volcanic_region",
            ],
        );
        assert_eq!(
            domain
                .destinations(TravelMode::Fly, &state, &flags)
                .iter()
                .map(|d| d.id.as_str())
                .collect::<Vec<_>>(),
            ["frostholm", "volcanic_region"]
        );
    }

    #[test]
    fn an_interior_excludes_its_own_regional_anchor_and_locked_rows_explain_why() {
        let domain = domain();
        let state = map(
            "town_01_ardel_inn_01",
            &["town_01_ardel", "town_02_millhaven"],
        );
        let locked = domain.destinations(TravelMode::Sail, &state, &RuntimeFlags::default());
        assert_eq!(
            locked.iter().map(|d| d.id.as_str()).collect::<Vec<_>>(),
            ["millhaven_lock"]
        );
        assert_eq!(
            locked[0].availability.reason(),
            Some("Sail has not been charted yet.")
        );

        let unlocked = RuntimeFlags::from_bootstrap(["transport_sail_unlocked"]);
        assert_eq!(
            domain
                .destinations(TravelMode::Sail, &state, &unlocked)
                .iter()
                .map(|d| d.id.as_str())
                .collect::<Vec<_>>(),
            ["millhaven_lock"]
        );
    }

    #[test]
    fn fly_needs_another_visited_regional_anchor() {
        let domain = domain();
        let flags = RuntimeFlags::from_bootstrap(["transport_fly_unlocked"]);
        let state = map("zone_01_starting_forest", &["town_01_ardel"]);
        assert_eq!(
            domain
                .availability(TravelMode::Fly, &state, &flags)
                .reason(),
            Some("No other visited destination is available.")
        );
        assert!(
            domain
                .destinations(TravelMode::Fly, &state, &flags)
                .is_empty()
        );
    }
}
