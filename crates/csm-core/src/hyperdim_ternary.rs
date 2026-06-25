//! Ultra-Quantisation: 1.58-bit ternary encoding for hypervectors.
//! Based on "Ultra-Quantisation: Efficient Embedding Search via 1.58-bit Encodings"
//! (arXiv:2506.00528, Jun 2026).
//!
//! Maps float vectors to {-1, 0, 1} ternary space (1.58 bits/dim), then
//! losslessly maps to paired binary bitmasks for fast bitwise scalar product.

use crate::hyperdim::{HVec10240, Hypervector};

/// Ternary hypervector: each element ∈ {-1, 0, 1}.
///
/// Storage: two bitmasks (`positive`, `negative`) of length `DIMENSION`.
/// Value at position `i`:
/// - `positive[i]=1, negative[i]=0` → `+1`
/// - `positive[i]=0, negative[i]=1` → `-1`
/// - both `0` → `0`
///
/// Maps losslessly to `2×DIMENSION` binary bits for bitwise scalar product.
/// Cost: 2 AND + 1 XOR + popcount ≈ 3 SIMD instructions per 64 bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TernaryHVec {
    /// Bitmask: 1 where value > 0
    pub positive: [u64; HVec10240::DIMENSION / 64],
    /// Bitmask: 1 where value < 0
    pub negative: [u64; HVec10240::DIMENSION / 64],
}

impl TernaryHVec {
    pub const DIMENSION: usize = HVec10240::DIMENSION;
    /// Threshold for ternary quantization: |x| > THRESHOLD → ±1, else 0.
    /// 0.33 gives approximately 1/3 density for each of {-1, 0, +1}.
    const THRESHOLD: f32 = 0.33;

    /// Zero vector.
    #[inline]
    pub fn zero() -> Self {
        Self {
            positive: [0u64; Self::DIMENSION / 64],
            negative: [0u64; Self::DIMENSION / 64],
        }
    }

    /// Quantize a float slice to ternary `{-1, 0, 1}`.
    #[inline]
    pub fn from_f32_slice(values: &[f32]) -> Self {
        let mut result = Self::zero();
        let len = values.len().min(Self::DIMENSION);
        for i in 0..len {
            let word = i / 64;
            let bit = i % 64;
            if values[i] > Self::THRESHOLD {
                result.positive[word] |= 1u64 << bit;
            } else if values[i] < -Self::THRESHOLD {
                result.negative[word] |= 1u64 << bit;
            }
        }
        result
    }

    /// Convert from a binary hypervector (sign-bit quantization).
    #[inline]
    pub fn from_hvec(vec: &HVec10240) -> Self {
        let bytes = vec.to_bytes();
        let mut result = Self::zero();
        for i in 0..Self::DIMENSION {
            let byte_idx = i / 8;
            let bit_idx = i % 8;
            if byte_idx < bytes.len() && (bytes[byte_idx] & (1 << bit_idx)) != 0 {
                result.positive[i / 64] |= 1u64 << (i % 64);
            } else {
                result.negative[i / 64] |= 1u64 << (i % 64);
            }
        }
        result
    }

    /// Ternary scalar product: agreements minus disagreements.
    ///
    /// - `+1/+1` or `-1/-1` → +1
    /// - `+1/-1` → -1
    /// - anything with `0` → 0
    #[inline]
    pub fn ternary_scalar_product(&self, other: &Self) -> i64 {
        let mut sum = 0i64;
        for i in 0..Self::DIMENSION / 64 {
            let agree_pos = self.positive[i] & other.positive[i];
            let agree_neg = self.negative[i] & other.negative[i];
            let disagree =
                (self.positive[i] ^ other.negative[i]) & (self.negative[i] ^ other.positive[i]);
            sum += (agree_pos.count_ones() + agree_neg.count_ones()) as i64;
            sum -= disagree.count_ones() as i64;
        }
        sum
    }

    /// Approximate cosine similarity via ternary scalar product.
    /// Normalized by dimension for values in `[-1, 1]`.
    #[inline]
    pub fn cosine_similarity(&self, other: &Self) -> f32 {
        let dot = self.ternary_scalar_product(other) as f32;
        dot / Self::DIMENSION as f32
    }

    /// Convert to paired binary representation for storage.
    /// Returns `(positive_bits, negative_bits)` as byte vectors.
    pub fn to_binary_pair(&self) -> (Vec<u8>, Vec<u8>) {
        let word_count = Self::DIMENSION / 64;
        let byte_count = word_count * 8;
        let mut pos = vec![0u8; byte_count];
        let mut neg = vec![0u8; byte_count];

        for i in 0..word_count {
            let base = i * 8;
            for b in 0..8 {
                pos[base + b] = (self.positive[i] >> (b * 8)) as u8;
                neg[base + b] = (self.negative[i] >> (b * 8)) as u8;
            }
        }
        (pos, neg)
    }

    /// Number of nonzero (±1) elements.
    #[inline]
    pub fn nonzero_count(&self) -> u32 {
        let mut count = 0u32;
        for i in 0..Self::DIMENSION / 64 {
            count += self.positive[i].count_ones();
            count += self.negative[i].count_ones();
        }
        count
    }

    /// Fraction of nonzero elements (theoretical: ~0.667 for balanced ternary).
    #[inline]
    pub fn density(&self) -> f32 {
        self.nonzero_count() as f32 / Self::DIMENSION as f32
    }
}

