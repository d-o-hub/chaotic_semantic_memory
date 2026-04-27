//! Compact sparse row storage (CSR-like) for fast row-wise dot products.

use rand::RngExt;
use rand::rngs::StdRng;

/// Compact sparse row storage (CSR-like) for fast row-wise dot products.
pub(crate) struct SparseWeights {
    row_offsets: Vec<usize>,
    indices: Vec<usize>,
    weights: Vec<f32>,
}

impl SparseWeights {
    pub(crate) fn build(rows: usize, cols: usize, degree: usize, rng: &mut StdRng) -> Self {
        let nnz = rows.saturating_mul(degree);
        let mut row_offsets = Vec::with_capacity(rows + 1);
        let mut indices = Vec::with_capacity(nnz);
        let mut weights = Vec::with_capacity(nnz);
        row_offsets.push(0);

        for _ in 0..rows {
            for _ in 0..degree {
                indices.push(rng.random_range(0..cols));
                weights.push(rng.random_range(-1.0..1.0));
            }
            row_offsets.push(indices.len());
        }

        Self {
            row_offsets,
            indices,
            weights,
        }
    }

    pub(crate) fn build_local_reservoir(
        size: usize,
        degree: usize,
        window: usize,
        rng: &mut StdRng,
    ) -> Self {
        let nnz = size.saturating_mul(degree);
        let mut row_offsets = Vec::with_capacity(size + 1);
        let mut indices = Vec::with_capacity(nnz);
        let mut weights = Vec::with_capacity(nnz);
        let half = window / 2;
        row_offsets.push(0);

        for row in 0..size {
            for _ in 0..degree {
                let delta = rng.random_range(0..window);
                let idx = (row + size + delta - half) % size;
                indices.push(idx);
                weights.push(rng.random_range(-1.0..1.0));
            }
            row_offsets.push(indices.len());
        }

        Self {
            row_offsets,
            indices,
            weights,
        }
    }

    #[inline(always)]
    pub(crate) fn dot_row(&self, row: usize, values: &[f32]) -> f32 {
        let start = self.row_offsets[row];
        let end = self.row_offsets[row + 1];
        let indices = &self.indices[start..end];
        let weights = &self.weights[start..end];
        let mut i = 0;

        // Use multiple accumulators to break the serial dependency chain of mul_add.
        // This allows the CPU to utilize multiple execution ports for ILP.
        let mut sum0 = 0.0;
        let mut sum1 = 0.0;
        let mut sum2 = 0.0;
        let mut sum3 = 0.0;

        while i + 3 < indices.len() {
            sum0 = weights[i].mul_add(values[indices[i]], sum0);
            sum1 = weights[i + 1].mul_add(values[indices[i + 1]], sum1);
            sum2 = weights[i + 2].mul_add(values[indices[i + 2]], sum2);
            sum3 = weights[i + 3].mul_add(values[indices[i + 3]], sum3);
            i += 4;
        }

        let mut sum = (sum0 + sum1) + (sum2 + sum3);
        while i < indices.len() {
            sum = weights[i].mul_add(values[indices[i]], sum);
            i += 1;
        }
        sum
    }

    pub(crate) fn scale(&mut self, scale: f32) {
        for w in &mut self.weights {
            *w *= scale;
        }
    }
}
