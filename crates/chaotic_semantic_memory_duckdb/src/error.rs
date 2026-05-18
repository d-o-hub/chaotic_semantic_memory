use thiserror::Error;

#[derive(Debug, Error)]
pub enum AnalyticsError {
    #[error("DuckDB error: {0}")]
    DuckDb(#[from] duckdb::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, AnalyticsError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversions() {
        let io_err = std::io::Error::new(std::io::ErrorKind::Other, "test");
        let err: AnalyticsError = io_err.into();
        assert!(matches!(err, AnalyticsError::Io(_)));

        let anyhow_err = anyhow::anyhow!("test");
        let err: AnalyticsError = anyhow_err.into();
        assert!(matches!(err, AnalyticsError::Anyhow(_)));
    }
}
