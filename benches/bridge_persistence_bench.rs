use chaotic_semantic_memory::persistence::Persistence;
use chaotic_semantic_memory::semantic_bridge::{CanonicalConcept, ConceptGraph};
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use tempfile::NamedTempFile;

const NS: &str = "_default";

fn make_large_graph(size: usize) -> ConceptGraph {
    let mut graph = ConceptGraph::new();
    for i in 0..size {
        let mut concept = CanonicalConcept::new(format!("concept-{}", i))
            .with_label(format!("label-{}-1", i))
            .with_label(format!("label-{}-2", i));

        if i > 0 {
            concept = concept.with_related(format!("concept-{}", i - 1));
        }
        graph.add_concept(concept);
    }
    graph
}

fn bench_save_concept_graph(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("bridge_persistence");

    for size in [10, 100, 500] {
        group.bench_function(format!("save_graph_{size}"), |b| {
            let graph = make_large_graph(size);
            b.iter(|| {
                let temp = NamedTempFile::new().unwrap();
                let path = temp.path().to_str().unwrap();
                rt.block_on(async {
                    let persistence = Persistence::new_local(path).await.unwrap();
                    persistence
                        .save_concept_graph(NS, black_box(&graph))
                        .await
                        .unwrap();
                    black_box(persistence)
                })
            })
        });
    }

    group.finish();
}

criterion_group!(benches, bench_save_concept_graph);
criterion_main!(benches);
