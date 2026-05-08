//! Incremental bundle accumulator for streaming/sliding-window memory.

use crate::error::{MemoryError, Result};
use crate::hyperdim::{HVec10240, Hypervector};

/// Incremental bundle accumulator for streaming/sliding-window memory.
///
/// Maintains signed bit counts for efficient add/remove operations.
/// Finalize applies majority threshold to produce a bundled hypervector.
#[derive(Debug, Clone)]
pub struct BundleAccumulator {
    counts: Vec<i32>,
    n: u32,
}

impl Default for BundleAccumulator {
    fn default() -> Self {
        Self {
            counts: vec![0i32; HVec10240::DIMENSION],
            n: 0,
        }
    }
}

impl BundleAccumulator {
    /// Create a new empty accumulator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a hypervector to the accumulator.
    pub fn add(&mut self, hv: &HVec10240) {
        for (i, &val) in hv.data.iter().enumerate() {
            // Convert f32 to binary: positive -> 1, negative/zero -> 0
            if val >= 0.0 {
                self.counts[i] += 1;
            } else {
                self.counts[i] -= 1;
            }
        }
        self.n = self.n.saturating_add(1);
    }

    /// Remove a hypervector from the accumulator.
    ///
    /// Saturates at zero: removing from an empty accumulator is a no-op.
    /// Use [`Self::try_remove`] if you need to detect underflow.
    pub fn remove(&mut self, hv: &HVec10240) {
        if self.n == 0 {
            return;
        }
        for (i, &val) in hv.data.iter().enumerate() {
            // Convert f32 to binary: positive -> 1, negative/zero -> 0
            if val >= 0.0 {
                self.counts[i] -= 1;
            } else {
                self.counts[i] += 1;
            }
        }
        self.n = self.n.saturating_sub(1);
    }

    /// Remove a hypervector from the accumulator, returning an error if empty.
    ///
    /// Returns `Err(MemoryError::InvalidInput)` when the accumulator is empty.
    pub fn try_remove(&mut self, hv: &HVec10240) -> Result<()> {
        if self.n == 0 {
            return Err(MemoryError::InvalidInput {
                field: "accumulator".to_string(),
                reason: "cannot remove from empty BundleAccumulator".to_string(),
            });
        }
        self.remove(hv);
        Ok(())
    }

    /// Finalize the accumulator into a bundled hypervector.
    ///
    /// Applies majority threshold: bits with count > 0 are set to 1.
    /// Returns zero vector if accumulator is empty.
    pub fn finalize(&self) -> HVec10240 {
        if self.n == 0 {
            return HVec10240::zero();
        }

        let mut data = [0.0f32; HVec10240::DIMENSION];
        for (i, count) in self.counts.iter().enumerate() {
            // Majority threshold: count > 0 means more positives than negatives
            if *count > 0 {
                data[i] = 1.0;
            } else {
                data[i] = -1.0;
            }
        }

        HVec10240 { data }
    }

    /// Get the number of hypervectors in the accumulator.
    pub const fn len(&self) -> u32 {
        self.n
    }

    /// Check if the accumulator is empty.
    pub const fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// Clear the accumulator.
    pub fn clear(&mut self) {
        self.counts.fill(0i32);
        self.n = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundle_accumulator_add_finalize() {
        let v1 = HVec10240::random();
        let v2 = HVec10240::random();
        let v3 = HVec10240::random();

        let mut acc = BundleAccumulator::new();
        acc.add(&v1);
        acc.add(&v2);
        acc.add(&v3);

        let bundled = acc.finalize();
        // Bundle should be valid (not zero)
        assert_ne!(bundled, HVec10240::zero());
        // Should have 3 vectors
        assert_eq!(acc.len(), 3);
    }

    #[test]
    fn test_bundle_accumulator_remove() {
        let v1 = HVec10240::random();
        let v2 = HVec10240::random();

        let mut acc = BundleAccumulator::new();
        acc.add(&v1);
        acc.add(&v2);
        acc.remove(&v2);

        assert_eq!(acc.len(), 1);
        let bundled = acc.finalize();
        // Single vector bundle captures sign pattern, yielding high cosine similarity
        // For random vectors uniformly in [-1,1), expected cosine ≈ 0.866
        assert!(bundled.cosine_similarity(&v1) > 0.8);
    }

    #[test]
    fn test_bundle_accumulator_empty() {
        let acc = BundleAccumulator::new();
        assert!(acc.is_empty());
        assert_eq!(acc.finalize(), HVec10240::zero());
    }

    #[test]
    fn test_bundle_accumulator_clear() {
        let mut acc = BundleAccumulator::new();
        acc.add(&HVec10240::random());
        acc.clear();
        assert!(acc.is_empty());
    }
}
