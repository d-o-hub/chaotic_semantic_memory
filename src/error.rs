//! Error types for chaotic semantic memory

use thiserror::Error;

#[derive(Error, Debug)]
pub enum MemoryError {
    #[error(
        "Database error: {message}. Hint: check database connection and ensure migrations are applied"
    )]
    Database {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Invalid input for '{field}': {reason}. Hint: verify the value meets constraints")]
    InvalidInput { field: String, reason: String },

    #[error(
        "Invalid vector dimension: expected {expected}, got {actual}. Hint: use HVec::random() or ensure vector matches 10240-bit dimension"
    )]
    InvalidDimension { expected: usize, actual: usize },

    #[error("{entity} not found: '{id}'. Hint: verify the ID exists or create the entity first")]
    NotFound { entity: String, id: String },

    #[error("Unsupported operation: {0}. Hint: check feature flags or use an alternative method")]
    UnsupportedOperation(String),

    #[error(
        "Reservoir error: {message}. Hint: ensure reservoir is initialized with valid parameters"
    )]
    Reservoir {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error(
        "Persistence error: {0}. Hint: enable 'persistence' feature and check file permissions"
    )]
    Persistence(String),

    #[error("IO error: {0}. Hint: verify file paths exist and have correct permissions")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}. Hint: ensure data is valid JSON and types match schema")]
    Serialization(#[from] serde_json::Error),
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
