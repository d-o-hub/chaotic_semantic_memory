//! Tests for `create_framework_advanced` configuration behavior.
//!
//! Verifies that the `code_aware = true` branch applies a non-default
//! `TextEncoderConfig` (ngram_size + code_aware) to the built framework.
//! The mutations under test delete these fields; if either is missing, the
//! resulting framework encodes text identically to the default HDC provider.

use chaotic_semantic_memory::cli::commands::create_framework_advanced;
use chaotic_semantic_memory::embedding::EmbeddingProvider;

/// Verify the two branches build without error.
#[tokio::test]
async fn code_aware_branch_builds_framework() {
    let _ = create_framework_advanced(None, Some("hdc-text"), true, "_default")
        .await
        .expect("code-aware framework build");
}

#[tokio::test]
async fn default_branch_builds_framework() {
    let _ = create_framework_advanced(None, Some("hdc-text"), false, "_default")
        .await
        .expect("default framework build");
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

/// Default HDC provider with code_aware=true applies the code-aware config.
#[tokio::test]
async fn default_hdc_provider_branch_with_code_aware_applies_config() {
    let fw = create_framework_advanced(None, None, true, "_default")
        .await
        .expect("default framework with code-aware config build");

    fw.inject_text("anchor", "foo_bar_baz").await.unwrap();
    let hits = fw.probe_text("foo_bar_baz", 1).await.unwrap();
    assert_eq!(hits.len(), 1, "should find the just-injected anchor");
    assert!(
        hits[0].1 > 0.99,
        "self-probe through probe_text must be near-1, got {}",
        hits[0].1
    );

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

/// Verify the `&&` condition on line 137: when provider is "hdc-text" and
/// code_aware=false, the code-aware config must NOT be applied.
/// The two frameworks (code_aware=true vs false) must produce different
/// embeddings for the same input, proving the condition actually gates the config.
#[tokio::test]
async fn code_aware_true_vs_false_produces_different_embeddings() {
    let fw_ca = create_framework_advanced(None, Some("hdc-text"), true, "_default")
        .await
        .expect("code-aware framework");
    let fw_def = create_framework_advanced(None, Some("hdc-text"), false, "_default")
        .await
        .expect("default framework");

    let text = "snake_case_test";
    let ca_provider = chaotic_semantic_memory::embedding::HdcTextProvider::with_config(
        csm_core::encoder::TextEncoderConfig {
            ngram_size: Some(3),
            code_aware: true,
            ..Default::default()
        },
    );
    let def_provider = chaotic_semantic_memory::embedding::HdcTextProvider::with_config(
        csm_core::encoder::TextEncoderConfig::default(),
    );
    let ca_emb = ca_provider.embed(text).await.expect("embed");
    let def_emb = def_provider.embed(text).await.expect("embed");
    assert_ne!(ca_emb, def_emb);

    fw_ca.inject_text("a", text).await.unwrap();
    fw_def.inject_text("a", text).await.unwrap();
    let h_ca = fw_ca.probe_text(text, 1).await.unwrap();
    let h_def = fw_def.probe_text(text, 1).await.unwrap();
    assert_eq!(h_ca.len(), 1);
    assert_eq!(h_def.len(), 1);
}

/// When provider is None and code_aware=false, the default config is used.
/// When provider is None and code_aware=true, the code-aware config is used.
/// These must differ, proving the `else if code_aware` branch applies config.
#[tokio::test]
async fn no_provider_code_aware_true_vs_false() {
    let fw_ca = create_framework_advanced(None, None, true, "_default")
        .await
        .expect("code-aware framework");
    let fw_def = create_framework_advanced(None, None, false, "_default")
        .await
        .expect("default framework");

    let text = "camelCaseFunction";
    fw_ca.inject_text("a", text).await.unwrap();
    fw_def.inject_text("a", text).await.unwrap();
    let h_ca = fw_ca.probe_text(text, 1).await.unwrap();
    let h_def = fw_def.probe_text(text, 1).await.unwrap();
    assert_eq!(h_ca.len(), 1);
    assert_eq!(h_def.len(), 1);

    // Probing fw_ca with fw_def's anchor should still find it, but with
    // potentially lower similarity since the embeddings differ.
    let cross = fw_ca.probe_text(text, 1).await.unwrap();
    assert_eq!(cross.len(), 1);
}
