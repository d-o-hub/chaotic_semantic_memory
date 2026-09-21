//! ANN backend comparison: exact (brute force), HNSW and LSH indexes
//! measured directly, plus the bucketed candidate-generation path measured
//! through `Singularity`.

use std::collections::HashMap;
use std::time::Instant;

use serde_json::{Value, json};

use chaotic_semantic_memory::index::{IndexBackend, create_index};
use chaotic_semantic_memory::singularity::{Concept, Singularity, SingularityConfig};
use chaotic_semantic_memory::{HVec10240, RetrievalConfig};

use super::util::{Corpus, summarize_us};

const NS: &str = "_default";

/// Parameters for one ANN comparison run.
pub struct AnnParams {
    /// Concept counts, ascending.
    pub scales: Vec<usize>,
    /// Query vectors per scale.
    pub queries: usize,
    /// `top_k` for every search.
    pub top_k: usize,
    /// Corpus RNG seed.
    pub seed: u64,
    /// Cluster count for the synthetic corpus.
    pub clusters: usize,
    /// Bits flipped from the cluster centre per concept.
    pub noise_bits: usize,
    /// Probe width for the bucketed candidate path.
    pub bucket_probe_width: usize,
    /// Backends to measure (`exact`, `hnsw`, `lsh`, `bucket`); exact always runs
    /// because it is the ground truth.
    pub backends: Vec<String>,
}

pub fn run(params: &AnnParams) -> Value {
    let per_scale: Vec<Value> = params
        .scales
        .iter()
        .map(|&scale| {
            let (corpus, query_vectors) = Corpus::generate(
                params.seed,
                scale,
                params.queries,
                params.clusters,
                params.noise_bits,
            );
            let checksum = super::util::corpus_checksum(&corpus.vectors);
            let concepts = corpus.concepts();

            // Exact search supplies both the "exact" backend row and the
            // ground truth every approximate backend is scored against.
            let (exact, truth) = measure_index(
                "exact",
                &IndexBackend::BruteForce,
                &corpus,
                &concepts,
                &query_vectors,
                params.top_k,
                &[],
            );

            let wanted = |name: &str| {
                params.backends.is_empty() || params.backends.iter().any(|b| b == name)
            };
            let mut rows = vec![exact];
            for (name, backend) in [
                (
                    "hnsw",
                    IndexBackend::Hnsw {
                        m: 16,
                        ef_construction: 200,
                        ef_search: 50,
                    },
                ),
                (
                    "lsh",
                    IndexBackend::Lsh {
                        num_tables: 5,
                        hash_bits: 16,
                    },
                ),
            ] {
                if !wanted(name) {
                    continue;
                }
                let (row, _) = measure_index(
                    name,
                    &backend,
                    &corpus,
                    &concepts,
                    &query_vectors,
                    params.top_k,
                    &truth,
                );
                rows.push(row);
            }

            if wanted("bucket") {
                rows.push(measure_bucket(
                    &corpus,
                    &query_vectors,
                    params.top_k,
                    &truth,
                    params.bucket_probe_width,
                ));
            }

            json!({
                "concepts": scale,
                "queries": query_vectors.len(),
                "corpus_version": super::util::CORPUS_VERSION,
                "corpus_checksum": checksum,
                "backends": rows,
            })
        })
        .collect();

    json!({
        "kind": "ann_scale",
        "top_k": params.top_k,
        "seed": params.seed,
        "clusters": params.clusters,
        "noise_bits": params.noise_bits,
        "scales": per_scale,
    })
}

