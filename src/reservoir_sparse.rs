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

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn make_test_rng() -> StdRng {
        StdRng::from_seed([42u8; 32])
    }

    #[test]
    fn sparse_weights_build_structure() {
        let mut rng = make_test_rng();
        let weights = SparseWeights::build(10, 100, 4, &mut rng);

        // Verify row_offsets length (rows + 1)
        assert_eq!(weights.row_offsets.len(), 11);

        // Verify total non-zeros (rows * degree)
        assert_eq!(weights.indices.len(), 40);
        assert_eq!(weights.weights.len(), 40);

        // Verify each row has exactly degree entries
        for row in 0..10 {
            let start = weights.row_offsets[row];
            let end = weights.row_offsets[row + 1];
            assert_eq!(end - start, 4);
        }
    }

    #[test]
    fn sparse_weights_build_local_reservoir_wraparound() {
        let mut rng = make_test_rng();
        let size = 100;
        let weights = SparseWeights::build_local_reservoir(size, 4, 10, &mut rng);

        // Verify all indices are within bounds (0..size)
        for idx in &weights.indices {
            assert!(*idx < size);
        }

        // Verify row_offsets length
        assert_eq!(weights.row_offsets.len(), size + 1);

        // Verify total non-zeros
        assert_eq!(weights.indices.len(), size * 4);
    }

    #[test]
    fn dot_row_with_uniform_values() {
        let mut rng = make_test_rng();
        let weights = SparseWeights::build(5, 10, 3, &mut rng);

        // Create uniform input values
        let values = [1.0_f32; 10];

        // Compute dot product for row 0
        let result = weights.dot_row(0, &values);

        // Result should be sum of weights for row 0
        let start = weights.row_offsets[0];
        let end = weights.row_offsets[1];
        let expected: f32 = weights.weights[start..end].iter().sum();

        assert!((result - expected).abs() < 1e-5);
    }

    #[test]
    fn dot_row_with_zero_values() {
        let mut rng = make_test_rng();
        let weights = SparseWeights::build(5, 10, 3, &mut rng);

        // All zero input
        let values = [0.0_f32; 10];

        // Any dot product with zeros should be zero
        for row in 0..5 {
            let result = weights.dot_row(row, &values);
            assert_eq!(result, 0.0);
        }
    }

    #[test]
    fn dot_row_single_element() {
        // Manually construct sparse weights with single element per row
        let sparse = SparseWeights {
            row_offsets: vec![0, 1, 2, 3],
            indices: vec![0, 1, 2],
            weights: vec![0.5, 1.0, 2.0],
        };

        let values = [10.0, 20.0, 30.0, 40.0];

        // Row 0: weight 0.5 at index 0, value 10.0 → 5.0
        assert_eq!(sparse.dot_row(0, &values), 5.0);

        // Row 1: weight 1.0 at index 1, value 20.0 → 20.0
        assert_eq!(sparse.dot_row(1, &values), 20.0);

        // Row 2: weight 2.0 at index 2, value 30.0 → 60.0
        assert_eq!(sparse.dot_row(2, &values), 60.0);
    }

    #[test]
    fn dot_row_empty_row() {
        // Manually construct sparse weights with an empty row
        let sparse = SparseWeights {
            row_offsets: vec![0, 2, 2, 4], // Row 1 has 0 elements (offset 2..2)
            indices: vec![0, 1, 2, 3],
            weights: vec![1.0, 2.0, 3.0, 4.0],
        };

        let values = [10.0, 20.0, 30.0, 40.0];

        // Row 1 is empty → dot product should be 0
        assert_eq!(sparse.dot_row(1, &values), 0.0);

        // Row 0: (1.0 * 10.0) + (2.0 * 20.0) = 50.0
        assert_eq!(sparse.dot_row(0, &values), 50.0);

        // Row 2: (3.0 * 30.0) + (4.0 * 40.0) = 250.0
        assert_eq!(sparse.dot_row(2, &values), 250.0);
    }

    #[test]
    fn scale_multiplies_all_weights() {
        let mut rng = make_test_rng();
        let mut weights = SparseWeights::build(5, 10, 3, &mut rng);

        // Record original weights
        let original: Vec<f32> = weights.weights.clone();

        // Scale by 0.5
        weights.scale(0.5);

        // Verify all weights are halved
        for (i, w) in weights.weights.iter().enumerate() {
            assert!((w - original[i] * 0.5).abs() < 1e-5);
        }
    }

    #[test]
    fn scale_by_zero() {
        let mut rng = make_test_rng();
        let mut weights = SparseWeights::build(5, 10, 3, &mut rng);

        // Scale by zero
        weights.scale(0.0);

        // All weights should be zero
        for w in &weights.weights {
            assert_eq!(*w, 0.0);
        }
    }

    #[test]
    fn dot_row_with_negative_weights() {
        let sparse = SparseWeights {
            row_offsets: vec![0, 3],
            indices: vec![0, 1, 2],
            weights: vec![-1.0, 2.0, -3.0],
        };

        let values = [10.0, 20.0, 30.0];

        // Row 0: (-1.0 * 10.0) + (2.0 * 20.0) + (-3.0 * 30.0) = -10 + 40 - 90 = -60
        assert_eq!(sparse.dot_row(0, &values), -60.0);
    }

    #[test]
    fn dot_row_with_negative_values() {
        let sparse = SparseWeights {
            row_offsets: vec![0, 2],
            indices: vec![0, 1],
            weights: vec![1.0, -1.0],
        };

        let values = [-10.0, -20.0];

        // Row 0: (1.0 * -10.0) + (-1.0 * -20.0) = -10 + 20 = 10
        assert_eq!(sparse.dot_row(0, &values), 10.0);
    }
}
