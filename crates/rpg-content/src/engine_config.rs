//! Engine-level settings read from `assets/settings.yaml`.
//!
//! The source keeps these in `engine/settings/settings.yaml`, loaded by
//! `engine/settings/engine_config_data.py`. This is a deliberate subset: the port carries only the
//! keys it actually consumes, because a parsed-but-unread setting is exactly the kind of
//! aspirational surface `docs/adr/0007-inherited-scenario-data-debt.md` had to clean up later.
//!
//! Keys the source has that the port governs elsewhere, and therefore does not read here:
//!
//! | Source key | Where the port decides it |
//! | --- | --- |
//! | `display.*` | `docs/gameplay-canvas-policy.md` fixes the canvas at 1280x766 |
//! | `tiles.tile_size` | derived from each map's own TMX header |
//! | `saves.dir` | `save_store::resolve_save_directory`, which follows platform conventions |
//! | `audio.*` | the `RPG_S1_MUTE_AUDIO` environment variable |
//! | `debug.*` | `--party-preset` and `RPG_S1_DEBUG_COLLISION` |
//! | `movement.smooth_collision` | always on; the port has no non-sliding collision path |
//! | `enemy_spawn.global_interval` | per-encounter data in the scenario package |
//!
//! A missing file, block, or key falls back to the value the port already used, so the game runs
//! unchanged without a settings file rather than refusing to start over a display preference.

use serde::Deserialize;

use crate::scenario_yaml;

/// The source's three typewriter speeds (`dialogue_scene.py::TEXT_SPEEDS`).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextSpeed {
    Slow,
    #[default]
    Fast,
    VeryFast,
}

impl TextSpeed {
    /// Characters revealed per second, or `None` for the source's instant reveal.
    pub const fn chars_per_second(self) -> Option<f32> {
        match self {
            Self::Slow => Some(20.0),
            Self::Fast => Some(60.0),
            Self::VeryFast => None,
        }
    }
}

/// The source's four named font sizes (`fonts.sizes`).
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(default)]
pub struct FontSizes {
    pub small: f32,
    pub medium: f32,
    pub large: f32,
    pub xlarge: f32,
}

/// The source's shipped values, which are also the port's baseline.
impl Default for FontSizes {
    fn default() -> Self {
        Self {
            small: 20.0,
            medium: 24.0,
            large: 28.0,
            xlarge: 32.0,
        }
    }
}

