# Chaotic Semantic Memory — Improvement and Feature Analysis

**Date:** 2026-09-29  
**Baseline:** branch `docs/pr-roast-2026-09-28`, HEAD `3fd1911`  
**Product:** `0.3.8`  
**Scope:** read-only architecture/code survey, official-doc research, and targeted validation; no production implementation changes were made.

## Executive verdict

The project has strong breadth and good structural discipline: 33 skills, 1,029 reported tests, all first-party Rust files within the 500-line gate, ADR parity passing, a built `./target/debug/csm` reporting `0.3.8`, and a broad CLI/MCP/WASM/observability surface.

The next highest-value work is **consistency and correctness**, not another retrieval allocation micro-optimization. The principal product risk is that memory, indexes, caches, imports, TTL state, and persistence can temporarily or permanently disagree. That is especially dangerous for an AI-memory system because a silent false negative is harder for users to detect than an explicit error.

### Validation truth

- `./scripts/check-adr-parity.sh`: passed (`registry=94`, `disk=93`; ADR-0003 is intentionally absent).
- LOC gate: passed; largest first-party Rust files are at or below 500 lines.
- `cargo fmt` and workspace clippy: passed in `harness-check.sh all`.
- `cargo deny check`: exited successfully but emits non-blocking warnings for `cloudevents-sdk` license metadata, duplicate transitive versions, and unmatched license allowances.
- Full local all-features test/link step failed because `ld` was killed with signal 9 while linking several large targets; this is an environment/resource failure, not a compiler/test assertion failure. The gate is still red locally and should be made resource-aware.
- Latest listed CI failures are PR #790's performance-evidence checks (`## Performance Evidence` missing), not evidence that `main` is currently failing. The annotation was inspected.
- Working tree contains only harness-generated untracked `__agent__/`; no tracked source diff was present at audit start.

## Priority roadmap

### P0 — make state transitions safe

#### 1. Atomic, validated import/replace (highest priority)

**Evidence:** `src/framework_ops_import.rs` clears the target namespace in `clear_for_import_replace()` before `apply_import_payload()` validates each concept. The same ordering is used by JSON and binary import. A payload with an invalid ID or oversized metadata can therefore delete the existing in-memory namespace and return an error. If persistence is enabled, the memory clear occurs before the database clear and later writes are separate operations.

**Implement:**

- Parse and validate the complete payload, including IDs, metadata limits, vector sizes, association endpoints, strength ranges, duplicate policy, and namespace constraints, before changing state.
- Add an owner-crate persistence operation such as `replace_namespace_transactional()` that clears and inserts concepts/associations in one database transaction, including revision/absence/index metadata.
- Apply the in-memory replacement only after durable commit; on a post-commit memory failure, reload authoritative rows.
- Give WASM the same staged-validate-then-apply semantics.
- Define whether invalid associations reject the entire import or are reported as skipped; do not silently mix partial success with “replace” semantics.

**Acceptance:** malformed replacement leaves both memory and database byte/row-equivalent to the pre-import state; valid JSON and binary imports round-trip concepts, metadata, associations, TTL, and canonical IDs.

#### 2. Make every mutation durable-or-unchanged

**Evidence:** injection/deletion have an explicit durable-first path, but `update_concept_vector`, `update_concept_metadata`, `associate_many`, `disassociate`, and `clear_associations` mutate memory before persistence. A persistence error can therefore leave a visible in-memory change that will disappear after restart. `associate_many` can also partially mutate before a later item fails.

**Implement:** introduce a mutation coordinator or transaction boundary with a consistent rule:

1. validate the whole operation;
2. commit durable rows/revision;
3. apply memory/index/cache changes;
4. reload authoritative rows if step 3 fails.

Add failure-injection tests for each mutation and for batch operations. Reconcile `GOAP_STATE.md`'s `persistence_failure_leaves_memory_unchanged` flag with the actual scope of the invariant.

#### 3. Repair cache and capacity correctness

**Evidence:** `crates/csm-memory/src/singularity.rs::inject` invalidates the query cache when replacing an existing concept, but not when inserting a new concept. A cached query can miss a newly inserted exact match. Also, `evict_oldest_if_needed()` runs before determining whether an ID already exists, so reinserting/updating an existing ID at capacity may evict an unrelated concept.

