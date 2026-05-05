use chaotic_semantic_memory::HVec10240;
use chaotic_semantic_memory::persistence::Persistence;
use chaotic_semantic_memory::singularity::Concept;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::collections::HashMap;
use tempfile::NamedTempFile;

fn make_concept(id: &str) -> Concept {
    Concept {
        id: id.to_string(),
        vector: HVec10240::random(),
        metadata: HashMap::new(),
        created_at: 1,
        modified_at: 1,
        expires_at: None,
        canonical_concept_ids: Vec::new(),
    }
}

fn make_concepts(count: usize, prefix: &str) -> Vec<Concept> {
    (0..count)
        .map(|i| make_concept(&format!("{prefix}-{i}")))
        .collect()
}

fn make_concept_with_metadata(id: &str) -> Concept {
    use serde_json::json;
    let mut metadata = HashMap::new();
    metadata.insert("name".to_string(), json!("test"));
    metadata.insert("count".to_string(), json!(42));
    metadata.insert(
        "nested".to_string(),
        json!({"inner": "value", "number": 123}),
    );
    Concept {
        id: id.to_string(),
        vector: HVec10240::random(),
        metadata,
        created_at: 1,
        modified_at: 1,
        expires_at: None,
        canonical_concept_ids: Vec::new(),
    }
}

fn bench_persistence_cold(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("persistence_cold_start", |b| {
        b.iter(|| {
            let temp = NamedTempFile::new().unwrap();
            let path = temp.path().to_str().unwrap();
            rt.block_on(async {
                let persistence = Persistence::new_local(path).await.unwrap();
                black_box(persistence)
            })
        })
    });
}

fn bench_persistence_warm(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap().to_string();
    let persistence = rt.block_on(async { Persistence::new_local(&path).await.unwrap() });

    let mut group = c.benchmark_group("persistence_warm");

    group.bench_function("save_concept", |b| {
        b.iter(|| {
            let concept = make_concept_with_metadata("bench-save");
            rt.block_on(async {
                persistence
                    .save_concept("_default", black_box(&concept))
                    .await
                    .unwrap();
            })
        })
    });

    group.bench_function("load_concept", |b| {
        let concept = make_concept_with_metadata("bench-load");
        rt.block_on(async {
            persistence
                .save_concept("_default", &concept)
                .await
                .unwrap();
        });
        b.iter(|| {
            rt.block_on(async {
                let loaded = persistence
                    .load_concept("_default", black_box("bench-load"))
                    .await
                    .unwrap();
                black_box(loaded)
            })
        })
    });

    group.finish();
}

fn bench_delete_concept(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("delete_concept", |b| {
        b.iter(|| {
            let temp = NamedTempFile::new().unwrap();
            let path = temp.path().to_str().unwrap();
            rt.block_on(async {
                let persistence = Persistence::new_local(path).await.unwrap();
                persistence
                    .save_concept("_default", &make_concept("to-delete"))
                    .await
                    .unwrap();
                persistence
                    .delete_concept("_default", black_box("to-delete"))
                    .await
                    .unwrap();
                black_box(persistence)
            })
        })
    });
}

fn bench_delete_concept_with_cascade(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("delete_concept_with_cascade", |b| {
        b.iter(|| {
            let temp = NamedTempFile::new().unwrap();
            let path = temp.path().to_str().unwrap();
            rt.block_on(async {
                let persistence = Persistence::new_local(path).await.unwrap();
                let concepts = make_concepts(10, "cascade");
                persistence
                    .save_concepts("_default", &concepts)
                    .await
                    .unwrap();
                for i in 0..9 {
                    persistence
                        .save_association("_default", "cascade-0", &format!("cascade-{i}"), 0.5)
                        .await
                        .unwrap();
                }
                persistence
                    .delete_concept("_default", black_box("cascade-0"))
                    .await
                    .unwrap();
                black_box(persistence)
            })
        })
    });
}

fn bench_save_concepts_batch(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("save_concepts_batch");

    for size in [10, 100, 1000] {
        group.bench_function(format!("{size}_concepts"), |b| {
            b.iter(|| {
                let concepts = make_concepts(size, "batch");
                let temp = NamedTempFile::new().unwrap();
                let path = temp.path().to_str().unwrap();
                rt.block_on(async {
                    let persistence = Persistence::new_local(path).await.unwrap();
                    persistence
                        .save_concepts("_default", black_box(&concepts))
                        .await
                        .unwrap();
                    black_box(persistence)
                })
            })
        });
    }

    group.finish();
}

fn bench_load_all_concepts(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("load_all_concepts");

    for size in [10, 100, 1000] {
        group.bench_function(format!("{size}_concepts"), |b| {
            b.iter(|| {
                let temp = NamedTempFile::new().unwrap();
                let path = temp.path().to_str().unwrap();
                rt.block_on(async {
                    let persistence = Persistence::new_local(path).await.unwrap();
                    let concepts = make_concepts(size, "load-all");
                    persistence
                        .save_concepts("_default", &concepts)
                        .await
                        .unwrap();
                    let loaded = persistence.load_all_concepts("_default").await.unwrap();
                    black_box(loaded)
                })
            })
        });
    }

    group.finish();
}

fn bench_save_association(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("save_association", |b| {
        b.iter(|| {
            let temp = NamedTempFile::new().unwrap();
            let path = temp.path().to_str().unwrap();
            rt.block_on(async {
                let persistence = Persistence::new_local(path).await.unwrap();
                persistence
                    .save_concept("_default", &make_concept("assoc-from"))
                    .await
                    .unwrap();
                persistence
                    .save_concept("_default", &make_concept("assoc-to"))
                    .await
                    .unwrap();
                persistence
                    .save_association(
                        "_default",
                        black_box("assoc-from"),
                        black_box("assoc-to"),
                        black_box(0.75),
                    )
                    .await
                    .unwrap();
                black_box(persistence)
            })
        })
    });
}

