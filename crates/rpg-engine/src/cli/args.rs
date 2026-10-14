//! Command-line parsing into validated commands; nothing here touches the filesystem.

use std::{ffi::OsString, fmt, path::PathBuf};

use crate::{
    debug_launch::{DebugBattleFixture, DebugLaunchConfig, DebugPartyPreset},
    gameplay_rng::DEFAULT_GAMEPLAY_SEED,
    input_record::InputAutomation,
    scenario_root::{DEFAULT_SCENARIO_PACKAGE_KEY, ScenarioRoot},
};

pub(super) const USAGE: &str = "Usage:\n  rpg-s1\n  rpg-s1 play [PACKAGE_KEY] [--seed U64] [--timings] [DEBUG_OPTIONS]\n  rpg-s1 record OUTPUT [PACKAGE_KEY] [--seed U64] [--timings] [DEBUG_OPTIONS]\n  rpg-s1 replay INPUT\n  rpg-s1 validate-scenario [PACKAGE_KEY] [--baseline PATH]\n  rpg-s1 map-report [PACKAGE_KEY]\n  rpg-s1 map-sweep [PACKAGE_KEY]\n  rpg-s1 dialogue-report [PACKAGE_KEY]\n  rpg-s1 dialogue-sweep [PACKAGE_KEY]\n  rpg-s1 encounter-sweep [PACKAGE_KEY]\n  rpg-s1 import-python-save INPUT --slot 0..100 [--package PACKAGE_KEY] [--allow-unchecked] [--replace]\n\nDEBUG_OPTIONS:\n  --start-map MAP_ID --start-position X,Y\n  --party-preset solo|full\n  --battle-fixture feedback|status|enemy-ai|rewards\n  --set-flag FLAG_ID\n  --unset-flag FLAG_ID\n\nScenario commands default to package `rusted_kingdoms`. PACKAGE_KEY is a portable package name, not a path. `--baseline` gates validation against a file of accepted diagnostics instead of on validity: it fails on a diagnostic the file omits and on a file entry the report no longer produces. Gameplay defaults to deterministic seed 1. --timings logs world and battle hotspot measurements every 120 frames. Any debug option starts directly in the world; map and position must be supplied together. Battle fixtures enter a focused, session-only fight and default to a safe Starting Forest position. Record refuses to overwrite OUTPUT; replay takes its package, seed, and debug options from INPUT. Python import is explicit, one-way, and never scans for legacy saves.\n\nENVIRONMENT:\n  RPG_S1_PARTY_PRESET=solo|full  Debug party for `rpg-s1` and `play`/`record`; starts directly in the world. --party-preset wins. Ignored by replay.\n  RPG_S1_MUTE_AUDIO              Any value silences audio.\n  RPG_S1_DEBUG_COLLISION         Any value draws portal and collision outlines.\n  RPG_S1_HOT_RELOAD              Any value enables transactional scenario polling in idle World.";

pub(super) enum Command {
    Play(PlayArguments),
    Record {
        output: PathBuf,
        play: PlayArguments,
    },
    Replay(PathBuf),
    Validate(ValidateArguments),
    MapReport(ScenarioRoot),
    MapSweep(ScenarioRoot),
    DialogueReport(ScenarioRoot),
    DialogueSweep(ScenarioRoot),
    EncounterSweep(ScenarioRoot),
    ImportPython(ImportPythonArguments),
    Help,
}

/// Validated process options passed across the window-construction boundary.
#[derive(Debug)]
pub(crate) struct PlayArguments {
    pub(crate) root: ScenarioRoot,
    pub(crate) seed: u64,
    pub(crate) timings: bool,
    pub(crate) debug: Option<DebugLaunchConfig>,
    pub(crate) automation: Option<InputAutomation>,
}

