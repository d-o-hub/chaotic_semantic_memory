pub mod connection;
pub mod error;
pub mod ingest_bench;
pub mod ingest_export;
pub mod ingest_libsql;
pub mod schema;
pub mod stats;

pub use connection::Analytics;
pub use error::{AnalyticsError, Result};
pub use ingest_export::IngestReport;
pub use stats::{BenchmarkSummary, ConceptSummary};

impl Analytics {
    /// Run an arbitrary read-only SELECT query and return all record batches.
    /// Returns arrow RecordBatches.
    pub fn query(&self, sql: &str) -> Result<Vec<duckdb::arrow::array::RecordBatch>> {
        let mut stmt = self.conn.prepare(sql)?;
        let batches = stmt.query_arrow([])?;
        let result: std::result::Result<Vec<_>, _> = batches.collect();
        Ok(result?)
    }
}

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
