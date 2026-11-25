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
            generator: format!("csm-duckdb {}", env!("CARGO_PKG_VERSION")),
            // All workspace members carry one version (version-integrity
            // workflow), so this member's package version is the core
            // crate's version — no qualifier needed.
            core_crate_version: env!("CARGO_PKG_VERSION").to_string(),
            run_id: uuid::Uuid::new_v4().to_string(),
            exported_at,
            files,
            options: opts,
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::{ExportReport, ParquetExportOptions};
    use std::path::PathBuf;

    // Kills every "return early / copy nothing" mutant of create_manifest by
    // asserting the concrete contract: the manifest maps every report exactly
    // and labels generator/core versions honestly (an equality-only check
    // would pass against any uniformly-defaulted return).
    #[test]
    fn create_manifest_maps_reports_and_labels_versions_honestly() {
        let analytics = Analytics::open_in_memory().expect("in-memory analytics");

        let opts = ParquetExportOptions::default();
        let mut reports = BTreeMap::new();
        reports.insert(
            "concepts".to_string(),
            ExportReport {
                rows_written: 12,
                bytes_written: 3456,
                path: PathBuf::from("concepts.parquet"),
                sha256: "sha-concepts".to_string(),
            },
        );
        reports.insert(
            "associations".to_string(),
            ExportReport {
                rows_written: 7,
                bytes_written: 890,
                path: PathBuf::from("associations.parquet"),
                sha256: "sha-assoc".to_string(),
            },
        );

        let manifest = analytics
            .create_manifest(reports, opts.clone())
            .expect("create_manifest succeeds");

        assert_eq!(manifest.schema_version, 1, "manifest schema version is 1");
        assert_eq!(
            manifest.generator,
            format!("csm-duckdb {}", env!("CARGO_PKG_VERSION")),
            "generator names this crate plus its version"
        );
        assert_eq!(
            manifest.core_crate_version,
            env!("CARGO_PKG_VERSION"),
            "core crate version is the workspace-shared version, unqualified"
        );
        assert!(
            manifest.run_id.len() == 36 && manifest.run_id.split('-').count() == 5,
            "run_id is a UUID string, got: {}",
            manifest.run_id
        );
        assert_eq!(
            manifest.exported_at.len(),
            20,
            "exported_at is a %Y-%m-%dT%H:%M:%SZ timestamp, got: {}",
            manifest.exported_at
        );
        assert!(
            manifest.exported_at.ends_with('Z'),
            "exported_at is UTC: {}",
            manifest.exported_at
        );

        let names: Vec<&String> = manifest.files.keys().collect();
        assert_eq!(
            names,
            vec![&"associations".to_string(), &"concepts".to_string()],
            "files map is sorted and holds every report"
        );
        let concepts = manifest
            .files
            .get("concepts")
            .expect("concepts report is mapped");
        assert_eq!(concepts.rows, 12);
        assert_eq!(concepts.bytes, 3456);
        assert_eq!(concepts.sha256, "sha-concepts");
        let associations = manifest
            .files
            .get("associations")
            .expect("associations report is mapped");
        assert_eq!(associations.rows, 7);
        assert_eq!(associations.bytes, 890);
        assert_eq!(associations.sha256, "sha-assoc");

        assert_eq!(manifest.options.row_group_size, opts.row_group_size);
        assert!(manifest.options.partition_by.is_none());
        assert_eq!(manifest.options.include_manifest, opts.include_manifest);
    }
}
