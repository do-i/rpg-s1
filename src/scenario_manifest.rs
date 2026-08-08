//! Typed schema slices for the scenario manifest.
//!
//! Each milestone adds only the source fields it owns. Serde's default unknown-field
//! behavior is intentional: a partial schema must be able to load the pinned complete
//! manifest while its other sections are introduced by later milestones.

use bevy::{asset::Asset, reflect::TypePath};

use crate::scenario_path::{ScenarioRelativePath, ScenarioRelativePathError};
use crate::scenario_spatial::Position;
use crate::scenario_yaml::{deserialize_string, deserialize_strings};
use serde::{Deserialize, Deserializer};
use std::fmt;

/// The complete known manifest schema for the pinned Rusted Kingdoms source snapshot.
///
/// This composes the existing leaf types from M2.03 through M2.07 without duplicating their
/// nested field schemas. It intentionally does not flatten the partial adapters: Serde flatten
/// loses the parent segment of a missing nested field, which would make required-field errors
/// report `cursor_icon` instead of `title.cursor_icon`. `signs` remains accepted for its future
/// owning milestone; strict unknown-field rejection is added once every pinned field has an
/// owner.
#[derive(Asset, Clone, Debug, Deserialize, Eq, PartialEq, TypePath)]
pub struct Manifest {
    #[serde(deserialize_with = "deserialize_string")]
    pub id: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub name: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub version: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub window_title: String,
    pub title: ManifestTitle,
    pub font: ManifestFont,
    pub ui: ManifestUi,
    pub apothecary: ManifestApothecary,
    pub inn: ManifestServiceSprite,
    pub item_shop: ManifestServiceSprite,
    pub weapon_shop: ManifestServiceSprite,
    pub armor_shop: ManifestServiceSprite,
    pub item_box: ManifestServiceSprite,
    pub protagonist: ManifestProtagonist,
    pub start: ManifestStart,
    #[serde(deserialize_with = "deserialize_strings")]
    pub bootstrap_flags: Vec<String>,
    #[serde(deserialize_with = "deserialize_strings")]
    pub engine_managed_flags: Vec<String>,
    pub refs: ManifestRefs,
}

/// The manifest fields that identify scenario content and label its game window.
///
/// `id` and `version` are content identity rather than the selected package key; see
/// ADR 0004. The fields are source-authored strings so their exact values remain available
/// to save, cache, recording, and UI systems that are added later.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ManifestIdentityWindow {
    #[serde(deserialize_with = "deserialize_string")]
    pub id: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub name: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub version: String,
    #[serde(deserialize_with = "deserialize_string")]
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
/// and intro dialogue values name files beneath the scenario root. `position` preserves the
/// source `[x, y]` scalar sequence through the shared [`Position`] type.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ManifestProtagonistStart {
    pub protagonist: ManifestProtagonist,
    pub start: ManifestStart,
}

/// The source-authored identity and field-sprite selection for the protagonist.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ManifestProtagonist {
    #[serde(deserialize_with = "deserialize_string")]
    pub id: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub name: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub class: String,
    pub sprite: ScenarioRelativePath,
}

/// The authored new-game map location and opening dialogue.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ManifestStart {
    #[serde(deserialize_with = "deserialize_string")]
    pub map: String,
    pub position: Position,
    pub intro_dialogue: ScenarioRelativePath,
}

/// The manifest-owned initial and engine-controlled flags, plus every data root.
///
/// This is a partial manifest adapter alongside the other manifest slices. Flag order and
/// spelling are source content: the state builder and validator will give each list its
/// runtime meaning in later milestones.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ManifestFlagsRefs {
    #[serde(deserialize_with = "deserialize_strings")]
    pub bootstrap_flags: Vec<String>,
    #[serde(deserialize_with = "deserialize_strings")]
    pub engine_managed_flags: Vec<String>,
    pub refs: ManifestRefs,
}

/// Every source-authored manifest reference.
///
/// The pinned manifest marks catalog roots with a trailing slash. `ScenarioDirectoryPath`
/// consumes that source-only marker and retains the normalized, scenario-contained path, while
/// file references remain ordinary [`ScenarioRelativePath`] values.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ManifestRefs {
    pub party: ScenarioRelativePath,
    pub classes: ScenarioDirectoryPath,
    pub maps: ScenarioDirectoryPath,
    pub dialogue: ScenarioDirectoryPath,
    pub items: ScenarioDirectoryPath,
    pub enemies: ScenarioDirectoryPath,
    pub encount: ScenarioDirectoryPath,
    pub recipe: ScenarioDirectoryPath,
    pub quests: ScenarioRelativePath,
    pub balance: ScenarioRelativePath,
    pub battle_backgrounds: ScenarioRelativePath,
    pub assets: ScenarioDirectoryPath,
    pub tmx: ScenarioDirectoryPath,
}

