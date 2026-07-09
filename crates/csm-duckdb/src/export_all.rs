use crate::connection::Analytics;
use crate::error::Result;
use crate::export_parquet::{BundleReport, ParquetExportOptions};
use std::collections::BTreeMap;
use std::path::Path;

impl Analytics {
    /// Convenience: writes all four datasets into a directory and emits a manifest.json.
    pub fn export_all_parquet<P: AsRef<Path>>(
        &self,
        out_dir: P,
        opts: &ParquetExportOptions,
    ) -> Result<BundleReport> {
        let out_dir = out_dir.as_ref();
        if out_dir.exists() && !out_dir.is_dir() {
            return Err(crate::error::AnalyticsError::InvalidInput(
                "Output path exists and is not a directory".to_string(),
            ));
        }
        if !out_dir.exists() {
            std::fs::create_dir_all(out_dir)?;
        }

        let concepts = self.export_concepts_parquet(out_dir.join("concepts.parquet"), opts)?;
        let associations =
            self.export_associations_parquet(out_dir.join("associations.parquet"), opts)?;
        let versions = self.export_versions_parquet(out_dir.join("versions.parquet"), opts)?;
        let benchmarks =
            self.export_benchmarks_parquet(out_dir.join("benchmarks.parquet"), opts)?;

        let manifest_path = if opts.include_manifest {
            let mut reports = BTreeMap::new();
            // We use the file name as the key in the manifest
            reports.insert("concepts.parquet".to_string(), concepts.clone());
            reports.insert("associations.parquet".to_string(), associations.clone());
            reports.insert("versions.parquet".to_string(), versions.clone());
            reports.insert("benchmarks.parquet".to_string(), benchmarks.clone());

            let manifest = self.create_manifest(reports, opts.clone())?;
            let path = out_dir.join("manifest.json");
            let file = std::fs::File::create(&path)?;
            serde_json::to_writer_pretty(file, &manifest)?;
            Some(path)
        } else {
            None
        };

        Ok(BundleReport {
            concepts,
            associations,
            versions,
            benchmarks,
            manifest_path,
        })
    }
}
