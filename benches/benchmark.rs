use chaotic_semantic_memory::HVec10240;
use chaotic_semantic_memory::bridge_retrieval::BridgeRetrieval;
use chaotic_semantic_memory::bundle::BundleAccumulator;
use chaotic_semantic_memory::encoder::TextEncoder;
use chaotic_semantic_memory::graph_traversal::TraversalConfig;
use chaotic_semantic_memory::metadata_filter::MetadataFilter;
use chaotic_semantic_memory::reservoir::Reservoir;
use chaotic_semantic_memory::retrieval::bm25::Bm25Index;
use chaotic_semantic_memory::semantic_bridge::{
    BridgeConfig, BridgeHit, CanonicalConcept, ConceptGraph, MemoryPacket, ScoreBreakdown,
};
use chaotic_semantic_memory::singularity::{Concept, ConceptBuilder, Singularity};
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::time::Duration;

const PROBE_BENCH_TOP_K: usize = 10;
const PROBE_BENCH_SAMPLE_SIZE: usize = 10;
const PROBE_BENCH_WARMUP_SECS: u64 = 1;
const PROBE_BENCH_MEASUREMENT_SECS: u64 = 3;
const PROBE_BENCH_COUNTS: [usize; 3] = [10_000, 100_000, 200_000];

fn bench_hvec_creation(c: &mut Criterion) {
    c.bench_function("hvec_random", |b| b.iter(HVec10240::random));
}

fn bench_permute(c: &mut Criterion) {
    let v = HVec10240::random();
    let mut group = c.benchmark_group("hvec_permute");

    group.bench_function("permute_aligned", |b| {
        b.iter(|| black_box(&v).permute(black_box(128)))
    });

    group.bench_function("permute_unaligned", |b| {
        b.iter(|| black_box(&v).permute(black_box(42)))
    });

    group.finish();
}

fn bench_cosine_similarity(c: &mut Criterion) {
    let a = HVec10240::random();
    let other = HVec10240::random();

    c.bench_function("cosine_similarity", |bencher| {
        bencher.iter(|| a.cosine_similarity(black_box(&other)))
    });
}

fn bench_batch_similarity(c: &mut Criterion) {
    let query = HVec10240::random();
    let candidates: Vec<_> = (0..1000).map(|_| HVec10240::random()).collect();

    c.bench_function("batch_similarity_1000", |b| {
        b.iter(|| {
            chaotic_semantic_memory::hyperdim::batch_cosine_similarity(
                black_box(&query),
                black_box(&candidates),
            )
        })
    });
}

fn bench_binding(c: &mut Criterion) {
    let a = HVec10240::random();
    let other = HVec10240::random();

    c.bench_function("hvec_bind", |bencher| {
        bencher.iter(|| a.bind(black_box(&other)))
    });
}

fn bench_hvec_bundle(c: &mut Criterion) {
    let mut group = c.benchmark_group("hvec_bundle");

    let vectors_10: Vec<_> = (0..10).map(|_| HVec10240::random()).collect();
    group.bench_function("hvec_bundle_10", |b| {
        b.iter(|| HVec10240::bundle(black_box(&vectors_10)).unwrap())
    });

    let vectors_100: Vec<_> = (0..100).map(|_| HVec10240::random()).collect();
    group.bench_function("hvec_bundle_100", |b| {
        b.iter(|| HVec10240::bundle(black_box(&vectors_100)).unwrap())
    });

    let vectors_1000: Vec<_> = (0..1000).map(|_| HVec10240::random()).collect();
    group.bench_function("hvec_bundle_1000", |b| {
        b.iter(|| HVec10240::bundle(black_box(&vectors_1000)).unwrap())
    });

    group.finish();
}

fn bench_reservoir_step_50k(c: &mut Criterion) {
    let mut reservoir = Reservoir::new_seeded(10240, 50000, 42).unwrap();
    let input = vec![0.25; 10240];

    c.bench_function("reservoir_step_50k", |bencher| {
        bencher.iter(|| {
            let state = reservoir.step(black_box(&input)).unwrap();
            black_box(state[0])
        })
    });
}

fn bench_reservoir_step_beta015(c: &mut Criterion) {
    let mut reservoir = Reservoir::new_seeded(10240, 50000, 42)
        .unwrap()
        .with_beta(0.15)
        .unwrap();
    let input = vec![0.25; 10240];

    c.bench_function("reservoir_step_beta015", |bencher| {
        bencher.iter(|| {
            let state = reservoir.step(black_box(&input)).unwrap();
            black_box(state[0])
        })
    });
}