/// Validated `validate-scenario` options.
///
/// `baseline` selects the gating mode: without it the command reports and exits on validity, which
/// is permanently non-zero for the production package's accepted inherited debt; with it the exit
/// code tracks the diff against the accepted set instead, which is what CI can enforce.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ValidateArguments {
    pub(super) root: ScenarioRoot,
    pub(super) baseline: Option<PathBuf>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ImportPythonArguments {
    pub(super) input: PathBuf,
    pub(super) slot: usize,
    pub(super) root: ScenarioRoot,
    pub(super) allow_unchecked: bool,
    pub(super) replace: bool,
}

#[derive(Debug, Eq, PartialEq)]
pub(super) struct UsageError(pub(super) String);

impl fmt::Display for UsageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

pub(super) fn parse_command(
    arguments: impl IntoIterator<Item = OsString>,
) -> Result<Command, UsageError> {
    let arguments = arguments
        .into_iter()
        .map(|argument| {
            argument
                .into_string()
                .map_err(|_| UsageError("arguments must be valid UTF-8".to_owned()))
        })
        .collect::<Result<Vec<_>, _>>()?;

    match arguments.as_slice() {
        [] => Ok(Command::Play(default_play_arguments())),
        [command, flag] if command == "play" && (flag == "-h" || flag == "--help") => {
            Ok(Command::Help)
        }
        [command, rest @ ..] if command == "play" => parse_play_arguments(rest).map(Command::Play),
        [command, flag]
            if (command == "record" || command == "replay")
                && (flag == "-h" || flag == "--help") =>
        {
            Ok(Command::Help)
        }
        [command, output, rest @ ..] if command == "record" && !output.starts_with('-') => {
            Ok(Command::Record {
                output: PathBuf::from(output),
                play: parse_play_arguments(rest)?,
            })
        }
        [command, ..] if command == "record" => {
            Err(UsageError("record requires OUTPUT first".to_owned()))
        }
        [command, input] if command == "replay" && !input.starts_with('-') => {
            Ok(Command::Replay(PathBuf::from(input)))
        }
        [command, ..] if command == "replay" => {
            Err(UsageError("replay requires exactly one INPUT".to_owned()))
        }
        [flag] if flag == "-h" || flag == "--help" => Ok(Command::Help),
        [command, flag] if command == "validate-scenario" && (flag == "-h" || flag == "--help") => {
            Ok(Command::Help)
        }
        [command, flag] if command == "map-report" && (flag == "-h" || flag == "--help") => {
            Ok(Command::Help)
        }
        [command, flag] if command == "dialogue-report" && (flag == "-h" || flag == "--help") => {
            Ok(Command::Help)
        }
        [command, flag] if command == "dialogue-sweep" && (flag == "-h" || flag == "--help") => {
            Ok(Command::Help)
        }
        [command, flag] if command == "encounter-sweep" && (flag == "-h" || flag == "--help") => {
            Ok(Command::Help)
        }
        [command, flag] if command == "map-sweep" && (flag == "-h" || flag == "--help") => {
            Ok(Command::Help)
        }
        [command, flag]
            if command == "import-python-save" && (flag == "-h" || flag == "--help") =>
        {
            Ok(Command::Help)
        }
        [command, rest @ ..] if command == "validate-scenario" => {
            parse_validate_arguments(rest).map(Command::Validate)
        }
        [command] if command == "map-report" => Ok(Command::MapReport(
            ScenarioRoot::try_for_package_key(DEFAULT_SCENARIO_PACKAGE_KEY)
                .expect("the default package key is valid"),
        )),
        [command, package_key] if command == "map-report" => {
            ScenarioRoot::try_for_package_key(package_key.clone())
                .map(Command::MapReport)
                .map_err(|error| {
                    UsageError(format!("invalid package key `{package_key}`: {error}"))
                })
        }
        [command, ..] if command == "map-report" => Err(UsageError(
            "map-report accepts at most one package key".to_owned(),
        )),
        [command] if command == "map-sweep" => Ok(Command::MapSweep(
            ScenarioRoot::try_for_package_key(DEFAULT_SCENARIO_PACKAGE_KEY)
                .expect("the default package key is valid"),
        )),
        [command, package_key] if command == "map-sweep" => {
            ScenarioRoot::try_for_package_key(package_key.clone())
                .map(Command::MapSweep)
                .map_err(|error| {
                    UsageError(format!("invalid package key `{package_key}`: {error}"))
                })
        }
        [command, ..] if command == "map-sweep" => Err(UsageError(
            "map-sweep accepts at most one package key".to_owned(),
        )),
        [command] if command == "dialogue-report" => Ok(Command::DialogueReport(
            ScenarioRoot::try_for_package_key(DEFAULT_SCENARIO_PACKAGE_KEY)
                .expect("the default package key is valid"),
        )),
        [command, package_key] if command == "dialogue-report" => {
            ScenarioRoot::try_for_package_key(package_key.clone())
                .map(Command::DialogueReport)
                .map_err(|error| {
                    UsageError(format!("invalid package key `{package_key}`: {error}"))
                })
        }
        [command, ..] if command == "dialogue-report" => Err(UsageError(
            "dialogue-report accepts at most one package key".to_owned(),
        )),
        [command] if command == "dialogue-sweep" => Ok(Command::DialogueSweep(
            ScenarioRoot::try_for_package_key(DEFAULT_SCENARIO_PACKAGE_KEY)
                .expect("the default package key is valid"),
        )),
        [command, package_key] if command == "dialogue-sweep" => {
            ScenarioRoot::try_for_package_key(package_key.clone())
                .map(Command::DialogueSweep)
                .map_err(|error| {
                    UsageError(format!("invalid package key `{package_key}`: {error}"))
                })
        }
        [command, ..] if command == "dialogue-sweep" => Err(UsageError(
            "dialogue-sweep accepts at most one package key".to_owned(),
        )),
        [command] if command == "encounter-sweep" => Ok(Command::EncounterSweep(
            ScenarioRoot::try_for_package_key(DEFAULT_SCENARIO_PACKAGE_KEY)
                .expect("the default package key is valid"),
        )),
        [command, package_key] if command == "encounter-sweep" => {
            ScenarioRoot::try_for_package_key(package_key.clone())
                .map(Command::EncounterSweep)
                .map_err(|error| {
                    UsageError(format!("invalid package key `{package_key}`: {error}"))
                })
        }
        [command, ..] if command == "encounter-sweep" => Err(UsageError(
            "encounter-sweep accepts at most one package key".to_owned(),
        )),
        [command, rest @ ..] if command == "import-python-save" => {
            parse_import_python_arguments(rest).map(Command::ImportPython)
        }
        [command, ..] => Err(UsageError(format!("unknown command `{command}`"))),
    }
}

