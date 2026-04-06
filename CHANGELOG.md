# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.9]

## [0.2.9] - 2026-04-06

### Added

- **ADR-0061: Semantic Bridge Layer**: Architecture design for zero-drift semantic
  generalization overlay on HDC memory system. Adds canonical concept graph, 
  bridge retrieval pipeline, and memory packet compiler. Status: Accepted.

### Changed

- **ADR_REGISTRY.md**: Updated ADR status tracking for ADR-0061 (Accepted) and
  ADR-0062 (Hybrid BM25-HDC Retrieval - Implemented).

## [0.2.8] - 2026-04-05

### Fixed

- **npm publish workflow**: Copy prepared package.json with scoped name
  `@d-o-hub/chaotic_semantic_memory` to wasm/pkg directory after wasm-pack build,
  fixing "403 Forbidden" error due to unscoped package name mismatch.

## [0.2.7] - 2026-04-05

### Fixed

- **npm publish workflow**: Corrected wasm-pack build path to run from repository root
  instead of `wasm/` subdirectory, fixing "pkg/: Not a directory" error.

## [0.2.6] - 2026-04-05

### Added

- **memory-context integration** (issue #43): Complete CLI tooling for AI coding assistants:
  - `index-jsonl`: Stream JSONL files with `--field` extraction, `--id-field`, `--tag-field`, and `--code-aware` encoding
  - `index-dir`: Markdown directory ingest with glob patterns (`--glob`), heading-based chunking (`--heading-level`), and code-aware encoding
  - `query`: Text-based similarity search with `--top-k`, `--min-score`, `--output json`, and `--compact` flags
  - Git-local default storage: `.git/memory-index/csm.db` for "never committed, per-clone" memory indexes
  - Code-aware tokenization: Splits identifiers on `_`, `-`, `.`, `/`, `::` with character trigram overlay

- **Documentation**: Added Turso vector alternative section to README, encoder docs,
  and introduction. Users can now use Turso's native `F32_BLOB` and `vector_top_k()`
  for semantic similarity while keeping HDC for lexical matching.

## [0.2.5] - 2026-03-22

### Changed
- **README**: Clarified that text encoding uses Hyperdimensional Computing (HDC), not
  transformer embeddings. Added "How Text Encoding Works" section explaining the
  FNV-1a → PRNG → positional permutation → bundling pipeline.
- **README**: Added "Concurrency Model" section documenting `tokio::sync::RwLock`,
  SQLite WAL mode, multi-instance safety, and `block_on` warning.
- **README**: Added library-only installation note (`default-features = false`).
- **lib.rs**: Expanded module-level documentation with encoding model, concurrency
  guarantees, and WASM parity notes.

### Fixed
- Addressed documentation gaps identified in [#35](https://github.com/d-o-hub/chaotic_semantic_memory/issues/35):
  HDC vs embeddings clarification, concurrency model, and feature-gating guidance.

## [0.2.4] - 2026-03-19

### Added
- **Reduced-Candidate Retrieval** (`singularity.rs`): Introduced a two-stage retrieval pipeline.
  Supports vector-bucket and graph-neighborhood candidate generation before exact reranking.
- **Retrieval Observability**: `RetrievalStats` struct and `last_retrieval_stats()` method
  provide visibility into candidate counts and stage-specific latencies.
- **Public re-exports**: `BundleAccumulator`, `RetrievalStats`, `RetrievalConfig`, and
  `CandidateSource` are now exported from crate root and prelude.
- **Concurrent Persistence Benchmarks**: Added `shared_store_concurrent_10_saves` to
  `persistence_benchmark.rs` to measure contention under shared-store conditions.
- **ADR-0059** documenting retrieval optimizations and benchmark hygiene.

### Changed
- **Optimized Exact Retrieval** (`singularity.rs`): Refactored `Singularity` to use dense storage
  for concept vectors and indices, resulting in ~2.6x speedup for exact similarity scans.
- **Benchmark Methodology**: Persistence benchmarks now distinguish between `cold` (with setup)
  and `warm` (steady-state) operations.

### Fixed
- **Release workflow** (`release.yml`): Fixed crates.io version check using wrong variable
  (`steps.version` instead of `needs.validate.outputs.version`), which could cause publish
  failures or duplicate publish attempts.

## [0.2.3] - 2026-03-16

### Fixed
- **Import/Export Serialization** (`hyperdim.rs`, `export_payload.rs`, `framework_ops.rs`):
  - HVec10240 JSON serialization now uses base64 encoding for human-readable formats
  - Binary export uses bincode-compatible BinaryExportPayload with BinaryMetadataValue
  - Fixed "invalid type: sequence, expected byte array" error for JSON import
  - Fixed "Bincode does not support deserialize_any" error for binary import

### Added
- **Turso Memory Verification Skill** (`.agents/skills/turso-memory-verification/`):
  - New skill for verifying memory persistence before releases
  - Automated test script (`scripts/verify-memory-roundtrip.sh`) for JSON/binary roundtrip testing
- **ADR-0058** documenting the serialization fixes and migration path

## [0.2.2] - 2026-03-16

### Changed
- Version sync for 0.2.2 across docs, examples, and WASM package metadata.
- Dependency refresh via Cargo.lock update (clap 4.6.0, tracing-subscriber 0.3.23,
  tempfile 3.27.0, plus transitive updates).

### Fixed
- Release validation artifacts regenerated (`llms.txt`, `llms-full.txt`).

## [0.2.1] - 2026-03-09

### Fixed
- **Probe cache-miss allocation path** (`singularity.rs`): `find_similar_cached` no longer
  materializes an intermediate `Vec<(String, HVec10240)>` before similarity scoring.
- **Local SQLite WAL policy** (`persistence.rs`): local connections now enable
  `PRAGMA journal_mode=WAL` while preserving per-connection `PRAGMA foreign_keys=ON`.
- **WASM metadata fidelity** (`wasm.rs`): metadata values are now converted with
  `js_sys::JSON::parse`, preserving non-string JSON types in browser bindings.

### Added
- **Scale probe benchmark coverage** (`benches/benchmark.rs`): exact-search benchmark group at
  10k, 100k, and 200k concept scales.
- **WAL validation tests** (`tests/persistence_crud.rs`, `tests/performance_targets.rs`):
  assertions for `journal_mode=WAL` and checkpoint compatibility.
- **mdBook chapters** (`book/src/encoder.md`, `book/src/graph.md`): text encoding and graph
  traversal guides linked from `book/src/SUMMARY.md`.

## [0.2.0] - 2026-03-04

### Added
- **FNV-1a hash stability** (`encoder.rs`): `TextEncoder` now uses FNV-1a (64-bit) instead of
  `DefaultHasher` (SipHash), guaranteeing deterministic encoding across Rust versions and platforms.
  `DefaultHasher` is explicitly non-stable; FNV-1a is a breaking change for persisted encoded vectors
  but is required for correctness (ADR-0055).
- **Weighted Dijkstra `shortest_path`** (`graph_traversal.rs`): `Singularity::shortest_path` now
  implements true weighted Dijkstra with `-ln(strength)` edge cost, preferring paths through stronger
  associations. The previous BFS behavior is preserved as `shortest_path_hops` for backward compat.
- **`BundleAccumulator::try_remove`** (`bundle.rs`): Fallible variant that returns
  `Err(MemoryError::InvalidInput)` on empty accumulator instead of panicking.
- **`BundleAccumulator::remove` no-op on empty** (`bundle.rs`): The infallible `remove` now saturates
  at zero instead of panicking, matching the documented "sliding-window" use case.
- **WASM parity** (`wasm.rs`): New WASM bindings for `update_concept_metadata`, `clear_associations`,
  `neighbors`, `bfs`, and `shortest_path` — completing the graph/metadata API surface in the browser.
- **Wave 16 test suite** (`tests/wave16_features.rs`): 21 new tests covering TextEncoder golden
  vectors, graph traversal cycle/disconnect edge cases, BundleAccumulator edge cases, and filtered
  search edge cases.
- **Wave 16 benchmarks** (`benches/benchmark.rs`): Criterion benchmarks for `TextEncoder::encode`
  (short/medium/long), `find_similar_filtered` (100/1k concepts), graph traversal BFS/Dijkstra
  (sparse/dense), and `BundleAccumulator` add/remove/finalize cycles.

### Fixed
- **Panic elimination** (`reservoir.rs`): `Reservoir::to_hypervector` no longer panics on internal
  `try_into` failure; maps to `MemoryError::Reservoir` instead.
- **Doc/code mismatch** (`graph_traversal.rs`): `shortest_path` docs claimed weighted Dijkstra but
  implemented unweighted BFS. Now correctly implements Dijkstra; BFS variant renamed `shortest_path_hops`.
- **Hash instability** (`encoder.rs`): `TextEncoder::stable_hash` used `DefaultHasher` (SipHash),
  which is non-stable across Rust versions. Replaced with inline FNV-1a implementation.

### Changed
- `TextEncoder` encoding output will differ from v0.1.x for any text input due to the FNV-1a hash
  change. Re-encode and re-persist any stored text-derived vectors after upgrading.
- `Singularity::shortest_path` now returns the minimum-cost (Dijkstra) path, not the fewest-hop
  (BFS) path. Use `shortest_path_hops` for the previous behavior.

## [0.1.3] - 2026-02-28

### Added
- ADR-0051: Real-World Readiness & Quality Hardening
- Real-world examples: chatbot memory, document RAG, knowledge graph, streaming temporal
- Edge case tests: builder config propagation, import adversarial payloads, eviction cache

### Fixed
- max_cached_top_k propagation: FrameworkBuilder now correctly forwards config to Singularity
- Default max_cached_top_k aligned to 100 (matching SingularityConfig)
- NotFound error variant: Cleaner error handling for missing concepts
- JSON import size limit: Added 100MB cap to prevent OOM

## [0.1.2] - 2026-02-28

### Fixed
- npm package name: corrected `@d-o-hub/chaotic-semantic-memory` → `@d-o-hub/chaotic_semantic_memory` (underscore)
- npm publishing: v0.1.2 now published with OIDC provenance via GitHub Actions
- Updated workflow to use Node.js 24 with npm fallback for token authentication

### Changed
- npm workflow now uses trusted publishing (OIDC) when configured

## [0.1.1] - 2026-02-27

### Added
- ADR-0048: wasm-pack bulk memory fix configuration
- `#[instrument(err)]` on framework and framework_ops fallible functions
- Tracing instrumentation for reservoir and persistence operations

### Fixed
- wasm-pack build failing due to missing `--enable-bulk-memory` wasm-opt flag
- npm publishing workflow (ADR-0046)
- Various dependency updates

### Changed
- Optimized find_similar and to_hypervector with parallelization

## [0.1.0] - 2026-02-17

### Added
- Initial release of `chaotic_semantic_memory`
- Hyperdimensional vector operations with SIMD support
- Echo state network reservoir for temporal processing
- Semantic memory framework with concept storage
- Turso/libSQL persistence layer
- WASM target support
- Property-based testing with proptest
- Fuzzing targets for critical paths
- Comprehensive benchmark suite
- Performance gates (< 100μs reservoir step)
- Memory footprint validation (< 12MB for 10M concepts)
- WASM binary size gate (< 500KiB)
- Schema migration support
- Export/import functionality (JSON + binary)
- Concept versioning
- Backup/restore operations
- Structured logging with tracing
- Metrics collection
- Connection pooling for Turso
- LRU concept cache
- Zero-allocation query cache

### Changed
- CLI crate (`csm` binary) with inject, probe, associate, export, import, and completions commands
- Shell completion generation for bash, zsh, fish, and powershell
- `ConceptBuilder` module for ergonomic concept construction
- `FrameworkBuilder` and `FrameworkConfig` as dedicated types
- Release management skill (`.agents/skills/release-management/`)
- GitHub Pages documentation workflow with mdBook
- crates.io Trusted Publishing workflow (OIDC-based)
- npm publishing workflow with provenance for WASM bindings
- **Published v0.1.0 to npm** (@d-o-hub/chaotic-semantic_memory)
- Upgraded to Rust Edition 2024 (MSRV 1.85)
- Added `#[must_use]` annotations to constructors
- Improved unsafe block documentation in SIMD operations
- Replaced format! JSON with serde_json in CLI
- Updated MSRV from 1.82 to 1.85

### Fixed
- CLI exit codes now correctly map to error types (ADR-0032)
- CLI JSON output escaping safety (ADR-0032)
- Async lock safety across await points (ADR-0031)
- WASM Reflect::set panic safety (ADR-0033)
- Query cache memory blow-up prevention (ADR-0035)
- LOC gate now includes src/cli/ and src/bin/ (ADR-0036)

### Security
- Added `SECURITY.md` with vulnerability reporting process
- Added `CODE_OF_CONDUCT.md` (Contributor Covenant 2.1)
- Updated CI workflow with security permissions and concurrency controls
- Trusted Publishing eliminates need for long-lived API tokens

[unreleased]: https://github.com/d-o-hub/chaotic_semantic_memory/compare/v0.2.8...HEAD
[0.2.8]: https://github.com/d-o-hub/chaotic_semantic_memory/releases/tag/v0.2.8
[0.2.7]: https://github.com/d-o-hub/chaotic_semantic_memory/releases/tag/v0.2.7
[0.2.6]: https://github.com/d-o-hub/chaotic_semantic_memory/releases/tag/v0.2.6
[0.2.5]: https://github.com/d-o-hub/chaotic_semantic_memory/releases/tag/v0.2.5
[0.2.4]: https://github.com/d-o-hub/chaotic_semantic_memory/releases/tag/v0.2.4
[0.2.3]: https://github.com/d-o-hub/chaotic_semantic_memory/releases/tag/v0.2.3
[0.2.2]: https://github.com/d-o-hub/chaotic_semantic_memory/releases/tag/v0.2.2
[0.2.1]: https://github.com/d-o-hub/chaotic_semantic_memory/releases/tag/v0.2.1
[0.2.0]: https://github.com/d-o-hub/chaotic_semantic_memory/releases/tag/v0.2.0
[0.1.1]: https://github.com/d-o-hub/chaotic_semantic_memory/releases/tag/v0.1.1
[0.1.0]: https://github.com/d-o-hub/chaotic_semantic_memory/releases/tag/v0.1.0
