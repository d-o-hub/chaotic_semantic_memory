use chaotic_semantic_memory::encoder::TextEncoder;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_encode_short(c: &mut Criterion) {
    let encoder = TextEncoder::new();
    c.bench_function("encode_short_3tok", |b| {
        b.iter(|| encoder.encode(black_box("hello world rust")))
    });
}

fn bench_encode_long(c: &mut Criterion) {
    let encoder = TextEncoder::new();
    let text = "the quick brown fox jumps over the lazy dog and then runs across the field towards the river bank where it finally stops to drink some water";
    c.bench_function("encode_long_27tok", |b| {
        b.iter(|| encoder.encode(black_box(text)))
    });
}

fn bench_encode_batch(c: &mut Criterion) {
    let encoder = TextEncoder::new();
    let docs: Vec<&str> = vec![
        "reservoir computing overview",
        "chaotic dynamics in neural systems",
        "hyperdimensional binary vectors",
        "semantic memory retrieval",
        "approximate nearest neighbor search",
    ];

    c.bench_function("encode_batch_5docs", |b| {
        b.iter(|| {
            for doc in &docs {
                let _ = black_box(encoder.encode(doc));
            }
        })
    });
}

criterion_group!(
    benches,
    bench_encode_short,
    bench_encode_long,
    bench_encode_batch
);
criterion_main!(benches);
