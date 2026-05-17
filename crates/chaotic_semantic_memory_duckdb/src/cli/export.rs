use crate::Analytics;
use crate::cli::ExportArgs;
use crate::error::Result;

pub async fn run(analytics: &Analytics, args: ExportArgs) -> Result<()> {
    #[cfg(not(feature = "parquet"))]
    {
        let _ = analytics;
        let _ = args;
        return Err(crate::error::AnalyticsError::InvalidInput(
            "Parquet feature not enabled".to_string(),
        ));
    }

    #[cfg(feature = "parquet")]
    {
        use crate::export_parquet::ParquetExportOptions;

        let opts = ParquetExportOptions {
            compression: args.compression,
            row_group_size: args.row_group_size,
            partition_by: args.partition_by,
            include_manifest: true,
        };

        println!("Exporting to {}...", args.out.display());
        let report = analytics.export_all_parquet(&args.out, &opts)?;

        println!("Export complete.");
        println!("Concepts:     {} rows", report.concepts.rows_written);
        println!("Associations: {} rows", report.associations.rows_written);
        println!("Versions:     {} rows", report.versions.rows_written);
        println!("Benchmarks:   {} rows", report.benchmarks.rows_written);
        if let Some(m) = report.manifest_path {
            println!("Manifest:     {}", m.display());
        }

        Ok(())
    }
}
