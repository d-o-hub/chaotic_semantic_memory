//! Echo State Network for temporal dynamics

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

use crate::error::{MemoryError, Result};
use crate::hyperdim::HVec10240;

/// Sparse Echo State Network with chaotic dynamics
pub struct Reservoir {
    size: usize,
    input_size: usize,
    state: Vec<f32>,
    scratch: Vec<f32>,
    w_in: Vec<Vec<(usize, f32)>>,
    w_res: Vec<Vec<(usize, f32)>>,
    spectral_radius: f32,
    alpha: f32,
}

impl Reservoir {
    pub const DEFAULT_SIZE: usize = 50000;
    pub const DEFAULT_RADIUS: f32 = 0.95;
    pub const DEFAULT_ALPHA: f32 = 0.3;
    const INPUT_DEGREE: usize = 32;
    const RESERVOIR_DEGREE: usize = 64;

    pub fn new(input_size: usize, size: usize) -> Result<Self> {
        let seed = rand::thread_rng().gen();
        Self::new_seeded(input_size, size, seed)
    }

    pub fn new_seeded(input_size: usize, size: usize, seed: u64) -> Result<Self> {
        if input_size == 0 || size == 0 {
            return Err(MemoryError::Reservoir(
                "Input size and reservoir size must be greater than zero".to_string(),
            ));
        }

        let mut rng = StdRng::seed_from_u64(seed);

        let w_in = Self::build_sparse_weights(size, input_size, Self::INPUT_DEGREE, &mut rng);
        let mut w_res =
            Self::build_sparse_weights(size, size, Self::RESERVOIR_DEGREE.min(size), &mut rng);

        let current_radius = Self::estimate_spectral_radius(&w_res, size);
        if current_radius > 0.0 {
            let scale = Self::DEFAULT_RADIUS / current_radius;
            Self::scale_weights(&mut w_res, scale);
        }

        Ok(Self {
            size,
            input_size,
            state: vec![0.0; size],
            scratch: vec![0.0; size],
            w_in,
            w_res,
            spectral_radius: Self::DEFAULT_RADIUS,
            alpha: Self::DEFAULT_ALPHA,
        })
    }

    /// Single reservoir step
    pub fn step(&mut self, input: &[f32]) -> Result<&[f32]> {
        if input.len() != self.input_size {
            return Err(MemoryError::Reservoir(format!(
                "Input size mismatch: expected {}, got {}",
                self.input_size,
                input.len()
            )));
        }

        let state = &self.state;
        let w_in = &self.w_in;
        let w_res = &self.w_res;
        let alpha = self.alpha;

        #[cfg(not(target_arch = "wasm32"))]
        self.scratch
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, out)| {
                let input_sum = dot_sparse_row(&w_in[i], input);
                let res_sum = dot_sparse_row(&w_res[i], state);
                let activated = (input_sum + res_sum).tanh();
                *out = state[i] * (1.0 - alpha) + activated * alpha;
            });

        #[cfg(target_arch = "wasm32")]
        for i in 0..self.size {
            let input_sum = dot_sparse_row(&self.w_in[i], input);
            let res_sum = dot_sparse_row(&self.w_res[i], &self.state);
            let activated = (input_sum + res_sum).tanh();
            self.scratch[i] = self.state[i] * (1.0 - self.alpha) + activated * self.alpha;
        }

        std::mem::swap(&mut self.state, &mut self.scratch);
        Ok(&self.state)
    }

    /// Run reservoir for multiple steps
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
    pub fn set_spectral_radius(&mut self, radius: f32) -> Result<()> {
        if !(0.9..=1.1).contains(&radius) {
            return Err(MemoryError::Reservoir(
                "Spectral radius must be in [0.9, 1.1]".to_string(),
            ));
        }

        let current = Self::estimate_spectral_radius(&self.w_res, self.size);
        if current > 0.0 {
            let scale = radius / current;
            Self::scale_weights(&mut self.w_res, scale);
            self.spectral_radius = radius;
        }

        Ok(())
    }

    /// Reset reservoir state
    pub fn reset(&mut self) {
        self.state.fill(0.0);
        self.scratch.fill(0.0);
    }

    /// Project state to hypervector
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
                let start = bit_index * chunk_size;
                let end = start + chunk_size;
                let mut sum = 0.0;
                for value in &self.state[start..end] {
                    sum += *value;
                }
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

    fn build_sparse_weights(
        rows: usize,
        cols: usize,
        degree: usize,
        rng: &mut StdRng,
    ) -> Vec<Vec<(usize, f32)>> {
        let mut matrix = Vec::with_capacity(rows);
        for _ in 0..rows {
            let mut row = Vec::with_capacity(degree);
            for _ in 0..degree {
                row.push((rng.gen_range(0..cols), rng.gen_range(-1.0..1.0)));
            }
            matrix.push(row);
        }
        matrix
    }

    fn scale_weights(weights: &mut [Vec<(usize, f32)>], scale: f32) {
        for row in weights {
            for (_, w) in row {
                *w *= scale;
            }
        }
    }

    /// Estimate spectral radius using power iteration
    fn estimate_spectral_radius(w: &[Vec<(usize, f32)>], size: usize) -> f32 {
        let mut v = vec![1.0f32 / size as f32; size];
        let mut y = vec![0.0f32; size];

        for _ in 0..16 {
            for i in 0..size {
                y[i] = dot_sparse_row(&w[i], &v);
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
        for i in 0..size {
            wv[i] = dot_sparse_row(&w[i], &v);
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
fn dot_sparse_row(weights: &[(usize, f32)], values: &[f32]) -> f32 {
    let mut sum = 0.0;
    for (index, weight) in weights {
        sum += *weight * values[*index];
    }
    sum
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
        let seed = rand::thread_rng().gen();
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reservoir_creation() {
        let reservoir = Reservoir::new_seeded(10, 256, 42).unwrap();
        assert_eq!(reservoir.size(), 256);
    }

    #[test]
    fn test_reservoir_step() {
        let mut reservoir = Reservoir::new_seeded(10, 256, 42).unwrap();
        let input = vec![0.5; 10];
        let result = reservoir.step(&input).unwrap();
        assert_eq!(result.len(), 256);
    }

    #[test]
    fn test_spectral_radius_constraint() {
        let mut reservoir = Reservoir::new_seeded(10, 256, 42).unwrap();
        assert!(reservoir.set_spectral_radius(1.05).is_ok());
        assert!(reservoir.set_spectral_radius(1.2).is_err());
    }

    #[test]
    fn test_chaotic_reservoir() {
        let mut reservoir = ChaoticReservoir::new_seeded(10, 256, 0.1, 42).unwrap();
        let input = vec![0.5; 10];
        let result = reservoir.step(&input).unwrap();
        assert_eq!(result.len(), 256);
    }

    #[test]
    fn test_to_hypervector_small_reservoir_errors() {
        let reservoir = Reservoir::new_seeded(10, 256, 42).unwrap();
        assert!(reservoir.to_hypervector().is_err());
    }
}
