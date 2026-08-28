//! The manifest-declared keeper faces and recipe-status icons the service overlays draw.
//!
//! `manifest.yaml` has named a sprite for every service since the content was authored, and the
//! apothecary additionally names three lock-status icons. Nothing consumed them while the
//! overlays were a text blob. Handles are loaded once at startup rather than when a shop opens,
//! so the atlas is already resident the first time the player talks to a keeper — an overlay
//! that only rebuilds on state change would otherwise miss a late-arriving asset.

use std::collections::BTreeMap;

use bevy::{image::TextureAtlas, prelude::*};

use crate::{
    scenario_inventory::ScenarioInventory, scenario_root::ScenarioRoot,
    service_domain::RecipeAvailability, tsx_atlas_asset::TsxAtlasAsset,
};

/// The tileset-local frame a keeper face uses: the idle `Down` row, column zero.
///
/// Source `SpriteSheet.load_npc_face` (`engine/world/sprite_sheet.py:145`) takes exactly this
/// frame, and the four-direction character profile puts the `Down` row's owner tile at 18.
const KEEPER_FACE_TILE: u32 = 18;

/// Which service's keeper is being drawn.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum ServiceKeeper {
    Apothecary,
    Inn,
    ItemShop,
    WeaponShop,
    ArmorShop,
}

/// The three authored recipe-status icons.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum RecipeIcon {
    Locked,
    Ready,
    Missing,
}

impl RecipeIcon {
    /// Collapses the six modelled recipe states onto the three authored icons.
    ///
    /// The source only distinguishes locked, missing-inputs, and ready. The port models two more
    /// refusals — a unique output already owned, and a full output stack — and both are states
    /// the recipe can never be crafted out of by gathering more, so they read as locked.
    pub(crate) const fn for_availability(availability: RecipeAvailability) -> Self {
        match availability {
            RecipeAvailability::Ready => Self::Ready,
            RecipeAvailability::MissingInputs | RecipeAvailability::Unaffordable => Self::Missing,
            RecipeAvailability::Locked
            | RecipeAvailability::UniqueOwned
            | RecipeAvailability::OutputCap => Self::Locked,
        }
    }
}

#[derive(Debug, Default, Resource)]
pub(crate) struct ServiceSprites {
    keepers: BTreeMap<ServiceKeeper, Handle<TsxAtlasAsset>>,
    icons: BTreeMap<RecipeIcon, Handle<Image>>,
}

impl ServiceSprites {
    /// Builds the UI image for one keeper's face, or `None` until its atlas finishes loading.
    pub(crate) fn keeper_face(
        &self,
        keeper: ServiceKeeper,
        atlases: &Assets<TsxAtlasAsset>,
    ) -> Option<ImageNode> {
        let atlas = atlases.get(self.keepers.get(&keeper)?)?;
        if KEEPER_FACE_TILE >= atlas.metadata().tile_count() {
            return None;
        }
        Some(ImageNode::from_atlas_image(
            atlas.image().clone(),
            TextureAtlas {
                layout: atlas.layout().clone(),
                index: KEEPER_FACE_TILE as usize,
            },
        ))
    }

    pub(crate) fn recipe_icon(&self, icon: RecipeIcon) -> Option<Handle<Image>> {
        self.icons.get(&icon).cloned()
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
    for (icon, path) in [
        (RecipeIcon::Locked, &art.recipe_locked_icon),
        (RecipeIcon::Ready, &art.recipe_ready_icon),
        (RecipeIcon::Missing, &art.recipe_missing_icon),
    ] {
        if let Some(path) = path {
            sprites
                .icons
                .insert(icon, asset_server.load(root.resolve(path)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_every_recipe_state_onto_an_authored_icon() {
        for (availability, expected) in [
            (RecipeAvailability::Ready, RecipeIcon::Ready),
            (RecipeAvailability::MissingInputs, RecipeIcon::Missing),
            (RecipeAvailability::Unaffordable, RecipeIcon::Missing),
            (RecipeAvailability::Locked, RecipeIcon::Locked),
            (RecipeAvailability::UniqueOwned, RecipeIcon::Locked),
            (RecipeAvailability::OutputCap, RecipeIcon::Locked),
        ] {
            assert_eq!(RecipeIcon::for_availability(availability), expected);
        }
    }
}
