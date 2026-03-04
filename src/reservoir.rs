//! Echo State Network for temporal dynamics.
//!
//! # Invariants
//! - `input_size > 0`: Input vector dimensionality
//! - `reservoir_size > 0`: Internal node count
//! - `spectral_radius ∈ [0.0, 1.0]`: Stability constraint
//!
//! # Performance
//! - `step()`: O(reservoir_size × input_size)
//! - `to_hypervector()`: O(reservoir_size)

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tracing::instrument;

#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

use crate::error::{MemoryError, Result};
use crate::hyperdim::HVec10240;

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

/// Compact sparse row storage (CSR-like) for fast row-wise dot products.
struct SparseWeights {
    row_offsets: Vec<usize>,
    indices: Vec<usize>,
    weights: Vec<f32>,
}

impl SparseWeights {
    fn build(rows: usize, cols: usize, degree: usize, rng: &mut StdRng) -> Self {
        let nnz = rows.saturating_mul(degree);
        let mut row_offsets = Vec::with_capacity(rows + 1);
        let mut indices = Vec::with_capacity(nnz);
        let mut weights = Vec::with_capacity(nnz);
        row_offsets.push(0);

        for _ in 0..rows {
            for _ in 0..degree {
                indices.push(rng.gen_range(0..cols));
                weights.push(rng.gen_range(-1.0..1.0));
            }
            row_offsets.push(indices.len());
        }

        Self {
            row_offsets,
            indices,
            weights,
        }
    }

    fn build_local_reservoir(size: usize, degree: usize, window: usize, rng: &mut StdRng) -> Self {
        let nnz = size.saturating_mul(degree);
        let mut row_offsets = Vec::with_capacity(size + 1);
        let mut indices = Vec::with_capacity(nnz);
        let mut weights = Vec::with_capacity(nnz);
        let half = window / 2;
        row_offsets.push(0);

        for row in 0..size {
            for _ in 0..degree {
                let delta = rng.gen_range(0..window);
                let idx = (row + size + delta - half) % size;
                indices.push(idx);
                weights.push(rng.gen_range(-1.0..1.0));
            }
            row_offsets.push(indices.len());
        }

        Self {
            row_offsets,
            indices,
            weights,
        }
    }

    #[inline]
    fn dot_row(&self, row: usize, values: &[f32]) -> f32 {
        let start = self.row_offsets[row];
        let end = self.row_offsets[row + 1];
        let mut sum = 0.0;
        let indices = &self.indices;
        let weights = &self.weights;
        for idx in start..end {
            sum = weights[idx].mul_add(values[indices[idx]], sum);
        }
        sum
    }

    fn scale(&mut self, scale: f32) {
        for w in &mut self.weights {
            *w *= scale;
        }
    }
}

/// Sparse Echo State Network with chaotic dynamics
pub struct Reservoir {
    size: usize,
    input_size: usize,
    state: Vec<f32>,
    scratch: Vec<f32>,
    w_in: SparseWeights,
    w_res: SparseWeights,
    input_cache: Vec<f32>,
    input_projection: Vec<f32>,
    input_projection_valid: bool,
    update_stride: usize,
    update_phase: usize,
    spectral_radius: f32,
    alpha: f32,
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

    pub fn new(input_size: usize, size: usize) -> Result<Self> {
        let seed = rand::thread_rng().r#gen();
        Self::new_seeded(input_size, size, seed)
    }

    pub fn new_seeded(input_size: usize, size: usize, seed: u64) -> Result<Self> {
        if input_size == 0 || size == 0 {
            return Err(MemoryError::Reservoir(
                "Input size and reservoir size must be greater than zero".to_string(),
            ));
        }

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
            w_in,
            w_res,
            input_cache: vec![0.0; input_size],
            input_projection: vec![0.0; size],
            input_projection_valid: false,
            update_stride: Self::PARTIAL_UPDATE_STRIDE,
            update_phase: 0,
            spectral_radius: Self::DEFAULT_RADIUS,
            alpha: Self::DEFAULT_ALPHA,
            metrics: ReservoirMetrics::default(),
        })
    }

    /// Single reservoir step
    #[cfg_attr(not(target_arch = "wasm32"), instrument(skip(self)))]
    pub fn step(&mut self, input: &[f32]) -> Result<&[f32]> {
        let started = Instant::now();
        if input.len() != self.input_size {
            return Err(MemoryError::Reservoir(format!(
                "Input size mismatch: expected {}, got {}",
                self.input_size,
                input.len()
            )));
        }

        if !self.input_projection_valid || self.input_cache != input {
            self.input_cache.copy_from_slice(input);
            self.input_projection_valid = true;

            #[cfg(not(target_arch = "wasm32"))]
            {
                let w_in = &self.w_in;
                self.input_projection
                    .par_iter_mut()
                    .enumerate()
                    .for_each(|(i, out)| {
                        *out = w_in.dot_row(i, input);
                    });
            }

            #[cfg(target_arch = "wasm32")]
            for (i, out) in self.input_projection.iter_mut().enumerate() {
                *out = self.w_in.dot_row(i, input);
            }
        }

        let state = &self.state;
        let one_minus_alpha = 1.0 - self.alpha;
        self.scratch.copy_from_slice(state);
        for i in (self.update_phase..self.size).step_by(self.update_stride) {
            let res_sum = self.w_res.dot_row(i, state);
            let activated = fast_tanh(self.input_projection[i] + res_sum);
            self.scratch[i] = state[i] * one_minus_alpha + activated * self.alpha;
        }
        self.update_phase = (self.update_phase + 1) % self.update_stride;

        std::mem::swap(&mut self.state, &mut self.scratch);
        let latency_us = started.elapsed().as_micros() as u64;
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
            return Err(MemoryError::Reservoir(
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
    }

    /// Project state to hypervector (parallel on non-WASM)
    #[cfg_attr(not(target_arch = "wasm32"), instrument(skip(self)))]
    #[cfg(not(target_arch = "wasm32"))]
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
        // SAFETY: We iterate exactly 80 times (0..80), so data always has 80 elements.
        // The map produces exactly 80 items; try_into can only fail if the length differs,
        // which is structurally impossible here — map to MemoryError instead of panicking.
        let data: [u128; 80] = data.try_into().map_err(|_| {
            MemoryError::Reservoir(
                "internal: par_iter produced unexpected element count".to_string(),
            )
        })?;
        Ok(HVec10240 { data })
    }

    #[cfg(target_arch = "wasm32")]
    #[instrument(skip(self))]
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
        let seed = rand::thread_rng().r#gen();
        Self::new_seeded(input_size, size, chaos_strength, seed)
    }

    pub fn new_seeded(
        input_size: usize,
        size: usize,
        chaos_strength: f32,
        seed: u64,
    ) -> Result<Self> {
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
            return Err(MemoryError::Reservoir(format!(
                "Input size mismatch: expected {}, got {}",
                self.noisy_input.len(),
                input.len()
            )));
        }

        for (i, value) in input.iter().enumerate() {
            self.noisy_input[i] = *value
                + self
                    .rng
                    .gen_range(-self.chaos_strength..self.chaos_strength);
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