/// A source-authored directory reference within the active scenario package.
///
/// Source manifests use a trailing slash to distinguish catalog roots from single-file refs.
/// The separator is accepted at this adapter boundary only; the stored value is a validated,
/// normalized [`ScenarioRelativePath`]. This keeps ADR 0004's general path policy strict while
/// preserving the pinned manifest spelling.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScenarioDirectoryPath(ScenarioRelativePath);

impl ScenarioDirectoryPath {
    /// Returns the normalized directory path without its source-only trailing slash.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Returns this directory reference as the shared validated path type.
    pub fn as_relative_path(&self) -> &ScenarioRelativePath {
        &self.0
    }
}

impl TryFrom<&str> for ScenarioDirectoryPath {
    type Error = ScenarioDirectoryPathError;

    fn try_from(path: &str) -> Result<Self, Self::Error> {
        let path = path
            .strip_suffix('/')
            .ok_or(ScenarioDirectoryPathError::MissingTrailingSlash)?;
        ScenarioRelativePath::try_from(path)
            .map(Self)
            .map_err(ScenarioDirectoryPathError::InvalidPath)
    }
}

impl TryFrom<String> for ScenarioDirectoryPath {
    type Error = ScenarioDirectoryPathError;

    fn try_from(path: String) -> Result<Self, Self::Error> {
        Self::try_from(path.as_str())
    }
}

impl<'de> Deserialize<'de> for ScenarioDirectoryPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::try_from(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

/// Why a manifest directory reference cannot be used.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScenarioDirectoryPathError {
    MissingTrailingSlash,
    InvalidPath(ScenarioRelativePathError),
}

impl fmt::Display for ScenarioDirectoryPathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingTrailingSlash => {
                formatter.write_str("manifest directory reference must end with '/'")
            }
            Self::InvalidPath(source) => source.fmt(formatter),
        }
    }
}

impl std::error::Error for ScenarioDirectoryPathError {}

