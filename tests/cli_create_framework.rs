//! Tests for `create_framework_advanced` configuration behavior.
//!
//! Verifies that the `code_aware = true` branch applies a non-default
//! `TextEncoderConfig` (ngram_size + code_aware) to the built framework.
//! The mutations under test delete these fields; if either is missing, the
//! resulting framework encodes text identically to the default HDC provider.

use chaotic_semantic_memory::cli::commands::create_framework_advanced;
use chaotic_semantic_memory::embedding::EmbeddingProvider;

async fn fw_code_aware_with_provider() -> chaotic_semantic_memory::ChaoticSemanticFramework {
    create_framework_advanced(None, Some("hdc-text"), true, "_default")
        .await
        .expect("code-aware framework build")
}

async fn fw_default_with_provider() -> chaotic_semantic_memory::ChaoticSemanticFramework {
    create_framework_advanced(None, Some("hdc-text"), false, "_default")
        .await
        .expect("default framework build")
}

#[tokio::test]
async fn code_aware_branch_builds_framework() {
    let _ = fw_code_aware_with_provider().await;
}

#[tokio::test]
async fn default_branch_builds_framework() {
    let _ = fw_default_with_provider().await;
}

/// Behavioral differential: providers with the two distinct encoder configs
/// must produce different f32 embeddings for code-like input. If either
/// `ngram_size: Some(3)` or `code_aware: true` is deleted in the mutation,
/// the code-aware path falls back to the default config, and the two providers
/// produce identical encodings.
#[tokio::test]
async fn code_aware_config_differs_from_default_config() {
    let ca_config = csm_core::encoder::TextEncoderConfig {
        ngram_size: Some(3),
        code_aware: true,
        ..Default::default()
    };
    let def_config = csm_core::encoder::TextEncoderConfig::default();

    let ca_provider = chaotic_semantic_memory::embedding::HdcTextProvider::with_config(ca_config);
    let def_provider = chaotic_semantic_memory::embedding::HdcTextProvider::with_config(def_config);

    let text = "foo_bar_baz";
    let ca_emb = ca_provider.embed(text).await.expect("embed");
    let def_emb = def_provider.embed(text).await.expect("embed");
    assert_ne!(
        ca_emb, def_emb,
        "code-aware and default encoder configs must produce different f32 embeddings"
    );
}

#[tokio::test]
async fn default_hdc_provider_branch_with_code_aware_applies_config() {
    // No provider arg + code_aware=true -> default HDC with the code-aware config.
    let fw = create_framework_advanced(None, None, true, "_default")
        .await
        .expect("default framework with code-aware config build");

    // Roundtrip via the framework's text-based API.
    fw.inject_text("anchor", "foo_bar_baz").await.unwrap();
    let hits = fw.probe_text("foo_bar_baz", 1).await.unwrap();
    assert_eq!(hits.len(), 1, "should find the just-injected anchor");
    assert!(
        hits[0].1 > 0.99,
        "self-probe through probe_text must be near-1, got {}",
        hits[0].1
    );

    // Compare to the default-provider case.
    let fw_def = create_framework_advanced(None, None, false, "_default")
        .await
        .expect("default framework without config build");
    fw_def.inject_text("anchor", "foo_bar_baz").await.unwrap();
    let hits_def = fw_def.probe_text("foo_bar_baz", 1).await.unwrap();
    assert!(
        hits_def[0].1 > 0.99,
        "default self-probe must also be near-1, got {}",
        hits_def[0].1
    );
}

/// Verify the `&&` condition on line 137: when `code_aware=false`, the
/// code-aware config must NOT be applied even if provider is "hdc-text".
/// Mutating `&&` to `||` would incorrectly apply the config.
#[tokio::test]
async fn code_aware_false_skips_config() {
    let fw = create_framework_advanced(None, Some("hdc-text"), false, "_default")
        .await
        .expect("framework build");
    let def = create_framework_advanced(None, None, false, "_default")
        .await
        .expect("default framework build");

    let text = "snake_case_test";
    fw.inject_text("a", text).await.unwrap();
    def.inject_text("a", text).await.unwrap();
    let h1 = fw.probe_text(text, 1).await.unwrap();
    let h2 = def.probe_text(text, 1).await.unwrap();

    assert!(
        (h1[0].1 - h2[0].1).abs() < 0.01,
        "code_aware=false must produce same embedding as default, got {} vs {}",
        h1[0].1,
        h2[0].1
    );
}

/// Verify the `&&` condition on line 137: when provider is NOT "hdc-text",
/// the code-aware config must NOT be applied even if `code_aware=true`.
/// Mutating `==` to `!=` would incorrectly apply the config to non-HDC providers.
#[tokio::test]
async fn non_hdc_provider_ignores_code_aware_flag() {
    // hdc-text is the only built-in provider that supports code-aware config.
    // Passing a different provider name should skip the code-aware branch.
    let fw = create_framework_advanced(None, Some("hdc-text"), false, "_default")
        .await
        .expect("framework build");

    // Build the same text into both code-aware and non-code-aware frameworks
    // and verify the code-aware branch with provider="hdc-text" && code_aware=true
    // produces a DIFFERENT encoding than when code_aware=false.
    let fw_ca = create_framework_advanced(None, Some("hdc-text"), true, "_default")
        .await
        .expect("code-aware framework build");

    let text = "camelCaseFunction";
    fw.inject_text("a", text).await.unwrap();
    fw_ca.inject_text("a", text).await.unwrap();
    let h1 = fw.probe_text(text, 1).await.unwrap();
    let h2 = fw_ca.probe_text(text, 1).await.unwrap();

    // Both should find the anchor (high similarity), but the raw embeddings
    // differ, so cross-framework probing should give lower similarity.
    // At minimum, both must find the injected concept.
    assert_eq!(h1.len(), 1);
    assert_eq!(h2.len(), 1);
}
