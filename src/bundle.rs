//! Incremental bundle accumulator for streaming/sliding-window memory.

use crate::error::{MemoryError, Result};
use crate::hyperdim::HVec10240;

/// Incremental bundle accumulator for streaming/sliding-window memory.
///
/// Maintains signed bit counts for efficient add/remove operations.
/// Finalize applies majority threshold to produce a bundled hypervector.
#[derive(Debug, Clone)]
pub struct BundleAccumulator {
    pub(crate) counts: Box<[i32; HVec10240::DIMENSION]>,
    pub(crate) n: u32,
}

impl Default for BundleAccumulator {
    fn default() -> Self {
        Self {
            counts: Box::new([0i32; HVec10240::DIMENSION]),
            n: 0,
        }
    }
}

impl BundleAccumulator {
    /// Wire-format version. Bump when `serde` layout changes.
    pub const WIRE_VERSION: u32 = 1;

    /// Create a new empty accumulator.
    pub fn new() -> Self {
        Self {
            counts: Box::new([0i32; HVec10240::DIMENSION]),
            n: 0,
        }
    }

    /// Add a hypervector to the accumulator.
    pub fn add(&mut self, hv: &HVec10240) {
        for i in 0..80 {
            let mut val = hv.data[i];
            while val != 0 {
                let j = val.trailing_zeros() as usize;
                self.counts[i * 128 + j] += 1;
                val &= val - 1;
            }
        }
        self.n += 1;
    }

    /// Remove a hypervector from the accumulator.
    ///
    /// Saturates at zero: removing from an empty accumulator is a no-op.
    /// Use [`Self::try_remove`] if you need to detect underflow.
    pub fn remove(&mut self, hv: &HVec10240) {
        if self.n == 0 {
            return;
        }
        for i in 0..80 {
            let mut val = hv.data[i];
            while val != 0 {
                let j = val.trailing_zeros() as usize;
                self.counts[i * 128 + j] -= 1;
                val &= val - 1;
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
        for i in 0..80 {
            let mut val = hv.data[i];
            while val != 0 {
                let j = val.trailing_zeros() as usize;
                self.counts[i * 128 + j] -= 1;
                val &= val - 1;
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

        #[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
        {
            if is_x86_feature_detected!("avx2") {
                // SAFETY: AVX2 feature detected at runtime.
                return unsafe { self.finalize_avx2() };
            }
        }

        self.finalize_scalar()
    }

    /// Scalar implementation of finalize.
    fn finalize_scalar(&self) -> HVec10240 {
        let mut data = [0u128; 80];
        let threshold = 0;

        for (i, word) in data.iter_mut().enumerate() {
            let offset = i * 128;
            for j in 0..128 {
                // Branchless bit construction to reduce misprediction penalties
                let condition = self.counts[offset + j] > threshold;
                *word |= (condition as u128) << j;
            }
        }

        HVec10240 { data }
    }

    /// AVX2-optimized implementation of finalize.
    ///
    /// Processes 8 counters (32-bit i32) at once using SIMD comparisons.
    /// The `_mm256_movemask_ps` trick extracts 8 bits (one per counter) into a u32.
    /// We then pack these u32s into the final u128 words.
    #[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
    #[target_feature(enable = "avx2")]
    unsafe fn finalize_avx2(&self) -> HVec10240 {
        use std::arch::x86_64::{
            __m256i, _mm256_castsi256_ps, _mm256_cmpgt_epi32, _mm256_loadu_si256,
            _mm256_movemask_ps, _mm256_setzero_si256,
        };

        let mut data = [0u128; 80];
        let zero = _mm256_setzero_si256();

        for i in 0..80 {
            let mut word = 0u128;
            let offset = i * 128;

            // 128 bits per word / 8 bits per AVX2 op = 16 iterations
            for k in 0..16 {
                // SAFETY: self.counts is a [i32; 10240]. offset + k*8 max is 79*128 + 15*8 + 7 = 10112 + 120 + 7 = 10239.
                // 10240 is multiple of 8, so we never read out of bounds.
                unsafe {
                    let ptr = self.counts.as_ptr().add(offset + k * 8) as *const __m256i;
                    let chunk = _mm256_loadu_si256(ptr);
                    // Compare counts > 0
                    let mask_vec = _mm256_cmpgt_epi32(chunk, zero);
                    // Extract 8 bits from the comparison mask (1 bit per 32-bit element)
                    let mask = _mm256_movemask_ps(_mm256_castsi256_ps(mask_vec)) as u32;
                    word |= (mask as u128) << (k * 8);
                }
            }
            data[i] = word;
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
