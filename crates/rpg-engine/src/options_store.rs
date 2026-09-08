//! Player-set options, persisted outside both the asset tree and the save slots.
//!
//! Three places could have held these, and the other two are wrong for a reason worth recording.
//!
//! `assets/settings.yaml` is the source-parity file described in
//! [`crate::engine_config`]: it ships with the game, it is read-only in practice, and its whole
//! discipline is that the port carries only keys the Python original had. Rebindings are a port
//! addition with no source counterpart, so writing them there would corrupt that contract.
//!
//! A save slot is wrong because bindings belong to the player, not the campaign. Starting a new
//! game would silently restore factory keys, and it would force a save-schema change on work that
//! has otherwise avoided one.
//!
//! So options live in the XDG *config* directory — the spec's home for preferences a user sets,
//! as opposed to the data directory where [`crate::save_store`] keeps saves. Keeping the two
//! apart also matters in practice: acceptance runs move `RPG_S1_SAVE_DIR` between fixture slots
//! constantly, and bindings must not reset every time they do.

use std::{
    collections::BTreeMap,
    error::Error,
    ffi::OsString,
    fmt,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    engine_config::TextSpeed,
    input_bindings::{AppAction, BindingTarget, InputBindings, MovementAction},
    input_names::{gamepad_button_from_name, gamepad_button_name, key_from_name, key_name},
};

pub(crate) const CONFIG_DIRECTORY_OVERRIDE: &str = "RPG_S1_CONFIG_DIR";
const OPTIONS_FILE_NAME: &str = "options.yaml";

/// The schema version this build writes.
///
/// A file from a newer build is not read: silently dropping keys it understands and we do not
/// would look to the player like the game forgetting their settings.
const CURRENT_VERSION: u32 = 1;

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum OptionsStoreError {
    InvalidDirectory(String),
    UnsupportedVersion(u32),
    Parse(String),
    Encode(String),
    Io {
        operation: &'static str,
        error: String,
    },
}

impl fmt::Display for OptionsStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDirectory(reason) => {
                write!(formatter, "invalid config directory: {reason}")
            }
            Self::UnsupportedVersion(version) => write!(
                formatter,
                "options file version {version} is newer than this build's {CURRENT_VERSION}"
            ),
            Self::Parse(reason) => write!(formatter, "could not parse options: {reason}"),
            Self::Encode(reason) => write!(formatter, "could not encode options: {reason}"),
            Self::Io { operation, error } => write!(formatter, "{operation} failed: {error}"),
        }
    }
}

impl Error for OptionsStoreError {}

/// Everything the options screen can change.
#[derive(Clone, Debug, Default, PartialEq, Resource)]
pub(crate) struct PlayerOptions {
    pub(crate) bindings: InputBindings,
    /// The player's typewriter-speed override, or `None` to follow `assets/settings.yaml`.
    pub(crate) text_speed: Option<TextSpeed>,
}

// ---------------------------------------------------------------------------
// On-disk shape
// ---------------------------------------------------------------------------

/// Bindings are stored as name lists keyed by action, not as a fixed struct.
///
/// A map keeps a hand-edited file forgiving in the direction that matters: omit an action and it
/// keeps its default, rather than the whole block failing. A misspelled action cannot be honored,
/// so it is collected as a warning and reported, never dropped in silence.
type BindingRows = BTreeMap<String, Vec<String>>;

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
struct DeviceBindings {
    menu: BindingRows,
    movement: BindingRows,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
struct OptionsDocument {
    version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    text_speed: Option<String>,
    keyboard: DeviceBindings,
    gamepad: DeviceBindings,
}

const fn text_speed_name(speed: TextSpeed) -> &'static str {
    match speed {
        TextSpeed::Slow => "slow",
        TextSpeed::Fast => "fast",
        TextSpeed::VeryFast => "very_fast",
    }
}

fn text_speed_from_name(name: &str) -> Option<TextSpeed> {
    match name {
        "slow" => Some(TextSpeed::Slow),
        "fast" => Some(TextSpeed::Fast),
        "very_fast" => Some(TextSpeed::VeryFast),
        _ => None,
    }
}

