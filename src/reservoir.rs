//! Echo State Network for temporal dynamics.

use crate::error::{MemoryError, Result};
use crate::hyperdim::HVec10240;
use crate::reservoir_sparse::SparseWeights;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
#[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
use rayon::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(not(target_arch = "wasm32"))]
use {std::time::Instant, tracing::instrument};

#[derive(Debug, Default)]
struct ReservoirMetrics {
    steps_total: AtomicU64,
    step_latency_us_total: AtomicU64,
    step_latency_count: AtomicU64,
    nodes_active: AtomicU64,
}

#[derive(Debug, Clone, Default)]
pub struct ReservoirMetricsSnapshot {
    pub reservoir_steps_total: u64,
    pub avg_reservoir_step_latency_us: f64,
    pub reservoir_nodes_active: u64,
}
impl ReservoirMetrics {
    fn observe_step(&self, latency_us: u64, nodes_active: u64) {
        self.steps_total.fetch_add(1, Ordering::Relaxed);
        self.step_latency_us_total
            .fetch_add(latency_us, Ordering::Relaxed);
        self.step_latency_count.fetch_add(1, Ordering::Relaxed);
        self.nodes_active.store(nodes_active, Ordering::Relaxed);
    }

    fn snapshot(&self) -> ReservoirMetricsSnapshot {
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
/// Sparse Echo State Network with chaotic dynamics
pub struct Reservoir {
    size: usize,
    input_size: usize,
    state: Vec<f32>,
    scratch: Vec<f32>,
    prev_state: Vec<f32>, // ADR-0064: t-1 state for inertia
    w_in: SparseWeights,
    w_res: SparseWeights,
    input_cache: Vec<f32>,
    input_projection: Vec<f32>,
    input_projection_valid: bool,
    update_stride: usize,
    update_phase: usize,
    spectral_radius: f32,
    alpha: f32,
    pub(crate) beta: f32, // ADR-0064: inertia coefficient
    metrics: ReservoirMetrics,
}
impl Reservoir {
    pub const DEFAULT_SIZE: usize = 50000;
    pub const DEFAULT_RADIUS: f32 = 0.95;
    pub const DEFAULT_ALPHA: f32 = 0.3;
    const INPUT_DEGREE: usize = 4;
    const RESERVOIR_DEGREE: usize = 8;
    const RESERVOIR_LOCAL_WINDOW: usize = 512;
    const PARTIAL_UPDATE_STRIDE: usize = 32;
    pub const MAX_SIZE: usize = 100_000;

    pub(crate) fn validate_params(size: usize, input_size: usize, chaos: f32) -> Result<()> {
        if size == 0 || size > Self::MAX_SIZE {
            return Err(MemoryError::InvalidInput {
                field: "reservoir_size".into(),
                reason: format!("must be 1..={}", Self::MAX_SIZE),
            });
        }
        if input_size == 0 || input_size > Self::MAX_SIZE {
            return Err(MemoryError::InvalidInput {
                field: "reservoir_input_size".into(),
                reason: format!("must be 1..={}", Self::MAX_SIZE),
            });
        }
        if !chaos.is_finite() || chaos < 0.0 {
            return Err(MemoryError::InvalidInput {
                field: "chaos_strength".into(),
                reason: "must be finite and non-negative".into(),
            });
        }
        Ok(())
    }

    pub fn new(input_size: usize, size: usize) -> Result<Self> {
        let seed = rand::rng().random();
        Self::new_seeded(input_size, size, seed)
    }

    pub fn new_seeded(input_size: usize, size: usize, seed: u64) -> Result<Self> {
        Self::validate_params(size, input_size, 0.0)?;
        let mut rng = StdRng::seed_from_u64(seed);

        let w_in = SparseWeights::build(size, input_size, Self::INPUT_DEGREE, &mut rng);
        let mut w_res = SparseWeights::build_local_reservoir(
            size,
            Self::RESERVOIR_DEGREE.min(size),
            Self::RESERVOIR_LOCAL_WINDOW.min(size),
            &mut rng,
        );

        let current_radius = Self::estimate_spectral_radius(&w_res, size);
        if current_radius > 0.0 {
            let scale = Self::DEFAULT_RADIUS / current_radius;
            w_res.scale(scale);
        }

        Ok(Self {
            size,
            input_size,
            state: vec![0.0; size],
            scratch: vec![0.0; size],
            prev_state: vec![0.0; size], // ADR-0064
            w_in,
            w_res,
            input_cache: vec![0.0; input_size],
            input_projection: vec![0.0; size],
            input_projection_valid: false,
            update_stride: Self::PARTIAL_UPDATE_STRIDE,
            update_phase: 0,
            spectral_radius: Self::DEFAULT_RADIUS,
            alpha: Self::DEFAULT_ALPHA,
            beta: 0.0, // ADR-0064: default no inertia
            metrics: ReservoirMetrics::default(),
        })
    }

    /// Single reservoir step
    #[cfg_attr(not(target_arch = "wasm32"), instrument(skip(self)))]
    pub fn step(&mut self, input: &[f32]) -> Result<&[f32]> {
        #[cfg(not(target_arch = "wasm32"))]
        let started = Instant::now();
        if input.len() != self.input_size {
            return Err(MemoryError::reservoir(format!(
                "Input size mismatch: expected {}, got {}",
                self.input_size,
                input.len()
            )));
        }

        if !self.input_projection_valid || self.input_cache != input {
            self.input_cache.copy_from_slice(input);
            self.input_projection_valid = true;

            #[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
            {
                let w_in = &self.w_in;
                self.input_projection
                    .par_iter_mut()
                    .enumerate()
                    .for_each(|(i, out)| {
                        *out = w_in.dot_row(i, input);
                    });
            }

            #[cfg(any(target_arch = "wasm32", not(feature = "parallel")))]
            for (i, out) in self.input_projection.iter_mut().enumerate() {
                *out = self.w_in.dot_row(i, input);
            }
        }

        let state = &self.state;
        let one_minus_alpha = 1.0 - self.alpha;
        let beta = self.beta;
        let prev_state = &self.prev_state;
        let update_phase = self.update_phase;
        for i in (update_phase..self.size).step_by(self.update_stride) {
            let res_sum = self.w_res.dot_row(i, state);
            let activated = fast_tanh(self.input_projection[i] + res_sum);
            let inertial = beta * (state[i] - prev_state[i]);
            self.scratch[i] = state[i] * one_minus_alpha + activated * self.alpha + inertial;
        }
        self.update_phase = (update_phase + 1) % self.update_stride;
        for i in (update_phase..self.size).step_by(self.update_stride) {
            self.prev_state[i] = self.state[i];
        }
        std::mem::swap(&mut self.state, &mut self.scratch);

        // Keep scratch in sync with state for the next partial-update step.
        for i in (update_phase..self.size).step_by(self.update_stride) {
            self.scratch[i] = self.state[i];
        }

        #[cfg(not(target_arch = "wasm32"))]
        let latency_us = started.elapsed().as_micros() as u64;
        #[cfg(target_arch = "wasm32")]
        let latency_us = 0;
        self.metrics.observe_step(latency_us, self.size as u64);
        Ok(&self.state)
    }

    /// Run reservoir for multiple steps
    #[cfg_attr(not(target_arch = "wasm32"), instrument(skip(self)))]
    pub fn run(&mut self, inputs: &[Vec<f32>]) -> Result<Vec<Vec<f32>>> {
        let mut states = Vec::with_capacity(inputs.len());
        for input in inputs {
            self.step(input)?;
            states.push(self.state.clone());
        }
        Ok(states)
    }

    /// Get current reservoir state
    pub fn state(&self) -> &[f32] {
        &self.state
    }

    /// Set spectral radius
    #[cfg_attr(not(target_arch = "wasm32"), instrument(skip(self)))]
    pub fn set_spectral_radius(&mut self, radius: f32) -> Result<()> {
        if !(0.9..=1.1).contains(&radius) {
            return Err(MemoryError::reservoir(
                "Spectral radius must be in [0.9, 1.1]".to_string(),
            ));
        }

        let current = Self::estimate_spectral_radius(&self.w_res, self.size);
        if current > 0.0 {
            let scale = radius / current;
            self.w_res.scale(scale);
            self.spectral_radius = radius;
        }

        Ok(())
    }

    /// Reset reservoir state
    #[cfg_attr(not(target_arch = "wasm32"), instrument(skip(self)))]
    pub fn reset(&mut self) {
        self.state.fill(0.0);
        self.scratch.fill(0.0);
        self.prev_state.fill(0.0);
    }

    /// Project state to hypervector (parallel on non-WASM)
    #[cfg_attr(
        all(not(target_arch = "wasm32"), feature = "parallel"),
        instrument(skip(self))
    )]
    #[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
    pub fn to_hypervector(&self) -> Result<HVec10240> {
        if self.size < HVec10240::DIMENSION {
            return Err(MemoryError::InvalidDimension {
                expected: HVec10240::DIMENSION,
                actual: self.size,
            });
        }
        let chunk_size = self.size / HVec10240::DIMENSION;
        let data: Vec<u128> = (0..80)
            .into_par_iter()
            .map(|i| {
                let mut word = 0u128;
                for j in 0..128 {
                    let bit_index = i * 128 + j;
                    let sum: f32 = self.state
                        [(bit_index * chunk_size)..(bit_index * chunk_size + chunk_size)]
                        .iter()
                        .sum();
                    if sum > 0.0 {
                        word |= 1u128 << j;
                    }
                }
                word
            })
            .collect();
        let data: [u128; 80] = data.try_into().map_err(|_| {
            MemoryError::reservoir(
                "internal: par_iter produced unexpected element count".to_string(),
            )
        })?;
        Ok(HVec10240 { data })
    }

