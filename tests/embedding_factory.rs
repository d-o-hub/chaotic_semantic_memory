//! Tests for the embedding provider factory `get_provider`.
//!
//! These tests verify the match arm logic and feature-gate error messages.
//! Without feature flags (default `cargo test`), each provider arm should
//! return a specific "feature not enabled" error. Deleting any match arm
//! would cause the test to receive the "unknown embedding provider" error
//! instead, killing the mutation.

use chaotic_semantic_memory::embedding::get_provider;

fn provider_error(name: &str) -> String {
    match get_provider(name) {
        Ok(p) => panic!(
            "expected error from get_provider({name:?}), got provider {}",
            p.name()
        ),
        Err(e) => e.to_string(),
    }
}

fn provider_ok_name(name: &str) -> String {
    match get_provider(name) {
        Ok(p) => p.name().to_string(),
        Err(e) => panic!("expected provider for {name:?}, got error: {e}"),
    }
}

#[test]
fn get_provider_hdc_text_succeeds() {
    assert_eq!(provider_ok_name("hdc-text"), "hdc-text");
}

#[test]
fn get_provider_hdc_alias_succeeds() {
    assert_eq!(provider_ok_name("hdc"), "hdc-text");
}

#[test]
fn get_provider_hdc_with_model_succeeds() {
    assert_eq!(provider_ok_name("hdc-text:any-model"), "hdc-text");
}

#[test]
fn get_provider_fastembed_arm_returns_feature_specific_error() {
    let msg = provider_error("fastembed");
    assert!(
        msg.contains("embed-fastembed feature not enabled"),
        "expected feature-specific error for fastembed arm, got: {msg}"
    );
    assert!(
        !msg.contains("unknown embedding provider"),
        "fastembed should hit its own arm, not the fallback: {msg}"
    );
}

#[test]
fn get_provider_openai_arm_returns_feature_specific_error() {
    let msg = provider_error("openai");
    assert!(
        msg.contains("embed-openai feature not enabled"),
        "expected feature-specific error for openai arm, got: {msg}"
    );
    assert!(
        !msg.contains("unknown embedding provider"),
        "openai should hit its own arm, not the fallback: {msg}"
    );
}

#[test]
fn get_provider_voyage_arm_returns_feature_specific_error() {
    let msg = provider_error("voyage");
    assert!(
        msg.contains("embed-voyage feature not enabled"),
        "expected feature-specific error for voyage arm, got: {msg}"
    );
    assert!(
        !msg.contains("unknown embedding provider"),
        "voyage should hit its own arm, not the fallback: {msg}"
    );
}

#[test]
fn get_provider_unknown_name_returns_fallback_error() {
    let msg = provider_error("definitely-not-a-real-provider");
    assert!(
        msg.contains("unknown embedding provider"),
        "fallback arm must report unknown provider, got: {msg}"
    );
}

#[test]
fn get_provider_unknown_with_model_returns_fallback_error() {
    let msg = provider_error("not-a-provider:some-model");
    assert!(
        msg.contains("unknown embedding provider"),
        "model qualifier must not affect fallback path, got: {msg}"
    );
}
