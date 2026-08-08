//! Scenario YAML deserialization boundary.
//!
//! Typed scenario schemas use this helper to deserialize the source-authored YAML documents
//! selected by ADR 0002. File access, schema validation, and multi-document streams are added
//! by later milestones.

use serde::de::DeserializeOwned;
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
