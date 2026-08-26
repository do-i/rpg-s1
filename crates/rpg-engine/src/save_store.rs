//! Linux save-directory selection, native slot discovery, and verified atomic writes.

use std::{
    error::Error,
    ffi::OsString,
    fmt,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use bevy::prelude::{FromWorld, Resource, World};

use crate::{
    game_state::GameState,
    save_data::{NativeSaveEnvelope, NativeSaveError, SaveMetadata},
    scenario_balance::BalanceData,
};

pub(crate) const SAVE_SLOT_COUNT: usize = 101;
pub(crate) const FIRST_PLAYER_SLOT: usize = 1;
pub(crate) const LAST_PLAYER_SLOT: usize = SAVE_SLOT_COUNT - 1;
pub(crate) const SAVE_DIRECTORY_OVERRIDE: &str = "RPG_S1_SAVE_DIR";
static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SaveSlotState {
    Empty,
    Valid,
    Corrupt(String),
    Incompatible(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SaveSlot {
    pub index: usize,
    pub state: SaveSlotState,
    pub metadata: Option<SaveMetadata>,
    pub saved_at_unix_seconds: Option<u64>,
}

impl SaveSlot {
    pub(crate) fn is_valid(&self) -> bool {
        self.state == SaveSlotState::Valid
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.state == SaveSlotState::Empty
    }

    pub(crate) fn label(&self) -> String {
        if self.index == 0 {
            "Autosave".to_owned()
        } else {
            format!("Slot {:02}", self.index)
        }
    }
}

#[derive(Clone, Debug, Resource)]
pub(crate) struct SaveStore {
    root: PathBuf,
}

impl SaveStore {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn slot_path(&self, slot: usize) -> Result<PathBuf, SaveStoreError> {
        require_slot(slot)?;
        Ok(self.root.join(format!("{slot:03}.yaml")))
    }

    pub(crate) fn enumerate(
        &self,
        scenario_id: &str,
        scenario_version: &str,
        balance: &BalanceData,
    ) -> Vec<SaveSlot> {
        (0..SAVE_SLOT_COUNT)
            .map(|index| self.inspect_slot(index, scenario_id, scenario_version, balance))
            .collect()
    }

    fn inspect_slot(
        &self,
        index: usize,
        scenario_id: &str,
        scenario_version: &str,
        balance: &BalanceData,
    ) -> SaveSlot {
        let path = self.root.join(format!("{index:03}.yaml"));
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return SaveSlot {
                    index,
                    state: SaveSlotState::Empty,
                    metadata: None,
                    saved_at_unix_seconds: None,
                };
            }
            Err(error) => {
                return SaveSlot {
                    index,
                    state: SaveSlotState::Corrupt(format!("could not read slot: {error}")),
                    metadata: None,
                    saved_at_unix_seconds: None,
                };
            }
        };
        match NativeSaveEnvelope::decode(&bytes, scenario_id, scenario_version, balance) {
            Ok((envelope, _)) => SaveSlot {
                index,
                state: SaveSlotState::Valid,
                metadata: Some(envelope.metadata),
                saved_at_unix_seconds: Some(envelope.saved_at_unix_seconds),
            },
            Err(
                error @ (NativeSaveError::UnsupportedVersion(_)
                | NativeSaveError::ScenarioMismatch { .. }),
            ) => SaveSlot {
                index,
                state: SaveSlotState::Incompatible(error.to_string()),
                metadata: None,
                saved_at_unix_seconds: None,
            },
            Err(error) => SaveSlot {
                index,
                state: SaveSlotState::Corrupt(error.to_string()),
                metadata: None,
                saved_at_unix_seconds: None,
            },
        }
    }

    pub(crate) fn load(
        &self,
        slot: usize,
        scenario_id: &str,
        scenario_version: &str,
        balance: &BalanceData,
    ) -> Result<(NativeSaveEnvelope, GameState), SaveStoreError> {
        let path = self.slot_path(slot)?;
        let bytes = fs::read(&path).map_err(|error| SaveStoreError::Io {
            operation: "read slot",
            error: error.to_string(),
        })?;
        NativeSaveEnvelope::decode(&bytes, scenario_id, scenario_version, balance)
            .map_err(SaveStoreError::Native)
    }

    pub(crate) fn write(
        &self,
        slot: usize,
        envelope: &NativeSaveEnvelope,
        overwrite: bool,
        balance: &BalanceData,
    ) -> Result<PathBuf, SaveStoreError> {
        self.write_with_hook(slot, envelope, overwrite, balance, |_, _| Ok(()))
    }

    fn write_with_hook(
        &self,
        slot: usize,
        envelope: &NativeSaveEnvelope,
        overwrite: bool,
        balance: &BalanceData,
        before_replace: impl FnOnce(&Path, &Path) -> Result<(), SaveStoreError>,
    ) -> Result<PathBuf, SaveStoreError> {
        require_slot(slot)?;
        fs::create_dir_all(&self.root).map_err(|error| SaveStoreError::Io {
            operation: "create save directory",
            error: error.to_string(),
        })?;
        let destination = self.slot_path(slot)?;
        if !overwrite && destination.exists() {
            return Err(SaveStoreError::DestinationExists(slot));
        }
        let bytes = envelope.encode().map_err(SaveStoreError::Native)?;
        NativeSaveEnvelope::decode(
            &bytes,
            &envelope.scenario_id,
            &envelope.scenario_version,
            balance,
        )
        .map_err(SaveStoreError::Native)?;
        let temp = self.root.join(format!(
            ".{slot:03}.{}.{}.tmp",
            std::process::id(),
            NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let mut cleanup = TemporaryFile::new(temp.clone());
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp)
            .map_err(|error| SaveStoreError::Io {
                operation: "create temporary save",
                error: error.to_string(),
            })?;
        file.write_all(&bytes).map_err(|error| SaveStoreError::Io {
            operation: "write temporary save",
            error: error.to_string(),
        })?;
        file.sync_all().map_err(|error| SaveStoreError::Io {
            operation: "sync temporary save",
            error: error.to_string(),
        })?;
        drop(file);
        let mut verified = Vec::new();
        File::open(&temp)
            .and_then(|mut file| file.read_to_end(&mut verified))
            .map_err(|error| SaveStoreError::Io {
                operation: "verify temporary save",
                error: error.to_string(),
            })?;
        NativeSaveEnvelope::decode(
            &verified,
            &envelope.scenario_id,
            &envelope.scenario_version,
            balance,
        )
        .map_err(SaveStoreError::Native)?;
        before_replace(&temp, &destination)?;
        fs::rename(&temp, &destination).map_err(|error| SaveStoreError::Io {
            operation: "atomically replace save slot",
            error: error.to_string(),
        })?;
        cleanup.disarm();
        File::open(&self.root)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| SaveStoreError::Io {
                operation: "sync save directory",
                error: error.to_string(),
            })?;
        Ok(destination)
    }
}

impl FromWorld for SaveStore {
    fn from_world(_: &mut World) -> Self {
        Self::new(
            resolve_save_directory(|name| std::env::var_os(name))
                .unwrap_or_else(|error| panic!("could not resolve save directory: {error}")),
        )
    }
}

pub(crate) fn resolve_save_directory(
    environment: impl Fn(&str) -> Option<OsString>,
) -> Result<PathBuf, SaveStoreError> {
    if let Some(override_path) = environment(SAVE_DIRECTORY_OVERRIDE) {
        let path = PathBuf::from(override_path);
        if path.as_os_str().is_empty() {
            return Err(SaveStoreError::InvalidDirectory(
                "RPG_S1_SAVE_DIR must not be empty".to_owned(),
            ));
        }
        return Ok(path);
    }
    if let Some(data_home) = environment("XDG_DATA_HOME") {
        let path = PathBuf::from(data_home);
        if path.is_absolute() {
            return Ok(path.join("rpg-s1/saves"));
        }
        return Err(SaveStoreError::InvalidDirectory(
            "XDG_DATA_HOME must be absolute".to_owned(),
        ));
    }
    let home = environment("HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or_else(|| {
            SaveStoreError::InvalidDirectory(
                "HOME must be an absolute path when XDG_DATA_HOME is unset".to_owned(),
            )
        })?;
    Ok(home.join(".local/share/rpg-s1/saves"))
}

