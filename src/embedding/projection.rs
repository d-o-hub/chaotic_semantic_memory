#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
//! Sparse random projection (Achlioptas method) for embedding → HVec10240.
//!
//! Johnson-Lindenstrauss lemma guarantees that random projection preserves
//! pairwise distances with high probability. Achlioptas showed that sparse
//! projections (values in {-1, 0, +1} with p=1/3 for zeros) work just as well
//! as dense Gaussian projections, with ~3x speedup.

// Casts are intentional for dimension math (usize to f32 for sparsity ratio)

use crate::hyperdim::HVec10240;

/// Configuration for projection matrix generation.
#[derive(Debug, Clone)]
pub struct ProjectionConfig {
    /// Seed for reproducible projection matrix.
    pub seed: u64,
    /// Native embedding dimension (input size).
    pub native_dim: usize,
    /// Target dimension (always 10240 for HVec).
    pub target_dim: usize,
    /// Sparsity factor: probability of zero entry.
    /// Default: 0.666... (Achlioptas optimal: 2/3 zeros).
    pub sparsity: f32,
}

impl Default for ProjectionConfig {
    fn default() -> Self {
        Self {
            seed: 42,
            native_dim: 384, // Typical embedding dim (e.g., bge-small)
            target_dim: 10240,
            sparsity: 2.0 / 3.0,
        }
    }
}

/// Sparse random projection matrix for f32[native_dim] → HVec10240.
///
/// Matrix is 10240 × native_dim with values in {-1, 0, +1}.
/// Stored as flat array with indices for non-zero entries.
#[derive(Debug, Clone)]
pub struct Projection {
    /// Non-zero indices and values: (row, col, value).
    /// value ∈ {-1, +1}.
    entries: Vec<(usize, usize, i8)>,
    /// Native dimension (input size).
    native_dim: usize,
}

impl Projection {
    /// Create a new projection matrix with given configuration.
    #[must_use]
    pub fn new(config: &ProjectionConfig) -> Self {
        use rand::RngExt;
        use rand::SeedableRng;
        use rand::rngs::StdRng;

        let mut rng = StdRng::seed_from_u64(config.seed);
        let mut entries = Vec::new();

        // Generate sparse entries: for each (row, col), 1/3 chance of non-zero.
        for row in 0..config.target_dim {
            for col in 0..config.native_dim {
                let r: f32 = rng.random();
                if r < config.sparsity {
                    // Zero entry - skip
                    continue;
                }
                // Non-zero: ±1 with equal probability
                let value: i8 = if rng.random_bool(0.5) { 1 } else { -1 };
                entries.push((row, col, value));
            }
        }

        Self {
            entries,
            native_dim: config.native_dim,
        }
    }

    /// Project a native embedding to HVec10240.
    ///
    /// Computes: output[row] = sign(sum_{col} projection[row,col] * input[col])
    /// Using sparse matrix multiplication.
    pub fn project(&self, vec: &[f32]) -> HVec10240 {
        assert!(vec.len() == self.native_dim, "input dimension mismatch");

        // Accumulate sparse dot products for each row.
        let mut sums = vec![0.0_f32; 10240];

        for &(row, col, value) in &self.entries {
            sums[row] += value as f32 * vec[col];
        }

        // Convert to bipolar HVec: sign of each sum.
        // bit = 1 if sum >= 0, bit = 0 if sum < 0
        let mut hv = HVec10240::zero();
        for (i, &sum) in sums.iter().enumerate() {
            if sum >= 0.0 {
                // Set bit at position i
                let word = i / 128;
                let bit = i % 128;
                hv.data[word] |= 1u128 << bit;
            }
        }

        hv
    }

    /// Get the number of non-zero entries in the projection matrix.
    #[must_use]
    pub fn nnz(&self) -> usize {
        self.entries.len()
    }

    /// Get sparsity ratio (fraction of non-zeros).
    #[must_use]
    pub fn sparsity_ratio(&self) -> f32 {
        let total = 10240 * self.native_dim;
        self.entries.len() as f32 / total as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection_sparsity_is_correct() {
        let config = ProjectionConfig {
            seed: 42,
            native_dim: 384,
            target_dim: 10240,
            sparsity: 2.0 / 3.0,
        };
        let proj = Projection::new(&config);

        // Expected: ~1/3 non-zeros (sparsity is 2/3, so density is 1/3)
        let expected_nnz = 10240 * 384 / 3;
        let actual = proj.nnz();

        // Allow 10% variance due to randomness
        assert!(
            actual > expected_nnz * 9 / 10 && actual < expected_nnz * 11 / 10,
            "nnz {actual} not close to expected {expected_nnz}"
        );
    }

    #[test]
    fn projection_is_deterministic() {
        let config = ProjectionConfig::default();
        let p1 = Projection::new(&config);
        let p2 = Projection::new(&config);

        assert_eq!(p1.entries, p2.entries);
    }

    #[test]
    fn projection_preserves_similarity() {
        use crate::hyperdim::HVec10240;

        let config = ProjectionConfig {
            seed: 42,
            native_dim: 384,
            target_dim: 10240,
            sparsity: 2.0 / 3.0,
        };
        let proj = Projection::new(&config);

        // Create two similar vectors in native space
        let v1 = vec![0.1_f32; 384];
        let mut v2 = vec![0.1_f32; 384];
        // Make them 90% similar
        for i in 0..38 {
            v2[i] = 0.2; // Different in ~10% of dimensions
        }

        let h1 = proj.project(&v1);
        let h2 = proj.project(&v2);

        // Similarity should be preserved (not exact due to binarization)
        let sim = HVec10240::cosine_similarity(&h1, &h2);
        assert!(sim > 0.5, "similarity {sim} too low after projection");
    }
}
