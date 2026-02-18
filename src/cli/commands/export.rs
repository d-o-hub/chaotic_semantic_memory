use std::path::Path;

use anyhow::{Context, Result};

use crate::cli::args::{ExportArgs, ExportFormat, OutputFormat};

use super::{create_framework, print_success, print_warning};

pub async fn run_export(args: ExportArgs, db_path: Option<&Path>, format: OutputFormat) -> Result<()> {
    let framework = create_framework(db_path)
        .await
        .context("failed to initialize framework")?;

    let path_str = args.output.to_string_lossy();
    let stats = framework.stats().await?;

    if stats.concept_count == 0 {
        print_warning("exporting empty memory state", format);
    } else if matches!(format, OutputFormat::Table) {
        eprintln!(
            "Exporting {} concepts to {}...",
            stats.concept_count,
            args.output.display()
        );
    }

    let result = match args.format {
        ExportFormat::Json => framework.export_json(&path_str).await,
        ExportFormat::Binary => framework.export_binary(&path_str).await,
    };

    match result {
        Ok(()) => {
            print_success(
                &format!("exported {} concepts to {}", stats.concept_count, args.output.display()),
                format,
            );
            if matches!(format, OutputFormat::Json) {
                println!(
                    r#"{{"exported":{},"path":"{}"}}"#,
                    stats.concept_count,
                    args.output.display()
                );
            }
        }
        Err(e) => {
            let msg = format!("export failed: {}", e);
            return Err(anyhow::anyhow!(msg));
        }
    }

    Ok(())
}
