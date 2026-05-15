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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_in_memory() {
        let analytics = Analytics::open_in_memory();
        assert!(analytics.is_ok());
    }

    #[test]
    fn test_open_file() {
        let temp = ::tempfile::NamedTempFile::new().unwrap();
        let analytics = Analytics::open(temp.path());
        assert!(analytics.is_ok());
    }
}
