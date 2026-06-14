# GitHub Issue Decomposition — 2026-06-10

Read-only inputs reviewed:

- GitHub issues/PRs 353–374.
- `plans/ACTIONS.md`.
- `plans/GOAP_STATE.md`.
- `plans/GOAP_CI_REMEDIATION_MUTATION_PR363.md`.
- `plans/adr/0075-quantized-binary-hypervectors.md`.
- Working tree via `git status --short --untracked-files=all` and diffs.

No source files were modified for this review.

## 1. Current Open Issue Map

| Issue | State | Summary | Immediate use |
|---:|---|---|---|
| #353 | OPEN | Quantized binary hypervectors / ADR-0075 | Feature track; decompose into core, memory, persistence, CLI, benchmarks |
| #364 | OPEN | Complete `csm-embedding` workspace extraction | Workspace extraction track; already has `jules` label |
| #365 | OPEN | Finalize `csm-memory` extraction and dependency cycle | Workspace extraction track; central blocker |
| #366 | OPEN | Create `csm-retrieval` workspace crate | Workspace extraction track; depends on #365 |
| #367 | OPEN | Create `csm-persistence` workspace crate | Workspace extraction track; depends on #365 |
| #368 | OPEN | Extract CLI into `csm-cli` workspace crate | Workspace extraction track; depends on #365, #366, #367 |
| #369 | OPEN | Extract WASM bindings into `csm-wasm` workspace crate | Workspace extraction track; depends on #365, #367 |
| #370 | OPEN | Finalize root workspace members and shared dependencies | Workspace orchestration; depends on #364–#369 |
| #371 | OPEN | Remove bridge/stub modules after extraction | Cleanup; depends on #364–#369 and #370 |
| #372 | OPEN | Add `csm-core` WASM32 CI check | CI hardening; can run before full extraction |
| #373 | OPEN | Update CI/pre-release for workspace per-crate jobs | CI hardening; depends on #370 |
| #374 | OPEN | Regenerate `llms.txt`, `llms-full.txt`, `export.json` | Docs/tooling; depends on workspace extraction |

Closed/merged context:

- #355 was the parent workspace-split tracking issue and is CLOSED.
- #356 merged a partial split and extracted `csm-core`.
- #362 merged observability/Prometheus bridge and CI fixes.
- #363 merged BM25 perf changes; CI remediation for its mutation gate is still represented by uncommitted test changes.

## 2. `plans/ACTIONS.md` Status Relevant to This Plan

Observed non-complete statuses:

- `create_derive_macros` — `cancelled`.
- `deferred_concept_ttl` — `deferred`.
- `deferred_performance_phase2` — `deferred`.
- `deferred_association_decay` — `deferred`.
- `deferred_namespace_isolation` — `deferred`.
- `add_otlp_grpc_exporter` — `deferred`.
- `implement_quantized_binary_hypervectors` — `delegated`, `jules_issue: 353`.

No `queued` or `blocked` actions were found in the current `plans/ACTIONS.md` scan. The quantized hypervector work should stay linked to #353 rather than being re-labeled as `deferred_performance_phase2`; #353 is ADR-0075, while the deferred performance phase is ADR-0024.

## 3. Uncommitted Working Tree Changes

Current uncommitted changes:

- `.opencode/package-lock.json` — tooling lockfile changed from OpenAI plugin 1.3.15 to OpenAI plugin 1.3.17 plus Kilo plugin 7.3.41. Treat as unrelated unless the next task is tooling maintenance.
- `export.json` — timestamp drift only; directly relevant to #374.
- `plans/ACTIONS.md` — marks `auto_wire_framework_prom_metrics` complete and records Wave 27 completion.
- `plans/GOAP_STATE.md` — refreshed module LOC map, test counts, PR #363 BM25 notes, and PR #363 CI remediation state.
- `src/retrieval/bm25/tests.rs` — adds two BM25 mutation-killing regression tests for PR #363.
- `.fastembed_cache/...` — generated FastEmbed model cache; should be cleaned or gitignored before workspace/CI work.
- `mutants.out/`, `mutants.out.old/` — generated mutation-test artifacts; should be cleaned or gitignored.
- `plans/GOAP_CI_REMEDIATION_MUTATION_PR363.md` — untracked CI remediation plan artifact; should be committed with the PR #363 remediation branch or moved into an agreed plans location.

