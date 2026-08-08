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

/// The source manifest fields selecting sprites for field services and item boxes.
///
/// This is a partial manifest adapter alongside the presentation and identity adapters.
/// The apothecary is the only current service with nested icon assets; retaining that
/// structure prevents a later service implementation from guessing icon ownership.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ManifestServiceSprites {
    pub apothecary: ManifestApothecary,
    pub inn: ManifestServiceSprite,
    pub item_shop: ManifestServiceSprite,
    pub weapon_shop: ManifestServiceSprite,
    pub armor_shop: ManifestServiceSprite,
    pub item_box: ManifestServiceSprite,
}

/// Apothecary presentation assets, including its source-authored lock-status icons.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ManifestApothecary {
    pub sprite: ScenarioRelativePath,
    pub icons: ManifestApothecaryIcons,
}

/// The three source-authored apothecary icon states.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ManifestApothecaryIcons {
    pub locked: ScenarioRelativePath,
    pub ready: ScenarioRelativePath,
    pub missing: ScenarioRelativePath,
}

/// A service or item-box sprite selected by the manifest.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ManifestServiceSprite {
    pub sprite: ScenarioRelativePath,
}

/// The manifest fields that identify the default protagonist and new-game start.
///
/// This is a partial manifest adapter alongside the other manifest slices. `id`,
/// `class`, and `map` are scenario identifiers rather than paths; only the sprite
/// and intro dialogue values name files beneath the scenario root. `position`
/// preserves the source `[x, y]` scalar sequence for this manifest-owned slice.
/// M2.11 will replace that temporary representation with the shared position type.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ManifestProtagonistStart {
    pub protagonist: ManifestProtagonist,
    pub start: ManifestStart,
}

/// The source-authored identity and field-sprite selection for the protagonist.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ManifestProtagonist {
    pub id: String,
    pub name: String,
    pub class: String,
    pub sprite: ScenarioRelativePath,
}

/// The authored new-game map location and opening dialogue.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ManifestStart {
    pub map: String,
    pub position: [i32; 2],
    pub intro_dialogue: ScenarioRelativePath,
}

#[cfg(test)]
mod tests {
    use super::{
        ManifestApothecary, ManifestApothecaryIcons, ManifestFont, ManifestIdentityWindow,
        ManifestProtagonist, ManifestProtagonistStart, ManifestServiceSprite,
        ManifestServiceSprites, ManifestStart, ManifestTitle, ManifestTitleFontUi, ManifestUi,
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

    #[test]
    fn loads_pinned_service_sprites_and_apothecary_icon_states() {
        let manifest: ManifestServiceSprites = scenario_yaml::from_str(include_str!(
            "../tests/fixtures/rusted-kingdoms-manifest-service-sprites.yaml"
        ))
        .expect("the pinned manifest service-sprite slice should deserialize");

        assert_eq!(
            manifest,
            ManifestServiceSprites {
                apothecary: ManifestApothecary {
                    sprite: "assets/sprites/npc/female_wiz_01.tsx".try_into().unwrap(),
                    icons: ManifestApothecaryIcons {
                        locked: "assets/images/icons/lock-locked-red-small.webp"
                            .try_into()
                            .unwrap(),
                        ready: "assets/images/icons/lock-unlocked-green-small.webp"
                            .try_into()
                            .unwrap(),
                        missing: "assets/images/icons/lock-unlocked-yellow-small.webp"
                            .try_into()
                            .unwrap(),
                    },
                },
                inn: ManifestServiceSprite {
                    sprite: "assets/sprites/npc/female_blue_01.tsx".try_into().unwrap(),
                },
                item_shop: ManifestServiceSprite {
                    sprite: "assets/sprites/npc/teen_halfmessy_01.tsx"
                        .try_into()
                        .unwrap(),
                },
                weapon_shop: ManifestServiceSprite {
                    sprite: "assets/sprites/npc/male_sword_fighter_axe_fighter.tsx"
                        .try_into()
                        .unwrap(),
                },
                armor_shop: ManifestServiceSprite {
                    sprite: "assets/sprites/npc/plate_knight_base.tsx"
                        .try_into()
                        .unwrap(),
                },
                item_box: ManifestServiceSprite {
                    sprite: "assets/sprites/objects/item_box.tsx".try_into().unwrap(),
                },
            }
        );
    }

    #[test]
    fn loads_pinned_protagonist_and_new_game_start_values() {
        let manifest: ManifestProtagonistStart = scenario_yaml::from_str(include_str!(
            "../tests/fixtures/rusted-kingdoms-manifest-protagonist-start.yaml"
        ))
        .expect("the pinned manifest protagonist/start slice should deserialize");

        assert_eq!(
            manifest,
            ManifestProtagonistStart {
                protagonist: ManifestProtagonist {
                    id: "aric".to_owned(),
                    name: "Aric".to_owned(),
                    class: "hero".to_owned(),
                    sprite: "assets/sprites/party/01_aric_walk.tsx".try_into().unwrap(),
                },
                start: ManifestStart {
                    map: "town_01_ardel".to_owned(),
                    position: [14, 5],
                    intro_dialogue: "data/dialogue/intro_cutscene.yaml".try_into().unwrap(),
                },
            }
        );
    }
}
