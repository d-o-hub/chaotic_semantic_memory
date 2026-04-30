# Gap Analysis & Roadmap — 2026-04-30

## Method

- Scanned `src/` for `TODO|FIXME|unimplemented!|todo!` → **0 markers** (production code clean).
- Cross-referenced `src/framework.rs` public API against `src/cli/commands/*` registrations.
- Compared `plans/ADR_REGISTRY.md` claimed ADRs against on-disk ADR files in `docs/adr/` and `plans/adr/`.
- Reviewed deferred items in `plans/ACTIONS.md` (lines 1117-1187) against current production needs.
- Surveyed competitor / state-of-the-art memory systems (mem0, Letta/MemGPT, Zep, LlamaIndex memory, Cognee, AutoGen memory) for missing surfaces.

## Summary

The crate is feature-complete against its original v0.3 charter (598 tests, 93% coverage, all clippy gates green, WASM parity, Semantic Bridge Layer, hybrid BM25+HDC, graph traversal, text encoder). Gaps are now in **distribution surface, scale ceiling, and AI-agent ergonomics**, not in correctness or core algorithms.

## Findings

### F1 — CLI ↔ Framework API parity gap (P0)

`Framework` exposes 22 public async methods; only 9 are surfaced by the CLI.

| Framework API | CLI counterpart | Status |
|---|---|---|
| `inject_concept` / `inject_concept_with_metadata` | `csm inject` | ✅ |
| `probe` | `csm probe`, `csm query` | ✅ |
| `probe_filtered` | — | ❌ missing |
| `associate` | `csm associate` | ✅ |
| `disassociate` / `clear_associations` | — | ❌ missing |
| `delete_concept` | — | ❌ missing |
| `get_concept` | — | ❌ missing |
| `get_associations` | — | ❌ missing |
| `traverse` (BFS) | — | ❌ missing |
| `shortest_path` | — | ❌ missing |
| `update_concept_vector` / `update_concept_metadata` | — | ❌ missing |
| `stats` | — | ❌ missing |
| `metrics_snapshot` | — | ❌ missing |
| `subscribe` (events) | — | ❌ missing |
| `persistence_health_check` | — | ❌ missing |
| `export` / `import` | `csm export` / `csm import` | ✅ |

**Impact:** Skill-memory and shell users cannot exercise graph features, deletes, or live updates without writing Rust. Limits dogfooding and external adoption.

### F2 — ADR file coverage gap (P1)

`plans/ADR_REGISTRY.md` references ADRs 0024–0066. On disk:
- `docs/adr/`: only `0064`, `0065`
- `plans/adr/`: 9 files (0042, 0046, 0057-0063)

Roughly **40+ ADRs claimed in registry have no on-disk file**. Decision provenance for v0.1.x–v0.3.x lives only in commit messages and GOAP_STATE comments.

### F3 — Scale ceiling at ~200k concepts (P1)

Per `state.deferred_phase2_optimizations` and ADR-0056 trigger criteria:
- Current probe path is brute-force linear scan with Rayon + integer Hamming.
- At >200k concepts the linear scan exceeds the 10ms target.
- No ANN index (HNSW, IVF, LSH, ScaNN-style) is implemented.

`hyperdim_simd.rs` and `singularity_retrieval.rs` are the bottleneck call sites.

### F4 — No agent-protocol surface (P0 for AI memory positioning)

The library positions as "AI memory" but exposes only Rust/WASM/CLI. Modern LLM agents speak:
- **MCP** (Model Context Protocol) — Anthropic / OpenAI / VS Code / Cursor
- **OpenAI tool-calling JSON schema**
- **REST/JSON-RPC** for language-agnostic clients

None are shipped. `mem0`, `Letta`, `Zep` all expose REST + MCP.

### F5 — Embedding model bridge missing (P1)

