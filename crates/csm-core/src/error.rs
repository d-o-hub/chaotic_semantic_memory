//! Error types for chaotic semantic memory

use thiserror::Error;

#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("Database error: {message}")]
    Database {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Invalid input for '{field}': {reason}")]
    InvalidInput { field: String, reason: String },

    #[error("Invalid vector dimension: expected {expected}, got {actual}")]
    InvalidDimension { expected: usize, actual: usize },

    #[error("{entity} not found: '{id}'")]
    NotFound { entity: String, id: String },

    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),

    #[error("Reservoir error: {message}")]
    Reservoir {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Persistence error: {0}")]
    Persistence(String),

    #[error("External service error: {0}")]
    External(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Observability error: {0}")]
    Observability(String),

    #[error("Observability feature '{feature}' is not enabled; rebuild with --features {feature}")]
    ObservabilityFeatureDisabled { feature: &'static str },

    #[error("Observability stack already initialised in this process")]
    ObservabilityAlreadyInitialised,
}

impl MemoryError {
    pub fn database(message: impl Into<String>) -> Self {
        Self::Database {
            message: message.into(),
            source: None,
        }
    }

    pub fn database_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Database {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    pub fn reservoir(message: impl Into<String>) -> Self {
        Self::Reservoir {
            message: message.into(),
            source: None,
        }
    }

    pub fn reservoir_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Reservoir {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    pub fn remediation(&self) -> Option<&'static str> {
        match self {
            Self::Database { .. } => Some(
                "Check that the database path is accessible and the schema is up to date. Run with persistence disabled to rule out DB issues.",
            ),
            Self::InvalidInput { .. } => Some(
                "Verify the input format matches the expected type and constraints for the given field.",
            ),
            Self::InvalidDimension { .. } => Some(
                "Ensure vector dimension matches the configured size. Use FrameworkBuilder::with_reservoir_input_size() to adjust.",
            ),
            Self::NotFound { .. } => Some(
                "Check that the concept ID exists before performing this operation. Use get() or probe() to verify.",
            ),
            Self::UnsupportedOperation(_) => Some(
                "This operation is not supported in the current configuration. Check feature flags and configuration.",
            ),
            Self::Reservoir { .. } => Some(
                "Check reservoir configuration (input_size, spectral_radius). Reset with FrameworkBuilder defaults if needed.",
            ),
            Self::Persistence(_) => Some(
                "Verify database connectivity and file system permissions. Check that the database file is not corrupted.",
            ),
            Self::External(_) => {
                Some("Check the external service configuration and network connectivity.")
            }
            Self::Config(_) => Some(
                "Review configuration parameters. Use FrameworkBuilder defaults for a known-good starting point.",
            ),
            Self::Io(_) => Some("Check file system permissions and available disk space."),
            Self::Serialization(_) => Some(
                "Ensure data is valid JSON/binary format. Use export/import functions for safe serialization.",
            ),
            Self::Observability(_) => Some(
                "Check observability configuration. Ensure endpoints are reachable and feature flags are enabled.",
            ),
            Self::ObservabilityFeatureDisabled { .. } => {
                Some("Rebuild with the required feature flag enabled.")
            }
            Self::ObservabilityAlreadyInitialised => Some(
                "Observability stack can only be initialised once per process. Check for duplicate initialization.",
            ),
        }
    }
}

pub type Result<T> = std::result::Result<T, MemoryError>;

#[cfg(test)]
mod tests {
    use super::MemoryError;

    #[test]
    fn database_error_exposes_source_chain() {
        let io = std::io::Error::other("inner-io");
        let err = MemoryError::database_with_source("db failed", io);
        let source = std::error::Error::source(&err).expect("source should exist");
        assert_eq!(source.to_string(), "inner-io");
    }

    #[test]
    fn reservoir_error_exposes_source_chain() {
        let io = std::io::Error::other("inner-reservoir");
        let err = MemoryError::reservoir_with_source("reservoir failed", io);
        let source = std::error::Error::source(&err).expect("source should exist");
        assert_eq!(source.to_string(), "inner-reservoir");
    }

    #[test]
    fn remediation_returns_hints_for_all_variants() {
        let cases: Vec<(MemoryError, &str)> = vec![
            (
                MemoryError::database("db"),
                "Check that the database path is accessible",
            ),
            (
                MemoryError::InvalidInput {
                    field: "f".into(),
                    reason: "r".into(),
                },
                "Verify the input format",
            ),
            (
                MemoryError::InvalidDimension {
                    expected: 128,
                    actual: 64,
                },
                "Ensure vector dimension",
            ),
            (
                MemoryError::NotFound {
                    entity: "concept".into(),
                    id: "x".into(),
                },
                "Check that the concept ID",
            ),
            (
                MemoryError::UnsupportedOperation("op".into()),
                "This operation is not supported",
            ),
            (
                MemoryError::reservoir("res"),
                "Check reservoir configuration",
            ),
            (
                MemoryError::Persistence("p".into()),
                "Verify database connectivity",
            ),
            (
                MemoryError::External("e".into()),
                "Check the external service",
            ),
            (
                MemoryError::Config("c".into()),
                "Review configuration parameters",
            ),
            (
                MemoryError::Io(std::io::Error::other("io")),
                "Check file system permissions",
            ),
            (
                MemoryError::Serialization(serde_json::from_str::<i32>("bad").unwrap_err()),
                "Ensure data is valid JSON",
            ),
            (
                MemoryError::Observability("o".into()),
                "Check observability configuration",
            ),
            (
                MemoryError::ObservabilityFeatureDisabled { feature: "metrics" },
                "Rebuild with the required feature flag",
            ),
            (
                MemoryError::ObservabilityAlreadyInitialised,
                "Observability stack can only be initialised once",
            ),
        ];
        for (err, prefix) in cases {
            let hint = err.remediation().expect("remediation should return Some");
            assert!(
                hint.starts_with(prefix),
                "remediation hint for {:?} should start with {:?}, got {:?}",
                err,
                prefix,
                hint
            );
        }
    }
}
