//! Typed schema slices for the scenario manifest.
//!
//! Each milestone adds only the source fields it owns. Serde's default unknown-field
//! behavior is intentional: a partial schema must be able to load the pinned complete
//! manifest while its other sections are introduced by later milestones.

use crate::scenario_path::ScenarioRelativePath;
use serde::Deserialize;

/// The manifest fields that identify scenario content and label its game window.
///
/// `id` and `version` are content identity rather than the selected package key; see
/// ADR 0004. The fields are source-authored strings so their exact values remain available
/// to save, cache, recording, and UI systems that are added later.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ManifestIdentityWindow {
    pub id: String,
    pub name: String,
    pub version: String,
    pub window_title: String,
}

/// The source manifest fields that select the title presentation assets.
///
/// This is a partial manifest adapter alongside [`ManifestIdentityWindow`]: both deserialize
/// the same document, while later milestones add their own owned slices. All paths remain
/// scenario-relative so loading and validation share ADR 0002 and ADR 0004 containment rules.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ManifestTitleFontUi {
    pub title: ManifestTitle,
    pub font: ManifestFont,
    pub ui: ManifestUi,
}

/// Title-specific assets selected by the scenario.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ManifestTitle {
    pub image: ScenarioRelativePath,
    pub cursor_icon: ScenarioRelativePath,
}

/// The scenario-selected display font.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ManifestFont {
    pub path: ScenarioRelativePath,
}

/// Shared field-menu presentation assets.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ManifestUi {
    pub menu_backdrop: ScenarioRelativePath,
}

#[cfg(test)]
mod tests {
    use super::{
        ManifestFont, ManifestIdentityWindow, ManifestTitle, ManifestTitleFontUi, ManifestUi,
    };
    use crate::scenario_yaml;

    #[test]
    fn loads_pinned_identity_and_window_values_from_complete_manifest_shape() {
        let manifest: ManifestIdentityWindow = scenario_yaml::from_str(include_str!(
            "../tests/fixtures/rusted-kingdoms-manifest-identity-window.yaml"
        ))
        .expect("the pinned manifest identity/window slice should deserialize");

        assert_eq!(
            manifest,
            ManifestIdentityWindow {
                id: "my_rpg_story".to_owned(),
                name: "Chronicles of the Lost Flame".to_owned(),
                version: "1.0.0".to_owned(),
                window_title: "Rusted Kingdoms".to_owned(),
            }
        );
    }

    #[test]
    fn loads_title_font_and_ui_paths_with_the_repaired_cursor_reference() {
        let manifest: ManifestTitleFontUi = scenario_yaml::from_str(include_str!(
            "../tests/fixtures/rusted-kingdoms-manifest-title-font-ui.yaml"
        ))
        .expect("the title/font/UI slice should deserialize");

        assert_eq!(
            manifest,
            ManifestTitleFontUi {
                title: ManifestTitle {
                    image: "assets/images/title_bg/title_lost_flame.webp"
                        .try_into()
                        .unwrap(),
                    cursor_icon: "assets/images/icons/arrow-head-right.webp"
                        .try_into()
                        .unwrap(),
                },
                font: ManifestFont {
                    path: "assets/fonts/Philosopher-Regular.ttf".try_into().unwrap(),
                },
                ui: ManifestUi {
                    menu_backdrop: "assets/images/battle_bg/zone4-sanctum-bg-1280x468.webp"
                        .try_into()
                        .unwrap(),
                },
            }
        );
    }
}
