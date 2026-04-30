use serde::{Deserialize, Serialize};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Default)]
pub(crate) struct QueryCache {
    pub(crate) capacity: usize,
    pub(crate) order: VecDeque<u64>,
    pub(crate) results: HashMap<u64, Arc<[(String, f32)]>>,
}

impl QueryCache {
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            order: VecDeque::new(),
            results: HashMap::new(),
        }
    }

    pub(crate) fn get(&mut self, key: u64) -> Option<Arc<[(String, f32)]>> {
        let value = Arc::clone(self.results.get(&key)?);
        if let Some(pos) = self.order.iter().position(|k| *k == key) {
            self.order.remove(pos);
        }
        self.order.push_back(key);
        Some(value)
    }

    pub(crate) fn put(&mut self, key: u64, value: Arc<[(String, f32)]>) -> bool {
        if let Entry::Occupied(mut entry) = self.results.entry(key) {
            entry.insert(value);
            if let Some(pos) = self.order.iter().position(|k| *k == key) {
                self.order.remove(pos);
            }
            self.order.push_back(key);
            return false;
        }

        let mut evicted = false;
        if self.results.len() >= self.capacity {
            if let Some(oldest) = self.order.pop_front() {
                self.results.remove(&oldest);
                evicted = true;
            }
        }
        self.order.push_back(key);
        self.results.insert(key, value);
        evicted
    }

    pub(crate) fn clear(&mut self) {
        self.order.clear();
        self.results.clear();
    }
}

#[derive(Debug, Default)]
pub(crate) struct CacheMetrics {
    pub(crate) hits_total: AtomicU64,
    pub(crate) misses_total: AtomicU64,
    pub(crate) evictions_total: AtomicU64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheMetricsSnapshot {
    pub cache_hits_total: u64,
    pub cache_misses_total: u64,
    pub cache_evictions_total: u64,
}

impl CacheMetrics {
    pub(crate) fn snapshot(&self) -> CacheMetricsSnapshot {
        CacheMetricsSnapshot {
            cache_hits_total: self.hits_total.load(Ordering::Relaxed),
            cache_misses_total: self.misses_total.load(Ordering::Relaxed),
            cache_evictions_total: self.evictions_total.load(Ordering::Relaxed),
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_results(ids: &[&str]) -> Arc<[(String, f32)]> {
        ids.iter()
            .map(|id| (id.to_string(), 0.5))
            .collect::<Vec<_>>()
            .into()
    }

    #[test]
    fn lru_eviction_at_capacity() {
        let mut cache = QueryCache::with_capacity(3);

        cache.put(1, make_results(&["a"]));
        cache.put(2, make_results(&["b"]));
        cache.put(3, make_results(&["c"]));

        // Cache is at capacity
        assert_eq!(cache.results.len(), 3);

        // Inserting 4 should evict 1 (oldest)
        let evicted = cache.put(4, make_results(&["d"]));
        assert!(evicted);
        assert_eq!(cache.results.len(), 3);
        assert!(!cache.results.contains_key(&1));
        assert!(cache.results.contains_key(&4));
    }

    #[test]
    fn lru_get_updates_order() {
        let mut cache = QueryCache::with_capacity(3);

        cache.put(1, make_results(&["a"]));
        cache.put(2, make_results(&["b"]));
        cache.put(3, make_results(&["c"]));

        // Get key 1 - moves it to most-recently-used position
        cache.get(1);

        // Now insert 4 - should evict 2 (not 1)
        cache.put(4, make_results(&["d"]));
        assert!(cache.results.contains_key(&1)); // 1 should still be there
        assert!(!cache.results.contains_key(&2)); // 2 should be evicted
    }

    #[test]
    fn multiple_evictions_under_pressure() {
        let mut cache = QueryCache::with_capacity(2);

        // Insert 10 items, should cause 8 evictions
        for i in 0..10 {
            cache.put(i, make_results(&[&format!("item-{i}")]));
        }

        assert_eq!(cache.results.len(), 2);
        // Only 8 and 9 should remain
        assert!(cache.results.contains_key(&8));
        assert!(cache.results.contains_key(&9));
    }

    #[test]
    fn capacity_one_edge_case() {
        let mut cache = QueryCache::with_capacity(1);

        cache.put(1, make_results(&["a"]));
        assert_eq!(cache.results.len(), 1);

        // Every insert should evict
        let evicted = cache.put(2, make_results(&["b"]));
        assert!(evicted);
        assert!(!cache.results.contains_key(&1));

        let evicted = cache.put(3, make_results(&["c"]));
        assert!(evicted);
        assert!(!cache.results.contains_key(&2));
    }

    #[test]
    fn clear_removes_all_entries() {
        let mut cache = QueryCache::with_capacity(10);

        for i in 0..5 {
            cache.put(i, make_results(&[&format!("item-{i}")]));
        }

        assert_eq!(cache.results.len(), 5);
        cache.clear();
        assert_eq!(cache.results.len(), 0);
        assert_eq!(cache.order.len(), 0);
    }

    #[test]
    fn get_returns_none_for_missing_key() {
        let mut cache = QueryCache::with_capacity(5);
        cache.put(1, make_results(&["a"]));

        let result = cache.get(999);
        assert!(result.is_none());
    }

    #[test]
    fn put_returns_false_for_update_without_eviction() {
        let mut cache = QueryCache::with_capacity(5);

        cache.put(1, make_results(&["a"]));
        let evicted = cache.put(1, make_results(&["b"])); // Update same key

        assert!(!evicted); // No eviction for update
        assert_eq!(cache.results.len(), 1);
    }

    #[test]
    fn cache_metrics_snapshot() {
        let metrics = CacheMetrics::default();
        metrics.hits_total.store(10, Ordering::Relaxed);
        metrics.misses_total.store(5, Ordering::Relaxed);
        metrics.evictions_total.store(2, Ordering::Relaxed);

        let snap = metrics.snapshot();
        assert_eq!(snap.cache_hits_total, 10);
        assert_eq!(snap.cache_misses_total, 5);
        assert_eq!(snap.cache_evictions_total, 2);
    }

    #[test]
    fn with_capacity_zero_becomes_one() {
        let cache = QueryCache::with_capacity(0);
        assert_eq!(cache.capacity, 1);
    }
}