impl OptionsDocument {
    fn from_options(options: &PlayerOptions) -> Self {
        let mut document = Self {
            version: CURRENT_VERSION,
            text_speed: options
                .text_speed
                .map(|speed| text_speed_name(speed).to_owned()),
            ..Self::default()
        };
        for target in BindingTarget::all() {
            let (key_rows, button_rows) = match target {
                BindingTarget::Menu(_) => (&mut document.keyboard.menu, &mut document.gamepad.menu),
                BindingTarget::Movement(_) => (
                    &mut document.keyboard.movement,
                    &mut document.gamepad.movement,
                ),
            };
            let file_key = match target {
                BindingTarget::Menu(action) => action.file_key(),
                BindingTarget::Movement(action) => action.file_key(),
            };
            key_rows.insert(
                file_key.to_owned(),
                options
                    .bindings
                    .keys(target)
                    .iter()
                    .filter_map(|key| key_name(*key).map(str::to_owned))
                    .collect(),
            );
            button_rows.insert(
                file_key.to_owned(),
                options
                    .bindings
                    .buttons(target)
                    .iter()
                    .filter_map(|button| gamepad_button_name(*button).map(str::to_owned))
                    .collect(),
            );
        }
        document
    }

    /// Folds the document onto the shipped defaults, reporting anything it could not honor.
    ///
    /// Every recoverable problem returns a warning and leaves that one row at its default. Only a
    /// version this build cannot read is fatal, because that is the one case where continuing
    /// would discard the player's real settings rather than one malformed line.
    fn into_options(self) -> Result<(PlayerOptions, Vec<String>), OptionsStoreError> {
        if self.version > CURRENT_VERSION {
            return Err(OptionsStoreError::UnsupportedVersion(self.version));
        }

        let mut warnings = Vec::new();
        let mut options = PlayerOptions::default();

        if let Some(name) = &self.text_speed {
            match text_speed_from_name(name) {
                Some(speed) => options.text_speed = Some(speed),
                None => warnings.push(format!(
                    "text_speed \"{name}\" is not one of slow, fast, very_fast; ignoring it"
                )),
            }
        }

        for (block, rows) in [
            ("keyboard.menu", &self.keyboard.menu),
            ("keyboard.movement", &self.keyboard.movement),
            ("gamepad.menu", &self.gamepad.menu),
            ("gamepad.movement", &self.gamepad.movement),
        ] {
            let is_menu = block.ends_with("menu");
            let is_keyboard = block.starts_with("keyboard");
            for (action_name, names) in rows {
                let Some(target) = binding_target(is_menu, action_name) else {
                    warnings.push(format!(
                        "{block}.{action_name} is not a known action; ignoring it"
                    ));
                    continue;
                };
                let result = if is_keyboard {
                    resolve_names(names, key_from_name).and_then(|keys| {
                        options
                            .bindings
                            .set_keys(target, keys)
                            .map_err(|error| error.message())
                    })
                } else {
                    resolve_names(names, gamepad_button_from_name).and_then(|buttons| {
                        options
                            .bindings
                            .set_buttons(target, buttons)
                            .map_err(|error| error.message())
                    })
                };
                if let Err(reason) = result {
                    warnings.push(format!(
                        "{block}.{action_name} kept its default binding: {reason}"
                    ));
                }
            }
        }

        Ok((options, warnings))
    }
}

fn resolve_names<T>(
    names: &[String],
    lookup: impl Fn(&str) -> Option<T>,
) -> Result<Vec<T>, String> {
    names
        .iter()
        .map(|name| lookup(name).ok_or_else(|| format!("\"{name}\" is not a bindable input")))
        .collect()
}

fn binding_target(is_menu: bool, action_name: &str) -> Option<BindingTarget> {
    if is_menu {
        AppAction::ALL
            .into_iter()
            .find(|action| action.file_key() == action_name)
            .map(BindingTarget::Menu)
    } else {
        MovementAction::ALL
            .into_iter()
            .find(|action| action.file_key() == action_name)
            .map(BindingTarget::Movement)
    }
}

// ---------------------------------------------------------------------------
// Filesystem
// ---------------------------------------------------------------------------

