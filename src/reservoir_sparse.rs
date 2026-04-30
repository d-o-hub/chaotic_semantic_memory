//! Compact sparse row storage (CSR-like) for fast row-wise dot products.

use rand::RngExt;
use rand::rngs::StdRng;

/// A single entry in the sparse weight matrix.
///
/// Algorithmic Optimization: Fuses index and weight into a single struct (Array-of-Structures)
/// to improve cache locality during dot product scans. Using `u32` for indices reduces the
/// memory footprint per entry from 12-16 bytes to 8 bytes compared to `usize` + `f32`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WeightEntry {
    pub index: u32,
    pub weight: f32,
}

/// Compact sparse row storage (CSR-like) for fast row-wise dot products.
///
/// Uses a fused `WeightEntry` representation to minimize cache misses and reduce
/// memory bandwidth requirements during high-frequency reservoir updates.
pub(crate) struct SparseWeights {
    row_offsets: Vec<usize>,
    entries: Vec<WeightEntry>,
}

impl SparseWeights {
    pub(crate) fn build(rows: usize, cols: usize, degree: usize, rng: &mut StdRng) -> Self {
        let nnz = rows.saturating_mul(degree);
        let mut row_offsets = Vec::with_capacity(rows + 1);
        let mut entries = Vec::with_capacity(nnz);
        row_offsets.push(0);

        debug_assert!(cols <= u32::MAX as usize, "Column count exceeds u32 range");
        for _ in 0..rows {
            for _ in 0..degree {
                entries.push(WeightEntry {
                    index: rng.random_range(0..cols) as u32,
                    weight: rng.random_range(-1.0..1.0),
                });
            }
            row_offsets.push(entries.len());
        }

        Self {
            row_offsets,
            entries,
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
        let mut entries = Vec::with_capacity(nnz);
        debug_assert!(
            size <= u32::MAX as usize,
            "Reservoir size exceeds u32 range"
        );
        let half = window / 2;
        row_offsets.push(0);

        for row in 0..size {
            for _ in 0..degree {
                let delta = rng.random_range(0..window);
                let idx = (row + size + delta - half) % size;
                entries.push(WeightEntry {
                    index: idx as u32,
                    weight: rng.random_range(-1.0..1.0),
                });
            }
            row_offsets.push(entries.len());
        }

        Self {
            row_offsets,
            entries,
        }
    }

    #[inline(always)]
    pub(crate) fn dot_row(&self, row: usize, values: &[f32]) -> f32 {
        let start = self.row_offsets[row];
        let end = self.row_offsets[row + 1];
        let entries = &self.entries[start..end];
        let mut i = 0;

        // Use multiple accumulators to break the serial dependency chain of mul_add.
        // This allows the CPU to utilize multiple execution ports for ILP.
        let mut sum0 = 0.0;
        let mut sum1 = 0.0;
        let mut sum2 = 0.0;
        let mut sum3 = 0.0;

        while i + 3 < entries.len() {
            sum0 = entries[i]
                .weight
                .mul_add(values[entries[i].index as usize], sum0);
            sum1 = entries[i + 1]
                .weight
                .mul_add(values[entries[i + 1].index as usize], sum1);
            sum2 = entries[i + 2]
                .weight
                .mul_add(values[entries[i + 2].index as usize], sum2);
            sum3 = entries[i + 3]
                .weight
                .mul_add(values[entries[i + 3].index as usize], sum3);
            i += 4;
        }

        let mut sum = (sum0 + sum1) + (sum2 + sum3);
        while i < entries.len() {
            sum = entries[i]
                .weight
                .mul_add(values[entries[i].index as usize], sum);
            i += 1;
        }
        sum
    }

    pub(crate) fn scale(&mut self, scale: f32) {
        for entry in &mut self.entries {
            entry.weight *= scale;
        }
    }
}
