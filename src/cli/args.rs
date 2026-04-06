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
    Query(QueryArgs),
    Associate(AssociateArgs),
    Export(ExportArgs),
    Import(ImportArgs),
    Version(VersionArgs),
    Completions(CompletionsArgs),
    /// Index JSONL file content into memory.
    IndexJsonl(IndexJsonlArgs),
    /// Index Markdown files from directory into memory.
    IndexDir(IndexDirArgs),
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

/// Arguments for text-based similarity query.
#[derive(Args, Debug, Clone)]
pub struct QueryArgs {
    /// Text to encode and search for similar concepts.
    #[arg(required = true)]
    pub text: String,

    /// Maximum number of results to return.
    #[arg(short = 'k', long, default_value = "10")]
    pub top_k: usize,

    /// Minimum similarity score (0.0-1.0) for results.
    #[arg(short, long, default_value = "0.0")]
    pub min_score: f64,

    /// Use code-aware encoding for source code content.
    #[arg(long)]
    pub code_aware: bool,

    /// Compact output: trim long text to 200 characters.
    #[arg(long)]
    pub compact: bool,

    /// Disable hybrid mode (use semantic-only HDC search).
    #[arg(long)]
    pub semantic_only: bool,

    /// Use keyword-only BM25 search (no semantic matching).
    #[arg(long)]
    pub keyword_only: bool,

    /// Override automatic keyword weight (0.0-1.0).
    /// Default is query-length-dependent: 0.9 for 1-2 tokens, 0.7 for 3-4,
    /// 0.4 for 5-8, 0.2 for 9+.
    #[arg(long, value_name = "WEIGHT")]
    pub keyword_weight: Option<f64>,
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

/// Arguments for indexing JSONL files.
#[derive(Args, Debug, Clone)]
pub struct IndexJsonlArgs {
    /// Path to JSONL file to index.
    #[arg(short = 'F', long, value_name = "FILE")]
    pub file: PathBuf,

    /// Field name containing text to encode (default: "text").
    #[arg(short, long, default_value = "text")]
    pub field: String,

    /// Field name containing unique ID (optional).
    #[arg(long, value_name = "FIELD")]
    pub id_field: Option<String>,

    /// Field name containing comma-separated tags (optional).
    #[arg(long, value_name = "FIELD")]
    pub tag_field: Option<String>,

    /// Use code-aware encoding for source code content.
    #[arg(long)]
    pub code_aware: bool,
}

/// Arguments for indexing Markdown directory.
#[derive(Args, Debug, Clone)]
pub struct IndexDirArgs {
    /// Glob pattern(s) for files to index (can be repeated).
    #[arg(short, long, required = true, value_name = "PATTERN")]
    pub glob: Vec<String>,

    /// Minimum heading level to chunk (default: 2 for ## sections).
    #[arg(long, default_value = "2", value_name = "LEVEL")]
    pub heading_level: usize,

    /// Use code-aware encoding for source code content.
    #[arg(long)]
    pub code_aware: bool,
}
