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
}
