//! Catalog loading: the asset handles, the systems that load and track them, and the
//! construction of the catalog from the loaded assets.

use super::*;

/// Resolves the visited-map teleport destination list from every loaded map's metadata and
/// portals, (minus the caller's
/// visited/current filter, applied later by [`FieldMenuCatalog::eligible_warp_destinations`]):
///
/// - A destination is a `source` that is not a *submap* of another `source` — one whose id is
///   some other source's id followed by `_` (interiors like `..._shop_01`, and numbered segments
///   like `..._02`, both use this convention). Note this is checked
///   only against the *other loaded sources*, i.e. every map id that
///   actually has its own TMX — a segment whose bare parent id has no TMX of its own (e.g. the
///   shipped scenario's `zone_05_mountain_foothills_01/02/03`, which have no
///   `zone_05_mountain_foothills.tmx`) is therefore *not* excluded as a submap.
/// - Its landing tile comes from the scenario-wide incoming-portal index: every portal in every
///   `source` targeting it, preferring a non-submap source, tied broken by source id.
///   A destination with no incoming portal at all is dropped
///   (nowhere to land).
/// - It is categorized `town` when its map data has an `inn` or `shop` block, `world` otherwise.
/// - It must declare `warp_order`; see [`WarpOrderError`].
///
/// Grouped towns-then-world, ordered by `warp_order` within each group.
pub(super) fn compute_warp_destinations(
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

#[derive(Clone, Debug, Default, Resource)]
pub(crate) struct FieldMenuCatalogLoad {
    items: Vec<(String, Handle<ItemCatalogFile>)>,
    field_use: Option<Handle<FieldUseCatalogFile>>,
    classes: Vec<(String, Handle<ClassDefinition>)>,
    maps: Vec<(String, Handle<MapMetadata>, Handle<TmxGroundAsset>)>,
    service_maps: Vec<(String, Handle<MapMetadata>)>,
    recipes: Vec<(String, Handle<RecipeCatalogFile>)>,
    quests: Option<Handle<QuestCatalogFile>>,
    party: Option<Handle<PartyCatalog>>,
}

pub(crate) fn begin_catalog_load(
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
    load.party = inventory
        .party
        .as_ref()
        .map(|path| asset_server.load(root.resolve(path)));
}

#[expect(
    clippy::too_many_arguments,
    reason = "catalog publication checks each independently typed asset collection"
)]
pub(super) fn track_catalog_load(
    asset_server: Res<AssetServer>,
    item_assets: Res<Assets<ItemCatalogFile>>,
    field_assets: Res<Assets<FieldUseCatalogFile>>,
    class_assets: Res<Assets<ClassDefinition>>,
    map_assets: Res<Assets<MapMetadata>>,
    tmx_assets: Res<Assets<TmxGroundAsset>>,
    recipe_assets: Res<Assets<RecipeCatalogFile>>,
    quest_assets: Res<Assets<QuestCatalogFile>>,
    party_assets: Res<Assets<PartyCatalog>>,
    load: Res<FieldMenuCatalogLoad>,
    mut catalog: ResMut<FieldMenuCatalog>,
) {
    if catalog.status != CatalogStatus::Loading || load.field_use.is_none() || load.quests.is_none()
    {
        return;
    }
    let field_handle = load.field_use.as_ref().expect("checked above");
    let quest_handle = load.quests.as_ref().expect("checked above");
    if let Some(failure) = first_load_failure(&asset_server, &load, field_handle, quest_handle) {
        catalog.status = CatalogStatus::Failed;
        catalog.failure = Some(failure);
        return;
    }
    let assets = CatalogAssets {
        items: &item_assets,
        field_uses: &field_assets,
        classes: &class_assets,
        maps: &map_assets,
        tmx: &tmx_assets,
        recipes: &recipe_assets,
        quests: &quest_assets,
        party: &party_assets,
    };
    if !all_loaded(&load, &assets, field_handle, quest_handle) {
        return;
    }
    match build_catalog_parts(&load, &assets, field_handle, quest_handle) {
        Ok(parts) => publish_catalog(&mut catalog, parts),
        Err(failure) => {
            catalog.status = CatalogStatus::Failed;
            catalog.failure = Some(failure);
        }
    }
}

