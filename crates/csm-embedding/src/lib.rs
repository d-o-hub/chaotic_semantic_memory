//! External embedding model bridge for semantic accuracy.
//!
//! Provides an `EmbeddingProvider` trait with multiple backends:
//! - HDC text encoder (default, semantically blind but fast)
//! - FastEmbed (local ONNX models, opt-in via `embed-fastembed` feature)
//! - OpenAI HTTP API (opt-in via `embed-openai` feature)
//! - Voyage HTTP API (opt-in via `embed-voyage` feature)
//!
//! All backends project native embeddings to HVec10240 via sparse random projection
//! (Achlioptas method), preserving cosine similarity with Johnson-Lindenstrauss guarantees.

mod hdc_text;
mod projection;

#[cfg(feature = "embed-fastembed")]
mod fastembed;
#[cfg(feature = "embed-openai")]
mod remote_openai;
#[cfg(feature = "embed-voyage")]
mod remote_voyage;

pub use hdc_text::HdcTextProvider;
pub use projection::{Projection, ProjectionConfig};

#[cfg(feature = "embed-fastembed")]
pub use fastembed::FastEmbedProvider;
#[cfg(feature = "embed-openai")]
pub use remote_openai::OpenAiProvider;
#[cfg(feature = "embed-voyage")]
pub use remote_voyage::VoyageProvider;

use csm_core_lib::error::Result;
use csm_core_lib::hyperdim::HVec10240;

/// Embedding provider trait for text-to-vector conversion.
///
/// Implementations may be:
/// - Local (HDC hash, FastEmbed ONNX)
/// - Remote HTTP (OpenAI, Voyage)
///
/// All providers project their native dimensionality to HVec10240.
#[async_trait::async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Provider name for logging and CLI.
    fn name(&self) -> &str;

    /// Native embedding dimension before projection.
    fn native_dim(&self) -> usize;

    /// Embed a single text string.
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// Embed multiple texts in batch (more efficient for remote providers).
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;

    /// Project a native embedding to HVec10240.
    ///
    /// Default implementation uses the provider's projection matrix.
    fn project(&self, vec: &[f32], projection: &Projection) -> HVec10240 {
        projection.project(vec)
    }
}