/// The directory holding `options.yaml`, following the XDG base-directory spec.
///
/// Deliberately parallel to [`crate::save_store::resolve_save_directory`], including the explicit
/// rejection of a relative `XDG_CONFIG_HOME`: the spec says a relative value must be ignored, and
/// silently writing a `.config` tree into the current directory is the failure that produces.
pub(crate) fn resolve_config_directory(
    environment: impl Fn(&str) -> Option<OsString>,
) -> Result<PathBuf, OptionsStoreError> {
    if let Some(override_path) = environment(CONFIG_DIRECTORY_OVERRIDE) {
        let path = PathBuf::from(override_path);
        if path.as_os_str().is_empty() {
            return Err(OptionsStoreError::InvalidDirectory(format!(
                "{CONFIG_DIRECTORY_OVERRIDE} must not be empty"
            )));
        }
        return Ok(path);
    }
    if let Some(config_home) = environment("XDG_CONFIG_HOME") {
        let path = PathBuf::from(config_home);
        if path.is_absolute() {
            return Ok(path.join("rpg-s1"));
        }
        return Err(OptionsStoreError::InvalidDirectory(
            "XDG_CONFIG_HOME must be absolute".to_owned(),
        ));
    }
    let home = environment("HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or_else(|| {
            OptionsStoreError::InvalidDirectory(
                "HOME must be an absolute path when XDG_CONFIG_HOME is unset".to_owned(),
            )
        })?;
    Ok(home.join(".config/rpg-s1"))
}

#[derive(Clone, Debug, Resource)]
pub(crate) struct OptionsStore {
    root: PathBuf,
}

impl OptionsStore {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub(crate) fn path(&self) -> PathBuf {
        self.root.join(OPTIONS_FILE_NAME)
    }

    /// Reads the options file, or returns the shipped defaults when none exists yet.
    pub(crate) fn load(&self) -> Result<(PlayerOptions, Vec<String>), OptionsStoreError> {
        let path = self.path();
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok((PlayerOptions::default(), Vec::new()));
            }
            Err(error) => {
                return Err(OptionsStoreError::Io {
                    operation: "read options",
                    error: error.to_string(),
                });
            }
        };
        let document: OptionsDocument = serde_yaml_ng::from_str(&text)
            .map_err(|error| OptionsStoreError::Parse(error.to_string()))?;
        document.into_options()
    }

    /// Writes the options file atomically, so a crash mid-write cannot truncate it.
    ///
    /// The same temp-write, fsync, rename, fsync-directory sequence [`crate::save_store`] uses.
    /// Options are smaller and less precious than a save, but they are written from a settings
    /// screen the player may quit out of immediately, which is exactly when a partial file lands.
    pub(crate) fn store(&self, options: &PlayerOptions) -> Result<PathBuf, OptionsStoreError> {
        fs::create_dir_all(&self.root).map_err(|error| OptionsStoreError::Io {
            operation: "create config directory",
            error: error.to_string(),
        })?;
        let document = OptionsDocument::from_options(options);
        let body = serde_yaml_ng::to_string(&document)
            .map_err(|error| OptionsStoreError::Encode(error.to_string()))?;
        let bytes = format!(
            "# rpg-s1 player options. Delete this file to return to the shipped defaults.\n{body}"
        )
        .into_bytes();

        let temp = self.root.join(format!(
            ".{OPTIONS_FILE_NAME}.{}.{}.tmp",
            std::process::id(),
            NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let mut cleanup = TemporaryFile::new(temp.clone());
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp)
            .map_err(|error| OptionsStoreError::Io {
                operation: "create temporary options",
                error: error.to_string(),
            })?;
        file.write_all(&bytes)
            .map_err(|error| OptionsStoreError::Io {
                operation: "write temporary options",
                error: error.to_string(),
            })?;
        file.sync_all().map_err(|error| OptionsStoreError::Io {
            operation: "sync temporary options",
            error: error.to_string(),
        })?;
        drop(file);

        let mut verified = String::new();
        File::open(&temp)
            .and_then(|mut file| file.read_to_string(&mut verified))
            .map_err(|error| OptionsStoreError::Io {
                operation: "verify temporary options",
                error: error.to_string(),
            })?;
        let reread: OptionsDocument = serde_yaml_ng::from_str(&verified)
            .map_err(|error| OptionsStoreError::Parse(error.to_string()))?;
        if reread != document {
            return Err(OptionsStoreError::Encode(
                "options did not survive a write and re-read".to_owned(),
            ));
        }

        let destination = self.path();
        fs::rename(&temp, &destination).map_err(|error| OptionsStoreError::Io {
            operation: "atomically replace options",
            error: error.to_string(),
        })?;
        cleanup.disarm();
        File::open(&self.root)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| OptionsStoreError::Io {
                operation: "sync config directory",
                error: error.to_string(),
            })?;
        Ok(destination)
    }
}

