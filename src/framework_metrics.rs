#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
//! Framework metrics for performance monitoring

use crate::singularity_cache::CacheMetrics;
use csm_core_lib::reservoir::ReservoirMetrics;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug)]
pub struct FrameworkMetrics {
    pub(crate) concepts_injected_total: AtomicU64,
    pub(crate) associations_created_total: AtomicU64,
    pub(crate) probes_total: AtomicU64,
    pub(crate) probe_latency_ms_total: AtomicU64,
    pub(crate) probe_latency_count: AtomicU64,
    pub(crate) persist_latency_ms_total: AtomicU64,
    pub(crate) persist_latency_count: AtomicU64,
    pub(crate) delete_concepts_total: AtomicU64,
    pub(crate) traversals_total: AtomicU64,
    pub(crate) shortest_path_total: AtomicU64,
    pub(crate) disassociations_total: AtomicU64,
    pub(crate) cache_metrics: Arc<CacheMetrics>,
    pub(crate) reservoir_metrics: Arc<ReservoirMetrics>,
}

impl Default for FrameworkMetrics {
    fn default() -> Self {
        Self {
            concepts_injected_total: AtomicU64::new(0),
            associations_created_total: AtomicU64::new(0),
            probes_total: AtomicU64::new(0),
            probe_latency_ms_total: AtomicU64::new(0),
            probe_latency_count: AtomicU64::new(0),
            persist_latency_ms_total: AtomicU64::new(0),
            persist_latency_count: AtomicU64::new(0),
            delete_concepts_total: AtomicU64::new(0),
            traversals_total: AtomicU64::new(0),
            shortest_path_total: AtomicU64::new(0),
            disassociations_total: AtomicU64::new(0),
            cache_metrics: Arc::new(CacheMetrics::default()),
            reservoir_metrics: Arc::new(ReservoirMetrics::default()),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn test_avg_latency_no_division_by_zero() {
        let metrics = FrameworkMetrics::default();
        let snapshot = metrics.snapshot();
        assert!(snapshot.avg_probe_latency_ms < f64::EPSILON);
        assert!(snapshot.avg_persist_latency_ms < f64::EPSILON);
        assert_eq!(snapshot.persist_ops_total, 0);
        assert_eq!(snapshot.probes_total, 0);
    }

    #[test]
    fn test_snapshot_completeness() {
        let metrics = FrameworkMetrics::default();
        metrics.concepts_injected_total.store(1, Ordering::Relaxed);
        metrics
            .associations_created_total
            .store(2, Ordering::Relaxed);
        metrics.probes_total.store(3, Ordering::Relaxed);
        metrics.observe_probe_latency_ms(100);
        metrics.observe_persist_latency_ms(200, "test");
        metrics.inc_delete_concepts(5);
        metrics.inc_traversals();
        metrics.inc_shortest_path();
        metrics.inc_disassociations();

        metrics
            .cache_metrics
            .hits_total
            .store(10, Ordering::Relaxed);
        metrics
            .cache_metrics
            .misses_total
            .store(5, Ordering::Relaxed);
        metrics
            .cache_metrics
            .evictions_total
            .store(2, Ordering::Relaxed);

        metrics
            .reservoir_metrics
            .steps_total
            .store(100, Ordering::Relaxed);
        metrics
            .reservoir_metrics
            .step_latency_us_total
            .store(5000, Ordering::Relaxed);
        metrics
            .reservoir_metrics
            .step_latency_count
            .store(100, Ordering::Relaxed);
        metrics
            .reservoir_metrics
            .nodes_active
            .store(50000, Ordering::Relaxed);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.concepts_injected_total, 1);
        assert_eq!(snapshot.associations_created_total, 2);
        assert_eq!(snapshot.probes_total, 4); // 3 + 1 from observe_probe_latency_ms
        assert!((snapshot.avg_probe_latency_ms - 100.0).abs() < f64::EPSILON);
        assert_eq!(snapshot.cache_hits_total, 10);
        assert_eq!(snapshot.cache_misses_total, 5);
        assert_eq!(snapshot.cache_evictions_total, 2);
        assert_eq!(snapshot.reservoir_steps_total, 100);
        assert!((snapshot.avg_reservoir_step_latency_us - 50.0).abs() < f64::EPSILON);
        assert_eq!(snapshot.reservoir_nodes_active, 50000);
        assert_eq!(snapshot.persist_ops_total, 1);
        assert!((snapshot.avg_persist_latency_ms - 200.0).abs() < f64::EPSILON);
        assert_eq!(snapshot.delete_concepts_total, 5);
        assert_eq!(snapshot.traversals_total, 1);
        assert_eq!(snapshot.shortest_path_total, 1);
        assert_eq!(snapshot.disassociations_total, 1);
    }
}

#[derive(Debug, Clone)]
pub struct FrameworkMetricsSnapshot {
    pub concepts_injected_total: u64,
    pub associations_created_total: u64,
    pub probes_total: u64,
    pub avg_probe_latency_ms: f64,
    pub cache_hits_total: u64,
    pub cache_misses_total: u64,
    pub cache_evictions_total: u64,
    pub reservoir_steps_total: u64,
    pub avg_reservoir_step_latency_us: f64,
    pub reservoir_nodes_active: u64,
    pub persist_ops_total: u64,
    pub avg_persist_latency_ms: f64,
    pub delete_concepts_total: u64,
    pub traversals_total: u64,
    pub shortest_path_total: u64,
    pub disassociations_total: u64,
}

impl FrameworkMetrics {
    pub(crate) fn inc_concepts_injected(&self, count: u64) {
        self.concepts_injected_total
            .fetch_add(count, Ordering::Relaxed);
        #[cfg(feature = "prometheus")]
        {
            crate::observability::prom::record_inject(false);
            let n = self.concepts_injected_total.load(Ordering::Relaxed) as i64;
            crate::observability::prom::set_concepts_count(n);
        }
    }

