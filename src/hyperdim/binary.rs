//! 10240-bit binary hypervector implementation (ADR-0075)

#![cfg(feature = "hv-binary")]

use super::Hypervector;
use crate::error::{MemoryError, Result};
use crate::hyperdim::hvec::HVec10240;

/// 10240-bit binary hypervector (160 x 64-bit words)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BHVec10240 {
    pub(crate) bits: [u64; 160],
}

impl BHVec10240 {
    /// Create a binary hypervector from an f32 hypervector (quantization).
    /// Bits are set to 1 if the corresponding float is >= 0, and 0 otherwise.
    pub fn from_f32(v: &HVec10240) -> Self {
        let mut bits = [0u64; 160];
        for (i, &val) in v.data.iter().enumerate() {
            if val >= 0.0 {
                let word_idx = i / 64;
                let bit_idx = i % 64;
                bits[word_idx] |= 1u64 << bit_idx;
            }
        }
        Self { bits }
    }

    /// Convert to an f32 hypervector (expansion).
    /// Bipolar expansion: bit 1 -> 1.0, bit 0 -> -1.0.
    pub fn to_f32(&self) -> HVec10240 {
        let mut data = [0.0f32; 10240];
        for i in 0..10240 {
            let word_idx = i / 64;
            let bit_idx = i % 64;
            if (self.bits[word_idx] & (1u64 << bit_idx)) != 0 {
                data[i] = 1.0;
            } else {
                data[i] = -1.0;
            }
        }
        HVec10240 { data }
    }
}

impl serde::Serialize for BHVec10240 {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeTuple;
        let mut seq = serializer.serialize_tuple(160)?;
        for bit in &self.bits {
            seq.serialize_element(bit)?;
        }
        seq.end()
    }
}

impl<'de> serde::Deserialize<'de> for BHVec10240 {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct BitsVisitor;
        impl<'de> serde::de::Visitor<'de> for BitsVisitor {
            type Value = [u64; 160];
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a sequence of 160 u64s")
            }
            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut bits = [0u64; 160];
                for bit in &mut bits {
                    *bit = seq
                        .next_element()?
                        .ok_or_else(|| serde::de::Error::custom("insufficient elements"))?;
                }
                Ok(bits)
            }
        }
        let bits = deserializer.deserialize_tuple(160, BitsVisitor)?;
        Ok(BHVec10240 { bits })
    }
}

impl Hypervector for BHVec10240 {
    const DIMENSION: usize = 10240;

