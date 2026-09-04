use std::path::{Path, PathBuf};

use redb::{Database, TableDefinition};
use serde::{Deserialize, Serialize};

use crate::StoreError;

const SETTINGS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("settings");
const SETTINGS_KEY: &str = "app";
const SETTINGS_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, Default)]
pub enum StoredLocale {
    #[default]
    Turkish,
    English,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, Default)]
pub enum StoredAppearance {
    #[default]
    Dark,
    Light,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub version: u32,
    pub tos_accepted: bool,
    pub locale: StoredLocale,
    pub appearance: StoredAppearance,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            tos_accepted: false,
            locale: StoredLocale::Turkish,
            appearance: StoredAppearance::Dark,
        }
    }
}

pub struct SettingsStore {
    database: Database,
}

impl SettingsStore {
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|_| StoreError::DataDirectory)?;
        }
        Ok(Self {
            database: Database::create(path)?,
        })
    }

    pub fn open_default() -> Result<Self, StoreError> {
        Self::open(&default_settings_path()?)
    }

    pub fn load(&self) -> Result<Settings, StoreError> {
        let read = self.database.begin_read()?;
        let table = match read.open_table(SETTINGS_TABLE) {
            Ok(table) => table,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(Settings::default()),
            Err(_) => return Err(StoreError::Database),
        };
        match table.get(SETTINGS_KEY)? {
            Some(value) => decode_settings(value.value()),
            None => Ok(Settings::default()),
        }
    }

    pub fn save(&self, settings: &Settings) -> Result<(), StoreError> {
        let encoded = postcard::to_stdvec(settings).map_err(|_| StoreError::InvalidRecord)?;
        let write = self.database.begin_write()?;
        {
            let mut table = write.open_table(SETTINGS_TABLE)?;
            table.insert(SETTINGS_KEY, encoded.as_slice())?;
        }
        write.commit()?;
        Ok(())
    }
}

pub fn default_settings_path() -> Result<PathBuf, StoreError> {
    let mut path = rusticord_platform::data_directory().ok_or(StoreError::DataDirectory)?;
    path.push("settings.redb");
    Ok(path)
}

fn decode_settings(bytes: &[u8]) -> Result<Settings, StoreError> {
    let mut settings: Settings =
        postcard::from_bytes(bytes).map_err(|_| StoreError::InvalidRecord)?;
    if settings.version == 0 {
        return Err(StoreError::InvalidRecord);
    }
    if settings.version > SETTINGS_VERSION {
        return Err(StoreError::InvalidRecord);
    }
    settings.version = SETTINGS_VERSION;
    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::{Settings, SettingsStore, StoredAppearance, StoredLocale};
    use std::sync::atomic::{AtomicU64, Ordering};

    static UNIQUE: AtomicU64 = AtomicU64::new(0);

    fn temp_path() -> std::path::PathBuf {
        let id = UNIQUE.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!(
            "rusticord-settings-{}-{id}.redb",
            std::process::id()
        ))
    }

    #[test]
    fn missing_database_loads_defaults() {
        let path = temp_path();
        let _ = std::fs::remove_file(&path);
        let store = SettingsStore::open(&path).unwrap();
        let settings = store.load().unwrap();
        assert!(!settings.tos_accepted);
        assert_eq!(settings.locale, StoredLocale::Turkish);
        assert_eq!(settings.appearance, StoredAppearance::Dark);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tos_acceptance_roundtrips() {
        let path = temp_path();
        let _ = std::fs::remove_file(&path);
        let store = SettingsStore::open(&path).unwrap();
        let settings = Settings {
            tos_accepted: true,
            locale: StoredLocale::English,
            appearance: StoredAppearance::Light,
            ..Settings::default()
        };
        store.save(&settings).unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded, settings);
        let _ = std::fs::remove_file(&path);
    }
}
