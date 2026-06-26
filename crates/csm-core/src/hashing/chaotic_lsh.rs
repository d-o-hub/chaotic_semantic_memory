//! Chaotic Locality-Sensitive Hashing (LSH).
//!
//! Projects continuous vectors into binary hypervectors using 2D-SLHM trajectories.

use crate::hyperdim_binary::BHVec10240;
use crate::maps::hyperchaotic::Slhm2d;

/// Chaotic LSH projector using 2D-SLHM.
pub struct ChaoticLsh {
    projection_matrix: Vec<f32>,
    input_dim: usize,
}

impl ChaoticLsh {
    /// Create a new ChaoticLsh with given seed values and input dimension.
    pub fn new(x: f64, y: f64, a: f64, input_dim: usize) -> Self {
        let mut map = Slhm2d::new(x, y, a);
        let mut projection_matrix = Vec::with_capacity(10240 * input_dim);

        for _ in 0..10240 {
            for _ in 0..input_dim {
                // Chaotic value in [-1, 1]
                projection_matrix.push((map.next_value() * 2.0 - 1.0) as f32);
            }
        }

        Self {
            projection_matrix,
            input_dim,
        }
    }

    /// Project an input vector into a binary hypervector.
    ///
    /// This uses the pre-generated chaotic projection matrix.
    pub fn project(&self, input: &[f32]) -> BHVec10240 {
        let mut bits = [0u64; 160];

        if input.is_empty() || input.len() != self.input_dim {
            return BHVec10240::zero();
        }

        for i in 0..10240 {
            let mut dot_product = 0.0f32;
            let offset = i * self.input_dim;

            for (j, &val) in input.iter().enumerate() {
                dot_product += val * self.projection_matrix[offset + j];
            }

            if dot_product > 0.0 {
                let word_idx = i / 64;
                let bit_idx = i % 64;
                bits[word_idx] |= 1u64 << bit_idx;
            }
        }

        BHVec10240 { bits }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chaotic_lsh_locality() {
        let input_dim = 4;
        let lsh = ChaoticLsh::new(0.123, 0.456, 0.99, input_dim);

        let v1 = vec![1.0, 0.5, -0.2, 0.8];
        let v2 = vec![1.0, 0.5, -0.2, 0.81]; // Slightly different
        let v3 = vec![-1.0, -0.5, 0.2, -0.8]; // Very different

        // All projections use the same pre-generated matrix from a single LSH instance
        let h1 = lsh.project(&v1);
        let h2 = lsh.project(&v2);
        let h3 = lsh.project(&v3);

        let sim_12 = h1.cosine_similarity(&h2);
        let sim_13 = h1.cosine_similarity(&h3);

        assert!(
            sim_12 > 0.8,
            "Similar vectors should have high similarity: {}",
            sim_12
        );
        assert!(
            sim_13 < 0.6,
            "Different vectors should have low similarity: {}",
            sim_13
        );
    }
}