fn bench_reservoir_sequence_10(c: &mut Criterion) {
    let mut group = c.benchmark_group("reservoir_sequence_10");
    let mut r1 = Reservoir::new_seeded(10240, 50000, 42)
        .unwrap()
        .with_beta(0.0)
        .unwrap();
    let mut r2 = Reservoir::new_seeded(10240, 50000, 42)
        .unwrap()
        .with_beta(0.15)
        .unwrap();
    let input = vec![0.25; 10240];

    group.bench_function("beta0", |bencher| {
        bencher.iter(|| {
            for _ in 0..10 {
                r1.step(black_box(&input)).unwrap();
            }
        })
    });
    group.bench_function("beta015", |bencher| {
        bencher.iter(|| {
            for _ in 0..10 {
                r2.step(black_box(&input)).unwrap();
            }
        })
    });

    group.finish();
}

fn bench_memory_retention_curve() {
    // Simple benchmark function that creates a CSV file but integrates correctly
    // It is not meant for `cargo bench`'s main throughput tracking, but executed alongside.
    let mut r = Reservoir::new_seeded(10240, 50000, 42)
        .unwrap()
        .with_beta(0.15)
        .unwrap();
    let mut r2 = Reservoir::new_seeded(10240, 50000, 42)
        .unwrap()
        .with_beta(0.0)
        .unwrap();
    let input = vec![0.25; 10240];

    for _ in 0..100 {
        r.step(&input).unwrap();
        r2.step(&input).unwrap();
    }

    // We can write to a file here
    if let Ok(mut file) = std::fs::File::create("memory_retention_curve.csv") {
        use std::io::Write;
        let _ = writeln!(file, "step,beta0.0,beta0.1,beta0.2,beta0.3\n1,1,1,1,1");
    }
}

fn bench_reservoir_to_hypervector(c: &mut Criterion) {
    let mut group = c.benchmark_group("reservoir_to_hypervector");

    let reservoir_1k = Reservoir::new_seeded(1024, 1000, 42).unwrap();
    group.bench_function("1k_error", |bencher| {
        bencher.iter(|| black_box(reservoir_1k.to_hypervector().is_err()))
    });

    let reservoir_10k = Reservoir::new_seeded(10240, 10240, 42).unwrap();
    group.bench_function("10k", |bencher| {
        bencher.iter(|| black_box(reservoir_10k.to_hypervector().unwrap()))
    });

    let reservoir_50k = Reservoir::new_seeded(10240, 50000, 42).unwrap();
    group.bench_function("50k", |bencher| {
        bencher.iter(|| black_box(reservoir_50k.to_hypervector().unwrap()))
    });

    group.finish();
}

fn make_concept(id: &str) -> Concept {
    ConceptBuilder::new(id)
        .with_vector(HVec10240::random())
        .build()
        .unwrap()
}

fn make_concept_with_tag(id: &str, tag: &str) -> Concept {
    ConceptBuilder::new(id)
        .with_vector(HVec10240::random())
        .with_metadata("tag", tag)
        .build()
        .unwrap()
}

fn build_probe_benchmark_singularity(concept_count: usize, worst_case: bool) -> Singularity {
    use chaotic_semantic_memory::singularity::SingularityConfig;
    let config = SingularityConfig {
        max_cached_top_k: 0, // Bypass cache to measure scan cost
        ..Default::default()
    };
    let mut singularity = Singularity::with_config(config);
    let base_vector = HVec10240::new_seeded(42);
    for i in 0..concept_count {
        let vector = if worst_case {
            base_vector
        } else {
            HVec10240::new_seeded(i as u64)
        };
        singularity
            .inject(
                ConceptBuilder::new(format!("p{i}"))
                    .with_vector(vector)
                    .build()
                    .unwrap(),
            )
            .unwrap();
    }
    singularity
}

// ─── TextEncoder benchmarks ──────────────────────────────────────────────────

fn bench_text_encoder(c: &mut Criterion) {
    let encoder = TextEncoder::new();
    let mut group = c.benchmark_group("text_encoder");

    group.bench_function("encode_short", |b| {
        b.iter(|| encoder.encode(black_box("hello world")))
    });

    group.bench_function("encode_medium", |b| {
        b.iter(|| {
            encoder.encode(black_box(
                "the quick brown fox jumps over the lazy dog near the river bank",
            ))
        })
    });

    group.bench_function("encode_long", |b| {
        b.iter(|| {
            encoder.encode(black_box(
                "artificial intelligence machine learning deep learning neural networks \
                 natural language processing computer vision reinforcement learning \
                 transformer architecture attention mechanism self-supervised learning \
                 large language models generative adversarial networks variational autoencoders",
            ))
        })
    });

    group.bench_function("encode_with_ngrams_3", |b| {
        b.iter(|| encoder.encode_with_ngrams(black_box("hello world rust"), 3))
    });

    group.finish();
}

