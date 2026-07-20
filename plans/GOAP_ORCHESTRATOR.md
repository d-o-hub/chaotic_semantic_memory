# GOAP Orchestrator — Open Issues Wave 2026-07-18

## Target state
- Issues #524, #525, #526 closed via single PR
- All tests pass; framework_ops.rs ≤ 500 LOC
- GOAP_STATE / PROGRESS updated

## World scan
| Issue | Title | Module | Cost |
|-------|-------|--------|------|
| #524 | redundant `namespace.read()` | `framework_ops` | 2 |
| #525 | parallelize `inject_concepts` build | `framework_ops` | 3 |
| #526 | shorten import association write lock | `framework_ops` | 4 |

All three touch `src/framework_ops.rs` only → **single feature branch / one PR**.

## Action plan (dependency order)

### A1: `fix_redundant_namespace_reads` (#524)
- **Effect**: one `namespace` clone (or single guard) per op; no dual `read().await`
- **Why clone not guard**: tokio `RwLockReadGuard` is not held across `.await` (Send)
- **Funcs**: `associate_many`, `update_concept_vector`, `update_concept_metadata`,
  `disassociate`, `clear_associations`, import paths

### A2: `parallel_inject_concepts_build` (#525)
- **Effect**: Rayon `par_iter` construction before `durable_inject_concepts`
- **cfg**: `parallel` + non-wasm; serial fallback otherwise
- **Note**: write lock remains inside `durable_inject_concepts` (post-build)

### A3: `shorten_import_write_locks` (#526)
- **Effect**: validate concepts outside lock; phase-1 inject write; phase-2 associate write
- **TOCTOU**: documented; invalid assoc still skipped with `warn!` (existing semantics)
- **DRY**: shared `apply_import_payload` helper for json/binary

### A4: tests + bench + state
- Unit tests in `framework_ops_tests.rs`
- Optional criterion group for inject batch sizes
- Update GOAP_STATE / PROGRESS / close issues via PR body `Fixes`

## Branch
`feat/framework-ops-perf-524-525-526`

## Swarm
| Agent | Task |
|-------|------|
| orchestrator | plan, integrate, PR |
| implement | framework_ops + tests (same file → single agent) |
| validate | cargo test/clippy/fmt |