/// Set to `solo` or `full` to launch straight into the world with that debug party.
///
/// This exists so a launcher entry (see `menu.toml`) can start a debug session with no
/// CLI arguments at all, following the same env-var precedent as `RPG_S1_MUTE_AUDIO`
/// and `RPG_S1_DEBUG_COLLISION`. An explicit `--party-preset` always wins.
pub(crate) const PARTY_PRESET_ENV_VAR: &str = "RPG_S1_PARTY_PRESET";

fn parse_party_preset(value: &str) -> Option<DebugPartyPreset> {
    match value {
        "solo" => Some(DebugPartyPreset::Solo),
        "full" => Some(DebugPartyPreset::Full),
        _ => None,
    }
}

/// Fills in environment-provided debug defaults on an already-parsed command.
///
/// `Replay` is deliberately excluded: its debug config comes from the record, so a replay
/// stays faithful to what was recorded no matter what the environment says. `Record` is
/// included, so a session launched this way records the preset it actually ran with.
pub(super) fn apply_environment_debug_defaults(
    command: &mut Command,
    environment: impl Fn(&str) -> Option<OsString>,
) -> Result<(), UsageError> {
    let play = match command {
        Command::Play(play) | Command::Record { play, .. } => play,
        _ => return Ok(()),
    };
    let Some(raw) = environment(PARTY_PRESET_ENV_VAR) else {
        return Ok(());
    };
    let value = raw
        .into_string()
        .map_err(|_| UsageError(format!("{PARTY_PRESET_ENV_VAR} must be valid UTF-8")))?;
    let preset = parse_party_preset(value.trim())
        .ok_or_else(|| UsageError(format!("{PARTY_PRESET_ENV_VAR} requires solo|full")))?;
    let mut debug = play.debug.take().unwrap_or_default();
    if debug.party_preset.is_none() {
        debug.party_preset = Some(preset);
    }
    play.debug = debug.is_active().then_some(debug);
    Ok(())
}