/// Borrowed views of every asset collection the catalog is built from.
struct CatalogAssets<'a> {
    items: &'a Assets<ItemCatalogFile>,
    field_uses: &'a Assets<FieldUseCatalogFile>,
    classes: &'a Assets<ClassDefinition>,
    maps: &'a Assets<MapMetadata>,
    tmx: &'a Assets<TmxGroundAsset>,
    recipes: &'a Assets<RecipeCatalogFile>,
    quests: &'a Assets<QuestCatalogFile>,
    party: &'a Assets<PartyCatalog>,
}

/// Every catalog field built from the loaded assets, held until all fallible steps succeed so a
/// failure never leaves a partially published catalog.
struct CatalogParts {
    items: BTreeMap<String, ItemDefinition>,
    item_order: Vec<String>,
    field_uses: BTreeMap<String, FieldUseDefinition>,
    classes: BTreeMap<String, ClassDefinition>,
    member_genders: BTreeMap<String, PartyMemberGender>,
    warp_destinations: Vec<WarpDestination>,
    maps: BTreeMap<String, MapMetadata>,
    recipes: Vec<RecipeDefinition>,
    quests: Vec<QuestDefinition>,
}

/// The failure text of the first failed load, scanning kinds in this order; first failure wins.
fn first_load_failure(
    asset_server: &AssetServer,
    load: &FieldMenuCatalogLoad,
    field_handle: &Handle<FieldUseCatalogFile>,
    quest_handle: &Handle<QuestCatalogFile>,
) -> Option<String> {
    item_load_failure(asset_server, &load.items)
        .or_else(|| class_load_failure(asset_server, &load.classes))
        .or_else(|| map_load_failure(asset_server, &load.maps))
        .or_else(|| service_map_load_failure(asset_server, &load.service_maps))
        .or_else(|| field_use_load_failure(asset_server, field_handle))
        .or_else(|| recipe_load_failure(asset_server, &load.recipes))
        .or_else(|| quest_load_failure(asset_server, quest_handle))
        .or_else(|| party_load_failure(asset_server, load.party.as_ref()))
}

fn item_load_failure(
    asset_server: &AssetServer,
    items: &[(String, Handle<ItemCatalogFile>)],
) -> Option<String> {
    for (path, handle) in items {
        if let LoadState::Failed(error) = asset_server.load_state(handle.id()) {
            return Some(format!("{path}: {error}"));
        }
    }
    None
}

fn class_load_failure(
    asset_server: &AssetServer,
    classes: &[(String, Handle<ClassDefinition>)],
) -> Option<String> {
    for (path, handle) in classes {
        if let LoadState::Failed(error) = asset_server.load_state(handle.id()) {
            return Some(format!("{path}: {error}"));
        }
    }
    None
}

fn map_load_failure(
    asset_server: &AssetServer,
    maps: &[(String, Handle<MapMetadata>, Handle<TmxGroundAsset>)],
) -> Option<String> {
    for (stem, metadata, tmx) in maps {
        if let LoadState::Failed(error) = asset_server.load_state(metadata.id()) {
            return Some(format!("map metadata for `{stem}`: {error}"));
        }
        if let LoadState::Failed(error) = asset_server.load_state(tmx.id()) {
            return Some(format!("map TMX for `{stem}`: {error}"));
        }
    }
    None
}

fn service_map_load_failure(
    asset_server: &AssetServer,
    service_maps: &[(String, Handle<MapMetadata>)],
) -> Option<String> {
    for (stem, metadata) in service_maps {
        if let LoadState::Failed(error) = asset_server.load_state(metadata.id()) {
            return Some(format!("map metadata for `{stem}`: {error}"));
        }
    }
    None
}

fn field_use_load_failure(
    asset_server: &AssetServer,
    field_handle: &Handle<FieldUseCatalogFile>,
) -> Option<String> {
    if let LoadState::Failed(error) = asset_server.load_state(field_handle.id()) {
        return Some(format!("field-use catalog: {error}"));
    }
    None
}

