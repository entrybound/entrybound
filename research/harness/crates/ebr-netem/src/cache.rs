//! The CDN cache layer (`ebr-netem-proxy`'s "cache layer mode").
//!
//! A capacity-bounded, size-tracked LRU keyed by `(path, raw Range header)`.
//! `proxy.rs` is the only caller: on a cache hit it skips contacting the
//! origin (and the origin-side leg of the RTT delay) entirely and serves
//! straight from here; on a miss it fetches from the origin and, if the
//! response fits, inserts it. "Warm" vs. "cold" (the task's "warm/cold cache
//! ... configurable cache size and eviction") is a `proxy.rs`-level policy
//! about whether entries are pre-populated at startup -- this module is
//! agnostic to that and only implements the cache itself plus hit/miss/
//! eviction counting.
//!
//! Not revision-aware: an entry is served as-is until evicted, with no
//! revalidation against `ebr-origin`'s ETag. Combining this cache layer with
//! `ebr-origin`'s `--revision-churn-after` is therefore out of scope and
//! will silently serve stale bytes past the churn point -- a documented
//! limitation (see `README.md`), not a bug to fix here.

use hyper::body::Bytes;
use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub path: String,
    pub range: Option<String>,
}

impl CacheKey {
    pub fn new(path: impl Into<String>, range: Option<&str>) -> Self {
        CacheKey {
            path: path.into(),
            range: range.map(str::to_string),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CachedResponse {
    pub status: u16,
    pub etag: Option<String>,
    pub content_range: Option<String>,
    pub content_type: Option<String>,
    pub body: Bytes,
}

#[derive(Debug, Default)]
pub struct CacheStats {
    pub hits: AtomicU64,
    pub misses: AtomicU64,
    /// A response too large to ever fit in the configured capacity: served
    /// to the client normally, just never cached.
    pub bypasses: AtomicU64,
    pub evictions: AtomicU64,
    pub insertions: AtomicU64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheStatsSnapshot {
    pub hits: u64,
    pub misses: u64,
    pub bypasses: u64,
    pub evictions: u64,
    pub insertions: u64,
}

impl CacheStats {
    pub fn snapshot(&self) -> CacheStatsSnapshot {
        CacheStatsSnapshot {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            bypasses: self.bypasses.load(Ordering::Relaxed),
            evictions: self.evictions.load(Ordering::Relaxed),
            insertions: self.insertions.load(Ordering::Relaxed),
        }
    }
}

struct Slot {
    entry: CachedResponse,
    last_used: u64,
}

struct Inner {
    capacity_bytes: u64,
    used_bytes: u64,
    counter: u64,
    entries: HashMap<CacheKey, Slot>,
    /// `last_used` sequence number -> key, so the least-recently-used entry
    /// is always `recency.iter().next()`.
    recency: BTreeMap<u64, CacheKey>,
}

pub struct CdnCache {
    inner: Mutex<Inner>,
    stats: CacheStats,
    enabled: bool,
}

impl CdnCache {
    pub fn new(capacity_bytes: u64, enabled: bool) -> Self {
        CdnCache {
            inner: Mutex::new(Inner {
                capacity_bytes,
                used_bytes: 0,
                counter: 0,
                entries: HashMap::new(),
                recency: BTreeMap::new(),
            }),
            stats: CacheStats::default(),
            enabled,
        }
    }

    pub fn disabled() -> Self {
        Self::new(0, false)
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }

    /// A hit bumps the entry's recency and increments `stats.hits`; a miss
    /// increments `stats.misses`. Always `None` (and counts nothing) when
    /// the cache is disabled -- `proxy.rs` still calls this unconditionally
    /// rather than branching on `enabled()` itself.
    pub fn get(&self, key: &CacheKey) -> Option<CachedResponse> {
        if !self.enabled {
            return None;
        }
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let found = inner
            .entries
            .get(key)
            .map(|slot| (slot.entry.clone(), slot.last_used));
        match found {
            Some((entry, old_seq)) => {
                inner.counter += 1;
                let new_seq = inner.counter;
                inner.recency.remove(&old_seq);
                inner.recency.insert(new_seq, key.clone());
                if let Some(slot) = inner.entries.get_mut(key) {
                    slot.last_used = new_seq;
                }
                self.stats.hits.fetch_add(1, Ordering::Relaxed);
                Some(entry)
            }
            None => {
                self.stats.misses.fetch_add(1, Ordering::Relaxed);
                None
            }
        }
    }

    /// Inserts (or replaces) `key`'s entry, evicting least-recently-used
    /// entries first if needed. An entry larger than the total configured
    /// capacity is never cached at all (`stats.bypasses`); the caller still
    /// serves it to the client directly, just uncached. A no-op when the
    /// cache is disabled.
    pub fn insert(&self, key: CacheKey, entry: CachedResponse) {
        if !self.enabled {
            return;
        }
        let len = entry.body.len() as u64;
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if len > inner.capacity_bytes {
            self.stats.bypasses.fetch_add(1, Ordering::Relaxed);
            return;
        }
        // Replacing an existing entry: drop its old accounting first. This
        // matters when two concurrent misses for the same key both insert
        // -- without it, the first insert's now-orphaned recency pointer
        // would evict based on a stale sequence number and could double
        // count `used_bytes`.
        if let Some(old) = inner.entries.remove(&key) {
            inner.recency.remove(&old.last_used);
            inner.used_bytes = inner.used_bytes.saturating_sub(old.entry.body.len() as u64);
        }
        while inner.used_bytes + len > inner.capacity_bytes {
            let Some((oldest_seq, oldest_key)) = inner
                .recency
                .iter()
                .next()
                .map(|(seq, key)| (*seq, key.clone()))
            else {
                break;
            };
            inner.recency.remove(&oldest_seq);
            if let Some(slot) = inner.entries.remove(&oldest_key) {
                inner.used_bytes = inner
                    .used_bytes
                    .saturating_sub(slot.entry.body.len() as u64);
            }
            self.stats.evictions.fetch_add(1, Ordering::Relaxed);
        }
        inner.counter += 1;
        let seq = inner.counter;
        inner.used_bytes += len;
        inner.entries.insert(
            key.clone(),
            Slot {
                entry,
                last_used: seq,
            },
        );
        inner.recency.insert(seq, key);
        self.stats.insertions.fetch_add(1, Ordering::Relaxed);
    }

    #[cfg(test)]
    fn used_bytes(&self) -> u64 {
        self.inner.lock().unwrap().used_bytes
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.inner.lock().unwrap().entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(n: usize) -> CachedResponse {
        CachedResponse {
            status: 206,
            etag: Some("\"x\"".to_string()),
            content_range: Some(format!("bytes 0-{}/1000", n - 1)),
            content_type: None,
            body: Bytes::from(vec![b'a'; n]),
        }
    }

    #[test]
    fn disabled_cache_never_stores_or_counts_anything() {
        let cache = CdnCache::disabled();
        let key = CacheKey::new("/x.eb", Some("bytes=0-9"));
        assert!(cache.get(&key).is_none());
        cache.insert(key.clone(), entry(10));
        assert!(cache.get(&key).is_none());
        let snap = cache.stats().snapshot();
        assert_eq!(snap.hits, 0);
        assert_eq!(snap.misses, 0);
        assert_eq!(snap.insertions, 0);
    }

    #[test]
    fn miss_then_insert_then_hit_counts_correctly() {
        let cache = CdnCache::new(1_000, true);
        let key = CacheKey::new("/x.eb", Some("bytes=0-9"));
        assert!(cache.get(&key).is_none());
        cache.insert(key.clone(), entry(10));
        let hit = cache.get(&key).expect("should hit after insert");
        assert_eq!(hit.body.len(), 10);
        let snap = cache.stats().snapshot();
        assert_eq!(snap.misses, 1);
        assert_eq!(snap.hits, 1);
        assert_eq!(snap.insertions, 1);
    }

    #[test]
    fn distinct_ranges_of_the_same_path_are_distinct_keys() {
        let cache = CdnCache::new(1_000, true);
        let a = CacheKey::new("/x.eb", Some("bytes=0-9"));
        let b = CacheKey::new("/x.eb", Some("bytes=10-19"));
        cache.insert(a.clone(), entry(10));
        assert!(cache.get(&b).is_none());
        assert!(cache.get(&a).is_some());
    }

    #[test]
    fn an_entry_larger_than_capacity_is_bypassed_not_cached() {
        let cache = CdnCache::new(100, true);
        let key = CacheKey::new("/big.eb", None);
        cache.insert(key.clone(), entry(200));
        assert!(cache.get(&key).is_none());
        assert_eq!(cache.stats().snapshot().bypasses, 1);
        assert_eq!(cache.used_bytes(), 0);
    }

    #[test]
    fn eviction_removes_the_least_recently_used_entry_first() {
        let cache = CdnCache::new(150, true);
        let a = CacheKey::new("/a", None);
        let b = CacheKey::new("/b", None);
        cache.insert(a.clone(), entry(100));
        cache.insert(b.clone(), entry(100)); // 200 > 150 capacity: evicts `a`
        assert!(cache.get(&a).is_none(), "a should have been evicted");
        assert!(cache.get(&b).is_some(), "b should still be cached");
        assert_eq!(cache.stats().snapshot().evictions, 1);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn touching_an_entry_via_get_protects_it_from_the_next_eviction() {
        // A fresh `insert` always gets the newest possible recency (the
        // global counter only advances), so it is deliberately inserted
        // *before* the touch below: touching `a` after both `a` and `b`
        // are already cached is what makes `b` the older of the two, not
        // insertion order alone.
        let cache = CdnCache::new(200, true);
        let a = CacheKey::new("/a", None);
        let b = CacheKey::new("/b", None);
        cache.insert(a.clone(), entry(100));
        cache.insert(b.clone(), entry(50));
        assert_eq!(cache.used_bytes(), 150);

        cache.get(&a); // touch `a`: `b` is now the less-recently-used of the two

        let c = CacheKey::new("/c", None);
        cache.insert(c.clone(), entry(60)); // 150 + 60 > 200: evicts exactly `b`
        assert!(cache.get(&b).is_none(), "b should be evicted, not a");
        assert!(cache.get(&a).is_some());
        assert!(cache.get(&c).is_some());
    }

    #[test]
    fn replacing_an_existing_key_does_not_leak_used_bytes_accounting() {
        let cache = CdnCache::new(1_000, true);
        let key = CacheKey::new("/x.eb", None);
        cache.insert(key.clone(), entry(100));
        cache.insert(key.clone(), entry(50)); // replace with a smaller entry
        assert_eq!(cache.used_bytes(), 50, "old entry's bytes must be released");
        assert_eq!(cache.len(), 1);
    }
}
