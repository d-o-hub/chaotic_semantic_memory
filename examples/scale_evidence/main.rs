//! ADR-0095 scale-evidence runner.
//!
//! Produces the machine-readable artifacts the Tier-2 acceptance criteria ask
//! for: an ANN comparison (exact / HNSW / LSH / bucketed candidates), local
//! persistence contention, and a measured memory/storage model with a
//! held-out error check. The companion script `scripts/scale-evidence.sh`
//! wraps each mode with the ADR-0095 evidence manifest (commit, dirty state,
//! toolchain, hardware, corpus checksum, command).
//!
//! ```text
//! cargo run --release --example scale_evidence --features ann-hnsw,ann-lsh -- ann --out DIR
//! cargo run --release --example scale_evidence --features ann-hnsw,ann-lsh -- persistence --out DIR
//! cargo run --release --example scale_evidence --features ann-hnsw,ann-lsh -- memory --out DIR
//! ```
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    // Measurement code converts durations and counters into f64/u64 for JSON.
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation
)]

mod ann;
mod memory;
mod persistence;
mod util;

use std::path::PathBuf;

use serde_json::Value;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = args.first().cloned().unwrap_or_default();

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");

    let value = match mode.as_str() {
        "ann" => ann::run(&ann::AnnParams {
            scales: parse_list(&args, "--scales", vec![1_000, 10_000, 50_000]),
            queries: parse_flag(&args, "--queries", 50),
            top_k: parse_flag(&args, "--top-k", 10),
            seed: parse_flag_u64(&args, "--seed", 42),
            clusters: parse_flag(&args, "--clusters", 64),
            noise_bits: parse_flag(&args, "--noise-bits", 512),
        }),
        "persistence" => rt.block_on(persistence::run(&persistence::PersistenceParams {
            scales: parse_list(&args, "--scales", vec![1_000, 10_000, 50_000]),
            tasks: parse_flag(&args, "--tasks", 8),
            ops_per_task: parse_flag(&args, "--ops", 25),
        })),
        "memory" => {
            let fit = parse_list(&args, "--fit", vec![1_000, 5_000, 10_000, 50_000]);
            let holdout = parse_flag(&args, "--holdout", 100_000);
            let point = args
                .iter()
                .position(|a| a == "--point")
                .and_then(|i| args.get(i + 1))
                .and_then(|v| v.parse::<usize>().ok());

            if let Some(concepts) = point {
                // Child process: measure exactly one point and print it.
                let measured = rt.block_on(memory::measure_point(concepts));
                println!("{}", serde_json::to_string(&measured).expect("json"));
                return;
            }

            let mut points = Vec::new();
            let exe = std::env::current_exe().expect("current exe");
            for &scale in fit.iter().chain(std::iter::once(&holdout)) {
                let output = std::process::Command::new(&exe)
                    .args(["memory", "--point", &scale.to_string()])
                    .output()
                    .expect("spawn memory point");
                let stdout = String::from_utf8_lossy(&output.stdout);
                let line = stdout.lines().last().unwrap_or_default();
                let parsed: Value = serde_json::from_str(line).unwrap_or_else(|e| {
                    panic!(
                        "memory point {scale} produced no JSON ({e}); stderr: {}",
                        String::from_utf8_lossy(&output.stderr)
                    )
                });
                points.push(parsed);
            }

            memory::model(
                &memory::MemoryParams {
                    fit,
                    holdout,
                    projection: parse_flag_u64(&args, "--projection", 10_000_000),
                    claim_bytes: parse_flag_u64(&args, "--claim-bytes", 12 * 1024 * 1024),
                },
                &points,
            )
        }
        other => {
            eprintln!("unknown mode: {other:?} (expected ann | persistence | memory)");
            std::process::exit(2);
        }
    };

    let out_dir: PathBuf = args
        .iter()
        .position(|a| a == "--out")
        .and_then(|i| args.get(i + 1))
        .map_or_else(|| PathBuf::from("target/scale-evidence"), PathBuf::from);
    std::fs::create_dir_all(&out_dir).expect("create out dir");

    let file = match mode.as_str() {
        "ann" => "ann_scale.json",
        "persistence" => "persistence_scale.json",
        _ => "memory_model.json",
    };
    let path = out_dir.join(file);
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&value).expect("encode artifact"),
    )
    .expect("write artifact");
    println!("wrote {}", path.display());
}

fn arg_value(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn parse_flag(args: &[String], name: &str, default: usize) -> usize {
    arg_value(args, name)
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn parse_flag_u64(args: &[String], name: &str, default: u64) -> u64 {
    arg_value(args, name)
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn parse_list(args: &[String], name: &str, default: Vec<usize>) -> Vec<usize> {
    arg_value(args, name).map_or(default, |v| {
        v.split(',')
            .filter_map(|item| item.trim().parse().ok())
            .collect()
    })
}
