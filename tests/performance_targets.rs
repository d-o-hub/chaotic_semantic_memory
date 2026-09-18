#![cfg(feature = "persistence")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use std::time::Instant;

use chaotic_semantic_memory::persistence::Persistence;
use chaotic_semantic_memory::{ConceptBuilder, HVec10240};
use libsql::Builder;
use tempfile::NamedTempFile;

const NS: &str = "_default";
/// Scale point used by the measured-footprint test.
const MEASURED_MEMORY_SAMPLE_CONCEPTS: usize = 10_000;
/// Persisted bytes per concept (db + wal + shm, after checkpoint).
///
/// Measured at commit `4d81491` on an Intel i5-8350U and published as
/// `plans/evidence/scale_2026_09_17/memory_model.json`: 2 850 B/concept with a
/// held-out error of 0.06 % at 100 k concepts. The band guards against a
/// representation change that silently inflates the on-disk footprint (for
/// example storing a second copy of every vector).
const MEASURED_STORAGE_BYTES_PER_CONCEPT_MIN: u64 = 2_500;
const MEASURED_STORAGE_BYTES_PER_CONCEPT_MAX: u64 = 3_500;
/// Concept count the recorded release claim refers to.
const TEN_MILLION_CONCEPTS: u64 = 10_000_000;
const DEFAULT_LOCAL_ROUNDTRIP_SAMPLES: usize = 25;
const DEFAULT_LOCAL_ROUNDTRIP_MAX_P50_MS: f64 = 20.0;

/// Resident set size in bytes, from `/proc/self/statm` (Linux).
///
/// Informational only: the process is shared with parallel tests, so the value
/// is noisy by construction. The asserted footprint check uses persisted bytes,
/// which are process-independent.
fn rss_bytes() -> u64 {
    let statm = std::fs::read_to_string("/proc/self/statm").unwrap_or_default();
    let pages: u64 = statm
        .split_whitespace()
        .nth(1)
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    pages * 4096
}

/// Bytes held by the database and its WAL/shared-memory sidecars.
fn sqlite_bytes(db: &std::path::Path) -> u64 {
    let mut total = std::fs::metadata(db).map(|m| m.len()).unwrap_or(0);
    for suffix in ["-wal", "-shm"] {
        let sidecar = std::path::PathBuf::from(format!("{}{suffix}", db.display()));
        total += std::fs::metadata(&sidecar).map(|m| m.len()).unwrap_or(0);
    }
    total
}

fn p50_ms(samples: &mut [f64]) -> f64 {
    samples.sort_by(|a, b| a.total_cmp(b));
    samples[samples.len() / 2]
}

/// Measure the shipped footprint instead of asserting the arithmetic of an
/// unimplemented design.
///
/// The historical `< 12 MB for 10M concepts` gate describes ADR-0024 phase-2
/// product quantization (1 byte/concept + 2 MB codebook), which was never
/// implemented: the shipped representation stores a 1 280-byte `HVec10240` per
/// concept (plus index copies) and writes it twice (concept row + version row).
/// The evidence model in `plans/evidence/scale_2026_09_17/` measures 4 691 B
/// RSS and 2 850 B storage per concept; at that slope the 10M projection is
/// 43.7 GB / 26.5 GB, so the claim is not supportable and is not asserted here
/// (ADR-0095: memory claims come from measured points, not constants).
#[tokio::test]
async fn measured_memory_footprint_matches_the_evidence_model() {
    let db_file = NamedTempFile::new().expect("temp file");
    let db_path = db_file.path().to_string_lossy().to_string();
    let persistence = Persistence::new_local(&db_path).await.expect("new_local");

    let start_rss = rss_bytes();
    let mut batch = Vec::with_capacity(500);
    for i in 0..MEASURED_MEMORY_SAMPLE_CONCEPTS {
        let concept = ConceptBuilder::new(format!("measured-{i}"))
            .with_vector(HVec10240::random())
            .build()
            .expect("concept");
        batch.push(concept);
        if batch.len() == 500 {
            persistence
                .save_concepts(NS, &batch)
                .await
                .expect("save batch");
            batch.clear();
        }
    }
    persistence.checkpoint().await.expect("checkpoint");

    let persisted = sqlite_bytes(std::path::Path::new(&db_path));
    let per_concept = persisted / MEASURED_MEMORY_SAMPLE_CONCEPTS as u64;
    let rss_per_concept =
        rss_bytes().saturating_sub(start_rss) / MEASURED_MEMORY_SAMPLE_CONCEPTS as u64;

    let projected_storage_gb = per_concept * TEN_MILLION_CONCEPTS / 1024 / 1024 / 1024;
    let projected_rss_gb = rss_per_concept * TEN_MILLION_CONCEPTS / 1024 / 1024 / 1024;

    println!(
        "MEASURED_PERSISTED_BYTES_PER_CONCEPT={per_concept} \
         MEASURED_RSS_BYTES_PER_CONCEPT={rss_per_concept} \
         PROJECTED_10M_STORAGE_GB={projected_storage_gb} PROJECTED_10M_RSS_GB={projected_rss_gb}"
    );

    assert!(
        (MEASURED_STORAGE_BYTES_PER_CONCEPT_MIN..=MEASURED_STORAGE_BYTES_PER_CONCEPT_MAX)
            .contains(&per_concept),
        "persisted {per_concept} B/concept is outside the measured band \
         [{MEASURED_STORAGE_BYTES_PER_CONCEPT_MIN}, {MEASURED_STORAGE_BYTES_PER_CONCEPT_MAX}]; \
         the on-disk footprint changed, update the evidence model"
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
            .load_concept::<HVec10240>(NS, &id)
            .await
            .expect("load_concept");
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        assert!(loaded.is_some(), "concept should roundtrip");
        durations_ms.push(elapsed);
    }

    let p50 = p50_ms(&mut durations_ms);
    let persisted_bytes = std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);
    println!("LOCAL_ROUNDTRIP_P50_MS={p50:.3} PERSISTED_BYTES={persisted_bytes}");
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
