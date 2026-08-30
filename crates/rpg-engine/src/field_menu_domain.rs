//! Shared M6 field-menu catalogs and mutation services.

use std::{collections::BTreeMap, error::Error, fmt};

use bevy::{
    asset::{
        AssetApp, AssetLoader, AssetServer, Assets, Handle, LoadContext, LoadState, io::Reader,
    },
    prelude::*,
    reflect::TypePath,
};

use crate::{
    app_state::AppState,
    game_state::GameState,
    runtime_map::RuntimeMapState,
    runtime_member::{EquipmentSlot, RuntimeMember},
    scenario_class::{
        Ability, AbilityKind, AbilityTarget, ClassDefinition, ClassEquipmentSlots, HealingMethod,
        UtilityAbility,
    },
    scenario_inventory::ScenarioInventory,
    scenario_item::{
        BodyStats, FieldUseCatalogFile, FieldUseDefinition, FullRecoveryTarget, HelmetStats,
        ItemCatalogFile, ItemDefinition, ShieldStats, WeaponStats,
    },
    scenario_map::MapMetadata,
    scenario_quest::{QuestCatalogFile, QuestDefinition},
    scenario_recipe::{RecipeCatalogFile, RecipeDefinition},
    scenario_root::ScenarioRoot,
    scenario_spatial::Position,
    scenario_yaml::{self, ScenarioYamlError},
    tmx_ground_asset::TmxGroundAsset,
    world_transition::{RuntimePortal, runtime_portals},
};

#[cfg(test)]
use crate::scenario_path::ScenarioRelativePath;

pub(crate) struct FieldMenuDomainPlugin;

impl Plugin for FieldMenuDomainPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<ItemCatalogFile>()
            .init_asset::<FieldUseCatalogFile>()
            .init_asset::<ClassDefinition>()
            .init_asset::<RecipeCatalogFile>()
            .init_asset::<QuestCatalogFile>()
            .init_asset_loader::<ItemCatalogAssetLoader>()
            .init_asset_loader::<FieldUseCatalogAssetLoader>()
            .init_asset_loader::<ClassDefinitionAssetLoader>()
            .init_asset_loader::<RecipeCatalogAssetLoader>()
            .init_asset_loader::<QuestCatalogAssetLoader>()
            .init_resource::<FieldMenuCatalog>()
            .init_resource::<FieldMenuCatalogLoad>()
            .add_systems(OnEnter(AppState::World), begin_catalog_load)
            .add_systems(Update, track_catalog_load.run_if(in_state(AppState::World)));
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum CatalogStatus {
    #[default]
    Loading,
    Ready,
    Failed,
}

/// Immutable lookup built from every current source-authored M6 class and item file.
#[derive(Debug, Default, Resource)]
pub(crate) struct FieldMenuCatalog {
    status: CatalogStatus,
    items: BTreeMap<String, ItemDefinition>,
    field_uses: BTreeMap<String, FieldUseDefinition>,
    classes: BTreeMap<String, ClassDefinition>,
    item_order: Vec<String>,
    warp_destinations: Vec<WarpDestination>,
    maps: BTreeMap<String, MapMetadata>,
    recipes: Vec<RecipeDefinition>,
    quests: Vec<QuestDefinition>,
    failure: Option<String>,
}

impl FieldMenuCatalog {
    pub(crate) const fn status(&self) -> CatalogStatus {
        self.status
    }
    pub(crate) fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }
    pub(crate) fn item(&self, id: &str) -> Option<&ItemDefinition> {
        self.items.get(id)
    }
    pub(crate) fn ordered_items(&self) -> impl Iterator<Item = &ItemDefinition> {
        self.item_order.iter().filter_map(|id| self.items.get(id))
    }
    pub(crate) fn field_use(&self, id: &str) -> Option<&FieldUseDefinition> {
        self.field_uses.get(id)
    }
    pub(crate) fn class(&self, id: &str) -> Option<&ClassDefinition> {
        self.classes.get(id)
    }
    pub(crate) fn map(&self, id: &str) -> Option<&MapMetadata> {
        self.maps.get(id)
    }
    pub(crate) fn recipes(&self) -> &[RecipeDefinition] {
        &self.recipes
    }
    pub(crate) fn quests(&self) -> &[QuestDefinition] {
        &self.quests
    }

    pub(crate) fn unlocked_abilities(
        &self,
        class_id: &str,
        level: u32,
        flags: &crate::runtime_flags::RuntimeFlags,
    ) -> Vec<&Ability> {
        self.class(class_id)
            .into_iter()
            .flat_map(|class| &class.abilities)
            .filter(|ability| {
                ability.unlock_level.get() <= level
                    && ability
                        .unlock_flag
                        .as_ref()
                        .is_none_or(|flag| flags.is_set(flag))
            })
            .collect()
    }

    /// Builds the battle-facing subset of the production catalog for the encounter sweep.
    /// Battle construction reads item equipment stats and class abilities, but does not need
    /// field-use, warp, recipe, quest, or map services.
    pub(crate) fn for_encounter_sweep(
        item_files: impl IntoIterator<Item = ItemCatalogFile>,
        class_files: impl IntoIterator<Item = ClassDefinition>,
    ) -> Self {
        let mut items = BTreeMap::new();
        let mut item_order = Vec::new();
        for file in item_files {
            for item in file.entries() {
                item_order.push(item.id().to_owned());
                items.insert(item.id().to_owned(), item.clone());
            }
        }
        let classes = class_files
            .into_iter()
            .map(|class| (class.class_id.clone(), class))
            .collect();
        Self {
            status: CatalogStatus::Ready,
            items,
            classes,
            item_order,
            ..Self::default()
        }
    }

    pub(crate) fn eligible_warp_destinations(
        &self,
        map: &RuntimeMapState,
    ) -> Vec<&WarpDestination> {
        let current = map.current().map(|id| id.as_str());
        self.warp_destinations
            .iter()
            .filter(|destination| {
                current != Some(destination.map_id.as_str())
                    && crate::runtime_map::RuntimeMapId::try_new(destination.map_id.clone())
                        .is_ok_and(|id| map.has_visited(&id))
            })
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn production_class_fixture() -> Self {
        let classes = [
            include_str!("../../../assets/scenarios/rusted_kingdoms/data/classes/cleric.yaml"),
            include_str!("../../../assets/scenarios/rusted_kingdoms/data/classes/hero.yaml"),
            include_str!("../../../assets/scenarios/rusted_kingdoms/data/classes/rogue.yaml"),
            include_str!("../../../assets/scenarios/rusted_kingdoms/data/classes/sorcerer.yaml"),
            include_str!("../../../assets/scenarios/rusted_kingdoms/data/classes/warrior.yaml"),
        ]
        .into_iter()
        .map(|document| {
            let class: ClassDefinition = crate::scenario_yaml::from_str(document).unwrap();
            (class.class_id.clone(), class)
        })
        .collect();
        Self {
            status: CatalogStatus::Ready,
            classes,
            ..default()
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WarpDestination {
    pub(crate) map_id: String,
    pub(crate) name: String,
    pub(crate) position: Position,
    town: bool,
    order: u32,
}

/// A visited, warp-reachable map (top-level, with a known incoming-portal landing tile) whose map
/// data has no `warp_order`. Mirrors `engine/world/warp_logic.py::_map_order`: the teleport list
/// ordering is data-driven, so this is a scenario-authoring bug surfaced as a load failure rather
/// than silently dropping the destination or defaulting its position in the list.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WarpOrderError {
    map_id: String,
}

impl fmt::Display for WarpOrderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "warp destination `{}` has a known incoming-portal landing tile but no `warp_order` \
             in its map data; add an integer ordering it within its group (towns / world map), \
             e.g. `warp_order: 40`",
            self.map_id
        )
    }
}

impl Error for WarpOrderError {}

