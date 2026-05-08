use crate::error::Result;
use super::Hypervector;
use rand::RngExt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HVec10240 {
    pub(crate) data: [f32; 10240],
}

impl Eq for HVec10240 {}

impl HVec10240 {
    pub const DIMENSION: usize = 10240;
    pub const fn zero_const() -> Self { Self { data: [0.0; 10240] } }
    pub fn new_seeded(seed: u64) -> Self {
        use rand::SeedableRng;
        use rand::rngs::StdRng;
        let mut rng = StdRng::seed_from_u64(seed);
        let mut data = [0.0; 10240];
        for v in &mut data { *v = rng.random_range(-1.0..1.0); }
        Self { data }
    }
}

impl Hypervector for HVec10240 {
    const DIMENSION: usize = 10240;
    fn format_name() -> &'static str { "f32" }
    fn zero() -> Self { Self::zero_const() }
    fn random() -> Self {
        let mut rng = rand::rng();
        let mut data = [0.0; 10240];
        for v in &mut data { *v = rng.random_range(-1.0..1.0); }
        Self { data }
    }
    fn bind(&self, other: &Self) -> Self {
        let mut data = [0.0; 10240];
        for i in 0..10240 { data[i] = self.data[i] * other.data[i]; }
        Self { data }
    }
    fn bundle(vectors: &[Self]) -> Result<Self> {
        if vectors.is_empty() { return Ok(Self::zero()); }
        let mut data = [0.0; 10240];
        for v in vectors {
            for i in 0..10240 { data[i] += v.data[i]; }
        }
        Ok(Self { data })
    }
    fn permute(&self, shift: usize) -> Self {
        let mut data = [0.0; 10240];
        let s = shift % 10240;
        let (l, r) = self.data.split_at(10240 - s);
        data[..s].copy_from_slice(r);
        data[s..].copy_from_slice(l);
        Self { data }
    }
    fn cosine_similarity(&self, other: &Self) -> f32 {
        let mut dot = 0.0;
        let mut n1 = 0.0;
        let mut n2 = 0.0;
        for i in 0..10240 {
            dot += self.data[i] * other.data[i];
            n1 += self.data[i] * self.data[i];
            n2 += other.data[i] * other.data[i];
        }
        if n1 == 0.0 || n2 == 0.0 { return 0.0; }
        dot / (n1.sqrt() * n2.sqrt())
    }
    fn hamming_distance(&self, other: &Self) -> u32 {
        let mut d = 0;
        for i in 0..10240 {
            if (self.data[i] >= 0.0) != (other.data[i] >= 0.0) { d += 1; }
        }
        d
    }
    fn to_bytes(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(40960);
        for &v in &self.data { b.extend_from_slice(&v.to_le_bytes()); }
        b
    }
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 40960 { return Err(crate::error::MemoryError::InvalidDimension { expected: 40960, actual: bytes.len() }); }
        let mut data = [0.0; 10240];
        for i in 0..10240 {
            let mut b = [0u8; 4];
            b.copy_from_slice(&bytes[i*4..(i+1)*4]);
            data[i] = f32::from_le_bytes(b);
        }
        Ok(Self { data })
    }
}

impl serde::Serialize for HVec10240 {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> where S: serde::Serializer {
        serializer.serialize_bytes(&self.to_bytes())
    }
}

impl<'de> serde::Deserialize<'de> for HVec10240 {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error> where D: serde::Deserializer<'de> {
        let b = <Vec<u8>>::deserialize(deserializer)?;
        Self::from_bytes(&b).map_err(serde::de::Error::custom)
    }
}
