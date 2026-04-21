[The content of progress/LEARNINGS.md up to before my additions]

## BM25 Scoring Optimization (2026-03-10)
- **Optimization**: Hoisted document-length normalization constants and pre-calculated weighted IDF values in the search hot path.
- **Performance**: Verified ~4% speedup in `bm25_search_1000` benchmark.
- **Instruction Mapping**: Utilized `f32::mul_add` for potential FMA hardware acceleration.
- **API Flexibility**: Refactored `add_document` to be generic over `AsRef<str>`, enabling it to accept both slices and owned collections (like `Vec<String>`) without extra clones.

## BM25 Performance vs. Persistence Serialization (2026-03-10)
- **Constraint**: `Arc<str>` interning for terms significantly reduces memory and improves performance, but it complicates `Serialize`/`Deserialize` when those types are used in the persistence layer.
- **Decision**: Removed `Serialize`/`Deserialize` from `Bm25Index` and `Document` as they are currently used as in-memory transient indices built from persisted concepts, rather than being persisted themselves. This preserves the high-performance `Arc<str>` implementation.

## BM25 Scoring Optimization and WASM Fix (2026-03-10)
- **Problem**: `rayon` import was not gated for WASM targets, causing CI failure.
- **Fix**: Correctly gated `rayon` import with `#[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]`.
- **Optimization**: Restored `Arc<str>` interning for term storage in `Bm25Index` to minimize allocations and memory usage.
- **Serialization**: Removed `Serialize`/`Deserialize` from `Bm25Index` and `Document` as they are not currently persisted and `Arc<str>` does not implement `Deserialize` by default with the project's current dependencies.
- **API**: Kept `add_document` generic over `AsRef<str>` to support varied input types from CLI and other modules.

## BM25 Scoring Optimization and WASM Fix (2026-03-10)
- **Problem**: `rayon` import was not gated for WASM targets, causing CI failure.
- **Fix**: Correctly gated `rayon` import with `#[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]`.
- **Optimization**: Restored `Arc<str>` interning for term storage in `Bm25Index` to minimize allocations and memory usage.
- **Serialization**: Removed `Serialize`/`Deserialize` from `Bm25Index` and `Document` as they are currently used as in-memory transient indices built from persisted concepts, rather than being persisted themselves. This preserves the high-performance `Arc<str>` implementation.
- **API**: Kept `add_document` generic over `AsRef<str>` to support varied input types from CLI and other modules.