/// Resolves the visited-map teleport destination list from every loaded map's metadata and
/// portals, matching `engine/world/warp_logic.py::warp_destinations` (minus the caller's
/// visited/current filter, applied later by [`FieldMenuCatalog::eligible_warp_destinations`]):
///
/// - A destination is a `source` that is not a *submap* of another `source` — one whose id is
///   some other source's id followed by `_` (interiors like `..._shop_01`, and numbered segments
///   like `..._02`, both use this convention; see `warp_logic._is_submap`). Note this is checked
///   only against the *other loaded sources*, matching Python's check against every map id that
///   actually has its own TMX — a segment whose bare parent id has no TMX of its own (e.g. the
///   pinned scenario's `zone_05_mountain_foothills_01/02/03`, which have no
///   `zone_05_mountain_foothills.tmx`) is therefore *not* excluded as a submap.
/// - Its landing tile comes from the scenario-wide incoming-portal index: every portal in every
///   `source` targeting it, preferring a non-submap source, tied broken by source id
///   (`warp_logic.build_landing_index`). A destination with no incoming portal at all is dropped
///   (nowhere to land).
/// - It is categorized `town` when its map data has an `inn` or `shop` block, `world` otherwise.
/// - It must declare `warp_order`; see [`WarpOrderError`].
///
/// Grouped towns-then-world, ordered by `warp_order` within each group.
fn compute_warp_destinations(
    sources: &[(&str, &MapMetadata, &[RuntimePortal])],
) -> Result<Vec<WarpDestination>, WarpOrderError> {
    let map_ids: Vec<&str> = sources
        .iter()
        .map(|(stem, metadata, _)| metadata.effective_id(stem))
        .collect();

    let mut candidates: BTreeMap<&str, Vec<(bool, &str, Position)>> = BTreeMap::new();
    for (stem, metadata, portals) in sources {
        let source_id = metadata.effective_id(stem);
        for portal in *portals {
            let target = portal.target_map().as_str();
            let is_sub = source_id.starts_with(&format!("{target}_"));
            candidates.entry(target).or_default().push((
                is_sub,
                source_id,
                portal.target_position(),
            ));
        }
    }
    let mut landing: BTreeMap<&str, Position> = BTreeMap::new();
    for (destination, mut incoming) in candidates {
        incoming.sort_by_key(|(is_sub, source_id, _)| (*is_sub, *source_id));
        if let Some((_, _, position)) = incoming.into_iter().next() {
            landing.insert(destination, position);
        }
    }

    let mut towns = Vec::new();
    let mut world = Vec::new();
    for (stem, metadata, _) in sources {
        let map_id = metadata.effective_id(stem);
        let is_submap = map_ids
            .iter()
            .any(|&other| other != map_id && map_id.starts_with(&format!("{other}_")));
        if is_submap {
            continue;
        }
        let Some(&position) = landing.get(map_id) else {
            continue;
        };
        let Some(order) = metadata.warp_order else {
            return Err(WarpOrderError {
                map_id: map_id.to_owned(),
            });
        };
        let town = metadata.inn.is_some() || metadata.shop.is_some();
        let destination = WarpDestination {
            map_id: map_id.to_owned(),
            name: metadata.name.clone(),
            position,
            town,
            order,
        };
        (if town { &mut towns } else { &mut world }).push(destination);
    }
    towns.sort_by_key(|destination: &WarpDestination| {
        (destination.order, destination.map_id.clone())
    });
    world.sort_by_key(|destination: &WarpDestination| {
        (destination.order, destination.map_id.clone())
    });
    towns.extend(world);
    Ok(towns)
}

#[derive(Debug, Default, Resource)]
struct FieldMenuCatalogLoad {
    items: Vec<(String, Handle<ItemCatalogFile>)>,
    field_use: Option<Handle<FieldUseCatalogFile>>,
    classes: Vec<(String, Handle<ClassDefinition>)>,
    maps: Vec<(String, Handle<MapMetadata>, Handle<TmxGroundAsset>)>,
    service_maps: Vec<(String, Handle<MapMetadata>)>,
    recipes: Vec<(String, Handle<RecipeCatalogFile>)>,
    quests: Option<Handle<QuestCatalogFile>>,
}

fn begin_catalog_load(
    root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
    asset_server: Res<AssetServer>,
    mut load: ResMut<FieldMenuCatalogLoad>,
    mut catalog: ResMut<FieldMenuCatalog>,
) {
    if catalog.status == CatalogStatus::Ready {
        return;
    }
    catalog.status = CatalogStatus::Loading;
    catalog.failure = None;
    if let Some(failure) = inventory.failure.as_ref() {
        catalog.status = CatalogStatus::Failed;
        catalog.failure = Some(failure.clone());
        return;
    }
    if inventory.field_use.is_none() {
        catalog.status = CatalogStatus::Failed;
        catalog.failure = Some("scenario has no field-use catalog".to_owned());
        return;
    }
    load.items = inventory
        .item_catalogs
        .iter()
        .map(|path| {
            (
                path.as_str().to_owned(),
                asset_server.load(root.resolve(path)),
            )
        })
        .collect();
    load.field_use = inventory
        .field_use
        .as_ref()
        .map(|path| asset_server.load(root.resolve(path)));
    load.classes = inventory
        .classes
        .iter()
        .map(|path| {
            (
                path.as_str().to_owned(),
                asset_server.load(root.resolve(path)),
            )
        })
        .collect();
    load.maps = inventory
        .maps
        .iter()
        .filter_map(|(stem, metadata, tmx)| {
            tmx.as_ref().map(|tmx| {
                (
                    stem.clone(),
                    asset_server.load(root.resolve(metadata)),
                    asset_server.load(root.resolve(tmx)),
                )
            })
        })
        .collect();
    load.service_maps = inventory
        .maps
        .iter()
        .filter(|(_, _, tmx)| tmx.is_none())
        .map(|(stem, metadata, _)| (stem.clone(), asset_server.load(root.resolve(metadata))))
        .collect();
    load.recipes = inventory
        .recipes
        .iter()
        .map(|path| {
            (
                path.as_str().to_owned(),
                asset_server.load(root.resolve(path)),
            )
        })
        .collect();
    load.quests = inventory
        .quests
        .as_ref()
        .map(|path| asset_server.load(root.resolve(path)));
}

#[expect(
    clippy::too_many_arguments,
    reason = "catalog publication checks each independently typed asset collection"
)]
fn track_catalog_load(
    asset_server: Res<AssetServer>,
    item_assets: Res<Assets<ItemCatalogFile>>,
    field_assets: Res<Assets<FieldUseCatalogFile>>,
    class_assets: Res<Assets<ClassDefinition>>,
    map_assets: Res<Assets<MapMetadata>>,
    tmx_assets: Res<Assets<TmxGroundAsset>>,
    recipe_assets: Res<Assets<RecipeCatalogFile>>,
    quest_assets: Res<Assets<QuestCatalogFile>>,
    load: Res<FieldMenuCatalogLoad>,
    mut catalog: ResMut<FieldMenuCatalog>,
) {
    if catalog.status != CatalogStatus::Loading || load.field_use.is_none() || load.quests.is_none()
    {
        return;
    }
    for (path, handle) in &load.items {
        if let LoadState::Failed(error) = asset_server.load_state(handle.id()) {
            catalog.status = CatalogStatus::Failed;
            catalog.failure = Some(format!("{path}: {error}"));
            return;
        }
    }
    for (path, handle) in &load.classes {
        if let LoadState::Failed(error) = asset_server.load_state(handle.id()) {
            catalog.status = CatalogStatus::Failed;
            catalog.failure = Some(format!("{path}: {error}"));
            return;
        }
    }
    for (stem, metadata, tmx) in &load.maps {
        if let LoadState::Failed(error) = asset_server.load_state(metadata.id()) {
            catalog.status = CatalogStatus::Failed;
            catalog.failure = Some(format!("map metadata for `{stem}`: {error}"));
            return;
        }
        if let LoadState::Failed(error) = asset_server.load_state(tmx.id()) {
            catalog.status = CatalogStatus::Failed;
            catalog.failure = Some(format!("map TMX for `{stem}`: {error}"));
            return;
        }
    }
    for (stem, metadata) in &load.service_maps {
        if let LoadState::Failed(error) = asset_server.load_state(metadata.id()) {
            catalog.status = CatalogStatus::Failed;
            catalog.failure = Some(format!("map metadata for `{stem}`: {error}"));
            return;
        }
    }
    let field_handle = load.field_use.as_ref().expect("checked above");
    if let LoadState::Failed(error) = asset_server.load_state(field_handle.id()) {
        catalog.status = CatalogStatus::Failed;
        catalog.failure = Some(format!("field-use catalog: {error}"));
        return;
    }
    for (path, handle) in &load.recipes {
        if let LoadState::Failed(error) = asset_server.load_state(handle.id()) {
            catalog.status = CatalogStatus::Failed;
            catalog.failure = Some(format!("{path}: {error}"));
            return;
        }
    }
    let quest_handle = load.quests.as_ref().expect("checked above");
    if let LoadState::Failed(error) = asset_server.load_state(quest_handle.id()) {
        catalog.status = CatalogStatus::Failed;
        catalog.failure = Some(format!("quest catalog: {error}"));
        return;
    }
    if load
        .items
        .iter()
        .any(|(_, handle)| item_assets.get(handle).is_none())
        || load
            .classes
            .iter()
            .any(|(_, handle)| class_assets.get(handle).is_none())
        || field_assets.get(field_handle).is_none()
        || load.maps.iter().any(|(_, metadata, tmx)| {
            map_assets.get(metadata).is_none() || tmx_assets.get(tmx).is_none()
        })
        || load
            .service_maps
            .iter()
            .any(|(_, metadata)| map_assets.get(metadata).is_none())
        || load
            .recipes
            .iter()
            .any(|(_, handle)| recipe_assets.get(handle).is_none())
        || quest_assets.get(quest_handle).is_none()
    {
        return;
    }

    let mut items = BTreeMap::new();
    let mut item_order = Vec::new();
    for (_, handle) in &load.items {
        for item in item_assets.get(handle).expect("all checked").entries() {
            item_order.push(item.id().to_owned());
            items.insert(item.id().to_owned(), item.clone());
        }
    }
    let field_uses = field_assets
        .get(field_handle)
        .expect("checked")
        .entries()
        .iter()
        .map(|entry| (entry.id().to_owned(), entry.clone()))
        .collect();
    let classes = load
        .classes
        .iter()
        .map(|(_, handle)| {
            let class = class_assets.get(handle).expect("all checked").clone();
            (class.class_id.clone(), class)
        })
        .collect();
    let mut maps = BTreeMap::new();
    let mut warp_sources = Vec::with_capacity(load.maps.len());
    for (stem, metadata_handle, tmx_handle) in &load.maps {
        let metadata = map_assets.get(metadata_handle).expect("all checked");
        maps.insert(metadata.effective_id(stem).to_owned(), metadata.clone());
        let tmx = tmx_assets.get(tmx_handle).expect("all checked");
        // A malformed `portals` layer is a TMX-authoring concern outside this catalog's scope;
        // treat it as "no outgoing portals from this source" rather than failing catalog load.
        let portals = runtime_portals(tmx.document()).unwrap_or_default();
        warp_sources.push((stem.as_str(), metadata, portals));
    }
    for (stem, metadata_handle) in &load.service_maps {
        let metadata = map_assets.get(metadata_handle).expect("all checked");
        maps.insert(metadata.effective_id(stem).to_owned(), metadata.clone());
    }
    let warp_sources_ref: Vec<(&str, &MapMetadata, &[RuntimePortal])> = warp_sources
        .iter()
        .map(|(stem, metadata, portals)| (*stem, *metadata, portals.as_slice()))
        .collect();
    let warp_destinations = match compute_warp_destinations(&warp_sources_ref) {
        Ok(destinations) => destinations,
        Err(error) => {
            catalog.status = CatalogStatus::Failed;
            catalog.failure = Some(error.to_string());
            return;
        }
    };
    catalog.items = items;
    catalog.item_order = item_order;
    catalog.field_uses = field_uses;
    catalog.classes = classes;
    catalog.warp_destinations = warp_destinations;
    catalog.maps = maps;
    catalog.recipes = load
        .recipes
        .iter()
        .flat_map(|(_, handle)| {
            recipe_assets
                .get(handle)
                .expect("checked")
                .entries()
                .iter()
                .cloned()
        })
        .collect();
    catalog.quests = quest_assets
        .get(quest_handle)
        .expect("checked")
        .entries()
        .to_vec();
    catalog.failure = None;
    catalog.status = CatalogStatus::Ready;
}

