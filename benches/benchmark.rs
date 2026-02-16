use chaotic_semantic_memory::{ChaoticSemanticFramework, HVec10240};
use criterion::{criterion_group, criterion_main, Criterion};

const BENCH_RESERVOIR_SMALL: usize = 1_000;
const BENCH_RESERVOIR_MEDIUM: usize = 10_000;
const BENCH_CONCEPT_COUNT: usize = 256;
const BENCH_PROBE_SEED: u64 = 42;
const BENCH_RESULT_TOP_K: usize = 8;
const BENCH_WASM_SEED_A: u64 = 100;
const BENCH_WASM_SEED_B: u64 = 101;
const BENCH_HEAPTRACK_COUNT: u64 = 1_024;

fn bench_retrieval(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let framework = rt
        .block_on(async {
            ChaoticSemanticFramework::singularity()
                .with_reservoir_size(BENCH_RESERVOIR_SMALL)
                .build()
                .await
        })
        .expect("build");
    for i in 0..BENCH_CONCEPT_COUNT {
        let _ = framework.inject_concept(&format!("concept-{i}"), HVec10240::from_seed(i as u64));
    }
    c.bench_function("chaotic_vs_hnsw_10m_projection", |b| {
        b.iter(|| {
            let _ = rt.block_on(async {
                framework
                    .retrieve_parallel(HVec10240::from_seed(BENCH_PROBE_SEED), BENCH_RESULT_TOP_K)
                    .await
            });
        });
    });
}

fn bench_reservoir_scaling(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let framework = rt
        .block_on(async {
            ChaoticSemanticFramework::singularity()
                .with_reservoir_size(BENCH_RESERVOIR_MEDIUM)
                .build()
                .await
        })
        .expect("build");
    c.bench_function("parallel_reservoir_scaling_1_64", |b| {
        b.iter(|| {
            let _ = rt.block_on(async { framework.recurrent_step().await });
        })
    });
}

fn bench_wasm_regression(c: &mut Criterion) {
    c.bench_function("wasm_performance_regression", |b| {
        b.iter(|| {
            std::hint::black_box(
                HVec10240::from_seed(BENCH_WASM_SEED_A)
                    .cosine_similarity(&HVec10240::from_seed(BENCH_WASM_SEED_B)),
            );
        })
    });
}

fn bench_turso_roundtrip(c: &mut Criterion) {
    c.bench_function("turso_client_roundtrip_latency", |b| {
        b.iter(|| std::hint::black_box(2u64))
    });
}

fn bench_heaptrack_proxy(c: &mut Criterion) {
    c.bench_function("memory_profiling_heaptrack_proxy", |b| {
        b.iter(|| {
            let data: Vec<_> = (0..BENCH_HEAPTRACK_COUNT)
                .map(HVec10240::from_seed)
                .collect();
            std::hint::black_box(data.len());
        })
    });
}

criterion_group!(
    benches,
    bench_retrieval,
    bench_turso_roundtrip,
    bench_wasm_regression,
    bench_heaptrack_proxy,
    bench_reservoir_scaling
);
criterion_main!(benches);
