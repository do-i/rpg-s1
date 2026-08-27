//! Versioned, physical-key-independent gameplay input records.

use std::{error::Error, fmt, fs, path::PathBuf};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    action_input::{ActionState, update_action_state},
    app_state::AppState,
    debug_launch::DebugLaunchConfig,
    game_state::GameState,
    gameplay_rng::GameplayRng,
    save_data::SavePayload,
};

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub debug: Option<DebugLaunchConfig>,
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
            debug: None,
            action_frames: Vec::new(),
        }
    }

    pub(crate) fn with_debug(mut self, debug: Option<DebugLaunchConfig>) -> Self {
        self.debug = debug;
        self
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

/// Runtime input mode selected by the process command before the window opens.
#[derive(Debug, Resource)]
pub(crate) enum InputAutomation {
    Record {
        output: PathBuf,
        record: InputRecord,
        frame: u64,
        failure: Option<String>,
    },
    Replay {
        input: PathBuf,
        record: InputRecord,
        cursor: usize,
        frame: u64,
        failure: Option<String>,
        complete: bool,
    },
}

impl InputAutomation {
    pub(crate) fn record(output: PathBuf, record: InputRecord) -> Self {
        Self::Record {
            output,
            record,
            frame: 0,
            failure: None,
        }
    }

    pub(crate) fn replay(input: PathBuf, record: InputRecord) -> Self {
        Self::Replay {
            input,
            record,
            cursor: 0,
            frame: 0,
            failure: None,
            complete: false,
        }
    }
}

pub(crate) struct InputRecordPlugin;

impl Plugin for InputRecordPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, apply_replay_actions.after(update_action_state))
            .add_systems(Last, (checkpoint_action_frame, advance_frame).chain());
    }
}

fn apply_replay_actions(
    mut actions: ResMut<ActionState>,
    mut automation: Option<ResMut<InputAutomation>>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(automation) = automation.as_deref_mut() else {
        return;
    };
    let InputAutomation::Replay {
        record,
        cursor,
        frame,
        failure,
        ..
    } = automation
    else {
        return;
    };
    actions.replace_with_normalized(&[]);
    let Some(expected) = record.action_frames.get(*cursor) else {
        return;
    };
    if expected.frame == *frame {
        actions.replace_with_normalized(&expected.actions);
    } else if expected.frame < *frame {
        let message = format!(
            "replay missed action {} scheduled for frame {}",
            expected.index, expected.frame
        );
        eprintln!("Replay divergence: {message}");
        *failure = Some(message);
        exit.write(AppExit::error());
    }
}

fn checkpoint_action_frame(
    actions: Res<ActionState>,
    state: Option<Res<State<AppState>>>,
    game: Option<Res<GameState>>,
    startup_rng: Option<Res<GameplayRng>>,
    mut automation: Option<ResMut<InputAutomation>>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(automation) = automation.as_deref_mut() else {
        return;
    };
    let normalized = actions.normalized_actions();
    match automation {
        InputAutomation::Record {
            output,
            record,
            frame,
            failure,
        } if !normalized.is_empty() => {
            let index = record.action_frames.len() as u64;
            match checkpoint_hash(
                index,
                &normalized,
                state.as_deref(),
                game.as_deref(),
                startup_rng.as_deref(),
            ) {
                Ok(state_hash) => {
                    record.action_frames.push(RecordedActionFrame {
                        index,
                        frame: *frame,
                        actions: normalized,
                        state_hash,
                    });
                    if let Err(error) = persist_record(output, record) {
                        eprintln!("Input recording failed: {error}");
                        *failure = Some(error);
                        exit.write(AppExit::error());
                    }
                }
                Err(error) => {
                    eprintln!("Input recording failed: {error}");
                    *failure = Some(error);
                    exit.write(AppExit::error());
                }
            }
        }
        InputAutomation::Replay {
            input,
            record,
            cursor,
            frame,
            failure,
            complete,
        } => {
            let Some(expected) = record.action_frames.get(*cursor) else {
                return;
            };
            if expected.frame != *frame || failure.is_some() {
                return;
            }
            let actual = checkpoint_hash(
                expected.index,
                &normalized,
                state.as_deref(),
                game.as_deref(),
                startup_rng.as_deref(),
            );
            match actual {
                Ok(actual) if actual == expected.state_hash => {
                    *cursor += 1;
                    if *cursor == record.action_frames.len() {
                        *complete = true;
                        println!(
                            "Replay PASS: {} action frames matched ({})",
                            cursor,
                            input.display()
                        );
                        exit.write(AppExit::Success);
                    }
                }
                Ok(actual) => {
                    let message = format!(
                        "action {} at frame {}: expected {}, found {}",
                        expected.index, expected.frame, expected.state_hash, actual
                    );
                    eprintln!("Replay divergence: {message}");
                    *failure = Some(message);
                    exit.write(AppExit::error());
                }
                Err(error) => {
                    eprintln!("Replay failed: {error}");
                    *failure = Some(error);
                    exit.write(AppExit::error());
                }
            }
        }
        InputAutomation::Record { .. } => {}
    }
}

