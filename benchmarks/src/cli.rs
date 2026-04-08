use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum Mode {
    RetrievalOnly,
    ReaderLite,
}

#[derive(Parser, Debug)]
#[command(name = "do_chaotic_semantic_memory_bench")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(long, default_value = "benchmarks/datasets/v1/small")]
    pub dataset_dir: PathBuf,

    #[arg(long, default_value = "benchmarks/results")]
    pub out_dir: PathBuf,

    #[arg(long, value_enum, default_value = "retrieval-only")]
    pub mode: Mode,

    #[arg(long, default_value_t = 10)]
    pub top_k: usize,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Generate a dataset
    Generate {
        #[arg(long, default_value = "benchmarks/datasets/v1/small")]
        out_dir: PathBuf,

        #[arg(long, default_value_t = 10)]
        count: usize,

        #[arg(long, default_value_t = 42)]
        seed: u64,
    },
}