/// SIMD-accelerated ternary scalar product for x86_64 AVX2.
///
/// Processes 4 × 64 = 256 bits per iteration using 256-bit integer intrinsics.
#[cfg(target_arch = "x86_64")]
#[inline]
pub fn ternary_scalar_product_avx2(a: &TernaryHVec, b: &TernaryHVec) -> i64 {
    use std::arch::x86_64::*;

    let mut sum = 0i64;
    let word_count = TernaryHVec::DIMENSION / 64;

    unsafe {
        for i in (0..word_count).step_by(4) {
            let a_pos = _mm256_loadu_si256(a.positive[i..].as_ptr() as *const __m256i);
            let a_neg = _mm256_loadu_si256(a.negative[i..].as_ptr() as *const __m256i);
            let b_pos = _mm256_loadu_si256(b.positive[i..].as_ptr() as *const __m256i);
            let b_neg = _mm256_loadu_si256(b.negative[i..].as_ptr() as *const __m256i);

            let agree = _mm256_or_si256(
                _mm256_and_si256(a_pos, b_pos),
                _mm256_and_si256(a_neg, b_neg),
            );
            let disagree = _mm256_and_si256(
                _mm256_xor_si256(a_pos, b_neg),
                _mm256_xor_si256(a_neg, b_pos),
            );

            sum += popcount_256(agree) as i64;
            sum -= popcount_256(disagree) as i64;
        }
    }
    sum
}

/// Population count for a 256-bit integer via AVX2.
#[cfg(target_arch = "x86_64")]
#[inline]
unsafe fn popcount_256(v: std::arch::x86_64::__m256i) -> u64 {
    use std::arch::x86_64::*;
    let lookup = _mm256_setr_epi8(
        0, 1, 1, 2, 1, 2, 2, 3, 1, 2, 2, 3, 2, 3, 3, 4, 0, 1, 1, 2, 1, 2, 2, 3, 1, 2, 2, 3, 2, 3,
        3, 4,
    );
    let low_mask = _mm256_set1_epi8(0x0F);
    let lo = _mm256_and_si256(v, low_mask);
    let hi = _mm256_and_si256(_mm256_srli_epi16(v, 4), low_mask);
    let pop_lo = _mm256_shuffle_epi8(lookup, lo);
    let pop_hi = _mm256_shuffle_epi8(lookup, hi);
    let total = _mm256_add_epi8(pop_lo, pop_hi);

    // Horizontal sum of bytes into u64
    let zero = _mm256_setzero_si256();
    let acc = _mm256_sad_epu8(total, zero);
    let lo64 = _mm_cvtsi128_si64(unsafe { _mm256_extracti128_si256(acc, 0) });
    let hi64 = _mm_cvtsi128_si64(unsafe { _mm256_extracti128_si256(acc, 1) });
    (lo64 + hi64) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_product() {
        let a = TernaryHVec::zero();
        let b = TernaryHVec::zero();
        assert_eq!(a.ternary_scalar_product(&b), 0);
    }

    #[test]
    fn test_self_product_positive() {
        let mut a = TernaryHVec::zero();
        for w in a.positive.iter_mut() {
            *w = u64::MAX;
        }
        let dot = a.ternary_scalar_product(&a);
        assert_eq!(dot, HVec10240::DIMENSION as i64);
    }

    #[test]
    fn test_opposite_vectors() {
        let mut a = TernaryHVec::zero();
        let mut b = TernaryHVec::zero();
        for w in a.positive.iter_mut() {
            *w = u64::MAX;
        }
        for w in b.negative.iter_mut() {
            *w = u64::MAX;
        }
        let dot = a.ternary_scalar_product(&b);
        assert_eq!(dot, -(HVec10240::DIMENSION as i64));
    }

    #[test]
    fn test_density_is_balanced() {
        let values: Vec<f32> = (0..10240).map(|i| (i as f32 - 5120.0) / 5120.0).collect();
        let t = TernaryHVec::from_f32_slice(&values);
        let d = t.density();
        assert!(d > 0.5, "density too low: {d}");
        assert!(d < 0.8, "density too high: {d}");
    }

    #[test]
    fn test_from_hvec_produces_valid_ternary() {
        let hv = HVec10240::random();
        let t = TernaryHVec::from_hvec(&hv);
        // Every bit should be in exactly one of positive/negative
        for i in 0..TernaryHVec::DIMENSION / 64 {
            let p = t.positive[i];
            let n = t.negative[i];
            // No bit should be in both (that would be invalid)
            assert_eq!(p & n, 0, "overlapping bits at word {i}");
            // Every bit should be in exactly one
            assert_eq!(p | n, u64::MAX, "missing bits at word {i}");
        }
    }

    #[test]
    fn test_to_binary_pair_roundtrip() {
        let values: Vec<f32> = (0..10240)
            .map(|i| ((i as f32 - 5120.0) / 5120.0) * 0.5)
            .collect();
        let t = TernaryHVec::from_f32_slice(&values);
        let (pos, neg) = t.to_binary_pair();
        assert_eq!(pos.len(), 1280);
        assert_eq!(neg.len(), 1280);
    }

    #[test]
    fn test_scalar_product_is_commutative() {
        let a = TernaryHVec::from_f32_slice(&vec![0.5; 10240]);
        let b = TernaryHVec::from_f32_slice(&vec![-0.5; 10240]);
        assert_eq!(a.ternary_scalar_product(&b), b.ternary_scalar_product(&a));
    }
}