fn default_play_arguments() -> PlayArguments {
    PlayArguments {
        root: ScenarioRoot::try_for_package_key(DEFAULT_SCENARIO_PACKAGE_KEY)
            .expect("the default package key is valid"),
        seed: DEFAULT_GAMEPLAY_SEED,
        timings: false,
        debug: None,
        automation: None,
    }
}

fn parse_validate_arguments(arguments: &[String]) -> Result<ValidateArguments, UsageError> {
    let mut package_key: Option<&String> = None;
    let mut baseline: Option<PathBuf> = None;
    let mut remaining = arguments.iter();
    while let Some(argument) = remaining.next() {
        match argument.as_str() {
            "--baseline" => {
                let path = remaining
                    .next()
                    .ok_or_else(|| UsageError("--baseline requires a PATH".to_owned()))?;
                if baseline.replace(PathBuf::from(path)).is_some() {
                    return Err(UsageError("--baseline accepts one PATH".to_owned()));
                }
            }
            value if value.starts_with('-') => {
                return Err(UsageError(format!(
                    "unknown validate-scenario option `{value}`"
                )));
            }
            _ => {
                if package_key.replace(argument).is_some() {
                    return Err(UsageError(
                        "validate-scenario accepts at most one package key".to_owned(),
                    ));
                }
            }
        }
    }
    let key = package_key
        .map(String::as_str)
        .unwrap_or(DEFAULT_SCENARIO_PACKAGE_KEY);
    let root = ScenarioRoot::try_for_package_key(key)
        .map_err(|error| UsageError(format!("invalid package key `{key}`: {error}")))?;
    Ok(ValidateArguments { root, baseline })
}

