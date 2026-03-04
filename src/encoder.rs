//! Text-to-Hypervector Encoding using Hyperdimensional Computing (HDC) principles.
//!
//! This module provides a deterministic text encoder that converts text strings into
//! `HVec10240` hypervectors without requiring external ML dependencies or embeddings.
//!
//! # Algorithm
//!
//! 1. **Tokenize**: Split on whitespace, lowercase, optional unicode segmentation
//! 2. **Token → base HVec**: FNV-1a hash → seeded PRNG → random HVec10240
//! 3. **Position encoding**: `token_hv.permute(position * stride)`
//! 4. **Bundle**: Majority-rule bundling of all position-encoded token vectors
//! 5. **Optional**: Character n-gram overlay for typo robustness
//!
//! # Hash Stability
//!
//! Token hashing uses FNV-1a (Fowler–Noll–Vo 1a, 64-bit), implemented inline with
//! no external dependencies. FNV-1a is guaranteed stable across Rust versions and
//! platforms, unlike `std::collections::hash_map::DefaultHasher` (SipHash), which
//! is explicitly documented as non-stable. This ensures encoded vectors are
//! reproducible across Rust upgrades and different builds.
//!
//! # Example
//!
//! ```
//! use chaotic_semantic_memory::encoder::{TextEncoder, TextEncoderConfig};
//! use chaotic_semantic_memory::HVec10240;
//!
//! let encoder = TextEncoder::new();
//! let hv1 = encoder.encode("hello world");
//! let hv2 = encoder.encode("hello world");
//! assert!(hv1.cosine_similarity(&hv2) > 0.99); // Deterministic
//! ```

use crate::hyperdim::HVec10240;

/// FNV-1a 64-bit offset basis and prime (Fowler–Noll–Vo).
const FNV1A_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV1A_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Compute a stable FNV-1a 64-bit hash for a byte slice.
///
/// This is guaranteed stable across Rust versions and platforms, unlike
/// `DefaultHasher` (SipHash), which is explicitly non-stable.
#[inline]
fn fnv1a_hash(bytes: &[u8]) -> u64 {
    let mut hash = FNV1A_OFFSET_BASIS;
    for &byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV1A_PRIME);
    }
    hash
}

/// Configuration for the text encoder.
#[derive(Debug, Clone)]
pub struct TextEncoderConfig {
    /// Number of positions to shift for position encoding.
    /// Default: 1 (each token position shifts by 1 permutation).
    pub position_stride: usize,

    /// Whether to include character n-grams for typo robustness.
    /// Default: false.
    pub ngram_size: Option<usize>,

    /// Whether to lowercase text before encoding.
    /// Default: true.
    pub lowercase: bool,
}

impl Default for TextEncoderConfig {
    fn default() -> Self {
        Self {
            position_stride: 1,
            ngram_size: None,
            lowercase: true,
        }
    }
}

/// Deterministic text-to-hypervector encoder using HDC principles.
///
/// Produces consistent `HVec10240` vectors from text input without external dependencies.
/// The encoding is:
/// - **Deterministic**: Same input always produces same output
/// - **Similarity-preserving**: Similar texts produce similar vectors
/// - **WASM-compatible**: No external dependencies
#[derive(Debug, Clone)]
pub struct TextEncoder {
    config: TextEncoderConfig,
}

impl Default for TextEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl TextEncoder {
    /// Create a new encoder with default configuration.
    pub fn new() -> Self {
        Self {
            config: TextEncoderConfig::default(),
        }
    }

    /// Create an encoder with custom configuration.
    pub fn with_config(config: TextEncoderConfig) -> Self {
        Self { config }
    }

    /// Encode text into a hypervector.
    ///
    /// The encoding process:
    /// 1. Tokenize (whitespace split, optional lowercase)
    /// 2. Generate deterministic base vector for each token
    /// 3. Apply position encoding via permutation
    /// 4. Bundle all position-encoded vectors
    /// 5. Optionally add n-gram overlay
    pub fn encode(&self, text: &str) -> HVec10240 {
        let processed = if self.config.lowercase {
            text.to_lowercase()
        } else {
            text.to_string()
        };

        let tokens: Vec<&str> = processed.split_whitespace().collect();

        if tokens.is_empty() {
            return HVec10240::zero();
        }

        // Generate position-encoded vectors for each token
        let encoded_vectors: Vec<HVec10240> = tokens
            .iter()
            .enumerate()
            .map(|(pos, token)| {
                let base = self.token_to_hvec(token);
                base.permute(pos * self.config.position_stride)
            })
            .collect();

        // Bundle all position-encoded vectors.
        // `HVec10240::bundle` only fails on empty input; we guard against that above,
        // so the fallback to zero is a defensive no-op that avoids propagating an
        // unreachable error through the public API.
        let mut result = HVec10240::bundle(&encoded_vectors).unwrap_or_else(|_| HVec10240::zero());

        // Optionally add n-gram overlay.
        // Same reasoning: bundle of non-empty slice is infallible in practice.
        if let Some(n) = self.config.ngram_size {
            let ngram_hv = self.encode_ngrams(&processed, n);
            // Blend n-gram encoding with token encoding
            result = HVec10240::bundle(&[result, ngram_hv]).unwrap_or_else(|_| HVec10240::zero());
        }

        result
    }

