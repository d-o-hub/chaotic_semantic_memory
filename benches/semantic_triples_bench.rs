use chaotic_semantic_memory::semantic_triples::{SemanticTriple, StructuredMemoryPayload};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_semantic_triples(c: &mut Criterion) {
    let payload = StructuredMemoryPayload {
        raw_summary:
            "User discussed their preferences for code generation formatting and UI layout."
                .to_string(),
        triples: vec![
            SemanticTriple::new("User", "prefers", "dark mode"),
            SemanticTriple::new("User", "dislikes", "tabs"),
            SemanticTriple::new("User", "requests", "Python snippets"),
            SemanticTriple::new("System", "generates", "code"),
        ],
    };

    c.bench_function("format_context_string", |b| {
        b.iter(|| black_box(payload.to_context_string()))
    });
}

criterion_group!(benches, bench_semantic_triples);
criterion_main!(benches);
