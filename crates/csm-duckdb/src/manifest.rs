use crate::connection::Analytics;
use crate::error::Result;
use crate::export_parquet::{ExportReport, ParquetExportOptions};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Debug)]
pub struct FileInfo {
    pub rows: u64,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportManifest {
    pub schema_version: u32,
    pub generator: String,
    pub core_crate_version: String,
    pub run_id: String,
    pub exported_at: String,
    pub files: BTreeMap<String, FileInfo>,
    pub options: ParquetExportOptions,
}

impl Analytics {
    pub fn create_manifest(
        &self,
        reports: BTreeMap<String, ExportReport>,
        opts: ParquetExportOptions,
    ) -> Result<ExportManifest> {
        let mut files = BTreeMap::new();
        for (name, report) in reports {
            files.insert(
                name,
                FileInfo {
                    rows: report.rows_written,
                    bytes: report.bytes_written,
                    sha256: report.sha256,
                },
            );
        }

        let exported_at: String = self.conn.query_row(
            "SELECT strftime(now()::TIMESTAMP, '%Y-%m-%dT%H:%M:%SZ')",
            [],
            |row| row.get(0),
        )?;

        Ok(ExportManifest {
            schema_version: 1,
            generator: format!(
                "csm-duckdb {}",
                env!("CARGO_PKG_VERSION")
            ),
            core_crate_version: format!("{} (core stub)", env!("CARGO_PKG_VERSION")),
            run_id: uuid::Uuid::new_v4().to_string(),
            exported_at,
            files,
            options: opts,
        })
    }
}
