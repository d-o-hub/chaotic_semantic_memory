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

    /// Namespace for isolation (default: _default).
    #[arg(long, global = true, default_value = "_default")]
    pub namespace: String,
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
    /// Delete a concept from memory.
    Delete(DeleteArgs),
    /// Get concept details by ID.
    Get(GetArgs),
    /// Get history of versions for a concept.
    History(HistoryArgs),
    /// Diff two concept versions.
    Diff(DiffArgs),
    /// Rollback concept to a historical version.
    Rollback(RollbackArgs),
    /// Update concept vector or metadata.
    Update(UpdateArgs),
    /// Remove association(s) from a concept.
    Disassociate(DisassociateArgs),
    /// List associations for a concept.
    Associations(AssociationsArgs),
    /// Traverse graph from a starting concept.
    Traverse(TraverseArgs),
    /// Find shortest path between two concepts.
    Path(PathArgs),
    /// Similarity search with metadata filter.
    ProbeFiltered(ProbeFilteredArgs),
    /// Database statistics.
    Stats(StatsArgs),
    /// Framework performance metrics.
    Metrics(MetricsArgs),
    /// Watch for real-time memory events.
    Watch(WatchArgs),
    /// GraphRAG retrieval: similarity + graph traversal hybrid.
    ProbeGraph(ProbeGraphArgs),
    /// Prune orphaned associations from the database.
    Prune(PruneArgs),
    /// Compact the database to reclaim space.
    Compact(CompactArgs),
    /// MCP server commands.
    #[cfg(feature = "mcp")]
    #[command(subcommand)]
    Mcp(crate::cli::mcp::McpCommands),
}

#[derive(Args, Debug, Clone)]
pub struct InjectArgs {
    #[arg(long, global = true, default_value = "_default")]
    pub namespace: String,
    #[arg(required = true)]
    pub concept_id: String,
    #[arg(short, long)]
    pub from_file: Option<PathBuf>,
    #[arg(long, default_value = "random", value_enum)]
    pub vector_source: VectorSource,
    /// Text to encode into a vector.
    #[arg(short, long)]
    pub text: Option<String>,
    /// Use external embedding model if configured.
    #[arg(long)]
    pub use_embeddings: bool,
    /// Embedding provider: 'hdc', 'fastembed[:model]', 'openai[:model]', 'voyage[:model]'.
    #[arg(long, value_name = "PROVIDER")]
    pub provider: Option<String>,
    #[arg(short, long, value_name = "JSON")]
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum VectorSource {
    Random,
    File,
    Stdin,
    Text,
}

#[derive(Args, Debug, Clone)]
pub struct ProbeArgs {
    #[arg(long, global = true, default_value = "_default")]
    pub namespace: String,
    #[arg(required = true)]
    pub concept_id: String,
    #[arg(short = 'k', long, default_value = "10")]
    pub top_k: usize,
    #[arg(short, long)]
    pub threshold: Option<f64>,
}

/// Arguments for text-based similarity query.
#[derive(Args, Debug, Clone)]
#[command(alias = "probe-text")]
pub struct QueryArgs {
    #[arg(long, global = true, default_value = "_default")]
    pub namespace: String,
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

