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
    fn wrapper_projection_matches_the_owner_projector() {
        // The wrapper's whole job is delegation, so assert the bits are the
        // owner's instead of re-deriving the owner's locality thresholds here.
        // The locality property itself belongs to `csm-chaos`
        // (`test_chaotic_lsh_locality`), which owns the projection math.
        let owner = RawChaoticLsh::new(0.5, 0.7, 0.95, 4);
        let wrapper = ChaoticLsh::new(0.5, 0.7, 0.95, 4);

        for input in [[1.0, 0.5, -0.2, 0.8], [-1.0, -0.5, 0.2, -0.8]] {
            assert_eq!(wrapper.project(&input).bits, owner.project(&input));
            assert_eq!(
                wrapper.project_scalar(&input).bits,
                owner.project_scalar(&input)
            );
            assert_eq!(
                wrapper.project_simd(&input).bits,
                owner.project_simd(&input)
            );
        }
    }

    #[test]
    fn wrapper_simd_and_scalar_paths_agree() {
        let lsh = ChaoticLsh::new(0.5, 0.7, 0.95, 8);
        let input = vec![0.1, -0.3, 0.5, 0.7, -0.2, 0.4, -0.6, 0.8];
        let scalar = lsh.project_scalar(&input);
        let result = lsh.project(&input);
        assert_eq!(scalar.bits, result.bits);
    }
}
