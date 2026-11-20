pub mod codec;
pub mod ram_fallback;
pub mod redb_store;

use std::sync::atomic::{AtomicU64, Ordering};

use bytes::Bytes;
use sha2::{Digest, Sha256};

use crate::config::CacheConfig;
use crate::error::EngineError;

use self::codec::{CacheEntry, decode_entry, encode_entry};
use self::ram_fallback::RamCache;
use self::redb_store::RedbCache;

const REDB_FILENAME: &str = "http_cache.redb";

/// Canonical 256-bit cache key isolated by HTTP method, URL, and `auth_fingerprint`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CacheKey([u8; 32]);

impl CacheKey {
    /// Hash `(method, canonical_url, auth_fingerprint)` with SHA-256 and NUL separators.
    pub fn from_request(method: &str, canonical_url: &str, auth_fingerprint: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(method.as_bytes());
        hasher.update(b"\0");
        hasher.update(canonical_url.as_bytes());
        hasher.update(b"\0");
        hasher.update(auth_fingerprint.as_bytes());
        let digest: [u8; 32] = hasher.finalize().into();
        Self(digest)
    }

    /// Expose the raw 32-byte key slice.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Snapshot of runtime cache usage and hit/miss counters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheStats {
    pub backend: &'static str,
    pub entries: u64,
    pub hits: u64,
    pub misses: u64,
    pub revalidations: u64,
    pub size_bytes: u64,
}

enum CacheBackend {
    Redb(RedbCache),
    Ram(RamCache),
    Disabled,
}

/// Unified cache facade managing `redb` persistence, `zstd` compression, and RAM fallback.
pub struct CacheManager {
    backend: CacheBackend,
    zstd_min_bytes: usize,
    zstd_level: i32,
    hits: AtomicU64,
    misses: AtomicU64,
    revalidations: AtomicU64,
}

impl CacheManager {
    /// Open the configured cache backend, gracefully falling back to RAM on disk/lock errors.
    pub fn open(cfg: &CacheConfig) -> Self {
        let backend = Self::init_backend(cfg);
        Self {
            backend,
            zstd_min_bytes: cfg.zstd_min_bytes,
            zstd_level: cfg.zstd_level,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            revalidations: AtomicU64::new(0),
        }
    }

    fn init_backend(cfg: &CacheConfig) -> CacheBackend {
        if !cfg.enabled {
            return CacheBackend::Disabled;
        }

        let Some(cache_dir) = cfg.resolved_cache_dir() else {
            tracing::warn!("unable to resolve OS cache directory; using bounded RAM LRU cache");
            return CacheBackend::Ram(RamCache::new(cfg.ram_fallback_max_bytes));
        };

        let db_path = cache_dir.join(REDB_FILENAME);
        match RedbCache::open(&db_path) {
            Ok(store) => CacheBackend::Redb(store),
            Err(err) => {
                tracing::warn!(
                    path = %db_path.display(),
                    error = %err,
                    "failed to open persistent redb cache; falling back to bounded RAM LRU cache"
                );
                CacheBackend::Ram(RamCache::new(cfg.ram_fallback_max_bytes))
            }
        }
    }

