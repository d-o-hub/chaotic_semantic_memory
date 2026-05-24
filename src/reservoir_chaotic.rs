//! Chaotic reservoir wrapper for temporal dynamics.

use crate::error::Result;
use crate::hyperdim::HVec10240;
use crate::reservoir::{Reservoir, ReservoirMetrics, ReservoirMetricsSnapshot, ReservoirStepOutput};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use std::sync::Arc;

/// Chaotic reservoir with configurable dynamics
pub struct ChaoticReservoir {
    pub(crate) base: Reservoir,
    pub(crate) chaos_strength: f32,
    pub(crate) rng: StdRng,
    pub(crate) noisy_input: Vec<f32>,
}

impl ChaoticReservoir {
    pub fn new(input_size: usize, size: usize, chaos_strength: f32) -> Result<Self> {
        let seed = rand::rng().random();
        Self::new_seeded(input_size, size, chaos_strength, seed)
    }

    pub fn new_with_metrics(
        input_size: usize,
        size: usize,
        chaos_strength: f32,
        metrics: Arc<ReservoirMetrics>,
    ) -> Result<Self> {
        let seed = rand::rng().random();
        Self::new_seeded_with_metrics(input_size, size, chaos_strength, seed, metrics)
    }

    pub fn new_seeded(
        input_size: usize,
        size: usize,
        chaos_strength: f32,
        seed: u64,
    ) -> Result<Self> {
        Self::new_seeded_with_metrics(
            input_size,
            size,
            chaos_strength,
            seed,
            Arc::new(ReservoirMetrics::default()),
        )
    }

    pub fn new_seeded_with_metrics(
        input_size: usize,
        size: usize,
        chaos_strength: f32,
        seed: u64,
        metrics: Arc<ReservoirMetrics>,
    ) -> Result<Self> {
        Reservoir::validate_params(size, input_size, chaos_strength)?;
        let mut base = Reservoir::new_seeded_with_metrics(input_size, size, seed, metrics)?;
        base.set_spectral_radius(1.0)?;
        Ok(Self {
            base,
            chaos_strength,
            rng: StdRng::seed_from_u64(seed ^ 0xA5A5_5A5A_F0F0_0F0F),
            noisy_input: vec![0.0; input_size],
        })
    }

    pub fn step(&mut self, input: &[f32]) -> Result<ReservoirStepOutput<'_>> {
        if input.len() != self.noisy_input.len() {
            return Err(crate::error::MemoryError::reservoir(format!(
                "Input size mismatch: expected {}, got {}",
                self.noisy_input.len(),
                input.len()
            )));
        }
        for (i, value) in input.iter().enumerate() {
            let noise = if self.chaos_strength > 0.0 {
                self.rng
                    .random_range(-self.chaos_strength..self.chaos_strength)
            } else {
                0.0
            };
            // SAFETY: noisy_input is sized to input_size, same as input.
            unsafe {
                *self.noisy_input.get_unchecked_mut(i) = *value + noise;
            }
        }
        self.base.step(&self.noisy_input)
    }

    pub fn reset(&mut self) {
        self.base.reset();
    }

    pub fn state(&self) -> &[f32] {
        self.base.state()
    }

    pub fn to_hypervector(&self) -> Result<HVec10240> {
        self.base.to_hypervector()
    }

    pub fn metrics_snapshot(&self) -> ReservoirMetricsSnapshot {
        self.base.metrics_snapshot()
    }
}