fn recipe_load_failure(
    asset_server: &AssetServer,
    recipes: &[(String, Handle<RecipeCatalogFile>)],
) -> Option<String> {
    for (path, handle) in recipes {
        if let LoadState::Failed(error) = asset_server.load_state(handle.id()) {
            return Some(format!("{path}: {error}"));
        }
    }
    None
}

fn quest_load_failure(
    asset_server: &AssetServer,
    quest_handle: &Handle<QuestCatalogFile>,
) -> Option<String> {
    if let LoadState::Failed(error) = asset_server.load_state(quest_handle.id()) {
        return Some(format!("quest catalog: {error}"));
    }
    None
}

fn party_load_failure(
    asset_server: &AssetServer,
    party_handle: Option<&Handle<PartyCatalog>>,
) -> Option<String> {
    if let Some(party_handle) = party_handle
        && let LoadState::Failed(error) = asset_server.load_state(party_handle.id())
    {
        return Some(format!("party catalog: {error}"));
    }
    None
}

/// Whether every requested asset has arrived. This order (field-use right after classes) differs
/// from [`first_load_failure`]'s on purpose: both are kept as they were authored, so do not unify
/// them.
fn all_loaded(
    load: &FieldMenuCatalogLoad,
    assets: &CatalogAssets,
    field_handle: &Handle<FieldUseCatalogFile>,
    quest_handle: &Handle<QuestCatalogFile>,
) -> bool {
    !(load
        .items
        .iter()
        .any(|(_, handle)| assets.items.get(handle).is_none())
        || load
            .classes
            .iter()
            .any(|(_, handle)| assets.classes.get(handle).is_none())
        || assets.field_uses.get(field_handle).is_none()
        || load.maps.iter().any(|(_, metadata, tmx)| {
            assets.maps.get(metadata).is_none() || assets.tmx.get(tmx).is_none()
        })
        || load
            .service_maps
            .iter()
            .any(|(_, metadata)| assets.maps.get(metadata).is_none())
        || load
            .recipes
            .iter()
            .any(|(_, handle)| assets.recipes.get(handle).is_none())
        || assets.quests.get(quest_handle).is_none()
        || load
            .party
            .as_ref()
            .is_some_and(|handle| assets.party.get(handle).is_none()))
}

/// Builds every catalog field, in order; the only fallible step is the warp destination list.
fn build_catalog_parts(
    load: &FieldMenuCatalogLoad,
    assets: &CatalogAssets,
    field_handle: &Handle<FieldUseCatalogFile>,
    quest_handle: &Handle<QuestCatalogFile>,
) -> Result<CatalogParts, String> {
    let (items, item_order) = build_items(load, assets);
    let field_uses = build_field_uses(assets, field_handle);
    let classes = build_classes(load, assets);
    let (maps, warp_destinations) = build_maps_and_warp_destinations(load, assets)?;
    Ok(CatalogParts {
        items,
        item_order,
        field_uses,
        classes,
        member_genders: build_member_genders(load, assets),
        warp_destinations,
        maps,
        recipes: build_recipes(load, assets),
        quests: build_quests(assets, quest_handle),
    })
}

fn build_items(
    load: &FieldMenuCatalogLoad,
    assets: &CatalogAssets,
) -> (BTreeMap<String, ItemDefinition>, Vec<String>) {
    let mut items = BTreeMap::new();
    let mut item_order = Vec::new();
    for (_, handle) in &load.items {
        for item in assets.items.get(handle).expect("all checked").entries() {
            item_order.push(item.id().to_owned());
            items.insert(item.id().to_owned(), item.clone());
        }
    }
    crate::forge::expand_catalog(&mut items, &mut item_order);
    (items, item_order)
}

fn build_field_uses(
    assets: &CatalogAssets,
    field_handle: &Handle<FieldUseCatalogFile>,
) -> BTreeMap<String, FieldUseDefinition> {
    assets
        .field_uses
        .get(field_handle)
        .expect("checked")
        .entries()
        .iter()
        .map(|entry| (entry.id().to_owned(), entry.clone()))
        .collect()
}

