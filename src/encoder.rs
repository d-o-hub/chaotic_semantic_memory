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

    /// Enable code-aware tokenization (split on `_`, `-`, `.`, `/`, `::`).
    /// Default: false.
    pub code_aware: bool,
}

impl Default for TextEncoderConfig {
    fn default() -> Self {
        Self {
            position_stride: 1,
            ngram_size: None,
            lowercase: true,
            code_aware: false,
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

    /// Create a code-aware encoder with character trigram overlay.
    /// This is the recommended configuration for CLI memory-context integration.
    pub fn new_code_aware() -> Self {
        Self {
            config: TextEncoderConfig {
                ngram_size: Some(3), // Character trigram overlay
                code_aware: true,
                ..Default::default()
            },
        }
    }

    /// Get the encoder configuration.
    pub fn config(&self) -> &TextEncoderConfig {
        &self.config
    }

    /// Tokenize text with code-aware splitting.
    ///
    /// Splits on: `_`, `-`, `.`, `/`, `::` in addition to whitespace.
    /// This improves retrieval for identifiers like `my_function_name`, `MyClass.method`.
    fn tokenize_code(text: &str) -> Vec<String> {
        let mut tokens = Vec::new();

        // First split on whitespace
        for word in text.split_whitespace() {
            // Then split on code separators: `::`, `_`, `-`, `.`, `/`
            // Process `::` first since it's multi-char
            let parts = Self::split_on_separators(word);
            tokens.extend(parts);
        }

        tokens
    }

    /// Split a single word on code separators.
    fn split_on_separators(word: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut current = String::new();
        let chars: Vec<char> = word.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // Check for `::` (double colon)
            if i + 1 < chars.len() && chars[i] == ':' && chars[i + 1] == ':' {
                if !current.is_empty() {
                    result.push(current.clone());
                    current.clear();
                }
                i += 2;
                continue;
            }

            // Check for single-char separators: `_`, `-`, `.`, `/`
            let c = chars[i];
            if c == '_' || c == '-' || c == '.' || c == '/' {
                if !current.is_empty() {
                    result.push(current.clone());
                    current.clear();
                }
                i += 1;
                continue;
            }

            current.push(c);
            i += 1;
        }

        if !current.is_empty() {
            result.push(current);
        }

        result
    }

    /// Encode text into a hypervector.
    ///
    /// The encoding process:
    /// 1. Tokenize (whitespace split, optional lowercase, optional code-aware)
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

        let tokens = if self.config.code_aware {
            Self::tokenize_code(&processed)
        } else {
            processed
                .split_whitespace()
                .map(|s| s.to_string())
                .collect()
        };

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

    /// Tokenize text into a vector of tokens.
    ///
    /// This is a convenience function for reuse by other modules that need
    /// tokenization consistent with the encoder's logic.
    ///
    /// # Arguments
    /// * `text` - Input text to tokenize
    /// * `code_aware` - Enable code-aware splitting (on `_`, `-`, `.`, `/`, `::`)
    /// * `lowercase` - Convert tokens to lowercase
    pub fn tokenize(text: &str, code_aware: bool, lowercase: bool) -> Vec<String> {
        let processed = if lowercase {
            text.to_lowercase()
        } else {
            text.to_string()
        };

        if code_aware {
            Self::tokenize_code(&processed)
        } else {
            processed
                .split_whitespace()
                .map(|s| s.to_string())
                .collect()
        }
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

    #[test]
    fn test_code_aware_tokenize_snake_case() {
        let tokens = TextEncoder::tokenize_code("my_function_name");
        assert_eq!(tokens, vec!["my", "function", "name"]);
    }

    #[test]
    fn test_code_aware_tokenize_camel_case() {
        // camelCase doesn't split on separators, so it stays as one token
        // but the ngram overlay helps with similarity
        // Note: tokenize_code doesn't lowercase - that's done in encode()
        let tokens = TextEncoder::tokenize_code("MyClassName");
        assert_eq!(tokens, vec!["MyClassName"]);
    }

    #[test]
    fn test_code_aware_tokenize_path() {
        let tokens = TextEncoder::tokenize_code("src/lib.rs");
        assert_eq!(tokens, vec!["src", "lib", "rs"]);
    }
    #[test]
    fn test_code_aware_tokenize_double_colon() {
        let tokens = TextEncoder::tokenize_code("std::collections::HashMap");
        assert_eq!(tokens, vec!["std", "collections", "HashMap"]);
    }

    #[test]
    fn test_code_aware_tokenize_mixed() {
        let tokens = TextEncoder::tokenize_code("my_module::MyClass.method_name");
        assert_eq!(tokens, vec!["my", "module", "MyClass", "method", "name"]);
    }

    #[test]
    fn test_code_aware_similarity() {
        let encoder = TextEncoder::new_code_aware();
        // Similar function names should have high similarity
        let hv1 = encoder.encode("get_user_by_id");
        let hv2 = encoder.encode("get_user_by_name");
        assert!(hv1.cosine_similarity(&hv2) > 0.5);
    }

    #[test]
    fn test_code_aware_deterministic() {
        let encoder = TextEncoder::new_code_aware();
        let hv1 = encoder.encode("fn process_data(input: &str) -> Result");
        let hv2 = encoder.encode("fn process_data(input: &str) -> Result");
        assert!(hv1.cosine_similarity(&hv2) > 0.99);
    }

    #[test]
    fn test_code_aware_vs_regular() {
        let regular = TextEncoder::new();
        let code_aware = TextEncoder::new_code_aware();
        // Code-aware should produce different vectors due to splitting
        let hv1 = regular.encode("my_function_name");
        let hv2 = code_aware.encode("my_function_name");
        // They should be different (not identical)
        assert!(hv1.cosine_similarity(&hv2) < 0.95);
    }

    #[test]
    fn test_split_on_separators_edge_cases() {
        // Empty string
        let tokens = TextEncoder::split_on_separators("");
        assert!(tokens.is_empty());
        // Only separators
        let tokens = TextEncoder::split_on_separators("___");
        assert!(tokens.is_empty());
        // Leading separator
        let tokens = TextEncoder::split_on_separators("_leading");
        assert_eq!(tokens, vec!["leading"]);
        // Trailing separator
        let tokens = TextEncoder::split_on_separators("trailing_");
        assert_eq!(tokens, vec!["trailing"]);
    }
}