fn parse_play_arguments(arguments: &[String]) -> Result<PlayArguments, UsageError> {
    let mut package = None;
    let mut seed = None;
    let mut timings = false;
    let mut start_map = None;
    let mut start_position = None;
    let mut party_preset = None;
    let mut battle_fixture = None;
    let mut flag_overrides = std::collections::BTreeMap::new();
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--seed" => {
                index += 1;
                let value = arguments
                    .get(index)
                    .ok_or_else(|| UsageError("--seed requires a U64 value".to_owned()))?;
                let parsed = value
                    .parse::<u64>()
                    .map_err(|_| UsageError("--seed requires a U64 value".to_owned()))?;
                if seed.replace(parsed).is_some() {
                    return Err(UsageError("--seed may appear only once".to_owned()));
                }
            }
            "--timings" if !timings => timings = true,
            "--start-map" => {
                index += 1;
                let value = arguments
                    .get(index)
                    .ok_or_else(|| UsageError("--start-map requires MAP_ID".to_owned()))?;
                if value.is_empty() || start_map.replace(value.clone()).is_some() {
                    return Err(UsageError(
                        "--start-map must appear once with a nonempty MAP_ID".to_owned(),
                    ));
                }
            }
            "--start-position" => {
                index += 1;
                let value = arguments
                    .get(index)
                    .ok_or_else(|| UsageError("--start-position requires X,Y".to_owned()))?;
                let (x, y) = value
                    .split_once(',')
                    .ok_or_else(|| UsageError("--start-position requires X,Y".to_owned()))?;
                let position = crate::scenario_spatial::Position::new(
                    x.parse()
                        .map_err(|_| UsageError("--start-position requires X,Y".to_owned()))?,
                    y.parse()
                        .map_err(|_| UsageError("--start-position requires X,Y".to_owned()))?,
                );
                if start_position.replace(position).is_some() {
                    return Err(UsageError(
                        "--start-position may appear only once".to_owned(),
                    ));
                }
            }
            "--party-preset" => {
                index += 1;
                let value = arguments
                    .get(index)
                    .ok_or_else(|| UsageError("--party-preset requires solo|full".to_owned()))?;
                let preset = parse_party_preset(value.as_str())
                    .ok_or_else(|| UsageError("--party-preset requires solo|full".to_owned()))?;
                if party_preset.replace(preset).is_some() {
                    return Err(UsageError("--party-preset may appear only once".to_owned()));
                }
            }
            "--battle-fixture" => {
                index += 1;
                let value = arguments.get(index).ok_or_else(|| {
                    UsageError(
                        "--battle-fixture requires feedback|status|enemy-ai|rewards".to_owned(),
                    )
                })?;
                let fixture = match value.as_str() {
                    "feedback" => DebugBattleFixture::Feedback,
                    "status" => DebugBattleFixture::Status,
                    "enemy-ai" => DebugBattleFixture::EnemyAi,
                    "rewards" => DebugBattleFixture::Rewards,
                    _ => {
                        return Err(UsageError(
                            "--battle-fixture requires feedback|status|enemy-ai|rewards".to_owned(),
                        ));
                    }
                };
                if battle_fixture.replace(fixture).is_some() {
                    return Err(UsageError(
                        "--battle-fixture may appear only once".to_owned(),
                    ));
                }
            }
            option @ ("--set-flag" | "--unset-flag") => {
                index += 1;
                let value = arguments
                    .get(index)
                    .ok_or_else(|| UsageError(format!("{option} requires FLAG_ID")))?;
                if !valid_debug_flag_id(value) || flag_overrides.contains_key(value) {
                    return Err(UsageError(format!(
                        "{option} requires a unique nonempty FLAG_ID"
                    )));
                }
                flag_overrides.insert(value.clone(), option == "--set-flag");
            }
            option if option.starts_with('-') => {
                return Err(UsageError(format!("unknown play option `{option}`")));
            }
            package_key => {
                if package.replace(package_key.to_owned()).is_some() {
                    return Err(UsageError(
                        "play accepts at most one package key".to_owned(),
                    ));
                }
            }
        }
        index += 1;
    }
    let package = package.unwrap_or_else(|| DEFAULT_SCENARIO_PACKAGE_KEY.to_owned());
    let root = ScenarioRoot::try_for_package_key(package.clone())
        .map_err(|error| UsageError(format!("invalid package key `{package}`: {error}")))?;
    if start_map.is_some() != start_position.is_some() {
        return Err(UsageError(
            "--start-map and --start-position must be supplied together".to_owned(),
        ));
    }
    if battle_fixture.is_some() && start_map.is_none() {
        start_map = Some("zone_01_starting_forest".to_owned());
        start_position = Some(crate::scenario_spatial::Position::new(29, 1));
    }
    if let Some(fixture) = battle_fixture {
        let required = match fixture {
            DebugBattleFixture::EnemyAi | DebugBattleFixture::Rewards => DebugPartyPreset::Solo,
            DebugBattleFixture::Feedback | DebugBattleFixture::Status => DebugPartyPreset::Full,
        };
        match party_preset {
            None => party_preset = Some(required),
            Some(actual) if actual != required => {
                return Err(UsageError(format!(
                    "--battle-fixture {} requires --party-preset {}",
                    battle_fixture_label(fixture),
                    match required {
                        DebugPartyPreset::Solo => "solo",
                        DebugPartyPreset::Full => "full",
                    }
                )));
            }
            Some(_) => {}
        }
    }
    let debug = DebugLaunchConfig {
        start_map,
        start_position,
        party_preset,
        battle_fixture,
        flag_overrides,
    };
    Ok(PlayArguments {
        root,
        seed: seed.unwrap_or(DEFAULT_GAMEPLAY_SEED),
        timings,
        debug: debug.is_active().then_some(debug),
        automation: None,
    })
}

