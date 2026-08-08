//! Scenario YAML deserialization boundary.
//!
//! Typed scenario schemas use this helper to deserialize the source-authored YAML documents
//! selected by ADR 0002. File access, schema validation, and multi-document streams are added
//! by later milestones.

use serde::de::DeserializeOwned;

/// Deserializes one YAML document into a caller-provided typed schema.
pub fn from_str<T>(document: &str) -> Result<T, serde_yaml_ng::Error>
where
    T: DeserializeOwned,
{
    serde_yaml_ng::from_str(document)
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
}
