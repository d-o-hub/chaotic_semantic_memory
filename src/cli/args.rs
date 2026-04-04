use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "csm")]
#[command(about = "Chaotic Semantic Memory CLI", long_about = None)]
#[command(version)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Path to database file. If not specified, uses git-local storage when in a git repo.
    #[arg(short, long, global = true, value_name = "PATH")]
    pub database: Option<PathBuf>,

    /// Force git-local storage (.git/memory-index/csm.db).
    /// Creates "never committed, per-clone" storage inside the .git directory.
    /// Error if not in a git repository.
    #[arg(long, global = true)]
    pub git_local: bool,

    /// Override the default git-local index path.
    /// Only used when --git-local is specified or no database is given in a git repo.
    #[arg(long, global = true, value_name = "PATH")]
    pub index_path: Option<PathBuf>,

    #[arg(long, global = true, value_enum, default_value = "table")]
    pub output_format: OutputFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Json,
    Table,
    Quiet,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Inject(InjectArgs),
    Probe(ProbeArgs),
    Associate(AssociateArgs),
    Export(ExportArgs),
    Import(ImportArgs),
    Version(VersionArgs),
    Completions(CompletionsArgs),
}

#[derive(Args, Debug, Clone)]
pub struct InjectArgs {
    #[arg(required = true)]
    pub concept_id: String,

    #[arg(short, long)]
    pub from_file: Option<PathBuf>,

    #[arg(long, default_value = "random", value_enum)]
    pub vector_source: VectorSource,

    #[arg(short, long, value_name = "JSON")]
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum VectorSource {
    Random,
    File,
    Stdin,
}

#[derive(Args, Debug, Clone)]
pub struct ProbeArgs {
    #[arg(required = true)]
    pub concept_id: String,

    #[arg(short = 'k', long, default_value = "10")]
    pub top_k: usize,

    #[arg(short, long)]
    pub threshold: Option<f64>,
}

#[derive(Args, Debug, Clone)]
pub struct AssociateArgs {
    #[arg(required = true)]
    pub source_id: String,

    #[arg(required = true)]
    pub target_id: String,

    #[arg(short, long, default_value = "1.0")]
    pub strength: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ExportFormat {
    Json,
    Binary,
}

#[derive(Args, Debug, Clone)]
pub struct ExportArgs {
    #[arg(short, long, default_value = "export.json")]
    pub output: PathBuf,

    #[arg(long, value_enum, default_value = "json")]
    pub format: ExportFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ImportFormat {
    Json,
    Binary,
    Auto,
}

#[derive(Args, Debug, Clone)]
pub struct ImportArgs {
    #[arg(required = true)]
    pub input: PathBuf,

    #[arg(long, value_enum, default_value = "auto")]
    pub format: ImportFormat,

    #[arg(long)]
    pub merge: bool,
}

#[derive(Args, Debug, Clone)]
pub struct VersionArgs {
    #[arg(long)]
    pub detailed: bool,
}

#[derive(Args, Debug, Clone)]
pub struct CompletionsArgs {
    #[arg(value_enum)]
    pub shell: clap_complete::Shell,

    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,
}