pub(crate) fn unix_timestamp_now() -> Result<u64, SaveStoreError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|error| SaveStoreError::Clock(error.to_string()))
}

fn require_slot(slot: usize) -> Result<(), SaveStoreError> {
    if slot < SAVE_SLOT_COUNT {
        Ok(())
    } else {
        Err(SaveStoreError::InvalidSlot(slot))
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SaveStoreError {
    InvalidDirectory(String),
    InvalidSlot(usize),
    DestinationExists(usize),
    Clock(String),
    Native(NativeSaveError),
    Io {
        operation: &'static str,
        error: String,
    },
    #[cfg(test)]
    InjectedInterruption,
}

impl fmt::Display for SaveStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDirectory(reason) => write!(formatter, "invalid save directory: {reason}"),
            Self::InvalidSlot(slot) => write!(formatter, "save slot {slot} is outside 0..=100"),
            Self::DestinationExists(slot) => {
                write!(
                    formatter,
                    "save slot {slot} already exists; confirmation is required"
                )
            }
            Self::Clock(reason) => write!(formatter, "system clock is unavailable: {reason}"),
            Self::Native(error) => error.fmt(formatter),
            Self::Io { operation, error } => write!(formatter, "{operation} failed: {error}"),
            #[cfg(test)]
            Self::InjectedInterruption => formatter.write_str("save write was interrupted"),
        }
    }
}

