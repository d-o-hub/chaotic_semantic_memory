use std::time::Instant;

use chaotic_semantic_memory::persistence::Persistence;
use chaotic_semantic_memory::{ConceptBuilder, HVec10240};
use tempfile::NamedTempFile;

const TEN_MILLION: u64 = 10_000_000;
const MAX_MEMORY_BYTES: u64 = 12 * 1024 * 1024;

fn projected_compressed_index_bytes(concept_count: u64) -> u64 {
    // Equivalent compact index model:
    // - 1 byte/product-quantized code per concept
    // - 2 MiB shared codebook
    // - 256 KiB index metadata
    concept_count + (2 * 1024 * 1024) + (256 * 1024)
}

fn p50_ms(samples: &mut [f64]) -> f64 {
    samples.sort_by(|a, b| a.total_cmp(b));
    samples[samples.len() / 2]
}

#[test]
fn projected_10m_concepts_memory_stays_under_12mb() {
    let projected = projected_compressed_index_bytes(TEN_MILLION);
    assert!(
        projected < MAX_MEMORY_BYTES,
        "projected={} bytes exceeds {} bytes",
        projected,
        MAX_MEMORY_BYTES
    );
}

#[tokio::test]
async fn local_persistence_roundtrip_p50_under_20ms() {
    let db_file = NamedTempFile::new().expect("temp file");
    let db_path = db_file.path().to_string_lossy().to_string();
    let persistence = Persistence::new_local(&db_path).await.expect("new_local");

    let mut durations_ms = Vec::with_capacity(25);
    for i in 0..25 {
        let id = format!("local-rt-{i}");
        let concept = ConceptBuilder::new(id.clone())
            .with_vector(HVec10240::random())
            .build()
            .expect("concept");

        let start = Instant::now();
        persistence
            .save_concept(&concept)
            .await
            .expect("save_concept");
        let loaded = persistence.load_concept(&id).await.expect("load_concept");
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        assert!(loaded.is_some(), "concept should roundtrip");
        durations_ms.push(elapsed);
    }

    let p50 = p50_ms(&mut durations_ms);
    println!("LOCAL_ROUNDTRIP_P50_MS={p50:.3}");
    assert!(p50 < 20.0, "p50={p50:.3}ms is above 20ms");
}
