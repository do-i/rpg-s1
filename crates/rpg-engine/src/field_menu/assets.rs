//! Scenario assets the field menu loads: its font, status portraits, and party walk sheets.

use bevy::prelude::*;

use super::ui::{large_status_portrait_path, load_status_image, profile_portrait_path};
use crate::{
    game_state::GameState, scenario_inventory::ScenarioInventory, scenario_root::ScenarioRoot,
    tsx_atlas_asset::TsxAtlasAsset,
};

pub(super) fn scenario_font(
    asset_server: &AssetServer,
    root: &ScenarioRoot,
    inventory: &ScenarioInventory,
) -> Option<Handle<Font>> {
    inventory
        .font
        .as_ref()
        .map(|path| asset_server.load(root.resolve(path)))
}

#[derive(Default)]
pub(super) struct StatusPortraitAssets {
    profiles: std::collections::BTreeMap<String, Handle<Image>>,
    large: std::collections::BTreeMap<String, Handle<Image>>,
}

impl StatusPortraitAssets {
    pub(super) fn load(asset_server: &AssetServer, root: &ScenarioRoot, game: &GameState) -> Self {
        let mut portraits = Self::default();
        for member in game.party().members() {
            if let Some(handle) =
                load_status_image(asset_server, root, &profile_portrait_path(member.id()))
            {
                portraits.profiles.insert(member.id().to_owned(), handle);
            }
            if let Some(handle) =
                load_status_image(asset_server, root, &large_status_portrait_path(member.id()))
            {
                portraits.large.insert(member.id().to_owned(), handle);
            }
        }
        portraits
    }

    pub(super) fn profile(&self, member_id: &str) -> Option<Handle<Image>> {
        self.profiles.get(member_id).cloned()
    }

    pub(super) fn large(&self, member_id: &str) -> Option<Handle<Image>> {
        self.large.get(member_id).cloned()
    }
}

/// Tile row holding the downward-facing idle frame on a cardinal walk sheet.
pub(super) const WALK_SHEET_IDLE_DOWN_TILE: usize = 18;

/// Strong handles to each party member's walk sheet, kept so the switch overlay can draw a real
/// sprite frame instead of a lettered placeholder.
#[derive(Debug, Default, Resource)]
pub(super) struct PartyWalkSheets {
    sheets: Vec<(String, Handle<TsxAtlasAsset>)>,
    variants: Vec<(String, Handle<TsxAtlasAsset>)>,
}

impl PartyWalkSheets {
    pub(super) fn variant(&self, id: &str) -> Option<&Handle<TsxAtlasAsset>> {
        self.variants
            .iter()
            .find(|(key, _)| key == id)
            .map(|(_, handle)| handle)
    }
    pub(super) fn sheet(&self, member_id: &str) -> Option<&Handle<TsxAtlasAsset>> {
        self.sheets
            .iter()
            .find(|(id, _)| id == member_id)
            .map(|(_, handle)| handle)
    }
}

pub(super) fn load_party_walk_sheets(
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    inventory: Res<ScenarioInventory>,
    mut sheets: ResMut<PartyWalkSheets>,
) {
    if !sheets.sheets.is_empty() {
        return;
    }
    sheets.sheets = inventory
        .party_sprites
        .iter()
        .map(|(member_id, path)| (member_id.clone(), asset_server.load(root.resolve(path))))
        .collect();
    sheets.variants = inventory
        .protagonist_sprite_variants
        .iter()
        .map(|variant| {
            (
                variant.id.clone(),
                asset_server.load(root.resolve(&variant.sprite)),
            )
        })
        .collect();
}
