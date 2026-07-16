# Wave 32 P2 Progress — 2026-07-16

## Scope

Independent Wave 32 actions that do **not** require full workspace ownership consolidation
(ADR-0094 migration) or release-grade benchmark evidence tiers (ADR-0095 Phase 3).

| Field | Value |
|---|---|
| Branch | `feat/wave32-p2-ttl-absence-contracts` |
| PR | [#517](https://github.com/d-o-hub/chaotic_semantic_memory/pull/517) |
| Commit | `bc30e08` |
| GOAP action key | `wave32_p2_ttl_absence_contracts_2026_07_16` |
| Roadmap | `plans/GOAP_AUDIT_2026_07_14.md` |

## Completed (this PR)

| Action | ADR | Outcome |
|---|---|---|
| `own_ttl_cleanup_lifecycle` | 0093 | `TtlCleanupControl` owns cancel + `JoinHandle`; Drop aborts; `shutdown_ttl_cleanup` (2s); tests in `tests/ttl_lifecycle.rs` |
| `implement_or_remove_bm25_absence_short_circuit` | 0094 | Production TODO removed; hybrid CLI skips BM25 when `is_known_absent` (≥3 attempts); tests in `tests/bm25_absence_short_circuit.rs` + bridge_persistence unit assert |
| `enforce_workspace_feature_contracts` | 0094 | Owner crates `default-features = false`; persistence/parallel forward; csm-memory rayon optional; MSRV 1.88; `--no-default-features` has no libSQL |
| `replace_persistence_disabled_noops` | 0094 | `with_local_db`/`with_turso` cfg-absent without feature; disabled stubs return `UnsupportedOperation` |
| `align_wasm_ci_release_artifact` | 0094 | CI + release build `crates/csm-wasm` with `--out-name chaotic_semantic_memory`; nodejs JS smoke path added |
| `fuzz_short_and_scheduled_runs` | 0095 | Short smoke targets after fuzz-build; `.github/workflows/fuzz-scheduled.yml` weekly/dispatch |
| `harden_mutation_evidence` | 0095 | Score = killed only (timeouts unresolved); inventory in report; broad mcp/persistence path excludes removed |

### Local validation (pre-push)

- `cargo clippy -- -D warnings` — pass
- `cargo test --lib` — 189 pass
- `cargo test --test ttl_lifecycle --test bm25_absence_short_circuit` — pass
- `cargo check --no-default-features` — pass (no libSQL)
- `cargo test --test cli_parity --features cli` — pass
- LOC gate — all files ≤500

## CI status (PR #517 — live)

**Green (sampled):** workspace crate matrix, cargo-deny, benchmark workspace, detect-changes, tooling/version/changelog, Codacy, SonarCloud, several CodeQL language jobs.

**Red (blockers — must fix before merge):**

| Job | Priority | Likely cause / fix |
|---|---|---|
| commitlint | P0 | Subject/scope not in allowed set — amend or add `wave32`/adjust message to match commitlint enum |
| wasm | P0 | New wasm-pack out-name / nodejs smoke step — inspect job log; keep web build + smoke resilient |
| Fuzz Workspace Build | P0 | Short `cargo fuzz run` step after compile check — nightly/cargo-fuzz install or runtime; degrade to compile-only if short runs unstable on runners |

**Pending at last poll:** main `test`, `lint`, `miri`, `mcp-feature`, DuckDB, Analyze(rust).

## Follow-ups

### A. Immediate (PR #517 unblock)

1. **fix_pr517_commitlint** — make commit message / commitlint config agree.
2. **fix_pr517_wasm_job** — restore green wasm job (smoke optional behind flag if flaky).
3. **fix_pr517_fuzz_short_runs** — green fuzz job; prefer keep short runs, else fail-soft with compile-only required gate.

### B. Still queued in `plans/ACTIONS.md` (Wave 32 remainder)

Ordered by recommended next waves:

#### Wave 33a — Ownership façades (ADR-0094) — P1

| Action | Cost | Blocks |
|---|---:|---|
| `consolidate_retrieval_ownership` | 8 | persistence/CLI/WASM consolidation, test dedupe |
| `consolidate_persistence_cli_wasm_ownership` | 10 | `workspace_implementation_owners_unique` |

#### Wave 33b — Evidence tiers (ADR-0095) — P2

| Action | Cost | Notes |
|---|---:|---|
| `establish_tiered_benchmark_evidence` | 8 | PR / scheduled / release manifests |
| `add_ann_and_persistence_scale_benchmarks` | 8 | needs evidence tiers |
| `replace_formula_only_memory_claim` | 4 | measured RSS/model; not arithmetic-only |

#### Wave 33c — Agent / plan hygiene (ADR-0096) — P2/P3

| Action | Cost | Notes |
|---|---:|---|
| `run_critical_skill_behavioral_evals` | 5 | ≥19/20 on five critical skills; fail-closed exit codes |
| `canonicalize_hooks_skill_refs_and_catalog` | 5 | single catalog + one hook bootstrap |
| `reconcile_harness_engineering_state` | 2 | dated matrix on ADR-0090 |
| `compact_active_plans_non_destructively` | 3 | archive manifest; keep user-owned recommendations |

#### Post-ownership

| Action | Cost | Notes |
|---|---:|---|
| `deduplicate_test_and_source_surfaces` | 8 | after unique owners |

### C. Suggested (not yet ACTIONS names)

| ID | Priority | Scope |
|---|---|---|
| `absence_invalidation_policy` | P3 | Clear/refresh absence rows when matching content is injected |
| `branch_protection_fuzz_job` | P3 | Require **Fuzz Workspace Build** on main once short runs are stable |
| `wasm_release_smoke_parity` | P3 | Run the same nodejs smoke artifact path in release as CI |

## Explicit non-goals (unchanged)

- Full dual-source blind re-export without parity characterization
- Hardware-sensitive microbench gates on unpinned PR runners
- Claiming 10M memory / 1M ANN SLOs without measured artifacts
- Destructive plan archive of `plans/RECOMMENDATIONS_2026_07_14.md` without user approval

## File map (P2)

```
src/framework_ttl.rs          TtlCleanupControl + shutdown API
src/framework_builder.rs      owned cleanup spawn
src/framework.rs              ttl_cleanup field
src/cli/commands/query.rs     BM25 absence short-circuit
src/retrieval/bm25.rs         DEFAULT_ABSENCE_MIN_ATTEMPTS; TODO gone
src/lib.rs                    UnsupportedOperation stubs
Cargo.toml + crates/*/        feature contracts + MSRV 1.88
.github/workflows/ci.yml      wasm smoke + fuzz short runs
.github/workflows/release.yml csm-wasm canonical artifact
.github/workflows/fuzz-scheduled.yml
scripts/mutation_test.sh      timeouts unresolved + inventory
tests/ttl_lifecycle.rs
tests/bm25_absence_short_circuit.rs
plans/ACTIONS.md / GOAP_STATE.md
```

## Implementation extension (same PR, post-docs)

In addition to the P2 code already listed, this PR now also ships:

| Action | Status |
|---|---|
| CI remediations (wasm mut, fuzz lock, commitlint rewrite) | done |
| `run_critical_skill_behavioral_evals` | `scripts/eval-critical-skills.sh` 20/20 |
| `canonicalize_hooks_skill_refs_and_catalog` | catalog + pre-commit + CI |
| `reconcile_harness_engineering_state` | ADR-0090 matrix 2026-07-16 |
| `compact_active_plans_non_destructively` | archive manifest + history stubs |
| `establish_tiered_benchmark_evidence` | crates/** paths + evidence_manifest.json |
| `add_ann_and_persistence_scale_benchmarks` | `bench_ann_scale` exact/bucket 2k |
| `replace_formula_only_memory_claim` | measured RSS scale fit test |
| `consolidate_retrieval_ownership` (hybrid phase) | csm-retrieval owner façade |

Still queued: full persistence/CLI/WASM ownership + test surface dedupe.
