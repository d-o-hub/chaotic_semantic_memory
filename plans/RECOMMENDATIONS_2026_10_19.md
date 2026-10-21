# Chaotic Semantic Memory — Audit, Improvements, and Follow-ups

**Date:** 2026-10-19  
**Baseline:** `origin/main` fetched at `e62c63f2`; audit branch rebased onto it  
**Scope:** product code, tests, coding workflow, docs/README, skills, and official
Cargo/GitHub Actions guidance. No remote GitHub state was changed.

## Executive verdict

The repository has unusually strong local guardrails, but two small workflow
defects made the canonical truth less reliable than the individual checks:

1. `scripts/check-adr-parity.sh` did not recognize the intentional ADR-0003
   `Superseded | N/A` row under the container's `awk`, so the required parity
   command was red while `validate.sh` used a separate hard-coded implementation.
2. `scripts/goap-orchestrator.sh status` counted historical prose as active
   actions and parsed an inline comment as the last completed action.

Both defects are fixed in this branch with negative fixtures. The remaining
product backlog is consistency-first: persistence failure semantics, import
atomicity, TTL durability, snapshot completeness, and one retrieval contract
across framework, CLI, MCP, and WASM. Another unmeasured BM25 micro-optimization
is lower value than those correctness gaps.

## Evidence and current truth

- Source LOC gate: all first-party Rust files are at or below 500 LOC; several
  files are exactly 500 and should be extracted before substantive growth.
- Skills: 33 `SKILL.md` files on disk; format/catalog gates exist, but behavioral
  skill evaluations remain deferred (`critical_skill_evals_passing: false`).
- Integration files: 73 under `tests/`.
- `scripts/coverage-report.sh inventory`: 1,099 registered test functions. A
  separate attribute scan found 1,101 because the inventory parser does not
  recognize parameterized `#[tokio::test(...)]` forms. The canonical state value
  `tests_count: 1054` is therefore a last-known passing delta, not a fresh full
  suite inventory; do not silently replace it with an unexecuted count.
- ADRs: 95 registry rows and 94 unique on-disk IDs; ADR-0003 is intentionally
  `Superseded | N/A`, and ADR-0024 has two historical files for one ID.
- `gh` is not installed in this environment, so open PRs, workflow conclusions,
  rulesets, and current remote mergeability remain unverified.

## Completed in this audit

### Workflow and plans

- Fixed portable parsing of superseded ADR rows and unified `validate.sh` with
  the canonical parity checker.
- Added `scripts/test-check-adr-parity.sh`, covering both the allowed sentinel
  and a real missing accepted ADR.
- Made GOAP status parsing comment-safe and action-block-scoped; added
  `scripts/test-goap-orchestrator-status.sh`.
- Updated the plans index and corrected unsupported performance goals: the
  10M-concept/12MB claim is not supported, and the WASM gate is 800,000 bytes.

### Documentation and README

- Corrected CLI JSONL indexing syntax (`--file`), output-format values, and the
  location of `query --compact`.
- Corrected WASM examples to use async `WasmFramework.new()`,
  `random_hypervector`, camelCase JS exports, and the required `merge` argument.
- Updated the migration example comment to avoid a stale schema version.
- Replaced obsolete Python `pre-commit` setup instructions with
  `scripts/install-hooks.sh` and aligned release instructions with the protected
  `main`/workflow-run process.
- Corrected the npm CLI's `quiet` format and the documented WASM size evidence.
- Fixed the malformed scaling table row in the root README.

## Prioritized product follow-ups

### P0 — durable-or-unchanged mutation contract

**Evidence:** `src/framework.rs`, `src/framework_ops.rs`, and
`src/framework_ttl.rs` still contain mutations that update in-memory state
before persistence or do not persist the operation at all. Existing failure
tests cover inject/delete/batch inject, not vector/metadata updates,
association changes, reinforcement, pruning, or purge.

**Acceptance:** with simulated persistence failure, every public mutation returns
an error and both the current framework and a fresh reload equal the pre-state;
invalid `associate_many` input has no partial effect; durable graph mutations
advance namespace revision exactly once. Introduce a shared mutation coordinator
only after these tests define the contract.

### P0 — transactional import/replace

