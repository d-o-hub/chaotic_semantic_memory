use crate::hashing::{InputDimensionError, clamped_input_dim, validate_input_dim};
use crate::maps::hyperchaotic::Slhm2d;
use alloc::vec::Vec;
use core::cmp::Ordering;

/// Locality-Sensitive Hashing based on sparse random projections and winner-take-all,
/// inspired by the Drosophila olfactory circuit (FlyHash).
///
/// See: "FlyHash for LLM Agent Memory: An Empirical Validation and a New Application"
/// (Mohan Bheemadolu, Chrisogonas O. Odhiambo, 2026).
#[derive(Debug, Clone, PartialEq)]
pub struct FlyHash {
    /// Dimension of the input vectors (e.g., 1536).
    pub input_dim: usize,
    /// The proportion of non-zero entries in the projection matrix (0.0 to 1.0).
    pub sparsity: f32,
    /// The number of winning neurons to activate (e.g., 200 to 500 for a 10240 representation).
    pub top_k: usize,
    /// The flattened sparse projection matrix of size `10240 * input_dim`.
    /// Stored column-major or row-major depending on dot product needs; we use row-major.
    pub projection_matrix: Vec<f32>,
}

impl FlyHash {
    /// Infallible constructor that clamps `input_dim` to `1..=MAX_INPUT_DIM`.
    /// `sparsity` specifies the probability of an entry being non-zero (0.0 < sparsity <= 1.0).
    ///
    /// # Panics
    /// Panics if `try_new` fails after dimension clamping, which indicates a logical error in clamping.
    #[must_use]
    #[allow(clippy::expect_used)]
    pub fn new(x0: f64, y0: f64, sparsity: f32, top_k: usize, input_dim: usize) -> Self {
        let input_dim = clamped_input_dim(input_dim);
        Self::try_new(x0, y0, sparsity, top_k, input_dim).expect("clamped dim must be valid")
    }

    /// Failable constructor returning `InputDimensionError` if `input_dim` is 0 or > `MAX_INPUT_DIM`.
    /// Validates `sparsity` is between 0 and 1, and `top_k` <= 10240.
    pub fn try_new(
        x0: f64,
        y0: f64,
        sparsity: f32,
        top_k: usize,
        input_dim: usize,
    ) -> Result<Self, InputDimensionError> {
        let input_dim = validate_input_dim(input_dim)?;
        let top_k = top_k.clamp(1, 10240);
        let sparsity = sparsity.clamp(0.0, 1.0);
        #[allow(clippy::cast_possible_truncation)]
        let sparsity_f64 = f64::from(sparsity);

        // Generate the projection matrix deterministically using Slhm2d map
        let mut map = Slhm2d::new(x0, y0, 0.99); // r=0.99 standard hyperchaotic regime
        let num_elements = 10240 * input_dim;
        let mut projection_matrix = Vec::with_capacity(num_elements);

        for _ in 0..num_elements {
            // next_value() usually returns in roughly [-1.0, 1.0] for SLHM2D
            // we will use the map to generate both the sparsity mask and the weight
            let v1 = map.next_value();
            let v2 = map.next_value();

            // Transform v1 into a uniform [0, 1] probability
            let prob = (v1 + 1.0) / 2.0;

            if prob <= sparsity_f64 {
                #[allow(clippy::cast_possible_truncation)]
                projection_matrix.push(v2 as f32);
            } else {
                projection_matrix.push(0.0);
            }
        }

        Ok(Self {
            input_dim,
            sparsity,
            top_k,
            projection_matrix,
        })
    }

    /// Project a dense vector of size `input_dim` into a 10240-bit FlyHash code.
    ///
    /// # Panics
    /// Panics if `input.len() != self.input_dim`.
    pub fn project(&self, input: &[f32]) -> [u64; 160] {
        assert_eq!(
            input.len(),
            self.input_dim,
            "Input slice length must exactly match the configured input_dim."
        );

        let len = self.input_dim;
        let mut activations = [0.0f32; 10240];

        // Compute dense-sparse matrix vector product
        for i in 0..10240 {
            let offset = i * len;
            let mut dot = 0.0;
            for j in 0..len {
                let weight = self.projection_matrix[offset + j];
                if weight != 0.0 {
                    dot += input[j] * weight;
                }
            }
            activations[i] = dot;
        }

        // Winner-take-all: Select top-k activations
        let mut indices: Vec<usize> = (0..10240).collect();

        // Sort indices by activation value descending
        indices.sort_unstable_by(|&a, &b| {
            let va = activations[a];
            let vb = activations[b];
            vb.partial_cmp(&va).unwrap_or(Ordering::Equal)
        });

        let mut bits = [0u64; 160];
        // Set the top_k bits
        for idx in indices.iter().take(self.top_k) {
            let word_idx = idx / 64;
            let bit_idx = idx % 64;
            bits[word_idx] |= 1 << bit_idx;
        }

        bits
    }
}

#[cfg(test)]
#[path = "fly_hash_tests.rs"]
mod tests;
