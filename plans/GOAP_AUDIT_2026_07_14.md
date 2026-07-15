# GOAP Codebase Audit and Wave 32 Roadmap — 2026-07-14

## Scope and constraints

This is a planning-only audit. It covers implementation correctness, architecture, missing capabilities, tests, fuzzing, benchmarks, CI, developer workflow, and `.agents/skills/`. No source, test, workflow, script, or skill file is changed by this plan.

The pre-existing working-tree changes `export.json`, `opencode.json`, and `plans/RECOMMENDATIONS_2026_07_14.md` are user-owned and intentionally excluded. The recommendations file was reviewed as an input but is not rewritten; its stale and still-valid findings are triaged below.

## Verified baseline

| Check | Evidence | Result |
|---|---|---|
| Branch baseline | Started on `main`; planning edits moved to `plan/codebase-audit-2026-07-14` | Protected-main rule observed |
| Main CI | GitHub Actions run `29327781685` | Passed |
| Open PRs | `gh pr list --state open` | None |
| ADR parity | `./scripts/check-adr-parity.sh` | `registry=88`, `disk=87`; ADR-0003 is intentionally N/A |
| Rust LOC | `find src crates -name '*.rs' ...` | All files `<=500`; highest is 499 |
| Skill inventory | `find .agents/skills -name SKILL.md` | 32 skills; one 294-line violation |
| Test inventory | literal `#[test]` / `#[tokio::test]` scan over `src crates tests` | 1,034 attributes; this is not equivalent to unique behavior coverage |
| Explicit production markers | Rust source scan | One TODO: `src/retrieval/bm25.rs:109`; no `todo!` or `unimplemented!` |

## Executive result

The repository has a green baseline but is not ready for an unqualified “all gates enforced” or “no missing implementations” claim. Four concerns dominate the next path:

1. **P0 correctness:** persisted ANN snapshots can replace a freshly reconstructed current index; invalid ANN configuration can panic; framework mutation errors do not imply unchanged state.
2. **P0 validation:** the fuzz workspace does not compile, and skill-local validators can report success after failed Cargo commands.
3. **P1 ownership/contracts:** root and extracted crates duplicate and diverge, feature disabling does not remove persistence/Rayon, MCP overstates its vector wire format, and CI/release build different WASM artifacts.
4. **P1/P2 evidence:** CI does not execute several recorded gates, benchmark metric definitions are wrong or weak, and memory/scale claims are not based on measured implementation state.

The roadmap therefore prioritizes restoring truth and fail-closed behavior before adding optional capabilities or micro-optimizations.

## Evidence-backed findings

### P0 — correctness and fail-closed validation

| ID | Finding | Evidence | Required outcome |
|---|---|---|---|
| C1 | A stale ANN snapshot can overwrite an index rebuilt from newer persisted concepts. | `src/framework_persistence.rs` injects loaded concepts, then deserializes any `main` snapshot; `src/persistence_index.rs` stores `modified_at` but `load_index` returns only bytes; normal inject/delete does not persist or invalidate the snapshot. | Persist and validate a namespace revision plus backend/config fingerprint; reject/rebuild stale or incompatible snapshots. |
| C2 | Invalid public ANN configuration can panic. | `FrameworkBuilder::with_index_backend` stores unvalidated config; `crates/csm-memory/src/singularity.rs::create_index` calls `expect`. | Validate during `build()` and make index construction fallible end-to-end. |
| C3 | Persistence errors can leave process memory changed while durable state is unchanged. | `inject_concept` and `delete_concept` mutate singularity before persistence `.await` in `src/framework.rs`. | Adopt explicit persistence-authoritative mutation semantics with deterministic recovery. |
| C4 | Framework state locks are held across persistence awaits. | `persist`, `load_replace`, and `load_merge` retain namespace or singularity guards during I/O in `src/framework_persistence.rs`, contrary to ADR-0040 and README claims. | Snapshot owned data under state locks; perform I/O after releasing them; use a separate mutation coordinator if serialization is needed. |
| Q1 | The fuzz workspace is broken. | `cargo check --manifest-path fuzz/Cargo.toml --all-targets --offline` fails: wrong metadata type, missing `canonical_concept_ids`, missing namespace argument in `persistence_save_concept.rs`. | Build all fuzz targets in CI and run changed/scheduled targets against product decoders. |
| W1 | Validation scripts can report false success. | Skill-local validators pipe Cargo commands through `grep`/`tail` and infer success from text; direct exit codes are not preserved. | One canonical fail-closed validator with negative fixture tests. |

