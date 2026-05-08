//! Tests for the text-to-hypervector encoder.
//!
//! These tests verify determinism, similarity preservation, and code-aware tokenization.

use chaotic_semantic_memory::HVec10240;
use chaotic_semantic_memory::encoder::{TextEncoder, TextEncoderConfig};

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
    let tokens = TextEncoder::tokenize("my_function_name", true, false);
    assert_eq!(tokens, vec!["my", "function", "name"]);
}

#[test]
fn test_code_aware_tokenize_camel_case() {
    // camelCase doesn't split on separators, so it stays as one token
    // but the ngram overlay helps with similarity
    let tokens = TextEncoder::tokenize("MyClassName", true, false);
    assert_eq!(tokens, vec!["MyClassName"]);
}

#[test]
fn test_code_aware_tokenize_path() {
    let tokens = TextEncoder::tokenize("src/lib.rs", true, false);
    assert_eq!(tokens, vec!["src", "lib", "rs"]);
}

#[test]
fn test_code_aware_tokenize_double_colon() {
    let tokens = TextEncoder::tokenize("std::collections::HashMap", true, false);
    assert_eq!(tokens, vec!["std", "collections", "HashMap"]);
}

#[test]
fn test_code_aware_tokenize_mixed() {
    let tokens = TextEncoder::tokenize("my_module::MyClass.method_name", true, false);
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
fn test_tokenize_edge_cases() {
    // Empty string
    let tokens = TextEncoder::tokenize("", true, false);
    assert!(tokens.is_empty());

    // Only separators (whitespace removed first, then splitting empty)
    let tokens = TextEncoder::tokenize("___", true, false);
    assert!(tokens.is_empty());

    // Leading separator
    let tokens = TextEncoder::tokenize("_leading", true, false);
    assert_eq!(tokens, vec!["leading"]);

    // Trailing separator
    let tokens = TextEncoder::tokenize("trailing_", true, false);
    assert_eq!(tokens, vec!["trailing"]);
}

#[test]
fn test_golden_vector_stability() {
    // Regression test: ensure encoding produces consistent vectors
    // across multiple invocations (FNV-1a hash stability)
    let encoder = TextEncoder::new();
    let input = "test stability golden vector";

    // Encode multiple times
    let vectors: Vec<HVec10240> = (0..5).map(|_| encoder.encode(input)).collect();

    // All should be identical
    for i in 1..5 {
        assert!(
            vectors[0].cosine_similarity(&vectors[i]) > 0.999,
            "vectors[0] and vectors[{i}] should be nearly identical"
        );
    }
}
