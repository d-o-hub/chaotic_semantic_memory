//! Incremental bundle accumulator for streaming/sliding-window memory.

use crate::error::{MemoryError, Result};
use crate::hyperdim::HVec10240;

/// Incremental bundle accumulator for streaming/sliding-window memory.
///
/// Maintains signed bit counts for efficient add/remove operations.
/// Finalize applies majority threshold to produce a bundled hypervector.
#[derive(Debug, Clone)]
pub struct BundleAccumulator {
    counts: Box<[i32; HVec10240::DIMENSION]>,
    n: u32,
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
            counts: Box::new([0i32; HVec10240::DIMENSION]),
            n: 0,
        }
    }

    /// Add a hypervector to the accumulator.
    pub fn add(&mut self, hv: &HVec10240) {
        #[allow(clippy::needless_range_loop)]
        for i in 0..80 {
            for j in 0..128 {
                if (hv.data[i] >> j) & 1 == 1 {
                    self.counts[i * 128 + j] += 1;
                }
            }
        }
        self.n += 1;
    }

    /// Remove a hypervector from the accumulator.
    ///
    /// Saturates at zero: removing from an empty accumulator is a no-op.
    /// Use [`try_remove`] if you need to detect underflow.
    pub fn remove(&mut self, hv: &HVec10240) {
        if self.n == 0 {
            return;
        }
        #[allow(clippy::needless_range_loop)]
        for i in 0..80 {
            for j in 0..128 {
                if (hv.data[i] >> j) & 1 == 1 {
                    self.counts[i * 128 + j] -= 1;
                }
            }
        }
        self.n -= 1;
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
        #[allow(clippy::needless_range_loop)]
        for i in 0..80 {
            for j in 0..128 {
                if (hv.data[i] >> j) & 1 == 1 {
                    self.counts[i * 128 + j] -= 1;
                }
            }
        }
        self.n -= 1;
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

        let mut data = [0u128; 80];
        let threshold = 0; // Majority threshold: count > 0

        #[allow(clippy::needless_range_loop)]
        for i in 0..80 {
            for j in 0..128 {
                if self.counts[i * 128 + j] > threshold {
                    data[i] |= 1u128 << j;
                }
            }
        }

        HVec10240 { data }
    }

    /// Get the number of hypervectors in the accumulator.
    pub fn len(&self) -> u32 {
        self.n
    }

    /// Check if the accumulator is empty.
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// Clear the accumulator.
    pub fn clear(&mut self) {
        *self.counts = [0i32; HVec10240::DIMENSION];
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
        // Single vector bundle should be close to the original
        assert!(bundled.cosine_similarity(&v1) > 0.9);
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
