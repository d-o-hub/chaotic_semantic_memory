#![cfg(feature = "chaotic-hashing")]
//! Backward-compatible Chaotic LSH wrapper.

use crate::hyperdim_binary::BHVec10240;
pub use csm_chaos::hashing::chaotic_lsh::ChaoticLsh as RawChaoticLsh;

/// Chaotic LSH projector wrapper for backward compatibility.
pub struct ChaoticLsh {
    inner: RawChaoticLsh,
}

impl ChaoticLsh {
    /// Create a new ChaoticLsh with given seed values and input dimension.
    pub fn new(x: f64, y: f64, a: f64, input_dim: usize) -> Self {
        Self {
            inner: RawChaoticLsh::new(x, y, a, input_dim),
        }
    }

    /// Scalar dot-product projection.
    pub fn project_scalar(&self, input: &[f32]) -> BHVec10240 {
        BHVec10240 {
            bits: self.inner.project_scalar(input),
        }
    }

    /// SIMD-accelerated projection.
    pub fn project_simd(&self, input: &[f32]) -> BHVec10240 {
        BHVec10240 {
            bits: self.inner.project_simd(input),
        }
    }

    /// Project an input vector into a binary hypervector.
    pub fn project(&self, input: &[f32]) -> BHVec10240 {
        BHVec10240 {
            bits: self.inner.project(input),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn test_chaotic_lsh_locality() {
        let input_dim = 4;
        let lsh = ChaoticLsh::new(0.123, 0.456, 0.99, input_dim);

        let v1 = vec![1.0, 0.5, -0.2, 0.8];
        let v2 = vec![1.0, 0.5, -0.2, 0.81];
        let v3 = vec![-1.0, -0.5, 0.2, -0.8];

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

    #[test]
    fn test_simd_scalar_parity() {
        let lsh = ChaoticLsh::new(0.5, 0.7, 0.95, 8);
        let input = vec![0.1, -0.3, 0.5, 0.7, -0.2, 0.4, -0.6, 0.8];
        let scalar = lsh.project_scalar(&input);
        let result = lsh.project(&input);
        assert_eq!(scalar.bits, result.bits);
    }
}