### P1 — architecture, API, and workflow contracts

| ID | Finding | Evidence | Required outcome |
|---|---|---|---|
| A1 | `--no-default-features` still resolves persistence, libSQL, and Rayon. | `cargo tree -p chaotic_semantic_memory --no-default-features -e features` shows `csm-persistence feature "default"`, `libsql`, and `rayon`; root and extracted manifests do not forward feature disabling consistently. | `no-default-features` excludes optional persistence/parallel dependencies; explicit features forward to owner crates. |
| A2 | Persistence-disabled APIs silently succeed or ignore configuration. | `with_local_db`/`with_turso` no-op stubs in `src/framework_builder.rs`; fallback persistence methods return empty/`Ok` results. | Compile-time absence or explicit `UnsupportedOperation`; never false success. |
| A3 | Implementation ownership is duplicated and diverged. | All five root retrieval modules differ from `csm-retrieval`; most CLI files are byte-identical duplicates; WASM core is duplicated and `wasm_ext` differs; persistence types also diverge. | One owning crate per concern; root is façade/orchestrator with compatibility re-exports. |
| A4 | MCP vector schema and parser disagree. | `src/mcp/schema.rs` advertises 80 `u128` words; `src/mcp/tools.rs::parse_hvec` accepts only `as_u64`; JSON numbers cannot safely carry arbitrary 64/128-bit integers. | Use canonical base64/bytes or explicitly encoded word halves, with schema and round-trip tests. |
| A5 | CI and release validate different WASM build targets. | CI runs `wasm-pack build crates/csm-wasm`; release runs root `wasm-pack build ... --features wasm`. | Build, smoke-test, size-check, and publish one canonical artifact. |
| A6 | Workspace CI is incomplete and supply-chain enforcement is not continuous. | CI matrix omits `csm-chaos`; no workflow command runs `cargo deny`, benchmark workspace unit tests, or JS WASM smoke. | Machine-derived complete package matrix and required release-SHA gates. |
| P1 | Persistence loading has an N+1 association query pattern. | `load_replace` and `load_merge` call `load_associations` inside the concept loop. | One namespace-scoped bulk association load with query-count regression tests. |
| W2 | Release skill violates hard limits and repository policy. | `.agents/skills/release-management/SKILL.md` is 294 lines, uses an invalid path, instructs direct-main pushes, and contradicts automatic tag creation. | `<=250` lines; branch/PR/CI flow; one release trigger matching the workflow. |

### P2 — benchmark truthfulness, missing behavior, and maintainability

| ID | Finding | Evidence | Required outcome |
|---|---|---|---|
| E1 | Recorded CI gate claims are false today. | Workflow scan finds no `cargo bench`, `cargo fuzz`, `cargo deny`, `cargo test --manifest-path benchmarks/Cargo.toml`, or `wasm/test.js` execution. | GOAP booleans remain false until literal required jobs exist and pass. |
| E2 | Benchmark metric definitions are misleading. | `hit_at_k` is hit rate but is reported as recall; NDCG uses `1/2^position` despite “logarithmic” docs; abstention truth is inferred from task type instead of `should_abstain`. | Correct formulas and hand-calculated multi-gold/label-disagreement tests. |
| E3 | The 10M/12MB test is configured arithmetic, not measurement. | `tests/performance_targets.rs` multiplies configured bytes/concept and adds constants; it allocates no implementation data. | Measure RSS/allocator/index bytes at multiple scales; only then project with bounded model error. |
| E4 | Benchmark CI omits extracted implementation paths and gates a weak metric subset. | `.github/workflows/benchmark-ci.yml` omits `crates/**`; it gates only Recall@1/5, abstention, and positive storage. | Workspace-aware triggers and tiered correctness/performance thresholds. |
| E5 | ANN and persistence scale evidence is insufficient. | Criterion stops near the ANN activation boundary; no HNSW/LSH build/update/delete/round-trip comparison at representative scales; concurrency retries are unbounded. | Exact/bucket/HNSW/LSH comparisons and bounded contention benchmarks. |
| E6 | Mutation score is weakened by broad exclusions and timeout classification. | `scripts/mutation_test.sh` excludes high-risk modules and counts timeouts as caught. | Timeouts unresolved; changed production files cannot be excluded; publish module-level inventory. |
| F1 | Absence memory is persisted but does not short-circuit BM25. | Sole production TODO at `src/retrieval/bm25.rs:109`; `is_known_absent` has no production caller. | Implement with explicit threshold/invalidation semantics or remove the premature API. |
| F2 | TTL background cleanup has no owned shutdown path. | `src/framework_builder.rs` calls `tokio::spawn` and discards the handle; no cancellation token or cleanup-task field was found. | Store cancellation/handle ownership; cancel and await bounded shutdown. |
| M1 | Duplicate source/tests inflate counts and allow drift. | Root/split BM25 tests are identical; many CLI/WASM sources are duplicates; raw count is 1,034 attributes. | Canonical test ownership and coverage metrics based on unique compiled behavior, not LOC/count ratios. |

