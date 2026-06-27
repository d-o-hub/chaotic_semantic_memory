# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.7] - 2026-06-27

### Fixed
- **Security**: Resolve SonarCloud path traversal warnings in `scripts/normalize_llms.py` and `scripts/yaml-to-drawio.py` by validating file paths stay within the project directory (pythonsecurity:S8707).
- **CI (Release workflow)**: Fix `cargo publish --dry-run` failure caused by workspace crates (`csm-core`, `csm-embedding`, etc.) not being published to crates.io. Use `--no-verify` for dry-run validation and publish workspace crates in dependency order before the main crate.

## [0.3.6] - 2026-05-20

### Added
- **MCP Server**: Implemented Model Context Protocol (MCP) server support with stdio transport, providing 12 tool and 3 resource handlers for LLM integration (ADR-0067).
- **DuckDB Companion Crate**: Added workspace member crate `chaotic_semantic_memory_duckdb` providing parquet export, analytical queries, and a `csm-analytics` tool (ADR-0079 to ADR-0082).
- **CLI**: Reached 100% feature-parity with the framework API across all 22 commands, locked with a new `cli_parity` smoke test suite (ADR-0066).
- **Events**: Pluggable memory event subscriptions in the framework using the CloudEvents specification.

### Changed
- **Hyperdim**: Refactored Bit-Sliced Hypervector Bundling with target-specific SIMD optimization blocks (AVX2/NEON) and a clean fallback path.

### Fixed
- **CLI / MCP**: Fixed stdout tracing corruption issues, refactored MCP subcommands to a modular structure under `src/cli/mcp.rs`, and resolved a compilation warning.

## [0.3.5] - 2026-04-28