fn bench_load_associations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("load_associations");

    for assoc_count in [1, 10, 50] {
        group.bench_function(format!("{assoc_count}_associations"), |b| {
            b.iter(|| {
                let temp = NamedTempFile::new().unwrap();
                let path = temp.path().to_str().unwrap();
                rt.block_on(async {
                    let persistence = Persistence::new_local(path).await.unwrap();
                    persistence
                        .save_concept("_default", &make_concept("hub"))
                        .await
                        .unwrap();
                    for i in 0..assoc_count {
                        persistence
                            .save_concept("_default", &make_concept(&format!("spoke-{i}")))
                            .await
                            .unwrap();
                    }
                    for i in 0..assoc_count {
                        persistence
                            .save_association("_default", "hub", &format!("spoke-{i}"), 0.5)
                            .await
                            .unwrap();
                    }
                    let associations = persistence
                        .load_associations("_default", black_box("hub"))
                        .await
                        .unwrap();
                    black_box(associations)
                })
            })
        });
    }

    group.finish();
}

fn bench_crud_roundtrip(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("crud_roundtrip", |b| {
        b.iter(|| {
            let concept = make_concept_with_metadata("roundtrip");
            let temp = NamedTempFile::new().unwrap();
            let path = temp.path().to_str().unwrap();
            rt.block_on(async {
                let persistence = Persistence::new_local(path).await.unwrap();

                persistence
                    .save_concept("_default", black_box(&concept))
                    .await
                    .unwrap();
                let loaded = persistence
                    .load_concept("_default", black_box("roundtrip"))
                    .await
                    .unwrap()
                    .unwrap();
                black_box(&loaded);
                persistence
                    .delete_concept("_default", black_box("roundtrip"))
                    .await
                    .unwrap();
                let gone = persistence
                    .load_concept("_default", "roundtrip")
                    .await
                    .unwrap();
                black_box(gone)
            })
        })
    });
}

fn bench_crud_roundtrip_with_associations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("crud_roundtrip_with_associations", |b| {
        b.iter(|| {
            let temp = NamedTempFile::new().unwrap();
            let path = temp.path().to_str().unwrap();
            rt.block_on(async {
                let persistence = Persistence::new_local(path).await.unwrap();

                let concepts = make_concepts(5, "rt");
                persistence
                    .save_concepts("_default", black_box(&concepts))
                    .await
                    .unwrap();

                persistence
                    .save_association("_default", "rt-0", "rt-1", 0.8)
                    .await
                    .unwrap();
                persistence
                    .save_association("_default", "rt-0", "rt-2", 0.6)
                    .await
                    .unwrap();
                persistence
                    .save_association("_default", "rt-1", "rt-3", 0.4)
                    .await
                    .unwrap();

                let loaded = persistence
                    .load_concept("_default", "rt-0")
                    .await
                    .unwrap()
                    .unwrap();
                black_box(&loaded);

                let associations = persistence
                    .load_associations("_default", "rt-0")
                    .await
                    .unwrap();
                black_box(&associations);

                persistence
                    .delete_concept("_default", "rt-0")
                    .await
                    .unwrap();

                let remaining = persistence.load_all_concepts("_default").await.unwrap();
                black_box(remaining.len())
            })
        })
    });
}

fn bench_checkpoint(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("checkpoint_after_100_saves", |b| {
        b.iter(|| {
            let temp = NamedTempFile::new().unwrap();
            let path = temp.path().to_str().unwrap();
            rt.block_on(async {
                let persistence = Persistence::new_local(path).await.unwrap();
                let concepts = make_concepts(100, "ckpt");
                persistence
                    .save_concepts("_default", &concepts)
                    .await
                    .unwrap();
                persistence.checkpoint().await.unwrap();
                black_box(persistence)
            })
        })
    });
}

fn bench_persistence_concurrency(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("shared_store_concurrent_10_saves", |b| {
        b.iter(|| {
            let temp = NamedTempFile::new().unwrap();
            let path = temp.path().to_str().unwrap().to_string();

            rt.block_on(async {
                let persistence = Persistence::new_local(&path).await.unwrap();
                let persistence = std::sync::Arc::new(persistence);
                let mut handles = Vec::new();
                for i in 0..10 {
                    let p = std::sync::Arc::clone(&persistence);
                    handles.push(tokio::spawn(async move {
                        let concept = make_concept(&format!("concurrent-{i}"));
                        // Retry loop for bench stability
                        loop {
                            match p.save_concept("_default", &concept).await {
                                Ok(_) => break,
                                Err(e) if format!("{e:?}").contains("database is locked") => {
                                    tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                                }
                                Err(e) => panic!("Unexpected error: {e:?}"),
                            }
                        }
                    }));
                }

                for handle in handles {
                    handle.await.unwrap();
                }
            })
        })
    });
}

criterion_group!(
    benches,
    bench_persistence_cold,
    bench_persistence_warm,
    bench_persistence_concurrency,
    bench_delete_concept,
    bench_delete_concept_with_cascade,
    bench_save_concepts_batch,
    bench_load_all_concepts,
    bench_save_association,
    bench_load_associations,
    bench_crud_roundtrip,
    bench_crud_roundtrip_with_associations,
    bench_checkpoint,
);
criterion_main!(benches);