### P3 — planning and skill governance

| ID | Finding | Evidence | Required outcome |
|---|---|---|---|
| G1 | Planning contains duplicate/stale canonical state. | Duplicate `tests_passing`, `tests_count`, and `harness_msrv_current`; Wave 31 remains active although all listed actions are complete; quantized HV action remains delegated despite implementation. | Unique keys/action names and dated historical snapshots. |
| G2 | Skill validation omits hard constraints. | `validate-skill-format.sh` exits 0 for all 32 skills while the release skill is 294 lines; it does not enforce quotes, LOC, references, or required sections. | Parse frontmatter and enforce inventory, LOC, references, and trigger/eval policy. |
| G3 | Hook/reference/workflow validators are fragmented. | Multiple hook installers enforce different sets; broken/stale skill references exist; workflow validator has false-positive behavior. | One bootstrap, one canonical gate graph, fixture-tested validators. |
| G4 | Active plans are too large to serve as current state. | `GOAP_STATE.md` is about 100KB and `ACTIONS.md` about 180KB. | Non-destructive archive design with manifest/redirects; no bulk move until references are audited. |

## Triage of the user-owned recommendations snapshot

`plans/RECOMMENDATIONS_2026_07_14.md` remains unchanged.

| Recommendation | Triage at audited HEAD |
|---|---|
| Merge PR #510 | Resolved; merged before this audit |
| MCP `concept_id`/`id` mismatch | Resolved by current HEAD |
| Blocking `std::fs` in async read | Resolved by current HEAD |
| Three workspace Rust LOC violations | Resolved; current files are under 500 |
| N+1 association loading | Still valid; action queued |
| Root/workspace ownership divergence | Still valid, but requires incremental migration rather than blind re-export |
| Persistence connection reuse | Needs benchmark/ADR reconciliation with ADR-0005/0014; not accepted as a direct fix |
| TTL task lifecycle | Still valid; action queued |
| Plans bloat | Still valid; only non-destructive compaction is proposed |
| MCP boundary length checks | Framework validation already exists; protocol-specific limits are optional defense-in-depth, not a verified P0 defect |

## Target state

```yaml
target_state:
  persistence_rows_authoritative: true
  ann_snapshot_revision_validated: true
  ann_config_is_fallible: true
  mutation_failure_semantics_documented_and_tested: true
  no_state_lock_across_io_await: true
  fuzz_workspace_compiles: true
  workspace_implementation_owners_unique: true
  no_default_features_is_lean: true
  disabled_features_never_false_succeed: true
  mcp_vector_wire_contract_roundtrips_full_10240_bits: true
  wasm_ci_release_artifact_identical: true
  benchmark_metrics_mathematically_correct: true
  performance_claims_have_reproducible_artifacts: true
  skill_validation_fail_closed: true
  skills_count: 32
  active_plan_keys_unique: true
```

## GOAP minimal path

### Phase 0 — decision gate

1. `review_adr_0093_persistence_consistency`
2. `review_adr_0094_workspace_contracts`
3. `review_adr_0095_evidence_policy`
4. `review_adr_0096_agent_validation`

Each decision is independent so partial acceptance cannot imply approval of another ADR.

### Phase 1 — P0 correctness and validation

1. `fix_ann_backend_validation`
2. `enforce_authoritative_persistence_and_ann_revision`
3. `repair_fuzz_workspace_and_gate`
4. `make_skill_validation_fail_closed`
5. `align_release_skill_with_protected_workflow`

