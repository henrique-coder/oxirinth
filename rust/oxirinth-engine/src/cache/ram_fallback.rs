use bytes::Bytes;
use lru::LruCache;
use parking_lot::Mutex;

struct RamCacheInner {
    entries: LruCache<[u8; 32], Bytes>,
    current_bytes: usize,
    max_bytes: usize,
}

/// Bounded thread-safe in-memory LRU fallback cache used when `redb` cannot open.
pub struct RamCache {
    inner: Mutex<RamCacheInner>,
}

impl RamCache {
    /// Create a new RAM LRU cache capped at `max_bytes` total stored envelope size.
    pub fn new(max_bytes: usize) -> Self {
        Self {
            inner: Mutex::new(RamCacheInner {
                entries: LruCache::unbounded(),
                current_bytes: 0,
                max_bytes,
            }),
        }
    }

    /// Retrieve encoded envelope bytes for `key`, promoting it to most-recently-used.
    pub fn get(&self, key: &[u8; 32]) -> Option<Bytes> {
        let mut guard = self.inner.lock();
        guard.entries.get(key).cloned()
    }

    /// Insert or replace encoded envelope bytes while enforcing `max_bytes`.
    pub fn put(&self, key: [u8; 32], value: Bytes) {
        let mut guard = self.inner.lock();
        if value.len() > guard.max_bytes {
            return;
        }

        if let Some(previous) = guard.entries.pop(&key) {
            guard.current_bytes = guard.current_bytes.saturating_sub(previous.len());
        }

        while guard.current_bytes + value.len() > guard.max_bytes {
            let Some((_, evicted)) = guard.entries.pop_lru() else {
                break;
            };
            guard.current_bytes = guard.current_bytes.saturating_sub(evicted.len());
        }

        guard.current_bytes += value.len();
        guard.entries.put(key, value);
    }

    /// Remove all entries from memory.
    pub fn clear(&self) {
        let mut guard = self.inner.lock();
        guard.entries.clear();
        guard.current_bytes = 0;
    }

    /// Return the current number of cached entries.
    pub fn len(&self) -> u64 {
        let guard = self.inner.lock();
        guard.entries.len() as u64
    }

    /// Return whether no entries are currently stored.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return total payload bytes currently retained in memory.
    pub fn size_bytes(&self) -> u64 {
        let guard = self.inner.lock();
        guard.current_bytes as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    #[test]
    fn evicts_least_recently_used_entries_under_byte_budget() {
        let cache = RamCache::new(10);
        cache.put(key(1), Bytes::from_static(b"12345"));
        cache.put(key(2), Bytes::from_static(b"67890"));
        assert_eq!(cache.size_bytes(), 10);

        // Touch key(1) so key(2) becomes the LRU candidate.
        assert_eq!(cache.get(&key(1)).as_deref(), Some(&b"12345"[..]));

        cache.put(key(3), Bytes::from_static(b"abcde"));
        assert!(cache.get(&key(2)).is_none());
        assert_eq!(cache.get(&key(1)).as_deref(), Some(&b"12345"[..]));
        assert_eq!(cache.get(&key(3)).as_deref(), Some(&b"abcde"[..]));
        assert_eq!(cache.size_bytes(), 10);
    }

    #[test]
    fn skips_single_entry_larger_than_budget_and_updates_accounting() {
        let cache = RamCache::new(8);
        cache.put(key(9), Bytes::from_static(b"oversized-entry"));
        assert_eq!(cache.len(), 0);

        cache.put(key(1), Bytes::from_static(b"aaaa"));
        cache.put(key(1), Bytes::from_static(b"bbbbbb"));
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.size_bytes(), 6);
        cache.clear();
        assert_eq!(cache.len(), 0);
        assert_eq!(cache.size_bytes(), 0);
    }
}
