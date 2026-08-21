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
    scenario_item::{
        BodyStats, FieldUseCatalogFile, FieldUseDefinition, FullRecoveryTarget, HelmetStats,
        ItemCatalogFile, ItemDefinition, ShieldStats, WeaponStats,
    },
    scenario_map::MapMetadata,
    scenario_path::ScenarioRelativePath,
    scenario_root::ScenarioRoot,
    scenario_yaml::{self, ScenarioYamlError},
    tmx_ground_asset::TmxGroundAsset,
    world_transition::runtime_portals,
};

const ITEM_FILES: [&str; 12] = [
    "accessories.yaml",
    "body.yaml",
    "consumables_battle_throw.yaml",
    "consumables_field.yaml",
    "consumables_recovery.yaml",
    "consumables_status_cure.yaml",
    "helmets.yaml",
    "key_items.yaml",
    "magic_cores.yaml",
    "materials.yaml",
    "shields.yaml",
    "weapons.yaml",
];
const CLASS_FILES: [&str; 5] = [
    "cleric.yaml",
    "hero.yaml",
    "rogue.yaml",
    "sorcerer.yaml",
    "warrior.yaml",
];
const FIELD_USE_FILE: &str = "field_use.yaml";
const MAP_FILES: [&str; 3] = [
    "town_01_ardel",
    "town_01_ardel_house_01",
    "zone_01_starting_forest",
];

pub(crate) struct FieldMenuDomainPlugin;

