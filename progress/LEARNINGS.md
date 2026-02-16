# Accumulated Knowledge

## 2026-02-16: Initial Learning Session

### What Worked
1. Creating modular structure with 500 LOC limit per file
2. Using libsql instead of non-existent turso-client
3. Organizing agent skills separately for better maintainability

### Technical Insights
- Using `[u128; 80]` for 10240-bit hypervectors is optimal for Rust SIMD
- Rayon provides excellent parallelization for similarity computations
- libsql supports both local SQLite and remote Turso with same API

### What to Avoid
- Don't try to use turso-client (doesn't exist)
- Don't exceed 500 LOC per file
- Don't use blocking I/O - always async/await

### Performance Targets
- Reservoir step: < 100μs at 50k nodes
- Turso roundtrip: < 20ms
- Memory: 10M concepts under 12MB (compressed)

## 2026-02-16: Iteration 2 Validation + Gap Closure

### What Worked
1. Treating stale GOAP state as a verification prompt and running full gates first.
2. Fixing persistence edge cases with explicit transactions and rollback.
3. Running criterion with `--save-baseline` before `--baseline` comparison.

### Technical Insights
- `PRAGMA wal_checkpoint(TRUNCATE)` in libsql should be handled via `query(...)` to consume returned rows.
- Concept deletion must remove `associations` (`from_id`/`to_id`) before deleting the concept to satisfy foreign keys.
- For criterion, run `cargo bench --bench benchmark -- --save-baseline <name>` once before `--baseline <name>`.

### What to Avoid
- Do not assume benchmark arg `--baseline` works via `cargo bench -- --baseline` when libtest benches are present.
- Do not return references from criterion closures that capture mutable benchmark state.

## 2026-02-16: Iteration 3 — GOAP Analysis + Architecture Decisions

### What Worked
1. Systematic codebase analysis before planning — found 16 real issues vs the GOAP state's 10.
2. Using oracle for deep code review across all modules simultaneously.
3. Writing ADRs for every non-trivial architectural change (sparse matrix, connection model, batch ops).
4. Deleting superseded ADRs (0003) rather than keeping stale docs.

### Technical Insights
- Dense `Array2<f32>` for 50k×50k reservoir is physically infeasible (~10 GB). CSR with fixed degree k=64 reduces to ~25 MB.
- `HVec10240::permute()` with `bit_shift == 0` causes `>> 128` which is undefined behavior for u128 — must guard with `if bit_shift == 0`.
- `Reservoir::to_hypervector()` with `size < 10240` causes `chunk_size = 0` from integer division — returns all-zero vectors silently.
- `Arc<RwLock<Connection>>` for libsql is unsafe under tokio multi-threaded runtime. Per-operation `db.connect()` is cheap and eliminates Send/Sync risks.
- `partial_cmp().unwrap()` panics on NaN — always use `f32::total_cmp()` for similarity sorting.
- `select_nth_unstable_by()` gives O(n) partial top-k vs O(n log n) full sort.

### What to Avoid
- Do not use dense matrices for reservoirs > ~2000 nodes.
- Do not share a single libsql `Connection` across async tasks via RwLock.
- Do not use `partial_cmp().unwrap()` on floats — NaN will panic.
- Do not assume `Vec<(String, f32)>` associations will deduplicate — use `HashMap<String, f32>`.

## 2026-02-16: Iteration 4 — Skills Overhaul

### What Worked
1. Replacing generic boilerplate skills with codebase-specific patterns made them actually useful.
2. Adding executable `scripts/` to skills — agent can run them directly instead of copy-pasting commands.
3. Creating domain-specific `debugging-reservoir` skill — ESN debugging requires specialized knowledge not covered by general Rust skills.
4. Using @-mentions in AGENTS.md to auto-inject key files into context.

### Technical Insights
- Skill `description` field is the only thing visible at startup — must contain enough trigger keywords for the agent to find the right skill.
- Old `references/quality-gates.md` duplicated `local-gates.md` in CI guardrails — single source of truth via scripts is better.
- `cargo bench -- --baseline` (without `--bench benchmark`) tries to run libtest benches too and fails silently. Always use `cargo bench --bench benchmark -- --baseline <name>`.
- Seeded RNG (`StdRng::seed_from_u64(42)`) in tests is essential — `Reservoir::new()` uses `thread_rng()` which makes tests non-deterministic.

### What to Avoid
- Do not duplicate gate commands across skills — put them in one script and reference it.
- Do not use generic module templates that don't reflect actual codebase patterns (WASM cfg gating, sparse weights, per-op connections).
- Do not keep stale `references/` directories when content has moved to `reference/` or `scripts/`.
