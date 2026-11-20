use std::fs;
use std::path::{Path, PathBuf};

use redb::{Database, ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition};

use crate::error::EngineError;

const HTTP_CACHE: TableDefinition<&[u8; 32], &[u8]> = TableDefinition::new("http_cache");

/// Persistent ACID `redb` key-value backend for encoded HTTP cache envelopes.
pub struct RedbCache {
    db: Database,
    path: PathBuf,
}

impl RedbCache {
    /// Open or create the `redb` database at `path` and initialize the `http_cache` table.
    pub fn open(path: &Path) -> Result<Self, redb::Error> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(redb::StorageError::from)?;
        }

        let db = Database::create(path)?;
        let write_txn = db.begin_write()?;
        {
            let _table = write_txn.open_table(HTTP_CACHE)?;
        }
        write_txn.commit()?;

        Ok(Self {
            db,
            path: path.to_path_buf(),
        })
    }

    /// Look up raw encoded envelope bytes by 32-byte canonical cache key.
    pub fn get(&self, key: &[u8; 32]) -> Result<Option<Vec<u8>>, EngineError> {
        let read_txn = self.db.begin_read().map_err(|err| {
            EngineError::Cache(format!("failed to begin redb read transaction: {err}"))
        })?;
        let table = read_txn
            .open_table(HTTP_CACHE)
            .map_err(|err| EngineError::Cache(format!("failed to open redb cache table: {err}")))?;

        let value = table
            .get(key)
            .map_err(|err| EngineError::Cache(format!("failed to read redb cache key: {err}")))?
            .map(|guard| guard.value().to_vec());

        Ok(value)
    }

    /// Persist raw encoded envelope bytes under `key`.
    pub fn put(&self, key: &[u8; 32], value: &[u8]) -> Result<(), EngineError> {
        let write_txn = self.db.begin_write().map_err(|err| {
            EngineError::Cache(format!("failed to begin redb write transaction: {err}"))
        })?;
        {
            let mut table = write_txn.open_table(HTTP_CACHE).map_err(|err| {
                EngineError::Cache(format!("failed to open redb cache table: {err}"))
            })?;
            table.insert(key, value).map_err(|err| {
                EngineError::Cache(format!("failed to write redb cache key: {err}"))
            })?;
        }
        write_txn.commit().map_err(|err| {
            EngineError::Cache(format!("failed to commit redb write transaction: {err}"))
        })?;

        Ok(())
    }

    /// Remove all cached entries while keeping the table initialized.
    pub fn clear(&self) -> Result<(), EngineError> {
        let write_txn = self.db.begin_write().map_err(|err| {
            EngineError::Cache(format!("failed to begin redb clear transaction: {err}"))
        })?;
        write_txn.delete_table(HTTP_CACHE).map_err(|err| {
            EngineError::Cache(format!("failed to delete redb cache table: {err}"))
        })?;
        {
            let _table = write_txn.open_table(HTTP_CACHE).map_err(|err| {
                EngineError::Cache(format!("failed to recreate redb cache table: {err}"))
            })?;
        }
        write_txn.commit().map_err(|err| {
            EngineError::Cache(format!("failed to commit redb clear transaction: {err}"))
        })?;

        Ok(())
    }

    /// Return the total number of persisted entries.
    pub fn entry_count(&self) -> Result<u64, EngineError> {
        let read_txn = self.db.begin_read().map_err(|err| {
            EngineError::Cache(format!("failed to begin redb stats transaction: {err}"))
        })?;
        let table = read_txn
            .open_table(HTTP_CACHE)
            .map_err(|err| EngineError::Cache(format!("failed to open redb cache table: {err}")))?;

        table
            .len()
            .map_err(|err| EngineError::Cache(format!("failed to query redb entry count: {err}")))
    }

    /// Return the on-disk size in bytes of the backing `redb` database file.
    pub fn file_size_bytes(&self) -> u64 {
        fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(prefix: &str) -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "oxirinth-redb-test-{prefix}-{}-{id}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn persists_across_reopen_and_clears_cleanly() {
        let dir = TempDir::new("persist");
        let db_path = dir.0.join("http_cache.redb");
        let key = [42u8; 32];

        {
            let store = RedbCache::open(&db_path).unwrap();
            store.put(&key, b"payload-v1").unwrap();
            assert_eq!(store.entry_count().unwrap(), 1);
            assert!(store.file_size_bytes() > 0);
        }

        let reopened = RedbCache::open(&db_path).unwrap();
        assert_eq!(
            reopened.get(&key).unwrap().as_deref(),
            Some(&b"payload-v1"[..])
        );

        reopened.clear().unwrap();
        assert_eq!(reopened.entry_count().unwrap(), 0);
        assert!(reopened.get(&key).unwrap().is_none());
    }
}
