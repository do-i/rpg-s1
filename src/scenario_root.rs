use bevy::prelude::Resource;
use std::fmt;

/// Default package selected by startup configuration.
pub const DEFAULT_SCENARIO_PACKAGE_KEY: &str = "rusted_kingdoms";

/// Why a package key cannot identify a scenario package.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScenarioPackageKeyError {
    Empty,
    InvalidFirstCharacter,
    InvalidCharacter,
}

impl fmt::Display for ScenarioPackageKeyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("package key must not be empty"),
            Self::InvalidFirstCharacter => formatter
                .write_str("package key must begin with a lowercase ASCII letter or digit"),
            Self::InvalidCharacter => formatter.write_str(
                "package key may contain only lowercase ASCII letters, digits, underscores, and hyphens",
            ),
        }
    }
}

impl std::error::Error for ScenarioPackageKeyError {}

/// The active scenario's logical root beneath Bevy's asset source.
///
/// This intentionally stores AssetServer-style paths instead of filesystem paths: callers use
/// `scenarios/<package-key>/...`, while Bevy supplies the physical `assets/` base. Construction
/// validates the package key; scenario-relative path containment is introduced by later
/// scenario-loading tasks.
#[derive(Clone, Debug, Eq, PartialEq, Resource)]
pub struct ScenarioRoot {
    package_key: String,
    logical_root: String,
}

impl Default for ScenarioRoot {
    fn default() -> Self {
        Self::try_for_package_key(DEFAULT_SCENARIO_PACKAGE_KEY)
            .expect("the compiled-in default scenario package key must be valid")
    }
}

impl ScenarioRoot {
    /// Creates the logical root for a package key accepted by ADR 0004.
    ///
    /// A key consists of lowercase ASCII letters, digits, `_`, or `-`, begins with a lowercase
    /// ASCII letter or digit, and contains no path separator or dot component.
    pub fn try_for_package_key(
        package_key: impl Into<String>,
    ) -> Result<Self, ScenarioPackageKeyError> {
        let package_key = package_key.into();
        validate_package_key(&package_key)?;
        let logical_root = format!("scenarios/{package_key}");

        Ok(Self {
            package_key,
            logical_root,
        })
    }

    /// Returns the selected package key for diagnostics and future scenario selection.
    pub fn package_key(&self) -> &str {
        &self.package_key
    }

    /// Returns this scenario's AssetServer logical prefix, relative to `assets/`.
    pub fn logical_root(&self) -> &str {
        &self.logical_root
    }

    /// Returns the selected scenario's manifest AssetServer path.
    pub fn manifest_asset_path(&self) -> String {
        self.resolve("manifest.yaml")
    }

    /// Resolves an already scenario-relative path under the active logical root.
    ///
    /// This is deliberately a mechanical join. Scenario-relative path validation,
    /// normalization, and containment are deferred to the M2.02 path type; callers must not
    /// pass unvalidated input here.
    pub fn resolve(&self, scenario_relative_path: &str) -> String {
        format!("{}/{scenario_relative_path}", self.logical_root)
    }
}

fn validate_package_key(package_key: &str) -> Result<(), ScenarioPackageKeyError> {
    let Some(first) = package_key.bytes().next() else {
        return Err(ScenarioPackageKeyError::Empty);
    };

    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return Err(ScenarioPackageKeyError::InvalidFirstCharacter);
    }

    if package_key.bytes().all(|character| {
        character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || matches!(character, b'_' | b'-')
    }) {
        Ok(())
    } else {
        Err(ScenarioPackageKeyError::InvalidCharacter)
    }
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_SCENARIO_PACKAGE_KEY, ScenarioPackageKeyError, ScenarioRoot};

    #[test]
    fn default_manifest_and_nested_path_resolve_under_rusted_kingdoms() {
        let root = ScenarioRoot::default();

        assert_eq!(root.package_key(), DEFAULT_SCENARIO_PACKAGE_KEY);
        assert_eq!(root.logical_root(), "scenarios/rusted_kingdoms");
        assert_eq!(
            root.manifest_asset_path(),
            "scenarios/rusted_kingdoms/manifest.yaml"
        );
        assert_eq!(
            root.resolve("assets/maps/town_01_ardel.tmx"),
            "scenarios/rusted_kingdoms/assets/maps/town_01_ardel.tmx"
        );
    }

    #[test]
    fn another_package_key_changes_only_the_logical_root() {
        let root = ScenarioRoot::try_for_package_key("test_campaign-2").unwrap();

        assert_eq!(root.package_key(), "test_campaign-2");
        assert_eq!(root.logical_root(), "scenarios/test_campaign-2");
        assert_eq!(
            root.manifest_asset_path(),
            "scenarios/test_campaign-2/manifest.yaml"
        );
        assert_eq!(
            root.resolve("data/dialogue/opening.yaml"),
            "scenarios/test_campaign-2/data/dialogue/opening.yaml"
        );
    }

    #[test]
    fn invalid_package_keys_cannot_construct_a_scenario_root() {
        assert_eq!(
            ScenarioRoot::try_for_package_key(""),
            Err(ScenarioPackageKeyError::Empty)
        );

        for package_key in [
            "-campaign",
            "_campaign",
            ".campaign",
            ".",
            "..",
            "/campaign",
            "Rusted_Kingdoms",
        ] {
            assert_eq!(
                ScenarioRoot::try_for_package_key(package_key),
                Err(ScenarioPackageKeyError::InvalidFirstCharacter),
                "{package_key}"
            );
        }

        for package_key in [
            "rusted_Kingdoms",
            "campaign/name",
            "campaign\\name",
            "c:/campaign",
            "campaign.name",
            "campaign name",
            "campaign!",
        ] {
            assert_eq!(
                ScenarioRoot::try_for_package_key(package_key),
                Err(ScenarioPackageKeyError::InvalidCharacter),
                "{package_key}"
            );
        }
    }

    #[test]
    fn logical_asset_paths_remain_relative() {
        let root = ScenarioRoot::default();

        for path in [
            root.logical_root().to_owned(),
            root.resolve("manifest.yaml"),
            root.resolve("assets/audio/overworld.ogg"),
        ] {
            assert!(!path.starts_with('/'));
            assert!(!path.starts_with("assets/"));
            assert!(!path.contains(":"));
        }
    }
}
