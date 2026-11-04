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
    ///
    /// The dimension is clamped to the owner crate's `MAX_INPUT_DIM`; use
    /// [`Self::try_new`] to reject an out-of-range request instead.
    #[must_use]
    pub fn new(x: f64, y: f64, a: f64, input_dim: usize) -> Self {
        Self {
            inner: RawChaoticLsh::new(x, y, a, input_dim),
        }
    }

    /// Create a new ChaoticLsh, rejecting an out-of-range input dimension.
    ///
    /// # Errors
    ///
    /// Returns the owner crate's [`csm_chaos::hashing::InputDimensionError`]
    /// when `input_dim` is zero or too large.
    pub fn try_new(
        x: f64,
        y: f64,
        a: f64,
        input_dim: usize,
    ) -> Result<Self, csm_chaos::hashing::InputDimensionError> {
        Ok(Self {
            inner: RawChaoticLsh::try_new(x, y, a, input_dim)?,
        })
    }

    /// The input dimension this projector actually uses.
    #[must_use]
    pub const fn input_dim(&self) -> usize {
        self.inner.input_dim()
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
    fn wrapper_reports_the_dimension_the_inner_projector_uses() {
        use csm_chaos::hashing::MAX_INPUT_DIM;

        assert_eq!(ChaoticLsh::new(0.123, 0.456, 0.99, 0).input_dim(), 1);
        assert_eq!(
            ChaoticLsh::new(0.123, 0.456, 0.99, MAX_INPUT_DIM + 1).input_dim(),
            MAX_INPUT_DIM
        );
        assert_eq!(ChaoticLsh::new(0.123, 0.456, 0.99, 8).input_dim(), 8);
        assert_eq!(
            ChaoticLsh::try_new(0.123, 0.456, 0.99, 8)
                .expect("an in-range dimension is accepted")
                .input_dim(),
            8
        );
        assert!(ChaoticLsh::try_new(0.123, 0.456, 0.99, 0).is_err());
        assert!(ChaoticLsh::try_new(0.123, 0.456, 0.99, MAX_INPUT_DIM + 1).is_err());
    }

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
            "Similar vectors should have high similarity: {sim_12}"
        );
        assert!(
            sim_13 < 0.6,
            "Different vectors should have low similarity: {sim_13}"
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