macro_rules! yaml_loader {
    ($loader:ident, $asset:ty) => {
        #[derive(Default, TypePath)]
        struct $loader;
        impl AssetLoader for $loader {
            type Asset = $asset;
            type Settings = ();
            type Error = CatalogAssetError;
            async fn load(
                &self,
                reader: &mut dyn Reader,
                _: &(),
                _: &mut LoadContext<'_>,
            ) -> Result<Self::Asset, Self::Error> {
                let mut bytes = Vec::new();
                reader
                    .read_to_end(&mut bytes)
                    .await
                    .map_err(CatalogAssetError::Io)?;
                let document = std::str::from_utf8(&bytes).map_err(CatalogAssetError::Utf8)?;
                scenario_yaml::from_str(document).map_err(CatalogAssetError::Yaml)
            }
            fn extensions(&self) -> &[&str] {
                &["yaml", "yml"]
            }
        }
    };
}
yaml_loader!(ItemCatalogAssetLoader, ItemCatalogFile);
yaml_loader!(FieldUseCatalogAssetLoader, FieldUseCatalogFile);
yaml_loader!(ClassDefinitionAssetLoader, ClassDefinition);
yaml_loader!(RecipeCatalogAssetLoader, RecipeCatalogFile);
yaml_loader!(QuestCatalogAssetLoader, QuestCatalogFile);

#[derive(Debug)]
enum CatalogAssetError {
    Io(std::io::Error),
    Utf8(std::str::Utf8Error),
    Yaml(ScenarioYamlError),
}
impl fmt::Display for CatalogAssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "read failed: {error}"),
            Self::Utf8(error) => write!(f, "asset is not UTF-8: {error}"),
            Self::Yaml(error) => write!(f, "YAML is invalid: {error}"),
        }
    }
}
impl Error for CatalogAssetError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InventoryTab {
    All,
    New,
    Recovery,
    Status,
    Battle,
    Material,
    Core,
    Key,
}

impl InventoryTab {
    pub(crate) const ALL: [Self; 8] = [
        Self::All,
        Self::New,
        Self::Recovery,
        Self::Status,
        Self::Battle,
        Self::Material,
        Self::Core,
        Self::Key,
    ];
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::New => "New",
            Self::Recovery => "Recovery",
            Self::Status => "Status",
            Self::Battle => "Battle",
            Self::Material => "Material",
            Self::Core => "Core",
            Self::Key => "Key",
        }
    }
}

/// The curatorial tags the tag editor offers on every item.
///
/// Ports `item_logic.EDITABLE_SYSTEM_TAGS`.
pub(crate) const EDITABLE_SYSTEM_TAGS: [&str; 3] = ["rare", "sell_soon", "favorite"];

/// Longest custom tag the editor accepts, matching `item_logic.CUSTOM_TAG_MAX_LEN`.
pub(crate) const CUSTOM_TAG_MAX_LENGTH: usize = 16;

/// Type-driven tags the catalog owns; the editor shows but never toggles them.
const TYPE_SYSTEM_TAGS: [&str; 13] = [
    "consumable",
    "material",
    "key",
    "magic_core",
    "equipment",
    "weapon",
    "shield",
    "helmet",
    "body",
    "accessory",
    "battle",
    "status",
    "recovery",
];

/// Ports `item_logic.is_system_tag`: engine-managed tags the player does not author.
pub(crate) fn is_system_tag(tag: &str) -> bool {
    EDITABLE_SYSTEM_TAGS.contains(&tag) || TYPE_SYSTEM_TAGS.contains(&tag)
}

/// The player-authored tags on one stack, sorted for a stable editor order.
pub(crate) fn custom_tags<'a>(game: &'a GameState, item_id: &str) -> Vec<&'a str> {
    let mut tags = game
        .repository()
        .item_tags(item_id)
        .filter(|tag| !is_system_tag(tag))
        .collect::<Vec<_>>();
    tags.sort_unstable();
    tags
}

/// Ports `item_logic.normalize_custom_tag`: lowercase, spaces to underscores, bounded charset.
///
/// Returns `None` for anything the editor must reject rather than silently repair.
pub(crate) fn normalize_custom_tag(raw: &str) -> Option<String> {
    let normalized = raw.trim().to_lowercase().replace(' ', "_");
    if normalized.is_empty() || normalized.chars().count() > CUSTOM_TAG_MAX_LENGTH {
        return None;
    }
    if !normalized.chars().all(|character| {
        character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
    }) {
        return None;
    }
    Some(normalized)
}

pub(crate) fn inventory_ids<'a>(
    game: &'a GameState,
    catalog: &'a FieldMenuCatalog,
    tab: InventoryTab,
) -> Vec<&'a str> {
    let mut ids = game
        .repository()
        .item_counts()
        .filter(|(id, _)| !game.repository().is_hidden(id))
        .filter_map(|(id, _)| {
            catalog
                .item(id)
                .filter(|item| item_matches_tab(item, game.repository().is_new_item(id), tab))
                .map(|_| id)
        })
        .collect::<Vec<_>>();
    if tab == InventoryTab::Core {
        ids.sort_by_key(|id| {
            catalog
                .item_order
                .iter()
                .position(|candidate| candidate == id)
                .unwrap_or(usize::MAX)
        });
    } else {
        ids.sort_unstable();
    }
    ids
}

fn item_matches_tab(item: &ItemDefinition, is_new: bool, tab: InventoryTab) -> bool {
    match tab {
        InventoryTab::All => true,
        InventoryTab::New => is_new,
        InventoryTab::Recovery => {
            matches!(item, ItemDefinition::Consumable(value) if !matches!(value.effect, crate::scenario_item::ConsumableEffect::Cure(_) | crate::scenario_item::ConsumableEffect::Throw(_)))
        }
        InventoryTab::Status => {
            matches!(item, ItemDefinition::Consumable(value) if matches!(value.effect, crate::scenario_item::ConsumableEffect::Cure(_)))
        }
        InventoryTab::Battle => {
            matches!(item, ItemDefinition::Consumable(value) if matches!(value.effect, crate::scenario_item::ConsumableEffect::Throw(_)))
        }
        InventoryTab::Material => matches!(item, ItemDefinition::Material(_)),
        InventoryTab::Core => matches!(item, ItemDefinition::MagicCore(_)),
        InventoryTab::Key => matches!(item, ItemDefinition::Key(_)),
    }
}

pub(crate) fn item_name(item: &ItemDefinition) -> &str {
    match item {
        ItemDefinition::Consumable(v) => &v.name,
        ItemDefinition::Material(v) => &v.name,
        ItemDefinition::Key(v) => &v.name,
        ItemDefinition::MagicCore(v) => &v.name,
        ItemDefinition::Weapon(v) => &v.name,
        ItemDefinition::Shield(v) => &v.name,
        ItemDefinition::Helmet(v) => &v.name,
        ItemDefinition::Body(v) => &v.name,
        ItemDefinition::Accessory(v) => &v.name,
    }
}

