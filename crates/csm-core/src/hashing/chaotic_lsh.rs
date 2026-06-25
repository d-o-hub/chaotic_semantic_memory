//! Chaotic Locality-Sensitive Hashing (LSH).
//!
//! Projects continuous vectors into binary hypervectors using 2D-SLHM trajectories.

use crate::error::Result;
use crate::hyperdim_binary::BHVec10240;
use crate::maps::hyperchaotic::Slhm2d;

/// Chaotic LSH projector using 2D-SLHM.
pub struct ChaoticLsh {
    map: Slhm2d,
}

impl ChaoticLsh {
    /// Create a new ChaoticLsh with given seed values.
    pub fn new(x: f64, y: f64, a: f64) -> Self {
        Self {
            map: Slhm2d::new(x, y, a),
        }
    }

    /// Project an input vector into a binary hypervector.
    ///
    /// This uses the chaotic trajectory to generate projection planes (bit-slicing).
    /// Each bit in the BHVec10240 is determined by the sign of the dot product
    /// between the input and a chaotic projection vector.
    pub fn project(&mut self, input: &[f32]) -> BHVec10240 {
        let mut bits = [0u64; 160];

        if input.is_empty() {
            return BHVec10240::zero();
        }

        // Pre-generate projection weights to allow for future SIMD optimization
        // and reduce the number of map iterations in the hot loop.
        for i in 0..10240 {
            let mut dot_product = 0.0f64;

            // Generate a projection vector of the same length as input using the map
            // Optimization: Unroll or use SIMD for the dot product if input is large
            for &val in input {
                let projection_weight = self.map.next_value() * 2.0 - 1.0;
                dot_product += (val as f64) * projection_weight;
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
        let mut lsh = ChaoticLsh::new(0.123, 0.456, 3.99);

        let v1 = vec![1.0, 0.5, -0.2, 0.8];
        let v2 = vec![1.0, 0.5, -0.2, 0.81]; // Slightly different
        let v3 = vec![-1.0, -0.5, 0.2, -0.8]; // Very different

        let h1 = lsh.project(&v1);

        let mut lsh2 = ChaoticLsh::new(0.123, 0.456, 3.99);
        let h2 = lsh2.project(&v2);

        let mut lsh3 = ChaoticLsh::new(0.123, 0.456, 3.99);
        let h3 = lsh3.project(&v3);

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
