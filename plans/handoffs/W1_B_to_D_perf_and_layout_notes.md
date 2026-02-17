# W1 B -> D: Performance and Layout Notes

Status: complete

Delivered notes:
- SIMD representation constraints (serialization/import):
  - `HVec10240` storage format remains canonical as `[u128; 80]` and serialized as 1280 LE bytes.
  - SIMD acceleration is implementation-local (`bind`/`cosine_similarity`) and does not alter on-disk or wire format.
  - Import/export and version-history payloads remain architecture-neutral.
- Batch API and pooling assumptions:
  - Framework batch APIs (`inject_concepts`, `associate_many`) depend on persistence batch transactions for amortized roundtrips.
  - Remote Turso path uses a bounded async slot model (`connection_pool_size`, default 10) to cap concurrency.
  - Local SQLite remains per-operation connection without pool overhead.
- Benchmark and compatibility caveats:
  - `reservoir_step_50k` gate passes (`<100us`) with partitioned updates per ADR-0009.
  - Turso roundtrip gate runs when secrets are present; local p50 gate remains mandatory in CI.
  - No schema-breaking layout changes introduced by SIMD or batching phases.
