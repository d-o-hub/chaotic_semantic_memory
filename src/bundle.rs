//! Concept bundling and superposition operations.

use crate::error::Result;
use crate::hyperdim::{HVec10240, Hypervector};
use serde::{Deserialize, Serialize};

/// Accumulator for bundling multiple hypervectors using majority rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleAccumulator {
    pub(crate) counts: Vec<i32>,
    pub(crate) n: usize,
}

impl Default for BundleAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

impl BundleAccumulator {
    /// Create a new empty accumulator.
    pub fn new() -> Self {
        Self {
            counts: vec![0; HVec10240::DIMENSION],
            n: 0,
        }
    }

    /// Add a hypervector to the accumulator.
    pub fn add(&mut self, hv: &HVec10240) {
        for (i, &val) in hv.data.iter().enumerate() {
            if val >= 0.0 {
                self.counts[i] += 1;
            } else {
                self.counts[i] -= 1;
            }
        }
        self.n += 1;
    }

    /// Finalize the accumulation into a single hypervector using majority rule.
    pub fn finalize(&self) -> Result<HVec10240> {
        let mut data = [0.0f32; 10240];
        for (i, &count) in self.counts.iter().enumerate() {
            data[i] = if count > 0 { 1.0 } else { -1.0 };
        }
        Ok(HVec10240 { data })
    }

    /// Reset the accumulator.
    pub fn reset(&mut self) {
        self.counts.fill(0);
        self.n = 0;
    }

    /// Get the number of vectors accumulated.
    pub const fn count(&self) -> usize {
        self.n
    }
}