#[cfg(test)]
mod tests {
    use super::{
        Manifest, ManifestApothecary, ManifestApothecaryIcons, ManifestFlagsRefs, ManifestFont,
        ManifestIdentityWindow, ManifestProtagonist, ManifestProtagonistStart, ManifestRefs,
        ManifestServiceSprite, ManifestServiceSprites, ManifestStart, ManifestTitle,
        ManifestTitleFontUi, ManifestUi, ScenarioDirectoryPath, ScenarioDirectoryPathError,
    };
    use crate::scenario_yaml;
    use crate::{scenario_path::ScenarioRelativePathError, scenario_spatial::Position};

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
    fn partial_manifest_scalar_strings_reject_non_string_yaml_types() {
        let identity =
            include_str!("../tests/fixtures/rusted-kingdoms-manifest-identity-window.yaml");
        for document in [
            identity.replacen("id: my_rpg_story", "id: 42", 1),
            identity.replacen("name: \"Chronicles of the Lost Flame\"", "name: false", 1),
            identity.replacen("version: \"1.0.0\"", "version: true", 1),
            identity.replacen("window_title: \"Rusted Kingdoms\"", "window_title: 7", 1),
        ] {
            assert!(
                scenario_yaml::from_str::<ManifestIdentityWindow>(&document).is_err(),
                "{document}"
            );
        }

        let protagonist =
            include_str!("../tests/fixtures/rusted-kingdoms-manifest-protagonist-start.yaml");
        for document in [
            protagonist.replacen("  id: aric", "  id: 42", 1),
            protagonist.replacen("  name: \"Aric\"", "  name: false", 1),
            protagonist.replacen("  class: hero", "  class: 7", 1),
            protagonist.replacen("  map: town_01_ardel", "  map: true", 1),
        ] {
            assert!(
                scenario_yaml::from_str::<ManifestProtagonistStart>(&document).is_err(),
                "{document}"
            );
        }
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
                    position: Position::new(14, 5),
                    intro_dialogue: "data/dialogue/intro_cutscene.yaml".try_into().unwrap(),
                },
            }
        );
    }

    #[test]
    fn loads_pinned_flags_and_every_reference_with_normalized_directory_paths() {
        let manifest: ManifestFlagsRefs = scenario_yaml::from_str(include_str!(
            "../tests/fixtures/rusted-kingdoms-manifest-flags-refs.yaml"
        ))
        .expect("the pinned manifest flags/refs slice should deserialize");

        assert_eq!(
            manifest.bootstrap_flags,
            ["story_quest_started", "aric_teleport_unlocked"]
        );
        assert_eq!(
            manifest.engine_managed_flags,
            [
                "story_act2_started",
                "story_act3_started",
                "story_act4_started",
                "boss_zone10_defeated",
            ]
        );
        assert_eq!(
            manifest.refs,
            ManifestRefs {
                party: "data/party.yaml".try_into().unwrap(),
                classes: "data/classes/".try_into().unwrap(),
                maps: "data/maps/".try_into().unwrap(),
                dialogue: "data/dialogue/".try_into().unwrap(),
                items: "data/items/".try_into().unwrap(),
                enemies: "data/enemies/".try_into().unwrap(),
                encount: "data/encount/".try_into().unwrap(),
                recipe: "data/recipe/".try_into().unwrap(),
                quests: "data/quests.yaml".try_into().unwrap(),
                balance: "data/balance.yaml".try_into().unwrap(),
                battle_backgrounds: "data/battle_backgrounds.yaml".try_into().unwrap(),
                assets: "assets/".try_into().unwrap(),
                tmx: "assets/maps/".try_into().unwrap(),
            }
        );
        assert_eq!(manifest.refs.classes.as_str(), "data/classes");
        assert_eq!(manifest.refs.tmx.as_relative_path().as_str(), "assets/maps");
    }

    #[test]
    fn partial_manifest_flag_lists_reject_non_string_elements() {
        let fixture = include_str!("../tests/fixtures/rusted-kingdoms-manifest-flags-refs.yaml");
        for document in [
            fixture.replacen("  - story_quest_started", "  - 42", 1),
            fixture.replacen("  - story_act2_started", "  - false", 1),
        ] {
            assert!(
                scenario_yaml::from_str::<ManifestFlagsRefs>(&document).is_err(),
                "{document}"
            );
        }
    }

    #[test]
    fn directory_references_require_the_source_marker_and_validate_the_result() {
        assert_eq!(
            ScenarioDirectoryPath::try_from("data/classes"),
            Err(ScenarioDirectoryPathError::MissingTrailingSlash)
        );
        assert_eq!(
            ScenarioDirectoryPath::try_from("data//classes/"),
            Err(ScenarioDirectoryPathError::InvalidPath(
                ScenarioRelativePathError::EmptyComponent
            ))
        );
        assert_eq!(
            ScenarioDirectoryPath::try_from("../outside/"),
            Err(ScenarioDirectoryPathError::InvalidPath(
                ScenarioRelativePathError::EscapesPackage
            ))
        );
    }

    #[test]
    fn composes_every_owned_manifest_slice_from_the_source_shape() {
        let manifest: Manifest = scenario_yaml::from_str(include_str!(
            "../tests/fixtures/rusted-kingdoms-manifest-complete.yaml"
        ))
        .expect("the complete source-shaped manifest should deserialize");

        assert_eq!(manifest.id, "my_rpg_story");
        assert_eq!(
            manifest.title.cursor_icon.as_str(),
            "assets/images/icons/arrow-head-right.webp"
        );
        assert_eq!(
            manifest.inn.sprite.as_str(),
            "assets/sprites/npc/female_blue_01.tsx"
        );
        assert_eq!(manifest.start.position, Position::new(14, 5));
        assert_eq!(manifest.refs.encount.as_str(), "data/encount");
    }

    #[test]
    fn complete_manifest_strings_and_flag_lists_reject_non_string_yaml_types() {
        let fixture = include_str!("../tests/fixtures/rusted-kingdoms-manifest-complete.yaml");
        for document in [
            fixture.replacen("id: my_rpg_story", "id: 42", 1),
            fixture.replacen("name: \"Chronicles of the Lost Flame\"", "name: false", 1),
            fixture.replacen("  id: aric", "  id: true", 1),
            fixture.replacen("  name: Aric", "  name: 7", 1),
            fixture.replacen("  map: town_01_ardel", "  map: false", 1),
            fixture.replacen("  - story_quest_started", "  - 42", 1),
            fixture.replacen("  - story_act2_started", "  - true", 1),
        ] {
            assert!(
                scenario_yaml::from_str::<Manifest>(&document).is_err(),
                "{document}"
            );
        }
    }

    #[test]
    fn complete_manifest_reports_a_missing_top_level_field_with_its_location() {
        let document = include_str!("../tests/fixtures/rusted-kingdoms-manifest-complete.yaml")
            .replacen("id: my_rpg_story\n", "", 1);
        let error = scenario_yaml::from_str::<Manifest>(&document)
            .expect_err("a complete manifest without id should fail");

        assert_eq!(error.path(), "id");
        let location = error
            .location()
            .expect("Serde YAML should retain the missing field location");
        assert_eq!((location.line(), location.column()), (2, 1));
        assert_eq!(
            error.to_string(),
            "id: missing field `id` at line 2 column 1"
        );
    }

    #[test]
    fn complete_manifest_reports_a_missing_nested_field_with_its_location() {
        let document = include_str!("../tests/fixtures/rusted-kingdoms-manifest-complete.yaml")
            .replacen(
                "  cursor_icon: assets/images/icons/arrow-head-right.webp\n",
                "",
                1,
            );
        let error = scenario_yaml::from_str::<Manifest>(&document)
            .expect_err("a complete manifest without title.cursor_icon should fail");

        assert_eq!(error.path(), "title.cursor_icon");
        let location = error
            .location()
            .expect("Serde YAML should retain the missing field location");
        assert_eq!((location.line(), location.column()), (8, 3));
        assert_eq!(
            error.to_string(),
            "title.cursor_icon: missing field `cursor_icon` at line 8 column 3"
        );
    }
}