`TextEncoder` in `src/encoder.rs` produces hypervectors via FNV-1a + seeded PRNG. This is deterministic and fast but **does not capture learned semantic similarity** — "king" and "monarch" are orthogonal.

There is no bridge to:
- Local embedding models (`candle`, `ort`, `fastembed`)
- Remote embedding APIs (OpenAI, Voyage, Cohere)
- Pre-computed embedding files (.parquet, .safetensors)

The Semantic Bridge Layer (ADR-0061) papers over this with BM25+HDC blending but cannot bridge true semantic distance.

### F6 — Observability outputs locked to `tracing` (P2)

`reservoir_tracing_added`, `persistence_tracing_added`, `cli_tracing_added` all true. But there's no:
- OTLP exporter (gRPC or HTTP)
- Prometheus `/metrics` endpoint
- Structured log shipping (loki, datadog)

Production deployments cannot integrate without writing custom subscriber wiring.

### F7 — No multi-tenancy / namespace isolation (P2)

`deferred_namespace_isolation: false` (deferred). All concepts share one keyspace. Cannot host multiple users / projects / agents in one DB without prefix collisions.

### F8 — Concept version history not user-visible (P2)

`persistence_versions.rs` records version history. CLI has no `csm history <id>` or `csm rollback <id> <version>` surface. WASM has no API. The data exists but is invisible.

### F9 — No reranking / MMR (P2)

`probe` returns top-K by cosine score. Modern retrieval pipelines apply:
- **MMR** (Maximum Marginal Relevance) for diversity
- **Cross-encoder reranking** for accuracy
- **Recency-weighted scoring** for time-aware memory

None present.

### F10 — Quantized / binary hypervectors not implemented (P3)

10240-dim f32 = 40 KB/concept. Binary hypervectors (10240 bits = 1.25 KB, 32× compression) with popcount Hamming are a known HDC technique. `hyperdim.rs` keeps everything as f32. Memory ceiling is hit ~25M concepts on 1TB RAM; binary HVs raise it to ~800M.

## Prioritized Recommendations (GOAP-ready)

| Priority | ADR | Title | Cost | Wave |
|---|---|---|---|---|
| P0 | ADR-0066 | CLI ↔ Framework API parity | 12 | 21 |
| P0 | ADR-0067 | MCP server (`csm mcp serve`) | 16 | 21 |
| P1 | ADR-0068 | HNSW ANN index for >200k scale | 18 | 22 |
| P1 | ADR-0069 | External embedding model bridge | 14 | 22 |
| P1 | ADR-0070 | GraphRAG hybrid retrieval | 8 | 22 |
| P2 | ADR-0071 | Reranking + MMR pipeline | 6 | 23 |
| P2 | ADR-0072 | OpenTelemetry OTLP exporter | 6 | 23 |
| P2 | ADR-0073 | Namespace isolation / multi-tenancy | 12 | 23 |
| P2 | ADR-0074 | Concept version history surface | 4 | 23 |
| P3 | ADR-0075 | Quantized binary hypervectors | 14 | 24 |
| P1 | ADR-0076 | ADR backfill — restore decision provenance | 6 | 21 |

Total estimated cost: **116 action units** across 4 waves.

## Sequencing Rationale

- **Wave 21 (P0)**: Unblocks adoption. CLI parity + MCP + ADR backfill = "production-ready external surface".
- **Wave 22 (P1 capabilities)**: Removes scale ceiling and semantic-quality ceiling.
- **Wave 23 (P2 polish)**: Production-grade observability, multi-tenancy, retrieval quality.
- **Wave 24 (P3 compression)**: Future-scale optimization once quality + tooling mature.

## References

- `src/framework.rs:100-491` — current public API
- `src/cli/commands/mod.rs` — current CLI command registry
- `plans/ADR_REGISTRY.md` — claimed ADRs vs on-disk
- `plans/ACTIONS.md:1117-1187` — deferred items
- ADR-0056 — probe scale trigger threshold
