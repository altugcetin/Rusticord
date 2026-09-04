mod secrets;
mod settings;

pub use secrets::{SessionToken, TokenStore};
pub use settings::{Settings, SettingsStore, StoredAppearance, StoredLocale};

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("data directory is unavailable")]
    DataDirectory,
    #[error("settings database failed")]
    Database,
    #[error("settings record was not valid")]
    InvalidRecord,
    #[error("credential store failed")]
    Secrets,
}

impl From<redb::DatabaseError> for StoreError {
    fn from(_: redb::DatabaseError) -> Self {
        Self::Database
    }
}

impl From<redb::TransactionError> for StoreError {
    fn from(_: redb::TransactionError) -> Self {
        Self::Database
    }
}

impl From<redb::TableError> for StoreError {
    fn from(_: redb::TableError) -> Self {
        Self::Database
    }
}

impl From<redb::StorageError> for StoreError {
    fn from(_: redb::StorageError) -> Self {
        Self::Database
    }
}

impl From<redb::CommitError> for StoreError {
    fn from(_: redb::CommitError) -> Self {
        Self::Database
    }
}