    /// Return the active backend name: `"redb"`, `"ram"`, or `"disabled"`.
    pub fn backend_name(&self) -> &'static str {
        match &self.backend {
            CacheBackend::Redb(_) => "redb",
            CacheBackend::Ram(_) => "ram",
            CacheBackend::Disabled => "disabled",
        }
    }

    /// Look up and decode a cached entry, returning `None` on miss or decode corruption.
    pub fn get(&self, key: &CacheKey) -> Option<CacheEntry> {
        let raw = match &self.backend {
            CacheBackend::Redb(store) => match store.get(key.as_bytes()) {
                Ok(Some(bytes)) => bytes,
                Ok(None) => return None,
                Err(err) => {
                    tracing::warn!(error = %err, "redb cache read failed");
                    return None;
                }
            },
            CacheBackend::Ram(ram) => ram.get(key.as_bytes())?.to_vec(),
            CacheBackend::Disabled => return None,
        };

        match decode_entry(&raw) {
            Ok(entry) => Some(entry),
            Err(err) => {
                tracing::warn!(error = %err, "discarding corrupted cache entry");
                None
            }
        }
    }

    /// Encode and store a cache entry without propagating transient disk-write errors.
    pub fn put(&self, key: &CacheKey, entry: &CacheEntry) {
        if matches!(self.backend, CacheBackend::Disabled) {
            return;
        }

        let encoded = match encode_entry(entry, self.zstd_min_bytes, self.zstd_level) {
            Ok(bytes) => bytes,
            Err(err) => {
                tracing::warn!(error = %err, "failed to encode cache entry");
                return;
            }
        };

        match &self.backend {
            CacheBackend::Redb(store) => {
                if let Err(err) = store.put(key.as_bytes(), &encoded) {
                    tracing::warn!(error = %err, "failed to persist cache entry in redb");
                }
            }
            CacheBackend::Ram(ram) => {
                ram.put(*key.as_bytes(), Bytes::from(encoded));
            }
            CacheBackend::Disabled => {}
        }
    }

    /// Update freshness metadata on `304 Not Modified` while preserving the existing body.
    pub fn refresh_metadata(
        &self,
        key: &CacheKey,
        now_unix: u64,
        ttl_secs: u32,
        etag: Option<String>,
        last_modified: Option<String>,
    ) -> Option<CacheEntry> {
        let mut entry = self.get(key)?;
        entry.stored_at_unix = now_unix;
        entry.ttl_secs = ttl_secs;
        if etag.is_some() {
            entry.etag = etag;
        }
        if last_modified.is_some() {
            entry.last_modified = last_modified;
        }
        self.put(key, &entry);
        Some(entry)
    }

    /// Increment cache hit counter.
    pub fn record_hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment cache miss counter.
    pub fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment conditional revalidation (`304 Not Modified`) counter.
    pub fn record_revalidation(&self) {
        self.revalidations.fetch_add(1, Ordering::Relaxed);
    }

    /// Remove all cached entries from the active backend.
    pub fn clear(&self) -> Result<(), EngineError> {
        match &self.backend {
            CacheBackend::Redb(store) => store.clear(),
            CacheBackend::Ram(ram) => {
                ram.clear();
                Ok(())
            }
            CacheBackend::Disabled => Ok(()),
        }
    }

    /// Return current entry counts, disk/memory footprint, and hit/miss counters.
    pub fn stats(&self) -> CacheStats {
        let (entries, size_bytes) = match &self.backend {
            CacheBackend::Redb(store) => {
                (store.entry_count().unwrap_or(0), store.file_size_bytes())
            }
            CacheBackend::Ram(ram) => (ram.len(), ram.size_bytes()),
            CacheBackend::Disabled => (0, 0),
        };

        CacheStats {
            backend: self.backend_name(),
            entries,
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            revalidations: self.revalidations.load(Ordering::Relaxed),
            size_bytes,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(prefix: &str) -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "oxirinth-cache-mgr-test-{prefix}-{}-{id}",
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
    fn isolates_cache_keys_by_auth_fingerprint() {
        let url = "https://api.modrinth.com/v3/project/sodium";
        let anon_key = CacheKey::from_request("GET", url, "anon");
        let token_key = CacheKey::from_request("GET", url, "user-a-fingerprint");
        assert_ne!(anon_key, token_key);
    }

    #[test]
    fn falls_back_to_ram_lru_when_redb_file_is_locked() {
        let dir = TempDir::new("lock-fallback");
        let db_path = dir.0.join(REDB_FILENAME);

        // Hold an exclusive database handle so opening CacheManager encounters DatabaseAlreadyOpen.
        let _held_lock = redb::Database::create(&db_path).unwrap();

        let mgr = CacheManager::open(&CacheConfig {
            cache_dir: Some(dir.0.clone()),
            ..CacheConfig::default()
        });
        assert_eq!(mgr.backend_name(), "ram");

        let key =
            CacheKey::from_request("GET", "https://api.modrinth.com/v3/project/sodium", "anon");
        let entry = CacheEntry {
            status: 200,
            stored_at_unix: 100,
            ttl_secs: 900,
            etag: Some("\"e1\"".to_owned()),
            last_modified: None,
            body: Bytes::from_static(br#"{"id":"AANobbMI"}"#),
        };
        mgr.put(&key, &entry);
        assert_eq!(mgr.get(&key), Some(entry));
        assert_eq!(mgr.stats().entries, 1);
    }

    #[test]
    fn supports_disabled_mode_and_metadata_refresh() {
        let disabled = CacheManager::open(&CacheConfig {
            enabled: false,
            ..CacheConfig::default()
        });
        assert_eq!(disabled.backend_name(), "disabled");
        let key = CacheKey::from_request("GET", "https://api.modrinth.com/v3/search", "anon");
        let entry = CacheEntry {
            status: 200,
            stored_at_unix: 100,
            ttl_secs: 60,
            etag: Some("\"old\"".to_owned()),
            last_modified: None,
            body: Bytes::from_static(br#"{"hits":[]}"#),
        };
        disabled.put(&key, &entry);
        assert!(disabled.get(&key).is_none());

        let dir = TempDir::new("refresh");
        let redb_mgr = CacheManager::open(&CacheConfig {
            cache_dir: Some(dir.0.clone()),
            ..CacheConfig::default()
        });
        assert_eq!(redb_mgr.backend_name(), "redb");

        redb_mgr.put(&key, &entry);
        let refreshed = redb_mgr
            .refresh_metadata(&key, 500, 120, Some("\"new\"".to_owned()), None)
            .unwrap();
        assert_eq!(refreshed.stored_at_unix, 500);
        assert_eq!(refreshed.ttl_secs, 120);
        assert_eq!(refreshed.etag.as_deref(), Some("\"new\""));
        assert_eq!(refreshed.body, entry.body);
    }
}
