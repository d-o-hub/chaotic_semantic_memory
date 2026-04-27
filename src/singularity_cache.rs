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
