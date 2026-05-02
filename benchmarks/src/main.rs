mod cli;
mod dataset;
mod generator;
mod memory_adapter;
mod metrics;
mod reader;
mod report;
mod runner;
mod scorer;
mod types;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Generate {
            out_dir,
            count,
            seed,
            min_turns,
            max_turns,
        }) => {
            println!(
                "Generating dataset to {} with {} sessions (seed: {}, turns: {}-{})...",
                out_dir.display(),
                count,
                seed,
                min_turns,
                max_turns
            );
            std::fs::create_dir_all(&out_dir)?;

            let sessions = generator::generate_sessions_with_range(seed, count, min_turns, max_turns);
            let queries = generator::generate_queries(&sessions);

            let sessions_path = out_dir.join("sessions.jsonl");
            let mut sessions_content = String::new();
            for s in sessions {
                sessions_content.push_str(&serde_json::to_string(&s)?);
                sessions_content.push('\n');
            }
            std::fs::write(sessions_path, sessions_content)?;

            let queries_path = out_dir.join("queries.jsonl");
            let mut queries_content = String::new();
            for q in queries {
                queries_content.push_str(&serde_json::to_string(&q)?);
                queries_content.push('\n');
            }
            std::fs::write(queries_path, queries_content)?;

            let manifest_path = out_dir.join("manifest.json");
            let manifest = serde_json::json!({
                "version": "v1",
                "seed": seed,
                "session_count": count,
            });
            std::fs::write(manifest_path, serde_json::to_string_pretty(&manifest)?)?;

            println!("Generation complete.");
        }
        None => {
            runner::run(cli).await?;
        }
    }

    Ok(())
}