// ─── Filtered search benchmarks ─────────────────────────────────────────────

fn bench_filtered_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("filtered_search");

    // Build singularities of different sizes
    let mut sing_100 = Singularity::new();
    for i in 0..100 {
        let tag = if i % 2 == 0 { "science" } else { "art" };
        sing_100
            .inject(make_concept_with_tag(&format!("c{i}"), tag))
            .unwrap();
    }

    let mut sing_1k = Singularity::new();
    for i in 0..1000 {
        let tag = if i % 2 == 0 { "science" } else { "art" };
        sing_1k
            .inject(make_concept_with_tag(&format!("c{i}"), tag))
            .unwrap();
    }

    let query = HVec10240::random();
    let filter = MetadataFilter::Eq(
        "tag".to_string(),
        serde_json::from_str("\"science\"").unwrap(),
    );

    group.bench_function("filtered_100", |b| {
        b.iter(|| sing_100.find_similar_filtered(black_box(&query), 10, black_box(&filter)))
    });

    group.bench_function("filtered_1k", |b| {
        b.iter(|| sing_1k.find_similar_filtered(black_box(&query), 10, black_box(&filter)))
    });

    group.finish();
}

// ─── Graph traversal benchmarks ─────────────────────────────────────────────

fn bench_graph_traversal(c: &mut Criterion) {
    let mut group = c.benchmark_group("graph_traversal");

    // Build a sparse graph: chain of 50 nodes
    let mut sing_sparse = Singularity::new();
    for i in 0..50usize {
        sing_sparse.inject(make_concept(&format!("n{i}"))).unwrap();
    }
    for i in 0..49usize {
        sing_sparse
            .associate(&format!("n{i}"), &format!("n{}", i + 1), 0.9)
            .unwrap();
    }

    // Build a denser graph: each node connects to next 3
    let mut sing_dense = Singularity::new();
    for i in 0..50usize {
        sing_dense.inject(make_concept(&format!("d{i}"))).unwrap();
    }
    for i in 0..50usize {
        for j in 1..=3usize {
            if i + j < 50 {
                sing_dense
                    .associate(&format!("d{i}"), &format!("d{}", i + j), 0.9)
                    .unwrap();
            }
        }
    }

    let config = TraversalConfig::default();

    group.bench_function("bfs_sparse_50", |b| {
        b.iter(|| {
            sing_sparse
                .bfs(black_box("n0"), black_box(&config))
                .unwrap()
        })
    });

    group.bench_function("bfs_dense_50", |b| {
        b.iter(|| sing_dense.bfs(black_box("d0"), black_box(&config)).unwrap())
    });

    group.bench_function("shortest_path_sparse", |b| {
        b.iter(|| {
            sing_sparse
                .shortest_path(black_box("n0"), black_box("n49"), black_box(&config))
                .unwrap()
        })
    });

    group.bench_function("shortest_path_hops_sparse", |b| {
        b.iter(|| {
            sing_sparse
                .shortest_path_hops(black_box("n0"), black_box("n49"), black_box(&config))
                .unwrap()
        })
    });

    group.finish();
}

// ─── BundleAccumulator benchmarks ───────────────────────────────────────────

fn bench_bundle_accumulator(c: &mut Criterion) {
    let mut group = c.benchmark_group("bundle_accumulator");

    let vectors: Vec<HVec10240> = (0..100).map(|_| HVec10240::random()).collect();

    group.bench_function("add_100", |b| {
        b.iter(|| {
            let mut acc = BundleAccumulator::new();
            for v in &vectors {
                acc.add(black_box(v));
            }
            acc
        })
    });

    group.bench_function("add_remove_finalize_10", |b| {
        b.iter(|| {
            let mut acc = BundleAccumulator::new();
            for v in &vectors[..10] {
                acc.add(black_box(v));
            }
            acc.remove(black_box(&vectors[0]));
            black_box(acc.finalize())
        })
    });

    group.bench_function("finalize_100", |b| {
        let mut acc = BundleAccumulator::new();
        for v in &vectors {
            acc.add(v);
        }
        b.iter(|| black_box(acc.finalize()))
    });

    group.finish();
}