**Evidence:** `src/framework_ops_import.rs` clears replacement state before full
validation and applies persistence in separate operations. The WASM path has the
same clear-before-validate shape.

**Acceptance:** malformed JSON/binary/WASM replacement leaves memory and every
durable table unchanged. Valid replacement commits concepts, associations,
metadata, TTL, canonical IDs, revisions, and derived-index policy atomically.
Choose and document reject-all versus explicitly reported skipped associations.

### P1 — one TTL and namespace policy

**Evidence:** top-k filtering can let expired rows consume result slots;
`probe_filtered` and GraphRAG do not share the same expiry filtering; purge
removes memory rows but not durable rows; a reload can resurrect expired data.
`set_namespace` changes the name without loading the persistent target, and cold
namespace export loads concepts without associations.

**Acceptance:** exact, filtered, bridge, hybrid, GraphRAG, export, purge, and
reload tests agree on expiry and namespace behavior. A persistent purge removes
expired concepts and associations durably. Cold export includes graph edges.

### P1 — complete/version snapshots and restore

**Evidence:** namespace bytes and file binary import use different bincode option
paths; backup/restore omits `csm_absences` and `csm_namespace_meta`. Bincode is
also unmaintained per RustSec RUSTSEC-2025-0141.

**Acceptance:** add a bounded, versioned snapshot envelope with an explicit
codec identifier and round-trip tests across native/WASM. Restore either carries
all schema-owned tables or explicitly rebuilds derived state and invalidates
negative/ANN caches. Preserve a compatibility reader before changing a public
format.

### P1 — unify hybrid retrieval

**Evidence:** `ChaoticSemanticFramework::probe_text`, MCP, and WASM use HDC-only
retrieval while the CLI rebuilds a BM25 index per query and indexes only a
200-character preview. The CLI JSON result also omits the stable concept ID.

**Acceptance:** a keyword-only term after character 200 is searchable through
framework/CLI/MCP/WASM surfaces; updates/deletes update postings; repeated
queries reuse a maintained index; JSON includes `concept_id`; semantic-only
mode remains explicit and tested.

### P1 — consolidate GraphRAG ownership and validation

**Evidence:** root `src/retrieval/graph_rag.rs` and
`crates/csm-retrieval/src/graph_rag.rs` remain divergent despite ADR-0094's
single-owner rule. Public config accepts NaN, infinity, and out-of-range weights.

**Acceptance:** one implementation/type owner, root re-export, shared parity
tests for cycles/ties/limits, and `InvalidInput` for invalid numeric config.

### P2 — test and fuzz the boundaries that are currently silent

- Add persistence-failure tests for every mutation and import replacement.
- Add a persisted filtered-absence regression; an unfiltered miss must not hide a
  filtered hit.
- Reuse the same query in cache-clear tests; the current test primes one random
  query and probes another.
- Add property tests for unique result IDs, `len <= top_k`, cached/uncached
  parity, and mutation invalidation.
- Make `fuzz_json_import` drive `ExportPayload` validation or the real import API;
  JSON syntax parsing alone does not fuzz import behavior.
- Add a skewed BM25 benchmark only as a correctness/workload fixture. Do not
  claim a speedup without evidence on unequal posting lengths and interleaved
  saved binaries.

## Coding workflow and release follow-ups

### P0 — make the canonical gate graph explicit

`AGENTS.md` describes `validate.sh` as covering Deny and ADR parity, but Deny,
workspace harness checks, and some validators remain separate or optional. The
next workflow action should define which checks are required locally, which are
CI-only, and which are informational. A missing required dependency must fail
closed; a deliberate skip must be outside the required gate.

Do not merge this with broad feature work. First add fixtures for missing
PyYAML/yamllint/ShellCheck/actionlint and verify that the same gate graph is
executed in every required CI job. Then decide whether `harness-check.sh all`
is a lower-level sensor suite or part of the single gate.

### P1 — harden mutating automation

`self-fix-loop.sh` can commit/push and has no hard refusal for `main`; GOAP
`plan`/`complete` write generated state without transactional updates to the
canonical files. Add tests for main/detached-HEAD refusal, dirty unrelated
files, and explicit confirmation before remote mutation. Never use `git add -A`
in automation. Keep `goap-orchestrator.sh status` read-only and make `plan`,
`execute`, and `complete` explicit operator commands.