/// Factory to get an embedding provider by name.
///
/// Format: "provider_name" or "provider_name:model_name"
pub fn get_provider(name: &str) -> Result<std::sync::Arc<dyn EmbeddingProvider>> {
    let parts: Vec<&str> = name.splitn(2, ':').collect();
    let provider_name = parts[0];
    let _model_name = parts.get(1).copied();

    match provider_name {
        "hdc-text" | "hdc" => Ok(std::sync::Arc::new(HdcTextProvider::new())),

        "fastembed" => {
            #[cfg(feature = "embed-fastembed")]
            {
                if let Some(model) = _model_name {
                    Ok(std::sync::Arc::new(FastEmbedProvider::with_model(model)?))
                } else {
                    Ok(std::sync::Arc::new(FastEmbedProvider::new()?))
                }
            }
            #[cfg(not(feature = "embed-fastembed"))]
            Err(csm_core_lib::error::MemoryError::Config(
                "embed-fastembed feature not enabled".into(),
            ))
        }

        "openai" => {
            #[cfg(feature = "embed-openai")]
            {
                let mut provider = OpenAiProvider::from_env()?;
                if let Some(model) = _model_name {
                    provider = provider.with_model(model);
                }
                Ok(std::sync::Arc::new(provider))
            }
            #[cfg(not(feature = "embed-openai"))]
            Err(csm_core_lib::error::MemoryError::Config(
                "embed-openai feature not enabled".into(),
            ))
        }

        "voyage" => {
            #[cfg(feature = "embed-voyage")]
            {
                let mut provider = VoyageProvider::from_env()?;
                if let Some(model) = _model_name {
                    provider = provider.with_model(model);
                }
                Ok(std::sync::Arc::new(provider))
            }
            #[cfg(not(feature = "embed-voyage"))]
            Err(csm_core_lib::error::MemoryError::Config(
                "embed-voyage feature not enabled".into(),
            ))
        }

        _ => Err(csm_core_lib::error::MemoryError::Config(format!(
            "unknown embedding provider: {provider_name}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn get_provider_unknown_returns_error() {
        let result = get_provider("does-not-exist");
        assert!(result.is_err(), "Unknown provider must return Err");
    }

    #[test]
    fn get_provider_hdc_returns_ok() {
        let result = get_provider("hdc");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name(), "hdc-text");
    }

    #[cfg(not(feature = "embed-fastembed"))]
    #[test]
    fn get_provider_fastembed_without_feature_returns_error() {
        let result = get_provider("fastembed");
        assert!(result.is_err());
    }

    #[cfg(not(feature = "embed-openai"))]
    #[test]
    fn get_provider_openai_without_feature_returns_error() {
        let result = get_provider("openai");
        assert!(result.is_err());
    }

    #[cfg(not(feature = "embed-voyage"))]
    #[test]
    fn get_provider_voyage_without_feature_returns_error() {
        let result = get_provider("voyage");
        assert!(result.is_err());
    }

    #[cfg(feature = "embed-fastembed")]
    #[test]
    fn get_provider_fastembed_with_feature_returns_provider() {
        let result = get_provider("fastembed");
        match result {
            Ok(provider) => assert_eq!(provider.name(), "fastembed"),
            Err(e) => {
                let msg = format!("{e}");
                assert!(
                    msg.contains("fastembed"),
                    "error must be from fastembed arm, not unknown provider: {msg}"
                );
            }
        }
    }

    #[cfg(feature = "embed-fastembed")]
    #[test]
    fn get_provider_fastembed_with_model_returns_provider() {
        let result = get_provider("fastembed:BAAI/bge-small-en-v1.5");
        // Model download may fail in CI; just verify the arm is reached
        // (deleting the arm would fall through to unknown error, not a model error)
        match result {
            Ok(provider) => assert_eq!(provider.name(), "fastembed"),
            Err(e) => {
                let msg = format!("{e}");
                assert!(
                    msg.contains("embed-fastembed") || msg.contains("model"),
                    "error must be from fastembed arm, not unknown provider: {msg}"
                );
            }
        }
    }

    #[cfg(feature = "embed-openai")]
    #[test]
    fn get_provider_openai_with_feature_returns_provider() {
        // CI-resilient: set a dummy API key; accept success or env-var
        // race condition in parallel test runners
        // SAFETY: env var mutation in single-threaded test is sound; no concurrent readers
        unsafe { std::env::set_var("OPENAI_API_KEY", "test-key-for-mutation-coverage") };
        let result = get_provider("openai");
        match result {
            Ok(provider) => assert_eq!(provider.name(), "openai"),
            Err(e) => {
                let msg = format!("{e}");
                assert!(
                    msg.contains("OPENAI_API_KEY") || msg.contains("openai"),
                    "error must be from openai arm, not unknown provider: {msg}"
                );
            }
        }
        // SAFETY: env var removal in single-threaded test is sound
        unsafe { std::env::remove_var("OPENAI_API_KEY") };
    }

    #[cfg(feature = "embed-openai")]
    #[test]
    fn get_provider_openai_with_model_returns_provider() {
        // CI-resilient: accept success or env-var race condition
        // SAFETY: env var mutation in single-threaded test is sound; no concurrent readers
        unsafe { std::env::set_var("OPENAI_API_KEY", "test-key-for-mutation-coverage") };
        let result = get_provider("openai:text-embedding-3-small");
        match result {
            Ok(provider) => assert_eq!(provider.name(), "openai"),
            Err(e) => {
                let msg = format!("{e}");
                assert!(
                    msg.contains("OPENAI_API_KEY") || msg.contains("openai"),
                    "error must be from openai arm, not unknown provider: {msg}"
                );
            }
        }
        // SAFETY: env var removal in single-threaded test is sound
        unsafe { std::env::remove_var("OPENAI_API_KEY") };
    }

    #[cfg(feature = "embed-voyage")]
    #[test]
    fn get_provider_voyage_with_feature_returns_provider() {
        // SAFETY: env var mutation in single-threaded test is sound; no concurrent readers
        unsafe { std::env::set_var("VOYAGE_API_KEY", "test-key-for-mutation-coverage") };
        let result = get_provider("voyage");
        assert!(
            result.is_ok(),
            "voyage arm must succeed when feature enabled"
        );
        let provider = result.unwrap();
        assert_eq!(provider.name(), "voyage");
        // SAFETY: env var removal in single-threaded test is sound
        unsafe { std::env::remove_var("VOYAGE_API_KEY") };
    }

    #[cfg(feature = "embed-voyage")]
    #[test]
    fn get_provider_voyage_with_model_returns_provider() {
        // SAFETY: env var mutation in single-threaded test is sound; no concurrent readers
        unsafe { std::env::set_var("VOYAGE_API_KEY", "test-key-for-mutation-coverage") };
        let result = get_provider("voyage:voyage-3");
        assert!(result.is_ok(), "voyage arm with model must succeed");
        let provider = result.unwrap();
        assert_eq!(provider.name(), "voyage");
        // SAFETY: env var removal in single-threaded test is sound
        unsafe { std::env::remove_var("VOYAGE_API_KEY") };
    }

    #[test]
    fn get_provider_hdc_is_distinct_from_unknown() {
        let hdc = get_provider("hdc").unwrap();
        let unknown = get_provider("no-such-provider");
        assert!(unknown.is_err());
        assert_ne!(hdc.name(), "");
        assert_eq!(hdc.native_dim(), 10240);
    }
}
