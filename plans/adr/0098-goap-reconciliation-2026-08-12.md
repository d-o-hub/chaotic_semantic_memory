# ADR-0098: GOAP Reconciliation 2026-08-12

## Status

Accepted

## Context

The 2026-08-12 PR roast wave landed eight PRs and trued the wave-33 goal
flags. `plans/ACTIONS.md` and `plans/GOAP_STATE.md` still reflected
pre-wave state: `workspace_implementation_owners_unique` was false although
the csm-cli/csm-wasm dead duplicates had been removed, and the two evidence
actions did not record that the bench harnesses had shipped.

## Decision

Record what landed on 2026-08-12 and reconcile the GOAP state files in place:

- #621: persistence-disabled builds reject configured DB/Turso; mutation
  tests run with `--no-default-features` for disabled-capability mutants
  (ADR-0094).
- #620: chaotic LSH projection loops unrolled (SIMD parity regression
  guarded by the new LSH bitwise-parity bench).
- #622: bhvec `from_bytes` deserialization optimization.
- #623: BM25 absence short-circuit wired into probe paths (M1); absence
  unit tests relocated to `src/retrieval/bm25/absence_short_circuit_tests.rs`
  to hold the 500-LOC gate; mutation excludes added for the
  integration-test-covered short-circuit and the test-scaffold stub.
- #624: wave-33 goal flags and README version/ANN truth synced.
- #626: dead duplicate CLI modules removed from `crates/csm-cli` (root owns
  CLI bodies; crate owns the bin facade) (ADR-0094).
- #627: dead duplicate WASM modules removed from `crates/csm-wasm` (root
  owns WASM bodies; crate owns the re-export facade) (ADR-0094).
- #628: bench harnesses shipped — LSH scalar-vs-SIMD bitwise parity bench,
  persistence CRUD p50/p95/p99 percentile bench, persisted-bytes metric
  (ADR-0095).

## Consequences

- `workspace_implementation_owners_unique` and
  `no_missing_implementations` are now true; `main_head`, ADR counts, and
  `queued_actions_count` updated in `plans/GOAP_STATE.md` (this file is the
  current-truth record; dated narrative stays in the archive snapshots).
- Remaining queued actions: `deduplicate_persistence_owner_bodies`
  (persistence body convergence between root and `csm-persistence`),
  `add_ann_and_persistence_scale_benchmarks` (full-scale artifacts still
  pending), `replace_formula_only_memory_claim` (full-scale memory model
  still pending), `deduplicate_test_and_source_surfaces`.
- `action_last_completed` set to `reconcile_pr_wave_2026_08_12` (exactly
  once, last key in `plans/GOAP_STATE.md`).

## References

- ADR-0094 (workspace ownership and feature contracts)
- ADR-0095 (evidence-driven quality gates)
- ADR-0097 (prior GOAP reconciliation and plans compaction)
- ADR-0089 / ADR-0092 (prior GOAP reconciliations)