### P1 — clarify release dependencies

`release.yml` waits on CI completion, while `pre-release-gate.yml` is a separate
workflow. Decide whether the pre-release checks are publication blockers. If
they are, release must verify the same SHA's completed gate; if not, rename and
document them as evidence. Also document the ownership split with `dist.yml`.

The mutation job must distinguish “no bounded PR diff, no mutants run” from a
successful mutation result. A clean main release cannot be called mutation
tested merely because the script exited zero.

### P2 — skill and script quality

- Select five critical skills and add deterministic behavioral evals, including
  negative cases for missing credentials, stale CI, skipped gates, and main
  branch mutation. Keep `critical_skill_evals_passing` false until measured.
- Generate a script catalog: owner, purpose, file mutation, Git/remote mutation,
  live caller, fixture, and status. Classify operator-only scripts explicitly.
- Expand `scripts/coverage-report.sh inventory` to recognize parameterized
  Tokio test attributes, then keep inventory counts separate from passing-test
  evidence.
- Add a current stable toolchain lane in addition to the intentional MSRV lane.

## Dependency-ordered GOAP follow-up set

This audit deliberately does not add untracked actions to `plans/ACTIONS.md`:
the queue gate requires each active action to have a live GitHub issue, and
`gh` is unavailable here. Once tracker access is restored, create issues and
queue them in this order:

1. Gate graph and plan truth: required dependency behavior, ADR/GOAP fixtures,
   active-only action queue, and state counter reconciliation.
2. Boundary tests: mutation failure matrix, import preservation, TTL reload,
   absence filter scope, cold namespace export, and backup/restore coverage.
3. Durable implementation: transactional mutation/import coordinator and
   revision-driven cache/index invalidation.
4. Lifecycle and snapshots: durable purge, namespace loading, versioned codec,
   complete restore.
5. Retrieval/API consistency: shared BM25 ownership, long-document indexing,
   stable JSON IDs, GraphRAG owner consolidation, MCP/WASM parity.
6. Evidence and operations: fuzz targets, property tests, behavioral skill
   evals, release/mutation semantics, stable-toolchain lane.

## Official references consulted

- [Cargo workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html):
  `default-members` controls root commands when no package selector is used;
  use explicit `--workspace`/`-p` in contributor commands where scope matters.
- [`cargo test`](https://doc.rust-lang.org/cargo/commands/cargo-test.html):
  tests run in multiple targets, doctests are included by default, and
  `--locked` makes dependency resolution reproducible.
- [GitHub workflow syntax](https://docs.github.com/en/actions/reference/workflows/workflow-syntax-for-github-actions):
  path-filtered workflows can remain pending when skipped, and `workflow_run`
  receives the completed workflow's SHA context.
- [GitHub Actions secure use](https://docs.github.com/en/actions/reference/security/secure-use):
  use least-privilege token permissions, pass untrusted values through `env`,
  pin third-party actions, and avoid checking out untrusted code in privileged
  `workflow_run`/`pull_request_target` jobs.

## Validation performed for this audit

```text
git fetch origin main
git rebase origin/main
./scripts/check-adr-parity.sh                         # pass: registry=95, disk=94
./scripts/test-check-adr-parity.sh                    # pass: 2/2 fixtures
./scripts/test-goap-orchestrator-status.sh            # pass
bash -n scripts/check-adr-parity.sh scripts/goap-orchestrator.sh \
  scripts/test-check-adr-parity.sh scripts/test-goap-orchestrator-status.sh \
  scripts/validate.sh                                  # pass
scripts/coverage-report.sh inventory                   # measured 1,099 inventory tests
find src crates ...                                    # all Rust files <=500 LOC
```

Full Rust validation, current GitHub CI, and remote PR triage were not claimed:
the pinned local toolchain lacks Cargo in this container and `gh` is not
installed/authenticated. A meaningful stable-toolchain subset did pass:
`cargo +stable test -p csm-core-lib --lib --quiet` (77 passed) and
`cargo +stable test -p csm-retrieval --lib --quiet` (58 passed). Run the
repository's real pinned-toolchain CI gates after the branch is reviewed and
before any merge.
