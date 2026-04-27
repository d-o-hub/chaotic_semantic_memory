use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Benchmark mode for memory system evaluation.
#[derive(Debug, Clone, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum Mode {
    /// Evaluate retrieval quality only (no LLM calls).
    RetrievalOnly,
    /// Include mock reader for answer quality evaluation.
    ReaderLite,
}

/// CLI configuration for the benchmark runner.
#[derive(Parser, Debug)]
#[command(name = "do_chaotic_semantic_memory_bench")]
pub struct Cli {
    /// Optional subcommand (e.g., generate dataset).
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Directory containing the dataset (sessions.jsonl, queries.jsonl).
    #[arg(long, default_value = "benchmarks/datasets/v1/small")]
    pub dataset_dir: PathBuf,

    /// Directory to write benchmark results.
    #[arg(long, default_value = "benchmarks/results")]
    pub out_dir: PathBuf,

    /// Benchmark mode: retrieval-only or reader-lite.
    #[arg(long, value_enum, default_value = "retrieval-only")]
    pub mode: Mode,

    /// Number of results to retrieve per query.
    #[arg(long, default_value_t = 10)]
    pub top_k: usize,

    /// Score threshold for abstention (retrieved[0].score < threshold triggers abstain).
    #[arg(long, default_value_t = 0.1)]
    pub abstain_threshold: f32,
}

/// Available subcommands.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Generate a deterministic seeded dataset.
    Generate {
        /// Output directory for generated dataset files.
        #[arg(long, default_value = "benchmarks/datasets/v1/small")]
        out_dir: PathBuf,

        /// Number of sessions to generate.
        #[arg(long, default_value_t = 10)]
        count: usize,

        /// Random seed for deterministic generation.
        #[arg(long, default_value_t = 42)]
        seed: u64,

        /// Minimum number of turns per session.
        #[arg(long, default_value_t = 3)]
        min_turns: usize,

        /// Maximum number of turns per session.
        #[arg(long, default_value_t = 3)]
        max_turns: usize,
    },
}
