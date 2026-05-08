# Issues Tracker — PR #169

## Issue #148: OpenTelemetry OTLP + Prometheus Exporter (ADR-0072)

- **Status**: Addressed in PR #179.
- **Outcome**: Successfully implemented OTLP gRPC and Prometheus exporters. 7 core metrics surfaced. CLI and environment variable integration complete.
- **Verification**: Smoke tests against local Jaeger and Prometheus passed.

## NON-FIXABLE: HNSW/LSH DeepSource Comments Not Applicable

The following DeepSource comments reference HNSW/LSH ANN index code that does **not exist** in this branch (`feat/graph-rag-hybrid-retrieval-5593725641071927824`). This branch implements GraphRAG hybrid retrieval (ADR-0070), not ANN indices (ADR-0068).

| DeepSource Comment | Reason Not Fixed | Action Required |
|-------------------|------------------|-----------------|
| Found call returning `Self` in `default()` (HNSW/LSH) | No `src/index/hnsw.rs` or `src/index/lsh.rs` files exist in this branch | Verify if HNSW/LSH code was removed or moved to another branch |
| Rebuild index after merge instead of deserializing snapshot | No `load_merge` function with ANN index exists in current code | Check if ANN persistence was deferred |
| Reuse existing HNSW node id when updating a concept | No `HnswIndex::insert` with `id_to_idx` mapping found | Implement if/when HNSW is added |
| Prevent stale index blob from overriding rebuilt state | No `load_replace` with ANN deserialize found | Implement if/when ANN is added |
| Fallback to exact scan when ANN yields zero candidates | No `LshIndex::search` found in codebase | Implement if/when LSH is added |
| Reject unsupported ANN backend instead of silent downgrade | No `IndexBackend::Hnsw` or `IndexBackend::Lsh` enums found | Add validation when ANN feature flags are implemented |
| Load persisted ANN snapshot during startup | No `persistence.load_index` + `index.deserialize` pattern found | Implement if/when ANN persistence is added |
| Rebuild index without cloning all concepts | No `rebuild_index_after_load` function found | Optimize when ANN rebuild is implemented |
| Avoid awaiting persistence under singularity write lock | No `load_replace`/`load_merge` with `self.singularity.write().await` pattern found | Review lock strategy when ANN load is implemented |
| Rebuild HNSW on delete to prevent dropped neighbors | No `HnswIndex::delete` found in codebase | Implement graph maintenance on delete for HNSW |
| Validate HNSW payload boundaries before slicing | No `csm_hnsw_graph` blob deserialization found | Add bounds checks when HNSW persistence is added |
| Propagate index serialization errors in persist | No `index.serialize()` error handling pattern found | Propagate errors when ANN persist is implemented |

## Codex Review Comments (Non-Fixable)

### P2: Implement HNSW index persistence instead of returning empty
- **File**: `src/index/hnsw.rs` (does not exist in this branch)
- **Reason not fixed**: HNSW code is not present in this GraphRAG branch
- **Action required**: When HNSW is implemented in a future PR, ensure `serialize` returns valid payload

## Warnings to Address in Future

1. **`.map().unwrap_or()` → `.map_or()`** pattern - Fixed in `src/cli/commands/query.rs`
2. **Formatting issues** - Fixed in commit `1c7066d`

## Pre-existing Issues Noted

- 3 low-severity vulnerabilities reported by GitHub Dependabot on default branch (not in this PR's scope)