fn advance_frame(mut automation: Option<ResMut<InputAutomation>>) {
    let Some(automation) = automation.as_deref_mut() else {
        return;
    };
    match automation {
        InputAutomation::Record { frame, .. } | InputAutomation::Replay { frame, .. } => {
            *frame = frame.saturating_add(1);
        }
    }
}

fn persist_record(output: &PathBuf, record: &InputRecord) -> Result<(), String> {
    let encoded = record.encode().map_err(|error| error.to_string())?;
    fs::write(output, encoded)
        .map_err(|error| format!("could not write record `{}`: {error}", output.display()))
}

#[derive(Serialize)]
struct Checkpoint<'a> {
    action_schema: &'static str,
    action_index: u64,
    actions: &'a [NormalizedAction],
    app_state: String,
    game: Option<SavePayload>,
    startup_rng_state: Option<u64>,
}

fn checkpoint_hash(
    action_index: u64,
    actions: &[NormalizedAction],
    state: Option<&State<AppState>>,
    game: Option<&GameState>,
    startup_rng: Option<&GameplayRng>,
) -> Result<String, String> {
    let mut payload = game
        .map(SavePayload::from_game_state)
        .transpose()
        .map_err(|error| error.to_string())?;
    if let Some(payload) = payload.as_mut() {
        payload.playtime_seconds = 0;
    }
    let checkpoint = Checkpoint {
        action_schema: NORMALIZED_ACTION_SCHEMA,
        action_index,
        actions,
        app_state: state
            .map(|state| format!("{:?}", state.get()))
            .unwrap_or_else(|| "Uninitialized".to_owned()),
        game: payload,
        startup_rng_state: game
            .is_none()
            .then(|| startup_rng.map(GameplayRng::state))
            .flatten(),
    };
    let canonical = serde_yaml_ng::to_string(&checkpoint).map_err(|error| error.to_string())?;
    Ok(format!("{:x}", Sha256::digest(canonical.as_bytes())))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use bevy::state::app::StatesPlugin;

    use super::*;

    static NEXT_PATH: AtomicU64 = AtomicU64::new(0);

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

    fn automation_app(automation: InputAutomation) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(StatesPlugin)
            .add_message::<AppExit>()
            .init_resource::<ButtonInput<KeyCode>>()
            .insert_resource(GameplayRng::from_seed(42))
            .insert_state(AppState::Title)
            .insert_resource(automation)
            .add_plugins(crate::action_input::ActionInputPlugin)
            .add_plugins(InputRecordPlugin);
        app
    }

    fn temporary_record_path() -> PathBuf {
        std::env::temp_dir().join(format!(
            "rpg-s1-input-record-{}-{}.yaml",
            std::process::id(),
            NEXT_PATH.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn physical_input_records_normalized_actions_and_replays_without_that_mapping() {
        let path = temporary_record_path();
        let mut recorder = automation_app(InputAutomation::record(path.clone(), record()));
        if let InputAutomation::Record { record, .. } = recorder
            .world_mut()
            .resource_mut::<InputAutomation>()
            .as_mut()
        {
            record.action_frames.clear();
        }
        recorder.update();
        recorder
            .world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        recorder.update();

        let recorded = InputRecord::decode(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(recorded.action_frames.len(), 1);
        assert_eq!(recorded.action_frames[0].frame, 1);
        assert_eq!(
            recorded.action_frames[0].actions,
            [NormalizedAction::Confirm]
        );

        let mut replay = automation_app(InputAutomation::replay(path.clone(), recorded));
        replay
            .world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        replay.update();
        assert!(
            !replay
                .world()
                .resource::<ActionState>()
                .just_pressed(crate::action_input::AppAction::Back)
        );
        replay.update();
        assert!(
            replay
                .world()
                .resource::<ActionState>()
                .just_pressed(crate::action_input::AppAction::Confirm)
        );
        let InputAutomation::Replay {
            cursor,
            complete,
            failure,
            ..
        } = replay.world().resource::<InputAutomation>()
        else {
            panic!("replay mode should remain installed");
        };
        assert_eq!(*cursor, 1);
        assert!(*complete);
        assert_eq!(failure, &None);

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn replay_reports_the_first_mismatching_action_checkpoint() {
        let path = temporary_record_path();
        let mut input = record();
        input.action_frames[0].frame = 0;
        input.action_frames[0].state_hash = "0".repeat(64);
        let mut replay = automation_app(InputAutomation::replay(path, input));

        replay.update();

        let InputAutomation::Replay {
            cursor,
            complete,
            failure,
            ..
        } = replay.world().resource::<InputAutomation>()
        else {
            panic!("replay mode should remain installed");
        };
        assert_eq!(*cursor, 0);
        assert!(!*complete);
        assert!(failure.as_deref().unwrap().contains("action 0 at frame 0"));
    }

    fn title_load_trace_app(save_directory: &std::path::Path) -> App {
        let mut app = crate::test_support::headless_title_app(AppState::Title);
        app.add_plugins(crate::save_ui::SaveUiPlugin)
            .add_plugins(InputRecordPlugin)
            .insert_resource(crate::save_store::SaveStore::new(save_directory.to_owned()));
        for _ in 0..5_000 {
            app.update();
            if app
                .world()
                .resource::<crate::save_ui::SaveSlotCatalog>()
                .has_valid()
            {
                return app;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        panic!("production title load catalog did not become ready");
    }

    fn release_keys(app: &mut App, keys: &[KeyCode]) {
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        for key in keys {
            input.release(*key);
        }
        input.clear();
    }

    #[test]
    fn production_title_to_world_trace_replays_to_the_same_final_state_hash() {
        let root = std::env::temp_dir().join(format!(
            "rpg-s1-title-world-replay-{}-{}",
            std::process::id(),
            NEXT_PATH.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("001.yaml"),
            include_bytes!("../../../tests/fixtures/native-save-v1.yaml"),
        )
        .unwrap();
        let record_path = root.join("title-to-world.yaml");
        let empty_record = InputRecord::new(
            "2026.8.1",
            RecordSource {
                package_key: "rusted_kingdoms".to_owned(),
                scenario_id: "my_rpg_story".to_owned(),
                scenario_version: "1.0.0".to_owned(),
            },
            1,
        );

        let mut recorder = title_load_trace_app(&root);
        recorder.insert_resource(InputAutomation::record(record_path.clone(), empty_record));
        {
            let mut input = recorder.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            input.press(KeyCode::ArrowDown);
            input.press(KeyCode::Enter);
        }
        recorder.update();
        release_keys(&mut recorder, &[KeyCode::ArrowDown, KeyCode::Enter]);
        recorder.update();
        recorder
            .world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        recorder.update();
        release_keys(&mut recorder, &[KeyCode::Enter]);
        recorder.update();
        recorder
            .world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowRight);
        recorder.update();
        assert_eq!(
            recorder.world().resource::<State<AppState>>().get(),
            &AppState::World
        );
        let recorded = InputRecord::decode(&fs::read(&record_path).unwrap()).unwrap();
        assert_eq!(recorded.action_frames.len(), 3);
        let expected_final_hash = recorded.action_frames.last().unwrap().state_hash.clone();

        let mut replay = title_load_trace_app(&root);
        replay.insert_resource(InputAutomation::replay(record_path.clone(), recorded));
        for _ in 0..8 {
            replay.update();
            let automation = replay.world().resource::<InputAutomation>();
            let InputAutomation::Replay {
                complete, failure, ..
            } = automation
            else {
                unreachable!()
            };
            assert!(failure.is_none(), "replay diverged: {failure:?}");
            if *complete {
                break;
            }
        }
        let InputAutomation::Replay {
            record,
            cursor,
            complete,
            failure,
            ..
        } = replay.world().resource::<InputAutomation>()
        else {
            unreachable!()
        };
        assert!(*complete);
        assert_eq!(*cursor, 3);
        assert_eq!(failure, &None);
        assert_eq!(
            record.action_frames.last().unwrap().state_hash,
            expected_final_hash
        );
        assert_eq!(
            replay.world().resource::<State<AppState>>().get(),
            &AppState::World
        );

        fs::remove_dir_all(root).unwrap();
    }
}