**Implement:**

- Invalidate namespace retrieval caches for every insert/update/delete/association or graph change that can affect candidate generation.
- Determine upsert-vs-insert before capacity eviction; only evict for a genuine new ID.
- Add regression tests: probe → insert exact match → probe; probe → update vector; probe → graph change; full store → upsert existing ID.
- Prefer a namespace revision in cache keys over scattered invalidation as a long-term defense.

### P1 — correct lifecycle and retrieval semantics

#### 4. Scope and expire negative retrieval cache

**Evidence:** `AbsenceEntry::id_for()` hashes only normalized query text. It omits namespace, corpus revision, metadata filter/session, embedding provider/model, retrieval mode, and threshold. `probe_text_filtered()` persists filtered misses through the same unscoped absence path. After three misses, later ingestion of a matching concept can be hidden by the short circuit.

**Implement:** version the absence key with at least `{namespace, query, filter, provider/model, retrieval mode, corpus revision}`; invalidate or supersede entries on relevant mutation; add a bounded TTL. Do not short-circuit a filtered query from an unfiltered miss. Consider storing absence records in the same namespace metadata/revision model as ANN snapshots.

**Acceptance:** same-query-after-insert succeeds; same query in another namespace succeeds; a miss under one session/filter does not suppress another; changing provider/model or corpus revision cannot reuse stale absence state.

#### 5. Give TTL cleanup one owner and one shutdown contract

**Evidence:** the framework stores the cleanup `JoinHandle` in an `Arc`, and `Drop` aborts it. Any clone carrying that handle—including a temporary namespace-export clone—can abort the shared worker. `purge_expired()` removes only in-memory concepts; expired rows can return after reload, and not all retrieval surfaces necessarily apply the same expiry behavior.

**Implement:** use a single shared task owner with `CancellationToken` and `TaskTracker` (or an equivalent owned supervisor), never abort shared work from arbitrary clone `Drop`. Expose an async `shutdown()`/`close()` that cancels and awaits cleanup with a bounded timeout. Make durable purge delete rows and associations in the same transaction, and centralize expiry filtering for exact, filtered, batch, hybrid, and GraphRAG queries.

#### 6. Make namespace snapshots faithful and codec-stable

**Evidence:** `ensure_namespace_loaded()` loads concepts but not associations, so cold namespace export can omit graph edges. Native export uses `bincode::serialize`, while native import uses `DefaultOptions::new().with_limit(...)`; bincode's documented function/default-options encodings differ.

**Implement:** add one versioned snapshot module with an explicit codec configuration, magic/version/backend/revision metadata, and bounded decoding. Cold export must load concepts and associations. Test export/import across separate framework instances and across native/WASM boundaries.

**Supply-chain note:** RustSec advisory RUSTSEC-2025-0141 marks bincode unmaintained with no patched version. Plan a migration away from direct bincode for the public snapshot format; retain a compatibility reader only if required. Evaluate a maintained binary format against the project's fixed-size vectors and WASM constraints, but do not silently break existing exports.

#### 7. Complete backup/restore state

**Evidence:** `csm-persistence/src/persistence_ops.rs::restore` copies concepts, associations, versions, HNSW graph, canonical rows, and schema version, but not the newer `csm_absences` or `csm_namespace_meta` tables.

**Implement:** restore all schema-owned tables or define a documented rebuild policy for derived/ephemeral tables. Restore must reset/revalidate namespace revisions and invalidate/rebuild ANN snapshots and negative-cache entries. Add a backup/restore test covering absence records, revisions, associations, and index behavior.

### P2 — user-facing correctness and operational quality

#### 8. Fix CLI indexing and output contracts

**Evidence:** `index_dir`/`index_jsonl` store only 200-character previews, while query-side BM25 reconstruction indexes those previews. Terms after byte/character 200 are not keyword-searchable. Query JSON returns score/text/path/metadata but omits the stable concept ID. `stats` accepts namespace at the CLI layer but `run_stats` constructs a default-scoped framework. Tokenization is duplicated and code-aware query tokenization does not match BM25 index construction.

**Implement:** store canonical source text or a separate searchable-token field, use one shared tokenizer, include `concept_id` in every machine-readable result, and pass namespace into stats/framework creation. Add end-to-end tests for long documents, code separators, JSON consumers, and non-default namespaces.