### Fixed
- **Reservoir**: Prevent panic on unsanitized `chaos_strength` and enforce bounds (#126).
- **CI (Release workflow)**: `wait-for-ci` guardrail now grants `actions: read`
  (scoped to the job), surfaces `gh run list` errors, and tolerates empty/null
  responses. Resolves 600 s timeouts that failed every push to `main` since v0.3.4.
- **CI (GitHub Pages)**: Placeholder fallback heredoc replaced with a `printf`
  array; the previous indented heredoc broke both bash and YAML block-scalar
  parsing on `book/**` pushes.
- **Docs**: Corrected stale npm package URL in release-management skill
  (`@d-o-hub/chaotic_semantic_memory`, `@d-o-hub/csm`) per issue #106.

### Performance
- **Retrieval**: Optimize parallel task granularity in BM25 search (#127).

### Internal
- **GOAP state**: Synced 3 stale `queued` actions to `complete`
  (inertial reservoir tests + benches, selectivity-aware retrieval tests).
- **CHANGELOG**: Back-filled `[0.3.3] [YANKED]` entry documenting the yanked
  release (NEON intrinsic build break on macOS arm64).

## [0.3.3] - 2026-04-24 [YANKED]

> **Yanked**: this version was tagged from buggy code that broke the macOS arm64
> build (`E0432` from non-existent NEON intrinsics `veorq_u128`/`vld1q_u128`/
> `vst1q_u128`). Use `0.3.4` instead.

### Fixed
- See `0.3.4` for the corrected NEON intrinsic types.

## [0.3.4] - 2026-04-25

### Fixed

- **Path Hijacking Prevention**: Filter PATH to exclude relative entries (CWE-426)
  when spawning git subprocesses to prevent command injection attacks.
- **CLI Constants**: Use static constants for environment variable names to avoid
  spelling errors (DeepSource RUST-R005).

### Security
- **Input Validation**: Added bounds for reservoir dimensions and history limit to prevent OOM/DoS.
- **Panic Prevention**: Guarded against NaN/negative chaos strength in noise generation.

- **git_local.rs**: Sanitized PATH lookup for git command execution.
- **benchmarks/runner.rs**: Sanitized PATH lookup for commit SHA resolution.

## [0.3.2] - 2026-04-09

### Added

- **Benchmark Harness Metrics**: Added p99 latency percentile, NDCG@k scoring, and
  cross-session query types (Association, MultiSession) for comprehensive evaluation.
- **Configurable Abstain Threshold**: New `--abstain-threshold` CLI parameter for
  tuning retrieval abstention behavior.

### Changed

- **Percentile Indexing**: Floor-based indexing for p50/p95/p99 calculations
  (industry standard: `latencies[(count-1)/2]` for median).
- **NDCG@k Implementation**: Logarithmic discount `1/2^position` for 0-indexed
  ranking, with HashSet O(1) lookup for gold evidence IDs.
- **sysinfo API**: Updated to v0.33 API (`refresh_processes(ProcessesToUpdate::Some(&[pid]), false)`).
- **Storage Bytes Estimation**: Added storage size tracking from sessions.jsonl metadata.
- **Variable Session Generation**: `generate_sessions_with_range()` supports min/max turn counts.

### Fixed

- **Benchmark Tests**: All 19 tests passing with new metric fields (p99_latency_ms, ndcg_at_10).

## [0.3.1] - 2026-04-09

### Added

- **BM25 Parallel Scoring**: Added Rayon `par_iter()` for parallel document scoring
  with cfg gates for non-WASM builds.
- **BM25 Scalability Benchmarks**: New benchmarks for 100, 1000, 10000 document queries.
- **Singularity Scalability Benchmarks**: New benchmarks for 100, 1000, 10000, 50000 concepts.
- **HVec JSON Serialization Tests**: Verify base64 encoding and legacy array format support.

### Changed

- **BM25 Optimization**: Combined PR #64 optimizations with parallel scoring:
  - Pre-calculate constants (k1+1, c1, c2) outside loops
  - Use document indices instead of cloning String IDs during scoring
  - Use `sort_unstable_by` for faster descending score sorting
  - Only clone IDs for top_k results (deferred allocation)
- **Performance Improvements**:
  - `bm25_search_1000`: ~40% faster (3.2ms → 1.9ms)
  - `bm25_search_10000`: ~47% faster (3.9ms → 1.9ms)
  - `reservoir_step_50k`: ~57μs (target: <100μs) ✅
  - `singularity_probe_50000`: ~3.7ms (excellent scalability)

### Fixed

- **CLI Binary Shadowing**: Fixed outdated CLI binary (v0.1.0) shadowing newly installed
  v0.3.0 in PATH priority.
- **Export/Import Roundtrip**: Verified JSON base64 serialization works correctly for
  concept vectors.

## [0.3.0] - 2026-04-08

### Added

- **ADR-0062: Hybrid BM25+HDC Retrieval**: Implemented query-length-dependent
  hybrid scoring for improved short-query recall. BM25 keyword index runs
  parallel with HDC semantic search. Weights shift from 90% keyword for 1-2
  token queries to 80% semantic for 9+ tokens.
- **ADR-0063: Database Table Prefix**: All tables now use `csm_` prefix
  (`csm_concepts`, `csm_associations`, `csm_versions`, `csm_schema_version`,
  `csm_canonical`) for namespace isolation in shared SQLite databases.
- **Semantic Bridge Layer (Issue #52)**: Complete implementation of zero-drift
  semantic generalization overlay:
  - `src/semantic_bridge.rs`: Core types (CanonicalConcept, ScoreBreakdown, MemoryPacket)
  - `src/concept_graph.rs`: In-memory canonical concept graph with token-to-concept index
  - `src/bridge_retrieval.rs`: Pipeline: normalize → recall → expand → rerank
  - `src/bridge_persistence.rs`: Feature-gated persistence for bridge tables
- **BM25 Optimization**: `swap_remove` for document removal (O(N) → O(1)), hoisted
  IDF calculations out of scoring loop (~33% search speedup).

### Changed

- **Encoder tests**: Moved from inline `src/encoder.rs` to standalone
  `tests/encoder_tests.rs` for better organization.
- **ADR-0062 status**: Updated from "Proposed" to "Implemented".
- **Schema migration v5**: Added migration to rename existing tables to
  prefixed versions for backward compatibility.

### Fixed

- **git_local doc test**: Enabled previously ignored doc test after path resolution fix.
- **Cosine similarity SIMD**: Optimized by removing SIMD-to-GPR stall (#58).
- **Concept ID injection**: Fixed medium-severity input validation issue.
- **bucket_probe_width overflow**: Bound to prevent Denial of Service panic.
- **WASM size gate**: Fixed script to check library WASM instead of CLI binary.

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

[0.3.5]: https://github.com/d-o-hub/chaotic_semantic_memory/compare/v0.3.4...v0.3.5
[0.3.6]: https://github.com/d-o-hub/chaotic_semantic_memory/compare/v0.3.5...HEAD
[0.3.4]: https://github.com/d-o-hub/chaotic_semantic_memory/compare/v0.3.2...v0.3.4
[0.3.2]: https://github.com/d-o-hub/chaotic_semantic_memory/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/d-o-hub/chaotic_semantic_memory/releases/tag/v0.3.1
[0.3.0]: https://github.com/d-o-hub/chaotic_semantic_memory/releases/tag/v0.3.0
[0.2.9]: https://github.com/d-o-hub/chaotic_semantic_memory/releases/tag/v0.2.9
[0.2.8]: https://github.com/d-o-hub/chaotic_semantic_memory/releases/tag/v0.2.8
[0.2.7]: https://github.com/d-o-hub/chaotic_semantic_memory/releases/tag/v0.2.7
[0.2.6]: https://github.com/d-o-hub/chaotic_semantic_memory/releases/tag/v0.2.6
[0.2.5]: https://github.com/d-o-hub/chaotic_semantic_memory/releases/tag/v0.2.5
[0.2.4]: https://github.com/d-o-hub/chaotic_semantic_memory/releases/tag/v0.2.4
[0.2.3]: https://github.com/d-o-hub/chaotic_semantic_memory/releases/tag/v0.2.3
[0.2.2]: https://github.com/d-o-hub/chaotic_semantic_memory/releases/tag/v0.2.2
[0.2.1]: https://github.com/d-o-hub/chaotic_semantic_memory/releases/tag/v0.2.1
[0.2.0]: https://github.com/d-o-hub/chaotic_semantic_memory/releases/tag/v0.2.0
[0.1.3]: https://github.com/d-o-hub/chaotic_semantic_memory/releases/tag/v0.1.3
[0.1.2]: https://github.com/d-o-hub/chaotic_semantic_memory/releases/tag/v0.1.2
[0.1.1]: https://github.com/d-o-hub/chaotic_semantic_memory/releases/tag/v0.1.1
[0.1.0]: https://github.com/d-o-hub/chaotic_semantic_memory/releases/tag/v0.1.0
