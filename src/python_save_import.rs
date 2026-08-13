//! One-way importer for the pinned Python YAML save schema from ADR 0001.

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt, fs,
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::{
    runtime_member::{EquipmentSlot, RuntimeMember, RuntimeMemberParts},
    runtime_repository::{RuntimeRepository, RuntimeRepositoryItemParts},
    save_data::{
        ImportProvenance, NATIVE_SAVE_FORMAT_VERSION, NativeSaveEnvelope, SaveEquipment, SaveMap,
        SaveMember, SaveMetadata, SaveOpenedBox, SavePayload, SaveRepository, SaveRepositoryItem,
        SaveStats,
    },
    save_store::{SaveStore, SaveStoreError},
    scenario_balance::BalanceData,
    scenario_class::{ClassDefinition, ClassEquipmentSlots},
    scenario_item::{ItemCatalogFile, ItemDefinition},
    scenario_manifest::Manifest,
    scenario_map::MapMetadata,
    scenario_party::{PartyCatalog, PartyRow},
    scenario_spatial::{CardinalDirection, Position},
    scenario_yaml,
};

pub(crate) const PYTHON_SAVE_ADAPTER_COMMIT: &str = "08970359d6cb03586948625d29b0d3351dbbf785";
const PYTHON_SAVE_SCENARIO_ID: &str = "my_rpg_story";
const PYTHON_SAVE_SCENARIO_VERSION: &str = "1.0.0";
const MAX_PYTHON_SAVE_BYTES: usize = 1_048_576;
static NEXT_IMPORT_TEMPORARY_FILE: AtomicU64 = AtomicU64::new(0);
const ITEM_FILES: [&str; 12] = [
    "accessories.yaml",
    "body.yaml",
    "consumables_battle_throw.yaml",
    "consumables_field.yaml",
    "consumables_recovery.yaml",
    "consumables_status_cure.yaml",
    "helmets.yaml",
    "key_items.yaml",
    "magic_cores.yaml",
    "materials.yaml",
    "shields.yaml",
    "weapons.yaml",
];

#[derive(Debug)]
pub(crate) struct PythonImportCatalog {
    pub manifest: Manifest,
    pub balance: BalanceData,
    party: PartyCatalog,
    classes: BTreeMap<String, ClassDefinition>,
    items: BTreeMap<String, ItemDefinition>,
    maps: BTreeMap<String, MapMetadata>,
}

impl PythonImportCatalog {
    pub(crate) fn load(package: &Path) -> Result<Self, PythonImportError> {
        let manifest: Manifest = load_yaml(&package.join("manifest.yaml"))?;
        if manifest.id != PYTHON_SAVE_SCENARIO_ID
            || manifest.version != PYTHON_SAVE_SCENARIO_VERSION
        {
            return Err(PythonImportError::Content(format!(
                "adapter {PYTHON_SAVE_ADAPTER_COMMIT} requires scenario {PYTHON_SAVE_SCENARIO_ID} {PYTHON_SAVE_SCENARIO_VERSION}, but the selected package provides {} {}",
                manifest.id, manifest.version
            )));
        }
        let balance: BalanceData = load_yaml(&package.join(manifest.refs.balance.as_str()))?;
        let party: PartyCatalog = load_yaml(&package.join(manifest.refs.party.as_str()))?;
        let class_root = package.join(manifest.refs.classes.as_str());
        let mut classes = BTreeMap::new();
        for entry in sorted_yaml_files(&class_root)? {
            let class: ClassDefinition = load_yaml(&entry)?;
            if classes.insert(class.class_id.clone(), class).is_some() {
                return Err(PythonImportError::Content(
                    "class catalog contains a duplicate id".to_owned(),
                ));
            }
        }
        let item_root = package.join(manifest.refs.items.as_str());
        let mut items = BTreeMap::new();
        for filename in ITEM_FILES {
            let catalog: ItemCatalogFile = load_yaml(&item_root.join(filename))?;
            for item in catalog.0 {
                if items.insert(item.id().to_owned(), item).is_some() {
                    return Err(PythonImportError::Content(
                        "item catalog contains a duplicate id".to_owned(),
                    ));
                }
            }
        }
        let map_root = package.join(manifest.refs.maps.as_str());
        let mut maps = BTreeMap::new();
        for entry in sorted_yaml_files(&map_root)? {
            let stem = entry
                .file_stem()
                .and_then(|value| value.to_str())
                .ok_or_else(|| {
                    PythonImportError::Content("map filename is not UTF-8".to_owned())
                })?;
            let map: MapMetadata = load_yaml(&entry)?;
            let id = map.effective_id(stem).to_owned();
            if maps.insert(id.clone(), map).is_some() {
                return Err(PythonImportError::Content(format!(
                    "map catalog contains duplicate id `{id}`"
                )));
            }
        }
        Ok(Self {
            manifest,
            balance,
            party,
            classes,
            items,
            maps,
        })
    }
}