    fn format_name() -> &'static str {
        "binary"
    }

    fn zero() -> Self {
        Self { bits: [0u64; 160] }
    }

    fn random() -> Self {
        use rand::RngExt;
        let mut rng = rand::rng();
        let mut bits = [0u64; 160];
        rng.fill(&mut bits);
        Self { bits }
    }

    fn bind(&self, other: &Self) -> Self {
        let mut bits = [0u64; 160];
        for i in 0..160 {
            bits[i] = self.bits[i] ^ other.bits[i];
        }
        Self { bits }
    }

    fn bundle(vectors: &[Self]) -> Result<Self> {
        if vectors.is_empty() {
            return Ok(Self::zero());
        }
        if vectors.len() == 1 {
            return Ok(vectors[0]);
        }

        // Majority rule bundling for binary vectors
        // Optimization: Use a smaller type for counts if possible, but i16 might overflow
        // if we bundle > 32k vectors. i32 is safe.
        let mut counts = Box::new([0i32; 10240]);

        #[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
        {
            use rayon::prelude::*;
            if vectors.len() > 100 {
                // Parallel count accumulation
                let partial_counts: Vec<Box<[i32; 10240]>> = vectors
                    .par_chunks(vectors.len() / rayon::current_num_threads().max(1))
                    .map(|chunk| {
                        let mut local_counts = Box::new([0i32; 10240]);
                        for v in chunk {
                            for i in 0..160 {
                                let word = v.bits[i];
                                let offset = i * 64;
                                for j in 0..64 {
                                    if (word & (1u64 << j)) != 0 {
                                        local_counts[offset + j] += 1;
                                    } else {
                                        local_counts[offset + j] -= 1;
                                    }
                                }
                            }
                        }
                        local_counts
                    })
                    .collect();

                for pc in partial_counts {
                    for i in 0..10240 {
                        counts[i] += pc[i];
                    }
                }
            } else {
                for v in vectors {
                    for i in 0..160 {
                        let word = v.bits[i];
                        let offset = i * 64;
                        for j in 0..64 {
                            if (word & (1u64 << j)) != 0 {
                                counts[offset + j] += 1;
                            } else {
                                counts[offset + j] -= 1;
                            }
                        }
                    }
                }
            }
        }

        #[cfg(any(target_arch = "wasm32", not(feature = "parallel")))]
        {
            for v in vectors {
                for i in 0..160 {
                    let word = v.bits[i];
                    let offset = i * 64;
                    for j in 0..64 {
                        if (word & (1u64 << j)) != 0 {
                            counts[offset + j] += 1;
                        } else {
                            counts[offset + j] -= 1;
                        }
                    }
                }
            }
        }

        let mut bits = [0u64; 160];
        for i in 0..10240 {
            if counts[i] > 0 {
                bits[i / 64] |= 1u64 << (i % 64);
            } else if counts[i] == 0 {
                // Tie-breaking: use deterministic pseudo-random or just 0
                // For simplicity and determinism, we'll use 0.
            }
        }
        Ok(Self { bits })
    }

    fn permute(&self, shift: usize) -> Self {
        let mut result = [0u64; 160];
        let bit_shift = shift % 64;
        let word_shift = (shift / 64) % 160;

        if bit_shift == 0 {
            for i in 0..160 {
                result[i] = self.bits[(i + 160 - word_shift) % 160];
            }
            return Self { bits: result };
        }

        let inv_bit_shift = 64 - bit_shift;
        for i in 0..160 {
            let src_idx = (i + 160 - word_shift) % 160;
            let src_idx_next = (src_idx + 159) % 160;

            let val = self.bits[src_idx];
            let val_next = self.bits[src_idx_next];

            result[i] = (val << bit_shift) | (val_next >> inv_bit_shift);
        }
        Self { bits: result }
    }

    fn cosine_similarity(&self, other: &Self) -> f32 {
        let dist = self.hamming_distance(other);
        // Cosine similarity for binary vectors is (N - 2*Hamming) / N
        // where N is the dimension (10240).
        (10240.0 - 2.0 * dist as f32) / 10240.0
    }

    fn hamming_distance(&self, other: &Self) -> u32 {
        let mut dist = 0u32;
        for i in 0..160 {
            dist += (self.bits[i] ^ other.bits[i]).count_ones();
        }
        dist
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(1280);
        for &word in &self.bits {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 1280 {
            return Err(MemoryError::InvalidDimension {
                expected: 1280,
                actual: bytes.len(),
            });
        }
        let mut bits = [0u64; 160];
        for i in 0..160 {
            let mut word_bytes = [0u8; 8];
            word_bytes.copy_from_slice(&bytes[i * 8..(i + 1) * 8]);
            bits[i] = u64::from_le_bytes(word_bytes);
        }
        Ok(Self { bits })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bhvec_serialization_length() {
        let v = BHVec10240::random();
        let bytes = v.to_bytes();
        assert_eq!(bytes.len(), 1280);
    }

    #[test]
    fn test_conversion_roundtrip() {
        let v1 = HVec10240::random();
        let bv = BHVec10240::from_f32(&v1);
        let v2 = bv.to_f32();
        // Since f32 random uses range [-1, 1], conversion to binary (sign) and back to f32 (bipolar 1.0/-1.0)
        // is lossy but signs should match.
        for i in 0..10240 {
            assert_eq!(v1.data[i] >= 0.0, v2.data[i] >= 0.0);
        }
    }

    #[test]
    fn test_bhvec_permute_identity() {
        let v = BHVec10240::random();
        assert_eq!(v, v.permute(0));
        assert_eq!(v, v.permute(64 * 160));

        let p64 = v.permute(64);
        let mut expected = [0u64; 160];
        for i in 0..160 {
            expected[i] = v.bits[(i + 159) % 160];
        }
        assert_eq!(p64.bits, expected);
    }
}
