use chaotic_semantic_memory::retrieval::bm25::Bm25Index;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_bm25_search(c: &mut Criterion) {
    let mut index = Bm25Index::new();
    let tokens = [
        "hello",
        "world",
        "rust",
        "performance",
        "optimization",
        "search",
        "index",
        "bm25",
        "algorithm",
        "ranking",
    ];

    // Index 1000 documents
    for i in 0..1000 {
        let mut doc_tokens = Vec::new();
        for (j, &token) in tokens.iter().enumerate().take(10) {
            if (i + j) % 2 == 0 {
                doc_tokens.push(token);
            }
        }
        index.add_document(&format!("doc_{i}"), &doc_tokens);
    }

    let query = vec!["hello", "rust", "bm25"];

    c.bench_function("bm25_search_1000", |b| {
        b.iter(|| index.search(black_box(&query), black_box(10)))
    });
}

fn bench_bm25_search_100000(c: &mut Criterion) {
    let mut index = Bm25Index::new();
    let tokens = [
        "hello",
        "world",
        "rust",
        "performance",
        "optimization",
        "search",
        "index",
        "bm25",
        "algorithm",
        "ranking",
    ];

    // Index 100000 documents
    for i in 0..100000 {
        let mut doc_tokens = Vec::new();
        for (j, &token) in tokens.iter().enumerate().take(10) {
            if (i + j) % 2 == 0 {
                doc_tokens.push(token);
            }
        }
        index.add_document(&format!("doc_{i}"), &doc_tokens);
    }

    let query = vec!["hello", "rust", "bm25"];

    c.bench_function("bm25_search_100000", |b| {
        b.iter(|| index.search(black_box(&query), black_box(10)))
    });
}

fn bench_bm25_search_10000(c: &mut Criterion) {
    let mut index = Bm25Index::new();
    let tokens = [
        "hello",
        "world",
        "rust",
        "performance",
        "optimization",
        "search",
        "index",
        "bm25",
        "algorithm",
        "ranking",
    ];

    // Index 10000 documents
    for i in 0..10000 {
        let mut doc_tokens = Vec::new();
        for (j, &token) in tokens.iter().enumerate().take(10) {
            if (i + j) % 2 == 0 {
                doc_tokens.push(token);
            }
        }
        index.add_document(&format!("doc_{i}"), &doc_tokens);
    }

    let query = vec!["hello", "rust", "bm25"];

    c.bench_function("bm25_search_10000", |b| {
        b.iter(|| index.search(black_box(&query), black_box(10)))
    });
}

/// Selective query benchmark: 100K docs, 1000-token vocabulary, ~20 tokens/doc.
/// Query terms appear in ~2% of documents, demonstrating O(T) where T << N.
fn bench_bm25_search_selective_100000(c: &mut Criterion) {
    let mut index = Bm25Index::new();
    // Large vocabulary: 1000 distinct terms
    let vocab: Vec<String> = (0..1000).map(|i| format!("term_{i}")).collect();

    // Index 100K documents with ~20 tokens each (2% coverage per term)
    for i in 0..100_000u64 {
        let doc_tokens: Vec<&str> = (0..20)
            .map(|j| vocab[(i as usize * 7 + j * 13) % 1000].as_str())
            .collect();
        index.add_document(&format!("doc_{i}"), &doc_tokens);
    }

    // Query with rare terms that hit few documents
    let query = vec![vocab[999].as_str(), vocab[997].as_str()];

    c.bench_function("bm25_search_selective_100000", |b| {
        b.iter(|| index.search(black_box(&query), black_box(10)))
    });
}

fn bench_bm25_replacement(c: &mut Criterion) {
    let mut index = Bm25Index::new();
    let tokens = vec!["hello", "world", "rust"];

    // Index 1000 documents
    for i in 0..1000 {
        index.add_document(&format!("doc_{i}"), &tokens);
    }

    c.bench_function("bm25_replace_doc_1000", |b| {
        b.iter(|| {
            // Replacing doc_0 repeatedly triggers remove_document_at
            index.add_document(black_box("doc_0"), black_box(&tokens));
        })
    });
}

criterion_group!(
    benches,
    bench_bm25_search,
    bench_bm25_search_10000,
    bench_bm25_search_100000,
    bench_bm25_search_selective_100000,
    bench_bm25_replacement
);
criterion_main!(benches);