pub(crate) fn convert_python_save(
    source_bytes: &[u8],
    allow_unchecked: bool,
    catalog: &PythonImportCatalog,
    saved_at_unix_seconds: u64,
) -> Result<NativeSaveEnvelope, PythonImportError> {
    if source_bytes.len() > MAX_PYTHON_SAVE_BYTES {
        return Err(PythonImportError::Input(format!(
            "Python save exceeds the {MAX_PYTHON_SAVE_BYTES}-byte limit"
        )));
    }
    let source = std::str::from_utf8(source_bytes)
        .map_err(|error| PythonImportError::Input(format!("save is not UTF-8: {error}")))?;
    let checksum_verified = verify_python_checksum(source, allow_unchecked)?;
    let deserializer = serde_yaml_ng::Deserializer::from_str(source);
    let legacy: PythonSave = serde_path_to_error::deserialize(deserializer)
        .map_err(|error| PythonImportError::Input(error.to_string()))?;
    let original_timestamp = nonempty(legacy.meta.timestamp.clone());
    let original_location = nonempty(legacy.meta.location_display.clone());
    let location = original_location
        .clone()
        .unwrap_or_else(|| legacy.map.current.clone());
    let payload = legacy.into_native_payload(catalog)?;
    let game = payload
        .clone()
        .into_game_state(&catalog.balance)
        .map_err(|error| PythonImportError::Content(error.to_string()))?;
    let protagonist = game.party().protagonist().expect("validated import party");
    let envelope = NativeSaveEnvelope {
        format_version: NATIVE_SAVE_FORMAT_VERSION,
        scenario_id: catalog.manifest.id.clone(),
        scenario_version: catalog.manifest.version.clone(),
        saved_at_unix_seconds,
        metadata: SaveMetadata {
            protagonist_name: protagonist.name().to_owned(),
            protagonist_level: protagonist.level(),
            location,
            playtime_seconds: game.playtime().to_seconds(),
        },
        payload,
        import_provenance: Some(ImportProvenance {
            source_kind: "python-yaml".to_owned(),
            adapter_commit: PYTHON_SAVE_ADAPTER_COMMIT.to_owned(),
            source_sha256: sha256_hex(source_bytes),
            original_timestamp,
            original_location,
            checksum_verified,
        }),
    };
    let encoded = envelope
        .encode()
        .map_err(|error| PythonImportError::Native(error.to_string()))?;
    let (decoded, restored) = NativeSaveEnvelope::decode(
        &encoded,
        &catalog.manifest.id,
        &catalog.manifest.version,
        &catalog.balance,
    )
    .map_err(|error| PythonImportError::Native(error.to_string()))?;
    let restored_payload = NativeSaveEnvelope::from_game_state(
        &restored,
        &catalog.manifest.id,
        &catalog.manifest.version,
        saved_at_unix_seconds,
        &decoded.metadata.location,
    )
    .map_err(|error| PythonImportError::Native(error.to_string()))?
    .payload;
    if restored_payload != envelope.payload {
        return Err(PythonImportError::Native(
            "native verification changed imported state".to_owned(),
        ));
    }
    Ok(envelope)
}

pub(crate) fn install_python_import(
    store: &SaveStore,
    slot: usize,
    envelope: &NativeSaveEnvelope,
    replace: bool,
    balance: &BalanceData,
) -> Result<ImportInstallResult, PythonImportError> {
    install_python_import_with_hook(store, slot, envelope, replace, balance, || Ok(()))
}