    #[cfg(any(target_arch = "wasm32", not(feature = "parallel")))]
    #[cfg_attr(not(target_arch = "wasm32"), instrument(skip(self)))]
    pub fn to_hypervector(&self) -> Result<HVec10240> {
        if self.size < HVec10240::DIMENSION {
            return Err(MemoryError::InvalidDimension {
                expected: HVec10240::DIMENSION,
                actual: self.size,
            });
        }
        let chunk_size = self.size / HVec10240::DIMENSION;
        let mut data = [0u128; 80];
        for (i, word) in data.iter_mut().enumerate() {
            for j in 0..128 {
                let bit_index = i * 128 + j;
                let sum: f32 = self.state
                    [(bit_index * chunk_size)..(bit_index * chunk_size + chunk_size)]
                    .iter()
                    .sum();
                if sum > 0.0 {
                    *word |= 1u128 << j;
                }
            }
        }
        Ok(HVec10240 { data })
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn metrics_snapshot(&self) -> ReservoirMetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Estimate spectral radius using power iteration
    fn estimate_spectral_radius(w: &SparseWeights, size: usize) -> f32 {
        let mut v = vec![1.0f32 / size as f32; size];
        let mut y = vec![0.0f32; size];

        for _ in 0..16 {
            for (i, y_i) in y.iter_mut().enumerate() {
                *y_i = w.dot_row(i, &v);
            }

            let mut norm = 0.0f32;
            for val in &y {
                norm += val * val;
            }
            norm = norm.sqrt();
            if norm == 0.0 {
                return 0.0;
            }

            for i in 0..size {
                v[i] = y[i] / norm;
            }
        }

        let mut wv = vec![0.0f32; size];
        for (i, wv_i) in wv.iter_mut().enumerate() {
            *wv_i = w.dot_row(i, &v);
        }

        let mut numerator = 0.0f32;
        let mut denominator = 0.0f32;
        for i in 0..size {
            numerator += v[i] * wv[i];
            denominator += v[i] * v[i];
        }

        if denominator == 0.0 {
            0.0
        } else {
            (numerator / denominator).abs()
        }
    }
}

#[inline]
fn fast_tanh(x: f32) -> f32 {
    let x2 = x * x;
    x * (27.0 + x2) / (27.0 + 9.0 * x2)
}

/// Chaotic reservoir with configurable dynamics
pub struct ChaoticReservoir {
    base: Reservoir,
    chaos_strength: f32,
    rng: StdRng,
    noisy_input: Vec<f32>,
}
impl ChaoticReservoir {
    pub fn new(input_size: usize, size: usize, chaos_strength: f32) -> Result<Self> {
        let seed = rand::rng().random();
        Self::new_seeded(input_size, size, chaos_strength, seed)
    }
    pub fn new_seeded(
        input_size: usize,
        size: usize,
        chaos_strength: f32,
        seed: u64,
    ) -> Result<Self> {
        Reservoir::validate_params(size, input_size, chaos_strength)?;
        let mut base = Reservoir::new_seeded(input_size, size, seed)?;
        base.set_spectral_radius(1.0)?;
        Ok(Self {
            base,
            chaos_strength,
            rng: StdRng::seed_from_u64(seed ^ 0xA5A5_5A5A_F0F0_0F0F),
            noisy_input: vec![0.0; input_size],
        })
    }
    pub fn step(&mut self, input: &[f32]) -> Result<&[f32]> {
        if input.len() != self.noisy_input.len() {
            return Err(MemoryError::reservoir(format!(
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
            self.noisy_input[i] = *value + noise;
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