fn valid_debug_flag_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

pub(super) const fn battle_fixture_label(fixture: DebugBattleFixture) -> &'static str {
    match fixture {
        DebugBattleFixture::Feedback => "feedback",
        DebugBattleFixture::Status => "status",
        DebugBattleFixture::EnemyAi => "enemy-ai",
        DebugBattleFixture::Rewards => "rewards",
    }
}

fn parse_import_python_arguments(
    arguments: &[String],
) -> Result<ImportPythonArguments, UsageError> {
    let Some(input) = arguments.first() else {
        return Err(UsageError("import-python-save requires INPUT".to_owned()));
    };
    if input.starts_with('-') {
        return Err(UsageError(
            "import-python-save INPUT must come first".to_owned(),
        ));
    }
    let mut slot = None;
    let mut package = DEFAULT_SCENARIO_PACKAGE_KEY.to_owned();
    let mut allow_unchecked = false;
    let mut replace = false;
    let mut index = 1;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--slot" => {
                index += 1;
                let value = arguments
                    .get(index)
                    .ok_or_else(|| UsageError("--slot requires a value".to_owned()))?;
                let parsed = value
                    .parse::<usize>()
                    .map_err(|_| UsageError("--slot must be an integer in 0..=100".to_owned()))?;
                if parsed > 100 || slot.replace(parsed).is_some() {
                    return Err(UsageError(
                        "--slot must appear once with a value in 0..=100".to_owned(),
                    ));
                }
            }
            "--package" => {
                index += 1;
                package = arguments
                    .get(index)
                    .ok_or_else(|| UsageError("--package requires a value".to_owned()))?
                    .clone();
            }
            "--allow-unchecked" if !allow_unchecked => allow_unchecked = true,
            "--replace" if !replace => replace = true,
            option => {
                return Err(UsageError(format!(
                    "unknown or repeated import-python-save option `{option}`"
                )));
            }
        }
        index += 1;
    }
    let slot = slot.ok_or_else(|| UsageError("import-python-save requires --slot".to_owned()))?;
    let root = ScenarioRoot::try_for_package_key(package.clone())
        .map_err(|error| UsageError(format!("invalid package key `{package}`: {error}")))?;
    Ok(ImportPythonArguments {
        input: PathBuf::from(input),
        slot,
        root,
        allow_unchecked,
        replace,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_requires_a_path_and_the_package_key_may_come_either_side_of_it() {
        assert_eq!(
            parse_validate_arguments(&["--baseline".to_owned()]),
            Err(UsageError("--baseline requires a PATH".to_owned()))
        );
        assert_eq!(
            parse_validate_arguments(&["--unknown".to_owned()]),
            Err(UsageError(
                "unknown validate-scenario option `--unknown`".to_owned()
            ))
        );
        assert_eq!(
            parse_validate_arguments(&["one".to_owned(), "two".to_owned()]),
            Err(UsageError(
                "validate-scenario accepts at most one package key".to_owned()
            ))
        );

        let leading = parse_validate_arguments(&[
            "fixture".to_owned(),
            "--baseline".to_owned(),
            "accepted.txt".to_owned(),
        ])
        .expect("a package key before the flag should parse");
        let trailing = parse_validate_arguments(&[
            "--baseline".to_owned(),
            "accepted.txt".to_owned(),
            "fixture".to_owned(),
        ])
        .expect("a package key after the flag should parse");
        assert_eq!(leading, trailing);
        assert_eq!(leading.baseline, Some(PathBuf::from("accepted.txt")));
        assert_eq!(leading.root.package_key(), "fixture");

        let bare = parse_validate_arguments(&[]).expect("a bare command should parse");
        assert_eq!(bare.baseline, None);
        assert_eq!(bare.root.package_key(), DEFAULT_SCENARIO_PACKAGE_KEY);
    }
}
