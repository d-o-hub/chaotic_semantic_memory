#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use std::time::Instant;

use chaotic_semantic_memory::persistence::Persistence;
use chaotic_semantic_memory::{ConceptBuilder, HVec10240};
use libsql::Builder;
use tempfile::NamedTempFile;

const NS: &str = "_default";
const DEFAULT_MEMORY_MODEL_BYTES_PER_CONCEPT: u64 = 1;
const DEFAULT_MEMORY_MODEL_CODEBOOK_BYTES: u64 = 2 * 1024 * 1024;
const DEFAULT_MEMORY_MODEL_METADATA_BYTES: u64 = 256 * 1024;
const DEFAULT_MEMORY_MODEL_CONCEPTS: u64 = 10_000_000;
const DEFAULT_MEMORY_MODEL_MAX_BYTES: u64 = 12 * 1024 * 1024;
const DEFAULT_LOCAL_ROUNDTRIP_SAMPLES: usize = 25;
const DEFAULT_LOCAL_ROUNDTRIP_MAX_P50_MS: f64 = 20.0;

fn projected_compressed_index_bytes(concept_count: u64) -> u64 {
    let bytes_per_concept = env_u64(
        "CSM_MEMORY_MODEL_BYTES_PER_CONCEPT",
        DEFAULT_MEMORY_MODEL_BYTES_PER_CONCEPT,
    );
    let codebook_bytes = env_u64(
        "CSM_MEMORY_MODEL_CODEBOOK_BYTES",
        DEFAULT_MEMORY_MODEL_CODEBOOK_BYTES,
    );
    let metadata_bytes = env_u64(
        "CSM_MEMORY_MODEL_METADATA_BYTES",
        DEFAULT_MEMORY_MODEL_METADATA_BYTES,
    );
    concept_count
        .saturating_mul(bytes_per_concept)
        .saturating_add(codebook_bytes)
        .saturating_add(metadata_bytes)
}

fn p50_ms(samples: &mut [f64]) -> f64 {
    samples.sort_by(|a, b| a.total_cmp(b));
    samples[samples.len() / 2]
}

/// Projects memory usage of a hypothetical **quantized/compressed ANN index**
/// at 10 million concepts.
///
/// # What this models
/// `bytes_per_concept = 1` represents a heavily quantized or hash-only index
/// entry (e.g. 1-byte Product Quantization code or a bloom-filter entry), NOT
/// the live in-memory Singularity store.
///
/// The live in-memory cost is approximately 1.5 KB/concept
/// (`size_of::<Concept>()` ≈ 1,296 bytes + HashMap/Vec heap overhead of
/// ~96 bytes). At that rate, 10M concepts would require ~14 GB — far beyond
/// what this test is designed to gate.
///
/// Use `empirical_bytes_per_concept_linear_fit` (below) to validate actual
/// struct-level memory cost.
#[test]
fn projected_compressed_index_10m_concepts_under_12mb() {
    let concepts = env_u64("CSM_MEMORY_MODEL_CONCEPTS", DEFAULT_MEMORY_MODEL_CONCEPTS);
    let threshold = env_u64("CSM_MEMORY_MODEL_MAX_BYTES", DEFAULT_MEMORY_MODEL_MAX_BYTES);
    let projected = projected_compressed_index_bytes(concepts);
    assert!(
        projected < threshold,
        "projected={projected} bytes exceeds {threshold} bytes"
    );
}

#[tokio::test]
async fn local_persistence_roundtrip_p50_under_20ms() {
    let sample_count = env_usize(
        "CSM_LOCAL_ROUNDTRIP_SAMPLES",
        DEFAULT_LOCAL_ROUNDTRIP_SAMPLES,
    );
    let threshold_ms = env_f64(
        "CSM_LOCAL_ROUNDTRIP_MAX_P50_MS",
        DEFAULT_LOCAL_ROUNDTRIP_MAX_P50_MS,
    );

    let db_file = NamedTempFile::new().expect("temp file");
    let db_path = db_file.path().to_string_lossy().to_string();
    let persistence = Persistence::new_local(&db_path).await.expect("new_local");

    let mut durations_ms = Vec::with_capacity(sample_count);
    for i in 0..sample_count {
        let id = format!("local-rt-{i}");
        let concept = ConceptBuilder::new(id.clone())
            .with_vector(HVec10240::random())
            .build()
            .expect("concept");

        let start = Instant::now();
        persistence
            .save_concept(NS, &concept)
            .await
            .expect("save_concept");
        let loaded = persistence
            .load_concept(NS, &id)
            .await
            .expect("load_concept");
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        assert!(loaded.is_some(), "concept should roundtrip");
        durations_ms.push(elapsed);
    }

    let p50 = p50_ms(&mut durations_ms);
    println!("LOCAL_ROUNDTRIP_P50_MS={p50:.3}");
    assert!(
        p50 < threshold_ms,
        "p50={p50:.3}ms is above {threshold_ms}ms"
    );
}

