# Implementation ownership map (ADR-0094)

Canonical owner per concern. Root `chaotic_semantic_memory` is a façade/orchestrator
during migration: it may re-export or adapt, not maintain a second algorithm body.

| Concern | Owner crate | Root path | Status (2026-07-16) |
|---|---|---|---|
| Hypervectors, reservoir, encoder | `csm-core` | re-export / `csm_core::*` | owned |
| Concepts, singularity, ANN indexes | `csm-memory` | `src/singularity*`, `src/index*` | transitional (root still hosts orchestration types) |
| BM25 / hybrid / GraphRAG / rerank algorithms | `csm-retrieval` | `src/retrieval/*` | **hybrid façade done**; BM25/graph_rag/rerank still dual until parity PRs |
| Persistence schema/CRUD | `csm-persistence` | `src/persistence*` | transitional (root libSQL impl still primary for CLI) |
| CLI args/commands | `csm-cli` | `src/cli/*` | dual (byte-identical in places) |
| WASM bindings | `csm-wasm` | `src/wasm*` | dual; **npm/CI artifact is `csm-wasm`** |
| Shared traits/events | `csm-traits` | re-export | owned |
| Chaos dynamics | `csm-chaos` | re-export | owned |
| Framework orchestration | root | `src/framework*` | intentional root owner |

## Migration rules

1. Add parity tests before deleting a root algorithm body.
2. Preserve root public paths for at least one compatibility window.
3. Do not blind re-export when behavior diverges (characterize first).
4. Feature flags on root forward to owner crates (`persistence`, `parallel`, ANN).

## Next ownership PRs

1. BM25: move absence short-circuit helpers into owner-neutral API or bridge adapter; then re-export index from `csm-retrieval`.
2. GraphRAG + rerank: parity tests, then façade.
3. Persistence: generalize root types onto `csm-persistence` traits.
4. CLI/WASM: one command surface → `csm-cli` / `csm-wasm` only.