    pub(crate) fn inc_associations_created(&self, count: u64) {
        self.associations_created_total
            .fetch_add(count, Ordering::Relaxed);
        #[cfg(feature = "prometheus")]
        {
            let n = self.associations_created_total.load(Ordering::Relaxed) as i64;
            crate::observability::prom::set_associations_count(n);
        }
    }

    pub(crate) fn observe_probe_latency_ms(&self, latency_ms: u64) {
        self.probes_total.fetch_add(1, Ordering::Relaxed);
        self.probe_latency_ms_total
            .fetch_add(latency_ms, Ordering::Relaxed);
        self.probe_latency_count.fetch_add(1, Ordering::Relaxed);
        #[cfg(feature = "prometheus")]
        crate::observability::prom::record_probe("ok", latency_ms as f64);
    }

    pub(crate) fn observe_persist_latency_ms(&self, latency_ms: u64, _op: &str) {
        self.persist_latency_ms_total
            .fetch_add(latency_ms, Ordering::Relaxed);
        self.persist_latency_count.fetch_add(1, Ordering::Relaxed);
        #[cfg(feature = "prometheus")]
        crate::observability::prom::record_persist(_op, latency_ms as f64);
    }

    pub(crate) fn inc_delete_concepts(&self, count: u64) {
        self.delete_concepts_total
            .fetch_add(count, Ordering::Relaxed);
    }

    pub(crate) fn inc_traversals(&self) {
        self.traversals_total.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn inc_shortest_path(&self) {
        self.shortest_path_total.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn inc_disassociations(&self) {
        self.disassociations_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> FrameworkMetricsSnapshot {
        let avg_probe = calculate_avg(
            self.probe_latency_ms_total.load(Ordering::Relaxed),
            self.probe_latency_count.load(Ordering::Relaxed),
        );

        let persist_count = self.persist_latency_count.load(Ordering::Relaxed);
        let avg_persist = calculate_avg(
            self.persist_latency_ms_total.load(Ordering::Relaxed),
            persist_count,
        );

        let cache = self.cache_metrics.snapshot();
        let reservoir = self.reservoir_metrics.snapshot();

        FrameworkMetricsSnapshot {
            concepts_injected_total: self.concepts_injected_total.load(Ordering::Relaxed),
            associations_created_total: self.associations_created_total.load(Ordering::Relaxed),
            probes_total: self.probes_total.load(Ordering::Relaxed),
            avg_probe_latency_ms: avg_probe,
            cache_hits_total: cache.cache_hits_total,
            cache_misses_total: cache.cache_misses_total,
            cache_evictions_total: cache.cache_evictions_total,
            reservoir_steps_total: reservoir.reservoir_steps_total,
            avg_reservoir_step_latency_us: reservoir.avg_reservoir_step_latency_us,
            reservoir_nodes_active: reservoir.reservoir_nodes_active,
            persist_ops_total: persist_count,
            avg_persist_latency_ms: avg_persist,
            delete_concepts_total: self.delete_concepts_total.load(Ordering::Relaxed),
            traversals_total: self.traversals_total.load(Ordering::Relaxed),
            shortest_path_total: self.shortest_path_total.load(Ordering::Relaxed),
            disassociations_total: self.disassociations_total.load(Ordering::Relaxed),
        }
    }
}

fn calculate_avg(total: u64, count: u64) -> f64 {
    if count == 0 {
        0.0
    } else {
        total as f64 / count as f64
    }
}
