//! Namespace management commands: list, delete, export.

use std::io::{self, Write};
use std::path::Path;

use tracing::instrument;

use crate::cli::args::{NamespaceDeleteArgs, NamespaceExportArgs, OutputFormat};
use crate::cli::error::{CliError, Result};
use colored::Colorize;

use super::{create_framework, create_framework_with_namespace, print_success};

/// List all namespaces in the database.
#[instrument(name = "cli_namespaces_list")]
pub async fn run_namespaces_list(db_path: Option<&Path>, format: OutputFormat) -> Result<()> {
    let framework = create_framework(db_path).await?;

    let namespaces = framework
        .list_namespaces()
        .await
        .map_err(|e| CliError::Persistence(format!("failed to list namespaces: {e}")))?;

    match format {
        OutputFormat::Json => {
            let output = serde_json::json!({
                "status": "ok",
                "namespaces": namespaces,
            });
            println!(
                "{}",
                serde_json::to_string(&output).map_err(|e| CliError::Output(format!(
                    "failed to serialize namespaces: {e}"
                )))?
            );
        }
        OutputFormat::Table => {
            if namespaces.is_empty() {
                eprintln!("No namespaces found.");
            } else {
                eprintln!("Namespaces ({}):", namespaces.len());
                for ns in &namespaces {
                    println!("  {ns}");
                }
            }
        }
        OutputFormat::Quiet => {
            for ns in &namespaces {
                println!("{ns}");
            }
        }
    }

    Ok(())
}

/// Delete a namespace and all its concepts.
#[instrument(name = "cli_namespaces_delete")]
pub async fn run_namespaces_delete(
    args: NamespaceDeleteArgs,
    db_path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    let framework = create_framework(db_path).await?;

    // Verify namespace exists
    let namespaces = framework
        .list_namespaces()
        .await
        .map_err(|e| CliError::Persistence(format!("failed to list namespaces: {e}")))?;

    if !namespaces.iter().any(|n| n == &args.ns) {
        return Err(CliError::Input(format!(
            "namespace '{}' not found",
            args.ns
        )));
    }

    // Confirmation prompt (skip if --force or non-table output)
    if !args.force && matches!(format, OutputFormat::Table) {
        eprintln!(
            "{} Delete namespace '{}' and all its concepts? [y/N]",
            "Confirm:".yellow(),
            args.ns
        );
        io::stdout().flush().map_err(CliError::Io)?;

        let mut input = String::new();
        io::stdin().read_line(&mut input).map_err(CliError::Io)?;

        let response = input.trim().to_lowercase();
        if response != "y" && response != "yes" {
            eprintln!("{} Operation cancelled", "Cancelled:".yellow());
            return Ok(());
        }
    }

    // Perform deletion using the namespace-aware framework
    let framework_ns = create_framework_with_namespace(db_path, &args.ns).await?;
    let count = framework_ns.delete_namespace(&args.ns).await.map_err(|e| {
        CliError::Persistence(format!("failed to delete namespace '{}': {e}", args.ns))
    })?;

    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::json!({
                    "status": "deleted",
                    "namespace": args.ns,
                    "concepts_removed": count,
                })
            );
        }
        OutputFormat::Table => {
            print_success(
                &format!(
                    "namespace '{}' deleted ({} concept(s) removed)",
                    args.ns, count
                ),
                format,
            );
        }
        OutputFormat::Quiet => {}
    }

    Ok(())
}

/// Export a namespace to a file.
#[instrument(name = "cli_namespaces_export")]
pub async fn run_namespaces_export(
    args: NamespaceExportArgs,
    db_path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    let framework = create_framework(db_path).await?;

    // Verify namespace exists
    let namespaces = framework
        .list_namespaces()
        .await
        .map_err(|e| CliError::Persistence(format!("failed to list namespaces: {e}")))?;

    if !namespaces.iter().any(|n| n == &args.ns) {
        return Err(CliError::Input(format!(
            "namespace '{}' not found",
            args.ns
        )));
    }

    let path = &args.output;

    if matches!(format, OutputFormat::Table) {
        eprintln!("Exporting namespace '{}' to {}...", args.ns, path.display());
    }

    framework
        .export_namespace(&args.ns, path)
        .await
        .map_err(|e| CliError::Output(format!("export failed: {e}")))?;

    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::json!({
                    "status": "exported",
                    "namespace": args.ns,
                    "path": path.display().to_string(),
                })
            );
        }
        OutputFormat::Table => {
            print_success(
                &format!("namespace '{}' exported to {}", args.ns, path.display()),
                format,
            );
        }
        OutputFormat::Quiet => {}
    }

    Ok(())
}
