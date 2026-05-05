#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
//! Framework metrics for performance monitoring

use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Default)]
pub struct FrameworkMetrics {
    pub(crate) concepts_injected_total: AtomicU64,
    pub(crate) associations_created_total: AtomicU64,
    pub(crate) probes_total: AtomicU64,
    pub(crate) probe_latency_ms_total: AtomicU64,
    pub(crate) probe_latency_count: AtomicU64,
    pub(crate) persist_latency_ms_total: AtomicU64,
    pub(crate) persist_latency_count: AtomicU64,
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
}

impl FrameworkMetrics {
    pub(crate) fn inc_concepts_injected(&self, count: u64, _with_metadata: bool) {
        self.concepts_injected_total
            .fetch_add(count, Ordering::Relaxed);
        #[cfg(feature = "prometheus")]
        {
            crate::observability::prom::INJECT_TOTAL
                .with_label_values(&[&_with_metadata.to_string()])
                .inc_by(count as f64);
        }
    }

    pub(crate) fn inc_associations_created(&self, count: u64) {
        self.associations_created_total
            .fetch_add(count, Ordering::Relaxed);
        #[cfg(feature = "prometheus")]
        {
            crate::observability::prom::ASSOCIATIONS_TOTAL.inc_by(count as f64);
        }
    }

    pub(crate) fn observe_probe_latency_ms(&self, latency_ms: u64, _top_k: usize, _success: bool) {
        self.probes_total.fetch_add(1, Ordering::Relaxed);
        self.probe_latency_ms_total
            .fetch_add(latency_ms, Ordering::Relaxed);
        self.probe_latency_count.fetch_add(1, Ordering::Relaxed);

        #[cfg(feature = "prometheus")]
        {
            let result_str = if _success { "ok" } else { "error" };
            crate::observability::prom::PROBE_TOTAL
                .with_label_values(&[result_str])
                .inc();
            crate::observability::prom::PROBE_LATENCY
                .with_label_values(&[&_top_k.to_string()])
                .observe(latency_ms as f64);
        }
    }

    pub(crate) fn observe_persist_latency_ms(&self, latency_ms: u64, _op: &str) {
        self.persist_latency_ms_total
            .fetch_add(latency_ms, Ordering::Relaxed);
        self.persist_latency_count.fetch_add(1, Ordering::Relaxed);

        #[cfg(feature = "prometheus")]
        {
            crate::observability::prom::PERSIST_LATENCY
                .with_label_values(&[_op])
                .observe(latency_ms as f64);
        }
    }

    // No-op without `prometheus` feature; cannot be `const fn` because the
    // feature-gated branch calls non-const Prometheus setters.
    pub(crate) fn update_gauges(
        &self,
        _concept_count: u64,
        _association_count: u64,
        _cache_hit_ratio: f64,
    ) {
        #[cfg(feature = "prometheus")]
        {
            crate::observability::prom::CONCEPTS_COUNT.set(_concept_count as f64);
            crate::observability::prom::ASSOCIATIONS_COUNT.set(_association_count as f64);
            crate::observability::prom::CACHE_HIT_RATIO.set(_cache_hit_ratio);
        }
    }

    pub(crate) fn snapshot(&self) -> FrameworkMetricsSnapshot {
        let count = self.probe_latency_count.load(Ordering::Relaxed);
        let total = self.probe_latency_ms_total.load(Ordering::Relaxed);
        // Atomic u64 to f64 for average latency
        let avg = if count == 0 {
            0.0
        } else {
            total as f64 / count as f64
        };

        FrameworkMetricsSnapshot {
            concepts_injected_total: self.concepts_injected_total.load(Ordering::Relaxed),
            associations_created_total: self.associations_created_total.load(Ordering::Relaxed),
            probes_total: self.probes_total.load(Ordering::Relaxed),
            avg_probe_latency_ms: avg,
            cache_hits_total: 0,
            cache_misses_total: 0,
            cache_evictions_total: 0,
            reservoir_steps_total: 0,
            avg_reservoir_step_latency_us: 0.0,
            reservoir_nodes_active: 0,
        }
    }
}
