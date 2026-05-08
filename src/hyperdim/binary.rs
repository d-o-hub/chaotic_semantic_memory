use super::Hypervector;
use crate::error::Result;
use crate::hyperdim::HVec10240;
use rand::RngExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BHVec10240 {
    pub(crate) bits: [u64; 160],
}

impl BHVec10240 {
    pub fn from_f32(v: &HVec10240) -> Self {
        let mut bits = [0u64; 160];
        for (i, &val) in v.data.iter().enumerate() {
            if val >= 0.0 {
                bits[i / 64] |= 1u64 << (i % 64);
            }
        }
        Self { bits }
    }
    pub fn to_f32(&self) -> HVec10240 {
        let mut data = [0.0; 10240];
        for i in 0..10240 {
            if (self.bits[i / 64] & (1u64 << (i % 64))) != 0 {
                data[i] = 1.0;
            } else {
                data[i] = -1.0;
            }
        }
        HVec10240 { data }
    }
}

impl Hypervector for BHVec10240 {
    const DIMENSION: usize = 10240;
    fn format_name() -> &'static str {
        "binary"
    }
    fn zero() -> Self {
        Self { bits: [0; 160] }
    }
    fn random() -> Self {
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
        let mut counts = vec![0i32; 10240];
        for v in vectors {
            for i in 0..10240 {
                if (v.bits[i / 64] & (1u64 << (i % 64))) != 0 {
                    counts[i] += 1;
                } else {
                    counts[i] -= 1;
                }
            }
        }
        let mut bits = [0u64; 160];
        for i in 0..10240 {
            if counts[i] > 0 {
                bits[i / 64] |= 1u64 << (i % 64);
            }
        }
        Ok(Self { bits })
    }
    fn permute(&self, shift: usize) -> Self {
        // Simple word-level for now
        let mut bits = [0u64; 160];
        let s = (shift / 64) % 160;
        for i in 0..160 {
            bits[i] = self.bits[(i + 160 - s) % 160];
        }
        Self { bits }
    }
    fn cosine_similarity(&self, other: &Self) -> f32 {
        let d = self.hamming_distance(other);
        (10240.0 - 2.0 * d as f32) / 10240.0
    }
    fn hamming_distance(&self, other: &Self) -> u32 {
        let mut d = 0;
        for i in 0..160 {
            d += (self.bits[i] ^ other.bits[i]).count_ones();
        }
        d
    }
    fn to_bytes(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(1280);
        for &w in &self.bits {
            b.extend_from_slice(&w.to_le_bytes());
        }
        b
    }
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 1280 {
            return Err(crate::error::MemoryError::InvalidDimension {
                expected: 1280,
                actual: bytes.len(),
            });
        }
        let mut bits = [0u64; 160];
        for i in 0..160 {
            let mut b = [0u8; 8];
            b.copy_from_slice(&bytes[i * 8..(i + 1) * 8]);
            bits[i] = u64::from_le_bytes(b);
        }
        Ok(Self { bits })
    }

    fn from_f32_array(arr: &[f32; 10240]) -> Self {
        let mut bits = [0u64; 160];
        for (i, &val) in arr.iter().enumerate() {
            if val >= 0.0 {
                bits[i / 64] |= 1u64 << (i % 64);
            }
        }
        Self { bits }
    }
}

impl serde::Serialize for BHVec10240 {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.to_bytes())
    }
}

impl<'de> serde::Deserialize<'de> for BHVec10240 {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let b = <Vec<u8>>::deserialize(deserializer)?;
        Self::from_bytes(&b).map_err(serde::de::Error::custom)
    }
}
