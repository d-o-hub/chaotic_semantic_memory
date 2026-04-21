[The content of progress/LEARNINGS.md up to before my additions]

## BM25 Scoring Optimization (2026-03-10)
- **Optimization**: Hoisted document-length normalization constants and pre-calculated weighted IDF values in the search hot path.
- **Performance**: Verified ~4% speedup in `bm25_search_1000` benchmark.
- **Instruction Mapping**: Utilized `f32::mul_add` for potential FMA hardware acceleration.
- **API Flexibility**: Refactored `add_document` to be generic over `AsRef<str>`, enabling it to accept both slices and owned collections (like `Vec<String>`) without extra clones.
