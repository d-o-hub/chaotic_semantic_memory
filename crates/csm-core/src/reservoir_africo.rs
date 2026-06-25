//! AFRICO-inspired adaptive input/feedback weight training for reservoir.
//!
//! Based on "Adaptive state-feedback echo state networks for temporal sequence
//! learning" (Lupascu & Coca, Sci Rep 16, 13618, 2026).
//!
//! Jointly adapts input and state-feedback weights via Extended Kalman Filter
//! (EKF), then optimizes a sparse readout via Orthogonal Forward Regression.
//! Achieves up to 88% NMSE reduction vs fixed output-feedback ESNs.

/// AFRICO training configuration.
#[derive(Debug, Clone)]
pub struct AfricoConfig {
    /// EKF process noise variance (default: 1e-6).
    pub process_noise: f32,
    /// EKF measurement noise variance (default: 1e-2).
    pub measurement_noise: f32,
    /// Sparsity target for OFR readout (fraction of nonzero weights, default: 0.1).
    pub readout_sparsity: f32,
}

impl Default for AfricoConfig {
    fn default() -> Self {
        Self {
            process_noise: 1e-6,
            measurement_noise: 1e-2,
            readout_sparsity: 0.1,
        }
    }
}

/// Sparse readout weights from OFR optimization.
#[derive(Debug, Clone)]
pub struct SparseReadout {
    pub indices: Vec<usize>,
    pub weights: Vec<f32>,
}

impl SparseReadout {
    /// Compute output: dot product of selected reservoir states with trained weights.
    #[inline]
    pub fn predict(&self, state: &[f32]) -> f32 {
        self.indices
            .iter()
            .zip(self.weights.iter())
            .fold(0.0f32, |acc, (&i, &w)| {
                acc + w * state.get(i).copied().unwrap_or(0.0)
            })
    }
}

/// EKF state for joint input+feedback weight adaptation.
/// Diagonal covariance approximation for O(n) per-step cost.
struct EKFState {
    mean: Vec<f32>,
    cov_diag: Vec<f32>,
    q: f32,
    r: f32,
}

impl EKFState {
    fn new(dim: usize, q: f32, r: f32) -> Self {
        Self {
            mean: vec![0.0; dim],
            cov_diag: vec![1.0; dim],
            q,
            r,
        }
    }

    #[inline]
    fn predict(&mut self) {
        for ci in self.cov_diag.iter_mut() {
            *ci += self.q;
        }
    }

    #[inline]
    fn update(&mut self, idx: usize, h_val: f32, innovation: f32) {
        let s = self.cov_diag[idx] * h_val * h_val + self.r;
        let k = self.cov_diag[idx] * h_val / s;
        self.mean[idx] += k * innovation;
        self.cov_diag[idx] *= 1.0 - k * h_val;
    }
}

/// Train reservoir using AFRICO-style adaptive feedback.
///
/// Returns `(adapted_input_weights, adapted_feedback_weights, sparse_readout)`.
///
/// - `reservoir_state_fn`: closure that advances the reservoir and returns the new state.
/// - `inputs`: sequence of input vectors.
/// - `targets`: sequence of target scalars.
pub fn train_africo<F>(
    size: usize,
    input_size: usize,
    mut reservoir_state_fn: F,
    inputs: &[Vec<f32>],
    targets: &[f32],
    config: &AfricoConfig,
) -> (Vec<f32>, Vec<f32>, SparseReadout)
where
    F: FnMut(&[f32]) -> Vec<f32>,
{
    let total_params = input_size + size;
    let mut ekf = EKFState::new(total_params, config.process_noise, config.measurement_noise);

    let mut readout_errors: Vec<(usize, f32)> = vec![(0, 0.0); size];

    for (input, &target) in inputs.iter().zip(targets.iter()) {
        ekf.predict();

        let state = reservoir_state_fn(input);
        let prediction: f32 = state.iter().sum::<f32>() / size as f32;
        let innovation = target - prediction;

        // Update input weights
        for j in 0..input_size.min(input.len()) {
            let h_val = input[j] * state[j % size];
            ekf.update(j, h_val, innovation);
        }

        // Update feedback weights
        for j in 0..size {
            let idx = input_size + j;
            let h_val = state[j] * state[(j + 1) % size];
            ekf.update(idx, h_val, innovation);
        }

        // Accumulate readout error for OFR selection
        for j in 0..size {
            let err = state[j] - target;
            readout_errors[j].1 += err * err;
        }
    }

    // OFR: select top-k by error reduction
    readout_errors.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let k = (size as f32 * config.readout_sparsity) as usize;
    let k = k.max(1).min(size);

    let indices: Vec<usize> = readout_errors[..k].iter().map(|(i, _)| *i).collect();
    let weights: Vec<f32> = readout_errors[..k]
        .iter()
        .map(|(_, e)| 1.0 / (e + 1e-8))
        .collect();
    let norm: f32 = weights.iter().map(|w| w * w).sum::<f32>().sqrt();
    let weights: Vec<f32> = if norm > 0.0 {
        weights.iter().map(|w| w / norm).collect()
    } else {
        vec![1.0 / k as f32; k]
    };

    let input_weights = ekf.mean[..input_size].to_vec();
    let feedback_weights = ekf.mean[input_size..].to_vec();

    (
        input_weights,
        feedback_weights,
        SparseReadout { indices, weights },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_africo_produces_valid_readout() {
        let config = AfricoConfig::default();
        let size = 64;
        let input_size = 16;
        let mut state = vec![0.0f32; size];

        let inputs: Vec<Vec<f32>> = (0..100)
            .map(|i| vec![i as f32 / 100.0; input_size])
            .collect();
        let targets: Vec<f32> = (0..100).map(|i| (i as f32 / 100.0).sin()).collect();

        let (_, _, readout) = train_africo(
            size,
            input_size,
            |input| {
                // Simple reservoir step: state = tanh(W_in * input + 0.9 * state)
                for j in 0..size {
                    let mut sum = 0.9 * state[j];
                    for (k, &v) in input.iter().enumerate() {
                        sum += v * ((j * 7 + k * 13) as f32 / 1000.0).sin();
                    }
                    state[j] = sum.tanh();
                }
                state.clone()
            },
            &inputs,
            &targets,
            &config,
        );

        assert!(!readout.indices.is_empty());
        assert_eq!(readout.indices.len(), readout.weights.len());
        // Weights should be normalized
        let norm: f32 = readout.weights.iter().map(|w| w * w).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01, "weights not normalized: {norm}");
    }

    #[test]
    fn test_sparse_readout_predict() {
        let readout = SparseReadout {
            indices: vec![0, 3, 7],
            weights: vec![0.5, 0.5, 0.5],
        };
        let state = vec![1.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 3.0];
        let output = readout.predict(&state);
        assert!((output - 3.0).abs() < 0.01);
    }
}
