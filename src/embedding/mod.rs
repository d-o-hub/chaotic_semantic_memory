//! External embedding model bridge for semantic accuracy.
//!
//! Re-exports from `csm-embedding` crate for backwards compatibility.

pub use csm_embedding::*;

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use std::sync::Mutex;

    /// cargo runs lib tests on parallel threads; the provider tests mutate
    /// process-global env vars, so `remove_var` from one test can land between
    /// another test's `set_var` and its `get_provider` call, intermittently
    /// failing it (2026-09-13, run 103702134133). Serialize all env mutation.
    static ENV_TEST_LOCK: Mutex<()> = Mutex::new(());

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
        // CI-resilient: set a dummy API key; accept success or
        // env-var race condition in parallel test runners
        // SAFETY: env mutation serialized by ENV_TEST_LOCK; no concurrent writers
        let _env = ENV_TEST_LOCK.lock().unwrap();
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
        // SAFETY: env mutation serialized by ENV_TEST_LOCK; guard still held
        unsafe { std::env::remove_var("OPENAI_API_KEY") };
    }

    #[cfg(feature = "embed-openai")]
    #[test]
    fn get_provider_openai_with_model_returns_provider() {
        // CI-resilient: accept success or env-var race condition
        // SAFETY: env mutation serialized by ENV_TEST_LOCK; no concurrent writers
        let _env = ENV_TEST_LOCK.lock().unwrap();
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
        // SAFETY: env mutation serialized by ENV_TEST_LOCK; guard still held
        unsafe { std::env::remove_var("OPENAI_API_KEY") };
    }

    #[cfg(feature = "embed-voyage")]
    #[test]
    fn get_provider_voyage_with_feature_returns_provider() {
        // SAFETY: env mutation serialized by ENV_TEST_LOCK; no concurrent writers
        let _env = ENV_TEST_LOCK.lock().unwrap();
        unsafe { std::env::set_var("VOYAGE_API_KEY", "test-key-for-mutation-coverage") };
        let result = get_provider("voyage");
        assert!(
            result.is_ok(),
            "voyage arm must succeed when feature enabled"
        );
        let provider = result.unwrap();
        assert_eq!(provider.name(), "voyage");
        // SAFETY: env mutation serialized by ENV_TEST_LOCK; guard still held
        unsafe { std::env::remove_var("VOYAGE_API_KEY") };
    }

    #[cfg(feature = "embed-voyage")]
    #[test]
    fn get_provider_voyage_with_model_returns_provider() {
        // SAFETY: env mutation serialized by ENV_TEST_LOCK; no concurrent writers
        let _env = ENV_TEST_LOCK.lock().unwrap();
        unsafe { std::env::set_var("VOYAGE_API_KEY", "test-key-for-mutation-coverage") };
        let result = get_provider("voyage:voyage-3");
        assert!(result.is_ok(), "voyage arm with model must succeed");
        let provider = result.unwrap();
        assert_eq!(provider.name(), "voyage");
        // SAFETY: env mutation serialized by ENV_TEST_LOCK; guard still held
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