pub(crate) fn item_description(item: &ItemDefinition) -> &str {
    match item {
        ItemDefinition::Consumable(v) => &v.description,
        ItemDefinition::Material(v) => &v.description,
        ItemDefinition::Key(v) => &v.description,
        ItemDefinition::MagicCore(v) => &v.description,
        ItemDefinition::Weapon(v) => &v.description,
        ItemDefinition::Shield(v) => &v.description,
        ItemDefinition::Helmet(v) => &v.description,
        ItemDefinition::Body(v) => &v.description,
        ItemDefinition::Accessory(v) => &v.description,
    }
}

pub(crate) fn discard_item(
    game: &mut GameState,
    catalog: &FieldMenuCatalog,
    item_id: &str,
    quantity: u32,
) -> Result<(), MenuMutationError> {
    let item = catalog
        .item(item_id)
        .ok_or_else(|| MenuMutationError::UnknownItem(item_id.to_owned()))?;
    if matches!(item, ItemDefinition::Key(_)) {
        return Err(MenuMutationError::KeyItem);
    }
    if game.repository().is_locked(item_id) {
        return Err(MenuMutationError::LockedItem);
    }
    game.repository_mut()
        .remove_item(item_id, quantity)
        .map_err(|error| MenuMutationError::Repository(error.to_string()))
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct DerivedStats {
    pub strength: i32,
    pub dexterity: i32,
    pub constitution: i32,
    pub intelligence: i32,
}

pub(crate) fn derived_stats(member: &RuntimeMember, catalog: &FieldMenuCatalog) -> DerivedStats {
    let mut totals = DerivedStats {
        strength: member.stats().strength() as i32,
        dexterity: member.stats().dexterity() as i32,
        constitution: member.stats().constitution() as i32,
        intelligence: member.stats().intelligence() as i32,
    };
    for slot in EquipmentSlot::ALL {
        if let Some(id) = member.equipment().get(slot).and_then(|id| catalog.item(id)) {
            add_item_stats(&mut totals, id);
        }
    }
    totals
}

pub(crate) fn preview_stats(
    member: &RuntimeMember,
    catalog: &FieldMenuCatalog,
    slot: EquipmentSlot,
    item_id: Option<&str>,
) -> DerivedStats {
    let mut totals = DerivedStats {
        strength: member.stats().strength() as i32,
        dexterity: member.stats().dexterity() as i32,
        constitution: member.stats().constitution() as i32,
        intelligence: member.stats().intelligence() as i32,
    };
    for active_slot in EquipmentSlot::ALL {
        let active = if active_slot == slot {
            item_id
        } else {
            member.equipment().get(active_slot)
        };
        if let Some(item) = active.and_then(|id| catalog.item(id)) {
            add_item_stats(&mut totals, item);
        }
    }
    totals
}

fn add_item_stats(totals: &mut DerivedStats, item: &ItemDefinition) {
    let mut add =
        |strength: Option<i32>, dex: Option<i32>, con: Option<i32>, intelligence: Option<i32>| {
            totals.strength += strength.unwrap_or(0);
            totals.dexterity += dex.unwrap_or(0);
            totals.constitution += con.unwrap_or(0);
            totals.intelligence += intelligence.unwrap_or(0);
        };
    match item {
        ItemDefinition::Weapon(value) => {
            let WeaponStats {
                strength,
                dex,
                intelligence,
            } = value.stats;
            add(strength, dex, None, intelligence);
        }
        ItemDefinition::Shield(value) => {
            let ShieldStats { con, dex } = value.stats;
            add(None, dex, Some(con), None);
        }
        ItemDefinition::Helmet(value) => match &value.stats {
            HelmetStats::Constitution(v) => add(None, None, Some(v.con), None),
            HelmetStats::Intelligence(v) => add(None, None, None, Some(v.intelligence)),
        },
        ItemDefinition::Body(value) => {
            let BodyStats {
                con,
                dex,
                intelligence,
            } = value.stats;
            add(None, dex, Some(con), intelligence);
        }
        ItemDefinition::Accessory(_)
        | ItemDefinition::Consumable(_)
        | ItemDefinition::Material(_)
        | ItemDefinition::Key(_)
        | ItemDefinition::MagicCore(_) => {}
    }
}

fn item_slot_and_category(item: &ItemDefinition) -> Option<(EquipmentSlot, &str)> {
    match item {
        ItemDefinition::Weapon(v) => Some((EquipmentSlot::Weapon, &v.slot_category)),
        ItemDefinition::Shield(v) => Some((EquipmentSlot::Shield, &v.slot_category)),
        ItemDefinition::Helmet(v) => Some((EquipmentSlot::Helmet, &v.slot_category)),
        ItemDefinition::Body(v) => Some((EquipmentSlot::Body, &v.slot_category)),
        ItemDefinition::Accessory(_) => Some((EquipmentSlot::Accessory, "all")),
        _ => None,
    }
}

fn slot_allowed(slots: &ClassEquipmentSlots, slot: EquipmentSlot, category: &str) -> bool {
    let values = match slot {
        EquipmentSlot::Weapon => &slots.weapon,
        EquipmentSlot::Shield => &slots.shield,
        EquipmentSlot::Helmet => &slots.helmet,
        EquipmentSlot::Body => &slots.body,
        EquipmentSlot::Accessory => &slots.accessory,
    };
    values
        .iter()
        .any(|allowed| allowed == "all" || allowed == category)
}

pub(crate) fn can_equip(
    member: &RuntimeMember,
    item: &ItemDefinition,
    catalog: &FieldMenuCatalog,
) -> Result<EquipmentSlot, MenuMutationError> {
    let (slot, category) = item_slot_and_category(item).ok_or(MenuMutationError::NotEquipment)?;
    if let ItemDefinition::Accessory(value) = item
        && !value.equippable.is_empty()
        && !value
            .equippable
            .iter()
            .any(|class| class == "all" || class == member.class_id())
    {
        return Err(MenuMutationError::ClassRestricted);
    }
    let class = catalog
        .class(member.class_id())
        .ok_or_else(|| MenuMutationError::UnknownClass(member.class_id().to_owned()))?;
    if !slot_allowed(&class.equipment_slots, slot, category) {
        return Err(MenuMutationError::SlotRestricted);
    }
    Ok(slot)
}

pub(crate) fn equip_item(
    game: &mut GameState,
    catalog: &FieldMenuCatalog,
    member_id: &str,
    item_id: &str,
) -> Result<Option<String>, MenuMutationError> {
    let item = catalog
        .item(item_id)
        .ok_or_else(|| MenuMutationError::UnknownItem(item_id.to_owned()))?;
    let member = game
        .party()
        .member(member_id)
        .ok_or_else(|| MenuMutationError::UnknownMember(member_id.to_owned()))?;
    let slot = can_equip(member, item, catalog)?;
    if game.repository().item_count(item_id) == 0 {
        return Err(MenuMutationError::NotOwned);
    }
    let old = member.equipment().get(slot).map(str::to_owned);
    if let Some(old_id) = &old {
        let room = game.repository().item_quantity_cap() - game.repository().item_count(old_id);
        if room == 0 && old_id != item_id {
            return Err(MenuMutationError::RepositoryFull);
        }
    }
    game.repository_mut()
        .remove_item(item_id, 1)
        .map_err(|error| MenuMutationError::Repository(error.to_string()))?;
    let displaced = game
        .party_mut()
        .member_mut(member_id)
        .expect("validated above")
        .equip(slot, item_id.to_owned())
        .map_err(|error| MenuMutationError::Repository(error.to_string()))?;
    if let Some(old_id) = &displaced {
        let _outcome = game
            .repository_mut()
            .add_item(old_id.clone(), 1)
            .map_err(|error| MenuMutationError::Repository(error.to_string()))?;
    }
    Ok(displaced)
}

pub(crate) fn unequip_item(
    game: &mut GameState,
    member_id: &str,
    slot: EquipmentSlot,
) -> Result<Option<String>, MenuMutationError> {
    let member = game
        .party()
        .member(member_id)
        .ok_or_else(|| MenuMutationError::UnknownMember(member_id.to_owned()))?;
    let Some(item_id) = member.equipment().get(slot).map(str::to_owned) else {
        return Ok(None);
    };
    if game.repository().item_count(&item_id) >= game.repository().item_quantity_cap() {
        return Err(MenuMutationError::RepositoryFull);
    }
    let removed = game
        .party_mut()
        .member_mut(member_id)
        .expect("validated above")
        .unequip(slot)
        .expect("the slot was populated above");
    let _outcome = game
        .repository_mut()
        .add_item(removed.clone(), 1)
        .map_err(|error| MenuMutationError::Repository(error.to_string()))?;
    Ok(Some(removed))
}

pub(crate) fn learned_field_abilities<'a>(
    member: &RuntimeMember,
    game: &GameState,
    catalog: &'a FieldMenuCatalog,
) -> Vec<&'a Ability> {
    catalog
        .unlocked_abilities(member.class_id(), member.level(), game.flags())
        .into_iter()
        .filter(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Heal(_)
                    | AbilityKind::Buff(_)
                    | AbilityKind::Utility(UtilityAbility::RemoveStatus { .. })
                    | AbilityKind::Utility(UtilityAbility::Warp { .. })
            )
        })
        .collect()
}