impl Error for SaveStoreError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save_data::tests::{fixture_balance, fixture_envelope};

    struct TemporaryDirectory(PathBuf);
    impl TemporaryDirectory {
        fn new(label: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "rpg-s1-save-store-{label}-{}-{}",
                std::process::id(),
                NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for TemporaryDirectory {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn linux_directory_precedence_and_validation_are_explicit() {
        let resolve = |pairs: &[(&str, &str)]| {
            resolve_save_directory(|name| {
                pairs
                    .iter()
                    .find(|(key, _)| *key == name)
                    .map(|(_, value)| OsString::from(value))
            })
        };
        assert_eq!(
            resolve(&[
                (SAVE_DIRECTORY_OVERRIDE, "/tmp/custom-saves"),
                ("XDG_DATA_HOME", "/tmp/data"),
                ("HOME", "/tmp/home"),
            ])
            .unwrap(),
            PathBuf::from("/tmp/custom-saves")
        );
        assert_eq!(
            resolve(&[("XDG_DATA_HOME", "/tmp/data")]).unwrap(),
            PathBuf::from("/tmp/data/rpg-s1/saves")
        );
        assert_eq!(
            resolve(&[("HOME", "/tmp/home")]).unwrap(),
            PathBuf::from("/tmp/home/.local/share/rpg-s1/saves")
        );
        assert!(resolve(&[(SAVE_DIRECTORY_OVERRIDE, "")]).is_err());
        assert!(resolve(&[("XDG_DATA_HOME", "relative")]).is_err());
        assert!(resolve(&[]).is_err());
    }

    #[test]
    fn empty_valid_corrupt_and_incompatible_slots_are_isolated() {
        let root = TemporaryDirectory::new("enumerate");
        let store = SaveStore::new(root.0.clone());
        let envelope = fixture_envelope();
        store
            .write(1, &envelope, false, &fixture_balance())
            .unwrap();
        fs::write(store.slot_path(2).unwrap(), b"not: [valid").unwrap();
        let incompatible = String::from_utf8(envelope.encode().unwrap())
            .unwrap()
            .replacen("format_version: 1", "format_version: 99", 1);
        fs::write(store.slot_path(3).unwrap(), incompatible).unwrap();

        let slots = store.enumerate("my_rpg_story", "1.0.0", &fixture_balance());
        assert_eq!(slots.len(), SAVE_SLOT_COUNT);
        assert!(slots[0].is_empty());
        assert!(slots[1].is_valid());
        assert!(matches!(slots[2].state, SaveSlotState::Corrupt(_)));
        assert!(matches!(slots[3].state, SaveSlotState::Incompatible(_)));
        assert!(slots[4].is_empty());
    }

    #[test]
    fn verified_atomic_replace_leaves_one_destination_and_no_temporary_file() {
        let root = TemporaryDirectory::new("replace");
        let store = SaveStore::new(root.0.clone());
        let first = fixture_envelope();
        store.write(1, &first, false, &fixture_balance()).unwrap();
        let mut replacement = first.clone();
        replacement.saved_at_unix_seconds += 1;
        store
            .write(1, &replacement, true, &fixture_balance())
            .unwrap();
        let files = fs::read_dir(&root.0)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<_>>();
        assert_eq!(files, [OsString::from("001.yaml")]);
        let (loaded, _) = store
            .load(1, "my_rpg_story", "1.0.0", &fixture_balance())
            .unwrap();
        assert_eq!(
            loaded.saved_at_unix_seconds,
            replacement.saved_at_unix_seconds
        );
    }

    #[test]
    fn interruption_before_replace_preserves_old_bytes_and_cleans_temp() {
        let root = TemporaryDirectory::new("interrupt");
        let store = SaveStore::new(root.0.clone());
        let first = fixture_envelope();
        let destination = store.write(1, &first, false, &fixture_balance()).unwrap();
        let old_bytes = fs::read(&destination).unwrap();
        let mut replacement = first.clone();
        replacement.saved_at_unix_seconds += 1;
        assert_eq!(
            store.write_with_hook(1, &replacement, true, &fixture_balance(), |_, _| Err(
                SaveStoreError::InjectedInterruption
            ),),
            Err(SaveStoreError::InjectedInterruption)
        );
        assert_eq!(fs::read(&destination).unwrap(), old_bytes);
        assert_eq!(fs::read_dir(&root.0).unwrap().count(), 1);
    }

    #[test]
    fn overwrite_requires_explicit_confirmation() {
        let root = TemporaryDirectory::new("confirm");
        let store = SaveStore::new(root.0.clone());
        let envelope = fixture_envelope();
        let destination = store
            .write(1, &envelope, false, &fixture_balance())
            .unwrap();
        let old_bytes = fs::read(&destination).unwrap();
        assert_eq!(
            store.write(1, &envelope, false, &fixture_balance()),
            Err(SaveStoreError::DestinationExists(1))
        );
        assert_eq!(fs::read(destination).unwrap(), old_bytes);
    }
}