fn build_classes(
    load: &FieldMenuCatalogLoad,
    assets: &CatalogAssets,
) -> BTreeMap<String, ClassDefinition> {
    load.classes
        .iter()
        .map(|(_, handle)| {
            let class = assets.classes.get(handle).expect("all checked").clone();
            (class.class_id.clone(), class)
        })
        .collect()
}

/// Map metadata keyed by effective id (TMX-backed maps, then service maps), plus the warp
/// destinations computed from the TMX-backed maps' portals.
fn build_maps_and_warp_destinations(
    load: &FieldMenuCatalogLoad,
    assets: &CatalogAssets,
) -> Result<(BTreeMap<String, MapMetadata>, Vec<WarpDestination>), String> {
    let mut maps = BTreeMap::new();
    let mut warp_sources = Vec::with_capacity(load.maps.len());
    for (stem, metadata_handle, tmx_handle) in &load.maps {
        let metadata = assets.maps.get(metadata_handle).expect("all checked");
        maps.insert(metadata.effective_id(stem).to_owned(), metadata.clone());
        let tmx = assets.tmx.get(tmx_handle).expect("all checked");
        // A malformed `portals` layer is a TMX-authoring concern outside this catalog's scope;
        // treat it as "no outgoing portals from this source" rather than failing catalog load.
        let portals = runtime_portals(tmx.document()).unwrap_or_default();
        warp_sources.push((stem.as_str(), metadata, portals));
    }
    for (stem, metadata_handle) in &load.service_maps {
        let metadata = assets.maps.get(metadata_handle).expect("all checked");
        maps.insert(metadata.effective_id(stem).to_owned(), metadata.clone());
    }
    let warp_sources_ref: Vec<(&str, &MapMetadata, &[RuntimePortal])> = warp_sources
        .iter()
        .map(|(stem, metadata, portals)| (*stem, *metadata, portals.as_slice()))
        .collect();
    let warp_destinations =
        compute_warp_destinations(&warp_sources_ref).map_err(|error| error.to_string())?;
    Ok((maps, warp_destinations))
}

fn build_member_genders(
    load: &FieldMenuCatalogLoad,
    assets: &CatalogAssets,
) -> BTreeMap<String, PartyMemberGender> {
    load.party
        .as_ref()
        .and_then(|handle| assets.party.get(handle))
        .map(|party| {
            party
                .party
                .iter()
                .map(|member| (member.data().id.clone(), member.data().gender))
                .collect()
        })
        .unwrap_or_default()
}

fn build_recipes(load: &FieldMenuCatalogLoad, assets: &CatalogAssets) -> Vec<RecipeDefinition> {
    load.recipes
        .iter()
        .flat_map(|(_, handle)| {
            assets
                .recipes
                .get(handle)
                .expect("checked")
                .entries()
                .iter()
                .cloned()
        })
        .collect()
}

fn build_quests(
    assets: &CatalogAssets,
    quest_handle: &Handle<QuestCatalogFile>,
) -> Vec<QuestDefinition> {
    assets
        .quests
        .get(quest_handle)
        .expect("checked")
        .entries()
        .to_vec()
}

/// Publishes the built parts in the original assignment order, clearing the failure and marking
/// the catalog ready last.
fn publish_catalog(catalog: &mut FieldMenuCatalog, parts: CatalogParts) {
    let CatalogParts {
        items,
        item_order,
        field_uses,
        classes,
        member_genders,
        warp_destinations,
        maps,
        recipes,
        quests,
    } = parts;
    catalog.items = items;
    catalog.item_order = item_order;
    catalog.field_uses = field_uses;
    catalog.classes = classes;
    catalog.member_genders = member_genders;
    catalog.warp_destinations = warp_destinations;
    catalog.maps = maps;
    catalog.recipes = recipes;
    catalog.quests = quests;
    catalog.failure = None;
    catalog.status = CatalogStatus::Ready;
}