pub(crate) fn cast_heal(
    game: &mut GameState,
    ability: &Ability,
    caster_id: &str,
    target_id: &str,
) -> Result<u32, MenuMutationError> {
    let AbilityKind::Heal(heal) = &ability.kind else {
        return Err(MenuMutationError::NotFieldHeal);
    };
    let caster = game
        .party()
        .member(caster_id)
        .ok_or_else(|| MenuMutationError::UnknownMember(caster_id.to_owned()))?;
    let target = game
        .party()
        .member(target_id)
        .ok_or_else(|| MenuMutationError::UnknownMember(target_id.to_owned()))?;
    if caster.mana() < ability.mp_cost {
        return Err(MenuMutationError::InsufficientMana);
    }
    if target.is_knocked_out() || target.health() == target.max_health() {
        return Err(MenuMutationError::InvalidTarget);
    }
    match heal.target {
        AbilityTarget::SingleAlly => {}
        AbilityTarget::SelfTarget if caster_id == target_id => {}
        _ => return Err(MenuMutationError::InvalidTarget),
    }
    let amount = match heal.healing {
        HealingMethod::Restore {
            coefficient,
            max_hp_percent,
        } => max_hp_percent.map_or_else(
            || (f64::from(caster.stats().intelligence()) * coefficient.get()) as u32,
            |pct| (f64::from(target.max_health()) * pct.get()) as u32,
        ),
        HealingMethod::Revive { .. } => return Err(MenuMutationError::InvalidTarget),
    };
    game.party_mut()
        .member_mut(caster_id)
        .expect("validated")
        .spend_mana(ability.mp_cost);
    Ok(game
        .party_mut()
        .member_mut(target_id)
        .expect("validated")
        .restore_health(amount))
}

/// True when an item's field effect covers the whole party instead of one chosen member.
///
/// The source branches on this before it ever offers a target picker
/// (`engine/item/item_scene.py::_begin_use`), which is why the port must ask the same question
/// first: routing a Tent through the picker is what made it unusable.
pub(crate) fn targets_whole_party(catalog: &FieldMenuCatalog, item_id: &str) -> bool {
    matches!(
        catalog.field_use(item_id),
        Some(FieldUseDefinition::RestoreFull {
            target: FullRecoveryTarget::AllAlive,
            ..
        })
    )
}

/// Spends one party-wide item on every living member.
///
/// Mirrors `_apply_aoe` with `valid_targets`: the targets are exactly the members above zero HP,
/// the effect lands on all of them, and the item is consumed once no matter how many it helped.
/// The source applies unconditionally rather than refusing a full-health party ("warn-and-allow"
/// in `item_effect_handler.apply`), so a Tent at full health is spent, not rejected.
pub(crate) fn use_field_item_on_party(
    game: &mut GameState,
    catalog: &FieldMenuCatalog,
    item_id: &str,
) -> Result<u32, MenuMutationError> {
    if game.repository().item_count(item_id) == 0 {
        return Err(MenuMutationError::NotOwned);
    }
    let effect = catalog
        .field_use(item_id)
        .ok_or(MenuMutationError::NotFieldUsable)?
        .clone();
    let FieldUseDefinition::RestoreFull {
        target: FullRecoveryTarget::AllAlive,
        cures,
        ..
    } = &effect
    else {
        return Err(MenuMutationError::InvalidTarget);
    };
    let cures = cures.clone();
    let living = game
        .party()
        .members()
        .filter(|member| !member.is_knocked_out())
        .map(|member| member.id().to_owned())
        .collect::<Vec<String>>();
    if living.is_empty() {
        return Err(MenuMutationError::InvalidTarget);
    }
    let mut changed = 0;
    for id in living {
        let member = game
            .party_mut()
            .member_mut(&id)
            .expect("a member listed a moment ago is still in the party");
        changed += member.restore_health(u32::MAX) + member.restore_mana(u32::MAX);
        changed += cures
            .iter()
            .filter(|status| member.cure_status(**status))
            .count() as u32;
    }
    if effect.consumable() {
        game.repository_mut()
            .remove_item(item_id, 1)
            .map_err(|error| MenuMutationError::Repository(error.to_string()))?;
    }
    Ok(changed)
}