impl FontSizes {
    /// How far this type scale departs from the shipped one.
    ///
    /// The port has named type roles on `UiTheme` rather than the source's four-entry palette, so
    /// the setting is honored as a multiplier on those roles. At the shipped sizes this is exactly
    /// `1.0` and no text moves.
    pub fn scale(self) -> f32 {
        let baseline = Self::default().medium;
        if self.medium > 0.0 && baseline > 0.0 {
            self.medium / baseline
        } else {
            1.0
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize)]
#[serde(default)]
struct DialogueSettings {
    text_speed: TextSpeed,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(default)]
struct ItemSettings {
    use_aoe_confirm: bool,
}

impl Default for ItemSettings {
    fn default() -> Self {
        Self {
            use_aoe_confirm: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(default)]
struct ShopSettings {
    mc_exchange_confirm_large: bool,
}

impl Default for ShopSettings {
    fn default() -> Self {
        Self {
            mc_exchange_confirm_large: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(default)]
struct FontSettings {
    sizes: FontSizes,
}

/// The settings file's shape, mirroring the source's block names.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(default)]
struct SettingsDocument {
    dialogue: DialogueSettings,
    item: ItemSettings,
    shop: ShopSettings,
    fonts: FontSettings,
}

/// Engine settings in the form the game reads them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EngineConfig {
    /// How fast dialogue lines type themselves out.
    pub text_speed: TextSpeed,
    /// Multiplier on `UiTheme`'s named font sizes.
    pub font_scale: f32,
    /// Ask before an item that targets the whole party is spent.
    pub use_aoe_confirm: bool,
    /// Ask before exchanging an L or XL magic core.
    pub mc_exchange_confirm_large: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self::from_document(SettingsDocument::default())
    }
}

impl EngineConfig {
    fn from_document(document: SettingsDocument) -> Self {
        Self {
            text_speed: document.dialogue.text_speed,
            font_scale: document.fonts.sizes.scale(),
            use_aoe_confirm: document.item.use_aoe_confirm,
            mc_exchange_confirm_large: document.shop.mc_exchange_confirm_large,
        }
    }

    /// Reads `settings.yaml` from an asset base.
    ///
    /// An absent file is the documented way to run on the defaults, so it is not an error. A file
    /// that exists but cannot be read or parsed is returned as one -- silently ignoring a settings
    /// file the player edited would be worse than saying so.
    pub fn load(asset_base: &std::path::Path) -> Result<Self, EngineConfigError> {
        let path = asset_base.join("settings.yaml");
        match std::fs::read_to_string(&path) {
            Ok(contents) => Self::from_yaml(&contents)
                .map_err(|error| EngineConfigError(format!("{}: {error}", path.display()))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(EngineConfigError(format!("{}: {error}", path.display()))),
        }
    }

    /// Parses settings text, falling back to the defaults for anything it does not set.
    ///
    /// Unknown keys are ignored rather than rejected: the shipped file carries the source's full
    /// block set so it stays diffable against the original, and only the consumed subset is read.
    pub fn from_yaml(contents: &str) -> Result<Self, EngineConfigError> {
        let document = scenario_yaml::from_str::<Option<SettingsDocument>>(contents)
            .map_err(|error| EngineConfigError(error.to_string()))?
            .unwrap_or_default();
        Ok(Self::from_document(document))
    }
}

/// The settings file exists but could not be understood.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EngineConfigError(pub String);

impl std::fmt::Display for EngineConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for EngineConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_settings_file_parses_to_the_ports_current_behavior() {
        let config = EngineConfig::from_yaml(include_str!("../../../assets/settings.yaml"))
            .expect("the shipped settings file must parse");

        assert_eq!(config.text_speed, TextSpeed::Fast);
        assert!(config.use_aoe_confirm);
        assert!(config.mc_exchange_confirm_large);
        assert_eq!(
            config.font_scale, 1.0,
            "the shipped type scale must move no text"
        );
    }

    #[test]
    fn an_empty_or_absent_document_keeps_every_default() {
        assert_eq!(
            EngineConfig::from_yaml("").unwrap(),
            EngineConfig::default()
        );
        assert_eq!(
            EngineConfig::from_yaml("# only a comment\n").unwrap(),
            EngineConfig::default()
        );
    }

    #[test]
    fn each_setting_is_read_from_its_source_block() {
        let config = EngineConfig::from_yaml(
            "dialogue:\n  text_speed: slow\nitem:\n  use_aoe_confirm: false\n\
             shop:\n  mc_exchange_confirm_large: false\nfonts:\n  sizes:\n    medium: 12\n",
        )
        .unwrap();

        assert_eq!(config.text_speed, TextSpeed::Slow);
        assert!(!config.use_aoe_confirm);
        assert!(!config.mc_exchange_confirm_large);
        assert_eq!(config.font_scale, 0.5);
    }

    #[test]
    fn a_block_the_port_does_not_consume_is_ignored_not_rejected() {
        // The shipped file keeps the source's full block set so the two stay diffable.
        let config = EngineConfig::from_yaml(
            "display:\n  screen_width: 1280\nsaves:\n  dir: ~/user_save_data\n\
             debug:\n  collision: true\ndialogue:\n  text_speed: very_fast\n",
        )
        .expect("unconsumed source blocks must not fail the load");

        assert_eq!(config.text_speed, TextSpeed::VeryFast);
    }

    #[test]
    fn text_speeds_match_the_sources_reveal_rates() {
        assert_eq!(TextSpeed::Slow.chars_per_second(), Some(20.0));
        assert_eq!(TextSpeed::Fast.chars_per_second(), Some(60.0));
        assert_eq!(
            TextSpeed::VeryFast.chars_per_second(),
            None,
            "the source treats 0 as instant, not as a stalled reveal"
        );
    }

    #[test]
    fn a_malformed_document_is_an_error_rather_than_a_silent_default() {
        assert!(EngineConfig::from_yaml("dialogue:\n  text_speed: turbo\n").is_err());
        assert!(EngineConfig::from_yaml("dialogue: [not, a, block]\n").is_err());
    }

    #[test]
    fn an_absent_settings_file_is_the_defaults_but_an_unreadable_one_is_an_error() {
        let base = std::env::temp_dir().join(format!(
            "rpg-s1-engine-config-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&base).unwrap();

        assert_eq!(
            EngineConfig::load(&base).unwrap(),
            EngineConfig::default(),
            "running without a settings file is supported, not a failure"
        );

        std::fs::write(
            base.join("settings.yaml"),
            "dialogue:\n  text_speed: turbo\n",
        )
        .unwrap();
        let error = EngineConfig::load(&base).expect_err("a broken file must be reported");
        assert!(
            error.0.contains("settings.yaml"),
            "the error must name the file the player edited: {error}"
        );

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn a_zero_or_missing_medium_size_cannot_scale_text_to_nothing() {
        assert_eq!(
            EngineConfig::from_yaml("fonts:\n  sizes:\n    medium: 0\n")
                .unwrap()
                .font_scale,
            1.0
        );
        assert_eq!(
            EngineConfig::from_yaml("fonts:\n  sizes:\n    small: 8\n")
                .unwrap()
                .font_scale,
            1.0,
            "an unset medium keeps the baseline scale"
        );
    }
}
