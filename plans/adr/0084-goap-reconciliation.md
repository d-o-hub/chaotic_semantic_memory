# ADR-0084: GOAP Reconciliation and Codebase Alignment

## Status

Accepted

## Context and Problem Statement

A codebase audit on 2026-05-20 revealed significant state drift between the actual implementation status of the repository and the planning records stored in the `plans/` directory (`GOAP_STATE.md` and `ACTIONS.md`).

Specifically, many major features previously classified as "queued" or "deferred" have already been fully implemented, tested, and integrated into `main`:
1. **Wave 22 Capabilities**:
   - **HNSW & LSH ANN Indexes** (`src/index/hnsw.rs`, `src/index/lsh.rs`, integrated with `src/singularity_search.rs`).
   - **Embedding Model Bridge** (`src/embedding/` with multiple providers: `fastembed`, `hdc_text`, `remote_openai`, `remote_voyage`).
   - **GraphRAG Hybrid Retrieval** (`src/retrieval/graph_rag.rs` and `src/framework_graph_rag.rs`).
2. **Wave 23 capabilities**:
   - **Reranking MMR Pipeline** (`src/retrieval/rerank.rs` and `src/framework_rerank.rs`).
   - **Namespace Isolation** (`src/framework_namespaces.rs` for multi-tenancy).
   - **CloudEvents Event Emitter** (`src/framework_events_ce.rs` for structured event broadcasting).

However, in `GOAP_STATE.md` and `ACTIONS.md`, these features are still marked as `false` or `queued`. This state drift misleads GOAP-based planning agents and human developers regarding the current capabilities of the system.

## Decision Drivers

- Maintain accurate GOAP planning files to ensure agents/developers starting new sessions have an accurate view of the project's completed versus pending capabilities.
- Ensure that durable records of the codebase's architecture and capabilities match reality.
- Enforce the project's strict line count limits (all source files $\le$ 500 lines) and validation gates during documentation updates.

## Considered Options

### Option 1: Reconcile all implemented features in GOAP, mark them complete, and queue the genuine missing features

Update all world state variables in `GOAP_STATE.md` that correspond to features already implemented in the code to `true`. Update `ACTIONS.md` to mark those corresponding actions as `complete`. Add/retain genuine missing features (OTLP Exporter, Quantized Binary Hypervectors, Version History Surface) as `false` and `queued`.

- Good, because it accurately aligns planning files with codebase reality.
- Good, because it prevents the planner from trying to re-implement existing features.
- Good, because it cleanly scopes future work to actual gaps.

### Option 2: Leave GOAP as-is and perform planning separately

Ignore the state drift in `plans/` and track work manually or in external ticket/issue lists.

- Bad, because it violates the `AGENTS.md` mandate to update plans and run the GOAP-based workflow.
- Bad, because planning state drift cascades and causes future agents to attempt redundant work.

## Decision Outcome

Chosen option: **Option 1 — Reconcile all implemented features in GOAP, mark them complete, and queue the genuine missing features**.

Rationale: In chaotic-semantic-memory, the planning files represent the canonical source of truth for agent-based execution. Aligning them with actual codebase reality preserves the integrity of the autonomous development workflow and ensures future sessions build directly on top of the implemented high-scale capabilities.

### Positive Consequences

- The planner now accurately recognizes that high-performance search (HNSW/LSH), rich text embeddings, GraphRAG, and namespaces are native capabilities of the system.
- Scope of next steps is precisely delimited to OTLP, Quantized Binary Hypervectors, and Version History Surface.
- Maintainers can reliably use the automated GOAP tools without false positives/negatives.

### Negative Consequences

- None identified.

## Follow-up Actions

- [ ] Update `plans/GOAP_STATE.md` with reconciled variables and singular `action_last_completed: goap_state_reconciliation_2026_05`.
- [ ] Update `plans/ACTIONS.md` to mark reconciled actions `complete` and retain/define remaining gaps as `queued`.
- [ ] Update `plans/ADR_REGISTRY.md` to register `ADR-0084`.
- [ ] Execute `scripts/check-adr-parity.sh` and `scripts/validate.sh` to ensure perfect documentation/validation state.
