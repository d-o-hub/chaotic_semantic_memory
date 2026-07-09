use crate::Analytics;
use crate::error::Result;
use crate::ingest_export::IngestReport;
use std::path::Path;

impl Analytics {
    pub fn attach_libsql<P: AsRef<Path>>(&mut self, path: P) -> Result<IngestReport> {
        let path_str = path.as_ref().to_str().ok_or_else(|| {
            crate::error::AnalyticsError::InvalidInput("Invalid path for libsql file".to_string())
        })?;

        // DuckDB SQLite scanner ATTACH
        // We use single quotes for the path; if the path contains single quotes, we escape them.
        let escaped_path = path_str.replace('\'', "''");
        self.conn.execute(
            &format!("ATTACH '{escaped_path}' AS csm_src (TYPE SQLITE, READ_ONLY)"),
            [],
        )?;

        // Copy concepts
        let concepts_loaded = match self.conn.execute(
            "INSERT OR REPLACE INTO concepts (id, namespace, created_at_us, updated_at_us, expires_at_us, metadata_json)
             SELECT id, 'default', created_at, modified_at, expires_at, metadata FROM csm_src.concepts",
            [],
        ) {
            Ok(n) => n,
            Err(e) => {
                let _ = self.conn.execute("DETACH csm_src", []);
                return Err(e.into());
            }
        };

        // Copy associations
        let associations_loaded = match self.conn.execute(
            "INSERT OR REPLACE INTO associations (src_id, dst_id, strength)
             SELECT from_id, to_id, strength FROM csm_src.associations",
            [],
        ) {
            Ok(n) => n,
            Err(e) => {
                let _ = self.conn.execute("DETACH csm_src", []);
                return Err(e.into());
            }
        };

        self.conn.execute("DETACH csm_src", [])?;

        Ok(IngestReport {
            concepts_loaded,
            associations_loaded,
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::schema::SCHEMA_DDL;
    use duckdb::Connection;

    #[test]
    fn test_attach_libsql_missing() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA_DDL).unwrap();
        let mut analytics = Analytics { conn };

        let res = analytics.attach_libsql("nonexistent.db");
        assert!(res.is_err());
    }
}
