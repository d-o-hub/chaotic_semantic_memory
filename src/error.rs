//! Error types for chaotic semantic memory

use thiserror::Error;

#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Invalid input for '{field}': {reason}")]
    InvalidInput { field: String, reason: String },

    #[error("Invalid vector dimension: expected {expected}, got {actual}")]
    InvalidDimension { expected: usize, actual: usize },

    #[error("{entity} not found: '{id}'")]
    NotFound { entity: String, id: String },

    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),

    #[error("Reservoir error: {0}")]
    Reservoir(String),

    #[error("Persistence error: {0}")]
    Persistence(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, MemoryError>;