fn install_python_import_with_hook(
    store: &SaveStore,
    slot: usize,
    envelope: &NativeSaveEnvelope,
    replace: bool,
    balance: &BalanceData,
    before_native_write: impl FnOnce() -> Result<(), PythonImportError>,
) -> Result<ImportInstallResult, PythonImportError> {
    let destination = store
        .slot_path(slot)
        .map_err(|error| PythonImportError::Install(error.to_string()))?;
    let backup = if destination.exists() {
        if !replace {
            return Err(PythonImportError::Install(format!(
                "{} already exists; pass --replace to preserve it and continue",
                destination
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("destination slot")
            )));
        }
        Some(create_verified_backup(store.root(), slot, &destination)?)
    } else {
        None
    };
    before_native_write()?;
    let destination = store
        .write(slot, envelope, replace, balance)
        .map_err(|error| PythonImportError::Install(error.to_string()))?;
    Ok(ImportInstallResult {
        destination,
        backup,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ImportInstallResult {
    pub destination: PathBuf,
    pub backup: Option<PathBuf>,
}

fn create_verified_backup(
    save_root: &Path,
    slot: usize,
    destination: &Path,
) -> Result<PathBuf, PythonImportError> {
    let old_bytes = fs::read(destination)
        .map_err(|error| PythonImportError::Install(format!("read old slot failed: {error}")))?;
    let digest = sha256_hex(&old_bytes);
    let backup_root = save_root.join("import-backups");
    fs::create_dir_all(&backup_root).map_err(|error| {
        PythonImportError::Install(format!("create import backup directory failed: {error}"))
    })?;
    let backup = backup_root.join(format!("{slot:03}-{digest}.yaml"));
    if backup.exists() {
        if fs::read(&backup).ok().as_deref() != Some(old_bytes.as_slice()) {
            return Err(PythonImportError::Install(
                "existing import backup conflicts with the old slot bytes".to_owned(),
            ));
        }
        return Ok(backup);
    }
    let nonce = NEXT_IMPORT_TEMPORARY_FILE.fetch_add(1, Ordering::Relaxed);
    let temporary = backup_root.join(format!(
        ".{slot:03}-{digest}.{}-{nonce}.tmp",
        std::process::id()
    ));
    let mut cleanup = ImportTemporaryFile::new(temporary.clone());
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(|error| PythonImportError::Install(format!("create backup failed: {error}")))?;
    file.write_all(&old_bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| PythonImportError::Install(format!("write backup failed: {error}")))?;
    if fs::read(&temporary).ok().as_deref() != Some(old_bytes.as_slice()) {
        return Err(PythonImportError::Install(
            "backup verification failed".to_owned(),
        ));
    }
    fs::rename(&temporary, &backup)
        .map_err(|error| PythonImportError::Install(format!("install backup failed: {error}")))?;
    cleanup.disarm();
    fs::File::open(&backup_root)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| PythonImportError::Install(format!("sync backup failed: {error}")))?;
    Ok(backup)
}

struct ImportTemporaryFile {
    path: PathBuf,
    armed: bool,
}

impl ImportTemporaryFile {
    fn new(path: PathBuf) -> Self {
        Self { path, armed: true }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for ImportTemporaryFile {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn verify_python_checksum(source: &str, allow_unchecked: bool) -> Result<bool, PythonImportError> {
    // The pinned Python loader verifies a parsed mapping, not the source bytes:
    // safe_load -> remove `checksum` -> yaml.dump(sort_keys=False) -> CRC32.
    // Re-serializing the generic mapping preserves that behavior for harmless
    // YAML differences such as comments and whitespace.
    let mut document: serde_yaml_ng::Value = serde_yaml_ng::from_str(source)
        .map_err(|error| PythonImportError::Input(error.to_string()))?;
    let mapping = document.as_mapping_mut().ok_or_else(|| {
        PythonImportError::Input("Python save must be a top-level mapping".to_owned())
    })?;
    let Some(stored) = mapping.remove(serde_yaml_ng::Value::String("checksum".to_owned())) else {
        if allow_unchecked {
            return Ok(false);
        }
        return Err(PythonImportError::Checksum(
            "save has no checksum; pass --allow-unchecked to import explicitly".to_owned(),
        ));
    };
    let stored = stored
        .as_str()
        .ok_or_else(|| PythonImportError::Checksum("checksum must be a YAML string".to_owned()))?;
    if stored.len() != 8 || !stored.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(PythonImportError::Checksum(
            "checksum must contain exactly eight hexadecimal digits".to_owned(),
        ));
    }
    let canonical = serde_yaml_ng::to_string(&document)
        .map_err(|error| PythonImportError::Checksum(error.to_string()))?;
    let computed = format!("{:08X}", crc32fast::hash(canonical.as_bytes()));
    if computed != stored.to_ascii_uppercase() {
        return Err(PythonImportError::Checksum(format!(
            "checksum mismatch: stored {stored}, computed {computed}"
        )));
    }
    Ok(true)
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PythonSave {
    meta: PythonMeta,
    party: Vec<PythonMember>,
    #[serde(default)]
    controlled_member_id: String,
    #[serde(default)]
    party_repository: PythonRepository,
    flags: Vec<String>,
    map: PythonMap,
    #[serde(default)]
    opened_boxes: Vec<String>,
    #[allow(dead_code)]
    checksum: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PythonMeta {
    #[serde(default)]
    timestamp: String,
    playtime_seconds: u64,
    #[serde(default)]
    location_display: String,
    #[allow(dead_code)]
    #[serde(default)]
    is_autosave: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PythonMember {
    id: String,
    name: String,
    protagonist: bool,
    #[serde(rename = "class")]
    class_id: String,
    level: u32,
    exp: u32,
    #[serde(default)]
    exp_next: Option<u32>,
    hp: u32,
    hp_max: u32,
    mp: u32,
    mp_max: u32,
    #[serde(rename = "str")]
    strength: u32,
    dex: u32,
    con: u32,
    #[serde(rename = "int")]
    intelligence: u32,
    equipped: BTreeMap<String, String>,
    #[serde(default)]
    row: Option<PartyRow>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct PythonRepository {
    #[serde(default)]
    gp: u32,
    #[serde(default)]
    items: Vec<PythonRepositoryItem>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PythonRepositoryItem {
    id: String,
    #[serde(default = "one")]
    qty: u32,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    locked: bool,
    #[serde(default)]
    is_loot: bool,
    #[serde(default)]
    loot_batch: u64,
}

const fn one() -> u32 {
    1
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PythonMap {
    current: String,
    position: Position,
    visited: Vec<String>,
}

impl PythonSave {
    fn into_native_payload(
        self,
        catalog: &PythonImportCatalog,
    ) -> Result<SavePayload, PythonImportError> {
        if self.party.is_empty() {
            return Err(PythonImportError::Content(
                "party must not be empty".to_owned(),
            ));
        }
        let mut flags = BTreeSet::new();
        for flag in self.flags {
            if flag.is_empty() {
                return Err(PythonImportError::Content(
                    "flags must not be empty".to_owned(),
                ));
            }
            if !flags.insert(flag.clone()) {
                return Err(PythonImportError::Content(format!(
                    "flag `{flag}` appears more than once"
                )));
            }
        }
        let party = self
            .party
            .into_iter()
            .map(|member| member.into_native(catalog))
            .collect::<Result<Vec<_>, _>>()?;
        let controlled_member_id = if self.controlled_member_id.is_empty() {
            party
                .iter()
                .find(|member| member.protagonist)
                .or_else(|| party.first())
                .expect("nonempty party")
                .id
                .clone()
        } else {
            self.controlled_member_id
        };
        if !party.iter().any(|member| member.id == controlled_member_id) {
            return Err(PythonImportError::Content(format!(
                "controlled member `{controlled_member_id}` is not in the party"
            )));
        }
        if !catalog.maps.contains_key(&self.map.current) {
            return Err(PythonImportError::Content(format!(
                "map `{}` is not available in the selected Rust scenario",
                self.map.current
            )));
        }
        let mut visited_seen = BTreeSet::new();
        for id in &self.map.visited {
            if !catalog.maps.contains_key(id) {
                return Err(PythonImportError::Content(format!(
                    "visited map `{id}` is not available in the selected Rust scenario"
                )));
            }
            if !visited_seen.insert(id.clone()) {
                return Err(PythonImportError::Content(format!(
                    "visited map `{id}` appears more than once"
                )));
            }
        }
        let repository = self.party_repository.into_native(catalog)?;
        let mut opened_seen = BTreeSet::new();
        let mut opened_boxes = Vec::new();
        for entry in self.opened_boxes {
            let (map_id, box_id) = entry.split_once(':').ok_or_else(|| {
                PythonImportError::Content(format!("opened-box entry `{entry}` is malformed"))
            })?;
            let map = catalog.maps.get(map_id).ok_or_else(|| {
                PythonImportError::Content(format!("opened-box map `{map_id}` is unavailable"))
            })?;
            if !map.item_boxes.iter().any(|item_box| item_box.id == box_id) {
                return Err(PythonImportError::Content(format!(
                    "opened box `{map_id}:{box_id}` is not in current map metadata"
                )));
            }
            if !opened_seen.insert((map_id.to_owned(), box_id.to_owned())) {
                return Err(PythonImportError::Content(format!(
                    "opened box `{entry}` appears more than once"
                )));
            }
            opened_boxes.push(SaveOpenedBox {
                map_id: map_id.to_owned(),
                box_id: box_id.to_owned(),
            });
        }
        opened_boxes.sort_by(|left, right| {
            (&left.map_id, &left.box_id).cmp(&(&right.map_id, &right.box_id))
        });
        let mut visited = self.map.visited;
        visited.sort();
        let normalized = serde_yaml_ng::to_string(&(
            &party,
            &repository,
            &flags,
            &self.map.current,
            self.map.position,
            &visited,
            &opened_boxes,
            &controlled_member_id,
            self.meta.playtime_seconds,
        ))
        .map_err(|error| PythonImportError::Native(error.to_string()))?;
        let mut hasher = Sha256::new();
        hasher.update(b"rpg-s1/python-save-rng/v1\0");
        hasher.update(PYTHON_SAVE_ADAPTER_COMMIT.as_bytes());
        hasher.update(b"\0");
        hasher.update(normalized.as_bytes());
        let digest = hasher.finalize();
        let rng_state = u64::from_be_bytes(digest[..8].try_into().expect("SHA-256 prefix"));
        Ok(SavePayload {
            flags: flags.into_iter().collect(),
            party,
            repository,
            map: SaveMap {
                current: self.map.current,
                position: self.map.position,
                facing: CardinalDirection::Down,
                visited,
            },
            opened_boxes,
            controlled_member_id,
            rng_state,
            playtime_seconds: self.meta.playtime_seconds,
        })
    }
}

impl PythonMember {
    fn into_native(self, catalog: &PythonImportCatalog) -> Result<SaveMember, PythonImportError> {
        let source = catalog
            .party
            .party
            .iter()
            .find(|candidate| candidate.data().id == self.id)
            .ok_or_else(|| {
                PythonImportError::Content(format!("party member `{}` is unavailable", self.id))
            })?;
        if source.data().class_id != self.class_id || source.is_protagonist() != self.protagonist {
            return Err(PythonImportError::Content(format!(
                "party member `{}` class/protagonist identity does not match scenario data",
                self.id
            )));
        }
        let class = catalog.classes.get(&self.class_id).ok_or_else(|| {
            PythonImportError::Content(format!("class `{}` is unavailable", self.class_id))
        })?;
        let row = self.row.unwrap_or(class.default_row);
        let equipment = SaveEquipment {
            weapon: equipment_value(&self.equipped, "weapon")?,
            shield: equipment_value(&self.equipped, "shield")?,
            helmet: equipment_value(&self.equipped, "helmet")?,
            body: equipment_value(&self.equipped, "body")?,
            accessory: equipment_value(&self.equipped, "accessory")?,
        };
        for key in self.equipped.keys() {
            if !["weapon", "shield", "helmet", "body", "accessory"].contains(&key.as_str()) {
                return Err(PythonImportError::Content(format!(
                    "member `{}` has unknown equipment slot `{key}`",
                    self.id
                )));
            }
        }
        for (slot, id) in [
            (EquipmentSlot::Weapon, equipment.weapon.as_deref()),
            (EquipmentSlot::Shield, equipment.shield.as_deref()),
            (EquipmentSlot::Helmet, equipment.helmet.as_deref()),
            (EquipmentSlot::Body, equipment.body.as_deref()),
            (EquipmentSlot::Accessory, equipment.accessory.as_deref()),
        ] {
            if let Some(id) = id {
                validate_equipment(id, slot, class, catalog)?;
            }
        }
        let experience_next = self.exp_next.unwrap_or_else(|| {
            if self.level >= catalog.balance.progression.level_cap.get() {
                0
            } else {
                (f64::from(class.exp_base.get())
                    * f64::from(self.level.saturating_add(1)).powf(class.exp_factor.get()))
                    as u32
            }
        });
        let save = SaveMember {
            id: self.id,
            name: self.name,
            protagonist: self.protagonist,
            class_id: self.class_id,
            level: self.level,
            experience: self.exp,
            experience_next,
            health: self.hp,
            max_health: self.hp_max,
            mana: self.mp,
            max_mana: self.mp_max,
            stats: SaveStats {
                strength: self.strength,
                dexterity: self.dex,
                constitution: self.con,
                intelligence: self.intelligence,
            },
            row,
            equipment,
            status_effects: Vec::new(),
        };
        RuntimeMember::try_from_saved(
            RuntimeMemberParts {
                id: save.id.clone(),
                name: save.name.clone(),
                protagonist: save.protagonist,
                class_id: save.class_id.clone(),
                level: save.level,
                experience: save.experience,
                experience_next: save.experience_next,
                health: save.health,
                max_health: save.max_health,
                mana: save.mana,
                max_mana: save.max_mana,
                stats: [
                    save.stats.strength,
                    save.stats.dexterity,
                    save.stats.constitution,
                    save.stats.intelligence,
                ],
                row: save.row,
                equipment: [
                    save.equipment.weapon.clone(),
                    save.equipment.shield.clone(),
                    save.equipment.helmet.clone(),
                    save.equipment.body.clone(),
                    save.equipment.accessory.clone(),
                ],
                status_effects: Vec::new(),
            },
            &catalog.balance.progression,
        )
        .map_err(|error| PythonImportError::Content(error.to_string()))?;
        Ok(save)
    }
}

impl PythonRepository {
    fn into_native(
        self,
        catalog: &PythonImportCatalog,
    ) -> Result<SaveRepository, PythonImportError> {
        let mut seen = BTreeSet::new();
        let mut items = Vec::new();
        for item in self.items {
            if !catalog.items.contains_key(&item.id) {
                return Err(PythonImportError::Content(format!(
                    "repository item `{}` is unavailable",
                    item.id
                )));
            }
            if !seen.insert(item.id.clone()) {
                return Err(PythonImportError::Content(format!(
                    "repository item `{}` appears more than once",
                    item.id
                )));
            }
            let mut tags = BTreeSet::new();
            for tag in item.tags {
                if tag.is_empty() || !tags.insert(tag.clone()) {
                    return Err(PythonImportError::Content(format!(
                        "repository item `{}` has empty or duplicate tags",
                        item.id
                    )));
                }
            }
            if item.id.starts_with("mc_") {
                tags.insert("magic_core".to_owned());
            }
            if tags.len() > catalog.balance.economy.max_tags_per_item as usize {
                return Err(PythonImportError::Content(format!(
                    "repository item `{}` exceeds the tag cap",
                    item.id
                )));
            }
            items.push(SaveRepositoryItem {
                id: item.id,
                quantity: item.qty,
                tags: tags.into_iter().collect(),
                locked: item.locked,
                is_loot: item.is_loot,
                loot_batch: item.loot_batch,
            });
        }
        items.sort_by(|left, right| left.id.cmp(&right.id));
        RuntimeRepository::try_from_saved(
            &catalog.balance.economy,
            self.gp,
            items
                .iter()
                .cloned()
                .map(|item| RuntimeRepositoryItemParts {
                    id: item.id,
                    quantity: item.quantity,
                    tags: item.tags,
                    locked: item.locked,
                    is_loot: item.is_loot,
                    loot_batch: item.loot_batch,
                }),
        )
        .map_err(|error| PythonImportError::Content(error.to_string()))?;
        Ok(SaveRepository { gp: self.gp, items })
    }
}

fn equipment_value(
    equipment: &BTreeMap<String, String>,
    slot: &str,
) -> Result<Option<String>, PythonImportError> {
    match equipment.get(slot) {
        None => Ok(None),
        Some(id) if id.is_empty() => Ok(None),
        Some(id) => Ok(Some(id.clone())),
    }
}

fn validate_equipment(
    id: &str,
    expected_slot: EquipmentSlot,
    class: &ClassDefinition,
    catalog: &PythonImportCatalog,
) -> Result<(), PythonImportError> {
    let item = catalog.items.get(id).ok_or_else(|| {
        PythonImportError::Content(format!("equipped item `{id}` is unavailable"))
    })?;
    let (slot, category, accessory_classes) = match item {
        ItemDefinition::Weapon(value) => {
            (EquipmentSlot::Weapon, value.slot_category.as_str(), None)
        }
        ItemDefinition::Shield(value) => {
            (EquipmentSlot::Shield, value.slot_category.as_str(), None)
        }
        ItemDefinition::Helmet(value) => {
            (EquipmentSlot::Helmet, value.slot_category.as_str(), None)
        }
        ItemDefinition::Body(value) => (EquipmentSlot::Body, value.slot_category.as_str(), None),
        ItemDefinition::Accessory(value) => (
            EquipmentSlot::Accessory,
            "all",
            Some(value.equippable.as_slice()),
        ),
        _ => {
            return Err(PythonImportError::Content(format!(
                "item `{id}` is not equipment"
            )));
        }
    };
    if slot != expected_slot {
        return Err(PythonImportError::Content(format!(
            "item `{id}` is stored in the wrong equipment slot"
        )));
    }
    if let Some(classes) = accessory_classes
        && !classes.is_empty()
        && !classes
            .iter()
            .any(|allowed| allowed == "all" || allowed == &class.class_id)
    {
        return Err(PythonImportError::Content(format!(
            "class `{}` cannot equip `{id}`",
            class.class_id
        )));
    }
    let allowed = slot_values(&class.equipment_slots, slot);
    if !allowed
        .iter()
        .any(|value| value == "all" || value == category)
    {
        return Err(PythonImportError::Content(format!(
            "class `{}` cannot equip `{id}` in `{}`",
            class.class_id,
            slot.as_str()
        )));
    }
    Ok(())
}

fn slot_values(slots: &ClassEquipmentSlots, slot: EquipmentSlot) -> &[String] {
    match slot {
        EquipmentSlot::Weapon => &slots.weapon,
        EquipmentSlot::Shield => &slots.shield,
        EquipmentSlot::Helmet => &slots.helmet,
        EquipmentSlot::Body => &slots.body,
        EquipmentSlot::Accessory => &slots.accessory,
    }
}

fn sorted_yaml_files(root: &Path) -> Result<Vec<PathBuf>, PythonImportError> {
    let mut files = fs::read_dir(root)
        .map_err(|error| {
            PythonImportError::Content(format!("read {} failed: {error}", root.display()))
        })?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| PythonImportError::Content(error.to_string()))?;
    files.retain(|path| {
        matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("yaml" | "yml")
        )
    });
    files.sort();
    Ok(files)
}

fn load_yaml<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, PythonImportError> {
    let bytes = fs::read(path).map_err(|error| {
        PythonImportError::Content(format!("read {} failed: {error}", path.display()))
    })?;
    let document = std::str::from_utf8(&bytes).map_err(|error| {
        PythonImportError::Content(format!("{} is not UTF-8: {error}", path.display()))
    })?;
    scenario_yaml::from_str(document)
        .map_err(|error| PythonImportError::Content(format!("{}: {error}", path.display())))
}

fn nonempty(value: String) -> Option<String> {
    (!value.is_empty()).then_some(value)
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum PythonImportError {
    Input(String),
    Checksum(String),
    Content(String),
    Native(String),
    Install(String),
}

impl fmt::Display for PythonImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Input(reason) => write!(formatter, "Python save input is invalid: {reason}"),
            Self::Checksum(reason) => write!(formatter, "Python save checksum failed: {reason}"),
            Self::Content(reason) => write!(formatter, "Python save is incompatible: {reason}"),
            Self::Native(reason) => write!(formatter, "native conversion failed: {reason}"),
            Self::Install(reason) => write!(formatter, "import installation failed: {reason}"),
        }
    }
}

impl Error for PythonImportError {}

impl From<SaveStoreError> for PythonImportError {
    fn from(error: SaveStoreError) -> Self {
        Self::Install(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save_store::SaveSlotState;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct TemporaryDirectory(PathBuf);
    impl TemporaryDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "rpg-s1-python-import-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
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

    fn catalog() -> PythonImportCatalog {
        PythonImportCatalog::load(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/scenarios/rusted_kingdoms"),
        )
        .unwrap()
    }

    fn fixture() -> &'static [u8] {
        include_bytes!("../tests/fixtures/python-save-0897035/007.yaml")
    }

    #[test]
    fn verified_real_python_fixture_maps_every_promised_state_group() {
        let catalog = catalog();
        let envelope = convert_python_save(fixture(), false, &catalog, 1_700_000_100).unwrap();
        assert_eq!(envelope.scenario_id, "my_rpg_story");
        assert_eq!(envelope.scenario_version, "1.0.0");
        assert_eq!(envelope.metadata.protagonist_name, "Imported Aric");
        assert_eq!(envelope.metadata.location, "zone_01_starting_forest");
        assert_eq!(envelope.payload.party.len(), 2);
        assert_eq!(envelope.payload.party[0].experience_next, 400);
        assert_eq!(envelope.payload.party[1].row, PartyRow::Back);
        assert!(
            envelope
                .payload
                .party
                .iter()
                .all(|member| member.status_effects.is_empty())
        );
        assert_eq!(envelope.payload.controlled_member_id, "elise");
        assert_eq!(envelope.payload.repository.gp, 1_234);
        let potion = envelope
            .payload
            .repository
            .items
            .iter()
            .find(|item| item.id == "potion")
            .unwrap();
        assert_eq!(potion.quantity, 7);
        assert_eq!(potion.tags, ["favorite", "recovery"]);
        assert!(potion.locked && potion.is_loot);
        assert_eq!(potion.loot_batch, 1);
        assert_eq!(envelope.payload.map.facing, CardinalDirection::Down);
        assert_eq!(envelope.payload.map.visited, ["town_01_ardel"]);
        assert_eq!(envelope.payload.opened_boxes[0].box_id, "forest_chest_01");
        assert_eq!(envelope.payload.playtime_seconds, 9_876);
        let provenance = envelope.import_provenance.as_ref().unwrap();
        assert!(provenance.checksum_verified);
        assert_eq!(provenance.adapter_commit, PYTHON_SAVE_ADAPTER_COMMIT);
        assert_eq!(provenance.source_sha256, sha256_hex(fixture()));
        assert_eq!(
            convert_python_save(fixture(), false, &catalog, 1_700_000_100)
                .unwrap()
                .payload
                .rng_state,
            envelope.payload.rng_state
        );
    }

    #[test]
    fn checksum_mismatch_and_missing_checksum_require_distinct_explicit_paths() {
        let catalog = catalog();
        let tampered = String::from_utf8(fixture().to_vec())
            .unwrap()
            .replacen("qty: 7", "qty: 8", 1);
        assert!(matches!(
            convert_python_save(tampered.as_bytes(), false, &catalog, 1),
            Err(PythonImportError::Checksum(_))
        ));
        let unchecked = String::from_utf8(fixture().to_vec())
            .unwrap()
            .lines()
            .filter(|line| !line.starts_with("checksum:"))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        assert!(matches!(
            convert_python_save(unchecked.as_bytes(), false, &catalog, 1),
            Err(PythonImportError::Checksum(_))
        ));
        let imported = convert_python_save(unchecked.as_bytes(), true, &catalog, 1).unwrap();
        assert!(!imported.import_provenance.unwrap().checksum_verified);
    }

    #[test]
    fn checksum_matches_python_loader_semantics_after_yaml_reformatting() {
        let catalog = catalog();
        let reformatted = String::from_utf8(fixture().to_vec()).unwrap().replacen(
            "meta:\n",
            "meta: # harmless YAML comment\n\n",
            1,
        );

        let envelope =
            convert_python_save(reformatted.as_bytes(), false, &catalog, 1_700_000_100).unwrap();
        let original = convert_python_save(fixture(), false, &catalog, 1_700_000_100).unwrap();

        assert!(envelope.import_provenance.unwrap().checksum_verified);
        assert_eq!(envelope.payload.rng_state, original.payload.rng_state);
    }

    #[test]
    fn pinned_older_omissions_apply_loader_defaults_and_magic_core_tag() {
        let legacy = br#"meta:
  playtime_seconds: 42
party:
- id: aric
  name: Older Aric
  protagonist: true
  class: hero
  level: 1
  exp: 0
  hp: 22
  hp_max: 22
  mp: 12
  mp_max: 12
  str: 28
  dex: 17
  con: 28
  int: 5
  equipped: {}
party_repository:
  items:
  - id: mc_s
flags: []
map:
  current: town_01_ardel
  position: [11, 8]
  visited: []
"#;

        let envelope = convert_python_save(legacy, true, &catalog(), 99).unwrap();
        let member = &envelope.payload.party[0];
        let item = &envelope.payload.repository.items[0];
        assert_eq!(member.row, PartyRow::Front);
        assert_eq!(member.experience_next, 400);
        assert_eq!(envelope.payload.controlled_member_id, "aric");
        assert_eq!(envelope.payload.repository.gp, 0);
        assert_eq!(item.quantity, 1);
        assert_eq!(item.tags, ["magic_core"]);
        assert!(!item.locked && !item.is_loot && item.loot_batch == 0);
        assert!(envelope.payload.opened_boxes.is_empty());
        assert_eq!(envelope.metadata.location, "town_01_ardel");
        let provenance = envelope.import_provenance.unwrap();
        assert!(!provenance.checksum_verified);
        assert_eq!(provenance.original_timestamp, None);
        assert_eq!(provenance.original_location, None);
    }

    #[test]
    fn malformed_duplicates_ranges_and_unavailable_references_are_rejected() {
        let catalog = catalog();
        for changed in [
            String::from_utf8(fixture().to_vec())
                .unwrap()
                .replacen("level: 1", "level: nope", 1)
                .replace("checksum: 897BA056\n", ""),
            String::from_utf8(fixture().to_vec())
                .unwrap()
                .replacen("level: 1", "level: 4294967295", 1)
                .replace("checksum: 897BA056\n", ""),
            String::from_utf8(fixture().to_vec())
                .unwrap()
                .replace("qty: 7", "qty: 1000")
                .replace("checksum: 897BA056\n", ""),
            String::from_utf8(fixture().to_vec())
                .unwrap()
                .replace("id: mc_s", "id: unavailable_item")
                .replace("checksum: 897BA056\n", ""),
            String::from_utf8(fixture().to_vec())
                .unwrap()
                .replace("- id: elise", "- id: aric")
                .replace("checksum: 897BA056\n", ""),
            String::from_utf8(fixture().to_vec())
                .unwrap()
                .replacen(
                    "- aric_teleport_unlocked\n",
                    "- aric_teleport_unlocked\n- aric_teleport_unlocked\n",
                    1,
                )
                .replace("checksum: 897BA056\n", ""),
        ] {
            assert!(convert_python_save(changed.as_bytes(), true, &catalog, 1).is_err());
        }
        assert!(convert_python_save(b"meta: [truncated", true, &catalog, 1).is_err());
    }

    #[test]
    fn destination_conflict_refuses_by_default_and_replace_preserves_verified_backup() {
        let catalog = catalog();
        let envelope = convert_python_save(fixture(), false, &catalog, 1_700_000_100).unwrap();
        let root = TemporaryDirectory::new();
        let store = SaveStore::new(root.0.clone());
        let original = crate::save_data::tests::fixture_envelope();
        let destination = store.write(7, &original, false, &catalog.balance).unwrap();
        let old_bytes = fs::read(&destination).unwrap();

        assert!(matches!(
            install_python_import(&store, 7, &envelope, false, &catalog.balance),
            Err(PythonImportError::Install(_))
        ));
        assert_eq!(fs::read(&destination).unwrap(), old_bytes);

        let result = install_python_import(&store, 7, &envelope, true, &catalog.balance).unwrap();
        let backup = result.backup.unwrap();
        assert_eq!(fs::read(backup).unwrap(), old_bytes);
        let slots = store.enumerate(
            &catalog.manifest.id,
            &catalog.manifest.version,
            &catalog.balance,
        );
        assert_eq!(slots[7].state, SaveSlotState::Valid);
        let (loaded, _) = store
            .load(
                7,
                &catalog.manifest.id,
                &catalog.manifest.version,
                &catalog.balance,
            )
            .unwrap();
        assert_eq!(loaded.payload, envelope.payload);
    }

    #[test]
    fn injected_import_failure_preserves_source_and_destination_bytes() {
        let catalog = catalog();
        let root = TemporaryDirectory::new();
        let source_path = root.0.join("legacy-input.yaml");
        fs::write(&source_path, fixture()).unwrap();
        let source_before = fs::read(&source_path).unwrap();
        let envelope = convert_python_save(&source_before, false, &catalog, 1_700_000_100).unwrap();
        let store = SaveStore::new(root.0.join("native"));
        let original = crate::save_data::tests::fixture_envelope();
        let destination = store.write(7, &original, false, &catalog.balance).unwrap();
        let destination_before = fs::read(&destination).unwrap();

        assert!(matches!(
            install_python_import_with_hook(
                &store,
                7,
                &envelope,
                true,
                &catalog.balance,
                || Err(PythonImportError::Install("injected failure".to_owned())),
            ),
            Err(PythonImportError::Install(reason)) if reason == "injected failure"
        ));
        assert_eq!(fs::read(source_path).unwrap(), source_before);
        assert_eq!(fs::read(destination).unwrap(), destination_before);
        let backups = fs::read_dir(store.root().join("import-backups"))
            .unwrap()
            .map(|entry| fs::read(entry.unwrap().path()).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(backups, [destination_before]);
    }

    #[test]
    fn converted_native_golden_is_stable() {
        let envelope = convert_python_save(fixture(), false, &catalog(), 1_700_000_100).unwrap();
        let actual = String::from_utf8(envelope.encode().unwrap()).unwrap();
        assert_eq!(
            actual,
            include_str!("../tests/fixtures/python-save-0897035/converted-native-v1.yaml")
        );
    }
}