Recommended handling:

1. Do not overwrite `src/retrieval/bm25/tests.rs`, `plans/GOAP_STATE.md`, or `plans/ACTIONS.md` until the PR #363 remediation is either committed or explicitly rebased.
2. Link `export.json` timestamp drift to #374 and regenerate after workspace extraction.
3. Treat `.fastembed_cache/` and `mutants.out*` as generated cleanup, preferably a small new issue linked to #364/#373 or a pre-work cleanup task.

## 4. Dependency Graph

Recommended execution order:

1. T00 — Clean generated artifacts and resolve uncommitted state.
2. T01 — Finish `csm-embedding` extraction (#364).
3. T02 — Resolve trait ownership and dependency graph for `csm-memory` (#365).
4. T03 — Extract `csm-memory` (#365).
5. T04 — Extract `csm-persistence` (#367).
6. T05 — Extract `csm-retrieval` (#366).
7. T06 — Extract `csm-cli` (#368).
8. T07 — Extract `csm-wasm` (#369).
9. T08 — Finalize root workspace metadata (#370).
10. T09 — Remove bridge/stub modules (#371).
11. T10 — Add workspace CI matrix and mutation scoping (#373).
12. T11 — Add `csm-core` WASM32 CI (#372).
13. T12 — Regenerate LLM/export docs (#374).
14. T13 — Implement quantized binary hypervectors (#353), with core work able to start before full extraction but integration work blocked by #365/#367/#368.
15. T14 — PR #363 CI remediation can proceed independently from workspace extraction, but should not be mixed into workspace extraction commits.

## 5. Small GitHub Issue Task Decomposition

### T00 — Workspace and generated-artifact hygiene

**Task ID:** `ci-cleanup-generated-artifacts-before-workspace-extraction`  
**Linked to existing issue:** New issue, linked to #364 and #373.  
**Suggested labels:** `cleanup`, `ci`, `good-first-issue`

**Dependencies:** None.

**Acceptance criteria:**

- `.fastembed_cache/` is removed from the working tree or covered by `.gitignore`.
- `mutants.out/` and `mutants.out.old/` are removed from the working tree or covered by `.gitignore`.
- `.opencode/package-lock.json` is either intentionally kept in scope or explicitly excluded from the workspace-extraction branch.
- `export.json` timestamp-only drift is either reverted or documented as part of #374.
- `git status --short` contains only intentional files for the active task.

---

### T01 — Register and test `csm-embedding` extraction

**Task ID:** `workspace-complete-csm-embedding-extraction`  
**Linked to existing issue:** Existing #364.  
**Suggested labels:** `refactor`, `workspace`, `embedding`, `jules`

**Dependencies:** #356 merged; no blocker.

**Acceptance criteria:**

- `crates/csm-embedding/Cargo.toml` exists and uses workspace dependencies.
- `crates/csm-embedding/src/` contains embedding provider modules.
- Root `Cargo.toml` includes `crates/csm-embedding`.
- Main crate no longer has `src/embedding/`.
- `cargo build -p csm-embedding` passes.
- `cargo test -p csm-embedding` passes.
- No new Clippy warnings.

---

### T02 — Decide trait ownership for memory/persistence/retrieval split

**Task ID:** `workspace-decide-persistence-trait-ownership`  
**Linked to existing issue:** New issue, linked to #365.  
**Suggested labels:** `refactor`, `workspace`, `architecture`

**Dependencies:** None, but should be resolved before moving persistence/retrieval crates.

**Acceptance criteria:**

- Decision recorded: `Persistence` trait owned by `csm-memory`, `csm-persistence`, or new `csm-traits`.
- Dependency graph documented for `csm-memory`, `csm-persistence`, and `csm-retrieval`.
- No duplicate trait definitions planned.
- Import path convention documented for internal workspace crates.

---

### T03 — Create `csm-memory` crate skeleton

**Task ID:** `workspace-create-csm-memory-skeleton`  
**Linked to existing issue:** Existing #365.  
**Suggested labels:** `refactor`, `workspace`, `memory`

**Dependencies:** T02; T01 recommended before full import migration.

**Acceptance criteria:**

- `crates/csm-memory/Cargo.toml` exists.
- Framework, Singularity, concept builder, metadata filter, events, namespaces, TTL, and graph-RAG/rerank stubs are moved or scheduled with explicit TODOs.
- Root workspace includes `crates/csm-memory`.
- `cargo check -p csm-memory` passes or fails only on documented follow-up extraction tasks.

---

### T04 — Break `csm-memory` circular dependencies

**Task ID:** `workspace-break-memory-persistence-retrieval-cycles`  
**Linked to existing issue:** Existing #365.  
**Suggested labels:** `refactor`, `workspace`, `blocked`

**Dependencies:** T02, T03.

**Acceptance criteria:**

- `csm-memory` imports only trait-level persistence/retrieval contracts.
- Concrete persistence implementation is not imported by `csm-memory`.
- Concrete retrieval implementation is not imported by `csm-memory`.
- `cargo check -p csm-memory` passes.
- No circular dependency appears in `cargo metadata`.

---

### T05 — Create `csm-persistence` crate

**Task ID:** `workspace-extract-csm-persistence`  
**Linked to existing issue:** Existing #367.  
**Suggested labels:** `refactor`, `workspace`, `persistence`

**Dependencies:** T02, T04.

**Acceptance criteria:**

- `crates/csm-persistence/Cargo.toml` exists.
- `persistence*.rs` and `bridge_persistence.rs` move into the crate.
- `Persistence` trait is imported, not duplicated.
- `cargo build -p csm-persistence` passes.
- `cargo test -p csm-persistence` passes.
- `cargo build -p csm-persistence --no-default-features --features wasm --target wasm32-unknown-unknown` passes if WASM persistence is retained.

---

### T06 — Create `csm-retrieval` crate

**Task ID:** `workspace-extract-csm-retrieval`  
**Linked to existing issue:** Existing #366.  
**Suggested labels:** `refactor`, `workspace`, `retrieval`

**Dependencies:** T04.

**Acceptance criteria:**

- `crates/csm-retrieval/Cargo.toml` exists.
- `src/retrieval/`, graph traversal, BM25, graph-RAG, reranking, and bridge retrieval move into the crate.
- `BridgeRetrieval`, `BM25Retriever`, `GraphRagRetriever`, and `RerankRetriever` are public crate APIs.
- `cargo build -p csm-retrieval` passes.
- `cargo test -p csm-retrieval` passes.
- No circular dependency with `csm-memory`.

---

### T07 — Extract CLI into `csm-cli`

**Task ID:** `workspace-extract-csm-cli`  
**Linked to existing issue:** Existing #368.  
**Suggested labels:** `refactor`, `workspace`, `cli`

**Dependencies:** T04, T05, T06.

**Acceptance criteria:**

- `crates/csm-cli/Cargo.toml` exists.
- `src/cli/` and `src/bin/csm.rs` move into the crate.
- `cargo build -p csm-cli --release` produces a working binary.
- CLI unit tests do not require live DB or network access.
- CLI e2e test covers at least `index-dir` and `probe` against an in-memory store.
- `cargo clippy -p csm-cli -- -D warnings` passes.

---

### T08 — Extract WASM bindings into `csm-wasm`

**Task ID:** `workspace-extract-csm-wasm`  
**Linked to existing issue:** Existing #369.  
**Suggested labels:** `refactor`, `workspace`, `wasm`

**Dependencies:** T04, T05, T06.

**Acceptance criteria:**

- `crates/csm-wasm/Cargo.toml` exists with `crate-type = ["cdylib", "rlib"]`.
- `wasm.rs`, `wasm_ext.rs`, `wasm_ext_tests.rs`, and `wasm_graph_rag.rs` move into the crate.
- Root native crate no longer contains `wasm-bindgen` or `js-sys` blocks.
- `wasm-pack build --target web --release -p csm-wasm` passes.
- WASM bundle size does not regress from the current baseline.
- `wasm_ext_tests` pass headlessly or are gated with a documented replacement.

---

### T09 — Finalize root workspace dependencies

**Task ID:** `workspace-finalize-root-cargo-toml`  
**Linked to existing issue:** Existing #370.  
**Suggested labels:** `chore`, `workspace`, `cargo`

**Dependencies:** T01, T03, T05, T06, T07, T08.

**Acceptance criteria:**

- Root `Cargo.toml` lists all extracted workspace members.
- Shared dependencies are centralized in `[workspace.dependencies]`.
- New crates inherit package metadata from `[workspace.package]`.
- `cargo metadata --format-version 1` shows all expected workspace members.
- `cargo build --workspace` passes.
- `cargo test --workspace` passes.
- `Cargo.lock` has no duplicate major versions for shared dependencies.

---

### T10 — Remove bridge/stub modules from root crate

**Task ID:** `workspace-remove-root-bridge-stubs`  
**Linked to existing issue:** Existing #371.  
**Suggested labels:** `refactor`, `workspace`, `cleanup`

**Dependencies:** T05, T06, T09.

**Acceptance criteria:**

- Root `src/lib.rs` contains only facade re-exports or is removed if the root package becomes binary-only.
- `src/bridge_persistence.rs` and `src/bridge_retrieval.rs` are removed from root.
- `src/semantic_bridge.rs` is moved, replaced by shared crate types, or deleted.
- `cargo check --workspace` passes.
- `cargo doc --workspace --no-deps` passes with no broken intra-doc links.

---

### T11 — Add `csm-core` WASM32 CI

**Task ID:** `ci-add-csm-core-wasm32-check`  
**Linked to existing issue:** Existing #372.  
**Suggested labels:** `ci`, `wasm`, `good-first-issue`

**Dependencies:** #356 merged; can run before full workspace extraction.

**Acceptance criteria:**

- `.github/workflows/ci.yml` has a `wasm-check` job.
- Job installs `wasm32-unknown-unknown`.
- Job runs `cargo build -p csm-core --target wasm32-unknown-unknown --no-default-features`.
- Job runs `cargo clippy -p csm-core --target wasm32-unknown-unknown --no-default-features -- -D warnings`.
- Job is required by branch protection or pre-release gate.

---

### T12 — Add workspace CI matrix

**Task ID:** `ci-workspace-per-crate-test-matrix`  
**Linked to existing issue:** Existing #373.  
**Suggested labels:** `ci`, `workspace`, `testing`

**Dependencies:** T09.

**Acceptance criteria:**

- CI test job uses a matrix over extracted crates.
- Each crate runs `cargo test -p ${{ matrix.crate }}`.
- `cargo clippy --workspace -- -D warnings` runs after crate tests.
- `cargo doc --workspace --no-deps` runs in CI.
- Pre-release gate depends on all matrix jobs.
- CI wall-clock increase is documented and kept under the #373 threshold.

---

### T13 — Add per-crate mutation scoping

**Task ID:** `ci-mutation-tests-per-changed-crate`  
**Linked to existing issue:** Existing #373.  
**Suggested labels:** `ci`, `mutation-testing`, `testing`

**Dependencies:** T09, T12.

**Acceptance criteria:**

- `scripts/mutation_test.sh` supports `--package` or equivalent crate scoping.
- CI determines changed crates from file paths.
- `cargo-mutants` runs only against affected crates.
- Equivalent mutants are documented in a plan/ADR note rather than hidden.
- Mutation threshold remains enforced at >= 85% for affected crate mutants.

---

### T14 — Regenerate LLM/export context after workspace extraction

**Task ID:** `docs-regenerate-llms-and-export-after-workspace`  
**Linked to existing issue:** Existing #374.  
**Suggested labels:** `documentation`, `workspace`, `llms`

**Dependencies:** T05, T06, T07, T08, T09, T10.

**Acceptance criteria:**

- `llms.txt` lists all workspace crates and roles.
- `llms-full.txt` contains correct module paths after extraction.
- `export.json` validates and contains no stale root-crate module paths.
- A CI/pre-commit check prevents future `export.json` drift.
- `plans/ACTIONS.md` and `plans/GOAP_STATE.md` are updated after extraction milestones.

---

### T15 — PR #363 BM25 mutation-gate remediation

**Task ID:** `ci-remediate-pr363-bm25-mutation-gate`  
**Linked to existing issue:** New issue, linked to merged PR #363.  
**Suggested labels:** `ci`, `mutation-testing`, `bm25`, `tests`

**Dependencies:** None; keep separate from workspace extraction.

**Acceptance criteria:**

- `src/retrieval/bm25/tests.rs` includes regression coverage for distinct query terms on both short-query and HashSet dedup paths.
- `cargo test --lib retrieval::bm25` passes with the new tests.
- `cargo fmt --check` passes.
- `cargo clippy --lib -- -D warnings` passes.
- Mutation score reaches >= 85% for the affected diff.
- Equivalent mutants are documented in `plans/GOAP_CI_REMEDIATION_MUTATION_PR363.md` or a follow-up note.

---

### T16 — Add generated-artifact ignore rules for mutation and FastEmbed outputs

**Task ID:** `ci-ignore-mutation-and-fastembed-cache-artifacts`  
**Linked to existing issue:** New issue, linked to #364 and #373.  
**Suggested labels:** `ci`, `cleanup`, `good-first-issue`

**Dependencies:** None.

**Acceptance criteria:**

- `.gitignore` covers mutation output directories consistently.
- `.gitignore` covers `.fastembed_cache/` or the project documents an alternate cache location.
- Existing generated artifacts are removed from the working tree.
- `git status --short --untracked-files=all` no longer shows generated cache or mutation artifacts.

---

### T17 — Implement `BHVec10240` in `csm-core`

**Task ID:** `quantized-add-bhvec10240-core-type`  
**Linked to existing issue:** New issue, linked to #353.  
**Suggested labels:** `performance`, `hypervector`, `wasm`, `jules`

**Dependencies:** #356 merged; no dependency on full workspace extraction.

**Acceptance criteria:**

- `BHVec10240` stores 160 `u64` words.
- Implements from/sign quantization, to-f32 expansion, XOR/bind, Hamming distance, bundle, and permute.
- `BHVec10240` file stays within LOC limits.
- Unit tests cover roundtrip, hamming, bundle majority, permute, and serialization/bytes behavior.
- `cargo test -p csm-core` passes once the crate is extracted, or `cargo test --lib` passes before extraction.

---

### T18 — Add `Hypervector` trait and `HVec10240` implementation

**Task ID:** `quantized-add-hypervector-trait`  
**Linked to existing issue:** New issue, linked to #353.  
**Suggested labels:** `performance`, `hypervector`, `architecture`

**Dependencies:** T17.

**Acceptance criteria:**

- `Hypervector` trait defines distance, bind, bundle, and any required conversion APIs.
- `HVec10240` implements the trait without changing default behavior.
- Existing public APIs remain backward compatible.
- All existing hypervector tests pass.
- No f32 performance regression in benchmark hot paths.

---

### T19 — Add binary hypervector serialization and format metadata

**Task ID:** `quantized-add-binary-vector-serialization`  
**Linked to existing issue:** New issue, linked to #353.  
**Suggested labels:** `performance`, `serialization`, `hypervector`

**Dependencies:** T17, T18.

**Acceptance criteria:**

- Binary vectors serialize/deserialize as packed bytes.
- Legacy f32 vectors continue to serialize/deserialize.
- Format metadata is explicit and versioned.
- Invalid byte lengths return `MemoryError` rather than panicking.
- Tests cover f32, binary, and invalid payloads.

---

### T20 — Make `Singularity<H>` generic over hypervector format

**Task ID:** `quantized-make-singularity-generic`  
**Linked to existing issue:** New issue, linked to #353 and #365.  
**Suggested labels:** `performance`, `hypervector`, `blocked`, `refactor`

**Dependencies:** T18, T04.

**Acceptance criteria:**

- `Singularity<H: Hypervector = HVec10240>` compiles.
- Default `Singularity` remains f32-compatible.
- Binary singularity type alias is added.
- Existing tests pass without changes unless API compatibility is intentionally extended.
- Query cache keys include vector format or otherwise cannot collide across formats.

---

### T21 — Add binary vector opt-in through framework configuration

**Task ID:** `quantized-framework-builder-binary-opt-in`  
**Linked to existing issue:** New issue, linked to #353.  
**Suggested labels:** `performance`, `hypervector`, `api`

**Dependencies:** T20.

**Acceptance criteria:**

- `FrameworkBuilder::with_quantized_vectors(true)` or equivalent selects binary storage.
- Default remains f32.
- Existing examples and tests continue to use f32 by default.
- Public API docs explain memory/accuracy tradeoff.
- `cargo test --workspace` passes after workspace extraction.

---

### T22 — Add persistence migration for vector format

**Task ID:** `quantized-persistence-vector-format-migration`  
**Linked to existing issue:** New issue, linked to #353 and #367.  
**Suggested labels:** `performance`, `hypervector`, `persistence`, `migration`

**Dependencies:** T05, T19.

**Acceptance criteria:**

- Migration `007_add_vector_format.sql` adds a versioned `vector_format` column or equivalent metadata.
- Existing rows default to `f32`.
- Binary rows store packed BLOBs.
- Load/save roundtrip passes for f32 and binary concepts.
- Migration tests cover fresh DB and existing DB upgrade paths.

---

### T23 — Add CLI `inject --format` flag

**Task ID:** `quantized-cli-inject-format-flag`  
**Linked to existing issue:** New issue, linked to #353 and #368.  
**Suggested labels:** `performance`, `hypervector`, `cli`

**Dependencies:** T07, T21.

**Acceptance criteria:**

- `csm inject` accepts `--format f32|binary`.
- Default remains f32.
- Invalid format returns a CLI error with exit code and JSON/table output parity.
- CLI tests cover both formats.
- Help text and completions include the new flag.

---

### T24 — Add quantized hypervector benchmarks

**Task ID:** `quantized-benchmark-binary-recall-and-storage`  
**Linked to existing issue:** New issue, linked to #353.  
**Suggested labels:** `performance`, `benchmark`, `hypervector`

**Dependencies:** T21, T22.

**Acceptance criteria:**

- Benchmark compares binary distance/bundle/search against f32.
- Benchmark reports memory/storage compression ratio.
- Recall@10 vs f32 benchmark report is generated.
- Memory benchmark demonstrates 32x vector storage compression.
- Benchmarks run under the workspace benchmark harness.

---

### T25 — Document quantized binary hypervectors

**Task ID:** `quantized-document-binary-hypervectors`  
**Linked to existing issue:** New issue, linked to #353.  
**Suggested labels:** `documentation`, `performance`, `hypervector`

**Dependencies:** T21, T22, T24.

**Acceptance criteria:**

- README and relevant book chapter explain opt-in binary vectors.
- Docs include memory/accuracy tradeoff.
- Docs include CLI and Rust API examples.
- CHANGELOG entry is drafted for the next release.
- ADR-0075 status is updated to Implemented only after benchmarks pass.

---

### T26 — Keep OTLP gRPC exporter deferred

**Task ID:** `observability-keep-otlp-grpc-deferred`  
**Linked to existing issue:** New issue only if activation is requested; otherwise no issue.  
**Suggested labels:** `deferred`, `observability`

**Dependencies:** None.

**Acceptance criteria:**

- `add_otlp_grpc_exporter` remains `deferred` in `plans/ACTIONS.md`.
- No implementation work starts unless a user explicitly requests gRPC OTLP.
- If activated later, link to ADR-0072 and ADR-0086.

## 6. Recommended First Implementation Slice

If implementation begins after this plan is approved, use this minimal first slice:

1. Resolve T00/T16 so generated artifacts do not pollute workspace extraction or CI work.
2. Continue #364 because it is already delegated and has no blocker.
3. Resolve T02 before moving persistence/retrieval crates.
4. Run T15 independently as a small CI/test-only PR for PR #363 remediation.
5. Start T17/T18 for quantized hypervectors only in `csm-core`; defer generic `Singularity<H>` until #365 lands.

This avoids mixing unrelated concerns and keeps CI remediation, workspace extraction, and quantized hypervector work on separate atomic PRs.