/// Measure one `AnnIndex` backend; `truth` is `None` for the exact row.
#[allow(clippy::too_many_arguments)]
fn measure_index(
    name: &str,
    backend: &IndexBackend,
    corpus: &Corpus,
    concepts: &[Concept],
    queries: &[HVec10240],
    top_k: usize,
    truth: &[Vec<String>],
) -> (Value, Vec<Vec<String>>) {
    let mut index = create_index::<HVec10240>(backend).expect("index construction");

    let build_start = Instant::now();
    for (id, vector) in corpus.ids.iter().zip(&corpus.vectors) {
        index.insert(id.clone(), vector).expect("insert");
    }
    let build_ms = build_start.elapsed().as_secs_f64() * 1000.0;

    let mut latencies = Vec::with_capacity(queries.len());
    let mut found: Vec<Vec<String>> = Vec::with_capacity(queries.len());
    for query in queries {
        let start = Instant::now();
        let hits = index.search(query, top_k).expect("search");
        latencies.push(start.elapsed().as_micros() as u64);
        found.push(hits.into_iter().map(|(id, _)| id).collect());
    }

    let recall = recall_at_k(&found, truth);
    let stats = index.stats();
    let serialized = index.serialize().expect("serialize");
    let serialize_bytes = serialized.len();

    // Reload: deserialize into a fresh index when the backend supports it,
    // otherwise rebuild from the concept set (brute force serializes empty).
    let reload_start = Instant::now();
    let reload_method = if serialize_bytes > 0 {
        let mut fresh = create_index::<HVec10240>(backend).expect("index construction");
        fresh.deserialize(&serialized).expect("deserialize");
        let hits = fresh
            .search(&queries[0], top_k)
            .expect("search after reload");
        format!("deserialize:{}", hits.len())
    } else {
        let map: HashMap<String, Concept> =
            concepts.iter().map(|c| (c.id.clone(), c.clone())).collect();
        index.rebuild(&map).expect("rebuild");
        format!("rebuild:{}", index.stats().count)
    };
    let reload_ms = reload_start.elapsed().as_secs_f64() * 1000.0;

    // Update path: delete and re-insert a bounded slice of ids.
    let update_count = (corpus.ids.len() / 10).clamp(1, 100);
    let delete_start = Instant::now();
    for id in corpus.ids.iter().take(update_count) {
        index.delete(id).expect("delete");
    }
    let delete_ms = delete_start.elapsed().as_secs_f64() * 1000.0;

    let insert_start = Instant::now();
    for i in 0..update_count {
        index
            .insert(corpus.ids[i].clone(), &corpus.vectors[i])
            .expect("re-insert");
    }
    let reinsert_ms = insert_start.elapsed().as_secs_f64() * 1000.0;

    let row = json!({
        "backend": name,
        "build_ms": build_ms,
        "query": summarize_us(latencies),
        "recall_at_k": if truth.is_empty() { 1.0 } else { recall },
        "is_ground_truth": truth.is_empty(),
        "index_bytes": stats.memory_usage_bytes,
        "index_count": stats.count,
        "serialize_bytes": serialize_bytes,
        "reload_ms": reload_ms,
        "reload_method": reload_method,
        "update_ms": {
            "delete": delete_ms,
            "reinsert": reinsert_ms,
            "count": update_count,
        },
    });
    (row, found)
}

/// Bucketed candidate generation, measured through the real `Singularity`
/// retrieval path (candidate filter + exact rescoring of candidates).
fn measure_bucket(
    corpus: &Corpus,
    queries: &[HVec10240],
    top_k: usize,
    truth: &[Vec<String>],
    probe_width: usize,
) -> Value {
    let mut engine = Singularity::with_config(SingularityConfig::default());
    engine
        .set_retrieval_config(RetrievalConfig {
            enable_bucket_candidates: true,
            bucket_probe_width: probe_width,
            enable_graph_candidates: false,
            ..RetrievalConfig::default()
        })
        .expect("retrieval config");

    let build_start = Instant::now();
    for concept in corpus.concepts() {
        engine.inject(NS, concept).expect("inject");
    }
    let build_ms = build_start.elapsed().as_secs_f64() * 1000.0;

    let mut latencies = Vec::with_capacity(queries.len());
    let mut found: Vec<Vec<String>> = Vec::with_capacity(queries.len());
    let mut candidates = 0u64;
    let mut scored = 0u64;
    let mut candidate_ns = 0u64;
    let mut scoring_ns = 0u64;
    let mut fallbacks = 0u64;

    for query in queries {
        let start = Instant::now();
        let hits = engine.find_similar(NS, query, top_k);
        latencies.push(start.elapsed().as_micros() as u64);
        found.push(hits.into_iter().map(|(id, _)| id).collect());

        let stats = engine.last_retrieval_stats(NS);
        candidates += stats.candidate_count as u64;
        scored += stats.scored_count as u64;
        candidate_ns += stats.candidate_ns;
        scoring_ns += stats.scoring_ns;
        fallbacks += u64::from(stats.fell_back_to_exact_scan);
    }

    let n = queries.len().max(1) as u64;
    json!({
        "backend": "bucket",
        "build_ms": build_ms,
        "query": summarize_us(latencies),
        "recall_at_k": recall_at_k(&found, truth),
        "index_bytes": 0,
        "index_count": corpus.ids.len(),
        "serialize_bytes": 0,
        "reload_ms": 0.0,
        "reload_method": "no-persistent-state",
        "update_ms": { "delete": 0.0, "reinsert": 0.0, "count": 0 },
        "candidate_count_avg": candidates as f64 / n as f64,
        "scored_count_avg": scored as f64 / n as f64,
        "candidate_ns_avg": candidate_ns as f64 / n as f64,
        "scoring_ns_avg": scoring_ns as f64 / n as f64,
        "exact_fallbacks": fallbacks,
        "bucket_probe_width": probe_width,
    })
}

/// Recall@k = relevant retrieved / total relevant, averaged over queries.
/// Hit rate is a different metric and is deliberately not used here
/// (ADR-0095 metric definitions).
fn recall_at_k(found: &[Vec<String>], truth: &[Vec<String>]) -> f64 {
    if truth.is_empty() {
        return f64::NAN;
    }
    let mut total = 0.0;
    for (hits, gold) in found.iter().zip(truth) {
        if gold.is_empty() {
            continue;
        }
        let relevant = gold.iter().filter(|g| hits.contains(g)).count();
        total += relevant as f64 / gold.len() as f64;
    }
    total / truth.len() as f64
}
