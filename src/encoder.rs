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
//! use chaotic_semantic_memory::hyperdim::Hypervector;
//! use chaotic_semantic_memory::HVec10240;
//!
//! let encoder = TextEncoder::new();
//! let hv1 = encoder.encode("hello world");
//! let hv2 = encoder.encode("hello world");
//! assert!(hv1.cosine_similarity(&hv2) > 0.99); // Deterministic
//! ```

use crate::hyperdim::{HVec10240, Hypervector};

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
#[derive(Debug, Clone, Default)]
pub struct TextEncoder {
    config: TextEncoderConfig,
}

impl TextEncoder {
    /// Create a new encoder with default configuration.
    pub fn new() -> Self {
        Self {
            config: TextEncoderConfig::default(),
        }
    }

    /// Create an encoder with custom configuration.
    pub const fn with_config(config: TextEncoderConfig) -> Self {
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
    pub const fn config(&self) -> &TextEncoderConfig {
        &self.config
    }

    /// Tokenize text with code-aware splitting.
    ///
    /// Splits on: `_`, `-`, `.`, `/`, `::` in addition to whitespace.
    /// This improves retrieval for identifiers like `my_function_name`, `MyClass.method`.
    fn tokenize_code(text: &str) -> Vec<&str> {
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
    fn split_on_separators(word: &str) -> Vec<&str> {
        let mut result = Vec::new();
        let mut start = 0;
        let mut char_indices = word.char_indices().peekable();

        while let Some((i, c)) = char_indices.next() {
            let is_sep = match c {
                ':' => {
                    if let Some(&(_, next_c)) = char_indices.peek() {
                        if next_c == ':' {
                            char_indices.next(); // consume second ':'
                            if i > start {
                                result.push(&word[start..i]);
                            }
                            start = i + 2; // '::' is 2 bytes
                            continue;
                        }
                    }
                    false
                }
                '_' | '-' | '.' | '/' => true,
                _ => false,
            };

            if is_sep {
                if i > start {
                    result.push(&word[start..i]);
                }
                start = i + 1; // these are all 1-byte ASCII
            }
        }

        if start < word.len() {
            result.push(&word[start..]);
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
        let processed_owned: Option<String>;
        let processed = if self.config.lowercase {
            processed_owned = Some(text.to_lowercase());
            processed_owned.as_ref().expect("owned string must be set")
        } else {
            processed_owned = None;
            text
        };
        let _ = &processed_owned; // Silence unused warning

        let tokens = if self.config.code_aware {
            Self::tokenize_code(processed)
        } else {
            processed.split_whitespace().collect()
        };

        if tokens.is_empty() {
            return HVec10240::zero();
        }

        // Generate position-encoded vectors for each token
        let encoded_vectors: Vec<HVec10240> = tokens
            .iter()
            .enumerate()
            .map(|(pos, &token)| {
                let base = self.token_to_hvec(token);
                Hypervector::permute(&base, pos * self.config.position_stride)
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
            let ngram_hv = self.encode_ngrams(processed, n);
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
        let processed_owned: Option<String>;
        let processed = if lowercase {
            processed_owned = Some(text.to_lowercase());
            processed_owned.as_ref().expect("owned string must be set")
        } else {
            processed_owned = None;
            text
        };
        let _ = &processed_owned; // Silence unused warning

        if code_aware {
            Self::tokenize_code(processed)
                .into_iter()
                .map(|s| s.to_string())
                .collect()
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
        let char_indices: Vec<(usize, char)> = text.char_indices().collect();

        if char_indices.len() < n {
            return HVec10240::zero();
        }

        let ngram_vectors: Vec<HVec10240> = char_indices
            .windows(n)
            .map(|window| {
                let start = window[0].0;
                let end = window[n - 1].0 + window[n - 1].1.len_utf8();
                self.token_to_hvec(&text[start..end])
            })
            .collect();

        HVec10240::bundle(&ngram_vectors).unwrap_or_else(|_| HVec10240::zero())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_deterministic() {
        let encoder = TextEncoder::new();
        let text = "hello world";
        let v1 = encoder.encode(text);
        let v2 = encoder.encode(text);
        // Same text → same vector (deterministic)
        assert_eq!(v1, v2);
    }

    #[test]
    fn encode_position_aware() {
        let encoder = TextEncoder::new();
        let v1 = encoder.encode("cat sat");
        let v2 = encoder.encode("sat cat");
        // Different order → different vectors
        assert_ne!(v1, v2);
    }

    #[test]
    fn tokenize_splits_whitespace() {
        let tokens = TextEncoder::tokenize("hello world test", false, true);
        assert_eq!(tokens, vec!["hello", "world", "test"]);
    }

    #[test]
    fn tokenize_lowercase() {
        let tokens = TextEncoder::tokenize("HELLO World", false, true);
        assert_eq!(tokens, vec!["hello", "world"]);
    }

    #[test]
    fn tokenize_code_aware() {
        let tokens = TextEncoder::tokenize("my_var::method", true, true);
        // Code-aware splits on :: and _ (underscore is a separator)
        // "my_var::method" → ["my", "var", "method"]
        assert!(tokens.contains(&"my".to_string()));
        assert!(tokens.contains(&"var".to_string()));
        assert!(tokens.contains(&"method".to_string()));
    }

    #[test]
    fn encode_with_ngrams() {
        let encoder = TextEncoder::new();
        let v = encoder.encode_with_ngrams("abc", 2);
        // N-gram encoding should produce a non-zero vector
        let zero = HVec10240::zero();
        assert!(v.hamming_distance(&zero) > 0);
    }

    #[test]
    fn stable_hash_consistent() {
        let encoder = TextEncoder::new();
        let h1 = encoder.stable_hash("test_token");
        let h2 = encoder.stable_hash("test_token");
        assert_eq!(h1, h2);
    }
}