impl Plugin for FieldMenuDomainPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<ItemCatalogFile>()
            .init_asset::<FieldUseCatalogFile>()
            .init_asset::<ClassDefinition>()
            .init_asset_loader::<ItemCatalogAssetLoader>()
            .init_asset_loader::<FieldUseCatalogAssetLoader>()
            .init_asset_loader::<ClassDefinitionAssetLoader>()
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
    pub(crate) fn field_use(&self, id: &str) -> Option<&FieldUseDefinition> {
        self.field_uses.get(id)
    }
    pub(crate) fn class(&self, id: &str) -> Option<&ClassDefinition> {
        self.classes.get(id)
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WarpDestination {
    pub(crate) map_id: String,
    pub(crate) name: String,
    pub(crate) position: crate::scenario_spatial::Position,
    town: bool,
    order: u32,
}

#[derive(Debug, Default, Resource)]
struct FieldMenuCatalogLoad {
    items: Vec<(String, Handle<ItemCatalogFile>)>,
    field_use: Option<Handle<FieldUseCatalogFile>>,
    classes: Vec<(String, Handle<ClassDefinition>)>,
    maps: Vec<(String, Handle<MapMetadata>, Handle<TmxGroundAsset>)>,
}

fn begin_catalog_load(
    root: Res<ScenarioRoot>,
    asset_server: Res<AssetServer>,
    mut load: ResMut<FieldMenuCatalogLoad>,
    mut catalog: ResMut<FieldMenuCatalog>,
) {
    if catalog.status == CatalogStatus::Ready {
        return;
    }
    catalog.status = CatalogStatus::Loading;
    catalog.failure = None;
    load.items = ITEM_FILES
        .iter()
        .map(|name| {
            let relative = ScenarioRelativePath::try_from(format!("data/items/{name}"))
                .expect("compiled item catalog path is valid");
            (
                (*name).to_owned(),
                asset_server.load(root.resolve(&relative)),
            )
        })
        .collect();
    let field_path = ScenarioRelativePath::try_from(format!("data/items/{FIELD_USE_FILE}"))
        .expect("compiled field-use path is valid");
    load.field_use = Some(asset_server.load(root.resolve(&field_path)));
    load.classes = CLASS_FILES
        .iter()
        .map(|name| {
            let relative = ScenarioRelativePath::try_from(format!("data/classes/{name}"))
                .expect("compiled class catalog path is valid");
            (
                (*name).to_owned(),
                asset_server.load(root.resolve(&relative)),
            )
        })
        .collect();
    load.maps = MAP_FILES
        .iter()
        .map(|stem| {
            let metadata = ScenarioRelativePath::try_from(format!("data/maps/{stem}.yaml"))
                .expect("compiled map metadata path is valid");
            let tmx = ScenarioRelativePath::try_from(format!("assets/maps/{stem}.tmx"))
                .expect("compiled TMX path is valid");
            (
                (*stem).to_owned(),
                asset_server.load(root.resolve(&metadata)),
                asset_server.load(root.resolve(&tmx)),
            )
        })
        .collect();
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
    load: Res<FieldMenuCatalogLoad>,
    mut catalog: ResMut<FieldMenuCatalog>,
) {
    if catalog.status != CatalogStatus::Loading || load.field_use.is_none() {
        return;
    }
    for (path, handle) in &load.items {
        if let LoadState::Failed(error) = asset_server.load_state(handle.id()) {
            catalog.status = CatalogStatus::Failed;
            catalog.failure = Some(format!("data/items/{path}: {error}"));
            return;
        }
    }
    for (path, handle) in &load.classes {
        if let LoadState::Failed(error) = asset_server.load_state(handle.id()) {
            catalog.status = CatalogStatus::Failed;
            catalog.failure = Some(format!("data/classes/{path}: {error}"));
            return;
        }
    }
    for (stem, metadata, tmx) in &load.maps {
        if let LoadState::Failed(error) = asset_server.load_state(metadata.id()) {
            catalog.status = CatalogStatus::Failed;
            catalog.failure = Some(format!("data/maps/{stem}.yaml: {error}"));
            return;
        }
        if let LoadState::Failed(error) = asset_server.load_state(tmx.id()) {
            catalog.status = CatalogStatus::Failed;
            catalog.failure = Some(format!("assets/maps/{stem}.tmx: {error}"));
            return;
        }
    }
    let field_handle = load.field_use.as_ref().expect("checked above");
    if let LoadState::Failed(error) = asset_server.load_state(field_handle.id()) {
        catalog.status = CatalogStatus::Failed;
        catalog.failure = Some(format!("data/items/{FIELD_USE_FILE}: {error}"));
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
    let mut warp_destinations = Vec::new();
    for (stem, metadata_handle, _) in &load.maps {
        let metadata = map_assets.get(metadata_handle).expect("all checked");
        let Some(order) = metadata.warp_order else {
            continue;
        };
        let map_id = metadata.effective_id(stem);
        if load
            .maps
            .iter()
            .any(|(other, _, _)| other != stem && map_id.starts_with(&format!("{other}_")))
        {
            continue;
        }
        let landing = load
            .maps
            .iter()
            .filter_map(|(source_stem, _, tmx_handle)| {
                let tmx = tmx_assets.get(tmx_handle).expect("all checked");
                runtime_portals(tmx.document())
                    .ok()?
                    .into_iter()
                    .find(|portal| portal.target_map().as_str() == map_id)
                    .map(|portal| {
                        let from_submap = source_stem.starts_with(&format!("{map_id}_"));
                        (from_submap, source_stem.as_str(), portal.target_position())
                    })
            })
            .min_by_key(|(from_submap, source, _)| (*from_submap, *source));
        let Some((_, _, position)) = landing else {
            continue;
        };
        warp_destinations.push(WarpDestination {
            map_id: map_id.to_owned(),
            name: metadata.name.clone(),
            position,
            town: metadata.inn.is_some() || metadata.shop.is_some(),
            order,
        });
    }
    warp_destinations.sort_by_key(|destination| {
        (
            !destination.town,
            destination.order,
            destination.map_id.clone(),
        )
    });
    catalog.items = items;
    catalog.item_order = item_order;
    catalog.field_uses = field_uses;
    catalog.classes = classes;
    catalog.warp_destinations = warp_destinations;
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
mod tests {
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
    };

    fn catalog() -> FieldMenuCatalog {
        let item_documents = [
            include_str!("../assets/scenarios/rusted_kingdoms/data/items/accessories.yaml"),
            include_str!("../assets/scenarios/rusted_kingdoms/data/items/body.yaml"),
            include_str!(
                "../assets/scenarios/rusted_kingdoms/data/items/consumables_battle_throw.yaml"
            ),
            include_str!("../assets/scenarios/rusted_kingdoms/data/items/consumables_field.yaml"),
            include_str!(
                "../assets/scenarios/rusted_kingdoms/data/items/consumables_recovery.yaml"
            ),
            include_str!(
                "../assets/scenarios/rusted_kingdoms/data/items/consumables_status_cure.yaml"
            ),
            include_str!("../assets/scenarios/rusted_kingdoms/data/items/helmets.yaml"),
            include_str!("../assets/scenarios/rusted_kingdoms/data/items/key_items.yaml"),
            include_str!("../assets/scenarios/rusted_kingdoms/data/items/magic_cores.yaml"),
            include_str!("../assets/scenarios/rusted_kingdoms/data/items/materials.yaml"),
            include_str!("../assets/scenarios/rusted_kingdoms/data/items/shields.yaml"),
            include_str!("../assets/scenarios/rusted_kingdoms/data/items/weapons.yaml"),
        ];
        let class_documents = [
            include_str!("../assets/scenarios/rusted_kingdoms/data/classes/cleric.yaml"),
            include_str!("../assets/scenarios/rusted_kingdoms/data/classes/hero.yaml"),
            include_str!("../assets/scenarios/rusted_kingdoms/data/classes/rogue.yaml"),
            include_str!("../assets/scenarios/rusted_kingdoms/data/classes/sorcerer.yaml"),
            include_str!("../assets/scenarios/rusted_kingdoms/data/classes/warrior.yaml"),
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
            "../assets/scenarios/rusted_kingdoms/data/items/field_use.yaml"
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
        }
    }

    fn game(flags: impl IntoIterator<Item = &'static str>) -> GameState {
        let party_source: PartyCatalog = scenario_yaml::from_str(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/party.yaml"
        ))
        .unwrap();
        let balance: BalanceData = scenario_yaml::from_str(include_str!(
            "../assets/scenarios/rusted_kingdoms/data/balance.yaml"
        ))
        .unwrap();
        let members = party_source.party[..2]
            .iter()
            .map(|source| RuntimeMember::try_from_catalog(source, &balance.progression).unwrap());
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
        assert_eq!(catalog.items.len(), 172);
        assert_eq!(catalog.item_order.len(), 172);
        assert_eq!(catalog.classes.len(), 5);
        assert_eq!(catalog.field_uses.len(), 13);
        for id in ["potion", "antidote", "iron_sword", "mc_s", "sky_crystal"] {
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
}