Exit: invalid ANN config returns `Err`; stale snapshot tests pass; persistence failures do not silently change visible memory; all fuzz targets build; negative validator fixtures fail.

### Phase 2 — P1 ownership and contracts

1. `bulk_load_associations_and_release_state_locks`
2. `enforce_workspace_feature_contracts`
3. `replace_persistence_disabled_noops`
4. `fix_mcp_hypervector_wire_format`
5. `align_wasm_ci_release_artifact`
6. `consolidate_retrieval_ownership`
7. `consolidate_persistence_cli_wasm_ownership`
8. `complete_workspace_ci_and_supply_chain_matrix`

Exit: one owner map, lean feature matrix, one WASM artifact, full workspace/supply-chain CI, constant-query load.

### Phase 3 — P2 evidence and missing behavior

1. `correct_benchmark_metric_definitions`
2. `establish_tiered_benchmark_evidence`
3. `add_ann_and_persistence_scale_benchmarks`
4. `replace_formula_only_memory_claim`
5. `harden_mutation_evidence`
6. `implement_or_remove_bm25_absence_short_circuit`
7. `own_ttl_cleanup_lifecycle`

Exit: PR/scheduled/release evidence tiers are enforced; metric math is tested; scale claims are measured; missing TODO/lifecycle behavior is resolved.

### Phase 4 — P3 consolidation

1. `deduplicate_test_and_source_surfaces`
2. `canonicalize_hooks_skill_refs_and_catalog`
3. `create_harness_md` followed by `reconcile_harness_engineering_state`
4. `compact_active_plans_non_destructively`

Exit: unique implementation/test ownership, 32-skill generated catalog, consistent hooks/references, compact active plans with archive manifest.

## TRIZ analysis

### Ideal final result

One authoritative data owner, one implementation owner, one evidence source, and one active planning source. Derived indexes, generated façades, benchmark reports, and catalogs can be rebuilt without changing behavior.

### Contradictions and principles

| Contradiction | Principle | Application |
|---|---|---|
| Faster ANN startup vs correct current data | Segmentation; prior action | Treat ANN as a revisioned derivative; accept only exact-revision snapshots, otherwise rebuild. |
| Durable writes vs concurrent probes | Separation in time; intermediary | Use a mutation coordinator across I/O while releasing state locks; probes read the previous committed state. |
| Reusable workspace crates vs stable root API | Extraction; intermediary | Workspace crates own code; root preserves public paths through façade adapters during migration. |
| Fast CI vs credible scale claims | Segmentation; periodic action | Deterministic PR gates, scheduled scale runs, release-grade claim artifacts. |
| Concise skills vs complete guidance | Taking out | Keep `SKILL.md <=250`; move detail into checked references and executable validators. |
| Compact plans vs historical auditability | Dynamization; nested doll | Active state plus indexed immutable archives and redirects, never destructive bulk deletion. |

## Explicit deferrals and non-goals

- Configurable hypervector dimensions (ADR-0060) remains deferred pending user demand.
- Do not add a connection pool solely from intuition; first benchmark the current ADR-0005/0014 model and choose local/remote behavior separately.
- Do not bulk re-export divergent modules until API/behavior parity is characterized.
- Do not enforce hardware-sensitive microbenchmark thresholds on unpinned shared runners.
- Do not claim 10M memory or 1M ANN SLOs until measured artifacts exist.
- Do not modify or archive the user-owned recommendations file without explicit approval.

## Decision records

- [ADR-0093](adr/0093-authoritative-persistence-and-derived-index-consistency.md): persistence authority, revisioned ANN snapshots, mutation coordination, lock discipline.
- [ADR-0094](adr/0094-workspace-ownership-and-feature-contracts.md): single implementation ownership, root façade, feature/API/WASM contracts.
- [ADR-0095](adr/0095-evidence-driven-quality-gates.md): correctness and performance evidence tiers.
- [ADR-0096](adr/0096-agent-skill-and-workflow-validation.md): fail-closed skill/workflow validation and plan hygiene.

## Validation commands for this planning change

```bash
./scripts/check-adr-parity.sh
grep -c '^  action_last_completed' plans/GOAP_STATE.md
# Assert no duplicate top-level world_state keys and no duplicate action names.
# Assert every newly queued action uses an allowed status and references a registered ADR where required.
git diff --name-only HEAD
# Only plans/ paths may be added or modified by this task; pre-existing user-owned paths remain untouched.
```
