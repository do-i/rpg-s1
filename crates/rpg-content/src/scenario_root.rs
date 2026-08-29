use bevy::prelude::Resource;
use std::fmt;

use crate::scenario_path::ScenarioRelativePath;

/// Default package selected by startup configuration.
pub const DEFAULT_SCENARIO_PACKAGE_KEY: &str = "rusted_kingdoms";

/// The scenario-relative path of every package's entry point.
pub const SCENARIO_MANIFEST_PATH: &str = "manifest.yaml";

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

/// Adds stable scenario provenance to an asset or data loading failure.
///
/// Displayed paths are qualified by package key but remain relative to the package, so this
/// context adds no machine path and does not repeat the `assets/scenarios/` storage layout. The
/// original error is retained as the standard error source.
#[derive(Debug)]
pub struct ScenarioLoadError<E> {
    package_key: String,
    scenario_relative_path: String,
    source: E,
}

impl<E> ScenarioLoadError<E> {
    fn new(root: &ScenarioRoot, scenario_relative_path: &ScenarioRelativePath, source: E) -> Self {
        Self {
            package_key: root.package_key.clone(),
            scenario_relative_path: scenario_relative_path.as_str().to_owned(),
            source,
        }
    }

    /// Returns the package key that was active when the error was created.
    pub fn package_key(&self) -> &str {
        &self.package_key
    }

    /// Returns the failing path relative to the selected scenario package.
    pub fn scenario_relative_path(&self) -> &str {
        &self.scenario_relative_path
    }

    /// Returns the underlying loader, parser, or I/O error.
    pub fn underlying(&self) -> &E {
        &self.source
    }

    /// Consumes the contextual error and returns its underlying cause.
    pub fn into_underlying(self) -> E {
        self.source
    }
}

impl<E: fmt::Display> fmt::Display for ScenarioLoadError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}:{}: {}",
            self.package_key, self.scenario_relative_path, self.source
        )
    }
}

impl<E: std::error::Error + 'static> std::error::Error for ScenarioLoadError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// The active scenario's logical root beneath Bevy's asset source.
///
/// This intentionally stores AssetServer-style paths instead of filesystem paths: callers use
/// `scenarios/<package-key>/...`, while Bevy supplies the physical `assets/` base. Construction
/// validates both the package key and every path joined to the root.
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
    /// Creates the logical root for an accepted scenario package key.
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
        self.resolve(&manifest_relative_path())
    }

    /// Resolves a validated scenario-relative path under the active logical root.
    pub fn resolve(&self, scenario_relative_path: &ScenarioRelativePath) -> String {
        format!("{}/{scenario_relative_path}", self.logical_root)
    }

    /// Adds this package's manifest provenance to an underlying loading error.
    pub fn manifest_load_error<E>(&self, source: E) -> ScenarioLoadError<E> {
        ScenarioLoadError::new(self, &manifest_relative_path(), source)
    }

    /// Adds this package and a validated scenario-relative path to an underlying loading error.
    pub fn load_error<E>(
        &self,
        scenario_relative_path: &ScenarioRelativePath,
        source: E,
    ) -> ScenarioLoadError<E> {
        ScenarioLoadError::new(self, scenario_relative_path, source)
    }
}

fn manifest_relative_path() -> ScenarioRelativePath {
    ScenarioRelativePath::try_from(SCENARIO_MANIFEST_PATH)
        .expect("the compiled-in scenario manifest path must be valid")
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
    use super::{
        DEFAULT_SCENARIO_PACKAGE_KEY, SCENARIO_MANIFEST_PATH, ScenarioPackageKeyError, ScenarioRoot,
    };
    use crate::scenario_path::ScenarioRelativePath;
    use std::{error::Error, fmt, io};

    #[derive(Debug, Eq, PartialEq)]
    struct ParseError {
        message: &'static str,
    }

    impl fmt::Display for ParseError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str(self.message)
        }
    }

    impl Error for ParseError {}

    #[test]
    fn default_manifest_and_nested_path_resolve_under_rusted_kingdoms() {
        let root = ScenarioRoot::default();
        let map_path = ScenarioRelativePath::try_from("assets/maps/town_01_ardel.tmx").unwrap();

        assert_eq!(root.package_key(), DEFAULT_SCENARIO_PACKAGE_KEY);
        assert_eq!(root.logical_root(), "scenarios/rusted_kingdoms");
        assert_eq!(
            root.manifest_asset_path(),
            "scenarios/rusted_kingdoms/manifest.yaml"
        );
        assert_eq!(
            root.resolve(&map_path),
            "scenarios/rusted_kingdoms/assets/maps/town_01_ardel.tmx"
        );
    }

    #[test]
    fn another_package_key_changes_only_the_logical_root() {
        let root = ScenarioRoot::try_for_package_key("test_campaign-2").unwrap();
        let dialogue_path = ScenarioRelativePath::try_from("data/dialogue/opening.yaml").unwrap();

        assert_eq!(root.package_key(), "test_campaign-2");
        assert_eq!(root.logical_root(), "scenarios/test_campaign-2");
        assert_eq!(
            root.manifest_asset_path(),
            "scenarios/test_campaign-2/manifest.yaml"
        );
        assert_eq!(
            root.resolve(&dialogue_path),
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
        let manifest_path = ScenarioRelativePath::try_from("manifest.yaml").unwrap();
        let audio_path = ScenarioRelativePath::try_from("assets/audio/overworld.ogg").unwrap();

        for path in [
            root.logical_root().to_owned(),
            root.resolve(&manifest_path),
            root.resolve(&audio_path),
        ] {
            assert!(!path.starts_with('/'));
            assert!(!path.starts_with("assets/"));
            assert!(!path.contains(":"));
        }
    }

    #[test]
    fn manifest_load_error_reports_package_relative_path_and_missing_cause() {
        let root = ScenarioRoot::default();
        let error = root.manifest_load_error(io::Error::new(
            io::ErrorKind::NotFound,
            "manifest file was not found",
        ));

        assert_eq!(
            error.to_string(),
            "rusted_kingdoms:manifest.yaml: manifest file was not found"
        );
        assert_eq!(error.package_key(), DEFAULT_SCENARIO_PACKAGE_KEY);
        assert_eq!(error.scenario_relative_path(), SCENARIO_MANIFEST_PATH);
        assert_eq!(error.underlying().kind(), io::ErrorKind::NotFound);
        assert_eq!(
            Error::source(&error).map(ToString::to_string),
            Some("manifest file was not found".to_owned())
        );
        assert!(!error.to_string().contains("assets/scenarios"));
    }

    #[test]
    fn nested_load_error_preserves_parse_cause_and_alternate_package() {
        let root = ScenarioRoot::try_for_package_key("test_campaign-2").unwrap();
        let dialogue_path = ScenarioRelativePath::try_from("data/dialogue/opening.yaml").unwrap();
        let error = root.load_error(
            &dialogue_path,
            ParseError {
                message: "expected a mapping at line 4",
            },
        );

        assert_eq!(
            error.to_string(),
            "test_campaign-2:data/dialogue/opening.yaml: expected a mapping at line 4"
        );
        assert_eq!(error.package_key(), "test_campaign-2");
        assert_eq!(error.scenario_relative_path(), "data/dialogue/opening.yaml");
        assert_eq!(
            error.underlying(),
            &ParseError {
                message: "expected a mapping at line 4"
            }
        );
        assert_eq!(
            Error::source(&error).map(ToString::to_string),
            Some("expected a mapping at line 4".to_owned())
        );
        assert_eq!(
            error.into_underlying(),
            ParseError {
                message: "expected a mapping at line 4"
            }
        );
    }
}