pub(crate) fn use_field_item(
    game: &mut GameState,
    catalog: &FieldMenuCatalog,
    item_id: &str,
    target_id: &str,
) -> Result<u32, MenuMutationError> {
    if game.repository().item_count(item_id) == 0 {
        return Err(MenuMutationError::NotOwned);
    }
    let effect = catalog
        .field_use(item_id)
        .ok_or(MenuMutationError::NotFieldUsable)?
        .clone();
    let target = game
        .party()
        .member(target_id)
        .ok_or_else(|| MenuMutationError::UnknownMember(target_id.to_owned()))?;
    let applicable = match &effect {
        FieldUseDefinition::RestoreHp { .. } => {
            !target.is_knocked_out() && target.health() < target.max_health()
        }
        FieldUseDefinition::RestoreMp { .. } => {
            !target.is_knocked_out() && target.mana() < target.max_mana()
        }
        FieldUseDefinition::RestoreFull {
            target: FullRecoveryTarget::SingleAlive,
            cures,
            ..
        } => {
            !target.is_knocked_out()
                && (target.health() < target.max_health()
                    || target.mana() < target.max_mana()
                    || cures.iter().any(|status| target.has_status(*status)))
        }
        FieldUseDefinition::RestoreFull {
            target: FullRecoveryTarget::AllAlive,
            ..
        } => return Err(MenuMutationError::InvalidTarget),
        FieldUseDefinition::Cure { cures, .. } => {
            !target.is_knocked_out() && cures.iter().any(|status| target.has_status(*status))
        }
        FieldUseDefinition::Revive { .. } => target.is_knocked_out(),
    };
    if !applicable {
        return Err(MenuMutationError::InvalidTarget);
    }
    let member = game.party_mut().member_mut(target_id).expect("validated");
    let changed = match &effect {
        FieldUseDefinition::RestoreHp { amount, .. } => member.restore_health(amount.get()),
        FieldUseDefinition::RestoreMp { amount, .. } => member.restore_mana(amount.get()),
        FieldUseDefinition::RestoreFull { cures, .. } => {
            let changed = member.restore_health(u32::MAX) + member.restore_mana(u32::MAX);
            changed
                + cures
                    .iter()
                    .filter(|status| member.cure_status(**status))
                    .count() as u32
        }
        FieldUseDefinition::Cure { cures, .. } => cures
            .iter()
            .filter(|status| member.cure_status(**status))
            .count() as u32,
        FieldUseDefinition::Revive { revive_hp_pct, .. } => {
            // RuntimeMember intentionally exposes ordinary healing as non-reviving; M6 needs one
            // source-authored revive path, implemented in `revive` below.
            member.revive(revive_hp_pct.get())
        }
    };
    if effect.consumable() {
        game.repository_mut()
            .remove_item(item_id, 1)
            .map_err(|error| MenuMutationError::Repository(error.to_string()))?;
    }
    Ok(changed)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum MenuMutationError {
    UnknownItem(String),
    UnknownClass(String),
    UnknownMember(String),
    NotOwned,
    KeyItem,
    LockedItem,
    NotEquipment,
    ClassRestricted,
    SlotRestricted,
    RepositoryFull,
    NotFieldUsable,
    NotFieldHeal,
    InsufficientMana,
    InvalidTarget,
    Repository(String),
}
impl fmt::Display for MenuMutationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownItem(id) => write!(f, "Unknown item: {id}"),
            Self::UnknownClass(id) => write!(f, "Unknown class: {id}"),
            Self::UnknownMember(id) => write!(f, "Unknown party member: {id}"),
            Self::NotOwned => f.write_str("That item is not in the repository."),
            Self::KeyItem => f.write_str("Key items cannot be discarded."),
            Self::LockedItem => f.write_str("That item is locked."),
            Self::NotEquipment => f.write_str("That item cannot be equipped."),
            Self::ClassRestricted => f.write_str("This class cannot equip that item."),
            Self::SlotRestricted => {
                f.write_str("That equipment category is not allowed in this slot.")
            }
            Self::RepositoryFull => {
                f.write_str("The displaced item would exceed the repository cap.")
            }
            Self::NotFieldUsable => f.write_str("That item cannot be used in the field."),
            Self::NotFieldHeal => f.write_str("That ability is not a field healing spell."),
            Self::InsufficientMana => f.write_str("Not enough MP."),
            Self::InvalidTarget => f.write_str("No valid target or effect."),
            Self::Repository(error) => f.write_str(error),
        }
    }
}
impl Error for MenuMutationError {}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{
        game_state::GameStateParts,
        gameplay_rng::GameplayRng,
        playtime::Playtime,
        runtime_flags::RuntimeFlags,
        runtime_map::{RuntimeMapId, RuntimeMapState},
        runtime_member::RuntimeMember,
        runtime_opened_boxes::RuntimeOpenedBoxes,
        runtime_party::RuntimeParty,
        runtime_repository::RuntimeRepository,
        scenario_balance::BalanceData,
        scenario_party::PartyCatalog,
        scenario_spatial::{CardinalDirection, Position},
        tmx_header::parse_tmx_map_document,
    };

    /// Parses one real scenario map's metadata and outgoing portals, for warp-destination parity
    /// fixtures that compute against actual shipped `rusted_kingdoms` data.
    fn parsed_map(stem: &str, tmx: &str, yaml: &str) -> (MapMetadata, Vec<RuntimePortal>) {
        let metadata: MapMetadata = scenario_yaml::from_str(yaml).unwrap();
        let path = ScenarioRelativePath::try_from(format!("assets/maps/{stem}.tmx")).unwrap();
        let document = parse_tmx_map_document(tmx, &path).unwrap();
        let portals = runtime_portals(&document).unwrap();
        (metadata, portals)
    }

    pub(crate) fn catalog() -> FieldMenuCatalog {
        let item_documents = [
            include_str!("../../../assets/scenarios/rusted_kingdoms/data/items/accessories.yaml"),
            include_str!("../../../assets/scenarios/rusted_kingdoms/data/items/body.yaml"),
            include_str!(
                "../../../assets/scenarios/rusted_kingdoms/data/items/consumables_battle_throw.yaml"
            ),
            include_str!(
                "../../../assets/scenarios/rusted_kingdoms/data/items/consumables_field.yaml"
            ),
            include_str!(
                "../../../assets/scenarios/rusted_kingdoms/data/items/consumables_recovery.yaml"
            ),
            include_str!(
                "../../../assets/scenarios/rusted_kingdoms/data/items/consumables_status_cure.yaml"
            ),
            include_str!("../../../assets/scenarios/rusted_kingdoms/data/items/helmets.yaml"),
            include_str!("../../../assets/scenarios/rusted_kingdoms/data/items/key_items.yaml"),
            include_str!("../../../assets/scenarios/rusted_kingdoms/data/items/magic_cores.yaml"),
            include_str!("../../../assets/scenarios/rusted_kingdoms/data/items/materials.yaml"),
            include_str!(
                "../../../assets/scenarios/rusted_kingdoms/data/items/migration_zone1_drops.yaml"
            ),
            include_str!("../../../assets/scenarios/rusted_kingdoms/data/items/shields.yaml"),
            include_str!("../../../assets/scenarios/rusted_kingdoms/data/items/weapons.yaml"),
        ];
        let class_documents = [
            include_str!("../../../assets/scenarios/rusted_kingdoms/data/classes/cleric.yaml"),
            include_str!("../../../assets/scenarios/rusted_kingdoms/data/classes/hero.yaml"),
            include_str!("../../../assets/scenarios/rusted_kingdoms/data/classes/rogue.yaml"),
            include_str!("../../../assets/scenarios/rusted_kingdoms/data/classes/sorcerer.yaml"),
            include_str!("../../../assets/scenarios/rusted_kingdoms/data/classes/warrior.yaml"),
        ];
        let mut items = BTreeMap::new();
        let mut item_order = Vec::new();
        for document in item_documents {
            let file: ItemCatalogFile = scenario_yaml::from_str(document).unwrap();
            for item in file.0 {
                item_order.push(item.id().to_owned());
                assert!(items.insert(item.id().to_owned(), item).is_none());
            }
        }
        let field_use: FieldUseCatalogFile = scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/data/items/field_use.yaml"
        ))
        .unwrap();
        let field_uses = field_use
            .0
            .into_iter()
            .map(|entry| (entry.id().to_owned(), entry))
            .collect();
        let classes = class_documents
            .into_iter()
            .map(|document| {
                let class: ClassDefinition = scenario_yaml::from_str(document).unwrap();
                (class.class_id.clone(), class)
            })
            .collect();
        FieldMenuCatalog {
            status: CatalogStatus::Ready,
            items,
            field_uses,
            classes,
            item_order,
            warp_destinations: Vec::new(),
            failure: None,
            ..default()
        }
    }

    pub(crate) fn game(flags: impl IntoIterator<Item = &'static str>) -> GameState {
        let party_source: PartyCatalog = scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/data/party.yaml"
        ))
        .unwrap();
        let balance: BalanceData = scenario_yaml::from_str(include_str!(
            "../../../assets/scenarios/rusted_kingdoms/data/balance.yaml"
        ))
        .unwrap();
        let members = party_source.party[..2].iter().map(|source| {
            RuntimeMember::try_from_catalog(
                source,
                &crate::runtime_member::test_class_of(source),
                &balance.progression,
            )
            .unwrap()
        });
        GameState::try_from_parts(GameStateParts {
            flags: RuntimeFlags::from_bootstrap(flags),
            party: RuntimeParty::try_from_members(members).unwrap(),
            repository: RuntimeRepository::from_balance(&balance.economy),
            map: RuntimeMapState::new(
                RuntimeMapId::try_new("town_01_ardel").unwrap(),
                Position::new(14, 5),
                CardinalDirection::Down,
            ),
            opened_boxes: RuntimeOpenedBoxes::default(),
            controlled_member_id: "aric".to_owned(),
            rng: GameplayRng::default(),
            playtime: Playtime::default(),
        })
        .unwrap()
    }

    #[test]
    fn production_catalog_addresses_all_current_items_classes_and_field_effects() {
        let catalog = catalog();
        assert_eq!(catalog.items.len(), 176);
        assert_eq!(catalog.item_order.len(), 176);
        assert_eq!(catalog.classes.len(), 5);
        assert_eq!(catalog.field_uses.len(), 13);
        for id in [
            "potion",
            "antidote",
            "iron_sword",
            "mc_s",
            "sky_crystal",
            "goblin_ear",
            "goblin_fang",
            "rusty_blade",
            "goblin_shield",
        ] {
            assert!(catalog.item(id).is_some(), "missing {id}");
        }
    }

    #[test]
    fn inventory_tabs_hidden_filter_and_latest_batch_share_repository_state() {
        let catalog = catalog();
        let mut game = game([]);
        let first = game.repository_mut().start_loot_batch();
        let _outcome = game
            .repository_mut()
            .add_item_in_batch("potion", 2, first)
            .unwrap();
        let _outcome = game.repository_mut().add_item("sky_crystal", 1).unwrap();
        let _outcome = game.repository_mut().add_item("mc_s", 3).unwrap();
        let latest = game.repository_mut().start_loot_batch();
        let _outcome = game
            .repository_mut()
            .add_item_in_batch("antidote", 1, latest)
            .unwrap();

        assert_eq!(
            inventory_ids(&game, &catalog, InventoryTab::New),
            ["antidote"]
        );
        assert_eq!(
            inventory_ids(&game, &catalog, InventoryTab::Recovery),
            ["potion"]
        );
        assert_eq!(
            inventory_ids(&game, &catalog, InventoryTab::Status),
            ["antidote"]
        );
        assert_eq!(inventory_ids(&game, &catalog, InventoryTab::Core), ["mc_s"]);
        assert_eq!(
            inventory_ids(&game, &catalog, InventoryTab::Key),
            ["sky_crystal"]
        );
        game.repository_mut().set_hidden("antidote", true);
        assert!(!inventory_ids(&game, &catalog, InventoryTab::All).contains(&"antidote"));
        assert!(inventory_ids(&game, &catalog, InventoryTab::New).is_empty());
    }

    #[test]
    fn discard_blocks_key_and_locked_items_and_removes_exact_quantities() {
        let catalog = catalog();
        let mut game = game([]);
        let _outcome = game.repository_mut().add_item("potion", 5).unwrap();
        let _outcome = game.repository_mut().add_item("sky_crystal", 1).unwrap();
        game.repository_mut().set_locked("potion", true);
        assert_eq!(
            discard_item(&mut game, &catalog, "potion", 2),
            Err(MenuMutationError::LockedItem)
        );
        assert_eq!(game.repository().item_count("potion"), 5);
        assert_eq!(
            discard_item(&mut game, &catalog, "sky_crystal", 1),
            Err(MenuMutationError::KeyItem)
        );
        assert_eq!(game.repository().item_count("sky_crystal"), 1);
        game.repository_mut().set_locked("potion", false);
        discard_item(&mut game, &catalog, "potion", 2).unwrap();
        assert_eq!(game.repository().item_count("potion"), 3);
        discard_item(&mut game, &catalog, "potion", 3).unwrap();
        assert!(!game.repository().contains_item("potion"));
    }

    #[test]
    fn healing_and_status_items_consume_only_on_a_valid_effect() {
        let catalog = catalog();
        let mut game = game([]);
        let _outcome = game.repository_mut().add_item("potion", 2).unwrap();
        let _outcome = game.repository_mut().add_item("antidote", 2).unwrap();
        game.party_mut()
            .member_mut("aric")
            .unwrap()
            .apply_damage(10);
        assert_eq!(
            use_field_item(&mut game, &catalog, "potion", "aric"),
            Ok(10)
        );
        assert_eq!(game.party().member("aric").unwrap().health(), 22);
        assert_eq!(game.repository().item_count("potion"), 1);
        assert_eq!(
            use_field_item(&mut game, &catalog, "potion", "aric"),
            Err(MenuMutationError::InvalidTarget)
        );
        assert_eq!(game.repository().item_count("potion"), 1);

        game.party_mut()
            .member_mut("aric")
            .unwrap()
            .add_status(crate::scenario_item::ItemStatus::Poison);
        assert_eq!(
            use_field_item(&mut game, &catalog, "antidote", "aric"),
            Ok(1)
        );
        assert!(
            !game
                .party()
                .member("aric")
                .unwrap()
                .has_status(crate::scenario_item::ItemStatus::Poison)
        );
        assert_eq!(game.repository().item_count("antidote"), 1);
        assert_eq!(
            use_field_item(&mut game, &catalog, "antidote", "aric"),
            Err(MenuMutationError::InvalidTarget)
        );
        assert_eq!(game.repository().item_count("antidote"), 1);
    }

    /// The Tent and Rest Capsule were unusable: the field menu sent every item through the
    /// single-target picker, and `use_field_item` rejects an `all_alive` effect on one member, so
    /// both shipped consumables refused every target in the party.
    #[test]
    fn a_party_wide_item_heals_everyone_alive_and_is_spent_once() {
        let catalog = catalog();
        let mut game = game([]);
        let _outcome = game.repository_mut().add_item("tent", 2).unwrap();
        game.party_mut()
            .member_mut("aric")
            .unwrap()
            .apply_damage(10);
        game.party_mut()
            .member_mut("elise")
            .unwrap()
            .apply_damage(6);
        game.party_mut()
            .member_mut("elise")
            .unwrap()
            .add_status(crate::scenario_item::ItemStatus::Poison);

        assert!(targets_whole_party(&catalog, "tent"));
        assert!(
            !targets_whole_party(&catalog, "potion"),
            "a single-target item must still reach the picker"
        );

        let changed = use_field_item_on_party(&mut game, &catalog, "tent").unwrap();

        assert!(
            changed >= 17,
            "both members healed, plus the cure: {changed}"
        );
        let aric = game.party().member("aric").unwrap();
        let elise = game.party().member("elise").unwrap();
        assert_eq!(aric.health(), aric.max_health());
        assert_eq!(elise.health(), elise.max_health());
        assert!(!elise.has_status(crate::scenario_item::ItemStatus::Poison));
        assert_eq!(
            game.repository().item_count("tent"),
            1,
            "one Tent covers the whole party, it is not spent per member"
        );
    }

    #[test]
    fn a_party_wide_item_is_refused_when_it_is_not_owned_or_not_party_wide() {
        let catalog = catalog();
        let mut game = game([]);

        assert_eq!(
            use_field_item_on_party(&mut game, &catalog, "tent"),
            Err(MenuMutationError::NotOwned)
        );

        let _outcome = game.repository_mut().add_item("potion", 1).unwrap();
        assert_eq!(
            use_field_item_on_party(&mut game, &catalog, "potion"),
            Err(MenuMutationError::InvalidTarget),
            "a single-target item must not be spendable through the party-wide path"
        );
        assert_eq!(game.repository().item_count("potion"), 1);
    }

    /// The source applies to a full-health party rather than refusing ("warn-and-allow" in
    /// `item_effect_handler.apply`), so the item is spent. Reproduced deliberately.
    #[test]
    fn a_party_wide_item_used_at_full_health_is_still_spent() {
        let catalog = catalog();
        let mut game = game([]);
        let _outcome = game.repository_mut().add_item("tent", 1).unwrap();

        assert_eq!(use_field_item_on_party(&mut game, &catalog, "tent"), Ok(0));
        assert_eq!(game.repository().item_count("tent"), 0);
    }

    #[test]
    fn equipment_preview_compatibility_and_swap_match_source_totals_atomically() {
        let catalog = catalog();
        let mut game = game([]);
        let _outcome = game.repository_mut().add_item("steel_axe", 1).unwrap();
        let _outcome = game.repository_mut().add_item("dagger", 1).unwrap();
        let aric = game.party().member("aric").unwrap();
        let before = derived_stats(aric, &catalog);
        assert_eq!(
            before,
            DerivedStats {
                strength: 32,
                dexterity: 17,
                constitution: 32,
                intelligence: 5,
            }
        );
        let preview = preview_stats(aric, &catalog, EquipmentSlot::Weapon, Some("steel_axe"));
        assert_eq!(preview.strength - before.strength, 1);
        assert_eq!(preview.dexterity - before.dexterity, -1);
        assert_eq!(
            can_equip(aric, catalog.item("dagger").unwrap(), &catalog),
            Err(MenuMutationError::SlotRestricted)
        );

        let displaced = equip_item(&mut game, &catalog, "aric", "steel_axe").unwrap();
        assert_eq!(displaced.as_deref(), Some("iron_sword"));
        assert_eq!(
            game.party()
                .member("aric")
                .unwrap()
                .equipment()
                .get(EquipmentSlot::Weapon),
            Some("steel_axe")
        );
        assert_eq!(game.repository().item_count("steel_axe"), 0);
        assert_eq!(game.repository().item_count("iron_sword"), 1);
        let before_rejection = game.repository().item_count("dagger");
        assert_eq!(
            equip_item(&mut game, &catalog, "aric", "dagger"),
            Err(MenuMutationError::SlotRestricted)
        );
        assert_eq!(game.repository().item_count("dagger"), before_rejection);
        assert_eq!(
            game.party()
                .member("aric")
                .unwrap()
                .equipment()
                .get(EquipmentSlot::Weapon),
            Some("steel_axe")
        );
    }

    #[test]
    fn learned_field_spells_gate_by_level_and_flag_and_valid_heal_spends_mp_once() {
        let catalog = catalog();
        let mut game = game(["aric_teleport_unlocked"]);
        let aric = game.party().member("aric").unwrap();
        assert_eq!(
            learned_field_abilities(aric, &game, &catalog)
                .iter()
                .map(|ability| ability.id.as_str())
                .collect::<Vec<_>>(),
            ["teleport"]
        );
        let elise = game.party().member("elise").unwrap();
        assert_eq!(
            learned_field_abilities(elise, &game, &catalog)
                .iter()
                .map(|ability| ability.id.as_str())
                .collect::<Vec<_>>(),
            ["heal"]
        );

        game.party_mut()
            .member_mut("aric")
            .unwrap()
            .apply_damage(10);
        let heal = learned_field_abilities(game.party().member("elise").unwrap(), &game, &catalog)
            [0]
        .clone();
        assert_eq!(cast_heal(&mut game, &heal, "elise", "aric"), Ok(10));
        assert_eq!(game.party().member("elise").unwrap().mana(), 14);
        assert_eq!(
            cast_heal(&mut game, &heal, "elise", "aric"),
            Err(MenuMutationError::InvalidTarget)
        );
        assert_eq!(game.party().member("elise").unwrap().mana(), 14);

        let second_wind = catalog
            .class("hero")
            .unwrap()
            .abilities
            .iter()
            .find(|ability| ability.id == "second_wind")
            .unwrap()
            .clone();
        game.party_mut()
            .member_mut("aric")
            .unwrap()
            .apply_damage(10);
        assert_eq!(
            cast_heal(&mut game, &second_wind, "aric", "elise"),
            Err(MenuMutationError::InvalidTarget)
        );
        assert_eq!(game.party().member("aric").unwrap().mana(), 12);
        assert_eq!(cast_heal(&mut game, &second_wind, "aric", "aric"), Ok(5));
        assert_eq!(game.party().member("aric").unwrap().mana(), 6);

        let without_flag = self::game([]);
        assert!(
            learned_field_abilities(
                without_flag.party().member("aric").unwrap(),
                &without_flag,
                &catalog
            )
            .is_empty()
        );
    }

    #[test]
    fn teleport_picker_excludes_current_and_unvisited_destinations() {
        let mut catalog = catalog();
        catalog.warp_destinations = vec![
            WarpDestination {
                map_id: "town_01_ardel".to_owned(),
                name: "Ardel Village".to_owned(),
                position: Position::new(27, 12),
                town: true,
                order: 10,
            },
            WarpDestination {
                map_id: "zone_01_starting_forest".to_owned(),
                name: "Starting Forest".to_owned(),
                position: Position::new(29, 1),
                town: false,
                order: 20,
            },
        ];
        let mut game = game([]);
        assert!(catalog.eligible_warp_destinations(game.map()).is_empty());
        game.map_mut().move_to(
            RuntimeMapId::try_new("zone_01_starting_forest").unwrap(),
            Position::new(29, 1),
            CardinalDirection::Down,
        );
        assert_eq!(
            catalog
                .eligible_warp_destinations(game.map())
                .iter()
                .map(|destination| destination.map_id.as_str())
                .collect::<Vec<_>>(),
            ["town_01_ardel"]
        );
    }

    // -- Parity fixtures: `compute_warp_destinations` vs the pinned `engine/world/warp_logic.py`
    // (parity plan P1.1). Expected values below were cross-checked two ways against this exact
    // scenario copy: by hand from the TMX/YAML, and by running the actual pinned `warp_logic.py`
    // module (with a real PyYAML available in a sibling checkout) against
    // `assets/scenarios/rusted_kingdoms`.

    #[test]
    fn warp_destinations_match_pinned_scenario_for_a_representative_visited_set() {
        let ardel = parsed_map(
            "town_01_ardel",
            include_str!("../../../assets/scenarios/rusted_kingdoms/assets/maps/town_01_ardel.tmx"),
            include_str!("../../../assets/scenarios/rusted_kingdoms/data/maps/town_01_ardel.yaml"),
        );
        // Real submap: `town_01_ardel_shop_01` extends `town_01_ardel`'s id with `_`, and
        // `town_01_ardel` has its own TMX, so `_is_submap` excludes it (warp_logic.py:68-78).
        let ardel_shop = parsed_map(
            "town_01_ardel_shop_01",
            include_str!(
                "../../../assets/scenarios/rusted_kingdoms/assets/maps/town_01_ardel_shop_01.tmx"
            ),
            include_str!(
                "../../../assets/scenarios/rusted_kingdoms/data/maps/town_01_ardel_shop_01.yaml"
            ),
        );
        let forest = parsed_map(
            "zone_01_starting_forest",
            include_str!(
                "../../../assets/scenarios/rusted_kingdoms/assets/maps/zone_01_starting_forest.tmx"
            ),
            include_str!(
                "../../../assets/scenarios/rusted_kingdoms/data/maps/zone_01_starting_forest.yaml"
            ),
        );
        let plains = parsed_map(
            "zone_02_open_plains",
            include_str!(
                "../../../assets/scenarios/rusted_kingdoms/assets/maps/zone_02_open_plains.tmx"
            ),
            include_str!(
                "../../../assets/scenarios/rusted_kingdoms/data/maps/zone_02_open_plains.yaml"
            ),
        );
        let millhaven = parsed_map(
            "town_02_millhaven",
            include_str!(
                "../../../assets/scenarios/rusted_kingdoms/assets/maps/town_02_millhaven.tmx"
            ),
            include_str!(
                "../../../assets/scenarios/rusted_kingdoms/data/maps/town_02_millhaven.yaml"
            ),
        );

        let sources: Vec<(&str, &MapMetadata, &[RuntimePortal])> = vec![
            ("town_01_ardel", &ardel.0, &ardel.1),
            ("town_01_ardel_shop_01", &ardel_shop.0, &ardel_shop.1),
            ("zone_01_starting_forest", &forest.0, &forest.1),
            ("zone_02_open_plains", &plains.0, &plains.1),
            ("town_02_millhaven", &millhaven.0, &millhaven.1),
        ];
        let destinations =
            compute_warp_destinations(&sources).expect("every candidate here has warp_order");

        // The interior is excluded as a submap, not silently missing for some other reason.
        assert!(
            !destinations
                .iter()
                .any(|destination| destination.map_id == "town_01_ardel_shop_01")
        );

        // Towns first (by warp_order), then world zones (by warp_order); landing tiles are the
        // (27, 12) / (29, 1) / (2, 1) / (19, 29) incoming-portal positions authored elsewhere in
        // the scenario, not each map's own spawn point.
        assert_eq!(
            destinations
                .iter()
                .map(|destination| (
                    destination.map_id.as_str(),
                    destination.town,
                    destination.order,
                    destination.position,
                ))
                .collect::<Vec<_>>(),
            vec![
                ("town_01_ardel", true, 10, Position::new(27, 12)),
                ("town_02_millhaven", true, 40, Position::new(19, 29)),
                ("zone_01_starting_forest", false, 20, Position::new(29, 1)),
                ("zone_02_open_plains", false, 30, Position::new(2, 1)),
            ]
        );

        // town_01_ardel's landing also proves the tie-break: zone_01_starting_forest.tmx has two
        // portals into town_01_ardel, (27, 12) [object id 1] and (20, 17) [object id 9] — the
        // first one in TMX document order wins over the later one from the same source.
        assert_eq!(
            destinations
                .iter()
                .find(|destination| destination.map_id == "town_01_ardel")
                .unwrap()
                .position,
            Position::new(27, 12)
        );
    }

    #[test]
    fn warp_destinations_do_not_treat_numbered_zone_segments_as_submaps_of_each_other() {
        // `zone_05_mountain_foothills_01/02/03` are numbered segments of the conceptual zone
        // `zone_05_mountain_foothills`, but that parent id has no `.tmx` of its own in the pinned
        // scenario — only `data/maps/zone_05_mountain_foothills.yaml`. `_is_submap`
        // checks each candidate's prefix against `all_ids`, built from `assets/maps/*.tmx` stems
        // only (warp_logic.py:68-78, 152), so a parent id with no TMX never enters that set and
        // its segments are *not* submaps of one another under the literal algorithm — confirmed
        // by running the pinned `warp_logic.py` against this scenario copy. Both are therefore
        // independent, real warp destinations.
        let segment_02 = parsed_map(
            "zone_05_mountain_foothills_02",
            include_str!(
                "../../../assets/scenarios/rusted_kingdoms/assets/maps/zone_05_mountain_foothills_02.tmx"
            ),
            include_str!(
                "../../../assets/scenarios/rusted_kingdoms/data/maps/zone_05_mountain_foothills_02.yaml"
            ),
        );
        let segment_03 = parsed_map(
            "zone_05_mountain_foothills_03",
            include_str!(
                "../../../assets/scenarios/rusted_kingdoms/assets/maps/zone_05_mountain_foothills_03.tmx"
            ),
            include_str!(
                "../../../assets/scenarios/rusted_kingdoms/data/maps/zone_05_mountain_foothills_03.yaml"
            ),
        );

        let sources: Vec<(&str, &MapMetadata, &[RuntimePortal])> = vec![
            (
                "zone_05_mountain_foothills_02",
                &segment_02.0,
                &segment_02.1,
            ),
            (
                "zone_05_mountain_foothills_03",
                &segment_03.0,
                &segment_03.1,
            ),
        ];
        let destinations =
            compute_warp_destinations(&sources).expect("both segments declare warp_order");

        assert_eq!(
            destinations
                .iter()
                .map(|destination| (
                    destination.map_id.as_str(),
                    destination.order,
                    destination.position
                ))
                .collect::<Vec<_>>(),
            vec![
                ("zone_05_mountain_foothills_02", 120, Position::new(53, 21)),
                ("zone_05_mountain_foothills_03", 125, Position::new(2, 20)),
            ]
        );
    }

    #[test]
    fn warp_destinations_surface_a_missing_warp_order_as_a_scenario_error_not_a_silent_drop() {
        // The pinned scenario has no `data/maps/zone_05_mountain_foothills_01.yaml` at all
        // (confirmed against both this repository and the upstream Python source directory) even
        // though its TMX is real, it has a genuine incoming portal from
        // `zone_04_ancient_ruins_03_sanctum`, and (per the sibling-segments test above) it is not
        // excluded as a submap. Under `warp_logic.py::_map_order` a qualifying destination with
        // no `warp_order` raises rather than being defaulted or dropped
        // (warp_logic.py:124-139); this is the same scenario-data gap, and this test's
        // hand-authored metadata (only `name`, standing in for the missing file) isolates that
        // one behavior.
        let sanctum = parsed_map(
            "zone_04_ancient_ruins_03_sanctum",
            include_str!(
                "../../../assets/scenarios/rusted_kingdoms/assets/maps/zone_04_ancient_ruins_03_sanctum.tmx"
            ),
            include_str!(
                "../../../assets/scenarios/rusted_kingdoms/data/maps/zone_04_ancient_ruins_03_sanctum.yaml"
            ),
        );
        let foothills_01_tmx = include_str!(
            "../../../assets/scenarios/rusted_kingdoms/assets/maps/zone_05_mountain_foothills_01.tmx"
        );
        let path = ScenarioRelativePath::try_from("assets/maps/zone_05_mountain_foothills_01.tmx")
            .unwrap();
        let document = parse_tmx_map_document(foothills_01_tmx, &path).unwrap();
        let foothills_01_portals = runtime_portals(&document).unwrap();
        let foothills_01_metadata: MapMetadata = scenario_yaml::from_str(
            "name: \"(no data/maps/zone_05_mountain_foothills_01.yaml in the pinned scenario)\"\n",
        )
        .unwrap();

        let sources: Vec<(&str, &MapMetadata, &[RuntimePortal])> = vec![
            ("zone_04_ancient_ruins_03_sanctum", &sanctum.0, &sanctum.1),
            (
                "zone_05_mountain_foothills_01",
                &foothills_01_metadata,
                &foothills_01_portals,
            ),
        ];
        let error = compute_warp_destinations(&sources)
            .expect_err("zone_05_mountain_foothills_01 has no warp_order");
        assert_eq!(error.map_id, "zone_05_mountain_foothills_01");
    }
}
