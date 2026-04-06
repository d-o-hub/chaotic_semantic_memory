## 2025-05-14 - Bit counting optimization in bundling operations
**Learning:** For 10240-bit hypervectors, iterating over every single bit during bundling/accumulation is a significant bottleneck. Using `trailing_zeros()` to iterate only over set bits reduced the `hvec_bundle_1000` time from ~17.7ms to ~4.0ms (~77% speedup) and `add_100` from ~6.79ms to ~1.07ms (~84% speedup).
**Action:** Always prefer popcount-based iteration (e.g., `trailing_zeros()` with `val &= val - 1`) over fixed-length bit-scanning for sparse or semi-dense bitsets.