    /// Embedding provider: 'hdc', 'fastembed[:model]', 'openai[:model]', 'voyage[:model]'.
    #[arg(long, value_name = "PROVIDER")]
    pub provider: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub struct AssociateArgs {
    #[arg(long, global = true, default_value = "_default")]
    pub namespace: String,
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
    #[arg(long, global = true, default_value = "_default")]
    pub namespace: String,
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
    #[arg(long, global = true, default_value = "_default")]
    pub namespace: String,
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
    #[arg(long, global = true, default_value = "_default")]
    pub namespace: String,
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
    #[arg(long, global = true, default_value = "_default")]
    pub namespace: String,
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

/// Arguments for the delete command.
#[derive(Args, Debug, Clone)]
pub struct DeleteArgs {
    #[arg(long, global = true, default_value = "_default")]
    pub namespace: String,
    /// Concept ID to delete.
    #[arg(required = true)]
    pub concept_id: String,

    /// Skip confirmation prompt.
    #[arg(short, long)]
    pub force: bool,
}

/// Arguments for the get command.
#[derive(Args, Debug, Clone)]
pub struct GetArgs {
    #[arg(long, global = true, default_value = "_default")]
    pub namespace: String,
    #[arg(required = true)]
    pub concept_id: String,
    /// Retrieve a specific historical version of the concept.
    #[arg(long)]
    pub version: Option<u64>,
}

/// Arguments for the update command.
#[derive(Args, Debug, Clone)]
pub struct UpdateArgs {
    #[arg(long, global = true, default_value = "_default")]
    pub namespace: String,
    /// Concept ID to update.
    #[arg(required = true)]
    pub concept_id: String,

    /// Generate new vector from text encoding.
    #[arg(long, value_name = "TEXT")]
    pub vector_from_text: Option<String>,

    /// Use code-aware encoding for vector generation.
    #[arg(long)]
    pub code_aware: bool,

    /// Update metadata with JSON object (merged with existing).
    #[arg(short, long, value_name = "JSON")]
    pub metadata: Option<String>,

    /// Replace metadata entirely instead of merging.
    #[arg(long)]
    pub replace_metadata: bool,
}

/// Arguments for the disassociate command.
#[derive(Args, Debug, Clone)]
pub struct DisassociateArgs {
    #[arg(long, global = true, default_value = "_default")]
    pub namespace: String,
    /// Source concept ID (association owner).
    #[arg(required = true)]
    pub from: String,

    /// Target concept ID to remove association to.
    /// If omitted, clears all associations from source.
    #[arg(required = false)]
    pub to: Option<String>,
}

/// Arguments for listing associations of a concept.
#[derive(Args, Debug, Clone)]
pub struct AssociationsArgs {
    #[arg(long, global = true, default_value = "_default")]
    pub namespace: String,
    /// Concept ID to query associations for.
    #[arg(required = true)]
    pub concept_id: String,

    /// List incoming associations (reverse direction) instead of outbound.
    #[arg(short, long)]
    pub reverse: bool,
}

/// Arguments for BFS traversal from a starting concept.
#[derive(Args, Debug, Clone)]
pub struct TraverseArgs {
    #[arg(long, global = true, default_value = "_default")]
    pub namespace: String,
    #[arg(required = true)]
    pub start: String,
    #[arg(long, default_value = "3")]
    pub depth: usize,
    #[arg(short, long, default_value = "0.0")]
    pub min_strength: f64,
}

/// Arguments for finding shortest path between two concepts.
#[derive(Args, Debug, Clone)]
pub struct PathArgs {
    #[arg(long, global = true, default_value = "_default")]
    pub namespace: String,
    #[arg(required = true)]
    pub from: String,
    #[arg(required = true)]
    pub to: String,
    #[arg(short, long)]
    pub weighted: bool,
}

/// Arguments for filtered similarity probe.
#[derive(Args, Debug, Clone)]
pub struct ProbeFilteredArgs {
    #[arg(long, global = true, default_value = "_default")]
    pub namespace: String,
    /// Concept ID to use as query vector.
    #[arg(required = true)]
    pub concept_id: String,

    /// Maximum number of results to return.
    #[arg(short = 'k', long, default_value = "10")]
    pub top_k: usize,

    /// JSON metadata filter expression.
    #[arg(short, long, value_name = "JSON")]
    pub filter: String,
}

#[derive(Args, Debug, Clone)]
pub struct StatsArgs;

#[derive(Args, Debug, Clone)]
pub struct MetricsArgs {
    #[arg(long)]
    pub reset: bool,
}

#[derive(Args, Debug, Clone)]
pub struct WatchArgs {
    #[arg(short, long, default_value = "all")]
    pub filter: String,
}

#[derive(Args, Debug, Clone)]
pub struct ProbeGraphArgs {
    #[arg(long, global = true, default_value = "_default")]
    pub namespace: String,
    #[arg(required = true)]
    pub text: String,
    #[arg(long, default_value = "5")]
    pub anchors: usize,
    #[arg(long, default_value = "2")]
    pub hops: usize,
    #[arg(long, default_value = "0.0")]
    pub min_strength: f32,
    #[arg(long, default_value = "0.6")]
    pub similarity_weight: f32,
    #[arg(long, default_value = "0.4")]
    pub graph_weight: f32,
    #[arg(short = 'k', long, default_value = "20")]
    pub top_k: usize,
}

/// Arguments for the history command.
#[derive(Args, Debug, Clone)]
pub struct HistoryArgs {
    #[arg(long, global = true, default_value = "_default")]
    pub namespace: String,
    #[arg(required = true)]
    pub concept_id: String,
}

/// Arguments for the diff command.
#[derive(Args, Debug, Clone)]
pub struct DiffArgs {
    #[arg(long, global = true, default_value = "_default")]
    pub namespace: String,
    #[arg(required = true)]
    pub concept_id: String,
    #[arg(long)]
    pub from: u64,
    #[arg(long)]
    pub to: u64,
}

/// Arguments for the rollback command.
#[derive(Args, Debug, Clone)]
pub struct RollbackArgs {
    #[arg(long, global = true, default_value = "_default")]
    pub namespace: String,
    #[arg(required = true)]
    pub concept_id: String,
    #[arg(long)]
    pub to: u64,
    #[arg(short, long)]
    pub confirm: bool,
}

#[derive(Args, Debug, Clone)]
pub struct PruneArgs;

#[derive(Args, Debug, Clone)]
pub struct CompactArgs;
