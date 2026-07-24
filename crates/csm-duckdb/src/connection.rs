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

pub(crate) fn validate_analytics_path(path: &Path, expected_extensions: &[&str]) -> Result<()> {
    let path_str = path.to_str().ok_or_else(|| {
        crate::error::AnalyticsError::InvalidInput("Path must be valid UTF-8".to_string())
    })?;

    if path_str.chars().count() > MAX_ANALYTICS_PATH_LENGTH {
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

    if !expected_extensions.is_empty() {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let ext_lower = ext.to_lowercase();
        if !expected_extensions.contains(&ext_lower.as_str()) {
            return Err(crate::error::AnalyticsError::InvalidInput(format!(
                "Invalid file extension. Expected one of: {:?}",
                expected_extensions
            )));
        }
    }

    if path.is_absolute() {
        let current_dir = std::env::current_dir().map_err(|e| {
            crate::error::AnalyticsError::InvalidInput(format!(
                "cannot determine current working directory: {e}"
            ))
        })?;

        let normalized = if path.exists() {
            path.canonicalize().map_err(|e| {
                crate::error::AnalyticsError::InvalidInput(format!(
                    "absolute path cannot be accessed: {e}"
                ))
            })?
        } else {
            if let Some(parent) = path.parent() {
                if parent.exists() {
                    let parent_normalized = parent.canonicalize().map_err(|e| {
                        crate::error::AnalyticsError::InvalidInput(format!(
                            "absolute path parent cannot be accessed: {e}"
                        ))
                    })?;
                    if let Some(file_name) = path.file_name() {
                        parent_normalized.join(file_name)
                    } else {
                        parent_normalized
                    }
                } else {
                    path.to_path_buf()
                }
            } else {
                path.to_path_buf()
            }
        };

        let temp_dir = std::env::temp_dir();
        let temp_dir_normalized = temp_dir.canonicalize().unwrap_or(temp_dir);

        let is_in_cwd = normalized.starts_with(&current_dir);
        let is_in_temp =
            normalized.starts_with(&temp_dir_normalized) || normalized.starts_with("/tmp");

        if !is_in_cwd && !is_in_temp {
            return Err(crate::error::AnalyticsError::InvalidInput(
                "absolute paths must be within current working directory or temporary directory"
                    .to_string(),
            ));
        }
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
}
