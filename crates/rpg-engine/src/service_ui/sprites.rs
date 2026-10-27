//! Manifest-declared keeper faces for the service overlays.
//!
//! Faces are preloaded so opening a service can reuse the shared dialogue portrait crop.

use std::collections::BTreeMap;

use bevy::prelude::*;

use crate::{
    dialogue_portrait::{DialoguePortrait, dialogue_portrait_from_atlas},
    scenario_inventory::ScenarioInventory,
    scenario_root::ScenarioRoot,
    tsx_atlas_asset::TsxAtlasAsset,
};

/// Which service's keeper is being drawn.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum ServiceKeeper {
    Apothecary,
    Inn,
    ItemShop,
    WeaponShop,
    ArmorShop,
}

#[derive(Debug, Default, Resource)]
pub(crate) struct ServiceSprites {
    keepers: BTreeMap<ServiceKeeper, Handle<TsxAtlasAsset>>,
}

impl ServiceSprites {
    /// Builds the shared shoulder-up popup portrait, or `None` until its atlas finishes loading.
    pub(crate) fn keeper_face(
        &self,
        keeper: ServiceKeeper,
        atlases: &Assets<TsxAtlasAsset>,
    ) -> Option<DialoguePortrait> {
        let atlas = atlases.get(self.keepers.get(&keeper)?)?;
        dialogue_portrait_from_atlas(atlas)
    }
}

pub(crate) fn load_service_sprites(
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
    mut sprites: ResMut<ServiceSprites>,
) {
    let art = &inventory.service_art;
    for (keeper, path) in [
        (ServiceKeeper::Apothecary, &art.apothecary_keeper),
        (ServiceKeeper::Inn, &art.inn_keeper),
        (ServiceKeeper::ItemShop, &art.item_shop_keeper),
        (ServiceKeeper::WeaponShop, &art.weapon_shop_keeper),
        (ServiceKeeper::ArmorShop, &art.armor_shop_keeper),
    ] {
        if let Some(path) = path {
            sprites
                .keepers
                .insert(keeper, asset_server.load(root.resolve(path)));
        }
    }
}
