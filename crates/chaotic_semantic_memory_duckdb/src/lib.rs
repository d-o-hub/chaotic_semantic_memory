pub mod connection;
pub mod error;
pub mod ingest_bench;
pub mod ingest_export;
pub mod ingest_libsql;
pub mod schema;
pub mod stats;

#[cfg(feature = "parquet")]
mod export_all;
#[cfg(feature = "parquet")]
pub mod export_parquet;
#[cfg(feature = "parquet")]
pub mod manifest;

pub use connection::Analytics;
pub use error::{AnalyticsError, Result};
pub use ingest_export::IngestReport;
pub use stats::{BenchmarkSummary, ConceptSummary};

#[cfg(feature = "parquet")]
pub use export_parquet::{BundleReport, ExportReport, ParquetCompression, ParquetExportOptions};
#[cfg(feature = "parquet")]
pub use manifest::{ExportManifest, FileInfo};

#[cfg(feature = "cli")]
pub mod cli;

use base64::Engine as _;

impl Analytics {
    /// Run an arbitrary read-only SELECT query.
    /// Returns data as JSON rows for cross-crate compatibility (no DuckDB/Arrow types in API).
    pub fn query(&self, sql: &str) -> Result<Vec<serde_json::Value>> {
        let sql_lower = sql.trim().to_lowercase();
        if !sql_lower.starts_with("select") && !sql_lower.starts_with("with") {
            return Err(AnalyticsError::InvalidInput(
                "Only SELECT queries are allowed".to_string(),
            ));
        }

        let mut stmt = self.conn.prepare(sql)?;
        let mut rows = stmt.query([])?;

        let mut results = Vec::new();
        let mut column_names = Vec::new();
        let mut column_count = 0;
        let mut meta_done = false;

        while let Some(row) = rows.next()? {
            if !meta_done {
                // Access Statement from Row via AsRef to get column metadata.
                // This ensures the statement is executed before accessing metadata.
                let stmt_ref: &duckdb::Statement = row.as_ref();
                column_names = stmt_ref.column_names();
                column_count = column_names.len();
                meta_done = true;
            }

            let mut map = serde_json::Map::new();
            for (i, name) in column_names.iter().enumerate().take(column_count) {
                let val = match row.get_ref(i)? {
                    duckdb::types::ValueRef::Null => serde_json::Value::Null,
                    duckdb::types::ValueRef::Boolean(b) => serde_json::Value::Bool(b),
                    duckdb::types::ValueRef::TinyInt(n) => serde_json::Value::Number(n.into()),
                    duckdb::types::ValueRef::SmallInt(n) => serde_json::Value::Number(n.into()),
                    duckdb::types::ValueRef::Int(n) => serde_json::Value::Number(n.into()),
                    duckdb::types::ValueRef::BigInt(n) => serde_json::Value::Number(n.into()),
                    duckdb::types::ValueRef::Float(n) => serde_json::Number::from_f64(n as f64)
                        .map(serde_json::Value::Number)
                        .unwrap_or(serde_json::Value::Null),
                    duckdb::types::ValueRef::Double(n) => serde_json::Number::from_f64(n)
                        .map(serde_json::Value::Number)
                        .unwrap_or(serde_json::Value::Null),
                    duckdb::types::ValueRef::Text(s) => {
                        serde_json::Value::String(String::from_utf8_lossy(s).into_owned())
                    }
                    duckdb::types::ValueRef::Blob(b) => serde_json::Value::String(
                        base64::engine::general_purpose::STANDARD.encode(b),
                    ),
                    _ => serde_json::Value::Null,
                };
                map.insert(name.clone(), val);
            }
            results.push(serde_json::Value::Object(map));
        }
        Ok(results)
    }
}

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::SCHEMA_DDL;
    use duckdb::Connection;

    #[test]
    fn test_query_validation() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA_DDL).unwrap();
        let analytics = Analytics { conn };

        assert!(analytics.query("SELECT 1").is_ok());
        assert!(
            analytics
                .query("WITH t AS (SELECT 1) SELECT * FROM t")
                .is_ok()
        );
        assert!(analytics.query("DROP TABLE concepts").is_err());
        assert!(
            analytics
                .query("INSERT INTO concepts DEFAULT VALUES")
                .is_err()
        );
    }

    #[test]
    fn test_query_result_mapping() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA_DDL).unwrap();
        conn.execute(
            "INSERT INTO concepts (id, namespace) VALUES ('c1', 'ns1')",
            [],
        )
        .unwrap();
        let analytics = Analytics { conn };

        let rows = analytics
            .query("SELECT id, namespace FROM concepts")
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["id"], "c1");
        assert_eq!(rows[0]["namespace"], "ns1");
    }
}