    /// Encode text with character n-grams for typo robustness.
    ///
    /// This is equivalent to setting `ngram_size` in the config.
    pub fn encode_with_ngrams(&self, text: &str, n: usize) -> HVec10240 {
        let config = TextEncoderConfig {
            ngram_size: Some(n),
            ..self.config.clone()
        };
        let encoder = Self::with_config(config);
        encoder.encode(text)
    }

    /// Convert a token to a deterministic hypervector.
    ///
    /// Uses FNV-1a hash → seeded PRNG → random HVec10240 for reproducibility.
    fn token_to_hvec(&self, token: &str) -> HVec10240 {
        // Compute stable hash
        let hash = self.stable_hash(token);

        // Use hash as seed for deterministic PRNG
        HVec10240::new_seeded(hash)
    }

    /// Compute a stable FNV-1a hash for a token.
    ///
    /// Uses FNV-1a (64-bit) for guaranteed cross-version stability.
    /// `DefaultHasher` (SipHash) is explicitly non-stable across Rust versions.
    fn stable_hash(&self, token: &str) -> u64 {
        fnv1a_hash(token.as_bytes())
    }

    /// Encode text using character n-grams.
    ///
    /// Generates n-grams, encodes each, and bundles them together.
    fn encode_ngrams(&self, text: &str, n: usize) -> HVec10240 {
        let chars: Vec<char> = text.chars().collect();

        if chars.len() < n {
            return HVec10240::zero();
        }

        let ngrams: Vec<String> = chars
            .windows(n)
            .map(|window| window.iter().collect::<String>())
            .collect();

        if ngrams.is_empty() {
            return HVec10240::zero();
        }

        let ngram_vectors: Vec<HVec10240> = ngrams
            .iter()
            .map(|ngram| self.token_to_hvec(ngram))
            .collect();

        HVec10240::bundle(&ngram_vectors).unwrap_or_else(|_| HVec10240::zero())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_deterministic() {
        let encoder = TextEncoder::new();
        let hv1 = encoder.encode("hello world");
        let hv2 = encoder.encode("hello world");
        assert!(hv1.cosine_similarity(&hv2) > 0.99);
    }

    #[test]
    fn test_encode_empty_returns_zero() {
        let encoder = TextEncoder::new();
        let hv = encoder.encode("");
        assert_eq!(hv, HVec10240::zero());
    }

    #[test]
    fn test_encode_whitespace_only_returns_zero() {
        let encoder = TextEncoder::new();
        let hv = encoder.encode("   \t\n  ");
        assert_eq!(hv, HVec10240::zero());
    }

    #[test]
    fn test_encode_similar_texts() {
        let encoder = TextEncoder::new();
        let hv1 = encoder.encode("the quick brown fox");
        let hv2 = encoder.encode("the quick brown fox jumps");
        // Similar texts should have positive similarity
        assert!(hv1.cosine_similarity(&hv2) > 0.5);
    }

    #[test]
    fn test_encode_dissimilar_texts() {
        let encoder = TextEncoder::new();
        let hv1 = encoder.encode("hello world");
        let hv2 = encoder.encode("xyzzy plugh");
        // Dissimilar texts should have lower similarity
        assert!(hv1.cosine_similarity(&hv2) < 0.7);
    }

    #[test]
    fn test_encode_with_ngrams() {
        let encoder = TextEncoder::new();
        let hv1 = encoder.encode_with_ngrams("hello", 2);
        let hv2 = encoder.encode_with_ngrams("hello", 2);
        assert!(hv1.cosine_similarity(&hv2) > 0.99);
    }

    #[test]
    fn test_encode_case_insensitive_by_default() {
        let encoder = TextEncoder::new();
        let hv1 = encoder.encode("Hello World");
        let hv2 = encoder.encode("hello world");
        assert!(hv1.cosine_similarity(&hv2) > 0.99);
    }

    #[test]
    fn test_encode_case_sensitive() {
        let config = TextEncoderConfig {
            lowercase: false,
            ..Default::default()
        };
        let encoder = TextEncoder::with_config(config);
        let hv1 = encoder.encode("Hello World");
        let hv2 = encoder.encode("hello world");
        // Case-sensitive should produce different vectors
        assert!(hv1.cosine_similarity(&hv2) < 0.99);
    }

    #[test]
    fn test_position_encoding_affects_result() {
        let encoder = TextEncoder::new();
        let hv1 = encoder.encode("cat dog");
        let hv2 = encoder.encode("dog cat");
        // Word order matters due to position encoding
        assert!(hv1.cosine_similarity(&hv2) < 0.99);
    }

    #[test]
    fn test_config_custom_stride() {
        let config = TextEncoderConfig {
            position_stride: 5,
            ..Default::default()
        };
        let encoder = TextEncoder::with_config(config);
        let hv = encoder.encode("hello world");
        // Should still produce a valid hypervector
        assert_ne!(hv, HVec10240::zero());
    }
}
