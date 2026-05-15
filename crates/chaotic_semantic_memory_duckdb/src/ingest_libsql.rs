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
            &format!(
                "ATTACH '{}' AS csm_src (TYPE SQLITE, READ_ONLY)",
                escaped_path
            ),
            [],
        )?;

        // Copy concepts
        let res = self.conn.execute(
            "INSERT OR REPLACE INTO concepts (id, namespace, created_at_us, updated_at_us, expires_at_us, metadata_json)
             SELECT id, 'default', created_at, modified_at, expires_at, metadata FROM csm_src.concepts",
            [],
        );

        if let Err(e) = res {
            let _ = self.conn.execute("DETACH csm_src", []);
            return Err(e.into());
        }
        let concepts_loaded = self.conn.changes();

        // Copy associations
        let res = self.conn.execute(
            "INSERT OR REPLACE INTO associations (src_id, dst_id, strength)
             SELECT from_id, to_id, strength FROM csm_src.associations",
            [],
        );

        if let Err(e) = res {
            let _ = self.conn.execute("DETACH csm_src", []);
            return Err(e.into());
        }
        let associations_loaded = self.conn.changes();

        self.conn.execute("DETACH csm_src", [])?;

        Ok(IngestReport {
            concepts_loaded,
            associations_loaded,
            ..Default::default()
        })
    }
}
