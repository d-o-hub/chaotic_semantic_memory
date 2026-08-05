//! Criterion benchmark for the reduced-candidate retrieval path with graph
//! candidate generation enabled.
//!
//! Measures `find_similar_cached` end-to-end on a 500-node association graph
//! (20 edges per node). `top_k > max_cached_top_k` (100) forces the similarity
//! cache to be bypassed, so every iteration exercises the full pipeline:
//! exact seed scan -> graph BFS (`generate_graph_candidates`) -> candidate
//! scoring. The graph BFS is where the String-clone-elimination optimization
//! (borrowed `&str` instead of owned `String`) lands.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chaotic_semantic_memory::prelude::*;
use chaotic_semantic_memory::singularity::{Concept, Singularity, SingularityConfig};
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::collections::HashMap;

const NS: &str = "_bench";
// Above max_cached_top_k (100) so the cache lookup is skipped and the graph
// candidate path runs on every call.
const TOP_K: usize = 200;

fn build_graph(nodes: usize, edges_per_node: usize) -> Singularity<HVec10240> {
    let mut sing = Singularity::<HVec10240>::new(SingularityConfig::default());
    let config = RetrievalConfig {
        enable_graph_candidates: true,
        graph_depth: 3,
        graph_fanout: 20,
        ..Default::default()
    };
    sing.set_retrieval_config(config).unwrap();

    let vectors: Vec<HVec10240> = (0..nodes).map(|_| HVec10240::random()).collect();
    for (i, v) in vectors.into_iter().enumerate() {
        sing.inject(
            NS,
            Concept {
                id: format!("c{i:05}"),
                vector: v,
                metadata: HashMap::new(),
                created_at: 1,
                modified_at: 1,
                expires_at: None,
                canonical_concept_ids: Vec::new(),
            },
        )
        .unwrap();
    }

    // Deterministic pseudo-random directed graph (associations are user/LLM
    // defined, not ring-ordered): every node links to `edges_per_node`
    // quasi-random distinct targets, so a seed expands into a wide BFS with
    // minimal neighbor overlap. Deterministic so both A/B sides build an
    // identical graph.
    for i in 0..nodes {
        for j in 1..=edges_per_node {
            let to = (i * 7919 + j * 104_729) % nodes;
            if to != i {
                sing.associate(NS, &format!("c{i:05}"), &format!("c{to:05}"), 0.9)
                    .unwrap();
            }
        }
    }
    sing
}

fn bench_graph_candidates(c: &mut Criterion) {
    let sing = build_graph(500, 20);
    let query = HVec10240::random();
    let mut group = c.benchmark_group("graph_candidates");
    group.bench_function("find_similar_graph_500x20_d3", |b| {
        b.iter(|| {
            black_box(sing.find_similar_cached(NS, &query, TOP_K));
        });
    });
    group.finish();
}

criterion_group!(benches, bench_graph_candidates);
criterion_main!(benches);
