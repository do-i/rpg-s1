//! Versioned, physical-key-independent gameplay input records.

use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

pub(crate) const INPUT_RECORD_FORMAT_VERSION: u32 = 1;
pub(crate) const NORMALIZED_ACTION_SCHEMA: &str = "rpg-s1.normalized-actions.v1";

/// Portable source identity captured before a recording window opens.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecordSource {
    pub package_key: String,
    pub scenario_id: String,
    pub scenario_version: String,
}

/// Semantic gameplay actions after physical key mapping.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum NormalizedAction {
    Back,
    Confirm,
    MenuUp,
    MenuDown,
    MoveUp,
    MoveLeft,
    MoveDown,
    MoveRight,
}

/// All normalized actions observed in one logical update and its resulting checkpoint.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecordedActionFrame {
    pub index: u64,
    pub frame: u64,
    pub actions: Vec<NormalizedAction>,
    pub state_hash: String,
}

/// Complete deterministic input record.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InputRecord {
    pub format_version: u32,
    pub game_version: String,
    pub source: RecordSource,
    pub seed: u64,
    pub action_schema: String,
    pub action_frames: Vec<RecordedActionFrame>,
}

impl InputRecord {
    pub(crate) fn new(game_version: impl Into<String>, source: RecordSource, seed: u64) -> Self {
        Self {
            format_version: INPUT_RECORD_FORMAT_VERSION,
            game_version: game_version.into(),
            source,
            seed,
            action_schema: NORMALIZED_ACTION_SCHEMA.to_owned(),
            action_frames: Vec::new(),
        }
    }

    pub(crate) fn encode(&self) -> Result<Vec<u8>, InputRecordError> {
        self.validate()?;
        serde_yaml_ng::to_string(self)
            .map(String::into_bytes)
            .map_err(|error| InputRecordError::Encode(error.to_string()))
    }

    pub(crate) fn decode(bytes: &[u8]) -> Result<Self, InputRecordError> {
        let document = std::str::from_utf8(bytes)
            .map_err(|error| InputRecordError::Decode(format!("record is not UTF-8: {error}")))?;
        let record: Self = serde_yaml_ng::from_str(document)
            .map_err(|error| InputRecordError::Decode(error.to_string()))?;
        record.validate()?;
        Ok(record)
    }

    fn validate(&self) -> Result<(), InputRecordError> {
        if self.format_version != INPUT_RECORD_FORMAT_VERSION {
            return Err(InputRecordError::UnsupportedFormat(self.format_version));
        }
        if self.action_schema != NORMALIZED_ACTION_SCHEMA {
            return Err(InputRecordError::UnsupportedActionSchema(
                self.action_schema.clone(),
            ));
        }
        for (expected_index, frame) in self.action_frames.iter().enumerate() {
            if frame.index != expected_index as u64 {
                return Err(InputRecordError::Invalid(format!(
                    "action frame index {} must be {}",
                    frame.index, expected_index
                )));
            }
            if expected_index > 0 && frame.frame <= self.action_frames[expected_index - 1].frame {
                return Err(InputRecordError::Invalid(format!(
                    "action frame {} must follow the previous logical frame",
                    frame.index
                )));
            }
            if frame.actions.is_empty() {
                return Err(InputRecordError::Invalid(format!(
                    "action frame {} has no actions",
                    frame.index
                )));
            }
            if frame.actions.windows(2).any(|pair| pair[0] >= pair[1]) {
                return Err(InputRecordError::Invalid(format!(
                    "action frame {} actions must be unique and schema-ordered",
                    frame.index
                )));
            }
            if frame.state_hash.len() != 64
                || !frame
                    .state_hash
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            {
                return Err(InputRecordError::Invalid(format!(
                    "action frame {} state_hash must be 64 lowercase hexadecimal characters",
                    frame.index
                )));
            }
        }
        if self.game_version.is_empty()
            || self.source.package_key.is_empty()
            || self.source.scenario_id.is_empty()
            || self.source.scenario_version.is_empty()
        {
            return Err(InputRecordError::Invalid(
                "record header identity fields must not be empty".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum InputRecordError {
    Encode(String),
    Decode(String),
    UnsupportedFormat(u32),
    UnsupportedActionSchema(String),
    Invalid(String),
}

impl fmt::Display for InputRecordError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Encode(error) => write!(formatter, "input record encode failed: {error}"),
            Self::Decode(error) => write!(formatter, "input record decode failed: {error}"),
            Self::UnsupportedFormat(version) => {
                write!(
                    formatter,
                    "unsupported input record format version {version}"
                )
            }
            Self::UnsupportedActionSchema(schema) => {
                write!(formatter, "unsupported input action schema `{schema}`")
            }
            Self::Invalid(error) => write!(formatter, "invalid input record: {error}"),
        }
    }
}

impl Error for InputRecordError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn record() -> InputRecord {
        let mut record = InputRecord::new(
            "2026.8.1",
            RecordSource {
                package_key: "invented".to_owned(),
                scenario_id: "invented_story".to_owned(),
                scenario_version: "2.3.4".to_owned(),
            },
            42,
        );
        record.action_frames.push(RecordedActionFrame {
            index: 0,
            frame: 7,
            actions: vec![NormalizedAction::Confirm, NormalizedAction::MoveRight],
            state_hash: "a".repeat(64),
        });
        record
    }

    #[test]
    fn versioned_header_and_normalized_action_schema_round_trip() {
        let expected = record();
        let encoded = expected.encode().unwrap();
        let text = std::str::from_utf8(&encoded).unwrap();

        assert!(text.contains("format_version: 1"));
        assert!(text.contains("game_version: 2026.8.1"));
        assert!(text.contains("scenario_version: 2.3.4"));
        assert!(text.contains("seed: 42"));
        assert!(text.contains("action_schema: rpg-s1.normalized-actions.v1"));
        assert!(text.contains("- confirm"));
        assert!(!text.contains("Enter"));
        assert_eq!(InputRecord::decode(&encoded), Ok(expected));
    }

    #[test]
    fn decoder_rejects_unknown_versions_schemas_fields_and_malformed_frames() {
        for document in [
            String::from_utf8(record().encode().unwrap())
                .unwrap()
                .replacen("format_version: 1", "format_version: 2", 1),
            String::from_utf8(record().encode().unwrap())
                .unwrap()
                .replacen("rpg-s1.normalized-actions.v1", "invented.v9", 1),
            format!(
                "{}unknown_header: true\n",
                String::from_utf8(record().encode().unwrap()).unwrap()
            ),
            String::from_utf8(record().encode().unwrap())
                .unwrap()
                .replacen("index: 0", "index: 1", 1),
            String::from_utf8(record().encode().unwrap())
                .unwrap()
                .replacen(&"a".repeat(64), "not-a-hash", 1),
        ] {
            assert!(InputRecord::decode(document.as_bytes()).is_err());
        }
    }
}
