//! Reservoir metrics tracking for performance monitoring.

use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Default)]
pub(crate) struct ReservoirMetrics {
    pub(crate) steps_total: AtomicU64,
    pub(crate) step_latency_us_total: AtomicU64,
    pub(crate) step_latency_count: AtomicU64,
    pub(crate) nodes_active: AtomicU64,
}

#[derive(Debug, Clone, Default)]
pub struct ReservoirMetricsSnapshot {
    pub reservoir_steps_total: u64,
    pub avg_reservoir_step_latency_us: f64,
    pub reservoir_nodes_active: u64,
}

impl ReservoirMetrics {
    pub(crate) fn observe_step(&self, latency_us: u64, nodes_active: u64) {
        self.steps_total.fetch_add(1, Ordering::Relaxed);
        self.step_latency_us_total
            .fetch_add(latency_us, Ordering::Relaxed);
        self.step_latency_count.fetch_add(1, Ordering::Relaxed);
        self.nodes_active.store(nodes_active, Ordering::Relaxed);
    }

    pub(crate) fn snapshot(&self) -> ReservoirMetricsSnapshot {
        let count = self.step_latency_count.load(Ordering::Relaxed);
        let total = self.step_latency_us_total.load(Ordering::Relaxed);
        let avg = if count == 0 {
            0.0
        } else {
            total as f64 / count as f64
        };
        ReservoirMetricsSnapshot {
            reservoir_steps_total: self.steps_total.load(Ordering::Relaxed),
            avg_reservoir_step_latency_us: avg,
            reservoir_nodes_active: self.nodes_active.load(Ordering::Relaxed),
        }
    }
}