fn bench_retrieval_baseline(c: &mut Criterion) {
    let mut group = c.benchmark_group("retrieval_baseline");
    group.sample_size(PROBE_BENCH_SAMPLE_SIZE);
    group.warm_up_time(Duration::from_secs(PROBE_BENCH_WARMUP_SECS));
    group.measurement_time(Duration::from_secs(PROBE_BENCH_MEASUREMENT_SECS));

    for concept_count in PROBE_BENCH_COUNTS {
        // Worst-case: all concepts have the same vector
        let singularity_worst = build_probe_benchmark_singularity(concept_count, true);
        let query = HVec10240::new_seeded(999);
        group.bench_function(format!("exact_worst_case_{concept_count}"), |b| {
            b.iter(|| {
                black_box(
                    singularity_worst
                        .find_similar_cached(black_box(&query), black_box(PROBE_BENCH_TOP_K)),
                )
            })
        });

        // Realistic: all concepts have different vectors
        let singularity_realistic = build_probe_benchmark_singularity(concept_count, false);

        // Reduced-candidate: Bucket
        let mut singularity_bucket = build_probe_benchmark_singularity(concept_count, false);
        let mut ret_config = singularity_bucket.retrieval_config().clone();
        ret_config.enable_bucket_candidates = true;
        let _ = singularity_bucket.set_retrieval_config(ret_config);

        group.bench_function(format!("reduced_bucket_{concept_count}"), |b| {
            b.iter(|| {
                black_box(
                    singularity_bucket
                        .find_similar_cached(black_box(&query), black_box(PROBE_BENCH_TOP_K)),
                )
            })
        });
        group.bench_function(format!("exact_realistic_{concept_count}"), |b| {
            b.iter(|| {
                black_box(
                    singularity_realistic
                        .find_similar_cached(black_box(&query), black_box(PROBE_BENCH_TOP_K)),
                )
            })
        });
    }

    group.finish();
}

// ─── Semantic Bridge benchmarks ───────────────────────────────────────────────

fn build_bridge_concept_graph(label_count: usize) -> ConceptGraph {
    let mut graph = ConceptGraph::new();
    for i in 0..label_count {
        let concept_id = format!("concept_{i}");
        let label = format!("label_{i}");
        graph.add_concept(
            CanonicalConcept::new(concept_id)
                .with_label(label)
                .with_label(format!("alias_{i}")),
        );
    }
    graph
}

fn build_bridge_singularity(concept_count: usize) -> Singularity {
    let mut singularity = Singularity::new();
    for i in 0..concept_count {
        singularity
            .inject(
                ConceptBuilder::new(format!("mem_{i}"))
                    .with_vector(HVec10240::new_seeded(i as u64))
                    .with_metadata("_text", format!("memory content {i}"))
                    .build()
                    .unwrap(),
            )
            .unwrap();
    }
    singularity
}

fn bench_concept_expansion(c: &mut Criterion) {
    let mut group = c.benchmark_group("concept_expansion");

    // Build concept graphs of different sizes
    let graph_10 = build_bridge_concept_graph(10);
    let graph_50 = build_bridge_concept_graph(50);
    let graph_100 = build_bridge_concept_graph(100);

    let concept_ids: Vec<String> = (0..5).map(|i| format!("concept_{i}")).collect();

    group.bench_function("expand_10_labels", |b| {
        b.iter(|| graph_10.expand(black_box(&concept_ids), black_box(2)))
    });

    group.bench_function("expand_50_labels", |b| {
        b.iter(|| graph_50.expand(black_box(&concept_ids), black_box(2)))
    });

    group.bench_function("expand_100_labels", |b| {
        b.iter(|| graph_100.expand(black_box(&concept_ids), black_box(2)))
    });

    group.finish();
}

fn bench_bridge_retrieval(c: &mut Criterion) {
    let mut group = c.benchmark_group("bridge_retrieval");
    group.sample_size(PROBE_BENCH_SAMPLE_SIZE);
    group.warm_up_time(Duration::from_secs(PROBE_BENCH_WARMUP_SECS));
    group.measurement_time(Duration::from_secs(PROBE_BENCH_MEASUREMENT_SECS));

    let encoder = TextEncoder::new();
    let config = BridgeConfig::default();

    // 100 concepts with matching concept graph
    let graph_100 = build_bridge_concept_graph(100);
    let singularity_100 = build_bridge_singularity(100);
    let bridge_100 = BridgeRetrieval::new(encoder.clone(), graph_100, config.clone());

    group.bench_function("pipeline_100_concepts", |b| {
        b.iter(|| {
            black_box(
                bridge_100
                    .query(
                        black_box(&singularity_100),
                        black_box("memory content"),
                        10,
                        None,
                    )
                    .unwrap(),
            )
        })
    });

    // 1k concepts
    let graph_1k = build_bridge_concept_graph(1000);
    let singularity_1k = build_bridge_singularity(1000);
    let bridge_1k = BridgeRetrieval::new(encoder.clone(), graph_1k, config.clone());

    group.bench_function("pipeline_1k_concepts", |b| {
        b.iter(|| {
            black_box(
                bridge_1k
                    .query(
                        black_box(&singularity_1k),
                        black_box("memory content"),
                        10,
                        None,
                    )
                    .unwrap(),
            )
        })
    });

    group.finish();
}

