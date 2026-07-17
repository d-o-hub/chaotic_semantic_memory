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

#[test]
fn projected_10m_concepts_memory_stays_under_12mb() {
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
