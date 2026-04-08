## 2025-05-14 - Bit counting optimization in bundling operations
**Learning:** For 10240-bit hypervectors, iterating over every single bit during bundling/accumulation is a significant bottleneck. Using `trailing_zeros()` to iterate only over set bits reduced the `hvec_bundle_1000` time from ~17.7ms to ~4.0ms (~77% speedup) and `add_100` from ~6.79ms to ~1.07ms (~84% speedup).
**Action:** Always prefer popcount-based iteration (e.g., `trailing_zeros()` with `val &= val - 1`) over fixed-length bit-scanning for sparse or semi-dense bitsets.

## 2025-05-15 - BM25 Index document removal and search optimization
**Learning:** `Bm25Index::remove_document_at` was $O(n)$ due to `Vec::remove` and a full hashmap rebuild, making document replacements very slow. Switching to `swap_remove` and incremental hashmap updates reduced replacement time for a 1000-doc index from ~75µs to ~613ns (~99% speedup). Additionally, hoisting IDF calculations and constants out of the document scoring loop in `search` gave a ~33% speedup (~208µs to ~138µs).
**Action:** Always use `swap_remove` for fast removal if order doesn't matter, and ensure loop invariants (especially expensive ones like `ln()`) are hoisted out of per-document scoring loops.