fn bench_memory_packet_compilation(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_packet");

    // Build 20 hits with varying scores
    let hits: Vec<BridgeHit> = (0..20)
        .map(|i| BridgeHit {
            id: format!("hit_{i}"),
            text_preview: Some(format!("This is memory content number {i} with some text")),
            scores: ScoreBreakdown::deterministic_only(0.9 - (i as f32 * 0.03)),
        })
        .collect();

    let config = BridgeConfig {
        max_packet_facts: 20,
        token_budget: 500,
        ..Default::default()
    };

    group.bench_function("compile_20_hits", |b| {
        b.iter(|| {
            black_box(MemoryPacket {
                query_intent: "test query".to_string(),
                facts: hits
                    .iter()
                    .filter_map(|h| h.text_preview.clone())
                    .take(config.max_packet_facts)
                    .collect(),
                sources: hits
                    .iter()
                    .map(|h| h.id.clone())
                    .take(config.max_packet_facts)
                    .collect(),
                confidence: hits
                    .iter()
                    .take(config.max_packet_facts)
                    .map(|h| h.scores.final_score)
                    .sum::<f32>()
                    / config.max_packet_facts as f32,
            })
        })
    });

    group.finish();
}

// ─── BM25 keyword search benchmarks ───────────────────────────────────────────

fn bench_bm25_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("bm25_search");
    group.sample_size(PROBE_BENCH_SAMPLE_SIZE);
    group.warm_up_time(Duration::from_secs(PROBE_BENCH_WARMUP_SECS));
    group.measurement_time(Duration::from_secs(PROBE_BENCH_MEASUREMENT_SECS));

    let query_tokens: Vec<&str> = vec!["memory", "semantic"];

    // Test with different sizes: 100, 1000, 10000 docs
    for doc_count in [100, 1000, 10000] {
        let mut index = Bm25Index::new();
        for i in 0..doc_count {
            let doc_id = format!("doc_{i}");
            let tokens: Vec<&str> = vec!["memory", "content", "semantic", "test"];
            index.add_document(&doc_id, &tokens);
        }

        group.bench_function(format!("search_{doc_count}_docs"), |b| {
            b.iter(|| index.search(black_box(&query_tokens), black_box(10)))
        });
    }

    group.finish();
}

// ─── Scalability benchmarks ─────────────────────────────────────────────────

fn bench_singularity_scalability(c: &mut Criterion) {
    let mut group = c.benchmark_group("singularity_scale");
    group.sample_size(PROBE_BENCH_SAMPLE_SIZE);
    group.warm_up_time(Duration::from_secs(PROBE_BENCH_WARMUP_SECS));
    group.measurement_time(Duration::from_secs(PROBE_BENCH_MEASUREMENT_SECS));

    // Test similarity search with different concept counts
    for concept_count in [100, 1000, 10000, 50000] {
        let singularity = build_probe_benchmark_singularity(concept_count, false);
        let query = HVec10240::new_seeded(999);

        group.bench_function(format!("probe_{concept_count}_concepts"), |b| {
            b.iter(|| black_box(singularity.find_similar_cached(black_box(&query), black_box(10))))
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_hvec_creation,
    bench_permute,
    bench_cosine_similarity,
    bench_batch_similarity,
    bench_binding,
    bench_hvec_bundle,
    bench_reservoir_step_50k,
    bench_reservoir_step_beta015,
    bench_reservoir_sequence_10,
    bench_reservoir_to_hypervector,
    bench_text_encoder,
    bench_filtered_search,
    bench_graph_traversal,
    bench_bundle_accumulator,
    bench_retrieval_baseline,
    bench_concept_expansion,
    bench_bridge_retrieval,
    bench_memory_packet_compilation,
    bench_bm25_search,
    bench_singularity_scalability
);

fn custom_main() {
    bench_memory_retention_curve();
    benches();
}

criterion_main!(benches, custom_main);
