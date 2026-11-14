//! Every `RPG_S1_*` environment variable the engine reads, in one place.
//!
//! The player-facing switches are listed in the `ENVIRONMENT:` section of the CLI help (`USAGE` in
//! `cli::args`). The test-only names at the end are opt-in hooks for regression runs and editor
//! writer checks, so they are left out of the help text.

/// Overrides the save directory. Read as a path; an empty value is an error.
pub(crate) const SAVE_DIR: &str = "RPG_S1_SAVE_DIR";
/// Overrides the options directory. Read as a path; an empty value is an error.
pub(crate) const CONFIG_DIR: &str = "RPG_S1_CONFIG_DIR";
/// Debug party for `play` and `record`: `solo` or `full`. Read as a value; `--party-preset` wins.
pub(crate) const PARTY_PRESET: &str = "RPG_S1_PARTY_PRESET";
/// Silences audio at launch. Presence check: any value counts.
pub(crate) const MUTE_AUDIO: &str = "RPG_S1_MUTE_AUDIO";
/// Draws portal and collision outlines in the World. Presence check: any value counts.
pub(crate) const DEBUG_COLLISION: &str = "RPG_S1_DEBUG_COLLISION";
/// Enables transactional scenario polling in the idle World. Presence check: any value counts.
pub(crate) const HOT_RELOAD: &str = "RPG_S1_HOT_RELOAD";
/// TEMPORARY v2 status-pose review switch. Read as a value: `0` is off, any other value is on.
pub(crate) const STATUS_TEST: &str = "RPG_S1_STATUS_TEST";

/// Test-only: directory that receives a copy of the persistence fixture YAML for the persistence export checklist.
#[cfg(test)]
pub(crate) const RF10_EXPORT: &str = "RPG_S1_RF10_EXPORT";
/// Test-only: phase name a re-run persistence test child reads to pick its step.
#[cfg(test)]
pub(crate) const REGRESSION_CHILD: &str = "RPG_S1_REGRESSION_CHILD";
/// Test-only: fixture root a re-run persistence test child saves into.
#[cfg(test)]
pub(crate) const REGRESSION_ROOT: &str = "RPG_S1_REGRESSION_ROOT";
/// Test-only: Python interpreter for the editor writer checks. Defaults to `python3`.
#[cfg(test)]
pub(crate) const EDITOR_TEST_PYTHON: &str = "RPG_S1_EDITOR_TEST_PYTHON";

#[cfg(test)]
mod tests {
    use super::*;

    /// The switches a player can set, which the CLI help must document.
    const DOCUMENTED: [&str; 6] = [
        PARTY_PRESET,
        MUTE_AUDIO,
        DEBUG_COLLISION,
        HOT_RELOAD,
        SAVE_DIR,
        CONFIG_DIR,
    ];

    #[test]
    fn every_documented_switch_has_a_line_in_the_cli_environment_help() {
        for name in DOCUMENTED {
            assert!(
                crate::cli::USAGE
                    .lines()
                    .any(|line| line.trim_start().starts_with(name)),
                "{name} is missing from the ENVIRONMENT section of the CLI help"
            );
        }
    }
}
