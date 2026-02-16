use rayon::prelude::*;

pub const HYPERVECTOR_WORDS: usize = 80;
pub const HYPERVECTOR_BITS: f64 = 10_240.0;
pub const WORD_BYTES: usize = 16;

#[derive(Clone, Copy, Debug)]
pub struct HVec10240(pub [u128; HYPERVECTOR_WORDS]);

impl HVec10240 {
    pub fn zero() -> Self {
        Self([0; HYPERVECTOR_WORDS])
    }

    pub fn from_seed(seed: u64) -> Self {
        let mut x = seed;
        let mut out = [0u128; HYPERVECTOR_WORDS];
        for item in &mut out {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            let hi = x as u128;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            let lo = x as u128;
            *item = (hi << 64) | lo;
        }
        Self(out)
    }

    pub fn cosine_similarity(&self, other: &Self) -> f64 {
        let dot: f64 = self
            .0
            .par_iter()
            .zip(other.0.par_iter())
            .map(|(a, b)| (!(a ^ b)).count_ones() as f64 - (a ^ b).count_ones() as f64)
            .sum();
        dot / HYPERVECTOR_BITS
    }

    pub fn xor_mix(&self, other: &Self) -> Self {
        let mut out = [0u128; HYPERVECTOR_WORDS];
        out.par_iter_mut().enumerate().for_each(|(i, dst)| {
            *dst = self.0[i] ^ other.0[i];
        });
        Self(out)
    }

    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.0)
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != HYPERVECTOR_WORDS * WORD_BYTES {
            return None;
        }
        let chunks: &[u128] = bytemuck::try_cast_slice(bytes).ok()?;
        let mut out = [0u128; HYPERVECTOR_WORDS];
        out.copy_from_slice(chunks);
        Some(Self(out))
    }
}