impl FromWorld for OptionsStore {
    fn from_world(_: &mut World) -> Self {
        Self::new(
            resolve_config_directory(|name| std::env::var_os(name))
                .unwrap_or_else(|error| panic!("could not resolve config directory: {error}")),
        )
    }
}

struct TemporaryFile {
    path: PathBuf,
    armed: bool,
}

impl TemporaryFile {
    fn new(path: PathBuf) -> Self {
        Self { path, armed: true }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for TemporaryFile {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

/// Reads options at startup, reporting rather than propagating a recoverable problem.
///
/// A bad options file must never stop the game from launching: the player would have no way in to
/// fix it. Warnings name the exact row so a hand-edit can be corrected.
pub(crate) fn load_player_options(store: &OptionsStore) -> PlayerOptions {
    match store.load() {
        Ok((options, warnings)) => {
            for warning in warnings {
                warn!("options: {warning}");
            }
            options
        }
        Err(error) => {
            error!(
                "options ignored, using defaults: {error} ({})",
                store.path().display()
            );
            PlayerOptions::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::input::{gamepad::GamepadButton, keyboard::KeyCode};

    fn resolve(pairs: &[(&str, &str)]) -> Result<PathBuf, OptionsStoreError> {
        resolve_config_directory(|name| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| OsString::from(value))
        })
    }

    fn temporary_store(label: &str) -> OptionsStore {
        let root = std::env::temp_dir().join(format!(
            "rpg-s1-options-{label}-{}-{}",
            std::process::id(),
            NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        OptionsStore::new(root)
    }

    #[test]
    fn the_override_outranks_xdg_which_outranks_home() {
        assert_eq!(
            resolve(&[
                (CONFIG_DIRECTORY_OVERRIDE, "/tmp/custom-config"),
                ("XDG_CONFIG_HOME", "/tmp/xdg"),
                ("HOME", "/tmp/home"),
            ]),
            Ok(PathBuf::from("/tmp/custom-config"))
        );
        assert_eq!(
            resolve(&[("XDG_CONFIG_HOME", "/tmp/xdg"), ("HOME", "/tmp/home")]),
            Ok(PathBuf::from("/tmp/xdg/rpg-s1"))
        );
        assert_eq!(
            resolve(&[("HOME", "/tmp/home")]),
            Ok(PathBuf::from("/tmp/home/.config/rpg-s1"))
        );
    }

    #[test]
    fn a_relative_or_missing_base_directory_is_an_error_not_a_guess() {
        assert!(matches!(
            resolve(&[("XDG_CONFIG_HOME", "relative/path"), ("HOME", "/tmp/home")]),
            Err(OptionsStoreError::InvalidDirectory(_))
        ));
        assert!(matches!(
            resolve(&[("HOME", "also/relative")]),
            Err(OptionsStoreError::InvalidDirectory(_))
        ));
        assert!(matches!(
            resolve(&[]),
            Err(OptionsStoreError::InvalidDirectory(_))
        ));
        assert!(matches!(
            resolve(&[(CONFIG_DIRECTORY_OVERRIDE, "")]),
            Err(OptionsStoreError::InvalidDirectory(_))
        ));
    }

    #[test]
    fn the_config_directory_is_not_the_save_directory() {
        // The whole point of a separate resolver: moving saves for a fixture run must not move
        // the player's bindings with them.
        let saves = crate::save_store::resolve_save_directory(|name| {
            (name == "HOME").then(|| OsString::from("/tmp/home"))
        })
        .expect("a save directory");
        let config = resolve(&[("HOME", "/tmp/home")]).expect("a config directory");

        assert_ne!(saves, config);
        assert!(!config.starts_with(&saves));
    }

    #[test]
    fn a_missing_file_yields_the_shipped_defaults() {
        let store = temporary_store("missing");

        let (options, warnings) = store.load().expect("a missing file is not an error");

        assert_eq!(options, PlayerOptions::default());
        assert!(warnings.is_empty());
        assert!(!store.path().exists(), "loading must not create the file");
    }

    #[test]
    fn options_survive_a_store_and_load_round_trip() {
        let store = temporary_store("round-trip");
        let mut options = PlayerOptions {
            text_speed: Some(TextSpeed::Slow),
            ..PlayerOptions::default()
        };
        options
            .bindings
            .bind_key(BindingTarget::Menu(AppAction::Travel), KeyCode::KeyG)
            .expect("a free key");
        options
            .bindings
            .bind_button(
                BindingTarget::Movement(MovementAction::Up),
                GamepadButton::North,
            )
            .expect("a free button");

        store.store(&options).expect("a writable directory");
        let (loaded, warnings) = store.load().expect("the file just written");

        assert_eq!(loaded, options);
        assert!(warnings.is_empty(), "a file we wrote must load cleanly");
        let _ = fs::remove_dir_all(store.root);
    }

    #[test]
    fn the_written_file_is_readable_yaml_with_a_guiding_comment() {
        let store = temporary_store("shape");
        store.store(&PlayerOptions::default()).expect("a write");

        let text = fs::read_to_string(store.path()).expect("the file");

        assert!(text.starts_with("# rpg-s1 player options."));
        assert!(text.contains("version: 1"));
        assert!(text.contains("confirm:"), "actions are named, not indexed");
        assert!(text.contains("NumpadEnter"));
        assert!(
            !text.contains("text_speed"),
            "an unset override is omitted rather than written as null"
        );
        let _ = fs::remove_dir_all(store.root);
    }

    #[test]
    fn a_partial_hand_written_file_keeps_defaults_for_every_absent_row() {
        let store = temporary_store("partial");
        fs::create_dir_all(&store.root).expect("a directory");
        fs::write(
            store.path(),
            "version: 1\nkeyboard:\n  menu:\n    travel: [G, H]\n",
        )
        .expect("a write");

        let (options, warnings) = store.load().expect("a valid partial file");

        assert!(warnings.is_empty());
        assert_eq!(
            options
                .bindings
                .keys(BindingTarget::Menu(AppAction::Travel)),
            [KeyCode::KeyG, KeyCode::KeyH]
        );
        assert_eq!(
            options.bindings.keys(BindingTarget::Menu(AppAction::Back)),
            [KeyCode::Escape],
            "an absent row is the default, not an empty row"
        );
        assert_eq!(options.text_speed, None);
        let _ = fs::remove_dir_all(store.root);
    }

    #[test]
    fn every_unhonorable_row_warns_and_keeps_its_default() {
        let store = temporary_store("warnings");
        fs::create_dir_all(&store.root).expect("a directory");
        fs::write(
            store.path(),
            concat!(
                "version: 1\n",
                "text_speed: instant\n",
                "keyboard:\n",
                "  menu:\n",
                "    travel: [Nonsense]\n",
                "    teleport: [G]\n",
                "    back: []\n",
            ),
        )
        .expect("a write");

        let (options, warnings) = store.load().expect("recoverable problems are not fatal");

        assert_eq!(
            warnings.len(),
            4,
            "one warning per unhonorable row: {warnings:?}"
        );
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("text_speed"))
        );
        assert!(warnings.iter().any(|warning| warning.contains("Nonsense")));
        assert!(warnings.iter().any(|warning| warning.contains("teleport")));
        assert_eq!(
            options,
            PlayerOptions::default(),
            "nothing in that file was honorable, so nothing changed"
        );
        let _ = fs::remove_dir_all(store.root);
    }

    #[test]
    fn a_newer_schema_version_is_refused_rather_than_silently_downgraded() {
        let store = temporary_store("version");
        fs::create_dir_all(&store.root).expect("a directory");
        fs::write(store.path(), "version: 99\n").expect("a write");

        assert_eq!(store.load(), Err(OptionsStoreError::UnsupportedVersion(99)));
        let _ = fs::remove_dir_all(store.root);
    }

    #[test]
    fn an_unparseable_file_is_an_error_the_caller_downgrades_to_defaults() {
        let store = temporary_store("garbage");
        fs::create_dir_all(&store.root).expect("a directory");
        fs::write(store.path(), "version: [this is not a number\n").expect("a write");

        assert!(matches!(store.load(), Err(OptionsStoreError::Parse(_))));
        // The launch path must still come up on defaults rather than refusing to start.
        assert_eq!(load_player_options(&store), PlayerOptions::default());
        let _ = fs::remove_dir_all(store.root);
    }

    #[test]
    fn storing_leaves_no_temporary_file_behind() {
        let store = temporary_store("temp");
        store.store(&PlayerOptions::default()).expect("a write");

        let stray: Vec<_> = fs::read_dir(&store.root)
            .expect("the directory")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name())
            .filter(|name| name != OPTIONS_FILE_NAME)
            .collect();

        assert!(stray.is_empty(), "left behind {stray:?}");
        let _ = fs::remove_dir_all(store.root);
    }
}
