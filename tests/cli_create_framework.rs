//! Tests for `create_framework_advanced` configuration behavior.
//!
//! Each test calls `create_framework_advanced()` and compares stored vectors
//! (via `get_concept`) to kill specific mutations in the function body.

use chaotic_semantic_memory::cli::commands::create_framework_advanced;

/// Kills: replace `&&` with `||` (line 137), replace `==` with `!=` (line 137).
///
/// When provider="hdc-text" and code_aware=true the `if` branch applies a
/// code-aware TextEncoderConfig.  With code_aware=false the `else` branch uses
/// the plain default provider.  Stored vectors must differ.
#[tokio::test]
async fn hdc_text_code_aware_vs_default_differ() {
    let fw_ca = create_framework_advanced(None, Some("hdc-text"), true, "_default")
        .await
        .expect("code-aware framework");
    let fw_def = create_framework_advanced(None, Some("hdc-text"), false, "_default")
        .await
        .expect("default framework");

    let text = "snake_case_test";
    fw_ca.inject_text("ca", text).await.unwrap();
    fw_def.inject_text("def", text).await.unwrap();

    let c_ca = fw_ca
        .get_concept("ca")
        .await
        .expect("get concept")
        .expect("concept exists");
    let c_def = fw_def
        .get_concept("def")
        .await
        .expect("get concept")
        .expect("concept exists");

    assert_ne!(
        c_ca.vector, c_def.vector,
        "code_aware=true and false must produce different stored vectors; \
         an &&→|| or ==→!= mutation would make them equal"
    );
}

/// Kills: delete field `ngram_size` (line 141), delete field `code_aware`
/// (line 142) in the `if` branch.
///
/// The `if` branch (provider="hdc-text", code_aware=true) and the `else if`
/// branch (provider=None, code_aware=true) both construct the same
/// TextEncoderConfig `{ ngram_size: Some(3), code_aware: true }`.
/// If a field is deleted from the `if` branch the configs diverge and the
/// stored vectors will differ.
#[tokio::test]
async fn if_branch_matches_elseif_branch_config() {
    let fw_if = create_framework_advanced(None, Some("hdc-text"), true, "_default")
        .await
        .expect("if-branch framework");
    let fw_elseif = create_framework_advanced(None, None, true, "_default")
        .await
        .expect("else-if-branch framework");

    let text = "camelCaseFunction";
    fw_if.inject_text("if", text).await.unwrap();
    fw_elseif.inject_text("elseif", text).await.unwrap();

    let c_if = fw_if
        .get_concept("if")
        .await
        .expect("get concept")
        .expect("concept exists");
    let c_elseif = fw_elseif
        .get_concept("elseif")
        .await
        .expect("get concept")
        .expect("concept exists");

    assert_eq!(
        c_if.vector, c_elseif.vector,
        "if-branch and else-if-branch must produce identical stored vectors; \
         a deleted config field (ngram_size or code_aware) would make them differ"
    );
}

/// Smoke: self-probe through probe_text must be near-1.0 for both code-aware
/// and default branches, confirming the encoder is wired correctly.
#[tokio::test]
async fn self_probe_near_1_for_both_branches() {
    let fw = create_framework_advanced(None, None, true, "_default")
        .await
        .expect("code-aware framework");
    fw.inject_text("a", "foo_bar_baz").await.unwrap();
    let hits = fw.probe_text("foo_bar_baz", 1).await.unwrap();
    assert_eq!(hits.len(), 1);
    assert!(
        hits[0].1 > 0.99,
        "code-aware self-probe must be near 1, got {}",
        hits[0].1
    );

    let fw_def = create_framework_advanced(None, None, false, "_default")
        .await
        .expect("default framework");
    fw_def.inject_text("a", "foo_bar_baz").await.unwrap();
    let hits_def = fw_def.probe_text("foo_bar_baz", 1).await.unwrap();
    assert!(
        hits_def[0].1 > 0.99,
        "default self-probe must also be near 1, got {}",
        hits_def[0].1
    );
}