#### 9. Modernize MCP compatibility and safety

The official MCP `2026-07-28` specification is stateless for Streamable HTTP: protocol sessions and `Mcp-Session-Id` were removed, `server/discover` was added, deterministic tool ordering is recommended for client caching, and HTTP+SSE is deprecated in favor of Streamable HTTP. Current code labels the transport `Sse`, uses `LocalSessionManager`, implements the older `initialize` shape, and does not visibly expose protocol-version/capability conformance tests.

**Implement:** explicitly choose a compatibility matrix (current 2026-07-28 plus fallback if rmcp supports it), rename the misleading `Sse` API, add conformance tests for discovery/version negotiation, deterministic `tools/list`, cancellation, auth/host policy, and trace-context propagation. Treat memory mutation tools as consent/authorization-sensitive operations.

#### 10. Improve production observability

`src/observability/otlp_grpc.rs` uses a simple exporter and ignores shutdown errors. OpenTelemetry guidance requires shutdown/flush to be bounded and one-time; batch processing is the appropriate production option. Add configurable batch export, bounded `force_flush`/shutdown, explicit provider ownership, and metrics for import failures, stale-cache hits, negative-cache short circuits, TTL purges, and persistence reconciliation.

#### 11. Make validation reproducible under resource limits

The local all-features gate is too parallel/link-memory intensive for this environment. Keep the full CI gate, but add a documented low-memory path using bounded Cargo jobs, split feature matrices, and separate link-heavy targets. The result must never convert a killed linker into a false pass. Continue running current stable and MSRV separately.

## New feature proposal: Consistency-first Memory Transactions

Rather than adding another isolated retrieval feature, expose a cohesive API around the defects above:

```text
MemoryTransaction
  validate_import / stage_upsert / stage_delete / stage_associate
  commit() -> CommitReceipt { namespace, revision, changed_ids, warnings }
  rollback()

Framework::begin_transaction()
Framework::snapshot(namespace) -> VersionedSnapshot
Framework::shutdown() -> Result<()>
```

### Why this is the right new feature

- It gives library, CLI, MCP, and WASM callers the same atomic semantics.
- A single namespace revision can drive ANN snapshot validity, query-cache keys, negative-cache invalidation, and observability.
- It turns “durable first, then memory” from an implementation convention into an API contract.
- It enables safe bulk ingestion and replace-import without holding async locks across database I/O.
- It provides a natural `CommitReceipt` for audit logs, MCP responses, and retry/idempotency behavior.

### Deliberate non-goals

- Do not add configurable hypervector dimensions without measured demand.
- Do not replace libSQL with Turso's newer `turso` crate immediately: official Turso guidance recommends `turso` for new local/sync applications but `libsql` remains the documented choice for existing libSQL/remote use, and this repository has an explicit libSQL constraint. First define a backend trait and benchmark local, remote, and sync modes.
- Do not add HNSW/LSH micro-optimizations until cache/revision correctness is covered.
- Do not claim 10M-concept memory limits beyond the measured evidence already recorded.

## Official September 2026 guidance applied

