use crate::error::Result;
use crate::schema::SCHEMA_DDL;
use duckdb::Connection;
use std::path::Path;

pub struct Analytics {
    pub conn: Connection,
}

impl Analytics {
    /// In-memory analytics database.
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let mut slf = Self { conn };
        slf.init_schema()?;
        Ok(slf)
    }

    /// File-backed analytics database (DuckDB native format).
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        let mut slf = Self { conn };
        slf.init_schema()?;
        Ok(slf)
    }

    fn init_schema(&mut self) -> Result<()> {
        self.conn.execute_batch(SCHEMA_DDL)?;
        Ok(())
    }
}

pub(crate) const MAX_ANALYTICS_PATH_LENGTH: usize = 4096;

pub(crate) fn validate_analytics_path(path: &Path) -> Result<()> {
    let path_str = path.to_str().ok_or_else(|| {
        crate::error::AnalyticsError::InvalidInput("Path must be valid UTF-8".to_string())
    })?;

    if path_str.len() > MAX_ANALYTICS_PATH_LENGTH {
        return Err(crate::error::AnalyticsError::InvalidInput(format!(
            "Path exceeds maximum length of {MAX_ANALYTICS_PATH_LENGTH} characters"
        )));
    }

    if path
        .components()
        .any(|c| c == std::path::Component::ParentDir)
    {
        return Err(crate::error::AnalyticsError::InvalidInput(
            "Path traversal '..' components are not allowed".to_string(),
        ));
    }

    if path_str
        .chars()
        .any(|c| c.is_control() || c == ';' || c == '\'' || c == '"')
    {
        return Err(crate::error::AnalyticsError::InvalidInput(
            "Path must not contain control characters, semicolons, or quotes".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn test_open_in_memory() {
        let analytics = Analytics::open_in_memory();
        assert!(analytics.is_ok());
    }

    #[test]
    fn test_open_file() {
        let temp_dir = ::tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let analytics = Analytics::open(&db_path);
        assert!(analytics.is_ok());
    }

    #[test]
    fn test_path_length_limit() {
        let at_limit: String = "a".repeat(MAX_ANALYTICS_PATH_LENGTH);
        assert!(validate_analytics_path(Path::new(&at_limit)).is_ok());
        let over_limit: String = "a".repeat(MAX_ANALYTICS_PATH_LENGTH + 1);
        assert!(validate_analytics_path(Path::new(&over_limit)).is_err());
    }
}