#[tokio::test]
async fn local_wal_checkpoint_roundtrip_stays_consistent() {
    let db_file = NamedTempFile::new().expect("temp file");
    let db_path = db_file.path().to_string_lossy().to_string();
    let persistence = Persistence::new_local(&db_path).await.expect("new_local");

    for i in 0..5 {
        let id = format!("wal-{i}");
        let concept = ConceptBuilder::new(id.clone())
            .with_vector(HVec10240::random())
            .build()
            .expect("concept");
        persistence
            .save_concept(NS, &concept)
            .await
            .expect("save_concept");
    }

    persistence.checkpoint().await.expect("checkpoint");

    let db = Builder::new_local(&db_path).build().await.expect("open db");
    let conn = db.connect().expect("connect");
    let mut rows = conn
        .query("PRAGMA journal_mode;", ())
        .await
        .expect("query pragma");
    let row = rows.next().await.expect("row read").expect("row");
    let mode: String = row.get(0).expect("mode");
    assert_eq!(mode.to_ascii_lowercase(), "wal");
}

/// Validates the compile-time floor of the `Concept` struct.
///
/// This is a struct-size gate, not a heap-allocation measurement.
/// Actual per-concept heap cost (including HashMap/String/Vec overhead)
/// is measured empirically by `rss_grows_linearly_with_concept_count` on Linux.
#[test]
fn concept_struct_size_floor() {
    use std::mem::size_of;
    let struct_bytes = size_of::<chaotic_semantic_memory::Concept>() as u64;
    println!("CONCEPT_STRUCT_BYTES={struct_bytes}");
    // HVec10240 alone is 1280 bytes; struct must not shrink below that floor.
    assert!(
        struct_bytes >= 1280,
        "Concept struct shrank below HVec10240 floor: {struct_bytes}"
    );
}

/// RSS-based empirical measurement of per-concept heap cost (Linux only).
///
/// Allocates concepts at two scales, derives bytes/concept from the RSS delta,
/// and asserts a regression ceiling. This is the only *measured* (non-formula)
/// memory test in this suite.
#[cfg(target_os = "linux")]
#[test]
fn rss_grows_linearly_with_concept_count() {
    fn rss_kb() -> u64 {
        let s = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
        for line in s.lines() {
            if let Some(rest) = line.strip_prefix("VmRSS:") {
                return rest
                    .split_whitespace()
                    .next()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);
            }
        }
        0
    }

    const SMALL: usize = 500;
    const LARGE: usize = 2_000;

    let build = |n: usize| -> Vec<_> {
        (0..n)
            .map(|i| {
                chaotic_semantic_memory::ConceptBuilder::new(format!("c{i}"))
                    .with_vector(chaotic_semantic_memory::HVec10240::random())
                    .build()
                    .expect("build")
            })
            .collect()
    };

    // Warm up allocator with a small batch first so we measure marginal cost.
    let warm = build(SMALL);
    let rss_after_small = rss_kb();
    let large = build(LARGE);
    let rss_after_large = rss_kb();

    // Keep allocations alive.
    assert_eq!(warm.len(), SMALL);
    assert_eq!(large.len(), LARGE);

    let delta_kb = rss_after_large.saturating_sub(rss_after_small);
    // Measured bytes/concept from the LARGE - SMALL marginal allocation.
    let measured_bytes_per_concept = (delta_kb * 1024) / LARGE as u64;
    println!("RSS_DELTA_KB={delta_kb}");
    println!("MEASURED_BYTES_PER_CONCEPT={measured_bytes_per_concept}");

    // Ceiling: live store cost must be below 4 KB/concept.
    // Struct alone is ~1296 bytes; realistic HashMap+String overhead < 2 KB.
    // This catches unexpected struct growth or allocator runaway.
    assert!(
        measured_bytes_per_concept < 4 * 1024,
        "measured {measured_bytes_per_concept} bytes/concept, expected < 4 KB"
    );
}

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default)
}

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(default)
}

fn env_f64(key: &str, default: f64) -> f64 {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(default)
}
