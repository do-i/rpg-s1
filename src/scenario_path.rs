//! Validated paths within one scenario package.

use std::fmt;

/// A normalized path relative to the active scenario package.
///
/// Values use forward slashes and contain no empty, `.` or `..` components. Construction
/// normalizes contained lexical traversal, but rejects traversal that would leave the package.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize)]
#[serde(try_from = "String")]
pub struct ScenarioRelativePath(String);

impl ScenarioRelativePath {
    /// Returns the normalized scenario-relative path.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for ScenarioRelativePath {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for ScenarioRelativePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl TryFrom<&str> for ScenarioRelativePath {
    type Error = ScenarioRelativePathError;

    fn try_from(path: &str) -> Result<Self, Self::Error> {
        normalize(path).map(Self)
    }
}

impl TryFrom<String> for ScenarioRelativePath {
    type Error = ScenarioRelativePathError;

    fn try_from(path: String) -> Result<Self, Self::Error> {
        Self::try_from(path.as_str())
    }
}

/// Why a source path cannot identify a file within one scenario package.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScenarioRelativePathError {
    Empty,
    Absolute,
    PlatformPrefix,
    UriLike,
    Backslash,
    EmptyComponent,
    EscapesPackage,
    NormalizesToEmpty,
}

impl fmt::Display for ScenarioRelativePathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("scenario-relative path must not be empty"),
            Self::Absolute => formatter.write_str("scenario-relative path must not be absolute"),
            Self::PlatformPrefix => {
                formatter.write_str("scenario-relative path must not have a platform prefix")
            }
            Self::UriLike => {
                formatter.write_str("scenario-relative path must not be URI-like or contain ':'")
            }
            Self::Backslash => {
                formatter.write_str("scenario-relative path must use forward slashes")
            }
            Self::EmptyComponent => {
                formatter.write_str("scenario-relative path must not contain empty components")
            }
            Self::EscapesPackage => {
                formatter.write_str("scenario-relative path must not escape its package")
            }
            Self::NormalizesToEmpty => formatter.write_str(
                "scenario-relative path must identify a file or directory in its package",
            ),
        }
    }
}

impl std::error::Error for ScenarioRelativePathError {}

fn normalize(path: &str) -> Result<String, ScenarioRelativePathError> {
    if path.is_empty() {
        return Err(ScenarioRelativePathError::Empty);
    }
    if path.starts_with('/') {
        return Err(ScenarioRelativePathError::Absolute);
    }

    let bytes = path.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        return Err(ScenarioRelativePathError::PlatformPrefix);
    }
    if path.contains(':') {
        return Err(ScenarioRelativePathError::UriLike);
    }
    if path.contains('\\') {
        return Err(ScenarioRelativePathError::Backslash);
    }

    let mut normalized = Vec::new();
    for component in path.split('/') {
        match component {
            "" => return Err(ScenarioRelativePathError::EmptyComponent),
            "." => {}
            ".." => {
                if normalized.pop().is_none() {
                    return Err(ScenarioRelativePathError::EscapesPackage);
                }
            }
            component => normalized.push(component),
        }
    }

    if normalized.is_empty() {
        Err(ScenarioRelativePathError::NormalizesToEmpty)
    } else {
        Ok(normalized.join("/"))
    }
}

#[cfg(test)]
mod tests {
    use super::{ScenarioRelativePath, ScenarioRelativePathError};

    #[test]
    fn accepts_safe_nested_forward_slash_paths() {
        for path in [
            "manifest.yaml",
            "data/dialogue/opening.yaml",
            "assets/maps/town_01_ardel.tmx",
        ] {
            let validated = ScenarioRelativePath::try_from(path).unwrap();
            assert_eq!(validated.as_str(), path);
        }
    }

    #[test]
    fn normalizes_contained_dot_components_and_parent_traversal() {
        assert_eq!(
            ScenarioRelativePath::try_from("./assets/maps/../tilesets/walls.tsx")
                .unwrap()
                .as_str(),
            "assets/tilesets/walls.tsx"
        );
    }

    #[test]
    fn rejects_unix_absolute_paths_and_parent_escapes() {
        assert_eq!(
            ScenarioRelativePath::try_from("/etc/passwd"),
            Err(ScenarioRelativePathError::Absolute)
        );

        for path in ["../secret.yaml", "data/../../secret.yaml"] {
            assert_eq!(
                ScenarioRelativePath::try_from(path),
                Err(ScenarioRelativePathError::EscapesPackage),
                "{path}"
            );
        }
    }

    #[test]
    fn rejects_cross_platform_and_uri_like_spellings() {
        for path in ["C:/scenario/file.yaml", "C:\\scenario\\file.yaml"] {
            assert_eq!(
                ScenarioRelativePath::try_from(path),
                Err(ScenarioRelativePathError::PlatformPrefix),
                "{path}"
            );
        }

        assert_eq!(
            ScenarioRelativePath::try_from("assets\\maps\\town.tmx"),
            Err(ScenarioRelativePathError::Backslash)
        );
        assert_eq!(
            ScenarioRelativePath::try_from("file:///scenario/manifest.yaml"),
            Err(ScenarioRelativePathError::UriLike)
        );
    }

    #[test]
    fn rejects_ambiguous_or_empty_paths() {
        let cases = [
            ("", ScenarioRelativePathError::Empty),
            (
                "data//party.yaml",
                ScenarioRelativePathError::EmptyComponent,
            ),
            ("data/", ScenarioRelativePathError::EmptyComponent),
            (".", ScenarioRelativePathError::NormalizesToEmpty),
            ("data/..", ScenarioRelativePathError::NormalizesToEmpty),
        ];

        for (path, expected) in cases {
            assert_eq!(
                ScenarioRelativePath::try_from(path),
                Err(expected),
                "{path}"
            );
        }
    }

    #[test]
    fn serde_deserialization_cannot_bypass_validation() {
        #[derive(Debug, serde::Deserialize)]
        struct PathHolder {
            path: ScenarioRelativePath,
        }

        let holder: PathHolder = serde_yaml_ng::from_str("path: data/./party.yaml").unwrap();
        assert_eq!(holder.path.as_str(), "data/party.yaml");

        let error = serde_yaml_ng::from_str::<PathHolder>("path: ../../outside.yaml").unwrap_err();
        assert!(error.to_string().contains("must not escape its package"));
    }
}
