use crate::connection::{Analytics, validate_analytics_path};
use crate::error::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
pub enum ParquetCompression {
    #[default]
    Zstd,
    Snappy,
    None,
}

impl std::fmt::Display for ParquetCompression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Zstd => write!(f, "ZSTD"),
            Self::Snappy => write!(f, "SNAPPY"),
            Self::None => write!(f, "UNCOMPRESSED"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParquetExportOptions {
    pub compression: ParquetCompression,
    pub row_group_size: usize,
    pub partition_by: Option<String>,
    pub include_manifest: bool,
}

impl Default for ParquetExportOptions {
    fn default() -> Self {
        Self {
            compression: ParquetCompression::Zstd,
            row_group_size: 122_880,
            partition_by: None,
            include_manifest: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ExportReport {
    pub rows_written: u64,
    pub bytes_written: u64,
    pub path: PathBuf,
    pub sha256: String,
}

pub struct BundleReport {
    pub concepts: ExportReport,
    pub associations: ExportReport,
    pub versions: ExportReport,
    pub benchmarks: ExportReport,
    pub manifest_path: Option<PathBuf>,
}

impl Analytics {
    pub fn export_concepts_parquet<P: AsRef<Path>>(
        &self,
        out_path: P,
        opts: &ParquetExportOptions,
    ) -> Result<ExportReport> {
        let sql = "SELECT id, text, namespace, created_at_us, updated_at_us, expires_at_us, metadata_json FROM concepts ORDER BY id".to_string();
        self.export_parquet_internal(sql, out_path, opts)
    }

    pub fn export_associations_parquet<P: AsRef<Path>>(
        &self,
        out_path: P,
        opts: &ParquetExportOptions,
    ) -> Result<ExportReport> {
        let sql =
            "SELECT src_id, dst_id, strength FROM associations ORDER BY src_id, dst_id".to_string();
        self.export_parquet_internal(sql, out_path, opts)
    }

    pub fn export_versions_parquet<P: AsRef<Path>>(
        &self,
        out_path: P,
        opts: &ParquetExportOptions,
    ) -> Result<ExportReport> {
        let sql = "SELECT id, version, text, created_us FROM concept_versions ORDER BY id, version"
            .to_string();
        self.export_parquet_internal(sql, out_path, opts)
    }

    pub fn export_benchmarks_parquet<P: AsRef<Path>>(
        &self,
        out_path: P,
        opts: &ParquetExportOptions,
    ) -> Result<ExportReport> {
        let sql = "SELECT suite, name, commit, run_at_us, p50_us, p95_us, p99_us, extras FROM benchmarks ORDER BY suite, name, run_at_us".to_string();
        self.export_parquet_internal(sql, out_path, opts)
    }

    fn export_parquet_internal<P: AsRef<Path>>(
        &self,
        query: String,
        out_path: P,
        opts: &ParquetExportOptions,
    ) -> Result<ExportReport> {
        let out_path = out_path.as_ref();
        validate_analytics_path(out_path, &["parquet"])?;

        let out_path_str = out_path
            .to_str()
            .ok_or_else(|| {
                crate::error::AnalyticsError::InvalidInput("Path must be valid UTF-8".to_string())
            })?
            .replace("'", "''");

        let mut copy_opts = vec![
            "FORMAT PARQUET".to_string(),
            format!("COMPRESSION {}", opts.compression),
            format!("ROW_GROUP_SIZE {}", opts.row_group_size),
        ];

        if let Some(ref part) = opts.partition_by {
            let part_trimmed = part.trim();
            if part_trimmed.is_empty() {
                return Err(crate::error::AnalyticsError::InvalidInput(
                    "partition_by cannot be empty".to_string(),
                ));
            }

            // Support comma-separated identifiers
            let mut validated_parts = Vec::new();
            for p in part_trimmed.split(',') {
                let p = p.trim();
                let is_valid = !p.is_empty()
                    && p.chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                    && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');

                if !is_valid {
                    return Err(crate::error::AnalyticsError::InvalidInput(
                        "Invalid partition_by identifier".to_string(),
                    ));
                }
                validated_parts.push(p);
            }

            copy_opts.push(format!("PARTITION_BY ({})", validated_parts.join(", ")));
        }

        let sql = format!(
            "COPY ({}) TO '{}' ({})",
            query,
            out_path_str,
            copy_opts.join(", ")
        );

        let rows_written: i64 = self.conn.query_row(&sql, [], |row| row.get(0))?;

        let (bytes_written, sha256) = if out_path.is_dir() || opts.partition_by.is_some() {
            (0, String::new())
        } else if out_path.exists() {
            let bytes = std::fs::metadata(out_path)?.len();
            let mut file = std::fs::File::open(out_path)?;
            let mut hasher = Sha256::new();
            let mut buffer = [0u8; 8192];
            loop {
                let n = file.read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                hasher.update(&buffer[..n]);
            }
            (bytes, format!("{:x}", hasher.finalize()))
        } else {
            (0, String::new())
        };

        Ok(ExportReport {
            rows_written: rows_written as u64,
            bytes_written,
            path: out_path.to_path_buf(),
            sha256,
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
    fn test_export_parquet_security_validation() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA_DDL).unwrap();
        let analytics = Analytics { conn };
        let opts = ParquetExportOptions::default();

        // Semicolon
        let res = analytics.export_concepts_parquet("test;parquet", &opts);
        assert!(matches!(
            res,
            Err(crate::error::AnalyticsError::InvalidInput(_))
        ));

        // Single quote
        let res = analytics.export_concepts_parquet("test'parquet", &opts);
        assert!(matches!(
            res,
            Err(crate::error::AnalyticsError::InvalidInput(_))
        ));

        // Double quote
        let res = analytics.export_concepts_parquet("test\"parquet", &opts);
        assert!(matches!(
            res,
            Err(crate::error::AnalyticsError::InvalidInput(_))
        ));

        // Path traversal
        let res = analytics.export_concepts_parquet("../test.parquet", &opts);
        assert!(matches!(
            res,
            Err(crate::error::AnalyticsError::InvalidInput(_))
        ));

        // Control characters (like newline)
        let res = analytics.export_concepts_parquet("test\nparquet", &opts);
        assert!(matches!(
            res,
            Err(crate::error::AnalyticsError::InvalidInput(_))
        ));

        // Null bytes
        let res = analytics.export_concepts_parquet("test\0parquet", &opts);
        assert!(matches!(
            res,
            Err(crate::error::AnalyticsError::InvalidInput(_))
        ));

        // Invalid extension
        let res = analytics.export_concepts_parquet("test.db", &opts);
        assert!(matches!(
            res,
            Err(crate::error::AnalyticsError::InvalidInput(_))
        ));

        // Absolute path outside allowed directory
        let res = analytics.export_concepts_parquet("/etc/passwd", &opts);
        assert!(matches!(
            res,
            Err(crate::error::AnalyticsError::InvalidInput(_))
        ));

        // Happy path (clean path with correct extension)
        let res = analytics.export_concepts_parquet("clean_test.parquet", &opts);
        // Since there is no actual data loaded or file written, we just check that it is NOT InvalidInput
        assert!(!matches!(
            res,
            Err(crate::error::AnalyticsError::InvalidInput(_))
        ));
    }
}
