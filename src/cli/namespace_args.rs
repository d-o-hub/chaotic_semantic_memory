//! Namespace command argument types.

use clap::{Args, Subcommand};
use std::path::PathBuf;

/// Namespace management subcommands.
#[derive(Subcommand, Debug, Clone)]
pub enum NamespaceCommand {
    /// List all namespaces.
    List,
    /// Delete a namespace and all its concepts.
    Delete(NamespaceDeleteArgs),
    /// Export a namespace to a file.
    Export(NamespaceExportArgs),
}

/// Arguments for the namespaces command group.
#[derive(Args, Debug, Clone)]
pub struct NamespaceArgs {
    #[command(subcommand)]
    pub command: NamespaceCommand,
}

/// Arguments for namespace delete subcommand.
#[derive(Args, Debug, Clone)]
pub struct NamespaceDeleteArgs {
    /// Namespace name to delete.
    #[arg(required = true)]
    pub ns: String,

    /// Skip confirmation prompt.
    #[arg(short, long)]
    pub force: bool,
}

/// Arguments for namespace export subcommand.
#[derive(Args, Debug, Clone)]
pub struct NamespaceExportArgs {
    /// Namespace name to export.
    #[arg(required = true)]
    pub ns: String,

    /// Output file path.
    #[arg(required = true)]
    pub output: PathBuf,
}
