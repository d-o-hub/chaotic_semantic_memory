
## 2026-04-11: Persistence Roundtrip Regression Guardrails

### Technical Insights
- Batch and single-row persistence paths must write identical concept fields; schema drift between `save_concept()` and `save_concepts()` can silently drop TTL/canonical linkage data.
- Binary export adapters are part of the persistence contract, not just transport glue; omitting fields in `BinaryConcept` causes lossy cross-format roundtrips.
- Persisted concept schema evolution should be handled via explicit migrations and validated by end-to-end tests that exercise JSON import -> binary export -> binary import.
- Path validation should distinguish "existing path required" operations from "new output file allowed" operations to avoid blocking valid exports.

### What to Avoid
- Do not add new concept fields without updating all persistence surfaces (single save, batch save, load-one, load-all, binary export/import, WASM serialization).
- Do not rely on unit tests that only assert `expires_at: None`; add positive tests with non-default TTL and canonical IDs.
- Do not assume CI's broad test pass catches field-loss regressions unless targeted regression tests are present.

## 2026-04-11: PR Triage & Issue Planning

### What Worked
1. Closing duplicate PRs (#65) that attempted unnecessary structural changes (converting flat files to directory modules)
2. Merging clean, minimal PRs (#66) - branchless bitmask optimization: ~40% finalize, ~10% bundle speedup
3. Data-driven PR evaluation: both PRs touched same files with same optimization, chose the minimal one

### Technical Insights
- Branchless bitmask construction (`(condition as u128) << j`) eliminates branch misprediction in tight 10240-iteration loops
- Converting a flat `.rs` file to a directory module (`src/hyperdim/`) is a structural change that affects imports; avoid unless LOC limit requires it
- hyperdim.rs is at 499 LOC — exactly at the limit, test extraction may be needed soon

### What to Avoid
- Do not accept PRs that change module structure (file → directory) unless LOC limit is exceeded
- Do not merge duplicate PRs — close the older/more invasive one
- When two PRs implement the same optimization, prefer the smaller diff

## 2026-02-16: Foundation & Architecture (Iterations 1–4)

### What Worked
1. Creating modular structure with 500 LOC limit per file; using libsql (not turso-client)
2. Treating stale GOAP state as a verification prompt and running full gates first
3. Systematic codebase analysis before planning — found 16 real issues vs the GOAP state's 10
4. Writing ADRs for every non-trivial architectural change (sparse matrix, connection model, batch ops)
5. Replacing generic boilerplate skills with codebase-specific patterns; adding executable `scripts/` to skills
6. Using @-mentions in AGENTS.md to auto-inject key files into context

### Technical Insights
- `[u128; 80]` for 10240-bit hypervectors is optimal for Rust SIMD
- libsql supports both local SQLite and remote Turso with same API
- `PRAGMA wal_checkpoint(TRUNCATE)` in libsql should be handled via `query(...)` to consume returned rows
- Concept deletion must remove `associations` (`from_id`/`to_id`) before deleting the concept to satisfy foreign keys
- Dense `Array2<f32>` for 50k×50k reservoir is physically infeasible (~10 GB). CSR with fixed degree k=64 reduces to ~25 MB
- `HVec10240::permute()` with `bit_shift == 0` causes `>> 128` (undefined behavior for u128)
- `Reservoir::to_hypervector()` with `size < 10240` causes `chunk_size = 0` — returns all-zero vectors silently
- `Arc<RwLock<Connection>>` for libsql is unsafe under tokio multi-threaded runtime. Per-operation `db.connect()` is cheap
- `partial_cmp().unwrap()` panics on NaN — always use `f32::total_cmp()` for similarity sorting
- `select_nth_unstable_by()` gives O(n) partial top-k vs O(n log n) full sort
- Skill `description` field is the only thing visible at startup — must contain enough trigger keywords
- Seeded RNG (`StdRng::seed_from_u64(42)`) in tests is essential — `Reservoir::new()` uses `thread_rng()`
- For criterion, always use `cargo bench --bench benchmark -- --baseline <name>` (not bare `cargo bench`)

### What to Avoid
- Don't try to use turso-client (doesn't exist); don't exceed 500 LOC per file; don't use blocking I/O
- Do not assume benchmark arg `--baseline` works via `cargo bench -- --baseline` when libtest benches are present
- Do not return references from criterion closures that capture mutable benchmark state
- Do not use dense matrices for reservoirs > ~2000 nodes
- Do not share a single libsql `Connection` across async tasks via RwLock
- Do not use `partial_cmp().unwrap()` on floats — NaN will panic
- Do not assume `Vec<(String, f32)>` associations will deduplicate — use `HashMap<String, f32>`
- Do not duplicate gate commands across skills — put them in one script and reference it
- Do not use generic module templates that don't reflect actual codebase patterns
- Do not keep stale `references/` directories when content has moved

### Performance Targets
- Reservoir step: < 100μs at 50k nodes
- Turso roundtrip: < 20ms
- Memory: 10M concepts under 12MB (compressed)

## 2026-02-16: Performance & Optimization (Iterations 5–7)

### What Worked
1. Treating GOAP_STATE booleans as executable acceptance criteria; closing them directly
2. Running wasm target checks in both default and feature-enabled modes to catch dependency wiring gaps
3. Flattening sparse rows into CSR-like contiguous arrays reduced pointer chasing
4. Local-neighborhood reservoir connectivity significantly reduced random state-memory access cost
5. Caching input projection for unchanged inputs eliminated repeated `W_in * input` work
6. Partitioned updates (rotating node subsets) crossed the `<100us` gate

### Technical Insights
- WASM crates must be target dependencies, not optional globals without active feature linkage
- `#[wasm_bindgen]` async exports require `wasm-bindgen-futures` on `wasm32`
- Benchmark variance can flip baseline comparison direction; use persisted median, keep target strict
- `Vec<Vec<(usize, f32)>>` incurs substantial allocator and cache overhead at 50k rows
- For large sparse reservoirs, memory locality and update policy dominate runtime more than arithmetic throughput
- A rotating partial-update schedule preserves state shape/API while dramatically lowering step latency

### What to Avoid
- Do not assume `wasm-bindgen` and `js-sys` optional deps are available just because code is cfg-gated by target
- Do not mark GOAP validation complete without rerunning both native gates and target-specific wasm checks
- Do not interpret benchmark p-values near threshold as target success; use absolute median
- Do not relax spectral-radius guardrails to chase speed; keep radius constraints explicit
- Do not treat architecture-changing performance fixes as implementation details; capture tradeoffs in ADRs
- Do not assume full synchronous ESN update semantics when partitioned updates are enabled

## 2026-02-16: Production Readiness (Iterations 8–10)

### What Worked
1. Migrating to `libsql::Builder` removed deprecated API usage without changing external behavior
2. Enabling `PRAGMA foreign_keys = ON` per connection made FK behavior deterministic
3. Capturing builder-time serialization errors and returning them at `build()` closed silent data-loss paths
4. Using `goap-planning` skill as orchestrator to systematically identify improvement opportunities
5. Creating specialized swarm skills for parallel execution by domain expertise
6. Using `SWARM_COORDINATION.md` as the single source of truth for swarm state

### Technical Insights
- FK enforcement must be applied per connection; schema-level FK declarations are not sufficient alone
- `ConceptBuilder` can preserve fluent API while surfacing metadata errors by storing first error and failing in `build()`
- Running `clippy` with `--all-targets --all-features` catches bench/test target lints
- SIMD benefits from scalar fallback for non-SIMD targets
- Connection pooling is only beneficial for remote Turso, not local SQLite
- Versioning strategy should use snapshots with bounded retention (not full event sourcing)
- Swarm groups work best on orthogonal concerns; ADR gate prevents architectural conflicts
- Phase boundaries provide natural integration points for cross-group validation

### What to Avoid
- Do not suppress deprecated `libsql` constructors with `#[allow(deprecated)]`; migrate to `Builder`
- Do not swallow serialization failures in builder APIs; this hides invalid input
- Do not assume FK constraints are active unless explicitly enabled on each SQLite connection path
- Do not implement SIMD without scalar fallback for non-SIMD targets
- Do not pool connections for local SQLite (no benefit, adds overhead)
- Do not make versioning mandatory (should be opt-in via config)
- Do not add heavy dependencies (Arrow/Parquet) for simple export/import
- Do not let swarm groups modify the same file simultaneously (coordinate via GOAP_STATE)
- Do not skip ADR review for cross-cutting changes (even within a group)
- Do not merge swarm work without running full validation gates
- Do not create circular dependencies between swarm groups

## 2026-02-27: Release Pipeline

### Technical Insights
- crates.io requires `publish-update` scope for existing crates (not just `publish-new`)
- npm provenance requires npm >= 11.5.1 and `--provenance` flag
- Action artifact version mismatch (v6 vs v7) causes workflow failures
- Node.js 22 ships npm v10; OIDC requires npm v11.5.1+ (Node.js 24)
- The error "Access token expired" is misleading — actually means registry doesn't recognize publisher
- Package name uses **underscore**: `@d-o-hub/chaotic_semantic_memory` (NOT hyphen)
- Single git tag triggers all publishes: crates.io (OIDC), GitHub Release, npm (OIDC provenance)
- npm OIDC > NPM_TOKEN: OIDC tokens auto-rotate, no expiration issues

### What to Avoid
- Do not use `publish-new` scope alone for existing crates
- Do not mix artifact upload/download action versions
- Do not publish without CHANGELOG update
- Do not use Node.js 22 for npm OIDC publishing; require Node.js 24
- Do not confuse underscore vs hyphen in npm package names
- Do not manually publish — let CI do it; do not remove token fallback

## 2026-03-16: Release Prep & CI Monitoring

### Technical Insights
- `sync-version.sh` updates Cargo.lock and bumps dependencies; full validation must be rerun afterward
- `scripts/validate.sh` regenerates `llms.txt` and `llms-full.txt`; include them in release commits
- Release workflows run on detached HEAD for tags; avoid push steps
- npm publish returns hard error when version exists; treat as no-op in CI
- `softprops/action-gh-release` can create a release off main when `tag_name` is provided
- `npm view <pkg> versions --json` is the most reliable way to detect already-published versions

### What to Avoid
- Do not treat stale CI failures as current when new runs are already in progress
- Do not skip changelog link updates when cutting a new patch release
- Do not ignore Node.js 20 deprecation warnings ahead of runner defaults switch
- Do not run release steps on every main push; gate on tag existence
- Do not rely on `npm publish` errors for control flow; preflight registry checks instead

## 2026-03-22: Documentation Audit

### Problem
21 discrepancies across 9 .md files: fictional APIs, wrong defaults, fabricated CLI flags.

### Documentation Accuracy Checklist
1. **Verify method signatures**: `grep` for the actual function, check parameters and return type
2. **Verify enum variants**: Check error.rs, args.rs for actual variant names
3. **Verify constants**: Check for `const DEFAULT_*` values in source
4. **Verify prelude exports**: Check `src/lib.rs` prelude module
5. **Verify CLI flags**: Check `src/cli/args.rs` for actual clap attributes
6. **Verify WASM exports**: Check `src/wasm.rs` for `#[wasm_bindgen]` methods
7. **Test code examples**: All Rust examples should be syntactically correct

### What to Avoid
- Do not write documentation from memory — always verify against source
- Do not assume method names match what "seems logical" — check actual code
- Do not copy-paste examples between files without re-verifying
- Do not document environment variables that don't exist in code
- Do not fabricate CLI flags or exit codes without checking args.rs

## 2026-04-06: Release Workflow & CHANGELOG

### Technical Insights
- CHANGELOG header format must be `## [VERSION] - YYYY-MM-DD` (single header, no duplicates)
- Duplicate `## [version]` headers break awk extraction in release workflow → fallback to empty body
- OIDC requires both GitHub-side (`id-token: write`) AND npm-side Trusted Publisher configuration
- Workflow creates tags from Cargo.toml version automatically — do not create tags manually
- Idempotent release workflow: checks crates.io, GitHub releases, and npm before publishing

### Release Checklist
1. Update `Cargo.toml` version
2. Run `./scripts/sync-version.sh <version>`
3. Add single CHANGELOG entry with proper format
4. Commit and push to main — workflow creates tag and publishes automatically

### What to Avoid
- Do not have duplicate `## [version]` headers in CHANGELOG.md
- Do not assume OIDC works without npm UI configuration
- Do not create git tags manually — workflow creates them
- Do not skip Version Integrity CI failures

## 2026-04-08: Benchmark Suite Bugs

### Text Storage Bug
- `inject_text()` only stores HDC vector, NOT text metadata
- Use `inject_text_with_metadata()` with `("_text", text)` for content-aware storage
- `get_concept()` returns metadata as `HashMap<String, serde_json::Value>`

### WASM Size Gate Bug
- `find | head -n 1` picked `csm.wasm` (5KB CLI binary) instead of `chaotic_semantic_memory.wasm` (870KB library)
- Filesystem order is not deterministic — always use explicit filenames or exclusion filters

### What to Avoid
- Do not use `inject_text()` when you need to retrieve original content later
- Do not assume concept ID equals stored text content
- Do not use `find | head -n 1` when specific file matters
- Do not assume filesystem order is consistent

## 2026-04-09: Benchmark Optimization

### Technical Insights
- `latencies[count / 2]` is biased high for even-length arrays; use `latencies[(count - 1) / 2]`
- p95/p99 should use floor (`as usize` truncation), not `.round()`
- NDCG@k: use logarithmic discount `1 / 2^position`, DCG/IDCG ratio
- HashSet for gold evidence lookups: O(1) vs O(n) nested iteration
- `sysinfo` v0.33: `refresh_process(pid)` replaced with `refresh_processes(ProcessesToUpdate::Some(&[pid]), false)`
- Sequential ingest adequate up to 500 sessions (~2.5ms/session avg); parallel not justified at current scale

### What to Avoid
- Do not use `count / 2` for percentile indexing (biased)
- Do not use `.round()` for percentile index calculation (can overshoot)
- Do not optimize ingest without measuring first (parallel may not be needed)
- Do not use `sysinfo::refresh_all()` when only checking one process
