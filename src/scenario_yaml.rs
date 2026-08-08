//! Scenario YAML deserialization boundary.
//!
//! Typed scenario schemas use this helper to deserialize the source-authored YAML documents
//! selected by ADR 0002. File access, schema validation, and multi-document streams are added
//! by later milestones.

use serde::{Deserialize, Deserializer, de::DeserializeOwned};
use std::error::Error as StdError;
use std::fmt;

/// Deserializes one YAML document into a caller-provided typed schema.
pub fn from_str<T>(document: &str) -> Result<T, ScenarioYamlError>
where
    T: DeserializeOwned,
{
    serde_path_to_error::deserialize(serde_yaml_ng::Deserializer::from_str(document))
        .map_err(ScenarioYamlError::from)
}

/// Deserializes exactly one YAML string scalar without scalar-to-text coercion.
///
/// `serde_yaml_ng` follows its predecessor's `deserialize_string` behavior and presents number
/// and boolean scalars as text to a plain Serde `String`. Scenario schemas use this helper when
/// YAML scalar type is part of their compatibility contract, so `42`, `true`, and `null` cannot
/// silently become identifiers or paths.
pub fn deserialize_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(StrictStringVisitor)
}

/// Deserializes a YAML sequence whose elements must each be YAML string scalars.
pub fn deserialize_strings<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Vec::<StrictString>::deserialize(deserializer)
        .map(|values| values.into_iter().map(|value| value.0).collect())
}

struct StrictString(String);

impl<'de> Deserialize<'de> for StrictString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_string(deserializer).map(Self)
    }
}

struct StrictStringVisitor;

impl serde::de::Visitor<'_> for StrictStringVisitor {
    type Value = String;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a YAML string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(value.to_owned())
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(value)
    }
}

/// A YAML deserialization failure with the most specific source field path available.
#[derive(Debug)]
pub struct ScenarioYamlError {
    path: String,
    message: String,
    source: serde_yaml_ng::Error,
}

impl ScenarioYamlError {
    /// Returns the YAML field path that owns the failure, or `.` for the document root.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the parser-provided YAML location when the failure has one.
    pub fn location(&self) -> Option<serde_yaml_ng::Location> {
        self.source.location()
    }
}

impl<E> From<serde_path_to_error::Error<E>> for ScenarioYamlError
where
    E: fmt::Display + Into<serde_yaml_ng::Error>,
{
    fn from(error: serde_path_to_error::Error<E>) -> Self {
        let serde_path = root_path(error.path().to_string());
        let source_message = error.inner().to_string();
        let path = required_field_path(serde_path.clone(), &source_message);
        let message = source_message
            .strip_prefix(&format!("{serde_path}: "))
            .unwrap_or(&source_message)
            .to_owned();
        Self {
            path,
            message,
            source: error.into_inner().into(),
        }
    }
}

impl fmt::Display for ScenarioYamlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.path, self.message)?;
        if let Some(location) = self.source.location() {
            let source = self.source.to_string();
            if !source.contains(" at line ") {
                write!(
                    formatter,
                    " at line {} column {}",
                    location.line(),
                    location.column()
                )?;
            }
        }
        Ok(())
    }
}

impl StdError for ScenarioYamlError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.source)
    }
}

fn required_field_path(path: String, message: &str) -> String {
    let Some(field) = message
        .split("missing field `")
        .nth(1)
        .and_then(|suffix| suffix.split_once('`').map(|(field, _)| field))
    else {
        return root_path(path);
    };

    let path = root_path(path);
    if path == "." {
        field.to_owned()
    } else {
        format!("{path}.{field}")
    }
}

fn root_path(path: String) -> String {
    if path.is_empty() {
        ".".to_owned()
    } else {
        path
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::from_str;

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    struct MinimalScenarioDocument {
        id: String,
        enabled: bool,
        retry_limit: u32,
        title: String,
        tags: Vec<String>,
        optional_note: Option<String>,
    }

    #[test]
    fn deserializes_minimal_on_disk_yaml_fixture() {
        let document: MinimalScenarioDocument =
            from_str(include_str!("../tests/fixtures/minimal-scenario.yaml"))
                .expect("minimal scenario fixture should deserialize");

        assert_eq!(
            document,
            MinimalScenarioDocument {
                id: "ardel_intro".to_owned(),
                enabled: true,
                retry_limit: 3,
                title: "A # quoted title".to_owned(),
                tags: vec!["intro".to_owned(), "tutorial".to_owned()],
                optional_note: None,
            }
        );
    }

    #[allow(dead_code)]
    #[derive(Debug, Deserialize)]
    struct RequiredTopLevel {
        id: String,
        title: RequiredTitle,
    }

    #[allow(dead_code)]
    #[derive(Debug, Deserialize)]
    struct RequiredTitle {
        image: String,
        cursor_icon: String,
    }

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    #[serde(deny_unknown_fields)]
    struct StrictStrings {
        #[serde(deserialize_with = "super::deserialize_string")]
        scalar: String,
        #[serde(deserialize_with = "super::deserialize_strings")]
        list: Vec<String>,
    }

    #[test]
    fn strict_string_helpers_accept_only_yaml_strings() {
        let strings: StrictStrings = from_str("scalar: plain\nlist: [plain, '42', \"true\"]\n")
            .expect("plain and quoted YAML strings should deserialize");
        assert_eq!(strings.scalar, "plain");
        assert_eq!(strings.list, ["plain", "42", "true"]);

        for document in [
            "scalar: 42\nlist: []\n",
            "scalar: true\nlist: []\n",
            "scalar: null\nlist: []\n",
            "scalar: okay\nlist: [42]\n",
            "scalar: okay\nlist: [false]\n",
            "scalar: okay\nlist: [null]\n",
            "scalar: okay\nlist: scalar\n",
        ] {
            assert!(from_str::<StrictStrings>(document).is_err(), "{document}");
        }
    }

    #[test]
    fn missing_nested_field_reports_the_complete_field_path_and_location() {
        let error = from_str::<RequiredTopLevel>("id: ardel_intro\ntitle:\n  image: title.webp\n")
            .expect_err("missing nested field should fail");

        assert_eq!(error.path(), "title.cursor_icon");
        let location = error
            .location()
            .expect("Serde YAML should retain a location");
        assert_eq!((location.line(), location.column()), (3, 3));
        assert_eq!(
            error.to_string(),
            "title.cursor_icon: missing field `cursor_icon` at line 3 column 3"
        );
    }

    #[test]
    fn missing_top_level_field_reports_its_field_path_and_location() {
        let error = from_str::<RequiredTopLevel>(
            "title:\n  image: title.webp\n  cursor_icon: cursor.webp\n",
        )
        .expect_err("missing top-level field should fail");

        assert_eq!(error.path(), "id");
        let location = error
            .location()
            .expect("Serde YAML should retain a location");
        assert_eq!((location.line(), location.column()), (1, 1));
        assert_eq!(
            error.to_string(),
            "id: missing field `id` at line 1 column 1"
        );
    }
}