| Source | Verified current fact | Repository implication |
|---|---|---|
| [Rust releases](https://blog.rust-lang.org/releases/latest) | Rust 1.98.1 was released 2026-09-03; repository toolchain/MSRV is 1.88. | Keep 1.88 only if intentional; add stable-1.98 CI and document an MSRV policy. Cargo's `rust-version` contract covers all targets, examples, tests, and features. |
| [Cargo `rust-version`](https://doc.rust-lang.org/cargo/reference/rust-version.html) | Cargo uses `rust-version` for diagnostics and dependency selection; supported functionality should be verified on supported toolchains. | Add a real MSRV job and a current-stable job; do not infer support from one default build. |
| [Tokio graceful shutdown](https://tokio.rs/tokio/topics/shutdown) and [`TaskTracker`](https://docs.rs/tokio-util/latest/tokio_util/task/task_tracker/struct.TaskTracker.html) | CancellationToken signals tasks; TaskTracker waits for them to finish. | Replace shared-handle abort-on-drop with owned cancellation and awaited shutdown. |
| [RustSec bincode advisory](https://rustsec.org/advisories/RUSTSEC-2025-0141.html) and [bincode 1.3 config docs](https://docs.rs/bincode/1.3.3/bincode/config/index.html) | bincode is unmaintained; function helpers and `DefaultOptions` use different integer/trailing-byte behavior. | Version the snapshot codec, migrate public persistence away from bincode, and test exact codec compatibility. |
| [MCP latest specification](https://modelcontextprotocol.io/specification/latest) and [2026-07-28 changelog](https://modelcontextprotocol.io/specification/2026-07-28/changelog) | Current protocol is stateless over Streamable HTTP; HTTP+SSE is deprecated; discovery, cache hints, deterministic lists, and trace metadata are specified. | Add explicit protocol compatibility/conformance and security tests before advertising the server as current. |
| [OpenTelemetry Rust](https://opentelemetry.io/docs/languages/rust/) and [SDK shutdown semantics](https://opentelemetry.io/docs/specs/otel/trace/sdk) | Rust traces/metrics/logs are beta; exporters should be shut down once and shutdown/flush should not block indefinitely. | Use batch export for production and bounded, observable shutdown. |
| [Turso Rust quickstart](https://docs.turso.tech/sdk/rust/quickstart) and [Rust reference](https://docs.turso.tech/sdk/rust/reference) | Turso recommends `turso` for local + sync new projects; `libsql` remains the remote libSQL path. | Preserve the libSQL constraint for now, but document the local/remote trade-off and isolate persistence behind an owner interface. |
| [wasm-bindgen guide](https://wasm-bindgen.github.io/wasm-bindgen/) | The official guide covers web workers, web targets, size optimization, and browser testing. | Make worker/off-main-thread execution and browser conformance a later WASM feature, not merely a compile check. |

## RYAN / FLASH / SOCRATES decision review

### RYAN — methodical risk view

The highest risk is not raw ANN latency; it is silent inconsistency across durable rows, in-memory state, derived indexes, caches, TTL, and MCP/CLI projections. A stale negative cache or omitted association can produce plausible but wrong agent behavior. The bincode supply-chain status and unbounded/abortive lifecycle paths add long-term operational risk.

### FLASH — shipping/value view

Ship the smallest high-impact sequence: cache invalidation + capacity-safe upsert, import pre-validation, and explicit shutdown first. They are bounded changes with direct regression tests. Defer broad backend replacement and advanced vector compression until user demand or measured scale requires them. The transaction API should be developed incrementally behind existing methods rather than blocking all bug fixes.

### SOCRATES — questions that must be answered

- What exact consistency guarantee does each public mutation promise when persistence fails?
- Is an import replacement expected to be all-or-nothing, or is partial association skipping an intentional product behavior?
- What corpus scope makes a negative retrieval reusable, and when does a mutation invalidate it?
- Which MCP protocol revisions does `rmcp 3.4` actually serve, and is `LocalSessionManager` compatibility-only or current behavior?
- What evidence justifies keeping bincode as a public format despite its unmaintained status?
- Which query surfaces must have identical TTL and namespace semantics?

## Dependency-ordered implementation plan

1. Write ADR for transaction/revision/codec contract; define failure semantics.
2. Add regression tests first: import preservation, mutation failure matrix, cache invalidation, capacity upsert, scoped absence, clone/shutdown, cold export, backup restore.
3. Implement cache/revision and capacity fixes.
4. Implement staged/transactional import and durable mutation coordinator.
5. Implement TTL owner/shutdown and durable purge.
6. Replace/version snapshot codec and complete backup/restore.
7. Fix CLI contracts and MCP conformance.
8. Add production observability and low-memory validation matrix.
9. Run `./scripts/validate.sh`, `./scripts/harness-check.sh all`, targeted feature matrices, WASM smoke tests, and current-stable/MSRV CI; update GOAP flags only after evidence.

## Existing queue items not re-proposed

The following are already authoritative in `plans/ACTIONS.md` and should be completed separately rather than duplicated here:

- `reconcile_wave_32_remainder_and_flag_truth`
- `migrate_release_wait_for_ci_to_workflow_run` — follow GitHub's `workflow_run` security warnings; never check out/run untrusted PR code in a privileged workflow.
- `fix_crates_publish_precheck_and_add_duckdb`

The report also does not re-propose completed ownership deduplication, WASM artifact parity, ANN scale evidence, or borrowed retrieval IDs.
