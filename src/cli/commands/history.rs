//! History command for listing concept version history.

use super::{
    create_framework_with_namespace, run_get, run_rollback, validate_concept_id,
};
use crate::cli::args::{GetArgs, HistoryArgs, OutputFormat, RollbackArgs};
use crate::cli::error::{CliError, Result};
use colored::Colorize;
use std::path::Path;
use tracing::instrument;

#[instrument(name = "cli_history")]
pub async fn run_history(
    args: HistoryArgs,
    db_path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    validate_concept_id(&args.concept_id)?;

    // Dispatch to get version if --version is specified
    if let Some(version) = args.version {
        return run_get(
            GetArgs {
                namespace: args.namespace,
                concept_id: args.concept_id,
                version: Some(version),
            },
            db_path,
            format,
        )
        .await;
    }

    // Dispatch to rollback if --rollback is specified
    if let Some(to_version) = args.rollback {
        return run_rollback(
            RollbackArgs {
                namespace: args.namespace,
                concept_id: args.concept_id,
                to: to_version,
                confirm: args.confirm,
            },
            db_path,
            format,
        )
        .await;
    }

    let framework = create_framework_with_namespace(db_path, &args.namespace).await?;
    let versions = framework
        .list_versions(&args.concept_id)
        .await
        .map_err(|e| CliError::Persistence(format!("failed to list concept versions: {e}")))?;

    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string(&versions)
                    .map_err(|e| CliError::Output(format!("failed to serialize versions: {e}")))?
            );
        }
        OutputFormat::Table => {
            if versions.is_empty() {
                println!("No version history found for concept '{}'", args.concept_id);
            } else {
                println!(
                    "{} Version history for concept '{}':",
                    "✓".green(),
                    args.concept_id
                );
                println!(
                    "{:<10} {:<24} {:<16} {:<16}",
                    "VERSION", "TIMESTAMP", "VECTOR CHANGED", "METADATA CHANGED"
                );
                println!("{:-<10} {:-<24} {:-<16} {:-<16}", "", "", "", "");
                for v in &versions {
                    let ts_str = format_timestamp(v.timestamp_unix as u64);
                    let vec_chg_str = if v.vector_changed {
                        "yes".green()
                    } else {
                        "no".normal()
                    };
                    let meta_chg_str = if v.metadata_changed {
                        "yes".green()
                    } else {
                        "no".normal()
                    };
                    println!(
                        "{:<10} {:<24} {:<16} {:<16}",
                        v.version, ts_str, vec_chg_str, meta_chg_str
                    );
                }
            }
        }
        OutputFormat::Quiet => {
            for v in &versions {
                println!("{}", v.version);
            }
        }
    }

    Ok(())
}

fn format_timestamp(ts: u64) -> String {
    let secs = ts % 60;
    let mins = (ts / 60) % 60;
    let hours = (ts / 3600) % 24;
    let days = ts / 86400;
    if days > 0 {
        format!("{days}d {hours:02}:{mins:02}:{secs:02}")
    } else {
        format!("{hours:02}:{mins:02}:{secs:02}")
    }
}
