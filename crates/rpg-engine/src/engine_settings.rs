//! The engine settings file, as a Bevy resource.
//!
//! [`crate::engine_config::EngineConfig`] lives in the content crate, which knows nothing about
//! Bevy, so this is the thin newtype that carries it through the app. Loading is a plain
//! filesystem read at startup, like `ScenarioInventory::discover`, rather than an asset load: the
//! settings decide how the first frame is drawn, so they cannot arrive a few frames later.

use bevy::prelude::*;

use crate::engine_config::EngineConfig;

#[derive(Resource, Clone, Copy, Debug, Default, Deref, PartialEq)]
pub(crate) struct EngineSettings(pub(crate) EngineConfig);

impl EngineSettings {
    /// Reads the settings beside the game's assets.
    ///
    /// A settings file that cannot be parsed is reported and then ignored. Refusing to launch
    /// over a display preference would be a worse outcome than starting on the defaults, but
    /// changing the file and seeing nothing happen with no explanation would be worse still.
    pub(crate) fn load(asset_base: &std::path::Path) -> Self {
        match EngineConfig::load(asset_base) {
            Ok(config) => Self(config),
            Err(error) => {
                error!("settings ignored, using defaults: {error}");
                Self::default()
            }
        }
    }
}
